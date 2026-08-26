// ─── why ────────────────────────────────────────────────────────
// The XLSX service. `mappedName` doubles as the cell address and `address`
// parses it; because the address IS the mapped name, two cells can never share
// one, so the app's headline "same name, same value" feature is PDF-only.
// ────────────────────────────────────────────────────────────────

mod address;

use std::path::Path;

use umya_spreadsheet::Workbook;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::model::{DocumentKind, MappedDocument, MappedInput, Sheet};

pub fn add(filename: &Path) -> AppResult<DocumentKind> {
    let book = read(filename)?;
    let sheets: Vec<Sheet> = book
        .sheet_collection_no_check()
        .iter()
        .map(|sheet| Sheet {
            id: Uuid::new_v4().to_string(),
            name: sheet.name().to_string(),
        })
        .collect();
    if sheets.is_empty() {
        return Err(AppError::Report(vec![
            "Die Excel Datei enthält keine Arbeitsmappen".into(),
        ]));
    }
    Ok(DocumentKind::Xlsx { sheets })
}

pub fn create(
    source: &Path,
    target: &Path,
    document: &MappedDocument,
    inputs: &[MappedInput],
) -> AppResult<()> {
    let mut book = read(source)?;
    for field in document.mapped_fields() {
        // An empty value leaves the cell alone: exporting a blank must not wipe
        // what the template already says.
        let Some(value) =
            super::value_for(inputs, &field.orig_id).filter(|value| !value.is_empty())
        else {
            continue;
        };
        let (sheet_name, cell) = address::split(&field.mapped_name)?;
        let Ok(sheet) = book.sheet_by_name_mut(sheet_name) else {
            return Err(AppError::Report(vec![
                format!(
                    "Die Arbeitsmappe „{sheet_name}“ gibt es in {} nicht mehr.",
                    document.name
                ),
                "Bitte entferne das Dokument und füge es erneut hinzu.".into(),
            ]));
        };
        sheet.cell_mut(cell).set_value(value);
    }
    umya_spreadsheet::writer::xlsx::write(&book, target).map_err(|error| {
        AppError::detail(
            format!(
                "Die Excel-Datei {} konnte nicht geschrieben werden.",
                super::file_name(target)
            ),
            error,
        )
    })
}

// Re-linking would have to regenerate the sheet ids that `mapped[].origId`
// points at, and the mapping cannot be rebuilt from the new file alone.
pub fn remap() -> AppResult<()> {
    Err(AppError::Report(vec![
        "Excel-Dokumente können nicht neu verknüpft werden.".into(),
        "Bitte entferne das Dokument und füge es unter dem neuen Dateinamen erneut hinzu.".into(),
    ]))
}

