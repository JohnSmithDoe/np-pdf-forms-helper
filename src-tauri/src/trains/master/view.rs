// ─── why ────────────────────────────────────────────────────────
// What the settings page needs and only the workbook knows: its sheet names,
// and the header row of each BOUND sheet, for the key and alias selects. Both
// come from the SCAN stored in the settings by the last mapping or import
// (`bindings::sync`), never from the workbook: only mapping, import and a sheet
// view read the file, and reading a header means deserialising a whole sheet. A missing file is a `problem` on the view, not
// an error, so the page can still show the settings that point at it.
//
// `header_row` and `fields` are shared with `sheet_view`: a header row by
// POSITION, and the field each position is read as. The sheets repeat header
// names (`einbau_am` twice, `an_wagen` twice), so a name is not an address. The
// field comes from the SAME rebind the import runs (`mirror::plan_of_headers` +
// `recognise::rebind`) with the binding's kind, or its template where it
// has no kind, so "column X is field F" is decided once, in Rust.
// ────────────────────────────────────────────────────────────────

use std::collections::HashMap;
use std::path::Path;

use umya_spreadsheet::Worksheet;

use super::{kinds, mirror};
use crate::trains::db::TrainsDb;
use crate::trains::model::{FieldKind, ImportTemplate, MasterBinding, MasterSettings, MasterView};
use crate::trains::recognise;

pub fn view(settings: &MasterSettings) -> MasterView {
    let mut view = MasterView {
        settings: settings.clone(),
        ..MasterView::default()
    };
    let Some(file) = settings.file.as_deref() else {
        return view;
    };
    if !Path::new(file).is_file() {
        view.problem = Some(format!("Die Master-Datei {file} gibt es nicht."));
        return view;
    }
    let Some(scan) = &settings.scan else {
        view.problem = Some("Die Master-Datei wurde noch nicht gelesen.".into());
        return view;
    };
    view.sheets = scan.sheets.iter().map(|sheet| sheet.name.clone()).collect();
    view.headers = scan
        .sheets
        .iter()
        .filter(|sheet| {
            settings
                .bindings
                .iter()
                .any(|binding| binding.sheet == sheet.name)
        })
        .cloned()
        .collect();
    view
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

pub(super) fn template_of(
    binding: &MasterBinding,
    db: &TrainsDb,
    header: &[(u32, String)],
) -> Option<ImportTemplate> {
    match binding.kind {
        Some(kind) => {
            let names: Vec<String> = header.iter().map(|(_, text)| text.clone()).collect();
            Some(kinds::template(kind, &names))
        }
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
