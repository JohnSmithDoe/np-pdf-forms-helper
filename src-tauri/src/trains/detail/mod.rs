// ─── why ────────────────────────────────────────────────────────
// One Wagen, Radsatz or Partner as its detail page shows it — backend for
// frontend. Every string is formatted HERE (dates German, Wagennummern in the
// Schattensystem's spelling, amounts with a comma), so the page renders rows and
// decides nothing; a second formatter in TypeScript would be the one that drifts.
//
// The page is the information hub's centre: everything stored about the entity,
// each row that names another entity carrying a `link` to it, so a Wagen leads
// to its Radsätze and Werkstätten and they lead back to their other Wagen.
//
// One generic shape — fields, then sections of rows — for all three kinds, so
// one page renders them. A section always comes back, empty or not, with the
// sentence that says so: a missing heading would read as „not loaded“.
//
// A Partner's lists are CAPPED at `LIMIT` rows plus a line naming the rest: a
// workshop with ten thousand Instandhaltungen is a list nobody reads on one page.
// A Prüfung past its due date and not done is `danger`; that is decided here
// against today's date, the one read of a clock this module makes.
// ────────────────────────────────────────────────────────────────

mod partner;
mod radsatz;
mod wagen;

use super::db::TrainsDb;
use super::model::{
    DetailField, DetailLink, DetailRow, DetailSection, DetailTone, EntityDetail, EntityRef,
    Instandhaltung, LinkKind,
};
use super::sanitise::format;
use crate::error::{AppError, AppResult};

const LIMIT: usize = 50;

pub fn build(db: &TrainsDb, kind: EntityRef, id: &str) -> AppResult<EntityDetail> {
    let detail = match kind {
        EntityRef::Wagen => wagen::detail(db, id),
        EntityRef::Radsatz => radsatz::detail(db, id),
        EntityRef::Partner => partner::detail(db, id),
    };
    detail.ok_or_else(|| {
        AppError::Report(vec![
            "Den Eintrag gibt es nicht mehr.".into(),
            "Vielleicht wurde er entfernt oder ein Master-Import hat die Daten neu aufgebaut."
                .into(),
        ])
    })
}

fn link(kind: impl Into<LinkKind>, id: &str) -> Option<DetailLink> {
    Some(DetailLink {
        kind: kind.into(),
        id: id.to_string(),
    })
}

fn field(label: &str, value: impl Into<String>) -> Option<DetailField> {
    let value = value.into();
    (!value.is_empty()).then(|| DetailField {
        label: label.into(),
        value,
        link: None,
    })
}

fn linked(label: &str, value: String, target: Option<DetailLink>) -> Option<DetailField> {
    field(label, value).map(|field| DetailField {
        link: target,
        ..field
    })
}

fn row(title: impl Into<String>, lines: Vec<String>) -> DetailRow {
    DetailRow {
        title: title.into(),
        lines: lines.into_iter().filter(|line| !line.is_empty()).collect(),
        link: None,
        tone: None,
    }
}

fn section(title: &str, empty: &str, mut rows: Vec<DetailRow>) -> DetailSection {
    let total = rows.len();
    if total > LIMIT {
        rows.truncate(LIMIT);
        rows.push(row(format!("… und {} weitere", total - LIMIT), Vec::new()));
    }
    DetailSection {
        title: title.into(),
        rows,
        empty: empty.into(),
    }
}

fn date(iso: Option<&String>) -> String {
    iso.map(|iso| format::iso_date(iso)).unwrap_or_default()
}

fn labelled(label: &str, value: String) -> String {
    if value.is_empty() {
        String::new()
    } else {
        format!("{label} {value}")
    }
}

pub(super) fn moment(iso: &str) -> String {
    let (day, time) = iso.split_once('T').unwrap_or((iso, ""));
    let time: String = time.chars().take(5).collect();
    format!("{} {time}", format::iso_date(day))
        .trim()
        .to_string()
}

