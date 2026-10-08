// ─── why ────────────────────────────────────────────────────────
// Everything stored about ONE Wagen, in the order a dispatcher reads it: who
// keeps and owns it, where it is (Telematik), what is fitted, what is open
// (Schäden, Aufträge, Prüfungen), then the history (past fittings, work done).
// Open Schadensmeldungen come before closed ones and carry `warning` — the
// orange row of the customer's dashboard; an open Auftrag (no Ausgang) is
// `medium`. Rows that name a Radsatz or a Partner link to it. Fitted Radsätze
// go by position, „ohne Position“ LAST — a master sheet without positions adds
// such fittings beside the placed ones, and they are the odd ones out.
// ────────────────────────────────────────────────────────────────

use super::{
    date, event_lines, event_title, field, labelled, link, linked, moment, newest_first, overdue,
    partner_name, row, section, uic,
};
use crate::trains::db::TrainsDb;
use crate::trains::model::{
    DetailField, DetailRow, DetailSection, DetailTone, Einbau, EntityDetail, EntityRef, Wagen,
};

pub fn detail(db: &TrainsDb, id: &str) -> Option<EntityDetail> {
    let wagen = db.wagen_by_id(id)?;
    let einbauten: Vec<Einbau> = db
        .einbauten()
        .into_iter()
        .filter(|einbau| einbau.wagen_id == id)
        .collect();
    Some(EntityDetail {
        kind: EntityRef::Wagen,
        id: id.to_string(),
        title: uic(db, &wagen.nummer),
        subtitle: wagen.bauart.clone(),
        fields: fields(db, wagen),
        sections: vec![
            telematik(db, id),
            fitted(db, &einbauten, true),
            schaeden(db, id),
            auftraege(db, id),
            pruefungen(db, id),
            fitted(db, &einbauten, false),
            instandhaltungen(db, id),
        ],
    })
}

