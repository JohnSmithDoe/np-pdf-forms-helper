// ─── why ────────────────────────────────────────────────────────
// The customer's master workbook is a HUB: a dashboard of VLOOKUPs over sheets
// that people fill by pasting portal exports into them. This refreshes those
// paste targets from the documents the app has filed — one binding per sheet,
// sheet ← template — and leaves everything else in the workbook alone. See
// `docs/decisions.md`, "The master is refreshed sheet by sheet,
// into a copy".
//
// THE ORIGINAL IS NEVER WRITTEN. The result is a dated copy beside it
// (`<Name> 2026-10-03.xlsx`, `free_path` for a second run the same day), so the
// one file whose loss would end the project cannot be lost here, and the user
// compares before switching. That is why no `.bak` is taken any more.
//
// The workbook is opened through `book` — `lazy_read`, one sheet at a time —
// and `.xlsm` is still refused: umya has no VBA story.
//
// Each binding takes the LATEST filed Dokument of its template, a user copy of a
// shipped template counting as that template. One sheet failing is a report line
// and the others still go in; only "no sheet could be refreshed" writes no copy,
// because a copy identical to the original would read as success.
//
// A binding is refreshed only when it names a template AND its kind is written
// back at all (`kinds::updates`): the dashboard is imported but never pasted
// into — it is formulas over the other sheets and the customer's own notes.
// ────────────────────────────────────────────────────────────────

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::{book, kinds};
use crate::error::{AppError, AppResult};
use crate::model::ClientReport;
use crate::trains::db::TrainsDb;
use crate::trains::model::{Dokument, MasterBinding};

#[derive(Debug, Default)]
pub struct Refresh {
    pub target: Option<PathBuf>,
    pub messages: Vec<String>,
    pub failed: usize,
}

impl Refresh {
    pub fn report(self) -> ClientReport {
        ClientReport {
            headline: if self.failed == 0 {
                "Master-Datei wurde aktualisiert".into()
            } else {
                "Master-Datei wurde mit Fehlern aktualisiert".into()
            },
            message_folder: self
                .target
                .as_deref()
                .and_then(Path::parent)
                .map(|folder| folder.to_string_lossy().into_owned()),
            messages: self.messages,
        }
    }
}

pub fn refresh(db: &TrainsDb, today: &str) -> AppResult<Refresh> {
    let settings = db.master();
    let Some(file) = settings.file.as_deref() else {
        return Err(AppError::Report(vec![
            "Es ist noch keine Master-Datei gewählt.".into(),
        ]));
    };
    let path = Path::new(file);
    if path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("xlsm"))
    {
        return Err(AppError::Report(vec![
            "Die Master-Datei ist eine .xlsm-Datei mit Makros.".into(),
            "Diese kann das Programm nicht schreiben, ohne die Makros zu verlieren.".into(),
        ]));
    }
    let bindings: Vec<&MasterBinding> = settings
        .bindings
        .iter()
        .filter(|binding| !binding.template_id.is_empty())
        .filter(|binding| binding.kind.is_none_or(kinds::updates))
        .collect();
    if bindings.is_empty() {
        return Err(AppError::Report(vec![
            "Es ist noch keinem Blatt der Master-Datei eine Vorlage zugeordnet.".into(),
        ]));
    }

    let mut book = book::open(path)?;
    let sheets = book::names(&book);
    let mut run = Refresh::default();
    let mut written = 0_usize;

    for binding in bindings {
        let sheet = &binding.sheet;
        let result = (|| {
            let dokument = latest(db, &binding.template_id).ok_or_else(|| {
                let template = db
                    .template(&binding.template_id)
                    .map_or_else(|| binding.template_id.clone(), |template| template.name);
                AppError::Report(vec![format!(
                    "„{sheet}“: Zur Vorlage „{template}“ gibt es noch kein bereinigtes Dokument."
                )])
            })?;
            let table = super::source::load(dokument)?;
            let index = book::index(&sheets, sheet)?;
            book::deserialise(&mut book, index, path)?;
            let worksheet = book.sheet_mut(index).map_err(|error| {
                AppError::detail(
                    format!("Das Blatt „{sheet}“ konnte nicht gelesen werden."),
                    error,
                )
            })?;
            super::paste::write(worksheet, binding, &table)
        })();
        match result {
            Ok(outcome) => {
                written += 1;
                run.messages.push(outcome.line);
                run.messages.extend(outcome.notes);
            }
            Err(error) => {
                run.failed += 1;
                run.messages.extend(error.into_messages());
            }
        }
    }

    if written == 0 {
        return Err(AppError::Report(run.messages));
    }

    let folder = path.parent().unwrap_or_else(|| Path::new("."));
    let stem = path.file_stem().map_or_else(
        || "Master".into(),
        |stem| stem.to_string_lossy().into_owned(),
    );
    let target = crate::doc::free_path(folder, &format!("{stem} {today}.xlsx"));
    crate::doc::write_book(
        &book,
        &target,
        format!(
            "Die aktualisierte Master-Datei {} konnte nicht geschrieben werden.",
            crate::doc::file_name(&target)
        ),
    )?;
    run.messages.push(format!(
        "Geschrieben als „{}“; das Original ist unverändert.",
        crate::doc::file_name(&target)
    ));
    run.target = Some(target);
    Ok(run)
}

