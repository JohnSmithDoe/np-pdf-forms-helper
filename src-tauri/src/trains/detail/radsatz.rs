// ─── why ────────────────────────────────────────────────────────
// One Radsatz: where it runs now, every Einbau newest first (each linking to
// its Wagen), and the work done to it. A Radsatznummer is not a key, so the
// spellings the senders use are shown with WHO uses them — an alias means
// „this sender calls it this“.
// ────────────────────────────────────────────────────────────────

use super::{
    date, event_lines, event_title, field, labelled, link, linked, newest_first, partner_name, row,
    section, wagen_label,
};
use crate::trains::db::TrainsDb;
use crate::trains::model::{DetailRow, EntityDetail, EntityRef};

pub fn detail(db: &TrainsDb, id: &str) -> Option<EntityDetail> {
    let radsatz = db.radsatz(id)?;
    let current = db.open_einbau(id);
    let aliases: Vec<String> = radsatz
        .aliases
        .iter()
        .map(|alias| {
            let sender = partner_name(db, alias.partner_id.as_ref());
            if sender.is_empty() {
                alias.match_key.clone()
            } else {
                format!("{} ({sender})", alias.match_key)
            }
        })
        .collect();

    let fields = [
        linked(
            "Eingebaut in",
            current
                .map(|einbau| wagen_label(db, &einbau.wagen_id))
                .unwrap_or_default(),
            current.and_then(|einbau| link(EntityRef::Wagen, &einbau.wagen_id)),
        ),
        field(
            "Position",
            current
                .and_then(|einbau| einbau.position.clone())
                .unwrap_or_default(),
        ),
        field("Bauart", radsatz.bauart.clone().unwrap_or_default()),
        field(
            "Radsatzwellennummer",
            radsatz.wellennummer.clone().unwrap_or_default(),
        ),
        field("Radsatz-ID", radsatz.system_id.clone().unwrap_or_default()),
        field("Schreibweisen", aliases.join(", ")),
        field("Bemerkung", radsatz.bemerkung.clone().unwrap_or_default()),
    ]
    .into_iter()
    .flatten()
    .collect();

    let mut einbauten: Vec<_> = db
        .einbauten()
        .into_iter()
        .filter(|einbau| einbau.radsatz_id == id)
        .collect();
    einbauten.sort_by(|left, right| right.eingebaut_am.cmp(&left.eingebaut_am));
    let einbau_rows = einbauten
        .iter()
        .map(|einbau| DetailRow {
            link: link(EntityRef::Wagen, &einbau.wagen_id),
            ..row(
                wagen_label(db, &einbau.wagen_id),
                vec![
                    einbau
                        .position
                        .as_ref()
                        .map(|position| format!("Position {position}"))
                        .unwrap_or_default(),
                    labelled("eingebaut", date(einbau.eingebaut_am.as_ref())),
                    if einbau.is_open() {
                        "läuft noch".into()
                    } else {
                        labelled("ausgebaut", date(einbau.ausgebaut_am.as_ref()))
                    },
                ],
            )
        })
        .collect();

    let mut events: Vec<_> = db
        .instandhaltungen()
        .filter(|event| event.radsatz_id.as_deref() == Some(id))
        .collect();
    newest_first(&mut events);
    let event_rows = events
        .into_iter()
        .map(|event| {
            let mut lines = event_lines(db, event);
            lines.push(wagen_label(db, &event.wagen_id));
            DetailRow {
                link: link(EntityRef::Wagen, &event.wagen_id),
                ..row(event_title(event), lines)
            }
        })
        .collect();

    Some(EntityDetail {
        kind: EntityRef::Radsatz,
        id: id.to_string(),
        title: radsatz.nummer.clone(),
        subtitle: Some(if current.is_some() {
            "eingebaut".into()
        } else {
            "nicht eingebaut".into()
        }),
        fields,
        sections: vec![
            section("Einbauten", "Noch nie eingebaut.", einbau_rows),
            section("Instandhaltungen", "Keine Instandhaltung.", event_rows),
        ],
    })
}
