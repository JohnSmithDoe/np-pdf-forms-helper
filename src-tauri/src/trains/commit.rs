// ─── why ────────────────────────────────────────────────────────
// The only module in trains that writes. Everything before it is a preview that
// cannot change a stored byte, which is what makes "half imported" unreachable.
//
// The REQUIRED FIELDS are checked here and not in `stage`. Staging a half-mapped
// file is the normal state of the mapping screen, and refusing it there makes
// the first pick fail and nothing mappable at all. This is where it matters.
//
// EVERY GATE IS RE-CHECKED HERE. The frontend's ticks are an INPUT, never the
// authority: a stale preview must not be able to mint four hundred partners, and
// a `Use { id }` naming an entity that has since been deleted must fail rather
// than dangle. That is the same reason `staging_id` is echoed — a preview built
// against a different file is not a decision about this one.
//
// The whole run is ONE transaction, so a two thousand row import is a handful of
// file writes rather than six thousand. A row that cannot be committed becomes a
// report line and the rest proceed; only an unusable store stops the run. That is
// `filler::import`'s rule — a per-item failure is a line, and only nothing at all
// is an error — scaled from files to rows.
//
// Confirming a `Likely` or a `New` partner LEARNS the raw spelling as an alias.
// That is the loop that makes the feature survive contact with a monthly file:
// "Fa. Müller GmbH & Co. KG" is asked about once, and every later file from that
// sender resolves without a question. Without it the user answers the same thing
// every month and stops using the import.
//
// A wagen's `Likely` does NOT rewrite the stored number. The stored one is
// presumed right and the incoming one is presumed to be the typo — the opposite
// would let one bad file rewrite the fleet.
// ────────────────────────────────────────────────────────────────

use std::collections::HashMap;

use uuid::Uuid;

use super::clock;
use super::db::{TrainsDb, Tx};
use super::model::{
    CommitDecisions, Einbau, EntityDecision, FieldKind, ImportTemplate, Instandhaltung, Partner,
    PartnerRolle, Provenance, Radsatz, Resolution, RowDecision, RowStatus, StagedImport, StagedRow,
    Wagen,
};
use super::resolve::partner::match_key;
use super::sanitise::{format, Value};
use super::stage::RowValues;
use crate::error::{AppError, AppResult};
use crate::model::ClientReport;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Committed {
    pub messages: Vec<String>,
    pub wagen: u32,
    pub partner: u32,
    pub instandhaltungen: u32,
    pub radsaetze: u32,
    pub einbauten: u32,
    pub skipped: u32,
}

impl Committed {
    pub fn report(self) -> ClientReport {
        ClientReport {
            headline: if self.skipped == 0 {
                "Import wurde erfolgreich übernommen".into()
            } else {
                "Import wurde mit Hinweisen übernommen".into()
            },
            messages: self.messages,
            message_folder: None,
        }
    }
}

