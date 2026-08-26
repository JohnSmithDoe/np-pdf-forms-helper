// ─── why ────────────────────────────────────────────────────────
// The reader set, as an ENUM and a match rather than a trait and a registry.
// `doc/mod.rs` reaches the same answer for a different reason, so this is
// reconciled with it rather than copied from it. Three reasons specific here:
//
//   • the set has to be ENUMERABLE and exhaustively matchable. Detection is a
//     fold over `ALL`, and an exhaustive match is what tells you at compile time
//     that a new reader is missing from the override list the user picks from.
//   • the choice is PERSISTED, in `ImportTemplate.reader`, and comes back months
//     later. An enum serialises as `"headerRow"` for free and a removed variant
//     fails to deserialise loudly; a trait object needs a string key, a registry
//     lookup and an "unknown reader" failure path invented purely by the
//     abstraction.
//   • a reader is a pure function — `detect(&Grid)` and `apply(&Grid, hint)` —
//     with no state for `&self` to be. A trait would be a namespace with extra
//     steps.
//
// The abstraction the seam actually provides is `Grid → Layout`, not the
// dispatch: what makes a reader cheap to add is that the type on each side is
// fixed and small. A one-record-per-sheet form becomes one file under here, one
// match arm, and one entry in `ALL`.
//
// `Manual` is a real variant whose `detect` answers `None` and whose `apply`
// just reads the user's hint. So "the user overrode detection" and "a reader
// detected it" travel the same code path — an override on its own path is the
// path nobody tests.
//
// `apply` stays on the enum although both arms currently build the same
// rectangle. A reader whose data is not a rectangle has to synthesise its own
// column set, and that is the arm this shape is holding open.
//
// DETECTION IS A PROPOSAL. `detect` returns every candidate it found, best
// first, and the UI shows the runner-up beside the winner. `choose` is the only
// thing that decides whether to apply one unasked, and it refuses on a weak best
// or a close second — the weights are tuned against real files, so a near-tie
// means the file is genuinely ambiguous and the user is the one to ask.
// ────────────────────────────────────────────────────────────────

pub mod header_row;
pub mod manual;

use serde::{Deserialize, Serialize};

use super::grid::Grid;
use super::layout::{Candidate, Layout, LayoutHint};
use crate::error::AppResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReaderKind {
    HeaderRow,
    Manual,
}

pub const ALL: [ReaderKind; 2] = [ReaderKind::HeaderRow, ReaderKind::Manual];

const MIN_SCORE: u8 = 45;
const MIN_MARGIN: u8 = 8;

impl ReaderKind {
    pub fn label(self) -> &'static str {
        match self {
            ReaderKind::HeaderRow => "Kopfzeile mit Datenzeilen",
            ReaderKind::Manual => "Manuell festgelegt",
        }
    }

    pub fn detect(self, grid: &Grid) -> Vec<Candidate> {
        match self {
            ReaderKind::HeaderRow => header_row::detect(grid),
            ReaderKind::Manual => manual::detect(),
        }
    }

    pub fn apply(self, grid: &Grid, hint: LayoutHint) -> AppResult<Layout> {
        match self {
            ReaderKind::HeaderRow => header_row::apply(grid, hint),
            ReaderKind::Manual => manual::apply(grid, hint),
        }
    }
}

pub fn detect(grid: &Grid) -> Vec<Candidate> {
    let mut candidates: Vec<Candidate> = ALL
        .into_iter()
        .flat_map(|reader| reader.detect(grid))
        .collect();
    candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.score));
    candidates
}

pub fn choose(candidates: &[Candidate]) -> Option<&Candidate> {
    let best = candidates.first()?;
    if best.score < MIN_SCORE {
        return None;
    }
    match candidates.get(1) {
        Some(second) if best.score - second.score < MIN_MARGIN => None,
        _ => Some(best),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(score: u8) -> Candidate {
        Candidate {
            reader: ReaderKind::HeaderRow,
            reader_label: ReaderKind::HeaderRow.label().into(),
            score,
            reason: String::new(),
            hint: LayoutHint {
                header_row: Some(1),
                first_data_row: 2,
                last_data_row: None,
            },
        }
    }

    #[test]
    fn a_clear_winner_is_chosen() {
        let candidates = [candidate(80), candidate(30)];
        assert_eq!(choose(&candidates).unwrap().score, 80);
    }

    #[test]
    fn a_weak_best_is_refused_rather_than_applied() {
        assert!(choose(&[candidate(MIN_SCORE - 1)]).is_none());
    }

    /// A near-tie means the file is ambiguous, and the user is the one to ask.
    #[test]
    fn a_close_second_is_refused() {
        assert!(choose(&[candidate(80), candidate(80 - MIN_MARGIN + 1)]).is_none());
        assert!(choose(&[candidate(80), candidate(80 - MIN_MARGIN)]).is_some());
    }

    #[test]
    fn nothing_found_chooses_nothing() {
        assert!(choose(&[]).is_none());
    }

    /// Every variant answers both halves of the seam, so adding one cannot
    /// leave the override list or the dispatch behind.
    #[test]
    fn every_reader_is_labelled_and_dispatches() {
        let grid = Grid::from_text("Tabelle1", &[&["Wagennummer"], &["318047401234"]]);
        for reader in ALL {
            assert!(!reader.label().is_empty());
            let _ = reader.detect(&grid);
        }
    }
}
