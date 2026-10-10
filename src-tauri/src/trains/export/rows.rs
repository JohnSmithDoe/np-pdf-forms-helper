// ─── why ────────────────────────────────────────────────────────
// The rows of each data sheet, in the column order `sheets` declares. Kept apart
// from the declarations because those are the overview's contract and these are
// only how the store is read into it.
//
// `Keys` is the one place a Wagen id becomes a Wagennummer cell, so the data
// sheets and the overview's key column cannot spell a number two ways. A row
// whose Wagen is gone is dropped by `ledger`: without its key no formula could
// find it. Rows sort by Wagennummer, then date, so a sheet reads as a ledger.
// ────────────────────────────────────────────────────────────────

use std::collections::BTreeMap;

use super::layout::Cell;
use crate::trains::db::TrainsDb;
use crate::trains::model::{Partner, UicStyle};

pub struct Keys {
    nummern: BTreeMap<String, String>,
    style: UicStyle,
}

impl Keys {
    pub fn of(db: &TrainsDb) -> Self {
        Keys {
            nummern: db
                .wagen()
                .into_iter()
                .map(|wagen| (wagen.id, wagen.nummer))
                .collect(),
            style: db.settings().wagennummer,
        }
    }

    fn nummer(&self, wagen_id: &str) -> Option<&str> {
        self.nummern.get(wagen_id).map(String::as_str)
    }

    fn cell(&self, wagen_id: &str) -> Cell {
        self.nummer(wagen_id)
            .map_or(Cell::Empty, |nummer| Cell::wagen(nummer, self.style))
    }

    pub fn sorted(&self) -> Vec<Cell> {
        let mut nummern: Vec<&String> = self.nummern.values().collect();
        nummern.sort();
        nummern
            .into_iter()
            .map(|nummer| Cell::wagen(nummer, self.style))
            .collect()
    }
}

fn ledger<T>(
    keys: &Keys,
    items: impl IntoIterator<Item = T>,
    wagen_id: impl Fn(&T) -> &str,
    date: impl Fn(&T) -> Option<String>,
    row: impl Fn(&T) -> Vec<Cell>,
) -> Vec<Vec<Cell>> {
    let mut keyed: Vec<(&str, Option<String>, Vec<Cell>)> = items
        .into_iter()
        .filter_map(|item| {
            let nummer = keys.nummer(wagen_id(&item))?;
            let mut cells = vec![keys.cell(wagen_id(&item))];
            cells.extend(row(&item));
            Some((nummer, date(&item), cells))
        })
        .collect();
    keyed.sort_by(|left, right| (left.0, &left.1).cmp(&(right.0, &right.1)));
    keyed.into_iter().map(|(_, _, cells)| cells).collect()
}

fn name(db: &TrainsDb, partner_id: Option<&str>) -> Option<String> {
    partner_id
        .and_then(|id| db.partner_by_id(id))
        .map(|partner: &Partner| partner.name.clone())
}

pub fn wagen(db: &TrainsDb, keys: &Keys) -> Vec<Vec<Cell>> {
    ledger(
        keys,
        db.wagen(),
        |wagen| wagen.id.as_str(),
        |_| None,
        |wagen| {
            vec![
                Cell::text(wagen.bauart.clone()),
                Cell::text(name(db, wagen.halter_id.as_deref())),
                Cell::text(name(db, wagen.eigentuemer_id.as_deref())),
                Cell::text(wagen.bemerkung.clone()),
            ]
        },
    )
}

pub fn radsaetze(db: &TrainsDb, keys: &Keys) -> Vec<Vec<Cell>> {
    let einbauten = db.einbauten();
    let mut rows: Vec<(String, Vec<Cell>)> = db
        .radsaetze()
        .into_iter()
        .map(|radsatz| {
            let open = einbauten
                .iter()
                .find(|einbau| einbau.radsatz_id == radsatz.id && einbau.is_open());
            let cells = vec![
                Cell::Text(radsatz.nummer.clone()),
                Cell::text(radsatz.system_id),
                Cell::text(radsatz.wellennummer),
                Cell::text(radsatz.bauart),
                open.map_or(Cell::Empty, |einbau| keys.cell(&einbau.wagen_id)),
                Cell::date(open.and_then(|einbau| einbau.eingebaut_am.as_deref())),
                Cell::text(open.and_then(|einbau| einbau.position.clone())),
                Cell::text(radsatz.bemerkung),
            ];
            (radsatz.nummer, cells)
        })
        .collect();
    rows.sort_by(|left, right| left.0.cmp(&right.0));
    rows.into_iter().map(|(_, cells)| cells).collect()
}

pub fn einbauten(db: &TrainsDb, keys: &Keys) -> Vec<Vec<Cell>> {
    ledger(
        keys,
        db.einbauten(),
        |einbau| einbau.wagen_id.as_str(),
        |einbau| einbau.eingebaut_am.clone(),
        |einbau| {
            vec![
                Cell::text(db.radsatz(&einbau.radsatz_id).map(|r| r.nummer.clone())),
                Cell::text(einbau.position.clone()),
                Cell::date(einbau.eingebaut_am.as_deref()),
                Cell::date(einbau.ausgebaut_am.as_deref()),
            ]
        },
    )
}

