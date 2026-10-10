// ─── why ────────────────────────────────────────────────────────
// The „Übersicht“: one row per Wagen, every column after the key a FORMULA into
// the data sheets — the way the customer's „Alle Wagen Überblick“ works, so a
// data sheet edited in Excel moves the overview with it. The values are not
// written beside the formulas: umya writes an old `calcId`, so Excel recalculates
// the whole book on open, and a second copy of each answer computed in Rust
// would be one more thing to keep in step with the formula.
//
// Each column answers by Wagennummer only, with `MINIFS`/`MAXIFS`/`COUNTIFS`
// over whole columns — whole, so a row appended by hand is counted. `MINIFS`
// and `MAXIFS` are Excel 2019 functions and are stored `_xlfn.`-prefixed, as
// Excel itself stores them. Nothing is COMPUTED that the data does not state:
// a due date is the stated one, never an Einbau plus months. „offen“ means an
// empty end date — `erledigt am`, `Ausgang`, `ausgebaut am`, `durchgeführt am`.
//
// Silence is calendar days, `HEUTE()` minus the day of the last Meldung, red
// above `telematik::STUMM_AB_TAGEN` — the app's own rule, so the file and the
// Telematik list agree. A device that never reported reads „nie“, and is red.
//
// The HAND columns are the customer's: written empty, with their own header
// colour, and NOT carried over from an earlier export — every export is a new
// file. The legend in row 2 says so, because a user who types into this file
// and exports again would otherwise lose the typing without a word.
//
// The key figures sit above the table and count the table itself, so they are
// right after any filter edit too. Formulas are written with English names and
// commas, which is how a file stores them in any Excel language.
// ────────────────────────────────────────────────────────────────

use umya_spreadsheet::Worksheet;

use super::layout::{self, column, letter_of, Cell, Column, Format};
use super::rows::Keys;
use super::sheets::{self, Sheet};
use crate::trains::db::TrainsDb;
use crate::trains::sanitise::{date::Date, format};
use crate::trains::telematik::STUMM_AB_TAGEN;

pub const NAME: &str = "Übersicht";
pub const HEADER_ROW: u32 = 6;

const COMPUTED: &str = "FFD9E1F2";
const HAND: &str = "FFFFF2CC";
const RED: &str = "FFFFC7CE";
const GREEN: &str = "FFC6EFCE";
const ORANGE: &str = "FFFCE4D6";

pub const COLUMNS: [Column; 19] = [
    column("Wagennummer", Format::Wagen, 16.0),
    column("Bauart", Format::Text, 14.0),
    column("Wo ist der Wagen?", Format::Text, 18.0),
    column("Tage ohne Meldung", Format::Integer, 10.0),
    column("Telematik angebaut am", Format::Date, 12.0),
    column("P8 fällig", Format::Date, 12.0),
    column("Revision fällig", Format::Date, 12.0),
    column("offene Schäden", Format::Count, 9.0),
    column("letzter Schaden gemeldet am", Format::Date, 12.0),
    column("offene Aufträge", Format::Count, 9.0),
    column("Werkstatteingang", Format::Date, 12.0),
    column("Radsätze eingebaut", Format::Count, 9.0),
    column("ältester Einbau", Format::Date, 12.0),
    column("letzte Instandhaltung", Format::Date, 12.0),
    column("Bemerkungen / notwendige Aktion", Format::Text, 36.0),
    column("AUSGESETZT ja/nein", Format::Text, 11.0),
    column("Wagen beladen ja/nein", Format::Text, 11.0),
    column("Auftrag muss noch versendet werden", Format::Text, 16.0),
    column("Notiz", Format::Text, 30.0),
];

const FIRST_HAND: usize = 14;