fn uic(db: &TrainsDb, nummer: &str) -> String {
    format::uic_in(nummer, db.settings().wagennummer)
}

fn wagen_label(db: &TrainsDb, id: &str) -> String {
    db.wagen_by_id(id)
        .map(|wagen| uic(db, &wagen.nummer))
        .unwrap_or_else(|| "unbekannter Wagen".into())
}

fn partner_name(db: &TrainsDb, id: Option<&String>) -> String {
    id.and_then(|id| db.partner_by_id(id))
        .map(|partner| partner.name.clone())
        .unwrap_or_default()
}

fn newest_first(events: &mut [&Instandhaltung]) {
    events.sort_by(|left, right| right.datum.cmp(&left.datum));
}

fn event_lines(db: &TrainsDb, event: &Instandhaltung) -> Vec<String> {
    vec![
        date(event.datum.as_ref()),
        partner_name(db, event.werkstatt_id.as_ref()),
        event
            .betrag_cent
            .map(|cents| format!("{} €", format::money(cents)))
            .unwrap_or_default(),
        event.bemerkung.clone().unwrap_or_default(),
    ]
}

fn event_title(event: &Instandhaltung) -> String {
    if event.leistung.is_empty() {
        "Instandhaltung".into()
    } else {
        event.leistung.clone()
    }
}

