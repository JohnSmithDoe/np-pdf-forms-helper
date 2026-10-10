// ─── why ────────────────────────────────────────────────────────
// The Telematik list, as its page shows it — backend for frontend. A task
// starts where its data is: someone looking for a silent device starts from the
// devices, not from the Wagen list, so this is its own view and not a filter on
// that one. One row per Wagen that has a device or a reading; a click opens the
// Wagen's detail page, which is where everything else about it lives.
//
// Ordered by how long a Wagen has been silent, longest first, because the
// silent ones are what the list is opened for. A device that never reported is
// the most silent of all and comes first. `STUMM_AB_TAGEN` is the customer's
// dashboard threshold (red above seven days) — the same number as the Wagen
// card's `STUMM_AB_TAGEN` in `util/wagen-zustand.utility.ts`; change one, change both.
//
// Days are counted in calendar days against `today`, the way the customer's
// sheet subtracts dates. `today` is an argument so the view is a pure function;
// `commands` passes `clock::today()`.
// ────────────────────────────────────────────────────────────────

use std::collections::BTreeMap;

use super::db::TrainsDb;
use super::detail::moment;
use super::model::{TelematikGeraet, TelematikMeldung, TelematikRow, TelematikView};
use super::sanitise::date::{days_from_civil, Date};
use super::sanitise::format;

pub const STUMM_AB_TAGEN: i64 = 7;

pub fn view(db: &TrainsDb, today: Date) -> TelematikView {
    let zustand = db.zustand();
    let mut geraete: BTreeMap<&str, Vec<&TelematikGeraet>> = BTreeMap::new();
    for geraet in &zustand.geraete {
        geraete.entry(&geraet.wagen_id).or_default().push(geraet);
    }
    let meldungen: BTreeMap<&str, &TelematikMeldung> = zustand
        .meldungen
        .iter()
        .map(|meldung| (meldung.wagen_id.as_str(), meldung))
        .collect();

    let mut ids: Vec<&str> = geraete.keys().chain(meldungen.keys()).copied().collect();
    ids.sort_unstable();
    ids.dedup();

    let mut rows: Vec<TelematikRow> = ids
        .into_iter()
        .filter_map(|id| {
            let wagen = db.wagen_by_id(id)?;
            Some(row(
                id,
                format::uic_in(&wagen.nummer, db.settings().wagennummer),
                wagen.nummer.clone(),
                geraete.get(id).map(Vec::as_slice).unwrap_or_default(),
                meldungen.get(id).copied(),
                today,
            ))
        })
        .collect();
    rows.sort_by(|left, right| {
        silence(right)
            .cmp(&silence(left))
            .then_with(|| left.nummer.cmp(&right.nummer))
    });
    TelematikView {
        stumm: rows.iter().filter(|row| row.stumm).count() as u32,
        rows,
    }
}

