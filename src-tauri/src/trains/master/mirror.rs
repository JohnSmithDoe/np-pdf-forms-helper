// ─── why ────────────────────────────────────────────────────────
// The master import: the Schattensystem as a MIRROR of the customer's master
// workbook, rebuilt on every run. Until the switch to the Schattensystem the
// customer keeps working in Excel, so the master is read again and again, and a
// merge of each run into the last would have to decide which of two of the
// customer's own values is right. A mirror never has to — see
// `docs/decisions.md`, "Der Master wird gespiegelt".
//
// `start` first re-reads the header rows if the file changed since the last
// read (`bindings::sync`), so a sheet new in the file is bound and imported
// without a visit to the settings. Then it empties the facts
// (`TrainsDb::clear_mirror`) and records the run:
// which sheets, and none done. The order is by KIND first — the Wagen lists,
// then the fitting list with positions, then the stock without — and by
// binding order inside a kind, because the stock must meet the fittings the
// positioned list already wrote, or its Radsätze would arrive without positions
// and the date conflicts could not be asked. Each sheet is then staged and
// walked on its own, through the same import walk as a filed document, and
// `done` ticks it off after its commit. A run with sheets left is INCOMPLETE and
// the page says so — a cancelled walk leaves a partial mirror, and a partial
// mirror that looked whole would be checked against the customer as if it were.
//
// Nothing here files a `Dokument`. The refresh reads the latest filed document
// per template, and a master sheet filed as one would become the source the
// next refresh pastes back into the master. The staging carries
// `StagingOrigin::Master` instead, which is what lets `commit` take it without a
// document and what keeps that gate shut for every other file.
//
// A sheet is read through `book` and `grid::from_master`: one sheet
// deserialised, the zero tail cut, cached formula errors empty. Its header row
// is row 1 by construction of these exports, so the plan is built from row 1
// and the kind's template rebound onto it by header (`recognise::rebind`) —
// not detected, because the dashboard's wrapped headers are exactly what a
// scoring reader would misjudge, and the binding already says what the sheet
// is.
// ────────────────────────────────────────────────────────────────

use std::path::Path;

use uuid::Uuid;

use super::{book, kinds};
use crate::error::{AppError, AppResult};
use crate::trains::db::TrainsDb;
use crate::trains::model::{
    ColumnBinding, FieldKind, ImportPlan, MasterBinding, MasterImportRun, MasterSettings,
    StagingOrigin,
};
use crate::trains::sheet::grid::{self, Grid};
use crate::trains::sheet::layout::LayoutHint;
use crate::trains::sheet::readers::ReaderKind;
use crate::trains::stage::{stage, HeldImport, StageInput};
use crate::trains::{entities, recognise};

pub fn start(db: &mut TrainsDb, today: &str) -> AppResult<MasterImportRun> {
    master_file(db.master())?;
    super::bindings::sync(db, false)?;
    let settings = db.master().clone();
    let mut bound: Vec<(u8, usize, &MasterBinding)> = settings
        .bindings
        .iter()
        .enumerate()
        .filter_map(|(at, binding)| binding.kind.map(|kind| (kinds::rank(kind), at, binding)))
        .collect();
    bound.sort_by_key(|(rank, at, _)| (*rank, *at));
    let sheets: Vec<String> = bound
        .into_iter()
        .map(|(_, _, binding)| binding.sheet.clone())
        .collect();
    if sheets.is_empty() {
        return Err(AppError::Report(vec![
            "Es ist noch keinem Blatt der Master-Datei eine Art zugeordnet.".into(),
            "Ohne Art weiß das Programm nicht, wie es ein Blatt lesen soll.".into(),
        ]));
    }

    db.clear_mirror()?;
    let run = MasterImportRun {
        started_at: today.to_string(),
        sheets,
        done: Vec::new(),
    };
    db.save_master(MasterSettings {
        import_run: Some(run.clone()),
        ..settings
    })?;
    Ok(run)
}