pub fn commit(
    db: &mut TrainsDb,
    staging: &StagedImport,
    values: &[RowValues],
    decisions: &CommitDecisions,
) -> AppResult<Committed> {
    let missing = staging.plan.missing_required();
    if !missing.is_empty() {
        return Err(AppError::Report(vec![
            "Es fehlen noch Pflichtzuordnungen.".into(),
            format!(
                "Bitte ordne zu: {}.",
                missing
                    .iter()
                    .map(|field| field.label())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ]));
    }

    if decisions.staging_id != staging.id {
        return Err(AppError::Report(vec![
            "Die Vorschau ist nicht mehr aktuell.".into(),
            "Bitte den Import erneut starten.".into(),
        ]));
    }

    let stamp = clock::today_iso();
    let file = staging.file.clone();
    let sheet = staging.sheet.clone();
    let template_sender = staging
        .plan
        .template_id
        .as_deref()
        .and_then(|id| db.template(id))
        .and_then(|template| template.partner_id.clone());

    let rows_by_number: HashMap<u32, &StagedRow> =
        staging.rows.iter().map(|row| (row.row, row)).collect();
    let values_by_number: HashMap<u32, &RowValues> =
        values.iter().map(|entry| (entry.row, entry)).collect();

    db.transaction(|tx| {
        let mut committed = Committed::default();

        for decision in &decisions.rows {
            let Some(row) = rows_by_number.get(&decision.row).copied() else {
                committed
                    .messages
                    .push(format!("Zeile {}: nicht in der Vorschau.", decision.row));
                committed.skipped += 1;
                continue;
            };
            let Some(row_values) = values_by_number.get(&decision.row).copied() else {
                committed.skipped += 1;
                continue;
            };

            let run = Run {
                file: &file,
                sheet: &sheet,
                stamp: &stamp,
                template_sender: template_sender.as_deref(),
            };
            match commit_row(tx, row, row_values, decision, &run) {
                Ok(Some(created)) => {
                    committed.wagen += u32::from(created.wagen);
                    committed.partner += created.partner;
                    committed.radsaetze += u32::from(created.radsatz);
                    committed.einbauten += u32::from(created.einbau);
                    committed.instandhaltungen += 1;
                }
                Ok(None) => committed.skipped += 1,
                Err(message) => {
                    committed
                        .messages
                        .push(format!("Zeile {}: {message}", decision.row));
                    committed.skipped += 1;
                }
            }
        }

        if let Some(name) = &decisions.save_template_as {
            tx.put_template(ImportTemplate {
                id: Uuid::new_v4().to_string(),
                name: name.clone(),
                fingerprint: super::fingerprint::of(&staging.sheet, &staging.plan.columns),
                plan: staging.plan.clone(),
                partner_id: None,
                created_at: stamp.clone(),
            });
            committed
                .messages
                .push(format!("Vorlage „{name}“ wurde gespeichert."));
        }

        committed.messages.insert(
            0,
            format!(
                "{} Wartung(en) übernommen, {} Wagen, {} Partner und {} Radsatz/Radsätze neu angelegt, {} Ein-/Ausbau(ten) erfasst, {} Zeile(n) übersprungen.",
                committed.instandhaltungen,
                committed.wagen,
                committed.partner,
                committed.radsaetze,
                committed.einbauten,
                committed.skipped
            ),
        );
        Ok(committed)
    })
}

/// Everything a row needs that is the same for the WHOLE run. Passed as one
/// value because it travels together and nothing here varies per row.
struct Run<'a> {
    file: &'a str,
    sheet: &'a str,
    stamp: &'a str,
    template_sender: Option<&'a str>,
}

/// The two raw cells a radsatz is built from. The Wellennummer rides along
/// because it is STORED but never matched on — see `db::fill_wellennummer`.
struct RadsatzCells<'a> {
    nummer: Option<&'a str>,
    welle: Option<&'a str>,
}

struct Created {
    wagen: bool,
    partner: u32,
    radsatz: bool,
    einbau: bool,
}

