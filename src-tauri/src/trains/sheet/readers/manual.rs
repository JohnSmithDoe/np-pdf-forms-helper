// ─── why ────────────────────────────────────────────────────────
// The user's own answer, as a real reader rather than a bypass.
//
// `detect` returns nothing, so this is never chosen automatically — it exists to
// be picked. `apply` takes the hint the user edited and builds the same `Layout`
// through the same builder every other reader uses.
//
// That is the entire point of it being a variant. If overriding detection went
// down its own code path, that path would be the one nothing else exercises, and
// the difference between "the reader found this" and "the user said this" would
// leak into every stage downstream. Here there is no difference to leak.
// ────────────────────────────────────────────────────────────────

use crate::error::AppResult;
use crate::trains::sheet::grid::Grid;
use crate::trains::sheet::layout::{Candidate, Layout, LayoutHint};

pub fn detect() -> Vec<Candidate> {
    Vec::new()
}

pub fn apply(grid: &Grid, hint: LayoutHint) -> AppResult<Layout> {
    Layout::from_hint(grid, hint)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_never_proposes_itself() {
        assert!(detect().is_empty());
    }

    /// The user pointing at a header row lands in the same place a reader would
    /// have — same builder, same result.
    #[test]
    fn the_users_hint_builds_the_same_layout_a_reader_would() {
        let grid = Grid::from_text(
            "Tabelle1",
            &[
                &["Müll", "", ""],
                &["Wagennummer", "Datum", "Werkstatt"],
                &["318047401234", "31.12.2025", "Müller"],
            ],
        );
        let hint = LayoutHint {
            header_row: Some(2),
            first_data_row: 3,
            last_data_row: None,
        };
        let manual = apply(&grid, hint).unwrap();
        let via_reader = super::super::header_row::apply(&grid, hint).unwrap();
        assert_eq!(manual, via_reader);
        assert_eq!(manual.columns[0].header, "Wagennummer");
    }

    /// A sheet with no header at all is still readable — the columns get
    /// letters, and every row is data.
    #[test]
    fn a_layout_with_no_header_row_is_legal() {
        let grid = Grid::from_text("Tabelle1", &[&["318047401234", "31.12.2025"]]);
        let layout = apply(
            &grid,
            LayoutHint {
                header_row: None,
                first_data_row: 1,
                last_data_row: None,
            },
        )
        .unwrap();
        assert_eq!(layout.header_row, None);
        assert_eq!(layout.columns[0].header, "Spalte A");
    }
}
