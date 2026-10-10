// ─── why ────────────────────────────────────────────────────────
// What a paste did to one worksheet, as cell edits for the patch writer
// (`doc::xlsx::patch`): the sheet as it was read, compared with the sheet the
// paste left in memory. The paste keeps working on umya's model, and the
// preview is still that model diffed; only the SAVING no longer lets umya
// write the workbook (decisions.md, „Die Master-Datei wird gepatcht“).
//
// A formula cell is compared by its text AND whether it is shared, never by
// its cached result: umya expands a shared child's text on read, so a group
// the paste made plain (`paste::empty_rows`) keeps every text and changes only
// that flag — and those members must be edits, or the patch writer would
// rightly refuse half a group. Comparing cached results would turn every
// untouched formula a recalculation could change into an edit.
//
// A value the paste could not have written — an error value — is refused
// rather than translated: the writer has no `t="e"` and nothing here should
// need one.
// ────────────────────────────────────────────────────────────────

use std::collections::BTreeSet;

use umya_spreadsheet::{Cell, CellRawValue, Worksheet};

use crate::doc::xlsx::patch::{CellEdit, Content};
use crate::error::{AppError, AppResult};

#[derive(Debug, PartialEq)]
enum State {
    Formula { text: String, shared: bool },
    Value(Content),
}

pub fn between(before: &Worksheet, after: &Worksheet) -> AppResult<Vec<CellEdit>> {
    let coordinates: BTreeSet<(u32, u32)> = before
        .cells()
        .into_iter()
        .chain(after.cells())
        .map(|cell| (cell.coordinate().row_num(), cell.coordinate().col_num()))
        .collect();
    let mut edits = Vec::new();
    for (row, col) in coordinates {
        let old = before.cell((col, row)).map(state).transpose()?;
        let new = after.cell((col, row)).map(state).transpose()?;
        if old == new {
            continue;
        }
        let content = match new {
            None => Content::Empty,
            Some(State::Formula { text, .. }) => Content::Formula(text),
            Some(State::Value(content)) => content,
        };
        edits.push(CellEdit { col, row, content });
    }
    Ok(edits)
}

fn state(cell: &Cell) -> AppResult<State> {
    if cell.is_formula() {
        return Ok(State::Formula {
            text: cell.formula().to_string(),
            shared: cell.formula_shared_index().is_some(),
        });
    }
    Ok(State::Value(match cell.raw_value() {
        CellRawValue::Empty => Content::Empty,
        CellRawValue::Numeric(number) => Content::Number(*number),
        CellRawValue::Bool(value) => Content::Bool(*value),
        CellRawValue::String(_) | CellRawValue::RichText(_) | CellRawValue::Lazy(_) => {
            Content::Text(cell.value().into_owned())
        }
        CellRawValue::Error(_) => {
            return Err(AppError::Report(vec![format!(
                "Die Zelle {}{} enthält einen Fehlerwert und kann nicht geschrieben werden.",
                crate::doc::xlsx::patch::letters(cell.coordinate().col_num()),
                cell.coordinate().row_num()
            )]))
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use umya_spreadsheet::structs::{CellFormula, CellFormulaValues};

    fn book() -> umya_spreadsheet::Workbook {
        umya_spreadsheet::new_file()
    }

    #[test]
    fn only_what_changed_is_an_edit_in_row_then_column_order() {
        let mut book = book();
        let sheet = book.sheet_mut(0).unwrap();
        sheet.cell_mut((1u32, 1u32)).set_value_string("Wagen");
        sheet.cell_mut((1u32, 2u32)).set_value_number(1.0);
        sheet.cell_mut((2u32, 2u32)).set_value_string("Alt");
        sheet.cell_mut((3u32, 2u32)).set_formula("A2*2");
        let before = sheet.clone();

        sheet.cell_mut((2u32, 2u32)).set_value_string("Neu");
        sheet.cell_mut((1u32, 3u32)).set_value_number(2.0);
        sheet.cell_mut((3u32, 3u32)).set_formula("A3*2");
        sheet.cell_mut((2u32, 3u32)).set_value_bool(true);
        let edits = between(&before, sheet).unwrap();

        assert_eq!(
            edits,
            [
                CellEdit {
                    col: 2,
                    row: 2,
                    content: Content::Text("Neu".into())
                },
                CellEdit {
                    col: 1,
                    row: 3,
                    content: Content::Number(2.0)
                },
                CellEdit {
                    col: 2,
                    row: 3,
                    content: Content::Bool(true)
                },
                CellEdit {
                    col: 3,
                    row: 3,
                    content: Content::Formula("A3*2".into())
                },
            ]
        );
    }

    #[test]
    fn a_blanked_cell_is_an_empty_edit_formula_and_all() {
        let mut book = book();
        let sheet = book.sheet_mut(0).unwrap();
        sheet.cell_mut((1u32, 2u32)).set_value_number(1.0);
        sheet.cell_mut((2u32, 2u32)).set_formula("A2*2");
        let before = sheet.clone();
        sheet.cell_mut((1u32, 2u32)).set_blank();
        sheet.cell_mut((2u32, 2u32)).set_blank();
        let edits = between(&before, sheet).unwrap();
        assert!(
            edits.iter().all(|edit| edit.content == Content::Empty),
            "{edits:?}"
        );
        assert_eq!(edits.len(), 2);
    }

    // The paste un-shares a group by re-setting each member's text; the text is
    // the same, only the shared flag goes — and that alone must be an edit.
    #[test]
    fn a_shared_formula_made_plain_is_an_edit_though_its_text_is_the_same() {
        let mut book = book();
        let sheet = book.sheet_mut(0).unwrap();
        let mut shared = CellFormula::default();
        shared.set_formula_type(CellFormulaValues::Shared);
        shared.set_shared_index(0);
        shared.set_text("B3");
        sheet.cell_mut((1u32, 3u32)).set_formula("B3");
        sheet
            .cell_mut((1u32, 3u32))
            .cell_value_mut()
            .set_formula_obj(shared);
        let before = sheet.clone();
        assert!(before
            .cell((1u32, 3u32))
            .unwrap()
            .formula_shared_index()
            .is_some());

        sheet.cell_mut((1u32, 3u32)).set_formula("B3");
        let edits = between(&before, sheet).unwrap();
        assert_eq!(
            edits,
            [CellEdit {
                col: 1,
                row: 3,
                content: Content::Formula("B3".into())
            }]
        );
    }

    #[test]
    fn an_untouched_sheet_has_no_edits() {
        let mut book = book();
        let sheet = book.sheet_mut(0).unwrap();
        sheet.cell_mut((1u32, 1u32)).set_value_string("x");
        sheet.cell_mut((2u32, 1u32)).set_formula("A1");
        let before = sheet.clone();
        assert!(between(&before, sheet).unwrap().is_empty());
    }
}
