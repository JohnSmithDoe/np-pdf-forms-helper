// ─── why ────────────────────────────────────────────────────────
// What an export changed, read off the sheet BEFORE and AFTER the paste rather
// than predicted from the source. The paste decides rows, types and the key
// carry-over; a second prediction of all that would be a second paste to keep
// in step, and would lie the first time they disagreed. So the preview is a
// real write into an in-memory book, and this compares the two grids.
//
// The result is ROWS, because the client reads the master by rows — a Wagen's
// line — and never by cells: one `RowChange` per row that changes, carrying
// the WHOLE row in every column of the sheet so it compares by eye with Excel,
// its changed cells flagged, and the per-cell before/after for the expansion.
//
// Only the columns the paste may change are compared (`Outcome.columns`):
// formula columns are re-emitted with stale cached results on purpose, so
// comparing them would report every row of every formula as changed. For the
// same reason a column that is not written is SHOWN with its value from before
// the paste — what Excel shows today — and only a row that did not exist yet
// takes the after value there.
//
// A row is `neu` when none of its written columns nor its key held anything
// before, `geleert` when the paste emptied it (`removed`) — shown with its old
// values, and not counted in the cell total, which counts updates.
//
// A cell is compared as the user SEES it — a date-formatted serial as
// `dd.mm.yyyy`, everything else as its text — so a value whose type alone moved
// (text `180028676` to the number) is not a change worth listing. Every row is
// sent, uncapped: the preview is what will be written, and a row the user
// cannot open is a change approved unseen. How many are RENDERED at once is
// the frontend's concern (`ui/row-changes`), not a reason to withhold data.
// ────────────────────────────────────────────────────────────────

use std::collections::HashSet;

use crate::trains::model::{CellChange, ChangeColumn, RowCell, RowChange, RowChangeStatus};
use crate::trains::sanitise::{date, format};
use crate::trains::sheet::grid::{Grid, RawCell};

#[derive(Debug, Default)]
pub struct Rows {
    pub cells: u32,
    pub columns: Vec<ChangeColumn>,
    pub rows: Vec<RowChange>,
}

pub fn rows(
    before: &Grid,
    after: &Grid,
    written: &[u32],
    key: Option<u32>,
    removed: &[u32],
) -> Rows {
    let removed: HashSet<u32> = removed.iter().copied().collect();
    let width = before.cols.max(after.cols);
    let mut out = Rows {
        columns: (1..=width)
            .map(|index| ChangeColumn {
                index,
                header: header(before, after, index),
            })
            .collect(),
        ..Rows::default()
    };
    for row in 2..=before.rows.max(after.rows) {
        let differs: Vec<(u32, String, String)> = written
            .iter()
            .filter_map(|col| {
                let old = shown_as(before.cell(*col, row));
                let new = shown_as(after.cell(*col, row));
                (old != new).then_some((*col, old, new))
            })
            .collect();
        if differs.is_empty() {
            continue;
        }
        let gone = removed.contains(&row);
        if !gone {
            out.cells += differs.len() as u32;
        }
        let existed = written
            .iter()
            .chain(key.iter())
            .any(|col| !shown_as(before.cell(*col, row)).is_empty());
        let status = match (gone, existed) {
            (true, _) => RowChangeStatus::Geleert,
            (false, true) => RowChangeStatus::Geaendert,
            (false, false) => RowChangeStatus::Neu,
        };
        let key = key.map_or_else(String::new, |key| {
            let now = shown_as(after.cell(key, row));
            if now.is_empty() {
                shown_as(before.cell(key, row))
            } else {
                now
            }
        });
        let cells = (1..=width)
            .map(|col| {
                let from_before = gone || (existed && !written.contains(&col));
                RowCell {
                    text: shown_as(if from_before { before } else { after }.cell(col, row)),
                    changed: differs.iter().any(|(changed, ..)| *changed == col),
                }
            })
            .collect();
        let changes = differs
            .into_iter()
            .map(|(col, before, after)| CellChange {
                cell: format!("{}{row}", letters(col)),
                row,
                column: out.columns[(col - 1) as usize].header.clone(),
                key: key.clone(),
                before,
                after,
            })
            .collect();
        out.rows.push(RowChange {
            row,
            key,
            status,
            cells,
            changes,
        });
    }
    out
}

