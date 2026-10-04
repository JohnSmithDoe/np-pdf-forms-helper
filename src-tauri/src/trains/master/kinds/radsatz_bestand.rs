// ─── why ────────────────────────────────────────────────────────
// The same portal report as `radsatz_einbau`, unfiltered and without positions:
// every Radsatz ever booked in, so some Wagen show five to eight "fitted" ones
// because an old set was never booked out. It is imported AFTER the fitting
// list, and the app picks no winner between the two — a Radsatz it names on the
// same Wagen with another date is a question in the walk (`entities`), and one
// the fitting list did not name arrives as an extra open Einbau without a
// position, visible for checking rather than guessed away.
//
// The same two rules as the fitting list hold: the first of each repeated
// header is the one bound, and the previous fitting's `ausbau_am` / `aus_wagen`
// stay unmapped.
// ────────────────────────────────────────────────────────────────

use crate::trains::builtin::shaped;
use crate::trains::model::{FieldKind, ImportTemplate};

pub const UPDATES: bool = true;

pub fn claims(names: &[String]) -> bool {
    super::has_all(names, &["an_wagen", "radsatzid", "radsatz", "einbau_am"])
}

pub fn template() -> ImportTemplate {
    shaped(
        super::id("radsatz-bestand"),
        "Master: Radsatz-Bestand ohne Position",
        &[
            ("an_wagen", FieldKind::Wagennummer),
            ("radsatzid", FieldKind::RadsatzSystemId),
            ("radsatz", FieldKind::Radsatznummer),
            ("einbau_am", FieldKind::EingebautAm),
            ("halter", FieldKind::Halter),
        ],
    )
}