fn commit_row(
    tx: &mut Tx<'_>,
    row: &StagedRow,
    values: &RowValues,
    decision: &RowDecision,
    run: &Run<'_>,
) -> Result<Option<Created>, String> {
    let (file, sheet, stamp) = (run.file, run.sheet, run.stamp);
    if row.status == RowStatus::Rejected {
        return Err("konnte nicht gelesen werden.".into());
    }

    let mut created = Created {
        wagen: false,
        partner: 0,
        radsatz: false,
        einbau: false,
    };

    let uic = match values.value(FieldKind::Wagennummer) {
        Some(Value::Uic(uic)) => Some(uic.as_str().to_string()),
        _ => None,
    };
    let date = match values.value(FieldKind::Datum) {
        Some(Value::Date(date)) => Some(date.to_iso()),
        _ => None,
    };

    let Some(uic) = uic else {
        return Err("Wagennummer fehlt.".into());
    };

    let wagen_id = match resolve_entity(tx, &decision.wagen, &row.wagen)? {
        Slot::Existing(id) => id,
        Slot::Skip => return Ok(None),
        Slot::Create => {
            let id = Uuid::new_v4().to_string();
            tx.put_wagen(Wagen {
                id: id.clone(),
                nummer: uic.clone(),
                halter_id: None,
                eigentuemer_id: None,
                bauart: None,
                bemerkung: None,
                created_at: stamp.to_string(),
                source: Some(provenance(file, sheet, row.row, stamp)),
            });
            created.wagen = true;
            id
        }
    };

    let werkstatt_id = commit_partner(
        tx,
        &decision.werkstatt,
        &row.werkstatt,
        PartnerRolle::Werkstatt,
        raw_of(row, FieldKind::Werkstatt),
        stamp,
        &mut created,
    )?;
    let halter_id = commit_partner(
        tx,
        &decision.halter,
        &row.halter,
        PartnerRolle::Halter,
        raw_of(row, FieldKind::Halter),
        stamp,
        &mut created,
    )?;

    let eigentuemer_id = commit_partner(
        tx,
        &decision.eigentuemer,
        &row.eigentuemer,
        PartnerRolle::Eigentuemer,
        raw_of(row, FieldKind::Eigentuemer),
        stamp,
        &mut created,
    )?;

    if halter_id.is_some() || eigentuemer_id.is_some() {
        if let Some(mut wagen) = tx.db().wagen_by_id(&wagen_id).cloned() {
            let mut changed = false;
            if wagen.halter_id.is_none() {
                if let Some(halter_id) = halter_id {
                    wagen.halter_id = Some(halter_id);
                    changed = true;
                }
            }
            if wagen.eigentuemer_id.is_none() {
                if let Some(eigentuemer_id) = eigentuemer_id {
                    wagen.eigentuemer_id = Some(eigentuemer_id);
                    changed = true;
                }
            }
            if changed {
                tx.put_wagen(wagen);
            }
        }
    }

    // Same rule staging used, re-derived here rather than trusted: the template's
    // partner if there is one, else the Werkstatt this row actually committed to.
    let sender = run
        .template_sender
        .map(str::to_string)
        .or_else(|| werkstatt_id.clone());
    let radsatz_id = commit_radsatz(
        tx,
        &decision.radsatz,
        &row.radsatz,
        RadsatzCells {
            nummer: raw_of(row, FieldKind::Radsatznummer),
            welle: raw_of(row, FieldKind::Wellennummer),
        },
        stamp,
        sender.as_deref(),
        &mut created,
    )?;

    // The fitting is its own record, and only written when the row actually says
    // something about one. A row that merely NAMES a radsatz is work done to it,
    // not a einbau.
    if let Some(radsatz_id) = &radsatz_id {
        let installed = iso_of(values, FieldKind::EingebautAm);
        let removed = iso_of(values, FieldKind::AusgebautAm);
        if installed.is_some() || removed.is_some() {
            if installed.is_some() {
                tx.close_open_einbau(radsatz_id, installed.clone());
            }
            tx.put_einbau(Einbau {
                id: Uuid::new_v4().to_string(),
                radsatz_id: radsatz_id.clone(),
                wagen_id: wagen_id.clone(),
                position: text_of(row, FieldKind::Einbauposition),
                eingebaut_am: installed,
                ausgebaut_am: removed,
                source: provenance(file, sheet, row.row, stamp),
            });
            created.einbau = true;
        }
    }

    if tx.db().event_exists(&values.dedupe_key) {
        return Ok(None);
    }

    tx.put_instandhaltung(Instandhaltung {
        id: Uuid::new_v4().to_string(),
        wagen_id,
        werkstatt_id,
        radsatz_id,
        datum: date,
        leistung: string_of(values, FieldKind::Leistung),
        betrag_cent: match values.value(FieldKind::Betrag) {
            Some(Value::Money(cents)) => Some(*cents),
            _ => None,
        },
        bemerkung: Some(string_of(values, FieldKind::Bemerkung)).filter(|text| !text.is_empty()),
        dedupe_key: values.dedupe_key.clone(),
        source: provenance(file, sheet, row.row, stamp),
    });

    Ok(Some(created))
}

enum Slot {
    Existing(String),
    Create,
    Skip,
}

fn resolve_entity(
    tx: &Tx<'_>,
    decision: &EntityDecision,
    resolution: &Resolution,
) -> Result<Slot, String> {
    match decision {
        EntityDecision::Skip => Ok(Slot::Skip),
        EntityDecision::Use { id } => {
            let known = tx.db().wagen_by_id(id).is_some()
                || tx.db().partner_by_id(id).is_some()
                || tx.db().radsatz(id).is_some();
            if known {
                Ok(Slot::Existing(id.clone()))
            } else {
                Err("die gewählte Zuordnung gibt es nicht mehr.".into())
            }
        }
        EntityDecision::Create => match resolution {
            Resolution::New { .. } | Resolution::Likely { .. } | Resolution::Ambiguous { .. } => {
                Ok(Slot::Create)
            }
            Resolution::Known { id, .. } => Ok(Slot::Existing(id.clone())),
            Resolution::Missing => Ok(Slot::Skip),
        },
    }
}