struct Ref(&'static Sheet);

impl Ref {
    fn col(&self, header: &str) -> String {
        let letter = letter_of(self.0.columns, header);
        format!("'{}'!${letter}:${letter}", self.0.name)
    }

    fn key(&self) -> String {
        self.col("Wagennummer")
    }
}

fn formulas(row: u32) -> Vec<String> {
    let key = format!("$A{row}");
    let wagen = Ref(&sheets::WAGEN);
    let telematik = Ref(&sheets::TELEMATIK);
    let pruefungen = Ref(&sheets::PRUEFUNGEN);
    let schaeden = Ref(&sheets::SCHAEDEN);
    let auftraege = Ref(&sheets::AUFTRAEGE);
    let einbauten = Ref(&sheets::EINBAUTEN);
    let events = Ref(&sheets::INSTANDHALTUNGEN);

    let lookup = |sheet: &Ref, header: &str| {
        let last = letter_of(sheet.0.columns, header);
        let index = sheet
            .0
            .columns
            .iter()
            .position(|c| c.header == header)
            .unwrap_or(0)
            + 1;
        format!(
            "IFERROR(VLOOKUP({key},'{}'!$A:${last},{index},FALSE)&\"\",\"\")",
            sheet.0.name
        )
    };
    let last_meldung = format!(
        "_xlfn.MAXIFS({},{},{key})",
        telematik.col("letzte Meldung"),
        telematik.key()
    );
    let pruefung = |art: &str| {
        format!(
            "_xlfn.MINIFS({},{},{key},{},\"{art}\",{},\"\")",
            pruefungen.col("fällig am"),
            pruefungen.key(),
            pruefungen.col("Art"),
            pruefungen.col("durchgeführt am")
        )
    };

    vec![
        lookup(&wagen, "Bauart"),
        lookup(&telematik, "Stadt"),
        format!(
            "IF(COUNTIF({},{key})=0,\"\",IF({last_meldung}=0,\"nie\",TODAY()-INT({last_meldung})))",
            telematik.key()
        ),
        format!(
            "_xlfn.MAXIFS({},{},{key})",
            telematik.col("angebaut am"),
            telematik.key()
        ),
        pruefung("P8"),
        pruefung("<>P8"),
        format!(
            "COUNTIFS({},{key},{},\"\")",
            schaeden.key(),
            schaeden.col("erledigt am")
        ),
        format!(
            "_xlfn.MAXIFS({},{},{key},{},\"\")",
            schaeden.col("gemeldet am"),
            schaeden.key(),
            schaeden.col("erledigt am")
        ),
        format!(
            "COUNTIFS({},{key},{},\"\")",
            auftraege.key(),
            auftraege.col("Ausgang")
        ),
        format!(
            "_xlfn.MAXIFS({},{},{key},{},\"\")",
            auftraege.col("Eingang"),
            auftraege.key(),
            auftraege.col("Ausgang")
        ),
        format!(
            "COUNTIFS({},{key},{},\"\")",
            einbauten.key(),
            einbauten.col("ausgebaut am")
        ),
        format!(
            "_xlfn.MINIFS({},{},{key},{},\"\")",
            einbauten.col("eingebaut am"),
            einbauten.key(),
            einbauten.col("ausgebaut am")
        ),
        format!(
            "_xlfn.MAXIFS({},{},{key})",
            events.col("Datum"),
            events.key()
        ),
    ]
}

fn rows(keys: &Keys) -> Vec<Vec<Cell>> {
    keys.sorted()
        .into_iter()
        .enumerate()
        .map(|(offset, key)| {
            let row = HEADER_ROW + 1 + offset as u32;
            let mut cells = vec![key];
            cells.extend(formulas(row).into_iter().map(Cell::Formula));
            cells
        })
        .collect()
}

pub fn write(sheet: &mut Worksheet, db: &TrainsDb, today: Date) {
    let rows = rows(&Keys::of(db));
    let first = HEADER_ROW + 1;
    let last = HEADER_ROW + rows.len().max(1) as u32;

    let title = sheet.cell_mut((1, 1));
    title.set_value_string("Übersicht aller Wagen");
    title.style_mut().font_mut().set_bold(true).set_size(14.0);
    sheet.cell_mut((1, 2)).set_value_string(format!(
        "Stand {} · blaue Spalten rechnen aus den Datenblättern · gelbe Spalten werden von Hand gepflegt und von keinem neuen Export übernommen",
        format::date(today)
    ));

    key_figures(sheet, first, last);
    layout::table(sheet, HEADER_ROW, &COLUMNS, &rows, COMPUTED);
    for (index, column) in COLUMNS.iter().enumerate().skip(FIRST_HAND) {
        layout::heading(sheet, index as u32 + 1, HEADER_ROW, column.header, HAND);
        for row in first..=last {
            layout::put(sheet, index as u32 + 1, row, &Cell::Empty, column.format);
        }
    }
    layout::freeze(sheet, HEADER_ROW, 1);
    colours(sheet, first, last);
}

fn key_figures(sheet: &mut Worksheet, first: u32, last: u32) {
    let range = |header: &str| {
        let col = letter_of(&COLUMNS, header);
        format!("{col}{first}:{col}{last}")
    };
    let silent = range("Tage ohne Meldung");
    let p8 = range("P8 fällig");
    let revision = range("Revision fällig");
    let figures = [
        ("Wagen", format!("COUNTA({})", range("Wagennummer"))),
        (
            "stumm",
            format!("COUNTIF({silent},\">{STUMM_AB_TAGEN}\")+COUNTIF({silent},\"nie\")"),
        ),
        (
            "P8 überfällig",
            format!("COUNTIFS({p8},\"<\"&TODAY(),{p8},\">0\")"),
        ),
        (
            "Revision überfällig",
            format!("COUNTIFS({revision},\"<\"&TODAY(),{revision},\">0\")"),
        ),
        (
            "offene Schäden",
            format!("SUM({})", range("offene Schäden")),
        ),
        (
            "in Werkstatt",
            format!("COUNTIF({},\">0\")", range("Werkstatteingang")),
        ),
    ];
    for (index, (label, formula)) in figures.into_iter().enumerate() {
        let col = index as u32 + 1;
        layout::heading(sheet, col, 3, label, COMPUTED);
        layout::put(sheet, col, 4, &Cell::Formula(formula), Format::Integer);
        sheet
            .cell_mut((col, 4))
            .style_mut()
            .font_mut()
            .set_bold(true);
    }
}

fn colours(sheet: &mut Worksheet, first: u32, last: u32) {
    let at = |header: &str| letter_of(&COLUMNS, header);
    let span = |col: &str| format!("{col}{first}:{col}{last}");

    let silent = at("Tage ohne Meldung");
    layout::highlight(
        sheet,
        &span(&silent),
        &format!("OR({silent}{first}=\"nie\",AND(ISNUMBER({silent}{first}),{silent}{first}>{STUMM_AB_TAGEN}))"),
        RED,
    );
    layout::highlight(
        sheet,
        &span(&silent),
        &format!("AND(ISNUMBER({silent}{first}),{silent}{first}<={STUMM_AB_TAGEN})"),
        GREEN,
    );
    for header in ["P8 fällig", "Revision fällig"] {
        let col = at(header);
        layout::highlight(
            sheet,
            &span(&col),
            &format!("AND({col}{first}>0,{col}{first}<TODAY())"),
            RED,
        );
        layout::highlight(sheet, &span(&col), &format!("{col}{first}>=TODAY()"), GREEN);
    }
    for header in ["offene Schäden", "offene Aufträge"] {
        let col = at(header);
        layout::highlight(sheet, &span(&col), &format!("{col}{first}>0"), ORANGE);
    }
    let entry = at("Werkstatteingang");
    layout::highlight(sheet, &span(&entry), &format!("{entry}{first}>0"), GREEN);
}
