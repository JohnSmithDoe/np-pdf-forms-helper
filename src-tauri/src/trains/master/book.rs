// ─── why ────────────────────────────────────────────────────────
// The master workbook opened the one way it may be: `lazy_read`, so only the
// sheets a run touches are deserialised. umya writes an undeserialised sheet
// back from its raw bytes, so on the real master 25 of 28 sheets leave the
// refresh byte-identical — measured, with the losses that remain, in
// `docs/footguns.md`. A full read of the same file took 2.6 GB.
//
// A sheet is reached through `read_sheet(index)` and `sheet_mut(index)`, NEVER
// `sheet_collection_mut()`: the latter deserialises every sheet in the book
// first, which cost the real master 4 s, a 7 s write and the byte-identical copy
// of every sheet the run never touched. `index` resolves a name here rather than
// through `read_sheet_by_name`, which unwraps an unknown one.
//
// Both umya calls run inside `AppError::reading`, because umya's parser has
// panicked on real files before, and a panic is not the German dialog.
// ────────────────────────────────────────────────────────────────

use std::path::Path;

use umya_spreadsheet::Workbook;

use crate::error::{AppError, AppResult};

pub fn open(path: &Path) -> AppResult<Workbook> {
    if !path.is_file() {
        return Err(AppError::Report(vec![format!(
            "Die Master-Datei {} gibt es nicht.",
            path.display()
        )]));
    }
    AppError::reading(unreadable(path), || {
        umya_spreadsheet::reader::xlsx::lazy_read(path)
    })
}

pub fn names(book: &Workbook) -> Vec<String> {
    book.sheet_collection_no_check()
        .iter()
        .map(|sheet| sheet.name().to_string())
        .collect()
}

pub fn index(sheets: &[String], sheet: &str) -> AppResult<usize> {
    sheets.iter().position(|name| name == sheet).ok_or_else(|| {
        AppError::Report(vec![format!(
            "Die Master-Datei hat kein Blatt „{sheet}“ mehr."
        )])
    })
}

pub fn deserialise(book: &mut Workbook, index: usize, path: &Path) -> AppResult<()> {
    AppError::reading(unreadable(path), || {
        book.read_sheet(index);
        Ok::<_, std::convert::Infallible>(())
    })
}

fn unreadable(path: &Path) -> String {
    format!(
        "Die Master-Datei {} konnte nicht gelesen werden.",
        crate::doc::file_name(path)
    )
}