fn commit_partner(
    tx: &mut Tx<'_>,
    decision: &EntityDecision,
    resolution: &Resolution,
    role: PartnerRolle,
    raw: Option<&str>,
    stamp: &str,
    created: &mut Created,
) -> Result<Option<String>, String> {
    if matches!(resolution, Resolution::Missing) {
        return Ok(None);
    }
    let key = raw.map(match_key).unwrap_or_default();

    match resolve_entity(tx, decision, resolution)? {
        Slot::Skip => Ok(None),
        Slot::Existing(id) => {
            tx.learn_alias(&id, &key);
            Ok(Some(id))
        }
        Slot::Create => {
            let name = raw.unwrap_or_default().trim().to_string();
            if name.is_empty() {
                return Ok(None);
            }
            let id = Uuid::new_v4().to_string();
            tx.put_partner(Partner {
                id: id.clone(),
                rollen: vec![role],
                name,
                match_key: key,
                aliases: Vec::new(),
                bemerkung: None,
                created_at: stamp.to_string(),
            });
            created.partner += 1;
            Ok(Some(id))
        }
    }
}

/// Confirming a radsatz LEARNS the spelling for THIS SENDER, which is what turns
/// that sender's next file into a `Known`. Creating one seeds the same alias, so
/// a radsatz is immediately recognised by whoever introduced it — and choosing
/// "neu" against an identical number from another sender leaves two radsaetze
/// sharing a key, each recognised only by its own sender. That is the whole
/// point; see `resolve::radsatz`.
fn cleaned(raw: Option<&str>) -> Option<String> {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn commit_radsatz(
    tx: &mut Tx<'_>,
    decision: &EntityDecision,
    resolution: &Resolution,
    cells: RadsatzCells<'_>,
    stamp: &str,
    sender: Option<&str>,
    created: &mut Created,
) -> Result<Option<String>, String> {
    let RadsatzCells { nummer: raw, welle } = cells;
    if matches!(resolution, Resolution::Missing) {
        return Ok(None);
    }
    let key = raw
        .map(super::resolve::radsatz::match_key)
        .unwrap_or_default();
    match resolve_entity(tx, decision, resolution)? {
        Slot::Skip => Ok(None),
        Slot::Existing(id) => {
            if !key.is_empty() {
                tx.learn_radsatz_alias(&id, &key, sender);
            }
            tx.fill_wellennummer(&id, welle);
            Ok(Some(id))
        }
        Slot::Create => {
            if key.is_empty() {
                return Ok(None);
            }
            let id = Uuid::new_v4().to_string();
            tx.put_radsatz(Radsatz {
                id: id.clone(),
                nummer: raw.map(str::trim).unwrap_or_default().to_string(),
                match_key: key.clone(),
                aliases: Vec::new(),
                wellennummer: cleaned(welle),
                bauart: None,
                bemerkung: None,
                created_at: stamp.to_string(),
                source: None,
            });
            tx.learn_radsatz_alias(&id, &key, sender);
            created.radsatz = true;
            Ok(Some(id))
        }
    }
}

fn iso_of(values: &RowValues, field: FieldKind) -> Option<String> {
    match values.value(field) {
        Some(Value::Date(date)) => Some(date.to_iso()),
        _ => None,
    }
}

fn text_of(row: &StagedRow, field: FieldKind) -> Option<String> {
    raw_of(row, field)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

fn provenance(file: &str, sheet: &str, row: u32, stamp: &str) -> Provenance {
    Provenance {
        file: file.to_string(),
        sheet: sheet.to_string(),
        row,
        imported_at: stamp.to_string(),
    }
}

fn raw_of(row: &StagedRow, field: FieldKind) -> Option<&str> {
    row.cells
        .iter()
        .find(|cell| cell.field == field)
        .map(|cell| cell.raw.as_str())
}

fn string_of(values: &RowValues, field: FieldKind) -> String {
    values.value(field).map(format::value).unwrap_or_default()
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::testing::TempDir;
    use crate::trains::model::ColumnBinding;
    use crate::trains::sheet::grid::Grid;
    use crate::trains::sheet::layout::LayoutHint;
    use crate::trains::sheet::readers::ReaderKind;
    use crate::trains::stage::{stage, StageInput, Staged};

    fn grid() -> Grid {
        Grid::from_text(
            "Tabelle1",
            &[
                &["Wagennummer", "Datum", "Werkstatt", "Leistung", "Kosten"],
                &[
                    "21 81 2471 217-3",
                    "31.12.2025",
                    "Fa. Müller GmbH",
                    "Bremsprobe",
                    "1.234,56",
                ],
            ],
        )
    }

    fn plan() -> super::super::model::ImportPlan {
        super::super::model::ImportPlan {
            reader: ReaderKind::HeaderRow,
            layout: LayoutHint {
                header_row: Some(1),
                first_data_row: 2,
                last_data_row: None,
            },
            columns: [
                (1, "Wagennummer", FieldKind::Wagennummer),
                (2, "Datum", FieldKind::Datum),
                (3, "Werkstatt", FieldKind::Werkstatt),
                (4, "Leistung", FieldKind::Leistung),
                (5, "Kosten", FieldKind::Betrag),
            ]
            .into_iter()
            .map(|(index, header, field)| ColumnBinding {
                header: header.into(),
                index,
                field,
                decimal: None,
                date_order: None,
            })
            .collect(),
            template_id: None,
            date1904: false,
        }
    }

    fn staged(db: &TrainsDb) -> Staged {
        stage(StageInput {
            id: "s1".into(),
            file: "monat.xlsx".into(),
            sheets: vec!["Tabelle1".into()],
            candidates: Vec::new(),
            grid: &grid(),
            plan: &plan(),
            db,
        })
        .unwrap()
    }

    pub(super) fn create_all(row: u32) -> RowDecision {
        RowDecision {
            row,
            wagen: EntityDecision::Create,
            werkstatt: EntityDecision::Create,
            halter: EntityDecision::Create,
            eigentuemer: EntityDecision::Skip,
            radsatz: EntityDecision::Create,
        }
    }

    pub(super) fn decisions(rows: Vec<RowDecision>) -> CommitDecisions {
        CommitDecisions {
            staging_id: "s1".into(),
            rows,
            save_template_as: None,
        }
    }

    fn fresh(label: &str) -> (TempDir, TrainsDb) {
        let folder = TempDir::new(label);
        let db = TrainsDb::load(&folder.config()).unwrap();
        (folder, db)
    }

    #[test]
    fn ticking_a_row_creates_its_entities_and_its_event() {
        let (_f, mut db) = fresh("commit-happy");
        let plan = staged(&db);
        let result = commit(
            &mut db,
            &plan.wire,
            &plan.values,
            &decisions(vec![create_all(2)]),
        )
        .unwrap();

        assert_eq!(
            (result.instandhaltungen, result.wagen, result.partner),
            (1, 1, 1)
        );
        assert_eq!(db.counts().wagen, 1);
        assert_eq!(db.counts().instandhaltungen, 1);
        let (rows, _) = db.instandhaltungen_page(None, 0, 10);
        assert_eq!(rows[0].datum.as_deref(), Some("2025-12-31"));
        assert_eq!(rows[0].leistung, "Bremsprobe");
        assert_eq!(rows[0].betrag_cent, Some(123_456));
        assert_eq!(rows[0].source.row, 2);
        assert_eq!(rows[0].source.file, "monat.xlsx");
    }

    /// The learning loop: the raw spelling becomes an alias, so the next file
    /// from the same sender resolves without asking.
    #[test]
    fn creating_a_partner_learns_the_spelling_that_produced_it() {
        let (_f, mut db) = fresh("commit-learn");
        let plan = staged(&db);
        commit(
            &mut db,
            &plan.wire,
            &plan.values,
            &decisions(vec![create_all(2)]),
        )
        .unwrap();

        let again = staged(&db);
        assert_eq!(again.wire.summary.ready, 0, "the row is now a duplicate");
        assert!(matches!(
            again.wire.rows[0].werkstatt,
            Resolution::Known { .. }
        ));
        assert!(matches!(again.wire.rows[0].wagen, Resolution::Known { .. }));
    }

    /// Re-importing the same file must not double the event.
    #[test]
    fn committing_the_same_file_twice_adds_nothing_the_second_time() {
        let (_f, mut db) = fresh("commit-twice");
        let first = staged(&db);
        commit(
            &mut db,
            &first.wire,
            &first.values,
            &decisions(vec![create_all(2)]),
        )
        .unwrap();

        let second = staged(&db);
        let result = commit(
            &mut db,
            &second.wire,
            &second.values,
            &CommitDecisions {
                staging_id: second.wire.id.clone(),
                rows: vec![create_all(2)],
                save_template_as: None,
            },
        )
        .unwrap();
        assert_eq!(result.instandhaltungen, 0);
        assert_eq!(db.counts().instandhaltungen, 1);
        assert_eq!(db.counts().wagen, 1);
    }

    #[test]
    fn a_skipped_row_writes_nothing_at_all() {
        let (_f, mut db) = fresh("commit-skip");
        let plan = staged(&db);
        let mut decision = create_all(2);
        decision.wagen = EntityDecision::Skip;
        let result = commit(
            &mut db,
            &plan.wire,
            &plan.values,
            &decisions(vec![decision]),
        )
        .unwrap();
        assert_eq!(result.instandhaltungen, 0);
        assert_eq!(result.skipped, 1);
        assert_eq!(db.counts(), Default::default());
    }

    /// A preview built against a different run is not a decision about this one.
    #[test]
    fn a_stale_staging_id_is_refused_before_anything_is_written() {
        let (_f, mut db) = fresh("commit-stale");
        let plan = staged(&db);
        let error = commit(
            &mut db,
            &plan.wire,
            &plan.values,
            &CommitDecisions {
                staging_id: "veraltet".into(),
                rows: vec![create_all(2)],
                save_template_as: None,
            },
        )
        .unwrap_err();
        assert!(error.into_messages()[0].contains("nicht mehr aktuell"));
        assert_eq!(db.counts(), Default::default());
    }

    /// The frontend's ticks are an input, never the authority.
    #[test]
    fn using_an_entity_that_is_gone_fails_the_row_rather_than_dangling() {
        let (_f, mut db) = fresh("commit-dangling");
        let plan = staged(&db);
        let mut decision = create_all(2);
        decision.wagen = EntityDecision::Use {
            id: "gibtsnicht".into(),
        };
        let result = commit(
            &mut db,
            &plan.wire,
            &plan.values,
            &decisions(vec![decision]),
        )
        .unwrap();
        assert_eq!(result.instandhaltungen, 0);
        assert_eq!(result.skipped, 1);
        assert!(result
            .messages
            .iter()
            .any(|line| line.contains("gibt es nicht mehr")));
        assert_eq!(db.counts().instandhaltungen, 0);
    }

    #[test]
    fn a_row_that_is_not_in_the_preview_is_reported_not_invented() {
        let (_f, mut db) = fresh("commit-unknown-row");
        let plan = staged(&db);
        let result = commit(
            &mut db,
            &plan.wire,
            &plan.values,
            &decisions(vec![create_all(99)]),
        )
        .unwrap();
        assert_eq!(result.skipped, 1);
        assert!(result.messages.iter().any(|line| line.contains("Zeile 99")));
    }

    #[test]
    fn the_run_reports_what_it_did_in_german_on_the_first_line() {
        let (_f, mut db) = fresh("commit-report");
        let plan = staged(&db);
        let result = commit(
            &mut db,
            &plan.wire,
            &plan.values,
            &decisions(vec![create_all(2)]),
        )
        .unwrap();
        assert!(
            result.messages[0].contains("1 Wartung(en) übernommen"),
            "{}",
            result.messages[0]
        );
    }

    #[test]
    fn saving_a_template_stores_the_plan_under_the_files_signature() {
        let (_f, mut db) = fresh("commit-template");
        let plan = staged(&db);
        commit(
            &mut db,
            &plan.wire,
            &plan.values,
            &CommitDecisions {
                staging_id: "s1".into(),
                rows: vec![create_all(2)],
                save_template_as: Some("Werkstatt Müller — Monatsliste".into()),
            },
        )
        .unwrap();
        let templates = db.templates();
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].name, "Werkstatt Müller — Monatsliste");
        assert_eq!(
            db.template_by_fingerprint(&templates[0].fingerprint)
                .unwrap()
                .id,
            templates[0].id
        );
    }

    #[test]
    fn the_owner_is_hooked_onto_the_wagen_it_was_read_beside() {
        let owned = Grid::from_text(
            "Tabelle1",
            &[
                &["Wagennummer", "Datum", "Eigentümer"],
                &["21 81 2471 217-3", "31.12.2025", "Bahn Nord"],
            ],
        );
        let (_f, mut db) = fresh("commit-owner");
        let mut owner_plan = plan();
        owner_plan.columns = owner_plan.columns[..2].to_vec();
        owner_plan.columns.push(ColumnBinding {
            header: "Eigentümer".into(),
            index: 3,
            field: FieldKind::Halter,
            decimal: None,
            date_order: None,
        });
        let staged = stage(StageInput {
            id: "s1".into(),
            file: "monat.xlsx".into(),
            sheets: vec!["Tabelle1".into()],
            candidates: Vec::new(),
            grid: &owned,
            plan: &owner_plan,
            db: &db,
        })
        .unwrap();

        commit(
            &mut db,
            &staged.wire,
            &staged.values,
            &decisions(vec![create_all(2)]),
        )
        .unwrap();
        let wagen = &db.wagen()[0];
        let owner = db
            .partner_by_id(wagen.halter_id.as_deref().unwrap())
            .unwrap();
        assert_eq!(owner.name, "Bahn Nord");
        assert!(owner.has_rolle(PartnerRolle::Halter));
    }
}

