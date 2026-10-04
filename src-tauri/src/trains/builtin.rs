// ─── why ────────────────────────────────────────────────────────
// The templates that ship with the program, one per kind of sender file seen so
// far. Written as Rust rather than shipped as JSON: a typo in a header list or a
// field name is then a compile error, not a template that silently never
// matches on the user's machine.
//
// THEY NAME NO FIRM. A template is the SHAPE of a file — "Telematikdaten", not
// whoever exported it — so it carries no partner, and that is safe: an alias
// learned without a sender is matched without one (`Radsatz::known_to`), so a
// second snapshot from the same source still resolves silently.
//
// Built-ins are READ-ONLY. Saving a confirmed reading against one writes a user
// copy whose `origin` points back here, and that copy shadows the built-in from
// then on (`TrainsDb::templates`). The built-in itself is never stored, so an
// update of the program can improve it without overwriting what the user made
// of it, and a reset brings it back.
//
// The master workbook's sheet kinds build their templates with `shaped` too, but
// are NOT in `all()`: they exist for the master import, which knows each sheet's
// kind from its binding, and offered to recognition the position-less Radsatz
// shape would also match every file carrying positions and make it ambiguous.
//
// The mappings are the ones `docs/decisions.md` argued for each file: the order
// feed is dated by the workshop exit, the wheelset snapshot is fittings plus the
// sender's own wheelset id, and the telematics export contributes only the
// Wagennummer until telematics has a model of its own.
// ────────────────────────────────────────────────────────────────

use super::model::{ColumnBinding, FieldKind, ImportPlan, ImportTemplate};
use super::sheet::layout::LayoutHint;
use super::sheet::readers::ReaderKind;

const PREFIX: &str = "builtin:";

pub fn all() -> Vec<ImportTemplate> {
    vec![
        template(
            "werkstattauftraege",
            "Werkstattaufträge",
            &[
                ("wagen", FieldKind::Wagennummer),
                ("empfaenger", FieldKind::Werkstatt),
                ("werk_ausg_ist", FieldKind::Datum),
                ("bemerkung_intern", FieldKind::Bemerkung),
            ],
        ),
        template(
            "radsatz-monitoring",
            "Radsatz-Monitoring",
            &[
                ("Wagennr.", FieldKind::Wagennummer),
                ("RadsatzID", FieldKind::RadsatzSystemId),
                ("Radsatznummer", FieldKind::Radsatznummer),
                ("Einbaudatum NACH letzter IS2/3", FieldKind::EingebautAm),
            ],
        ),
        template(
            "telematik",
            "Telematikdaten",
            &[("Asset", FieldKind::Wagennummer)],
        ),
    ]
}

pub fn is_builtin(id: &str) -> bool {
    id.starts_with(PREFIX)
}

fn template(slug: &str, name: &str, columns: &[(&str, FieldKind)]) -> ImportTemplate {
    shaped(format!("{PREFIX}{slug}"), name, columns)
}

pub fn shaped(id: String, name: &str, columns: &[(&str, FieldKind)]) -> ImportTemplate {
    ImportTemplate {
        id,
        name: name.into(),
        plan: ImportPlan {
            reader: ReaderKind::HeaderRow,
            layout: LayoutHint {
                header_row: Some(1),
                first_data_row: 2,
                last_data_row: None,
            },
            columns: columns
                .iter()
                .enumerate()
                .map(|(position, (header, field))| ColumnBinding {
                    header: (*header).into(),
                    index: position as u32 + 1,
                    field: *field,
                    decimal: None,
                    date_order: None,
                })
                .collect(),
            template_id: None,
            date1904: false,
        },
        partner_id: None,
        origin: None,
        builtin: true,
        created_at: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trains::recognise;

    fn headers(names: &[&str]) -> Vec<ColumnBinding> {
        names
            .iter()
            .enumerate()
            .map(|(position, header)| ColumnBinding {
                header: (*header).into(),
                index: position as u32 + 1,
                field: FieldKind::Ignorieren,
                decimal: None,
                date_order: None,
            })
            .collect()
    }

    fn matched(names: &[&str]) -> Vec<String> {
        let templates = all();
        recognise::matching(&templates, &headers(names))
            .into_iter()
            .map(|template| template.id.clone())
            .collect()
    }

    #[test]
    fn every_builtin_is_marked_and_prefixed() {
        for template in all() {
            assert!(template.builtin, "{}", template.id);
            assert!(is_builtin(&template.id), "{}", template.id);
            assert!(template.partner_id.is_none(), "{}", template.id);
            assert!(
                template.plan.missing_required().is_empty(),
                "{}",
                template.id
            );
        }
    }

    /// The header row of the order export, with columns nobody imports.
    #[test]
    fn the_order_feed_is_recognised() {
        assert_eq!(
            matched(&[
                "bestellnr",
                "sachb",
                "wagen",
                "eigentuemer",
                "best_datum",
                "empfaenger",
                "werk_ausg_ist",
                "status",
                "bemerkung_intern",
            ]),
            vec!["builtin:werkstattauftraege"]
        );
    }

    #[test]
    fn the_wheelset_snapshot_is_recognised() {
        assert_eq!(
            matched(&[
                "Kunde",
                "Wagennr.",
                "Vertragsnr.",
                "RadsatzID",
                "Radsatznummer",
                "Radsatzbauart",
                "Einbaudatum NACH letzter IS2/3",
            ]),
            vec!["builtin:radsatz-monitoring"]
        );
    }

    #[test]
    fn the_telematics_export_is_recognised() {
        assert_eq!(
            matched(&["Asset", "Anbaudatum", "Timestamp", "Laufleistung"]),
            vec!["builtin:telematik"]
        );
    }

    /// The snapshot's second sheet lists wagons, not wheelsets, and no shipped
    /// template claims it.
    #[test]
    fn a_sheet_of_a_known_file_can_still_be_unknown() {
        assert!(matched(&["Wagennr.", "Wagentyp", "Anzahl Achsen"]).is_empty());
    }
}
