// ─── why ────────────────────────────────────────────────────────
// The shape almost every sender uses: a header row somewhere near the top, data
// rows under it. Finding WHERE that header is, is the whole job.
//
// The scan stops at `MAX_HEADER_ROW`. A header further down than that is not a
// header, it is a second table inside one sheet, and guessing at those is how a
// reader picks up a total row as data.
//
// Every signal below is evidence somebody actually leaves in a file:
//   +25  three or more non-empty cells, all text — headers are words
//   +10  those cells are all distinct — a header row does not repeat itself
//   +20  three or more rows below fill half the header's columns — data
//    +8  each header word in the field lexicon, capped — the strongest signal
//        available, and the reason the lexicon is German AND English
//    +3  each column below whose cells are all one type, capped
//   −40  exactly one non-empty cell — that is a title banner, not a header
//   −25  the row below has the same type pattern — then this row is data too
//
// The weights are judgement tuned against real files and WILL change. Nothing
// should pin them: the tests here assert which row wins, never the number it
// won by. A test asserting `score == 78` gets re-pinned on every tuning pass and
// teaches nothing.
//
// Every plausible row comes back as a candidate, best first, because detection
// is a proposal — `readers::choose` alone decides whether to apply one unasked,
// and the UI shows the runner-up either way.
// ────────────────────────────────────────────────────────────────

use super::ReaderKind;
use crate::error::AppResult;
use crate::trains::sheet::grid::{Grid, RawCell};
use crate::trains::sheet::layout::{Candidate, Layout, LayoutHint};

const MAX_HEADER_ROW: u32 = 20;
const LOOKAHEAD: u32 = 10;

const LEXICON: [&str; 30] = [
    "wagen",
    "wagen",
    "wagon",
    "uic",
    "nummer",
    "nr",
    "fahrzeug",
    "datum",
    "date",
    "werkstatt",
    "werkstatt",
    "halter",
    "eigentum",
    "owner",
    "kunde",
    "kosten",
    "betrag",
    "preis",
    "eur",
    "summe",
    "leistung",
    "arbeit",
    "massnahme",
    "maßnahme",
    "bemerkung",
    "beschreibung",
    "bemerkung",
    "amount",
    "cost",
    "gattung",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum CellType {
    Empty,
    Number,
    Text,
}

fn cell_type(cell: &RawCell) -> CellType {
    if cell.is_empty() {
        CellType::Empty
    } else if cell.number.is_some() {
        CellType::Number
    } else {
        CellType::Text
    }
}

pub fn detect(grid: &Grid) -> Vec<Candidate> {
    if grid.is_empty() {
        return Vec::new();
    }
    let mut candidates: Vec<Candidate> = (1..=grid.rows.min(MAX_HEADER_ROW))
        .filter_map(|row| score(grid, row))
        .collect();
    candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.score));
    candidates.truncate(3);
    candidates
}

pub fn apply(grid: &Grid, hint: LayoutHint) -> AppResult<Layout> {
    Layout::from_hint(grid, hint)
}