#[cfg(test)]
mod radsatz_tests {
    use super::tests::*;
    use super::*;
    use crate::testing::TempDir;
    use crate::trains::model::ColumnBinding;
    use crate::trains::sheet::grid::Grid;
    use crate::trains::sheet::layout::LayoutHint;
    use crate::trains::sheet::readers::ReaderKind;
    use crate::trains::stage::{stage, StageInput, Staged};

    fn radsatz_plan(fields: &[(u32, &str, FieldKind)]) -> super::super::model::ImportPlan {
        super::super::model::ImportPlan {
            reader: ReaderKind::HeaderRow,
            layout: LayoutHint {
                header_row: Some(1),
                first_data_row: 2,
                last_data_row: None,
            },
            columns: fields
                .iter()
                .map(|(index, header, field)| ColumnBinding {
                    header: (*header).into(),
                    index: *index,
                    field: *field,
                    decimal: None,
                    date_order: None,
                })
                .collect(),
            template_id: None,
            date1904: false,
        }
    }

    fn run(grid: &Grid, plan: &super::super::model::ImportPlan, db: &TrainsDb) -> Staged {
        stage(StageInput {
            id: "s1".into(),
            file: "radsaetze.xlsx".into(),
            sheets: vec!["Tabelle1".into()],
            candidates: Vec::new(),
            grid,
            plan,
            db,
        })
        .unwrap()
    }