pub fn stage_sheet(db: &TrainsDb, sheet: &str) -> AppResult<HeldImport> {
    let settings = db.master();
    let path = master_file(settings)?;
    let binding = bound(settings, sheet)?;
    let Some(kind) = binding.kind else {
        return Err(AppError::Report(vec![format!(
            "Dem Blatt „{sheet}“ ist keine Art zugeordnet."
        )]));
    };

    let mut workbook = book::open(path)?;
    let sheets = book::names(&workbook);
    let index = book::index(&sheets, sheet)?;
    book::deserialise(&mut workbook, index, path)?;
    let (grid, _) = grid::from_master(&workbook.sheet_collection_no_check()[index])?;

    let header = header_plan(&grid);
    let names: Vec<String> = header
        .columns
        .iter()
        .map(|column| column.header.clone())
        .collect();
    let template = kinds::template(kind, &names);
    let plan = recognise::rebind(&template, &header);
    if !plan.missing_required().is_empty() {
        let wanted = template
            .plan
            .binding(FieldKind::Wagennummer)
            .map_or_else(String::new, |binding| binding.header.clone());
        return Err(AppError::Report(vec![
            format!(
                "Das Blatt „{sheet}“ passt nicht zur Art „{}“.",
                template.name
            ),
            format!("In Zeile 1 fehlt die Spalte „{wanted}“."),
        ]));
    }

    let mut staged = stage(StageInput {
        id: Uuid::new_v4().to_string(),
        file: crate::doc::file_name(path),
        sheets,
        candidates: Vec::new(),
        grid: &grid,
        plan: &plan,
        db,
        master: true,
    })?;
    staged.wire.origin = StagingOrigin::Master {
        sheet: sheet.to_string(),
    };
    let mut groups = entities::group(&staged.wire, &[]);
    groups.einbauten = entities::einbau_konflikte(db, &staged.wire, &staged.values);
    staged.wire.entities = Some(groups);

    Ok(HeldImport {
        grid,
        wire: staged.wire,
        values: staged.values,
    })
}

pub fn done(db: &mut TrainsDb, sheet: &str) -> AppResult<()> {
    let mut settings = db.master().clone();
    let Some(run) = settings.import_run.as_mut() else {
        return Ok(());
    };
    if !run.done.iter().any(|name| name == sheet) {
        run.done.push(sheet.to_string());
    }
    db.save_master(settings)
}

fn master_file(settings: &MasterSettings) -> AppResult<&Path> {
    settings
        .file
        .as_deref()
        .map(Path::new)
        .ok_or_else(|| AppError::Report(vec!["Es ist noch keine Master-Datei gewählt.".into()]))
}

fn bound<'a>(settings: &'a MasterSettings, sheet: &str) -> AppResult<&'a MasterBinding> {
    settings
        .bindings
        .iter()
        .find(|binding| binding.sheet == sheet)
        .ok_or_else(|| {
            AppError::Report(vec![format!(
                "Das Blatt „{sheet}“ ist in den Master-Einstellungen nicht zugeordnet."
            )])
        })
}

fn header_plan(grid: &Grid) -> ImportPlan {
    plan_of_headers(
        (1..=grid.cols)
            .map(|col| (col, grid.text(col, 1).trim().to_string()))
            .collect(),
    )
}