fn score(grid: &Grid, row: u32) -> Option<Candidate> {
    let cells = grid.row(row);
    let filled: Vec<&RawCell> = cells.iter().filter(|cell| !cell.is_empty()).collect();
    if filled.is_empty() || row >= grid.rows {
        return None;
    }

    let mut points: i32 = 0;

    let all_text = filled.iter().all(|cell| cell.number.is_none());
    if filled.len() >= 3 && all_text {
        points += 25;
    }

    let mut seen: Vec<String> = filled
        .iter()
        .map(|cell| cell.text.trim().to_lowercase())
        .collect();
    let before = seen.len();
    seen.sort();
    seen.dedup();
    if seen.len() == before {
        points += 10;
    }

    let header_columns: Vec<u32> = cells
        .iter()
        .enumerate()
        .filter(|(_, cell)| !cell.is_empty())
        .map(|(index, _)| index as u32 + 1)
        .collect();

    let last_look = (row + LOOKAHEAD).min(grid.rows);
    let filled_rows = (row + 1..=last_look)
        .filter(|below| {
            let hits = header_columns
                .iter()
                .filter(|column| !grid.text(**column, *below).trim().is_empty())
                .count();
            hits * 2 >= header_columns.len()
        })
        .count();
    if filled_rows >= 3 {
        points += 20;
    }

    let lexicon_hits = filled
        .iter()
        .filter(|cell| {
            let text = cell.text.to_lowercase();
            LEXICON.iter().any(|word| text.contains(word))
        })
        .count();
    points += (lexicon_hits as i32 * 8).min(32);

    let homogeneous = header_columns
        .iter()
        .filter(|column| {
            let below = grid.column(**column, row + 1, last_look);
            let types: Vec<CellType> = below
                .iter()
                .map(|cell| cell_type(cell))
                .filter(|kind| *kind != CellType::Empty)
                .collect();
            types.len() >= 2 && types.windows(2).all(|pair| pair[0] == pair[1])
        })
        .count();
    points += (homogeneous as i32 * 3).min(15);

    if filled.len() == 1 {
        points -= 40;
    }

    let pattern: Vec<CellType> = cells.iter().map(cell_type).collect();
    let below_pattern: Vec<CellType> = grid.row(row + 1).iter().map(cell_type).collect();
    if pattern == below_pattern {
        points -= 25;
    }

    let score = points.clamp(0, 100) as u8;
    Some(Candidate {
        reader: ReaderKind::HeaderRow,
        reader_label: ReaderKind::HeaderRow.label().into(),
        score,
        reason: format!(
            "Kopfzeile in Zeile {row}: {} Spaltentitel, {} Datenzeile(n) darunter.",
            filled.len(),
            grid.rows - row
        ),
        hint: LayoutHint {
            header_row: Some(row),
            first_data_row: row + 1,
            last_data_row: None,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn best_row(grid: &Grid) -> Option<u32> {
        detect(grid).first().and_then(|c| c.hint.header_row)
    }

    fn ordinary() -> Grid {
        Grid::from_text(
            "Tabelle1",
            &[
                &["Wagennummer", "Datum", "Werkstatt", "Kosten"],
                &["318047401234", "31.12.2025", "Müller", "1.234,56"],
                &["218124712173", "01.01.2026", "Schmidt", "987,00"],
                &["318047401234", "02.01.2026", "Müller", "45,00"],
                &["218124712173", "03.01.2026", "Schmidt", "12,50"],
            ],
        )
    }

    #[test]
    fn finds_the_header_on_the_first_row() {
        assert_eq!(best_row(&ordinary()), Some(1));
    }

    /// The common real shape: a title, a blank, then the table.
    #[test]
    fn finds_a_header_under_a_title_banner() {
        let grid = Grid::from_text(
            "Tabelle1",
            &[
                &["Monatsliste Januar", "", "", ""],
                &["", "", "", ""],
                &["Wagennummer", "Datum", "Werkstatt", "Kosten"],
                &["318047401234", "31.12.2025", "Müller", "1.234,56"],
                &["218124712173", "01.01.2026", "Schmidt", "987,00"],
                &["318047401234", "02.01.2026", "Müller", "45,00"],
                &["218124712173", "03.01.2026", "Schmidt", "12,50"],
            ],
        );
        assert_eq!(best_row(&grid), Some(3));
    }

    /// The probe found a real header row with a blank first cell — an unnamed
    /// index column. It must not cost the row its candidacy.
    #[test]
    fn a_blank_cell_in_the_header_does_not_disqualify_it() {
        let grid = Grid::from_text(
            "Tabelle1",
            &[
                &["", "Wagennummer", "Datum", "Werkstatt"],
                &["1", "318047401234", "31.12.2025", "Müller"],
                &["2", "218124712173", "01.01.2026", "Schmidt"],
                &["3", "318047401234", "02.01.2026", "Müller"],
            ],
        );
        assert_eq!(best_row(&grid), Some(1));
    }

    #[test]
    fn a_single_cell_banner_scores_below_the_real_header() {
        let candidates = detect(&Grid::from_text(
            "Tabelle1",
            &[
                &["Monatsliste", "", "", ""],
                &["Wagennummer", "Datum", "Werkstatt", "Kosten"],
                &["318047401234", "31.12.2025", "Müller", "1.234,56"],
                &["218124712173", "01.01.2026", "Schmidt", "987,00"],
                &["318047401234", "02.01.2026", "Müller", "45,00"],
            ],
        ));
        assert_eq!(candidates[0].hint.header_row, Some(2));
    }

    #[test]
    fn every_candidate_carries_a_german_reason_naming_its_row() {
        let candidate = detect(&ordinary()).into_iter().next().unwrap();
        assert!(candidate.reason.contains("Zeile 1"), "{}", candidate.reason);
        assert!(candidate.reason.contains("Spaltentitel"));
    }

    #[test]
    fn the_last_row_is_never_a_header_because_nothing_is_under_it() {
        let candidates = detect(&ordinary());
        assert!(candidates.iter().all(|c| c.hint.header_row != Some(5)));
    }

    #[test]
    fn an_empty_grid_offers_nothing() {
        assert!(detect(&Grid::from_text("Leer", &[])).is_empty());
    }

    #[test]
    fn the_winner_is_applied_through_the_shared_layout_builder() {
        let grid = ordinary();
        let candidate = detect(&grid).into_iter().next().unwrap();
        let layout = apply(&grid, candidate.hint).unwrap();
        assert_eq!(layout.header_row, Some(1));
        assert_eq!(layout.first_data_row, 2);
        assert_eq!(layout.columns.len(), 4);
        assert_eq!(layout.columns[0].header, "Wagennummer");
    }
}