    fn fresh(label: &str) -> (TempDir, TrainsDb) {
        let folder = TempDir::new(label);
        let db = TrainsDb::load(&folder.config()).unwrap();
        (folder, db)
    }

    fn fitted_grid() -> Grid {
        Grid::from_text(
            "Tabelle1",
            &[
                &["Wagennummer", "Radsatznummer", "Eingebaut am", "Position"],
                &["21 81 2471 217-3", "RS-4711", "01.03.2025", "1"],
            ],
        )
    }

    fn fitted_plan() -> super::super::model::ImportPlan {
        radsatz_plan(&[
            (1, "Wagennummer", FieldKind::Wagennummer),
            (2, "Radsatznummer", FieldKind::Radsatznummer),
            (3, "Eingebaut am", FieldKind::EingebautAm),
            (4, "Position", FieldKind::Einbauposition),
        ])
    }

    #[test]
    fn a_radsatz_is_created_and_fitted_to_its_wagen() {
        let (_f, mut db) = fresh("radsatz-fit");
        let staged = run(&fitted_grid(), &fitted_plan(), &db);
        let result = commit(
            &mut db,
            &staged.wire,
            &staged.values,
            &decisions(vec![create_all(2)]),
        )
        .unwrap();

        assert_eq!(result.radsaetze, 1);
        assert_eq!(result.einbauten, 1);

        let radsatz = &db.radsaetze()[0];
        // The number is kept as the sender wrote it; only the key is normalised,
        // so the list shows what the file said and matching still ignores dashes.
        assert_eq!(radsatz.nummer, "RS-4711");
        assert_eq!(radsatz.match_key, "RS4711");

        let einbauten = db.einbauten();
        let einbau = einbauten
            .iter()
            .find(|einbau| einbau.radsatz_id == radsatz.id && einbau.is_open())
            .expect("currently fitted");
        assert_eq!(einbau.eingebaut_am.as_deref(), Some("2025-03-01"));
        assert_eq!(einbau.position.as_deref(), Some("1"));
        assert!(einbau.ausgebaut_am.is_none());
    }

