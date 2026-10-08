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
// feed is a list of Werkstattaufträge (no Instandhaltung any more — „Der
// Wagen-Zustand ist typisiert“), the wheelset snapshot is fittings plus the
// sender's own wheelset id, and the telematics export is a device and its
// reading per Wagen. `vers_datum` in the order feed stays unmapped: it is when
// the WAGEN was sent, not the order. The P8 list and the revision report
// (`PowerBI` in the customer's master) are Prüfungen; the P8 list carries no Art
// column, so its template carries the Art (`ImportPlan.pruefart`).
//
// `master_hint` is what a template knows about its sheet in the customer's
// master — the SHAPE again, never a sheet name, which lives only in
// `master.json`: the column that identifies a row there, and the headers the
// master spells differently (`RadsatzID` is `Radsatz ID`). The wheelset
// snapshot needs both: its sheet holds one row per Radsatz, so the Wagennummer
// would never be a unique key, and without the respelling the sheet is not
// even recognised as the template's.
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
                ("bestellnr", FieldKind::Bestellnummer),
                ("wagen", FieldKind::Wagennummer),
                ("best_datum", FieldKind::AuftragErfasstAm),
                ("empfaenger", FieldKind::Werkstatt),
                ("eingang_ist", FieldKind::AuftragEingangAm),
                ("werk_ausg_ist", FieldKind::AuftragAusgangAm),
                ("status", FieldKind::AuftragStatus),
                ("bemerkung_intern", FieldKind::AuftragBemerkung),
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
            &[
                ("Asset", FieldKind::Wagennummer),
                ("Anbaudatum", FieldKind::TelematikAngebautAm),
                ("Timestamp", FieldKind::TelematikZeitpunkt),
                ("Energie-Reserve", FieldKind::TelematikEnergie),
                ("Pointer Name", FieldKind::TelematikGeraet),
                ("Stadt", FieldKind::TelematikStadt),
                ("Land", FieldKind::TelematikLand),
                ("Summe Laufleistung", FieldKind::TelematikLaufleistung),
                ("Standort", FieldKind::TelematikStandort),
                ("AccStatus Text", FieldKind::TelematikBewegung),
            ],
        ),
        pruefart(
            template(
                "p8",
                "P8-Fälligkeiten",
                &[
                    ("TRANSPORTMITTELNR", FieldKind::Wagennummer),
                    ("TERMIN", FieldKind::PruefungFaelligAm),
                    ("BESTELLNUMMER", FieldKind::Bestellnummer),
                    ("STATUS", FieldKind::PruefungStatus),
                ],
            ),
            "P8",
        ),
        template(
            "revision",
            "Revisionen",
            &[
                ("Wagennummer", FieldKind::Wagennummer),
                ("Prüfungsstatus", FieldKind::PruefungStatus),
                ("Prüfungsart", FieldKind::Pruefart),
                ("Bestellnummer", FieldKind::Bestellnummer),
                ("Fälligkeitstermin", FieldKind::PruefungFaelligAm),
                ("Plandatum", FieldKind::PruefungGeplantAm),
                ("Eingang in Werkstatt", FieldKind::AuftragEingangAm),
                ("Ausgang aus Werkstatt", FieldKind::PruefungDurchgefuehrtAm),
            ],
        ),
    ]
}

pub struct MasterHint {
    pub key: &'static str,
    pub renamed: &'static [(&'static str, &'static str)],
}

pub fn master_hint(id: &str) -> Option<MasterHint> {
    match id.strip_prefix(PREFIX)? {
        "radsatz-monitoring" => Some(MasterHint {
            key: "RadsatzID",
            renamed: &[("RadsatzID", "Radsatz ID")],
        }),
        _ => None,
    }
}

pub fn is_builtin(id: &str) -> bool {
    id.starts_with(PREFIX)
}

fn pruefart(mut template: ImportTemplate, art: &str) -> ImportTemplate {
    template.plan.pruefart = Some(art.into());
    template
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
            pruefart: None,
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
                "nwb",
                "wagen",
                "tm_typ",
                "eigentuemer",
                "best_datum",
                "versender",
                "vers_datum",
                "empfaenger",
                "eingang_ist",
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
            matched(&[
                "Asset",
                "Anbaudatum",
                "Timestamp",
                "Energie-Reserve",
                "Asset Typ",
                "Pointer Name",
                "Stadt",
                "Land",
                "Laufleistung",
                "Summe Laufleistung",
                "Standort",
                "AccStatus Text",
            ]),
            vec!["builtin:telematik"]
        );
    }

    // The P8 list has no Art column; the template supplies it.
    #[test]
    fn the_p8_list_is_recognised_and_carries_its_art() {
        assert_eq!(
            matched(&[
                "TRANSPORTMITTELNR",
                "TERMIN",
                "BESTELLNUMMER",
                "ERFASSUNGSDATUM",
                "STATUS",
                "Durchgeführt",
            ]),
            vec!["builtin:p8"]
        );
        let p8 = all()
            .into_iter()
            .find(|template| template.id == "builtin:p8")
            .unwrap();
        assert_eq!(p8.plan.pruefart.as_deref(), Some("P8"));
    }

    /// The snapshot's second sheet lists wagons, not wheelsets, and no shipped
    /// template claims it.
    #[test]
    fn a_sheet_of_a_known_file_can_still_be_unknown() {
        assert!(matched(&["Wagennr.", "Wagentyp", "Anzahl Achsen"]).is_empty());
    }
}