fn read(path: &Path) -> AppResult<Workbook> {
    umya_spreadsheet::reader::xlsx::read(path).map_err(|error| {
        AppError::detail(
            format!(
                "Die Excel-Datei {} konnte nicht gelesen werden.",
                super::file_name(path)
            ),
            error,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{input, pdf_document, resource_document, TempDir};
    use std::path::PathBuf;

    /// A workbook on disk with the given sheets, optionally pre-filled.
    fn workbook(
        folder: &TempDir,
        name: &str,
        sheets: &[&str],
        filled: &[(&str, &str, &str)],
    ) -> PathBuf {
        let mut book = umya_spreadsheet::new_file();
        for sheet in sheets {
            book.new_sheet(*sheet).unwrap();
        }
        for (sheet, cell, value) in filled {
            book.sheet_by_name_mut(sheet)
                .unwrap()
                .cell_mut(*cell)
                .set_value(*value);
        }
        let path = folder.join(name);
        umya_spreadsheet::writer::xlsx::write(&book, &path).unwrap();
        path
    }

    fn cell_at(path: &Path, sheet: &str, cell: &str) -> String {
        umya_spreadsheet::reader::xlsx::read(path)
            .unwrap()
            .sheet_by_name(sheet)
            .unwrap()
            .value(cell)
    }

    /// An xlsx document mapping `origId` → cell address, the way the field
    /// dialog writes it.
    fn document(id: &str, path: &Path, mapped: &[(&str, &str)]) -> MappedDocument {
        MappedDocument {
            kind: DocumentKind::Xlsx { sheets: Vec::new() },
            ..pdf_document(id, path, mapped)
        }
    }

    // ─── add ──────────────────────────────────────────────────────

    #[test]
    fn adding_records_every_sheet_in_order() {
        let folder = TempDir::new("xlsx-add");
        let file = workbook(&folder, "liste.xlsx", &["Tabelle1", "Tabelle2"], &[]);

        let DocumentKind::Xlsx { sheets } = add(&file).unwrap() else {
            panic!("expected an xlsx kind");
        };
        // `new_file` starts with "Sheet1"; the two added follow it.
        assert_eq!(
            sheets
                .iter()
                .map(|sheet| sheet.name.as_str())
                .collect::<Vec<_>>(),
            ["Sheet1", "Tabelle1", "Tabelle2"]
        );
        assert!(sheets.iter().all(|sheet| !sheet.id.is_empty()));
    }

    #[test]
    fn a_file_that_is_not_a_workbook_is_refused_with_its_name() {
        let folder = TempDir::new("xlsx-broken");
        let fake = folder.write("kaputt.xlsx", "kein XLSX");
        let messages = add(&fake).unwrap_err().into_messages();
        assert!(messages[0].contains("kaputt.xlsx"), "{messages:?}");
    }

    // ─── create ───────────────────────────────────────────────────

    #[test]
    fn a_mapped_cell_is_filled_from_its_input() {
        let folder = TempDir::new("xlsx-create");
        let source = workbook(&folder, "liste.xlsx", &["Tabelle1"], &[]);
        let document = document("d1", &source, &[("f1", "$Tabelle1.B2")]);
        let target = folder.join("gefüllt.xlsx");

        create(&source, &target, &document, &[input("Berlin", &["f1"])]).unwrap();
        assert_eq!(cell_at(&target, "Tabelle1", "B2"), "Berlin");
    }

    // The address IS the mapped name, so two cells can never share one — the
    // app's "same name, same value" feature is PDF-only. What one input CAN do
    // is carry several ids.
    #[test]
    fn one_input_fills_every_cell_whose_id_it_carries() {
        let folder = TempDir::new("xlsx-shared");
        let source = workbook(&folder, "liste.xlsx", &["Tabelle1"], &[]);
        let document = document(
            "d1",
            &source,
            &[("f1", "$Tabelle1.A1"), ("f2", "$Tabelle1.A2")],
        );
        let target = folder.join("gefüllt.xlsx");

        create(
            &source,
            &target,
            &document,
            &[input("Berlin", &["f1", "f2"])],
        )
        .unwrap();
        assert_eq!(cell_at(&target, "Tabelle1", "A1"), "Berlin");
        assert_eq!(cell_at(&target, "Tabelle1", "A2"), "Berlin");
    }

    // An empty value leaves the cell alone: exporting a blank must not wipe
    // what the template already says. Same rule as `pdf::create`.
    #[test]
    fn an_empty_value_does_not_wipe_the_template() {
        let folder = TempDir::new("xlsx-empty");
        let source = workbook(
            &folder,
            "liste.xlsx",
            &["Tabelle1"],
            &[("Tabelle1", "A1", "Vorgabe")],
        );
        let document = document("d1", &source, &[("f1", "$Tabelle1.A1")]);
        let target = folder.join("gefüllt.xlsx");

        create(&source, &target, &document, &[input("", &["f1"])]).unwrap();
        assert_eq!(cell_at(&target, "Tabelle1", "A1"), "Vorgabe");
    }

    #[test]
    fn a_cell_with_no_input_is_left_alone() {
        let folder = TempDir::new("xlsx-no-input");
        let source = workbook(
            &folder,
            "liste.xlsx",
            &["Tabelle1"],
            &[("Tabelle1", "A1", "Vorgabe")],
        );
        let document = document("d1", &source, &[("f1", "$Tabelle1.A1")]);
        let target = folder.join("gefüllt.xlsx");

        create(&source, &target, &document, &[]).unwrap();
        assert_eq!(cell_at(&target, "Tabelle1", "A1"), "Vorgabe");
    }

    #[test]
    fn a_sheet_that_is_gone_names_itself_and_the_document() {
        let folder = TempDir::new("xlsx-missing-sheet");
        let source = workbook(&folder, "liste.xlsx", &["Tabelle1"], &[]);
        let document = document("d1", &source, &[("f1", "$Weg.A1")]);

        let error = create(
            &source,
            &folder.join("aus.xlsx"),
            &document,
            &[input("Berlin", &["f1"])],
        )
        .unwrap_err();
        let messages = error.into_messages();
        assert!(messages[0].contains("Weg"), "{messages:?}");
        assert!(messages[0].contains("liste.xlsx"), "{messages:?}");
    }

    // The address is validated before `umya` sees it, because `umya` unwraps a
    // half-parsed address and PANICS — and a panic is not an `AppError`.
    #[test]
    fn a_broken_address_is_an_error_not_a_panic() {
        let folder = TempDir::new("xlsx-bad-address");
        let source = workbook(&folder, "liste.xlsx", &["Tabelle1"], &[]);
        let document = document("d1", &source, &[("f1", "$Tabelle1.")]);

        let error = create(
            &source,
            &folder.join("aus.xlsx"),
            &document,
            &[input("Berlin", &["f1"])],
        )
        .unwrap_err();
        assert!(error.into_messages()[0].contains("ungültig"));
    }

    // The original is never the file written — `umya` parses into memory.
    #[test]
    fn creating_leaves_the_source_untouched() {
        let folder = TempDir::new("xlsx-untouched");
        let source = workbook(&folder, "liste.xlsx", &["Tabelle1"], &[]);
        let document = document("d1", &source, &[("f1", "$Tabelle1.A1")]);

        create(
            &source,
            &folder.join("aus.xlsx"),
            &document,
            &[input("Berlin", &["f1"])],
        )
        .unwrap();
        assert_eq!(cell_at(&source, "Tabelle1", "A1"), "");
    }

    // ─── remap ────────────────────────────────────────────────────

    // Re-linking would have to regenerate the sheet ids that `mapped[].origId`
    // points at, and the mapping cannot be rebuilt from the new file alone.
    #[test]
    fn remapping_is_always_refused_and_says_what_to_do_instead() {
        let messages = remap().unwrap_err().into_messages();
        assert_eq!(messages.len(), 2);
        assert!(messages[0].contains("nicht neu verknüpft"), "{messages:?}");
        assert!(messages[1].contains("erneut hinzu"), "{messages:?}");
    }

    // ─── through the dispatcher ───────────────────────────────────

    // The refusal has to survive the route the app actually takes.
    #[test]
    fn the_dispatcher_refuses_an_xlsx_remap_too() {
        let folder = TempDir::new("xlsx-remap-dispatch");
        let old_file = workbook(&folder, "alt.xlsx", &["Tabelle1"], &[]);
        let new_file = workbook(&folder, "neu.xlsx", &["Tabelle1"], &[]);
        let mut document = MappedDocument {
            kind: DocumentKind::Xlsx { sheets: Vec::new() },
            ..resource_document("d1", &old_file)
        };

        assert!(crate::doc::remap(&mut document, &new_file, &folder.config()).is_err());
        // Refused means UNTOUCHED.
        assert_eq!(document.filename, old_file.to_string_lossy());
    }
}
