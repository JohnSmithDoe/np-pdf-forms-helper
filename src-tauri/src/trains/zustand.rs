// ─── why ────────────────────────────────────────────────────────
// The Wagen-Zustand half of a committed row: Telematik, Schadensmeldung,
// Werkstattauftrag, Prüfung. Called by `commit::commit_row` once the row's Wagen
// (and Werkstatt) is settled, inside the same transaction — none of these is
// asked about in the walk, because each hangs off a Wagen the user already
// decided. A declined Wagen never reaches here.
//
// THE CUSTOMER'S DASHBOARD IS A SET OF SNAPSHOTS, so every record here is found
// by its KEY and updated, never appended twice:
//
//   Telematik-Gerät      its `kennung` (the sender's pointer id) — a device
//                        moved to another Wagen follows it
//   Telematik-Meldung    the Wagen — only the LATEST reading is kept
//   Schadensmeldung      Wagen + gemeldet am + Schadcode
//   Werkstattauftrag     Wagen + Bestellnummer
//   Prüfung              Wagen + Art + fällig am
//
// AN EMPTY CELL NEVER CLEARS a stored value — a sender's export that lacks a
// column says nothing about it. The one exception is the Meldung, which is a
// reading and not a record: a newer one replaces the stored one WHOLE, because
// a position from 05.10 beside a km reading from 02.10 is a reading that never
// happened.
//
// AN OLDER MELDUNG IS SKIPPED AND COUNTED. That is the whole point of keeping
// the time: the master paste once wrote an export from 02.10 over the readings of
// 05.10 and nothing said so. Here the older file changes nothing and the report
// says how many readings it held back.
//
// A Werkstattauftrag needs its Bestellnummer AND one order column beside it: a
// P8 list names the Bestellnummer of a Prüfung, and that alone must not invent
// an empty order.
//
// A Prüfung's Art comes from its column, else from the template's fixed
// `ImportPlan.pruefart` — a P8 list has no Art column because the whole file is
// P8. The template's Art alone does not make a Prüfung: some Prüfung column of
// the row has to carry a value.
// ────────────────────────────────────────────────────────────────

use uuid::Uuid;

use super::db::Tx;
use super::model::{
    FieldKind, Provenance, Pruefung, Schadensmeldung, TelematikGeraet, TelematikMeldung,
    Werkstattauftrag,
};
use super::sanitise::{format, Value};
use super::stage::RowValues;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Zustand {
    pub geraete: u32,
    pub meldungen: u32,
    pub aelter: u32,
    pub schaeden: u32,
    pub auftraege: u32,
    pub pruefungen: u32,
}

impl Zustand {
    pub fn anything(&self) -> bool {
        self.geraete + self.meldungen + self.schaeden + self.auftraege + self.pruefungen > 0
    }

    pub fn add(&mut self, other: Zustand) {
        self.geraete += other.geraete;
        self.meldungen += other.meldungen;
        self.aelter += other.aelter;
        self.schaeden += other.schaeden;
        self.auftraege += other.auftraege;
        self.pruefungen += other.pruefungen;
    }

    pub fn line(&self) -> Option<String> {
        if !self.anything() && self.aelter == 0 {
            return None;
        }
        let mut line = format!(
            "Wagen-Zustand: {} Telematik-Meldung(en), {} Telematik-Gerät(e), {} Schadensmeldung(en), {} Werkstattauftrag/-aufträge und {} Prüfung(en) übernommen.",
            self.meldungen, self.geraete, self.schaeden, self.auftraege, self.pruefungen
        );
        if self.aelter > 0 {
            line.push_str(&format!(
                " {} Telematik-Meldung(en) waren älter als der gespeicherte Stand und wurden nicht übernommen.",
                self.aelter
            ));
        }
        Some(line)
    }
}

pub struct ZustandRow<'a> {
    pub values: &'a RowValues,
    pub wagen_id: &'a str,
    pub werkstatt_id: Option<&'a str>,
    pub pruefart: Option<&'a str>,
    pub source: Provenance,
}

pub fn write(tx: &mut Tx<'_>, row: &ZustandRow<'_>) -> Zustand {
    let mut zustand = Zustand::default();
    let geraet_id = geraet(tx, row, &mut zustand);
    meldung(tx, row, geraet_id, &mut zustand);
    schaden(tx, row, &mut zustand);
    auftrag(tx, row, &mut zustand);
    pruefung(tx, row, &mut zustand);
    zustand
}