fn overdue(faellig: Option<&String>, done: bool) -> Option<DetailTone> {
    let today = super::clock::today_iso();
    faellig
        .filter(|faellig| !done && faellig.as_str() < today.as_str())
        .map(|_| DetailTone::Danger)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_long_section_is_capped_with_a_line_naming_the_rest() {
        let rows = (0..LIMIT + 7)
            .map(|index| row(index.to_string(), Vec::new()))
            .collect();
        let capped = section("Liste", "leer", rows);
        assert_eq!(capped.rows.len(), LIMIT + 1);
        assert_eq!(capped.rows[LIMIT].title, "… und 7 weitere");
    }

    #[test]
    fn a_moment_renders_german_without_seconds() {
        assert_eq!(moment("2026-10-05T08:15:00"), "05.10.2026 08:15");
    }

    #[test]
    fn an_empty_value_makes_no_field_and_no_line() {
        assert!(field("Bauart", "").is_none());
        assert!(row("t", vec![String::new(), "x".into()]).lines == vec!["x"]);
        assert_eq!(labelled("seit", String::new()), "");
    }

    #[test]
    fn a_due_date_in_the_past_is_danger_unless_done() {
        let past = "2000-01-01".to_string();
        assert_eq!(overdue(Some(&past), false), Some(DetailTone::Danger));
        assert_eq!(overdue(Some(&past), true), None);
        assert_eq!(overdue(Some(&"2999-01-01".to_string()), false), None);
    }

    use crate::testing::TempDir;
    use crate::trains::model::{
        Einbau, Partner, PartnerRolle, Provenance, Pruefung, Radsatz, Schadensmeldung,
        TelematikMeldung, Wagen,
    };

    fn source() -> Provenance {
        Provenance {
            file: "liste.xlsx".into(),
            sheet: "Tabelle1".into(),
            row: 2,
            imported_at: "2026-10-06".into(),
        }
    }

    // One Wagen with a Halter who is also its Werkstatt, a fitted Radsatz, a
    // reading, an open damage report and an overdue P8 — every section that
    // links somewhere has something to link.
    fn seeded(label: &str) -> (TempDir, TrainsDb) {
        let folder = TempDir::new(label);
        let mut db = TrainsDb::load(&folder.config()).unwrap();
        db.transaction(|tx| {
            tx.put_partner(Partner {
                id: "p1".into(),
                rollen: vec![PartnerRolle::Halter, PartnerRolle::Werkstatt],
                name: "Wagenmut AG".into(),
                match_key: "wagenmut".into(),
                aliases: Vec::new(),
                bemerkung: None,
                created_at: "2026-10-01".into(),
            });
            tx.put_wagen(Wagen {
                id: "w1".into(),
                nummer: "218124712173".into(),
                halter_id: Some("p1".into()),
                eigentuemer_id: None,
                bauart: Some("Tanoos".into()),
                bemerkung: None,
                created_at: "2026-10-01".into(),
                source: None,
            });
            tx.put_radsatz(Radsatz {
                id: "r1".into(),
                nummer: "RS-0815".into(),
                match_key: "rs0815".into(),
                aliases: Vec::new(),
                wellennummer: None,
                system_id: None,
                bauart: None,
                bemerkung: None,
                created_at: "2026-10-01".into(),
                source: None,
            });
            tx.put_einbau(Einbau {
                id: "e1".into(),
                radsatz_id: "r1".into(),
                wagen_id: "w1".into(),
                position: Some("1".into()),
                eingebaut_am: Some("2026-02-02".into()),
                ausgebaut_am: None,
                source: source(),
            });
            let zustand = tx.zustand_mut();
            zustand.meldungen.push(TelematikMeldung {
                id: "m1".into(),
                wagen_id: "w1".into(),
                geraet_id: None,
                zeitpunkt: "2026-10-05T08:15:00".into(),
                stadt: Some("Neuhof".into()),
                land: Some("DE".into()),
                standort: None,
                laufleistung_km: Some(227_734),
                energie_prozent: None,
                bewegung: None,
                source: source(),
            });
            zustand.schaeden.push(Schadensmeldung {
                id: "s1".into(),
                wagen_id: "w1".into(),
                gemeldet_am: Some("2026-09-29".into()),
                gemeldet_von: None,
                schadcode: Some("3.3.4".into()),
                notiz: None,
                ausgesetzt: Some(true),
                beladen: None,
                ausfuehrender: None,
                geplant_am: None,
                aktion: None,
                erledigt_am: None,
                source: source(),
            });
            zustand.pruefungen.push(Pruefung {
                id: "pr1".into(),
                wagen_id: "w1".into(),
                art: Some("P8".into()),
                faellig_am: Some("2001-03-31".into()),
                geplant_am: None,
                durchgefuehrt_am: None,
                status: None,
                bestellnummer: None,
                source: source(),
            });
            Ok(())
        })
        .unwrap();
        (folder, db)
    }

    fn section_of<'a>(detail: &'a EntityDetail, title: &str) -> &'a DetailSection {
        detail
            .sections
            .iter()
            .find(|section| section.title == title)
            .unwrap_or_else(|| panic!("no section {title}"))
    }

    #[test]
    fn a_wagen_shows_everything_and_links_to_its_radsatz_and_halter() {
        let (_f, db) = seeded("detail-wagen");
        let detail = build(&db, EntityRef::Wagen, "w1").unwrap();

        assert_eq!(detail.title, "218124712173");
        let halter = detail.fields.iter().find(|f| f.label == "Halter").unwrap();
        assert_eq!(halter.value, "Wagenmut AG");
        assert_eq!(halter.link, link(EntityRef::Partner, "p1"));

        let fitted = section_of(&detail, "Eingebaute Radsätze");
        assert_eq!(fitted.rows[0].title, "RS-0815");
        assert_eq!(fitted.rows[0].link, link(EntityRef::Radsatz, "r1"));

        let telematik = section_of(&detail, "Telematik");
        assert_eq!(telematik.rows[0].title, "Letzte Meldung 05.10.2026 08:15");
        assert!(telematik.rows[0].lines.contains(&"227734 km".to_string()));
        // Telematik has no page per Wagen, so its rows open the list on this one.
        assert_eq!(telematik.rows[0].link, link(LinkKind::Telematik, "w1"));

        let schaden = &section_of(&detail, "Schadensmeldungen").rows[0];
        assert_eq!(schaden.tone, Some(DetailTone::Warning));
        assert!(schaden.lines.contains(&"ausgesetzt".to_string()));

        let p8 = &section_of(&detail, "Prüfungen").rows[0];
        assert_eq!(p8.title, "P8 fällig 31.03.2001");
        assert_eq!(p8.tone, Some(DetailTone::Danger));

        // An empty section still comes back, with the sentence that says so.
        let auftraege = section_of(&detail, "Werkstattaufträge");
        assert!(auftraege.rows.is_empty());
        assert_eq!(auftraege.empty, "Kein Werkstattauftrag.");
    }

    // A master sheet without positions adds an open fitting with none; it goes
    // after the placed ones, not before them as `Option`'s order would put it.
    #[test]
    fn a_fitting_without_position_is_listed_last() {
        let (_f, mut db) = seeded("detail-unplaced");
        db.transaction(|tx| {
            tx.put_radsatz(Radsatz {
                id: "r2".into(),
                nummer: "RS-0816".into(),
                match_key: "rs0816".into(),
                aliases: Vec::new(),
                wellennummer: None,
                system_id: None,
                bauart: None,
                bemerkung: None,
                created_at: "2026-10-01".into(),
                source: None,
            });
            tx.put_einbau(Einbau {
                id: "e2".into(),
                radsatz_id: "r2".into(),
                wagen_id: "w1".into(),
                position: None,
                eingebaut_am: Some("2026-06-19".into()),
                ausgebaut_am: None,
                source: source(),
            });
            Ok(())
        })
        .unwrap();
        let detail = build(&db, EntityRef::Wagen, "w1").unwrap();
        let titles: Vec<&str> = section_of(&detail, "Eingebaute Radsätze")
            .rows
            .iter()
            .map(|row| row.title.as_str())
            .collect();
        assert_eq!(titles, vec!["RS-0815", "RS-0816"]);
    }

    #[test]
    fn a_radsatz_links_back_to_its_wagen() {
        let (_f, db) = seeded("detail-radsatz");
        let detail = build(&db, EntityRef::Radsatz, "r1").unwrap();
        let current = detail
            .fields
            .iter()
            .find(|f| f.label == "Eingebaut in")
            .unwrap();
        assert_eq!(current.link, link(EntityRef::Wagen, "w1"));
        assert_eq!(
            section_of(&detail, "Einbauten").rows[0].lines[2],
            "läuft noch"
        );
    }

    #[test]
    fn a_partner_has_a_section_per_role_it_plays() {
        let (_f, db) = seeded("detail-partner");
        let detail = build(&db, EntityRef::Partner, "p1").unwrap();
        assert_eq!(detail.subtitle.as_deref(), Some("Halter, Werkstatt"));
        let titles: Vec<&str> = detail.sections.iter().map(|s| s.title.as_str()).collect();
        assert_eq!(
            titles,
            vec!["Wagen als Halter", "Werkstattaufträge", "Instandhaltungen"]
        );
        let kept = &section_of(&detail, "Wagen als Halter").rows[0];
        assert_eq!(kept.link, link(EntityRef::Wagen, "w1"));
    }

    #[test]
    fn an_entity_that_is_gone_is_a_german_error() {
        let (_f, db) = seeded("detail-gone");
        let error = build(&db, EntityRef::Wagen, "weg").unwrap_err();
        assert!(
            error.to_string().contains("gibt es nicht mehr")
                || format!("{error:?}").contains("gibt es nicht mehr")
        );
    }

    #[test]
    fn the_detail_goes_out_under_the_keys_the_frontend_reads() {
        let (_f, db) = seeded("detail-wire");
        let value = serde_json::to_value(build(&db, EntityRef::Wagen, "w1").unwrap()).unwrap();
        assert_eq!(value["kind"], "wagen");
        assert_eq!(value["fields"][0]["link"]["kind"], "partner");
        assert!(value["sections"][0].get("empty").is_some());
    }
}