    /// The event names the radsatz too, so "what was done to this radsatz" is
    /// answerable without a second event type.
    #[test]
    fn the_event_records_which_radsatz_the_work_was_about() {
        let (_f, mut db) = fresh("radsatz-event");
        let staged = run(&fitted_grid(), &fitted_plan(), &db);
        commit(
            &mut db,
            &staged.wire,
            &staged.values,
            &decisions(vec![create_all(2)]),
        )
        .unwrap();

        let (events, _) = db.instandhaltungen_page(None, 0, 10);
        let radsatz = &db.radsaetze()[0];
        assert_eq!(events[0].radsatz_id.as_deref(), Some(radsatz.id.as_str()));
    }

    /// A radsatz is under at most one wagen, so fitting it elsewhere ends the
    /// previous fitting rather than leaving two open.
    #[test]
    fn fitting_a_radsatz_elsewhere_closes_the_previous_fitting() {
        let (_f, mut db) = fresh("radsatz-move");
        let first = run(&fitted_grid(), &fitted_plan(), &db);
        commit(
            &mut db,
            &first.wire,
            &first.values,
            &decisions(vec![create_all(2)]),
        )
        .unwrap();

        let moved = Grid::from_text(
            "Tabelle1",
            &[
                &["Wagennummer", "Radsatznummer", "Eingebaut am", "Position"],
                &["31 80 4740 123-4", "RS-4711", "01.09.2025", "2"],
            ],
        );
        let second = run(&moved, &fitted_plan(), &db);
        commit(
            &mut db,
            &second.wire,
            &second.values,
            &CommitDecisions {
                staging_id: second.wire.id.clone(),
                rows: vec![create_all(2)],
                save_template_as: None,
            },
        )
        .unwrap();

        let radsatz = &db.radsaetze()[0];
        let mut history = db.einbauten();
        history.retain(|einbau| einbau.radsatz_id == radsatz.id);
        history.sort_by(|a, b| b.eingebaut_am.cmp(&a.eingebaut_am));
        assert_eq!(history.len(), 2, "both fittings are kept");
        assert_eq!(
            db.einbauten().iter().filter(|m| m.is_open()).count(),
            1,
            "only the current one is open"
        );
        assert_eq!(history[0].eingebaut_am.as_deref(), Some("2025-09-01"));
        assert_eq!(history[1].ausgebaut_am.as_deref(), Some("2025-09-01"));
    }

