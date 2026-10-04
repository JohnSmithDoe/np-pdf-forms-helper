// ─── why ────────────────────────────────────────────────────────
// The portal's list of fitted Radsätze WITH their position on the Wagen — the
// one export that says which axle a Radsatz sits on, and so the master import's
// first source of Einbauten.
//
// The header row repeats names (`einbau_am`, `ausbau_am` and `an_wagen` twice)
// and `recognise::rebind` gives a template column to the FIRST sheet column of
// that name. That is the right one each time: the second `an_wagen` holds the
// Wagen's type, the second date columns hold a constant `1`.
//
// `ausbau_am` and `aus_wagen` are NOT mapped. They describe the PREVIOUS
// fitting — the Wagen the Radsatz was taken out of before it went into
// `an_wagen` — and mapped as `AusgebautAm` they would close the current one.
// ────────────────────────────────────────────────────────────────

use crate::trains::builtin::shaped;
use crate::trains::model::{FieldKind, ImportTemplate};

pub const UPDATES: bool = true;

pub fn template() -> ImportTemplate {
    shaped(
        super::id("radsatz-einbau"),
        "Master: Radsatz-Einbauliste mit Position",
        &[
            ("an_wagen", FieldKind::Wagennummer),
            ("radsatzid", FieldKind::RadsatzSystemId),
            ("radsatz", FieldKind::Radsatznummer),
            ("einbau_am", FieldKind::EingebautAm),
            ("pos", FieldKind::Einbauposition),
            ("halter", FieldKind::Halter),
        ],
    )
}
