// ─── why ────────────────────────────────────────────────────────
// One Partner in every role it plays: the Wagen it keeps (Halter) or owns
// (Eigentümer), and as a Werkstatt its Aufträge and the work it did — each row
// linking to its Wagen. A role the partner does not have gets no section; one
// it has but with nothing behind it says so.
// ────────────────────────────────────────────────────────────────

use super::{
    date, event_lines, event_title, field, labelled, link, newest_first, row, section, wagen_label,
};
use crate::trains::db::TrainsDb;
use crate::trains::model::{DetailRow, DetailSection, EntityDetail, EntityRef, PartnerRolle};

pub fn detail(db: &TrainsDb, id: &str) -> Option<EntityDetail> {
    let partner = db.partner_by_id(id)?;
    let rollen: Vec<&str> = partner
        .rollen
        .iter()
        .map(|rolle| match rolle {
            PartnerRolle::Halter => "Halter",
            PartnerRolle::Eigentuemer => "Eigentümer",
            PartnerRolle::Werkstatt => "Werkstatt",
        })
        .collect();

    let mut sections = Vec::new();
    if partner.has_rolle(PartnerRolle::Halter) {
        sections.push(wagen_of(db, "Wagen als Halter", |wagen| {
            wagen.halter_id.as_deref() == Some(id)
        }));
    }
    if partner.has_rolle(PartnerRolle::Eigentuemer) {
        sections.push(wagen_of(db, "Wagen als Eigentümer", |wagen| {
            wagen.eigentuemer_id.as_deref() == Some(id)
        }));
    }
    if partner.has_rolle(PartnerRolle::Werkstatt) {
        sections.push(auftraege(db, id));
        sections.push(instandhaltungen(db, id));
    }

    Some(EntityDetail {
        kind: EntityRef::Partner,
        id: id.to_string(),
        title: partner.name.clone(),
        subtitle: Some(rollen.join(", ")),
        fields: [
            field("Schreibweisen", partner.aliases.join(", ")),
            field("Bemerkung", partner.bemerkung.clone().unwrap_or_default()),
            field("Angelegt am", date(Some(&partner.created_at))),
        ]
        .into_iter()
        .flatten()
        .collect(),
        sections,
    })
}

fn wagen_of(
    db: &TrainsDb,
    title: &str,
    keeps: impl Fn(&crate::trains::model::Wagen) -> bool,
) -> DetailSection {
    let mut rows: Vec<DetailRow> = db
        .wagen_refs()
        .filter(|wagen| keeps(wagen))
        .map(|wagen| DetailRow {
            link: link(EntityRef::Wagen, &wagen.id),
            ..row(
                wagen_label(db, &wagen.id),
                vec![wagen.bauart.clone().unwrap_or_default()],
            )
        })
        .collect();
    rows.sort_by(|left, right| left.title.cmp(&right.title));
    section(title, "Kein Wagen.", rows)
}

fn auftraege(db: &TrainsDb, id: &str) -> DetailSection {
    let rows = db
        .zustand()
        .auftraege
        .iter()
        .filter(|auftrag| auftrag.werkstatt_id.as_deref() == Some(id))
        .map(|auftrag| DetailRow {
            link: link(EntityRef::Wagen, &auftrag.wagen_id),
            ..row(
                format!("Auftrag {}", auftrag.bestellnummer),
                vec![
                    wagen_label(db, &auftrag.wagen_id),
                    auftrag.status.clone().unwrap_or_default(),
                    labelled("Eingang", date(auftrag.eingang_am.as_ref())),
                    labelled("Ausgang", date(auftrag.ausgang_am.as_ref())),
                ],
            )
        })
        .collect();
    section("Werkstattaufträge", "Kein Werkstattauftrag.", rows)
}

fn instandhaltungen(db: &TrainsDb, id: &str) -> DetailSection {
    let mut events: Vec<_> = db
        .instandhaltungen()
        .filter(|event| event.werkstatt_id.as_deref() == Some(id))
        .collect();
    newest_first(&mut events);
    let rows = events
        .into_iter()
        .map(|event| {
            let mut lines = event_lines(db, event);
            lines[1] = wagen_label(db, &event.wagen_id);
            DetailRow {
                link: link(EntityRef::Wagen, &event.wagen_id),
                ..row(event_title(event), lines)
            }
        })
        .collect();
    section("Instandhaltungen", "Keine Instandhaltung.", rows)
}