    /// A row that only NAMES a radsatz is work done to it, not a fitting.
    #[test]
    fn a_row_without_dates_records_no_fitting() {
        let (_f, mut db) = fresh("radsatz-nodates");
        let grid = Grid::from_text(
            "Tabelle1",
            &[
                &["Wagennummer", "Radsatznummer", "Leistung"],
                &["21 81 2471 217-3", "RS-4711", "Profil gedreht"],
            ],
        );
        let plan = radsatz_plan(&[
            (1, "Wagennummer", FieldKind::Wagennummer),
            (2, "Radsatznummer", FieldKind::Radsatznummer),
            (3, "Leistung", FieldKind::Leistung),
        ]);
        let staged = run(&grid, &plan, &db);
        let result = commit(
            &mut db,
            &staged.wire,
            &staged.values,
            &decisions(vec![create_all(2)]),
        )
        .unwrap();

        assert_eq!(result.radsaetze, 1);
        assert_eq!(result.einbauten, 0);
        assert_eq!(db.einbauten().len(), 0);
    }

    /// The same radsatz written four ways is one radsatz.
    #[test]
    fn a_radsatz_number_has_one_stored_form() {
        let (_f, mut db) = fresh("radsatz-normal");
        for written in ["RS-4711", "rs 4711", "RS.4711"] {
            let grid = Grid::from_text(
                "Tabelle1",
                &[
                    &["Wagennummer", "Radsatznummer"],
                    &["21 81 2471 217-3", written],
                ],
            );
            let plan = radsatz_plan(&[
                (1, "Wagennummer", FieldKind::Wagennummer),
                (2, "Radsatznummer", FieldKind::Radsatznummer),
            ]);
            let staged = run(&grid, &plan, &db);
            commit(
                &mut db,
                &staged.wire,
                &staged.values,
                &CommitDecisions {
                    staging_id: staged.wire.id.clone(),
                    rows: vec![create_all(2)],
                    save_template_as: None,
                },
            )
            .unwrap();
        }
        assert_eq!(db.radsaetze().len(), 1);
        assert_eq!(db.radsaetze_by_key("RS4711").len(), 1);
    }

    #[test]
    fn removing_a_radsatz_takes_its_history_with_it() {
        let (_f, mut db) = fresh("radsatz-remove");
        let staged = run(&fitted_grid(), &fitted_plan(), &db);
        commit(
            &mut db,
            &staged.wire,
            &staged.values,
            &decisions(vec![create_all(2)]),
        )
        .unwrap();

        let id = db.radsaetze()[0].id.clone();
        db.transaction(|tx| {
            tx.remove_radsatz(&id);
            Ok(())
        })
        .unwrap();

        assert_eq!(db.radsaetze().len(), 0);
        assert_eq!(db.einbauten().len(), 0);
        let (events, _) = db.instandhaltungen_page(None, 0, 10);
        assert!(
            events[0].radsatz_id.is_none(),
            "the event survives, unhooked"
        );
    }
}