pub(super) fn plan_of_headers(headers: Vec<(u32, String)>) -> ImportPlan {
    ImportPlan {
        reader: ReaderKind::HeaderRow,
        layout: LayoutHint {
            header_row: Some(1),
            first_data_row: 2,
            last_data_row: None,
        },
        columns: headers
            .into_iter()
            .filter(|(_, header)| !header.is_empty())
            .map(|(index, header)| ColumnBinding {
                header,
                index,
                field: FieldKind::Ignorieren,
                decimal: None,
                date_order: None,
            })
            .collect(),
        template_id: None,
        date1904: false,
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::testing::{workbook, TempDir};
    use crate::trains::commit;
    use crate::trains::model::{
        EinbauChoice, EntityChoice, EntityDecision, EntityDecisions, EntityGroup, MasterMode,
        Resolution, RowStatus, SheetKind,
    };

    const W1: &str = "218124712173";
    const W2: &str = "218124712181";

    // A dashboard, a fitting list with positions and repeated headers, and the
    // unfiltered stock: one disputed date (RS1), one identical fitting written
    // with dots (RS2), one Radsatz only the stock knows (RS3), then a zero tail.
    fn master(folder: &TempDir) -> std::path::PathBuf {
        workbook(
            folder,
            "Master.xlsx",
            &[
                (
                    "Übersicht",
                    &[
                        &["Wagennummer", "Bemerkungen"],
                        &["#218124712173", "beladen"],
                        &["#218124712181", ""],
                    ],
                ),
                (
                    "Einbauliste",
                    &[
                        &[
                            "an_wagen",
                            "radsatzid",
                            "radsatz",
                            "einbau_am",
                            "einbau_am",
                            "an_wagen",
                            "pos",
                            "halter",
                        ],
                        &[
                            "#218124712173",
                            "#9001",
                            "RS1",
                            "#45500",
                            "#1",
                            "4ACHS",
                            "#1",
                            "Wagenmut AG",
                        ],
                        &[
                            "#218124712173",
                            "#9002",
                            "RS2",
                            "#45500",
                            "#1",
                            "4ACHS",
                            "#2",
                            "Wagenmut AG",
                        ],
                    ],
                ),
                (
                    "Bestand",
                    &[
                        &["radsatzid", "radsatz", "einbau_am", "an_wagen", "halter"],
                        &["#9001", "RS1", "#45600", "#218124712173", "Wagenmut AG"],
                        &["#9002", "RS2", "#45500", "2181.247.1217-3", "Wagenmut AG"],
                        &["#9003", "RS3", "#45610", "#218124712181", "Wagenmut AG"],
                        &["#0", "0", "#0", "#0", "#0"],
                        &["#0", "0", "#0", "#0", "#0"],
                    ],
                ),
            ],
        )
    }

    fn binding(sheet: &str, kind: SheetKind) -> MasterBinding {
        MasterBinding {
            sheet: sheet.into(),
            template_id: String::new(),
            kind: Some(kind),
            mode: MasterMode::Snapshot,
            key: None,
            aliases: Vec::new(),
            auto: false,
        }
    }

    pub(in crate::trains::master) fn bound(folder: &TempDir) -> TrainsDb {
        let file = master(folder);
        let mut db = TrainsDb::load(&folder.config()).unwrap();
        db.save_master(MasterSettings {
            file: Some(file.to_string_lossy().into_owned()),
            bindings: vec![
                binding("Übersicht", SheetKind::Wagenliste),
                binding("Einbauliste", SheetKind::RadsatzEinbau),
                binding("Bestand", SheetKind::RadsatzBestand),
            ],
            import_run: None,
            scan: None,
        })
        .unwrap();
        db
    }

    // What a user pressing „Alle neuen anlegen“ on every step sends.
    fn answer(group: &EntityGroup) -> EntityChoice {
        let decision = match &group.resolution {
            Resolution::Known { id, .. } => EntityDecision::Use { id: id.clone() },
            Resolution::New { .. } => EntityDecision::Create,
            _ => EntityDecision::Skip,
        };
        EntityChoice {
            key: group.key.clone(),
            decision,
        }
    }

    pub(in crate::trains::master) fn walk(
        db: &mut TrainsDb,
        sheet: &str,
        uebernehmen: bool,
    ) -> HeldImport {
        let held = stage_sheet(db, sheet).unwrap();
        let groups = held.wire.entities.clone().unwrap();
        let decisions = EntityDecisions {
            staging_id: held.wire.id.clone(),
            partner: groups.partner.iter().map(answer).collect(),
            wagen: groups.wagen.iter().map(answer).collect(),
            radsaetze: groups.radsaetze.iter().map(answer).collect(),
            einbauten: groups
                .einbauten
                .iter()
                .map(|konflikt| EinbauChoice {
                    key: konflikt.key.clone(),
                    uebernehmen,
                })
                .collect(),
            rows: held
                .wire
                .rows
                .iter()
                .filter(|row| row.status != RowStatus::Rejected)
                .map(|row| row.row)
                .collect(),
        };
        let rows = entities::expand(&held.wire, &decisions);
        commit::commit(db, &held.wire, &held.values, &rows).unwrap();
        done(db, sheet).unwrap();
        held
    }

    fn einbau_of(db: &TrainsDb, radsatz: &str) -> Vec<crate::trains::model::Einbau> {
        let id = db.radsaetze_by_key(radsatz)[0].id.clone();
        db.einbauten()
            .into_iter()
            .filter(|einbau| einbau.radsatz_id == id)
            .collect()
    }

    #[test]
    fn a_run_without_any_kind_is_refused_and_empties_nothing() {
        let folder = TempDir::new("mirror-nokind");
        let mut db = bound(&folder);
        walk_first(&mut db);
        let mut settings = db.master().clone();
        for binding in &mut settings.bindings {
            binding.kind = None;
        }
        db.save_master(settings).unwrap();

        let error = start(&mut db, "2026-10-04").unwrap_err();
        assert!(error.into_messages()[0].contains("keinem Blatt"));
        assert_eq!(db.counts().wagen, 2, "nothing was emptied");
    }

    pub(in crate::trains::master) fn walk_first(db: &mut TrainsDb) {
        start(db, "2026-10-04").unwrap();
        walk(db, "Übersicht", false);
    }

    #[test]
    fn the_dashboard_brings_the_whole_fleet_and_the_run_ticks_it_off() {
        let folder = TempDir::new("mirror-fleet");
        let mut db = bound(&folder);
        let run = start(&mut db, "2026-10-04").unwrap();
        assert_eq!(run.sheets, ["Übersicht", "Einbauliste", "Bestand"]);

        let held = walk(&mut db, "Übersicht", false);
        assert_eq!(
            held.wire.origin,
            StagingOrigin::Master {
                sheet: "Übersicht".into()
            }
        );
        assert!(db.wagen_by_nummer(W1).is_some());
        assert!(db.wagen_by_nummer(W2).is_some());
        let run = db.master().import_run.clone().unwrap();
        assert_eq!(
            run.done,
            ["Übersicht"],
            "the run is incomplete until all three are done"
        );
    }

    #[test]
    fn the_fitting_list_writes_positions_and_names_its_own_sheet() {
        let folder = TempDir::new("mirror-einbau");
        let mut db = bound(&folder);
        walk_first(&mut db);
        walk(&mut db, "Einbauliste", false);

        let rs1 = einbau_of(&db, "RS1");
        assert_eq!(rs1.len(), 1);
        assert_eq!(rs1[0].position.as_deref(), Some("1"));
        assert!(rs1[0].is_open());
        assert_eq!(rs1[0].source.sheet, "Einbauliste");
        assert_eq!(einbau_of(&db, "RS2")[0].position.as_deref(), Some("2"));
        assert_eq!(
            db.radsaetze_by_key("RS1")[0].system_id.as_deref(),
            Some("9001"),
            "the first of the repeated headers is the one bound"
        );
    }

    #[test]
    fn a_disputed_date_is_asked_once_and_kept_without_an_answer() {
        let folder = TempDir::new("mirror-keep");
        let mut db = bound(&folder);
        walk_first(&mut db);
        walk(&mut db, "Einbauliste", false);
        let before = einbau_of(&db, "RS1")[0].eingebaut_am.clone();

        let held = walk(&mut db, "Bestand", false);
        let konflikte = held.wire.entities.unwrap().einbauten;
        assert_eq!(
            konflikte.len(),
            1,
            "only RS1 disagrees; RS2 is the same fitting"
        );
        assert_eq!(konflikte[0].radsatz, "RS1");
        assert_eq!(konflikte[0].bisher, before);
        assert_eq!(
            konflikte[0].bisher_quelle.as_deref(),
            Some("Einbauliste, Zeile 2")
        );

        let rs1 = einbau_of(&db, "RS1");
        assert_eq!(rs1.len(), 1, "no second fitting, no invented removal");
        assert!(rs1[0].is_open());
        assert_eq!(rs1[0].eingebaut_am, before);
    }

    #[test]
    fn taking_the_disputed_date_corrects_the_fitting_in_place() {
        let folder = TempDir::new("mirror-take");
        let mut db = bound(&folder);
        walk_first(&mut db);
        walk(&mut db, "Einbauliste", false);
        let before = einbau_of(&db, "RS1")[0].clone();

        walk(&mut db, "Bestand", true);
        let rs1 = einbau_of(&db, "RS1");
        assert_eq!(rs1.len(), 1);
        assert_eq!(rs1[0].id, before.id, "the same record, corrected");
        assert_ne!(rs1[0].eingebaut_am, before.eingebaut_am);
        assert!(rs1[0].is_open());
        assert_eq!(
            rs1[0].position.as_deref(),
            Some("1"),
            "the position survives"
        );
    }

    #[test]
    fn a_radsatz_only_the_stock_knows_arrives_open_and_without_a_position() {
        let folder = TempDir::new("mirror-extra");
        let mut db = bound(&folder);
        walk_first(&mut db);
        walk(&mut db, "Einbauliste", false);
        walk(&mut db, "Bestand", false);

        let rs3 = einbau_of(&db, "RS3");
        assert_eq!(rs3.len(), 1);
        assert!(rs3[0].is_open());
        assert_eq!(rs3[0].position, None);
        assert_eq!(rs3[0].wagen_id, db.wagen_by_nummer(W2).unwrap().id);
        assert_eq!(
            einbau_of(&db, "RS2").len(),
            1,
            "the dotted spelling is the same Wagen"
        );
        assert_eq!(db.counts().wagen, 2);
    }

    #[test]
    fn the_zero_tail_is_cut_and_never_staged() {
        let folder = TempDir::new("mirror-tail");
        let mut db = bound(&folder);
        walk_first(&mut db);
        let held = stage_sheet(&db, "Bestand").unwrap();
        assert_eq!(held.wire.rows.len(), 3);
        assert!(held
            .wire
            .rows
            .iter()
            .all(|row| row.status != RowStatus::Rejected));
    }

    #[test]
    fn an_open_run_rides_on_the_wire_and_a_finished_one_does_not() {
        use crate::trains::model::TrainsData;
        let folder = TempDir::new("mirror-wire");
        let mut db = bound(&folder);
        walk_first(&mut db);

        let open = TrainsData::nothing().master_import_run(db.master().import_run.clone());
        let json = serde_json::to_value(&open).unwrap();
        assert_eq!(json["masterImportRun"]["done"][0], "Übersicht");
        assert_eq!(
            json["masterImportRun"]["sheets"].as_array().unwrap().len(),
            3
        );

        walk(&mut db, "Einbauliste", false);
        walk(&mut db, "Bestand", false);
        let finished = TrainsData::nothing().master_import_run(db.master().import_run.clone());
        assert!(serde_json::to_value(&finished)
            .unwrap()
            .get("masterImportRun")
            .is_none());
    }

    #[test]
    fn the_next_run_empties_the_facts_and_keeps_the_partner() {
        let folder = TempDir::new("mirror-again");
        let mut db = bound(&folder);
        walk_first(&mut db);
        walk(&mut db, "Einbauliste", false);
        assert_eq!(db.partner().len(), 1);

        let run = start(&mut db, "2026-10-05").unwrap();
        assert!(run.done.is_empty());
        assert_eq!(db.counts().wagen, 0);
        assert!(db.einbauten().is_empty());
        assert!(db.radsaetze().is_empty());
        assert_eq!(db.partner().len(), 1, "identity survives the wipe");

        walk(&mut db, "Übersicht", false);
        let held = walk(&mut db, "Einbauliste", false);
        let halter = &held.wire.entities.unwrap().partner[0];
        assert!(
            matches!(halter.resolution, Resolution::Known { .. }),
            "the kept Halter is not asked about again"
        );
    }

    #[test]
    fn a_sheet_of_the_wrong_kind_names_the_missing_column() {
        let folder = TempDir::new("mirror-wrong");
        let mut db = bound(&folder);
        let mut settings = db.master().clone();
        settings.bindings[0].kind = Some(SheetKind::RadsatzEinbau);
        db.save_master(settings).unwrap();

        let Err(error) = stage_sheet(&db, "Übersicht") else {
            panic!("a dashboard is not a fitting list");
        };
        let messages = error.into_messages();
        assert!(messages[0].contains("passt nicht zur Art"), "{messages:?}");
        assert!(messages[1].contains("an_wagen"), "{messages:?}");
    }

    #[test]
    fn a_fehler_in_any_mapped_cell_rejects_the_row_and_a_zero_date_is_empty() {
        let folder = TempDir::new("mirror-fehler");
        let file = workbook(
            &folder,
            "Master.xlsx",
            &[(
                "Einbauliste",
                &[
                    &["an_wagen", "radsatzid", "radsatz", "einbau_am", "pos"],
                    &["#218124712173", "#9001", "RS1", "kein Datum", "#1"],
                    &["#218124712173", "#9002", "RS2", "#0", "#2"],
                    &["#218124712173", "#9003", "RS3", "#N/A", "#3"],
                ],
            )],
        );
        let mut db = TrainsDb::load(&folder.config()).unwrap();
        db.save_master(MasterSettings {
            file: Some(file.to_string_lossy().into_owned()),
            bindings: vec![binding("Einbauliste", SheetKind::RadsatzEinbau)],
            import_run: None,
            scan: None,
        })
        .unwrap();

        let held = stage_sheet(&db, "Einbauliste").unwrap();
        let status = |row: u32| {
            held.wire
                .rows
                .iter()
                .find(|staged| staged.row == row)
                .map(|staged| staged.status)
        };
        assert_eq!(
            status(2),
            Some(RowStatus::Rejected),
            "a bad date is not a hole"
        );
        assert_ne!(
            status(3),
            Some(RowStatus::Rejected),
            "a 0 date is the empty lookup"
        );
        assert_ne!(
            status(4),
            Some(RowStatus::Rejected),
            "#N/A is the empty lookup"
        );
    }
}
