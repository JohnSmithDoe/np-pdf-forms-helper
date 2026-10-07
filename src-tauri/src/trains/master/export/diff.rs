// ─── why ────────────────────────────────────────────────────────
// Which cells an export changed, read off the sheet BEFORE and AFTER the paste
// rather than predicted from the source. The paste decides rows, types and the
// key carry-over; a second prediction of all that would be a second paste to
// keep in step, and would lie the first time they disagreed. So the preview is
// a real write into an in-memory book, and this compares the two grids.
//
// Only the columns the paste may change are compared (`Outcome.columns`):
// formula columns are re-emitted with stale cached results on purpose, so
// comparing them would report every row of every formula as changed.
//
// Rows the paste EMPTIED are not diffed — the run names them by key, and one
// line per row says more than a cleared cell per column.
//
// A cell is compared as the user SEES it — a date-formatted serial as
// `dd.mm.yyyy`, everything else as its text — so a value whose type alone moved
// (text `180028676` to the number) is not a change worth listing. The list is
// capped; `changed` still counts every cell.
// ────────────────────────────────────────────────────────────────

use std::collections::HashSet;

use crate::trains::model::CellChange;
use crate::trains::sanitise::{date, format};
use crate::trains::sheet::grid::{Grid, RawCell};

pub const SHOWN: usize = 500;

pub fn changes(
    before: &Grid,
    after: &Grid,
    columns: &[u32],
    key: Option<u32>,
    removed: &[u32],
) -> (u32, Vec<CellChange>) {
    let removed: HashSet<u32> = removed.iter().copied().collect();
    let last = before.rows.max(after.rows);
    let mut changed = 0_u32;
    let mut shown = Vec::new();
    for row in (2..=last).filter(|row| !removed.contains(row)) {
        for col in columns {
            let old = shown_as(before.cell(*col, row));
            let new = shown_as(after.cell(*col, row));
            if old == new {
                continue;
            }
            changed += 1;
            if shown.len() < SHOWN {
                let key = key.map_or_else(String::new, |key| {
                    let now = shown_as(after.cell(key, row));
                    if now.is_empty() {
                        shown_as(before.cell(key, row))
                    } else {
                        now
                    }
                });
                let header = match after.text(*col, 1).trim() {
                    "" => before.text(*col, 1).trim().to_string(),
                    header => header.to_string(),
                };
                shown.push(CellChange {
                    cell: format!("{}{row}", letters(*col)),
                    row,
                    column: header,
                    key,
                    before: old,
                    after: new,
                });
            }
        }
    }
    (changed, shown)
}

fn shown_as(cell: Option<&RawCell>) -> String {
    let Some(cell) = cell.filter(|cell| !cell.is_empty()) else {
        return String::new();
    };
    match cell.number {
        Some(serial) if cell.date_format => date::from_serial(serial, false)
            .map_or_else(|_| cell.text.trim().to_string(), format::date),
        _ => cell.text.trim().to_string(),
    }
}

pub fn letters(col: u32) -> String {
    let mut col = col;
    let mut out = Vec::new();
    while col > 0 {
        let rest = (col - 1) % 26;
        out.push(b'A' + rest as u8);
        col = (col - 1) / 26;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_are_spelled_as_excel_spells_them() {
        assert_eq!(letters(1), "A");
        assert_eq!(letters(26), "Z");
        assert_eq!(letters(27), "AA");
        assert_eq!(letters(703), "AAA");
    }

    #[test]
    fn only_changed_cells_of_the_written_columns_are_listed_with_their_key() {
        let before = Grid::from_text(
            "Blatt",
            &[
                &["Wagen", "Stadt", "Notiz"],
                &["1", "Alt", "x"],
                &["2", "Alt", ""],
            ],
        );
        let after = Grid::from_text(
            "Blatt",
            &[
                &["Wagen", "Stadt", "Notiz"],
                &["1", "Alt", "y"],
                &["2", "Neu", ""],
                &["3", "Fulda", ""],
            ],
        );
        // Column 3 is not one the paste wrote, so its change is not reported.
        let (changed, shown) = changes(&before, &after, &[1, 2], Some(1), &[]);
        assert_eq!(changed, 3);
        assert_eq!(shown[0].cell, "B3");
        assert_eq!(shown[0].key, "2");
        assert_eq!(
            (shown[0].before.as_str(), shown[0].after.as_str()),
            ("Alt", "Neu")
        );
        assert_eq!(shown[1].cell, "A4");
        assert_eq!(shown[2].column, "Stadt");
    }

    #[test]
    fn a_removed_row_keeps_its_old_key() {
        let before = Grid::from_text("Blatt", &[&["Wagen"], &["1"]]);
        let after = Grid::from_text("Blatt", &[&["Wagen"]]);
        let (_, shown) = changes(&before, &after, &[1], Some(1), &[]);
        assert_eq!(shown[0].key, "1");
        assert_eq!(shown[0].after, "");
    }

    #[test]
    fn an_emptied_row_is_not_listed_cell_by_cell() {
        let before = Grid::from_text(
            "Blatt",
            &[&["Wagen", "Stadt"], &["1", "Alt"], &["2", "Alt"]],
        );
        let after = Grid::from_text("Blatt", &[&["Wagen", "Stadt"], &["", ""], &["2", "Neu"]]);
        let (changed, shown) = changes(&before, &after, &[1, 2], Some(1), &[2]);
        assert_eq!(changed, 1);
        assert_eq!(shown[0].cell, "B3");
    }
}