fn latest<'a>(db: &'a TrainsDb, template_id: &str) -> Option<&'a Dokument> {
    let mut related: HashSet<String> = HashSet::from([template_id.to_string()]);
    for template in db.templates() {
        if template.origin.as_deref() == Some(template_id) {
            related.insert(template.id.clone());
        }
        if template.id == template_id {
            if let Some(origin) = template.origin {
                related.insert(origin);
            }
        }
    }
    db.dokumente()
        .into_iter()
        .filter(|dokument| related.contains(&dokument.template_id))
        .max_by(|left, right| left.bereinigt_am.cmp(&right.bereinigt_am))
        .and_then(|found| db.dokument(&found.id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{workbook, TempDir};
    use crate::trains::master::view::view;
    use crate::trains::model::MasterSettings;
    use crate::trains::model::{
        CleanSummary, ColumnBinding, FieldKind, ImportPlan, MasterBinding, MasterMode,
    };
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
    /// cleaner writes it, and one unmapped column.
    fn filed(folder: &TempDir, db: &mut TrainsDb, stamp: &str, city: &str) {
        let cleaned = workbook(
            folder,
            &format!("assets-{stamp}.bereinigt.xlsx"),
            &[(
                "Sheet1",
                &[
                    &["Asset", "Anbaudatum", "Stadt"],
                    &["3385 0659 152-2", "20.07.2026", city],
                ],
            )],
        );
        let dokument = Dokument {
            id: format!("d-{stamp}"),
            name: format!("assets-{stamp}.xlsx"),
            sheet: "Sheet1".into(),
            template_id: "t-telematik".into(),
            template_name: "Telematik".into(),
            plan: plan(&[
                ("Asset", FieldKind::Wagennummer),
                ("Anbaudatum", FieldKind::Datum),
                ("Stadt", FieldKind::Ignorieren),
            ]),
            original_hash: format!("o-{stamp}"),
            cleaned_hash: crate::trains::dokument::hash_of(&cleaned).unwrap(),
            folder: String::new(),
            original: String::new(),
            cleaned: cleaned.to_string_lossy().into_owned(),
            summary: CleanSummary::default(),
            bereinigt_am: stamp.into(),
            importiert_am: None,
        };
        db.transaction(|tx| {
            tx.put_dokument(dokument);
            Ok(())
        })
        .unwrap();
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
                        &["#338506590003", "", "#45000", "Altstadt"],
                        &["#338506590011", "", "#45001", "Altstadt"],
                    ],
                ),
            ],
        )
    }

    fn bound(db: &mut TrainsDb, file: &Path) {
        db.save_master(MasterSettings {
            file: Some(file.to_string_lossy().into_owned()),
            bindings: vec![MasterBinding {
                sheet: "Telematik".into(),
                template_id: "t-telematik".into(),
                kind: None,
                mode: MasterMode::Snapshot,
                key: None,
                aliases: vec![],
                auto: false,
            }],
            import_run: None,
            scan: None,
        })
        .unwrap();
    }

    fn read(path: &Path) -> umya_spreadsheet::Workbook {
        umya_spreadsheet::reader::xlsx::read(path).unwrap()
    }

    #[test]
    fn the_refresh_writes_a_dated_copy_and_never_the_original() {
        let folder = TempDir::new("master-copy");
        let mut db = TrainsDb::load(&folder.config()).unwrap();
        let file = master(&folder);
        let before = std::fs::read(&file).unwrap();
        filed(&folder, &mut db, "2026-10-01", "Neuhof");
        bound(&mut db, &file);

        let run = refresh(&db, "2026-10-03").unwrap();

        assert_eq!(
            std::fs::read(&file).unwrap(),
            before,
            "the original is untouched"
        );
        let target = run.target.clone().unwrap();
        assert_eq!(crate::doc::file_name(&target), "Übersicht 2026-10-03.xlsx");
        assert_eq!(run.failed, 0);

        let book = read(&target);
        let sheet = book.sheet_by_name("Telematik").unwrap();
        assert_eq!(
            sheet.cell((1u32, 2u32)).unwrap().value_number(),
            Some(338_506_591_522.0)
        );
        assert_eq!(
            sheet.cell((3u32, 2u32)).unwrap().value_number(),
            Some(46_223.0)
        );
        assert_eq!(sheet.cell((4u32, 2u32)).unwrap().value(), "Neuhof");
        assert!(
            sheet.cell((1u32, 3u32)).is_none(),
            "the stale second row is gone"
        );
        assert_eq!(
            book.sheet_by_name("Überblick")
                .unwrap()
                .cell((1u32, 2u32))
                .unwrap()
                .value(),
            "338506591522",
            "an unbound sheet is left alone"
        );
    }

    #[test]
    fn the_latest_filed_document_of_the_template_is_the_source() {
        let folder = TempDir::new("master-latest");
        let mut db = TrainsDb::load(&folder.config()).unwrap();
        let file = master(&folder);
        filed(&folder, &mut db, "2026-10-02", "Neu");
        filed(&folder, &mut db, "2026-09-01", "Alt");
        bound(&mut db, &file);

        let run = refresh(&db, "2026-10-03").unwrap();
        let book = read(run.target.as_ref().unwrap());
        assert_eq!(
            book.sheet_by_name("Telematik")
                .unwrap()
                .cell((4u32, 2u32))
                .unwrap()
                .value(),
            "Neu"
        );
    }

    #[test]
    fn a_second_run_on_the_same_day_keeps_the_first_copy() {
        let folder = TempDir::new("master-twice");
        let mut db = TrainsDb::load(&folder.config()).unwrap();
        let file = master(&folder);
        filed(&folder, &mut db, "2026-10-01", "Neuhof");
        bound(&mut db, &file);

        let first = refresh(&db, "2026-10-03").unwrap().target.unwrap();
        let second = refresh(&db, "2026-10-03").unwrap().target.unwrap();
        assert_ne!(first, second);
        assert!(first.exists() && second.exists());
    }

    #[test]
    fn a_sheet_without_a_filed_document_is_a_line_and_nothing_is_written_if_none_worked() {
        let folder = TempDir::new("master-none");
        let mut db = TrainsDb::load(&folder.config()).unwrap();
        let file = master(&folder);
        bound(&mut db, &file);

        let messages = refresh(&db, "2026-10-03").unwrap_err().into_messages();
        assert!(
            messages[0].contains("kein bereinigtes Dokument"),
            "{messages:?}"
        );
        assert_eq!(
            std::fs::read_dir(folder.path()).unwrap().count(),
            2,
            "master + data, no copy"
        );
    }

    #[test]
    fn a_cleaned_copy_edited_since_filing_is_refused() {
        let folder = TempDir::new("master-edited");
        let mut db = TrainsDb::load(&folder.config()).unwrap();
        let file = master(&folder);
        filed(&folder, &mut db, "2026-10-01", "Neuhof");
        bound(&mut db, &file);
        std::fs::write(folder.join("assets-2026-10-01.bereinigt.xlsx"), "von Hand").unwrap();

        let messages = refresh(&db, "2026-10-03").unwrap_err().into_messages();
        assert!(messages[0].contains("verändert"), "{messages:?}");
    }

    #[test]
    fn an_xlsm_master_is_refused() {
        let folder = TempDir::new("master-xlsm");
        let mut db = TrainsDb::load(&folder.config()).unwrap();
        bound(&mut db, &folder.join("master.xlsm"));
        let messages = refresh(&db, "2026-10-03").unwrap_err().into_messages();
        assert!(messages[0].contains(".xlsm"));
    }

    #[test]
    fn the_view_names_every_sheet_and_the_headers_of_the_bound_ones() {
        let folder = TempDir::new("master-view");
        let mut db = TrainsDb::load(&folder.config()).unwrap();
        let file = master(&folder);
        bound(&mut db, &file);
        crate::trains::master::bindings::sync(&mut db, false).unwrap();

        let view = view(db.master());
        assert_eq!(view.sheets, ["Überblick", "Telematik"]);
        assert_eq!(
            view.headers.len(),
            2,
            "the unbound sheet was bound by default"
        );
        let telematik = view.headers.iter().find(|sheet| sheet.name == "Telematik");
        assert_eq!(telematik.unwrap().headers, ["Asset", "Anbaudatum", "Stadt"]);
        assert_eq!(view.problem, None);
    }

    #[test]
    fn a_missing_master_is_a_problem_on_the_view_not_an_error() {
        let folder = TempDir::new("master-missing");
        let mut db = TrainsDb::load(&folder.config()).unwrap();
        bound(&mut db, &folder.join("weg.xlsx"));
        let view = view(db.master());
        assert!(view.problem.unwrap().contains("gibt es nicht"));
        assert_eq!(view.settings.bindings.len(), 1);
    }
}