fn geraet(tx: &mut Tx<'_>, row: &ZustandRow<'_>, zustand: &mut Zustand) -> Option<String> {
    let kennung = text(row.values, FieldKind::TelematikGeraet)?;
    let angebaut_am = date(row.values, FieldKind::TelematikAngebautAm);
    let stored = tx
        .db()
        .zustand()
        .geraete
        .iter()
        .position(|geraet| geraet.kennung == kennung);
    let Some(index) = stored else {
        let id = Uuid::new_v4().to_string();
        tx.zustand_mut().geraete.push(TelematikGeraet {
            id: id.clone(),
            kennung,
            wagen_id: row.wagen_id.to_string(),
            angebaut_am,
            source: row.source.clone(),
        });
        zustand.geraete += 1;
        return Some(id);
    };
    let mut geraet = tx.db().zustand().geraete[index].clone();
    let mut changed = take(&mut geraet.angebaut_am, angebaut_am);
    if geraet.wagen_id != row.wagen_id {
        geraet.wagen_id = row.wagen_id.to_string();
        changed = true;
    }
    let id = geraet.id.clone();
    if changed {
        geraet.source = row.source.clone();
        tx.zustand_mut().geraete[index] = geraet;
        zustand.geraete += 1;
    }
    Some(id)
}

fn meldung(
    tx: &mut Tx<'_>,
    row: &ZustandRow<'_>,
    geraet_id: Option<String>,
    zustand: &mut Zustand,
) {
    let Some(Value::Zeitpunkt(moment)) = row.values.value(FieldKind::TelematikZeitpunkt) else {
        return;
    };
    let mut incoming = TelematikMeldung {
        id: Uuid::new_v4().to_string(),
        wagen_id: row.wagen_id.to_string(),
        geraet_id,
        zeitpunkt: moment.to_iso(),
        stadt: text(row.values, FieldKind::TelematikStadt),
        land: text(row.values, FieldKind::TelematikLand),
        standort: text(row.values, FieldKind::TelematikStandort),
        laufleistung_km: number(row.values, FieldKind::TelematikLaufleistung),
        energie_prozent: number(row.values, FieldKind::TelematikEnergie),
        bewegung: text(row.values, FieldKind::TelematikBewegung),
        source: row.source.clone(),
    };
    let stored = tx
        .db()
        .zustand()
        .meldungen
        .iter()
        .position(|meldung| meldung.wagen_id == row.wagen_id);
    let Some(index) = stored else {
        tx.zustand_mut().meldungen.push(incoming);
        zustand.meldungen += 1;
        return;
    };
    let current = &tx.db().zustand().meldungen[index];
    if current.zeitpunkt > incoming.zeitpunkt {
        zustand.aelter += 1;
        return;
    }
    incoming.id = current.id.clone();
    if same_reading(current, &incoming) {
        return;
    }
    tx.zustand_mut().meldungen[index] = incoming;
    zustand.meldungen += 1;
}

fn same_reading(left: &TelematikMeldung, right: &TelematikMeldung) -> bool {
    TelematikMeldung {
        source: right.source.clone(),
        ..left.clone()
    } == *right
}