fn fields(db: &TrainsDb, wagen: &Wagen) -> Vec<DetailField> {
    [
        linked(
            "Halter",
            partner_name(db, wagen.halter_id.as_ref()),
            wagen
                .halter_id
                .as_deref()
                .and_then(|id| link(EntityRef::Partner, id)),
        ),
        linked(
            "Eigentümer",
            partner_name(db, wagen.eigentuemer_id.as_ref()),
            wagen
                .eigentuemer_id
                .as_deref()
                .and_then(|id| link(EntityRef::Partner, id)),
        ),
        field("Bauart", wagen.bauart.clone().unwrap_or_default()),
        field("Bemerkung", wagen.bemerkung.clone().unwrap_or_default()),
        field("Angelegt am", date(Some(&wagen.created_at))),
        field(
            "Quelle",
            wagen
                .source
                .as_ref()
                .map(|source| format!("{}, {}, Zeile {}", source.file, source.sheet, source.row))
                .unwrap_or_default(),
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn telematik(db: &TrainsDb, id: &str) -> DetailSection {
    let zustand = db.zustand();
    let mut rows: Vec<DetailRow> = zustand
        .meldungen
        .iter()
        .filter(|meldung| meldung.wagen_id == id)
        .map(|meldung| {
            row(
                format!("Letzte Meldung {}", moment(&meldung.zeitpunkt)),
                vec![
                    [
                        meldung.standort.as_ref().or(meldung.stadt.as_ref()),
                        meldung.land.as_ref(),
                    ]
                    .into_iter()
                    .flatten()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", "),
                    meldung.bewegung.clone().unwrap_or_default(),
                    meldung
                        .laufleistung_km
                        .map(|km| format!("{km} km"))
                        .unwrap_or_default(),
                    meldung
                        .energie_prozent
                        .map(|percent| format!("Energie {percent} %"))
                        .unwrap_or_default(),
                ],
            )
        })
        .collect();
    rows.extend(
        zustand
            .geraete
            .iter()
            .filter(|geraet| geraet.wagen_id == id)
            .map(|geraet| {
                row(
                    format!("Gerät {}", geraet.kennung),
                    vec![labelled("angebaut am", date(geraet.angebaut_am.as_ref()))],
                )
            }),
    );
    section("Telematik", "Keine Telematik-Meldung.", rows)
}

fn fitted(db: &TrainsDb, einbauten: &[Einbau], open: bool) -> DetailSection {
    let mut chosen: Vec<&Einbau> = einbauten
        .iter()
        .filter(|einbau| einbau.is_open() == open)
        .collect();
    chosen.sort_by(|left, right| {
        if open {
            (left.position.is_none(), &left.position)
                .cmp(&(right.position.is_none(), &right.position))
        } else {
            right.ausgebaut_am.cmp(&left.ausgebaut_am)
        }
    });
    let rows = chosen
        .into_iter()
        .map(|einbau| {
            let nummer = db
                .radsatz(&einbau.radsatz_id)
                .map(|radsatz| radsatz.nummer.clone())
                .unwrap_or_else(|| "unbekannter Radsatz".into());
            DetailRow {
                link: link(EntityRef::Radsatz, &einbau.radsatz_id),
                ..row(
                    nummer,
                    vec![
                        einbau
                            .position
                            .as_ref()
                            .map(|position| format!("Position {position}"))
                            .unwrap_or_else(|| "ohne Position".into()),
                        labelled("eingebaut", date(einbau.eingebaut_am.as_ref())),
                        labelled("ausgebaut", date(einbau.ausgebaut_am.as_ref())),
                    ],
                )
            }
        })
        .collect();
    if open {
        section("Eingebaute Radsätze", "Kein Radsatz eingebaut.", rows)
    } else {
        section("Frühere Radsätze", "Kein Radsatz ausgebaut.", rows)
    }
}

fn schaeden(db: &TrainsDb, id: &str) -> DetailSection {
    let mut schaeden: Vec<_> = db
        .zustand()
        .schaeden
        .iter()
        .filter(|schaden| schaden.wagen_id == id)
        .collect();
    schaeden.sort_by_key(|schaden| (schaden.erledigt_am.is_some(), schaden.gemeldet_am.clone()));
    let rows = schaeden
        .into_iter()
        .map(|schaden| {
            let open = schaden.erledigt_am.is_none();
            DetailRow {
                tone: open.then_some(DetailTone::Warning),
                ..row(
                    format!(
                        "Schaden {}{}",
                        schaden.schadcode.clone().unwrap_or_default(),
                        if open { " · offen" } else { " · erledigt" }
                    ),
                    vec![
                        labelled("gemeldet am", date(schaden.gemeldet_am.as_ref())),
                        labelled("von", schaden.gemeldet_von.clone().unwrap_or_default()),
                        schaden.notiz.clone().unwrap_or_default(),
                        if schaden.ausgesetzt == Some(true) {
                            "ausgesetzt".into()
                        } else {
                            String::new()
                        },
                        if schaden.beladen == Some(true) {
                            "beladen".into()
                        } else {
                            String::new()
                        },
                        schaden.aktion.clone().unwrap_or_default(),
                        labelled(
                            "ausführend",
                            schaden.ausfuehrender.clone().unwrap_or_default(),
                        ),
                        labelled("geplant", date(schaden.geplant_am.as_ref())),
                        labelled("erledigt am", date(schaden.erledigt_am.as_ref())),
                    ],
                )
            }
        })
        .collect();
    section("Schadensmeldungen", "Keine Schadensmeldung.", rows)
}

fn auftraege(db: &TrainsDb, id: &str) -> DetailSection {
    let mut auftraege: Vec<_> = db
        .zustand()
        .auftraege
        .iter()
        .filter(|auftrag| auftrag.wagen_id == id)
        .collect();
    auftraege.sort_by_key(|auftrag| (auftrag.ausgang_am.is_some(), auftrag.bestellnummer.clone()));
    let rows = auftraege
        .into_iter()
        .map(|auftrag| DetailRow {
            link: auftrag
                .werkstatt_id
                .as_deref()
                .and_then(|id| link(EntityRef::Partner, id)),
            tone: auftrag.ausgang_am.is_none().then_some(DetailTone::Medium),
            ..row(
                format!("Auftrag {}", auftrag.bestellnummer),
                vec![
                    auftrag.status.clone().unwrap_or_default(),
                    partner_name(db, auftrag.werkstatt_id.as_ref()),
                    labelled("erfasst", date(auftrag.erfasst_am.as_ref())),
                    labelled("versendet", date(auftrag.versendet_am.as_ref())),
                    labelled("Eingang", date(auftrag.eingang_am.as_ref())),
                    labelled("Ausgang", date(auftrag.ausgang_am.as_ref())),
                    auftrag.bemerkung.clone().unwrap_or_default(),
                ],
            )
        })
        .collect();
    section("Werkstattaufträge", "Kein Werkstattauftrag.", rows)
}

fn pruefungen(db: &TrainsDb, id: &str) -> DetailSection {
    let mut pruefungen: Vec<_> = db
        .zustand()
        .pruefungen
        .iter()
        .filter(|pruefung| pruefung.wagen_id == id)
        .collect();
    pruefungen.sort_by(|left, right| left.faellig_am.cmp(&right.faellig_am));
    let rows = pruefungen
        .into_iter()
        .map(|pruefung| DetailRow {
            tone: overdue(
                pruefung.faellig_am.as_ref(),
                pruefung.durchgefuehrt_am.is_some(),
            ),
            ..row(
                format!(
                    "{} {}",
                    pruefung.art.clone().unwrap_or_else(|| "Prüfung".into()),
                    labelled("fällig", date(pruefung.faellig_am.as_ref()))
                )
                .trim()
                .to_string(),
                vec![
                    pruefung.status.clone().unwrap_or_default(),
                    labelled("geplant", date(pruefung.geplant_am.as_ref())),
                    labelled("durchgeführt", date(pruefung.durchgefuehrt_am.as_ref())),
                    labelled(
                        "Bestellung",
                        pruefung.bestellnummer.clone().unwrap_or_default(),
                    ),
                ],
            )
        })
        .collect();
    section("Prüfungen", "Keine Prüfung.", rows)
}

fn instandhaltungen(db: &TrainsDb, id: &str) -> DetailSection {
    let mut events: Vec<_> = db
        .instandhaltungen()
        .filter(|event| event.wagen_id == id)
        .collect();
    newest_first(&mut events);
    let rows = events
        .into_iter()
        .map(|event| {
            let radsatz = event
                .radsatz_id
                .as_deref()
                .and_then(|id| db.radsatz(id))
                .map(|radsatz| format!("Radsatz {}", radsatz.nummer))
                .unwrap_or_default();
            let mut lines = event_lines(db, event);
            lines.push(radsatz);
            DetailRow {
                link: event
                    .werkstatt_id
                    .as_deref()
                    .and_then(|id| link(EntityRef::Partner, id)),
                ..row(event_title(event), lines)
            }
        })
        .collect();
    section("Instandhaltungen", "Keine Instandhaltung.", rows)
}
