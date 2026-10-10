// ─── why ────────────────────────────────────────────────────────
// The data sheets of the master overview: one per entity of the Schattensystem,
// so each sheet is ONE kind of fact and every row is one of them. The overview
// computes from these, never from anything the app holds, so a row edited by
// hand in Excel moves the overview with it.
//
// The Wagennummer is column A of every sheet that has one, because that is the
// key every overview formula asks by, and the customer's own sheets put it there
// too. The column order is the formulas' contract: the overview resolves its
// letters from these headers (`layout::letter_of`), so renaming a header here
// without the overview is a test failure rather than a wrong column.
//
// The rows themselves are built in `rows`. Ids are not written — the file is
// for people, and nothing reads it back.
// ────────────────────────────────────────────────────────────────

use super::layout::{column, Cell, Column, Format};
use super::rows::{self, Keys};
use crate::trains::db::TrainsDb;

pub struct Sheet {
    pub name: &'static str,
    pub columns: &'static [Column],
}

pub const WAGEN: Sheet = Sheet {
    name: "Wagen",
    columns: &[
        column("Wagennummer", Format::Wagen, 16.0),
        column("Bauart", Format::Text, 16.0),
        column("Halter", Format::Text, 24.0),
        column("Eigentümer", Format::Text, 24.0),
        column("Bemerkung", Format::Text, 40.0),
    ],
};

pub const RADSAETZE: Sheet = Sheet {
    name: "Radsätze",
    columns: &[
        column("Radsatznummer", Format::Text, 16.0),
        column("Radsatz-ID", Format::Text, 14.0),
        column("Radsatzwellennummer", Format::Text, 18.0),
        column("Bauart", Format::Text, 12.0),
        column("eingebaut in Wagen", Format::Wagen, 16.0),
        column("eingebaut am", Format::Date, 12.0),
        column("Position", Format::Text, 9.0),
        column("Bemerkung", Format::Text, 40.0),
    ],
};

pub const EINBAUTEN: Sheet = Sheet {
    name: "Einbauten",
    columns: &[
        column("Wagennummer", Format::Wagen, 16.0),
        column("Radsatznummer", Format::Text, 16.0),
        column("Position", Format::Text, 9.0),
        column("eingebaut am", Format::Date, 12.0),
        column("ausgebaut am", Format::Date, 12.0),
    ],
};

pub const INSTANDHALTUNGEN: Sheet = Sheet {
    name: "Instandhaltungen",
    columns: &[
        column("Wagennummer", Format::Wagen, 16.0),
        column("Datum", Format::Date, 12.0),
        column("Werkstatt", Format::Text, 24.0),
        column("Leistung", Format::Text, 40.0),
        column("Radsatznummer", Format::Text, 16.0),
        column("Betrag", Format::Money, 12.0),
        column("Bemerkung", Format::Text, 40.0),
    ],
};

pub const TELEMATIK: Sheet = Sheet {
    name: "Telematik",
    columns: &[
        column("Wagennummer", Format::Wagen, 16.0),
        column("Gerät", Format::Text, 16.0),
        column("angebaut am", Format::Date, 12.0),
        column("letzte Meldung", Format::Moment, 16.0),
        column("Stadt", Format::Text, 18.0),
        column("Land", Format::Text, 6.0),
        column("Standort", Format::Text, 24.0),
        column("Laufleistung km", Format::Integer, 14.0),
        column("Energie %", Format::Integer, 10.0),
        column("Bewegung", Format::Text, 12.0),
    ],
};

pub const SCHAEDEN: Sheet = Sheet {
    name: "Schäden",
    columns: &[
        column("Wagennummer", Format::Wagen, 16.0),
        column("gemeldet am", Format::Date, 12.0),
        column("gemeldet von", Format::Text, 18.0),
        column("Schadcode", Format::Text, 10.0),
        column("Notiz", Format::Text, 40.0),
        column("ausgesetzt", Format::Text, 10.0),
        column("beladen", Format::Text, 9.0),
        column("ausführende Werkstatt/EVU", Format::Text, 22.0),
        column("geplant am", Format::Date, 12.0),
        column("notwendige Aktion", Format::Text, 40.0),
        column("erledigt am", Format::Date, 12.0),
    ],
};

pub const AUFTRAEGE: Sheet = Sheet {
    name: "Aufträge",
    columns: &[
        column("Wagennummer", Format::Wagen, 16.0),
        column("Bestellnummer", Format::Text, 14.0),
        column("Werkstatt", Format::Text, 24.0),
        column("Status", Format::Text, 16.0),
        column("erfasst am", Format::Date, 12.0),
        column("Eingang", Format::Date, 12.0),
        column("Ausgang", Format::Date, 12.0),
        column("versendet am", Format::Date, 12.0),
        column("Bemerkung", Format::Text, 40.0),
    ],
};

pub const PRUEFUNGEN: Sheet = Sheet {
    name: "Prüfungen",
    columns: &[
        column("Wagennummer", Format::Wagen, 16.0),
        column("Art", Format::Text, 10.0),
        column("fällig am", Format::Date, 12.0),
        column("geplant am", Format::Date, 12.0),
        column("durchgeführt am", Format::Date, 14.0),
        column("Status", Format::Text, 16.0),
        column("Bestellnummer", Format::Text, 14.0),
    ],
};

pub fn all(db: &TrainsDb) -> Vec<(&'static Sheet, Vec<Vec<Cell>>)> {
    let keys = Keys::of(db);
    vec![
        (&WAGEN, rows::wagen(db, &keys)),
        (&RADSAETZE, rows::radsaetze(db, &keys)),
        (&EINBAUTEN, rows::einbauten(db, &keys)),
        (&INSTANDHALTUNGEN, rows::instandhaltungen(db, &keys)),
        (&TELEMATIK, rows::telematik(db, &keys)),
        (&SCHAEDEN, rows::schaeden(db, &keys)),
        (&AUFTRAEGE, rows::auftraege(db, &keys)),
        (&PRUEFUNGEN, rows::pruefungen(db, &keys)),
    ]
}