fn schaden(tx: &mut Tx<'_>, row: &ZustandRow<'_>, zustand: &mut Zustand) {
    let values = row.values;
    let incoming = Schadensmeldung {
        id: Uuid::new_v4().to_string(),
        wagen_id: row.wagen_id.to_string(),
        gemeldet_am: date(values, FieldKind::SchadenGemeldetAm),
        gemeldet_von: text(values, FieldKind::SchadenGemeldetVon),
        schadcode: text(values, FieldKind::Schadcode),
        notiz: text(values, FieldKind::SchadenNotiz),
        ausgesetzt: flag(values, FieldKind::Ausgesetzt),
        beladen: flag(values, FieldKind::Beladen),
        ausfuehrender: text(values, FieldKind::SchadenAusfuehrender),
        geplant_am: date(values, FieldKind::SchadenGeplantAm),
        aktion: text(values, FieldKind::SchadenAktion),
        erledigt_am: date(values, FieldKind::SchadenErledigtAm),
        source: row.source.clone(),
    };
    let carries_any = incoming.gemeldet_am.is_some()
        || incoming.gemeldet_von.is_some()
        || incoming.schadcode.is_some()
        || incoming.notiz.is_some()
        || incoming.ausgesetzt.is_some()
        || incoming.beladen.is_some()
        || incoming.ausfuehrender.is_some()
        || incoming.geplant_am.is_some()
        || incoming.aktion.is_some()
        || incoming.erledigt_am.is_some();
    if !carries_any {
        return;
    }
    let stored = tx.db().zustand().schaeden.iter().position(|schaden| {
        schaden.wagen_id == incoming.wagen_id
            && schaden.gemeldet_am == incoming.gemeldet_am
            && schaden.schadcode == incoming.schadcode
    });
    let Some(index) = stored else {
        tx.zustand_mut().schaeden.push(incoming);
        zustand.schaeden += 1;
        return;
    };
    let mut schaden = tx.db().zustand().schaeden[index].clone();
    let changed = [
        take(&mut schaden.gemeldet_von, incoming.gemeldet_von),
        take(&mut schaden.notiz, incoming.notiz),
        take(&mut schaden.ausgesetzt, incoming.ausgesetzt),
        take(&mut schaden.beladen, incoming.beladen),
        take(&mut schaden.ausfuehrender, incoming.ausfuehrender),
        take(&mut schaden.geplant_am, incoming.geplant_am),
        take(&mut schaden.aktion, incoming.aktion),
        take(&mut schaden.erledigt_am, incoming.erledigt_am),
    ]
    .contains(&true);
    if changed {
        schaden.source = row.source.clone();
        tx.zustand_mut().schaeden[index] = schaden;
        zustand.schaeden += 1;
    }
}

fn auftrag(tx: &mut Tx<'_>, row: &ZustandRow<'_>, zustand: &mut Zustand) {
    let values = row.values;
    let Some(bestellnummer) = text(values, FieldKind::Bestellnummer) else {
        return;
    };
    let incoming = Werkstattauftrag {
        id: Uuid::new_v4().to_string(),
        wagen_id: row.wagen_id.to_string(),
        bestellnummer,
        werkstatt_id: row.werkstatt_id.map(str::to_string),
        status: text(values, FieldKind::AuftragStatus),
        erfasst_am: date(values, FieldKind::AuftragErfasstAm),
        eingang_am: date(values, FieldKind::AuftragEingangAm),
        ausgang_am: date(values, FieldKind::AuftragAusgangAm),
        versendet_am: date(values, FieldKind::AuftragVersendetAm),
        bemerkung: text(values, FieldKind::AuftragBemerkung),
        source: row.source.clone(),
    };
    let carries_any = incoming.status.is_some()
        || incoming.erfasst_am.is_some()
        || incoming.eingang_am.is_some()
        || incoming.ausgang_am.is_some()
        || incoming.versendet_am.is_some()
        || incoming.bemerkung.is_some();
    if !carries_any {
        return;
    }
    let stored = tx.db().zustand().auftraege.iter().position(|auftrag| {
        auftrag.wagen_id == incoming.wagen_id && auftrag.bestellnummer == incoming.bestellnummer
    });
    let Some(index) = stored else {
        tx.zustand_mut().auftraege.push(incoming);
        zustand.auftraege += 1;
        return;
    };
    let mut auftrag = tx.db().zustand().auftraege[index].clone();
    let changed = [
        take(&mut auftrag.werkstatt_id, incoming.werkstatt_id),
        take(&mut auftrag.status, incoming.status),
        take(&mut auftrag.erfasst_am, incoming.erfasst_am),
        take(&mut auftrag.eingang_am, incoming.eingang_am),
        take(&mut auftrag.ausgang_am, incoming.ausgang_am),
        take(&mut auftrag.versendet_am, incoming.versendet_am),
        take(&mut auftrag.bemerkung, incoming.bemerkung),
    ]
    .contains(&true);
    if changed {
        auftrag.source = row.source.clone();
        tx.zustand_mut().auftraege[index] = auftrag;
        zustand.auftraege += 1;
    }
}

