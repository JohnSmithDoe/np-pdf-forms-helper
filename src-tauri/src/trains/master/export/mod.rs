// ─── why ────────────────────────────────────────────────────────
// One filed Dokument into the sheets of the master the user ticked, written as
// a new dated copy. It replaced the one-click refresh (every binding from the
// latest document of its template): the user now sees what goes where before
// anything is written. See `docs/decisions.md`, „Export in die Master-Datei“.
//
// PLAN AND APPLY ARE ONE CODE PATH. `preview` pastes into an in-memory book and
// diffs the sheets before/after (`diff`); `write` runs the same and saves the
// book. A preview that predicted the paste would be a second paste, and the
// first one wrong would make the preview lie.
//
// A CONFLICT IS STRUCTURAL, never a value: every document is an increment, and a
// value differing from the master's is the update. What asks is a document
// column no master column takes (`open`) — answered once by an alias onto a
// hand-kept column or by „nicht übertragen“ (`ignored`), both remembered on the
// binding, so the next document of the template asks again only when the
// structure moved. A remembered alias or key the sheet no longer has is
// dropped and SAID (`conflicts`), never silently re-pointed. A sheet that cannot
// be written at all — no shared column, a feed without key — is a `problem`.
//
// THE ORIGINAL IS NEVER WRITTEN, as for every master write: the copy is
// `<Original> <Datum>.xlsx` beside the original (`free_path` for a second run the
// same day), recorded as `last_export` so the next document builds on it
// (`suggest::bases`). The cleaned copy is the source and refused when edited
// since filing (`source::load`). `.xlsm` is refused: umya has no VBA story.
// ────────────────────────────────────────────────────────────────

mod diff;
mod suggest;

use std::path::Path;

use umya_spreadsheet::Workbook;

use super::source::Table;
use super::{book, paste, view};
use crate::error::{AppError, AppResult};
use crate::trains::db::TrainsDb;
use crate::trains::model::{
    Dokument, MasterBinding, MasterExportChoice, MasterExportRequest, MasterExportRun,
    MasterExportSheetRun, MasterMode, MasterSettings,
};
use crate::trains::sheet::grid;

pub use suggest::start;

pub fn preview(db: &TrainsDb, request: &MasterExportRequest) -> AppResult<MasterExportRun> {
    apply(db, request).map(|(run, _)| run)
}

pub fn write(
    db: &mut TrainsDb,
    request: &MasterExportRequest,
    today: &str,
) -> AppResult<MasterExportRun> {
    let (mut run, book) = apply(db, request)?;
    if run.sheets.iter().all(|sheet| sheet.problem.is_some()) {
        return Err(AppError::Report(
            run.sheets
                .iter()
                .filter_map(|sheet| sheet.problem.clone())
                .collect(),
        ));
    }
    let original = original(db.master())?.to_path_buf();
    let folder = original.parent().unwrap_or_else(|| Path::new("."));
    let stem = original.file_stem().map_or_else(
        || "Master".into(),
        |stem| stem.to_string_lossy().into_owned(),
    );
    let target = crate::doc::free_path(folder, &format!("{stem} {today}.xlsx"));
    crate::doc::write_book(
        &book,
        &target,
        format!(
            "Die Master-Datei {} konnte nicht geschrieben werden.",
            crate::doc::file_name(&target)
        ),
    )?;

    let mut settings = db.master().clone();
    settings.last_export = Some(target.to_string_lossy().into_owned());
    if request.remember {
        let template = dokument(db, &request.dokument_id)?.template_id.clone();
        remember(&mut settings, &db.templates(), &template, &run);
    }
    db.save_master(settings)?;
    run.folder = Some(folder.to_string_lossy().into_owned());
    run.target = Some(target.to_string_lossy().into_owned());
    Ok(run)
}

pub(super) fn dokument<'a>(db: &'a TrainsDb, id: &str) -> AppResult<&'a Dokument> {
    db.dokument(id)
        .ok_or_else(|| AppError::Report(vec!["Das Dokument gibt es nicht mehr.".into()]))
}

pub(super) fn original(settings: &MasterSettings) -> AppResult<&Path> {
    let path =
        settings.file.as_deref().map(Path::new).ok_or_else(|| {
            AppError::Report(vec!["Es ist noch keine Master-Datei gewählt.".into()])
        })?;
    if path.is_file() {
        Ok(path)
    } else {
        Err(AppError::Report(vec![format!(
            "Die Master-Datei {} gibt es nicht.",
            path.display()
        )]))
    }
}

