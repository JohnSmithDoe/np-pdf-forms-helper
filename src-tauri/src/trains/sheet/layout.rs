// ─── why ────────────────────────────────────────────────────────
// The vocabulary a reader answers in, and nothing that reads a file.
//
// A reader's whole job is "which cells are the data". It never produces values,
// which is what keeps a second reader cheap: everything downstream takes a
// `Layout` and does not care how one was arrived at.
//
// `LayoutHint` is the small, serialisable half — what a template stores and what
// the user edits when they override detection. `Layout` is what `from_hint`
// builds out of it against a real grid: the same hint against a file with one
// more column produces one more `ColumnSlot`, which is why a stored template
// survives a sender adding a field.
//
// A column with no header text gets a synthesised `Spalte D` rather than an
// empty name, because the probe found real header rows with blank cells in them
// — an unnamed index column is ordinary. A column that is empty for its whole
// data range is dropped: there is nothing to map.
//
// `Candidate` carries the German `reason` next to the score because detection is
// a PROPOSAL. The UI shows what was found and why, and `readers::choose` decides
// only whether it is clear enough to apply without asking.
// ────────────────────────────────────────────────────────────────

use serde::{Deserialize, Serialize};

use super::grid::Grid;
use super::readers::ReaderKind;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnSlot {
    pub index: u32,
    pub header: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Layout {
    pub header_row: Option<u32>,
    pub first_data_row: u32,
    pub last_data_row: u32,
    pub columns: Vec<ColumnSlot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutHint {
    pub header_row: Option<u32>,
    pub first_data_row: u32,
    pub last_data_row: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub reader: ReaderKind,
    pub reader_label: String,
    pub score: u8,
    pub reason: String,
    pub hint: LayoutHint,
}

pub fn column_letter(index: u32) -> String {
    let mut letters = Vec::new();
    let mut remaining = index;
    while remaining > 0 {
        let position = (remaining - 1) % 26;
        letters.push((b'A' + position as u8) as char);
        remaining = (remaining - 1) / 26;
    }
    letters.iter().rev().collect()
}

impl Layout {
    pub fn from_hint(grid: &Grid, hint: LayoutHint) -> AppResult<Self> {
        if grid.is_empty() {
            return Err(AppError::Report(vec![format!(
                "Die Arbeitsmappe „{}“ ist leer.",
                grid.sheet
            )]));
        }

        let last_data_row = hint.last_data_row.unwrap_or(grid.rows).min(grid.rows);
        let first_data_row = hint.first_data_row;
        if first_data_row < 1 || first_data_row > last_data_row {
            return Err(AppError::Report(vec![
                format!("Die erste Datenzeile {first_data_row} liegt nicht im Blatt."),
                format!("Das Blatt hat {} Zeilen.", grid.rows),
            ]));
        }
        if let Some(header_row) = hint.header_row {
            if header_row < 1 || header_row > grid.rows {
                return Err(AppError::Report(vec![format!(
                    "Die Kopfzeile {header_row} liegt nicht im Blatt."
                )]));
            }
        }

        let columns = (1..=grid.cols)
            .filter_map(|index| {
                let header = hint
                    .header_row
                    .map(|row| grid.text(index, row).trim().to_string())
                    .unwrap_or_default();
                let has_data = (first_data_row..=last_data_row)
                    .any(|row| grid.cell(index, row).is_some_and(|cell| !cell.is_empty()));
                if header.is_empty() && !has_data {
                    return None;
                }
                Some(ColumnSlot {
                    index,
                    header: if header.is_empty() {
                        format!("Spalte {}", column_letter(index))
                    } else {
                        header
                    },
                })
            })
            .collect::<Vec<_>>();

        if columns.is_empty() {
            return Err(AppError::Report(vec![format!(
                "In der Arbeitsmappe „{}“ wurden keine Spalten mit Daten gefunden.",
                grid.sheet
            )]));
        }

        Ok(Self {
            header_row: hint.header_row,
            first_data_row,
            last_data_row,
            columns,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> Grid {
        Grid::from_text(
            "Tabelle1",
            &[
                &["", "Wagennummer", "Datum", ""],
                &["1", "318047401234", "31.12.2025", ""],
                &["2", "218124712173", "01.01.2026", ""],
            ],
        )
    }

    fn hint(header_row: Option<u32>, first_data_row: u32) -> LayoutHint {
        LayoutHint {
            header_row,
            first_data_row,
            last_data_row: None,
        }
    }

    #[test]
    fn column_letters_carry_past_z() {
        assert_eq!(column_letter(1), "A");
        assert_eq!(column_letter(26), "Z");
        assert_eq!(column_letter(27), "AA");
        assert_eq!(column_letter(52), "AZ");
        assert_eq!(column_letter(53), "BA");
    }

    #[test]
    fn columns_take_their_names_from_the_header_row() {
        let layout = Layout::from_hint(&grid(), hint(Some(1), 2)).unwrap();
        let headers: Vec<&str> = layout
            .columns
            .iter()
            .map(|slot| slot.header.as_str())
            .collect();
        assert_eq!(headers, ["Spalte A", "Wagennummer", "Datum"]);
    }

    /// A real header row can have blank cells — an unnamed index column is
    /// ordinary, and it still holds data worth mapping.
    #[test]
    fn a_headerless_column_that_holds_data_is_kept_and_named() {
        let layout = Layout::from_hint(&grid(), hint(Some(1), 2)).unwrap();
        assert_eq!(layout.columns[0].index, 1);
        assert_eq!(layout.columns[0].header, "Spalte A");
    }

    #[test]
    fn a_column_with_neither_header_nor_data_is_dropped() {
        let layout = Layout::from_hint(&grid(), hint(Some(1), 2)).unwrap();
        assert!(layout.columns.iter().all(|slot| slot.index != 4));
    }

    #[test]
    fn a_layout_without_a_header_names_every_column_by_letter() {
        let layout = Layout::from_hint(&grid(), hint(None, 1)).unwrap();
        let headers: Vec<&str> = layout
            .columns
            .iter()
            .map(|slot| slot.header.as_str())
            .collect();
        assert_eq!(headers, ["Spalte A", "Spalte B", "Spalte C"]);
    }

    #[test]
    fn the_data_range_is_inclusive_at_both_ends() {
        let layout = Layout::from_hint(&grid(), hint(Some(1), 2)).unwrap();
        assert_eq!((layout.first_data_row, layout.last_data_row), (2, 3));
    }

    #[test]
    fn a_data_range_past_the_end_of_the_sheet_is_clamped() {
        let hint = LayoutHint {
            header_row: Some(1),
            first_data_row: 2,
            last_data_row: Some(999),
        };
        let layout = Layout::from_hint(&grid(), hint).unwrap();
        assert_eq!(layout.last_data_row, 3);
    }

    #[test]
    fn a_first_data_row_outside_the_sheet_is_refused_in_german() {
        let error = Layout::from_hint(&grid(), hint(Some(1), 99)).unwrap_err();
        assert!(error.into_messages()[0].contains("99"));
    }

    #[test]
    fn an_empty_grid_is_refused_rather_than_producing_no_columns() {
        let empty = Grid::from_text("Leer", &[]);
        assert!(Layout::from_hint(&empty, hint(None, 1)).is_err());
    }
}