fn row(
    id: &str,
    title: String,
    nummer: String,
    geraete: &[&TelematikGeraet],
    meldung: Option<&TelematikMeldung>,
    today: Date,
) -> TelematikRow {
    let tage = meldung.and_then(|meldung| tage_seit(&meldung.zeitpunkt, today));
    let geraet = geraete
        .iter()
        .map(|geraet| geraet.kennung.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let lines = meldung
        .map(|meldung| {
            vec![
                format!("letzte Meldung {}", moment(&meldung.zeitpunkt)),
                meldung.bewegung.clone().unwrap_or_default(),
                meldung
                    .laufleistung_km
                    .map(|km| format!("{km} km"))
                    .unwrap_or_default(),
                meldung
                    .energie_prozent
                    .map(|percent| format!("Energie {percent} %"))
                    .unwrap_or_default(),
            ]
        })
        .unwrap_or_default()
        .into_iter()
        .filter(|line| !line.is_empty())
        .collect();
    TelematikRow {
        wagen_id: id.to_string(),
        title,
        nummer,
        geraet: (!geraet.is_empty()).then_some(geraet),
        standort: meldung.map(ort).unwrap_or_default(),
        funk: match (meldung, tage) {
            (None, _) => "noch keine Meldung".into(),
            (Some(_), None) => String::new(),
            (Some(_), Some(tage)) => funk(tage),
        },
        stumm: meldung.is_none() || tage.is_some_and(|tage| tage > STUMM_AB_TAGEN),
        tage,
        lines,
    }
}

fn silence(row: &TelematikRow) -> (bool, i64) {
    (row.funk == "noch keine Meldung", row.tage.unwrap_or(-1))
}

fn tage_seit(zeitpunkt: &str, today: Date) -> Option<i64> {
    let day = zeitpunkt.get(..10)?;
    let mut parts = day.split('-');
    let year = parts.next()?.parse().ok()?;
    let month = parts.next()?.parse().ok()?;
    let day = parts.next()?.parse().ok()?;
    let date = Date::new(year, month, day)?;
    let tage = days_from_civil(today.year, today.month, today.day)
        - days_from_civil(date.year, date.month, date.day);
    Some(tage.max(0))
}

fn funk(tage: i64) -> String {
    match tage {
        0 => "funkte heute".into(),
        1 => "funkte vor 1 Tag".into(),
        tage => format!("funkte vor {tage} Tagen"),
    }
}

fn ort(meldung: &TelematikMeldung) -> String {
    [
        meldung.standort.as_ref().or(meldung.stadt.as_ref()),
        meldung.land.as_ref(),
    ]
    .into_iter()
    .flatten()
    .cloned()
    .collect::<Vec<_>>()
    .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TempDir;
    use crate::trains::model::{Provenance, Wagen};

    fn source() -> Provenance {
        Provenance {
            file: "Telematik.xlsx".into(),
            sheet: "Tabelle1".into(),
            row: 2,
            imported_at: "2026-10-06".into(),
        }
    }

    fn wagen(id: &str, nummer: &str) -> Wagen {
        Wagen {
            id: id.into(),
            nummer: nummer.into(),
            halter_id: None,
            eigentuemer_id: None,
            bauart: None,
            bemerkung: None,
            created_at: "2026-10-01".into(),
            source: None,
        }
    }

    fn meldung(wagen_id: &str, zeitpunkt: &str) -> TelematikMeldung {
        TelematikMeldung {
            id: format!("m-{wagen_id}"),
            wagen_id: wagen_id.into(),
            geraet_id: None,
            zeitpunkt: zeitpunkt.into(),
            stadt: Some("Neuhof".into()),
            land: Some("DE".into()),
            standort: None,
            laufleistung_km: Some(227_734),
            energie_prozent: None,
            bewegung: None,
            source: source(),
        }
    }

    fn geraet(wagen_id: &str, kennung: &str) -> TelematikGeraet {
        TelematikGeraet {
            id: format!("g-{kennung}"),
            kennung: kennung.into(),
            wagen_id: wagen_id.into(),
            angebaut_am: None,
            source: source(),
        }
    }

    // Four Wagen: one reported today, one twelve days ago, one has a device
    // that never reported, and one has nothing and must not appear at all.
    fn seeded(label: &str) -> (TempDir, TrainsDb) {
        let folder = TempDir::new(label);
        let mut db = TrainsDb::load(&folder.config()).unwrap();
        db.transaction(|tx| {
            tx.put_wagen(wagen("w-heute", "218124712173"));
            tx.put_wagen(wagen("w-stumm", "318047401234"));
            tx.put_wagen(wagen("w-nie", "338047401230"));
            tx.put_wagen(wagen("w-ohne", "378047401236"));
            let zustand = tx.zustand_mut();
            zustand
                .meldungen
                .push(meldung("w-heute", "2026-10-08T07:30:00"));
            zustand
                .meldungen
                .push(meldung("w-stumm", "2026-09-26T23:59:00"));
            zustand.geraete.push(geraet("w-stumm", "PTR-17"));
            zustand.geraete.push(geraet("w-nie", "PTR-99"));
            Ok(())
        })
        .unwrap();
        (folder, db)
    }

    fn today() -> Date {
        Date::new(2026, 10, 8).unwrap()
    }

    #[test]
    fn the_longest_silent_comes_first_and_a_wagen_without_telematik_is_left_out() {
        let (_f, db) = seeded("telematik-order");
        let view = view(&db, today());
        let ids: Vec<&str> = view.rows.iter().map(|row| row.wagen_id.as_str()).collect();
        assert_eq!(ids, ["w-nie", "w-stumm", "w-heute"]);
        assert_eq!(view.stumm, 2);
    }

    #[test]
    fn silence_is_counted_in_calendar_days_and_red_above_seven() {
        let (_f, db) = seeded("telematik-days");
        let view = view(&db, today());
        let stumm = &view.rows[1];
        assert_eq!(stumm.tage, Some(12));
        assert_eq!(stumm.funk, "funkte vor 12 Tagen");
        assert!(stumm.stumm);
        assert_eq!(stumm.geraet.as_deref(), Some("PTR-17"));
        assert_eq!(stumm.standort, "Neuhof, DE");

        let heute = &view.rows[2];
        assert_eq!(heute.funk, "funkte heute");
        assert!(!heute.stumm);
        assert_eq!(
            heute.lines,
            ["letzte Meldung 08.10.2026 07:30", "227734 km"]
        );
    }

    #[test]
    fn a_device_that_never_reported_is_silent() {
        let (_f, db) = seeded("telematik-never");
        let nie = &view(&db, today()).rows[0];
        assert_eq!(nie.funk, "noch keine Meldung");
        assert!(nie.stumm);
        assert_eq!(nie.tage, None);
        assert!(nie.lines.is_empty());
    }

    #[test]
    fn a_moment_counts_by_its_date_and_an_unreadable_one_counts_nothing() {
        assert_eq!(tage_seit("2026-10-01T10:00:00", today()), Some(7));
        assert_eq!(tage_seit("kaputt", today()), None);
        assert_eq!(tage_seit("2026-10-09", today()), Some(0));
    }

    #[test]
    fn the_view_goes_out_under_the_keys_the_frontend_reads() {
        let (_f, db) = seeded("telematik-wire");
        let value = serde_json::to_value(view(&db, today())).unwrap();
        assert_eq!(value["stumm"], 2);
        assert_eq!(value["rows"][0]["wagenId"], "w-nie");
        assert!(value["rows"][2].get("geraet").is_none());
    }
}