fn apply(db: &TrainsDb, request: &MasterExportRequest) -> AppResult<(MasterExportRun, Workbook)> {
    let dokument = dokument(db, &request.dokument_id)?;
    if request.sheets.is_empty() {
        return Err(AppError::Report(vec!["Es ist kein Blatt gewählt.".into()]));
    }
    let settings = db.master();
    let (bases, _) = suggest::bases(settings)?;
    if !bases.iter().any(|base| base.path == request.base) {
        return Err(AppError::Report(vec![
            "Die gewählte Ausgangsdatei gehört nicht zur Master-Datei.".into(),
        ]));
    }
    let base = Path::new(&request.base);
    if base
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("xlsm"))
    {
        return Err(AppError::Report(vec![
            "Die Master-Datei ist eine .xlsm-Datei mit Makros.".into(),
            "Diese kann das Programm nicht schreiben, ohne die Makros zu verlieren.".into(),
        ]));
    }

    let table = super::source::load(dokument)?;
    let mut book = book::open(base)?;
    let names = book::names(&book);
    let sheets = request
        .sheets
        .iter()
        .map(|choice| {
            let binding = settings
                .bindings
                .iter()
                .find(|binding| binding.sheet == choice.sheet)
                .cloned()
                .unwrap_or_else(|| unbound(&choice.sheet));
            sheet(&mut book, &names, base, binding, choice, &table)
        })
        .collect();
    Ok((
        MasterExportRun {
            dokument_id: dokument.id.clone(),
            base: request.base.clone(),
            sheets,
            ..MasterExportRun::default()
        },
        book,
    ))
}

fn sheet(
    book: &mut Workbook,
    names: &[String],
    path: &Path,
    mut binding: MasterBinding,
    choice: &MasterExportChoice,
    table: &Table,
) -> MasterExportSheetRun {
    let mut out = MasterExportSheetRun {
        sheet: choice.sheet.clone(),
        mode: binding.mode,
        ..MasterExportSheetRun::default()
    };
    let result = (|| -> AppResult<()> {
        let index = book::index(names, &choice.sheet)?;
        book::deserialise(book, index, path)?;
        let worksheet = book.sheet_mut(index).map_err(|error| {
            AppError::detail(
                format!("Das Blatt „{}“ konnte nicht gelesen werden.", choice.sheet),
                error,
            )
        })?;

        let header: Vec<String> = view::header_row(worksheet)
            .into_iter()
            .map(|(_, text)| text)
            .collect();
        let has = |name: &str| header.iter().any(|own| own == name.trim());
        binding.aliases.clear();
        for alias in &choice.aliases {
            if has(&alias.master) && table.column(alias.source.trim()).is_some() {
                binding.aliases.push(alias.clone());
            } else {
                out.conflicts.push(format!(
                    "Die Zuordnung „{}“ ← „{}“ passt nicht mehr und wurde verworfen.",
                    alias.master, alias.source
                ));
            }
        }
        binding.key = match choice.key.as_deref().map(str::trim) {
            Some(key) if !key.is_empty() && has(key) => Some(key.to_string()),
            Some(key) if !key.is_empty() => {
                out.conflicts.push(format!(
                    "Die Schlüsselspalte „{key}“ gibt es im Blatt nicht mehr."
                ));
                None
            }
            _ => None,
        };

        let structure = paste::structure(worksheet, &binding, table)?;
        binding.ignored = choice
            .ignored
            .iter()
            .filter(|column| structure.unmatched.contains(column))
            .cloned()
            .collect();
        out.open = structure
            .unmatched
            .iter()
            .filter(|column| !binding.ignored.contains(column))
            .cloned()
            .collect();
        out.matched = structure.matched;
        out.targets = structure.hand;

        let before = grid::from_worksheet(worksheet)?;
        let outcome = paste::write(worksheet, &binding, table)?;
        let after = grid::from_worksheet(worksheet)?;
        (out.changed, out.changes) = diff::changes(&before, &after, &outcome.columns, outcome.key);
        out.line = outcome.line;
        out.notes = outcome.notes;
        Ok(())
    })();
    if let Err(error) = result {
        out.problem = Some(error.into_messages().join(" "));
    }
    out.key = binding.key;
    out.aliases = binding.aliases;
    out.ignored = binding.ignored;
    out
}