pub fn instandhaltungen(db: &TrainsDb, keys: &Keys) -> Vec<Vec<Cell>> {
    ledger(
        keys,
        db.instandhaltungen(),
        |event| event.wagen_id.as_str(),
        |event| event.datum.clone(),
        |event| {
            vec![
                Cell::date(event.datum.as_deref()),
                Cell::text(name(db, event.werkstatt_id.as_deref())),
                Cell::Text(event.leistung.clone()),
                Cell::text(
                    event
                        .radsatz_id
                        .as_deref()
                        .and_then(|id| db.radsatz(id))
                        .map(|radsatz| radsatz.nummer.clone()),
                ),
                Cell::money(event.betrag_cent),
                Cell::text(event.bemerkung.clone()),
            ]
        },
    )
}

pub fn telematik(db: &TrainsDb, keys: &Keys) -> Vec<Vec<Cell>> {
    let zustand = db.zustand();
    let mut wagen: BTreeMap<&str, ()> = BTreeMap::new();
    for id in zustand.geraete.iter().map(|g| g.wagen_id.as_str()) {
        wagen.insert(id, ());
    }
    for id in zustand.meldungen.iter().map(|m| m.wagen_id.as_str()) {
        wagen.insert(id, ());
    }
    ledger(
        keys,
        wagen.into_keys(),
        |id| id,
        |_| None,
        |id| {
            let mut geraete: Vec<_> = zustand
                .geraete
                .iter()
                .filter(|geraet| geraet.wagen_id == **id)
                .collect();
            geraete.sort_by(|left, right| right.angebaut_am.cmp(&left.angebaut_am));
            let meldung = zustand
                .meldungen
                .iter()
                .filter(|meldung| meldung.wagen_id == **id)
                .max_by(|left, right| left.zeitpunkt.cmp(&right.zeitpunkt));
            let kennungen: Vec<&str> = geraete.iter().map(|g| g.kennung.as_str()).collect();
            vec![
                Cell::text(Some(kennungen.join(", "))),
                Cell::date(geraete.first().and_then(|g| g.angebaut_am.as_deref())),
                Cell::date(meldung.map(|m| m.zeitpunkt.as_str())),
                Cell::text(meldung.and_then(|m| m.stadt.clone())),
                Cell::text(meldung.and_then(|m| m.land.clone())),
                Cell::text(meldung.and_then(|m| m.standort.clone())),
                Cell::number(meldung.and_then(|m| m.laufleistung_km)),
                Cell::number(meldung.and_then(|m| m.energie_prozent)),
                Cell::text(meldung.and_then(|m| m.bewegung.clone())),
            ]
        },
    )
}

pub fn schaeden(db: &TrainsDb, keys: &Keys) -> Vec<Vec<Cell>> {
    ledger(
        keys,
        db.zustand().schaeden.iter(),
        |schaden| schaden.wagen_id.as_str(),
        |schaden| schaden.gemeldet_am.clone(),
        |schaden| {
            vec![
                Cell::date(schaden.gemeldet_am.as_deref()),
                Cell::text(schaden.gemeldet_von.clone()),
                Cell::text(schaden.schadcode.clone()),
                Cell::text(schaden.notiz.clone()),
                Cell::yes_no(schaden.ausgesetzt),
                Cell::yes_no(schaden.beladen),
                Cell::text(schaden.ausfuehrender.clone()),
                Cell::date(schaden.geplant_am.as_deref()),
                Cell::text(schaden.aktion.clone()),
                Cell::date(schaden.erledigt_am.as_deref()),
            ]
        },
    )
}

pub fn auftraege(db: &TrainsDb, keys: &Keys) -> Vec<Vec<Cell>> {
    ledger(
        keys,
        db.zustand().auftraege.iter(),
        |auftrag| auftrag.wagen_id.as_str(),
        |auftrag| auftrag.erfasst_am.clone(),
        |auftrag| {
            vec![
                Cell::Text(auftrag.bestellnummer.clone()),
                Cell::text(name(db, auftrag.werkstatt_id.as_deref())),
                Cell::text(auftrag.status.clone()),
                Cell::date(auftrag.erfasst_am.as_deref()),
                Cell::date(auftrag.eingang_am.as_deref()),
                Cell::date(auftrag.ausgang_am.as_deref()),
                Cell::date(auftrag.versendet_am.as_deref()),
                Cell::text(auftrag.bemerkung.clone()),
            ]
        },
    )
}

pub fn pruefungen(db: &TrainsDb, keys: &Keys) -> Vec<Vec<Cell>> {
    ledger(
        keys,
        db.zustand().pruefungen.iter(),
        |pruefung| pruefung.wagen_id.as_str(),
        |pruefung| pruefung.faellig_am.clone(),
        |pruefung| {
            vec![
                Cell::text(pruefung.art.clone()),
                Cell::date(pruefung.faellig_am.as_deref()),
                Cell::date(pruefung.geplant_am.as_deref()),
                Cell::date(pruefung.durchgefuehrt_am.as_deref()),
                Cell::text(pruefung.status.clone()),
                Cell::text(pruefung.bestellnummer.clone()),
            ]
        },
    )
}
