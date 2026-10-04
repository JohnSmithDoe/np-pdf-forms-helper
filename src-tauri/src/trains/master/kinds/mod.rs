// ─── why ────────────────────────────────────────────────────────
// One module per KIND of master sheet, never per sheet. A kind is what a sheet
// IS — a fitting list with positions, the dashboard — and carries no name from
// the customer's file: the binding in `master.json` says which of the
// customer's sheets is of which kind, so a renamed sheet is a settings change
// and no firm or person ever appears in code.
//
// Each kind owns three answers, and this file only dispatches them: the
// template its sheet is read with, whether the refresh writes it back, and its
// label. Dispatch is a `match` and not a trait for the reason `doc/mod.rs`
// gives: the shared steps run in `mirror` before the kind is asked, so a kind
// cannot skip them, and an enum makes a missing answer a compile error.
// ────────────────────────────────────────────────────────────────

mod radsatz_bestand;
mod radsatz_einbau;
mod wagenliste;

use crate::trains::model::{ImportTemplate, SheetKind};

pub fn template(kind: SheetKind) -> ImportTemplate {
    match kind {
        SheetKind::Wagenliste => wagenliste::template(),
        SheetKind::RadsatzEinbau => radsatz_einbau::template(),
        SheetKind::RadsatzBestand => radsatz_bestand::template(),
    }
}

pub fn updates(kind: SheetKind) -> bool {
    match kind {
        SheetKind::Wagenliste => wagenliste::UPDATES,
        SheetKind::RadsatzEinbau => radsatz_einbau::UPDATES,
        SheetKind::RadsatzBestand => radsatz_bestand::UPDATES,
    }
}

fn id(slug: &str) -> String {
    format!("master:{slug}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trains::model::FieldKind;

    const ALL: [SheetKind; 3] = [
        SheetKind::Wagenliste,
        SheetKind::RadsatzEinbau,
        SheetKind::RadsatzBestand,
    ];

    #[test]
    fn every_kind_maps_the_wagennummer_and_names_no_firm() {
        for kind in ALL {
            let template = template(kind);
            assert!(template.plan.missing_required().is_empty(), "{kind:?}");
            assert!(template.partner_id.is_none(), "{kind:?}");
            assert!(template.id.starts_with("master:"), "{kind:?}");
        }
    }

    #[test]
    fn no_kind_is_offered_to_recognition() {
        let shipped: Vec<String> = crate::trains::builtin::all()
            .into_iter()
            .map(|template| template.id)
            .collect();
        for kind in ALL {
            assert!(!shipped.contains(&template(kind).id), "{kind:?}");
        }
    }

    // The positions are the one thing only the fitting list has; the stock
    // without them must never claim to carry any.
    #[test]
    fn only_the_fitting_list_maps_a_position() {
        let maps_position = |kind| {
            template(kind)
                .plan
                .binding(FieldKind::Einbauposition)
                .is_some()
        };
        assert!(maps_position(SheetKind::RadsatzEinbau));
        assert!(!maps_position(SheetKind::RadsatzBestand));
    }

    // The dashboard is formulas over the other sheets plus the customer's own
    // notes; pasting into it would destroy both.
    #[test]
    fn the_dashboard_is_never_written_back() {
        assert!(!updates(SheetKind::Wagenliste));
    }
}