fn pruefung(tx: &mut Tx<'_>, row: &ZustandRow<'_>, zustand: &mut Zustand) {
    let values = row.values;
    let column_art = text(values, FieldKind::Pruefart);
    let faellig_am = date(values, FieldKind::PruefungFaelligAm);
    let geplant_am = date(values, FieldKind::PruefungGeplantAm);
    let durchgefuehrt_am = date(values, FieldKind::PruefungDurchgefuehrtAm);
    let status = text(values, FieldKind::PruefungStatus);
    if column_art.is_none()
        && faellig_am.is_none()
        && geplant_am.is_none()
        && durchgefuehrt_am.is_none()
        && status.is_none()
    {
        return;
    }
    let incoming = Pruefung {
        id: Uuid::new_v4().to_string(),
        wagen_id: row.wagen_id.to_string(),
        art: column_art.or_else(|| row.pruefart.map(str::to_string)),
        faellig_am,
        geplant_am,
        durchgefuehrt_am,
        status,
        bestellnummer: text(values, FieldKind::Bestellnummer),
        source: row.source.clone(),
    };
    let stored = tx.db().zustand().pruefungen.iter().position(|pruefung| {
        pruefung.wagen_id == incoming.wagen_id
            && pruefung.art == incoming.art
            && pruefung.faellig_am == incoming.faellig_am
    });
    let Some(index) = stored else {
        tx.zustand_mut().pruefungen.push(incoming);
        zustand.pruefungen += 1;
        return;
    };
    let mut pruefung = tx.db().zustand().pruefungen[index].clone();
    let changed = [
        take(&mut pruefung.geplant_am, incoming.geplant_am),
        take(&mut pruefung.durchgefuehrt_am, incoming.durchgefuehrt_am),
        take(&mut pruefung.status, incoming.status),
        take(&mut pruefung.bestellnummer, incoming.bestellnummer),
    ]
    .contains(&true);
    if changed {
        pruefung.source = row.source.clone();
        tx.zustand_mut().pruefungen[index] = pruefung;
        zustand.pruefungen += 1;
    }
}

fn take<T: PartialEq>(slot: &mut Option<T>, incoming: Option<T>) -> bool {
    match incoming {
        Some(value) if slot.as_ref() != Some(&value) => {
            *slot = Some(value);
            true
        }
        _ => false,
    }
}

fn text(values: &RowValues, field: FieldKind) -> Option<String> {
    values
        .value(field)
        .map(format::value)
        .filter(|text| !text.is_empty())
}

fn date(values: &RowValues, field: FieldKind) -> Option<String> {
    match values.value(field) {
        Some(Value::Date(date)) => Some(date.to_iso()),
        _ => None,
    }
}

fn number(values: &RowValues, field: FieldKind) -> Option<i64> {
    match values.value(field) {
        Some(Value::Zahl(number)) => Some(*number),
        _ => None,
    }
}

