// ─── why ────────────────────────────────────────────────────────
// The master's dashboard: one row per Wagen, the Wagennummer stored as a number
// because every VLOOKUP in the workbook is keyed on it, and the rest lookups
// into the other sheets plus columns the customer keeps by hand. For now it
// contributes only the fleet — every Wagen, including those no other sheet
// names yet. Its hand-kept columns and fills are the Wagenmeldung phase.
//
// Never written back: pasting rows into it would replace the formulas that make
// it a dashboard and the notes nobody else has.
// ────────────────────────────────────────────────────────────────

use crate::trains::builtin::shaped;
use crate::trains::model::{FieldKind, ImportTemplate};

pub const UPDATES: bool = false;

pub fn template() -> ImportTemplate {
    shaped(
        super::id("wagenliste"),
        "Master: Wagenliste",
        &[("Wagennummer", FieldKind::Wagennummer)],
    )
}