fn unbound(sheet: &str) -> MasterBinding {
    MasterBinding {
        sheet: sheet.to_string(),
        template_id: String::new(),
        kind: None,
        mode: MasterMode::Snapshot,
        key: None,
        aliases: Vec::new(),
        ignored: Vec::new(),
        auto: true,
    }
}

fn remember(
    settings: &mut MasterSettings,
    templates: &[crate::trains::model::ImportTemplate],
    template_id: &str,
    run: &MasterExportRun,
) {
    let related = suggest::related(templates, template_id);
    for binding in &mut settings.bindings {
        match run.sheets.iter().find(|sheet| sheet.sheet == binding.sheet) {
            Some(sheet) if sheet.problem.is_none() => {
                binding.template_id = template_id.to_string();
                binding.key = sheet.key.clone();
                binding.aliases = sheet.aliases.clone();
                binding.ignored = sheet.ignored.clone();
                binding.auto = false;
            }
            Some(_) => {}
            None if related.contains(&binding.template_id) => {
                binding.template_id = String::new();
                binding.auto = false;
            }
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::testing::{workbook, TempDir};
    use crate::trains::model::{CleanSummary, ColumnBinding, FieldKind, ImportPlan, MasterAlias};
    use crate::trains::sheet::layout::LayoutHint;
    use crate::trains::sheet::readers::ReaderKind;

    fn plan(columns: &[(&str, FieldKind)]) -> ImportPlan {
        ImportPlan {
            reader: ReaderKind::HeaderRow,
            layout: LayoutHint {
                header_row: Some(1),
                first_data_row: 2,
                last_data_row: None,
            },
            columns: columns
                .iter()
                .enumerate()
                .map(|(index, (header, field))| ColumnBinding {
                    header: header.to_string(),
                    index: index as u32 + 1,
                    field: *field,
                    decimal: None,
                    date_order: None,
                })
                .collect(),
            template_id: Some("t-telematik".into()),
            date1904: false,
        }
    }

    /// A filed telematics export: Wagennummer in the grouped spelling, as the
    /// cleaner writes it, an install date and a city.
    fn filed(folder: &TempDir, db: &mut TrainsDb, headers: &[&str], city: &str) -> String {
        let cleaned = workbook(
            folder,
            "assets.bereinigt.xlsx",
            &[(
                "Sheet1",
                &[headers, &["3385 0659 152-2", "20.07.2026", city]],
            )],
        );
        let fields = [
            FieldKind::Wagennummer,
            FieldKind::Datum,
            FieldKind::Ignorieren,
        ];
        let columns: Vec<(&str, FieldKind)> = headers.iter().copied().zip(fields).collect();
        let dokument = Dokument {
            id: "d-1".into(),
            name: "assets.xlsx".into(),
            sheet: "Sheet1".into(),
            template_id: "t-telematik".into(),
            template_name: "Telematik".into(),
            plan: plan(&columns),
            original_hash: "o-1".into(),
            cleaned_hash: crate::trains::dokument::hash_of(&cleaned).unwrap(),
            folder: String::new(),
            original: String::new(),
            cleaned: cleaned.to_string_lossy().into_owned(),
            summary: CleanSummary::default(),
            bereinigt_am: "2026-10-01".into(),
            importiert_am: None,
        };
        db.transaction(|tx| {
            tx.put_dokument(dokument);
            Ok(())
        })
        .unwrap();
        "d-1".into()
    }

    fn master(folder: &TempDir) -> PathBuf {
        workbook(
            folder,
            "Übersicht.xlsx",
            &[
                ("Überblick", &[&["Wagennummer"], &["#338506591522"]]),
                (
                    "Telematik",
                    &[
                        &["Asset", "", "Anbaudatum", "Stadt"],
                        &["#338506591522", "", "#45000", "Altstadt"],
                        &["#338506590011", "", "#45001", "Altstadt"],
                    ],
                ),
            ],
        )
    }

    fn setup(folder: &TempDir, headers: &[&str]) -> (TrainsDb, PathBuf) {
        let mut db = TrainsDb::load(&folder.config()).unwrap();
        let file = master(folder);
        filed(folder, &mut db, headers, "Neuhof");
        db.save_master(MasterSettings {
            file: Some(file.to_string_lossy().into_owned()),
            ..MasterSettings::default()
        })
        .unwrap();
        crate::trains::master::bindings::sync(&mut db, false).unwrap();
        let mut settings = db.master().clone();
        for binding in &mut settings.bindings {
            binding.kind = None;
        }
        db.save_master(settings).unwrap();
        (db, file)
    }

    fn request(db: &TrainsDb, choice: MasterExportChoice) -> MasterExportRequest {
        MasterExportRequest {
            dokument_id: "d-1".into(),
            base: db.master().file.clone().unwrap(),
            sheets: vec![choice],
            remember: true,
        }
    }

    fn telematik() -> MasterExportChoice {
        MasterExportChoice {
            sheet: "Telematik".into(),
            key: Some("Asset".into()),
            aliases: vec![],
            ignored: vec![],
        }
    }

    const HEADERS: [&str; 3] = ["Asset", "Anbaudatum", "Stadt"];

    // The recognition binds every sheet with a Wagen key as the overview kind,
    // paste targets included: a remembered template must still win, and come
    // first; the overview without one is offered with a warning, not blocked.
    #[test]
    fn a_remembered_overview_kind_sheet_is_suggested_first() {
        let folder = TempDir::new("export-overview");
        let (mut db, _) = setup(&folder, &HEADERS);
        let mut settings = db.master().clone();
        for binding in &mut settings.bindings {
            binding.kind = Some(crate::trains::model::SheetKind::Wagenliste);
        }
        settings.bindings[1].template_id = "t-telematik".into();
        db.save_master(settings).unwrap();

        let start = start(&mut db, "d-1").unwrap();
        assert_eq!(start.sheets[0].sheet, "Telematik");
        assert!(start.sheets[0].suggested);
        assert_eq!(start.sheets[0].warning, None);
        assert!(!start.sheets[1].suggested);
        assert!(start.sheets[1].warning.is_some());
    }

    #[test]
    fn the_sheet_bound_to_the_template_is_suggested_and_the_dashboard_is_not() {
        let folder = TempDir::new("export-suggest");
        let (mut db, _) = setup(&folder, &HEADERS);
        let mut settings = db.master().clone();
        settings.bindings[1].template_id = "t-telematik".into();
        db.save_master(settings).unwrap();

        let start = start(&mut db, "d-1").unwrap();
        let telematik = start
            .sheets
            .iter()
            .find(|s| s.sheet == "Telematik")
            .unwrap();
        assert!(telematik.suggested, "{telematik:?}");
        assert!(telematik.reason.as_deref().unwrap().contains("Vorlage"));
        let overview = start
            .sheets
            .iter()
            .find(|s| s.sheet == "Überblick")
            .unwrap();
        assert!(!overview.suggested);
        assert_eq!(start.bases.len(), 1, "no copy yet");
    }

    #[test]
    fn the_preview_lists_the_changed_cells_and_writes_nothing() {
        let folder = TempDir::new("export-preview");
        let (db, file) = setup(&folder, &HEADERS);
        let before = std::fs::read(&file).unwrap();

        let run = preview(&db, &request(&db, telematik())).unwrap();
        let sheet = &run.sheets[0];
        assert_eq!(sheet.problem, None, "{sheet:?}");
        assert!(sheet.open.is_empty() && sheet.conflicts.is_empty());
        // The snapshot leaves one row: the date and the city of row 2 change,
        // and row 3 is emptied in both written columns plus its key.
        let cells: Vec<&str> = sheet.changes.iter().map(|c| c.cell.as_str()).collect();
        assert!(cells.contains(&"C2") && cells.contains(&"D2"), "{cells:?}");
        assert!(cells.contains(&"A3"), "{cells:?}");
        let city = sheet.changes.iter().find(|c| c.cell == "D2").unwrap();
        assert_eq!(
            (city.before.as_str(), city.after.as_str()),
            ("Altstadt", "Neuhof")
        );
        assert_eq!(city.key, "338506591522");
        assert_eq!(std::fs::read(&file).unwrap(), before);
        assert_eq!(std::fs::read_dir(folder.path()).unwrap().count(), 3);
    }

    #[test]
    fn the_write_is_a_dated_copy_the_next_export_builds_on() {
        let folder = TempDir::new("export-write");
        let (mut db, file) = setup(&folder, &HEADERS);
        let before = std::fs::read(&file).unwrap();

        let wanted = request(&db, telematik());
        let run = write(&mut db, &wanted, "2026-10-04").unwrap();
        let target = PathBuf::from(run.target.unwrap());
        assert_eq!(crate::doc::file_name(&target), "Übersicht 2026-10-04.xlsx");
        assert_eq!(std::fs::read(&file).unwrap(), before, "original untouched");
        assert_eq!(db.master().last_export.as_deref(), target.to_str());

        let (bases, base) = suggest::bases(db.master()).unwrap();
        assert_eq!(bases.len(), 2);
        assert_eq!(
            base,
            target.to_string_lossy(),
            "the newer copy is the default"
        );

        let book = umya_spreadsheet::reader::xlsx::read(&target).unwrap();
        let sheet = book.sheet_by_name("Telematik").unwrap();
        assert_eq!(sheet.cell((4u32, 2u32)).unwrap().value(), "Neuhof");
    }

    #[test]
    fn the_mapping_is_remembered_on_the_binding() {
        let folder = TempDir::new("export-remember");
        let (mut db, _) = setup(&folder, &HEADERS);
        let wanted = request(&db, telematik());
        write(&mut db, &wanted, "2026-10-04").unwrap();
        let binding = db
            .master()
            .bindings
            .iter()
            .find(|binding| binding.sheet == "Telematik")
            .unwrap();
        assert_eq!(binding.template_id, "t-telematik");
        assert_eq!(binding.key.as_deref(), Some("Asset"));
        assert!(!binding.auto);
    }

    // The master renamed „Stadt“ to „Ort“: the document's column has nowhere to
    // go, which is a question until answered — by an alias or by ignoring it.
    #[test]
    fn a_column_the_master_no_longer_has_is_open_until_answered() {
        let folder = TempDir::new("export-open");
        let (db, _) = setup(&folder, &["Asset", "Anbaudatum", "Ort"]);

        let run = preview(&db, &request(&db, telematik())).unwrap();
        assert_eq!(run.sheets[0].open, ["Ort"]);
        assert!(run.sheets[0].targets.contains(&"Stadt".to_string()));

        let mut aliased = telematik();
        aliased.aliases.push(MasterAlias {
            master: "Stadt".into(),
            source: "Ort".into(),
        });
        let run = preview(&db, &request(&db, aliased)).unwrap();
        assert!(run.sheets[0].open.is_empty());
        assert!(run.sheets[0].changes.iter().any(|c| c.after == "Neuhof"));

        let mut ignoring = telematik();
        ignoring.ignored.push("Ort".into());
        let run = preview(&db, &request(&db, ignoring)).unwrap();
        assert!(run.sheets[0].open.is_empty());
        assert_eq!(run.sheets[0].ignored, ["Ort"]);
    }

    #[test]
    fn a_key_the_sheet_lost_is_dropped_and_said() {
        let folder = TempDir::new("export-key");
        let (db, _) = setup(&folder, &HEADERS);
        let mut choice = telematik();
        choice.key = Some("Wagen".into());
        let run = preview(&db, &request(&db, choice)).unwrap();
        assert_eq!(run.sheets[0].key, None);
        assert!(run.sheets[0].conflicts[0].contains("„Wagen“"));
    }

    #[test]
    fn a_base_that_is_not_the_master_is_refused() {
        let folder = TempDir::new("export-base");
        let (db, _) = setup(&folder, &HEADERS);
        let mut wrong = request(&db, telematik());
        wrong.base = folder.join("anders.xlsx").to_string_lossy().into_owned();
        let messages = preview(&db, &wrong).unwrap_err().into_messages();
        assert!(messages[0].contains("Ausgangsdatei"));
    }

    #[test]
    fn nothing_is_written_when_no_sheet_could_be() {
        let folder = TempDir::new("export-none");
        let (mut db, _) = setup(&folder, &HEADERS);
        let choice = MasterExportChoice {
            sheet: "Gibt es nicht".into(),
            key: None,
            aliases: vec![],
            ignored: vec![],
        };
        let wanted = request(&db, choice);
        let messages = write(&mut db, &wanted, "2026-10-04")
            .unwrap_err()
            .into_messages();
        assert!(messages[0].contains("kein Blatt"), "{messages:?}");
        assert_eq!(db.master().last_export, None);
    }

    #[test]
    fn a_cleaned_copy_edited_since_filing_is_refused() {
        let folder = TempDir::new("export-edited");
        let (db, _) = setup(&folder, &HEADERS);
        std::fs::write(folder.join("assets.bereinigt.xlsx"), "von Hand").unwrap();
        let messages = preview(&db, &request(&db, telematik()))
            .unwrap_err()
            .into_messages();
        assert!(messages[0].contains("verändert"), "{messages:?}");
    }
}