fn flag(values: &RowValues, field: FieldKind) -> Option<bool> {
    match values.value(field) {
        Some(Value::Flag(flag)) => Some(*flag),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TempDir;
    use crate::trains::commit::{commit, Committed};
    use crate::trains::db::TrainsDb;
    use crate::trains::model::{
        ColumnBinding, CommitDecisions, EntityDecision, ImportPlan, RowDecision,
    };
    use crate::trains::sheet::grid::Grid;
    use crate::trains::sheet::layout::LayoutHint;
    use crate::trains::sheet::readers::ReaderKind;
    use crate::trains::stage::{stage, StageInput};

    const WAGEN_A: &str = "21 81 2471 217-3";
    const WAGEN_B: &str = "31 80 4740 123-4";

    fn fresh(label: &str) -> (TempDir, TrainsDb) {
        let folder = TempDir::new(label);
        let db = TrainsDb::load(&folder.config()).unwrap();
        (folder, db)
    }

    fn plan(columns: &[(&str, FieldKind)], pruefart: Option<&str>) -> ImportPlan {
        ImportPlan {
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
            pruefart: pruefart.map(str::to_string),
        }
    }

    // Stages `rows` under `columns` and commits every row with every entity
    // created — the walk's answers do not matter to the Zustand.
    fn import(
        db: &mut TrainsDb,
        columns: &[(&str, FieldKind)],
        rows: &[&[&str]],
        pruefart: Option<&str>,
    ) -> Committed {
        let header: Vec<&str> = columns.iter().map(|(header, _)| *header).collect();
        let mut table: Vec<&[&str]> = vec![&header];
        table.extend_from_slice(rows);
        let grid = Grid::from_text("Tabelle1", &table);
        let plan = plan(columns, pruefart);
        let staged = stage(StageInput {
            id: "s1".into(),
            file: "zustand.xlsx".into(),
            sheets: vec!["Tabelle1".into()],
            candidates: Vec::new(),
            grid: &grid,
            plan: &plan,
            db,
            master: false,
        })
        .unwrap();
        let decisions = CommitDecisions {
            staging_id: "s1".into(),
            rows: staged
                .wire
                .rows
                .iter()
                .map(|row| RowDecision {
                    row: row.row,
                    wagen: EntityDecision::Create,
                    werkstatt: EntityDecision::Create,
                    halter: EntityDecision::Skip,
                    eigentuemer: EntityDecision::Skip,
                    radsatz: EntityDecision::Skip,
                    einbau_uebernehmen: false,
                })
                .collect(),
        };
        commit(db, &staged.wire, &staged.values, &decisions).unwrap()
    }

    fn telematik() -> Vec<(&'static str, FieldKind)> {
        vec![
            ("Asset", FieldKind::Wagennummer),
            ("Pointer Name", FieldKind::TelematikGeraet),
            ("Anbaudatum", FieldKind::TelematikAngebautAm),
            ("Timestamp", FieldKind::TelematikZeitpunkt),
            ("Stadt", FieldKind::TelematikStadt),
            ("Summe Laufleistung", FieldKind::TelematikLaufleistung),
        ]
    }

    #[test]
    fn a_telematics_row_writes_its_device_and_its_reading_with_the_time() {
        let (_f, mut db) = fresh("zustand-telematik");
        let result = import(
            &mut db,
            &telematik(),
            &[&[
                WAGEN_A,
                "P1",
                "15.03.2023",
                "2026-10-05 08:15:00",
                "Bebra",
                "227.734",
            ]],
            None,
        );

        assert_eq!((result.zustand.meldungen, result.zustand.geraete), (1, 1));
        assert_eq!(result.instandhaltungen, 0, "a reading is not work");
        let zustand = db.zustand();
        let meldung = &zustand.meldungen[0];
        assert_eq!(meldung.zeitpunkt, "2026-10-05T08:15:00");
        assert_eq!(meldung.stadt.as_deref(), Some("Bebra"));
        assert_eq!(meldung.laufleistung_km, Some(227_734));
        assert_eq!(
            meldung.geraet_id.as_deref(),
            Some(zustand.geraete[0].id.as_str())
        );
        assert_eq!(
            zustand.geraete[0].angebaut_am.as_deref(),
            Some("2023-03-15")
        );
    }

    // The defect that made the time worth keeping: an export from 02.10 pasted
    // over readings of 05.10 rolled 360 positions back without a word.
    #[test]
    fn an_older_reading_is_held_back_and_reported() {
        let (_f, mut db) = fresh("zustand-older");
        import(
            &mut db,
            &telematik(),
            &[&[WAGEN_A, "P1", "", "2026-10-05 08:15:00", "Bebra", ""]],
            None,
        );
        let result = import(
            &mut db,
            &telematik(),
            &[&[WAGEN_A, "P1", "", "2026-10-02 13:37:01", "Hœnheim", ""]],
            None,
        );

        assert_eq!(result.zustand.aelter, 1);
        assert_eq!(result.zustand.meldungen, 0);
        assert!(
            result.messages[1].contains("älter"),
            "{:?}",
            result.messages
        );
        assert_eq!(db.zustand().meldungen[0].stadt.as_deref(), Some("Bebra"));
    }

    // Same day, later hour: only the time orders the two.
    #[test]
    fn a_newer_reading_replaces_the_stored_one_whole() {
        let (_f, mut db) = fresh("zustand-newer");
        import(
            &mut db,
            &telematik(),
            &[&[WAGEN_A, "P1", "", "2026-10-05 08:15:00", "Bebra", "1000"]],
            None,
        );
        let result = import(
            &mut db,
            &telematik(),
            &[&[WAGEN_A, "P1", "", "2026-10-05 17:40:00", "Fulda", ""]],
            None,
        );

        assert_eq!(result.zustand.meldungen, 1);
        let zustand = db.zustand();
        assert_eq!(zustand.meldungen.len(), 1, "only the latest is kept");
        assert_eq!(zustand.meldungen[0].stadt.as_deref(), Some("Fulda"));
        assert_eq!(
            zustand.meldungen[0].laufleistung_km, None,
            "a reading is replaced whole, not merged"
        );
    }

    #[test]
    fn the_same_file_twice_changes_nothing_the_second_time() {
        let (_f, mut db) = fresh("zustand-twice");
        let row: &[&str] = &[
            WAGEN_A,
            "P1",
            "15.03.2023",
            "2026-10-05 08:15:00",
            "Bebra",
            "5",
        ];
        import(&mut db, &telematik(), &[row], None);
        let result = import(&mut db, &telematik(), &[row], None);
        assert_eq!(result.zustand, Zustand::default());
    }

    #[test]
    fn a_device_fitted_to_another_wagen_follows_it() {
        let (_f, mut db) = fresh("zustand-device");
        import(
            &mut db,
            &telematik(),
            &[&[WAGEN_A, "P1", "", "2026-10-01 08:00:00", "", ""]],
            None,
        );
        import(
            &mut db,
            &telematik(),
            &[&[WAGEN_B, "P1", "", "2026-10-03 08:00:00", "", ""]],
            None,
        );
        let zustand = db.zustand();
        assert_eq!(zustand.geraete.len(), 1);
        let wagen_b = db.wagen_by_nummer("318047401234").unwrap();
        assert_eq!(zustand.geraete[0].wagen_id, wagen_b.id);
    }

    fn auftraege() -> Vec<(&'static str, FieldKind)> {
        vec![
            ("bestellnr", FieldKind::Bestellnummer),
            ("wagen", FieldKind::Wagennummer),
            ("empfaenger", FieldKind::Werkstatt),
            ("eingang_ist", FieldKind::AuftragEingangAm),
            ("werk_ausg_ist", FieldKind::AuftragAusgangAm),
            ("status", FieldKind::AuftragStatus),
        ]
    }

    // The order feed used to be read as finished Instandhaltungen, with every
    // unfinished order unticked by hand. Now each row is its order.
    #[test]
    fn two_orders_for_one_wagen_are_two_orders_and_no_work() {
        let (_f, mut db) = fresh("zustand-orders");
        let result = import(
            &mut db,
            &auftraege(),
            &[
                &[
                    "24094-26",
                    WAGEN_A,
                    "Schienenbein",
                    "01.10.2026",
                    "",
                    "BS_FREIGABE",
                ],
                &["24095-26", WAGEN_A, "Schienenbein", "", "", "BS_ERFASST"],
            ],
            None,
        );
        assert_eq!(result.zustand.auftraege, 2);
        assert_eq!(result.instandhaltungen, 0);
        let werkstatt = db.partner()[0].id.clone();
        assert_eq!(
            db.zustand().auftraege[0].werkstatt_id.as_deref(),
            Some(werkstatt.as_str())
        );
    }

    #[test]
    fn a_resent_order_is_updated_and_an_empty_cell_clears_nothing() {
        let (_f, mut db) = fresh("zustand-order-update");
        import(
            &mut db,
            &auftraege(),
            &[&["24094-26", WAGEN_A, "", "01.10.2026", "", "BS_FREIGABE"]],
            None,
        );
        let result = import(
            &mut db,
            &auftraege(),
            &[&["24094-26", WAGEN_A, "", "", "06.10.2026", "BS_ZUGESTELLT"]],
            None,
        );
        assert_eq!(result.zustand.auftraege, 1);
        let auftraege = &db.zustand().auftraege;
        assert_eq!(auftraege.len(), 1);
        assert_eq!(auftraege[0].eingang_am.as_deref(), Some("2026-10-01"));
        assert_eq!(auftraege[0].ausgang_am.as_deref(), Some("2026-10-06"));
        assert_eq!(auftraege[0].status.as_deref(), Some("BS_ZUGESTELLT"));
    }

    // The P8 list names a Bestellnummer and nothing else about the order, and
    // carries no Art column: the template's Art lands on the Prüfung.
    #[test]
    fn a_p8_row_is_a_pruefung_with_the_templates_art_and_no_order() {
        let (_f, mut db) = fresh("zustand-p8");
        let columns = [
            ("TRANSPORTMITTELNR", FieldKind::Wagennummer),
            ("TERMIN", FieldKind::PruefungFaelligAm),
            ("BESTELLNUMMER", FieldKind::Bestellnummer),
            ("STATUS", FieldKind::PruefungStatus),
        ];
        let result = import(
            &mut db,
            &columns,
            &[&[WAGEN_A, "30.06.2027", "27642-26", "A"]],
            Some("P8"),
        );
        assert_eq!(
            (result.zustand.pruefungen, result.zustand.auftraege),
            (1, 0)
        );
        let pruefung = &db.zustand().pruefungen[0];
        assert_eq!(pruefung.art.as_deref(), Some("P8"));
        assert_eq!(pruefung.faellig_am.as_deref(), Some("2027-06-30"));
        assert_eq!(pruefung.bestellnummer.as_deref(), Some("27642-26"));
    }

    #[test]
    fn a_column_art_beats_the_templates_and_keys_the_pruefung() {
        let (_f, mut db) = fresh("zustand-art");
        let columns = [
            ("Wagennummer", FieldKind::Wagennummer),
            ("Prüfungsart", FieldKind::Pruefart),
            ("Fälligkeitstermin", FieldKind::PruefungFaelligAm),
        ];
        import(
            &mut db,
            &columns,
            &[
                &[WAGEN_A, "G4.2", "30.06.2030"],
                &[WAGEN_A, "", "30.06.2027"],
            ],
            Some("P8"),
        );
        let arten: Vec<Option<&str>> = db
            .zustand()
            .pruefungen
            .iter()
            .map(|pruefung| pruefung.art.as_deref())
            .collect();
        assert_eq!(arten, vec![Some("G4.2"), Some("P8")]);
    }

    // A Schadensmeldung stays open until a later file gives its erledigt date;
    // the code `3.3.4` is text, never a date.
    #[test]
    fn a_damage_report_is_closed_by_a_later_file() {
        let (_f, mut db) = fresh("zustand-schaden");
        let columns = [
            ("Wagennummer", FieldKind::Wagennummer),
            ("gemeldet am", FieldKind::SchadenGemeldetAm),
            ("Schadcode", FieldKind::Schadcode),
            ("AUSGESETZT", FieldKind::Ausgesetzt),
            ("erledigt", FieldKind::SchadenErledigtAm),
        ];
        import(
            &mut db,
            &columns,
            &[&[WAGEN_A, "01.10.2026", "3.3.4", "JA", ""]],
            None,
        );
        let schaden = &db.zustand().schaeden[0];
        assert_eq!(schaden.schadcode.as_deref(), Some("3.3.4"));
        assert_eq!(schaden.ausgesetzt, Some(true));
        assert!(schaden.erledigt_am.is_none());

        import(
            &mut db,
            &columns,
            &[&[WAGEN_A, "01.10.2026", "3.3.4", "", "07.10.2026"]],
            None,
        );
        let schaeden = &db.zustand().schaeden;
        assert_eq!(schaeden.len(), 1);
        assert_eq!(schaeden[0].erledigt_am.as_deref(), Some("2026-10-07"));
        assert_eq!(
            schaeden[0].ausgesetzt,
            Some(true),
            "an empty cell clears nothing"
        );
    }

    #[test]
    fn removing_a_wagen_and_clearing_the_mirror_take_its_zustand() {
        let (_f, mut db) = fresh("zustand-cascade");
        import(
            &mut db,
            &telematik(),
            &[
                &[WAGEN_A, "P1", "", "2026-10-05 08:15:00", "", ""],
                &[WAGEN_B, "P2", "", "2026-10-05 08:15:00", "", ""],
            ],
            None,
        );
        let wagen_a = db.wagen_by_nummer("218124712173").unwrap().id.clone();
        db.transaction(|tx| {
            tx.remove_wagen(&wagen_a);
            Ok(())
        })
        .unwrap();
        assert_eq!(db.zustand().meldungen.len(), 1);
        assert_eq!(db.zustand().geraete.len(), 1);

        db.clear_mirror().unwrap();
        assert_eq!(*db.zustand(), crate::trains::model::WagenZustand::default());
    }

    #[test]
    fn the_zustand_survives_a_reload() {
        let (folder, mut db) = fresh("zustand-reload");
        import(
            &mut db,
            &telematik(),
            &[&[WAGEN_A, "P1", "", "2026-10-05 08:15:00", "Bebra", ""]],
            None,
        );
        let reloaded = TrainsDb::load(&folder.config()).unwrap();
        assert_eq!(reloaded.zustand(), db.zustand());
    }
}
