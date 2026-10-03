// ─── why ────────────────────────────────────────────────────────
// The cleaned copy: the sender's workbook, read WHOLE and written back with only
// the changed cells replaced, plus an `Änderungsprotokoll` sheet with one row
// per change. Every other sheet, column, colour and filter is the sender's, so
// the copy can be handed on in place of the original.
//
// The original is never written. The copy goes to its own dated folder beside
// the exports, under a name `free_path` keeps unique, so cleaning the same file
// twice keeps both results — the folder is the record of what came in and what
// was made of it.
//
// A FULL read, not `grid::read`'s `lazy_read`: lazy reading loads one sheet,
// and writing that book back would silently drop every other one.
//
// Cleaned values are written as TEXT (`set_value_string`), never through
// `set_value`, which guesses a type and would turn a cleaned `180043025` back
// into a number. A replaced cell loses its formula, which is the point: a
// cleaned cell holds what was read, not a lookup that may answer differently
// tomorrow.
//
// Written through `doc::write_book` — temp file and rename — so a failed write
// leaves no half-written copy under the final name.
// ────────────────────────────────────────────────────────────────

use std::path::{Path, PathBuf};

use super::Change;
use crate::error::{AppError, AppResult};
use crate::trains::export::fill;
use crate::trains::model::Tier;
use crate::trains::sheet::layout::column_letter;

pub const PROTOCOL: &str = "Änderungsprotokoll";

const HEADERS: [&str; 7] = [
    "Zeile",
    "Spalte",
    "Spaltenname",
    "Original",
    "Bereinigt",
    "Stufe",
    "Regel",
];

pub fn write(
    original: &Path,
    sheet: &str,
    changes: &[Change],
    folder: &Path,
) -> AppResult<PathBuf> {
    let name = crate::doc::file_name(original);
    let mut book = AppError::reading(
        format!("Die Excel-Datei {name} konnte nicht gelesen werden."),
        || umya_spreadsheet::reader::xlsx::read(original),
    )?;

    let data = book.sheet_by_name_mut(sheet).map_err(|_| {
        AppError::Report(vec![format!(
            "Die Arbeitsmappe „{sheet}“ gibt es in {name} nicht mehr."
        )])
    })?;
    for change in changes {
        data.cell_mut((change.column, change.row))
            .set_value_string(change.clean.clone());
    }

    let protocol = free_sheet_name(&book);
    let log = book.new_sheet(&protocol).map_err(|error| {
        AppError::detail(
            "Das Änderungsprotokoll konnte nicht angelegt werden.".into(),
            error,
        )
    })?;
    let rows = changes
        .iter()
        .map(|change| {
            vec![
                change.row.to_string(),
                column_letter(change.column),
                change.header.clone(),
                change.raw.clone(),
                change.clean.clone(),
                stufe(change).to_string(),
                change.rule.clone(),
            ]
        })
        .collect();
    fill(log, &HEADERS, rows);

    std::fs::create_dir_all(folder).map_err(|error| AppError::io(folder, error))?;
    let stem = original
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Datei".into());
    let target = crate::doc::free_path(folder, &format!("{stem}.bereinigt.xlsx"));
    let headline = format!(
        "Die bereinigte Datei {} konnte nicht geschrieben werden.",
        crate::doc::file_name(&target)
    );
    crate::doc::write_book(&book, &target, headline)?;
    Ok(target)
}

fn stufe(change: &Change) -> &'static str {
    match change.tier {
        Tier::Fehler => "Fehler (korrigiert)",
        Tier::Deutung => "Deutung (bestätigt)",
        Tier::Format => "Format",
    }
}

fn free_sheet_name(book: &umya_spreadsheet::Workbook) -> String {
    let taken = |name: &str| {
        book.sheet_collection_no_check()
            .iter()
            .any(|sheet| sheet.name() == name)
    };
    if !taken(PROTOCOL) {
        return PROTOCOL.to_string();
    }
    (2..)
        .map(|number| format!("{PROTOCOL} ({number})"))
        .find(|name| !taken(name))
        .unwrap_or_else(|| PROTOCOL.to_string())
}
