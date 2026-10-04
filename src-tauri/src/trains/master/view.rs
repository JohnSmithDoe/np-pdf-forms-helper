// ─── why ────────────────────────────────────────────────────────
// What the settings page needs and only the workbook knows: its sheet names,
// and the header row of each BOUND sheet, for the key and alias selects. A
// missing or unreadable file is a `problem` on the view, not an error, so the
// page can still show the settings that point at it.
//
// `header_row` and `fields` are shared with `sheet_view`: a header row by
// POSITION, and the field each position is read as. The sheets repeat header
// names (`einbau_am` twice, `an_wagen` twice), so a name is not an address. The
// field comes from the SAME rebind the import runs (`mirror::plan_of_headers` +
// `recognise::rebind`) with the binding's kind, or its refresh template where it
// has no kind, so "column X is field F" is decided once, in Rust.
// ────────────────────────────────────────────────────────────────

use std::collections::{HashMap, HashSet};
use std::path::Path;

use umya_spreadsheet::Worksheet;

use super::{book, kinds, mirror};
use crate::error::AppResult;
use crate::trains::db::TrainsDb;
use crate::trains::model::{
    FieldKind, ImportTemplate, MasterBinding, MasterSettings, MasterSheet, MasterView,
};
use crate::trains::recognise;

pub fn view(settings: &MasterSettings) -> MasterView {
    let mut view = MasterView {
        settings: settings.clone(),
        ..MasterView::default()
    };
    let Some(file) = settings.file.as_deref() else {
        return view;
    };
    match read_view(Path::new(file), settings) {
        Ok((sheets, headers)) => {
            view.sheets = sheets;
            view.headers = headers;
        }
        Err(error) => view.problem = error.into_messages().into_iter().next(),
    }
    view
}

fn read_view(path: &Path, settings: &MasterSettings) -> AppResult<(Vec<String>, Vec<MasterSheet>)> {
    let mut book = book::open(path)?;
    let sheets = book::names(&book);
    let bound: HashSet<&str> = settings
        .bindings
        .iter()
        .map(|binding| binding.sheet.as_str())
        .collect();
    let mut headers = Vec::new();
    for (index, name) in sheets.iter().enumerate() {
        if !bound.contains(name.as_str()) {
            continue;
        }
        book::deserialise(&mut book, index, path)?;
        let row = header_row(&book.sheet_collection_no_check()[index]);
        headers.push(MasterSheet {
            name: name.clone(),
            headers: row.into_iter().map(|(_, text)| text).collect(),
        });
    }
    Ok((sheets, headers))
}

pub(super) fn header_row(worksheet: &Worksheet) -> Vec<(u32, String)> {
    let mut row: Vec<(u32, String)> = worksheet
        .cells()
        .into_iter()
        .filter(|cell| cell.coordinate().row_num() == 1)
        .map(|cell| (cell.coordinate().col_num(), cell.value().trim().to_string()))
        .filter(|(_, text)| !text.is_empty())
        .collect();
    row.sort_by_key(|(col, _)| *col);
    row
}

pub(super) fn template_of(binding: &MasterBinding, db: &TrainsDb) -> Option<ImportTemplate> {
    match binding.kind {
        Some(kind) => Some(kinds::template(kind)),
        None => db.template(&binding.template_id),
    }
}

pub(super) fn fields(
    row: &[(u32, String)],
    template: Option<&ImportTemplate>,
) -> HashMap<u32, FieldKind> {
    template
        .map(|template| recognise::rebind(template, &mirror::plan_of_headers(row.to_vec())))
        .into_iter()
        .flat_map(|plan| plan.columns)
        .filter(|binding| binding.field != FieldKind::Ignorieren)
        .map(|binding| (binding.index, binding.field))
        .collect()
}