fn header(before: &Grid, after: &Grid, col: u32) -> String {
    match after.text(col, 1).trim() {
        "" => before.text(col, 1).trim().to_string(),
        header => header.to_string(),
    }
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
    fn a_changed_row_is_listed_whole_with_its_changed_cells_flagged() {
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
        // Column 3 is not one the paste wrote, so row 2's change there is not a change.
        let diff = rows(&before, &after, &[1, 2], Some(1), &[]);
        assert_eq!((diff.cells, diff.rows.len()), (3, 2));
        assert_eq!(diff.columns.len(), 3);
        assert_eq!(diff.columns[1].header, "Stadt");

        let changed = &diff.rows[0];
        assert_eq!((changed.row, changed.key.as_str()), (3, "2"));
        assert_eq!(changed.status, RowChangeStatus::Geaendert);
        let texts: Vec<&str> = changed
            .cells
            .iter()
            .map(|cell| cell.text.as_str())
            .collect();
        assert_eq!(texts, ["2", "Neu", ""]);
        let flags: Vec<bool> = changed.cells.iter().map(|cell| cell.changed).collect();
        assert_eq!(flags, [false, true, false]);
        assert_eq!(changed.changes.len(), 1);
        assert_eq!(changed.changes[0].cell, "B3");
        assert_eq!(changed.changes[0].column, "Stadt");
        assert_eq!(
            (
                changed.changes[0].before.as_str(),
                changed.changes[0].after.as_str()
            ),
            ("Alt", "Neu")
        );

        let new = &diff.rows[1];
        assert_eq!(new.status, RowChangeStatus::Neu);
        assert_eq!(new.changes.len(), 2);
    }

    #[test]
    fn an_unwritten_column_shows_what_excel_shows_today() {
        // A formula column re-emitted with a stale cache must not show the cache.
        let before = Grid::from_text(
            "Blatt",
            &[&["Wagen", "Stadt", "Formel"], &["1", "Alt", "42"]],
        );
        let after = Grid::from_text(
            "Blatt",
            &[&["Wagen", "Stadt", "Formel"], &["1", "Neu", "0"]],
        );
        let diff = rows(&before, &after, &[1, 2], Some(1), &[]);
        assert_eq!(diff.rows[0].cells[2].text, "42");
    }

    #[test]
    fn a_removed_row_keeps_its_old_key() {
        let before = Grid::from_text("Blatt", &[&["Wagen"], &["1"]]);
        let after = Grid::from_text("Blatt", &[&["Wagen"]]);
        let diff = rows(&before, &after, &[1], Some(1), &[]);
        assert_eq!(diff.rows[0].key, "1");
        assert_eq!(diff.rows[0].changes[0].after, "");
    }

    #[test]
    fn an_emptied_row_is_listed_with_its_old_values_and_not_counted_as_cells() {
        let before = Grid::from_text(
            "Blatt",
            &[&["Wagen", "Stadt"], &["1", "Alt"], &["2", "Alt"]],
        );
        let after = Grid::from_text("Blatt", &[&["Wagen", "Stadt"], &["", ""], &["2", "Neu"]]);
        let diff = rows(&before, &after, &[1, 2], Some(1), &[2]);
        assert_eq!((diff.cells, diff.rows.len()), (1, 2));
        let emptied = &diff.rows[0];
        assert_eq!(emptied.status, RowChangeStatus::Geleert);
        assert_eq!(emptied.key, "1");
        let texts: Vec<&str> = emptied
            .cells
            .iter()
            .map(|cell| cell.text.as_str())
            .collect();
        assert_eq!(texts, ["1", "Alt"]);
    }
}
