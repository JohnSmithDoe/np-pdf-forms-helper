// ─── why ────────────────────────────────────────────────────────
// Where the three halves meet: a `Grid` says what is in the file, a `Layout`
// says which cells are the data, the sanitisers say what those cells mean, and
// `resolve` says which entities they point at. Out comes a preview.
//
// STAGING NEVER TOUCHES THE DATABASE. That is what makes "the import went wrong
// halfway" a state this code cannot reach, and it is why a bad cell is a line in
// a report rather than an aborted run — there is nothing to abort to. The row
// loop returns `Vec<StagedRow>` and not `AppResult<Vec<StagedRow>>` precisely so
// a `?` cannot creep into it; if one appears in review, that is the regression.
//
// The interpretations are decided ONCE PER COLUMN, before any row is read, by
// `reading::read_column` — the same call `clean` makes, so the preview and the
// cleaned copy cannot read one file two ways. A column's question becomes a
// Warnung on exactly the rows that read differently under the alternative, not
// on every row of the column: the rows that do not differ have nothing to check.
//
// `interpret` returns one slot PER COLUMN, `None` where the column is ignored,
// so `stage_row` indexes it by position. Searching it by `binding.index` was a
// linear scan per cell per row.
//
// A cell Excel stored as a NUMBER never goes through the string parsers —
// `value_number()` already answered, and its stringification uses a `.` decimal
// whatever the file's own style is. For dates that means a serial, and whether a
// number IS a serial is decided by the column's aggregated date format, falling
// back to a plausibility window: a bare 45000 in a column of costs is a cost.
//
// A duplicate outranks needing input, and the order matters. A row that repeats
// one already staged must not be offered as "create these entities" — the user
// would be invited to mint a wagen for a row that should not be committed at
// all. The FIRST occurrence still asks; the repeat is only ever a repeat.
//
// The dedupe key is built from the file's OWN WORDS — the canonical wagen
// number and the werkstatt's normalised name — and never from a resolved entity
// id. An id only exists once the partner does, so a key using one would hash the
// same row differently before and after its first import, and re-sending a file
// would double every event in it. The key has to mean "this row", not "this row
// as currently resolved".
//
// The Wagen-Zustand's own keys — Bestellnummer, a Schadensmeldung's date and
// code, a Prüfung's Art and due date — join the key only when a row carries
// one, so two orders for one Wagen in one file are two rows, and every key
// stored before them still hashes alike.
//
// The radsatz and both fitting dates are IN the key. Without them a
// wheelset-monitoring export — four fitted radsaetze per wagen, no Datum,
// Leistung or Betrag — hashed all four rows of a wagen alike and flagged three
// of every four as duplicates. The radsatz goes in as its normalised NUMBER,
// for the same reason the werkstatt goes in as its name.
//
// AN INCOMPLETE PLAN IS NOT AN ERROR. Staging a half-mapped file is the normal
// state of the mapping screen — every field the user picks re-stages, so
// refusing until the required ones are mapped makes the FIRST pick fail and
// nothing can ever be mapped. Required-ness gates the review step and the
// commit, both of which check it themselves.
//
// EVERY column produces a cell, including an ignored one, tagged with the column
// it came from. Two reasons: the mapping screen shows sample values, and the
// columns most needing them are exactly the unmapped ones; and matching cells by
// FIELD rather than by column silently merges two columns mapped to the same
// field.
//
// `parse_cell` is shared with `clean`, so the cleaned file holds exactly what
// staging would have read from the original — a second reading of the same cell
// would be a second answer.
//
// THE MASTER is read by two rules of its own (`master`): a `0` in a DATE column
// is the empty result of a VLOOKUP, not 1899-12-30, so it reads as empty — in an
// amount column a `0` is a real amount and stays; and a Fehler in ANY mapped
// cell rejects the row rather than importing it with a hole, because the mirror
// is checked against the customer's sheet and a silently shortened row would
// pass for a correct one. The rejected row is listed, never dropped quietly.
//
// `AppError` is reserved for what stops the whole gesture — the layout does not
// fit the sheet, or no required field is mapped. Everything else is a `CellIssue`
// on a row that still arrives.
// ────────────────────────────────────────────────────────────────

use std::collections::HashSet;

use super::db::TrainsDb;
use super::hash;
use super::model::{
    CellIssue, ColumnBinding, FieldKind, ImportPlan, Resolution, RowStatus, Severity, StagedCell,
    StagedImport, StagedRow, StagedSummary, StagingOrigin,
};
use super::reading::{read_column, reads_a_date, Confirmed, Interpretation, Question};
use super::resolve;
use super::sanitise::{date, format, number, text, Parsed, Value};
use super::sheet::grid::Grid;
use super::sheet::layout::{Candidate, Layout};
use crate::error::AppResult;
use crate::trains::model::PartnerRolle;

pub struct RowValues {
    pub row: u32,
    pub values: Vec<(FieldKind, Value)>,
    pub dedupe_key: String,
}

impl RowValues {
    pub fn value(&self, field: FieldKind) -> Option<&Value> {
        value_of(&self.values, field)
    }
}

fn value_of(values: &[(FieldKind, Value)], field: FieldKind) -> Option<&Value> {
    values
        .iter()
        .find(|(kind, _)| *kind == field)
        .map(|(_, value)| value)
}

pub struct Staged {
    pub wire: StagedImport,
    pub values: Vec<RowValues>,
}

struct Read {
    interpretation: Interpretation,
    question: Option<Question>,
}

pub struct HeldImport {
    pub grid: Grid,
    pub wire: StagedImport,
    pub values: Vec<RowValues>,
}

pub struct StageInput<'a> {
    pub id: String,
    pub file: String,
    pub sheets: Vec<String>,
    pub candidates: Vec<Candidate>,
    pub grid: &'a Grid,
    pub plan: &'a ImportPlan,
    pub db: &'a TrainsDb,
    pub master: bool,
}

pub fn stage(input: StageInput<'_>) -> AppResult<Staged> {
    let layout = input.plan.reader.apply(input.grid, input.plan.layout)?;
    let interpretations = interpret(input.grid, input.plan, &layout);

    let mut rows = Vec::new();
    let mut values = Vec::new();
    let mut seen_keys: HashSet<String> = HashSet::new();
    let mut partners = resolve::PartnerMemo::default();
    for row in layout.first_data_row..=layout.last_data_row {
        if let Some((staged, staged_values)) =
            stage_row(&input, &interpretations, row, &mut seen_keys, &mut partners)
        {
            rows.push(staged);
            values.push(staged_values);
        }
    }

    let summary = summarise(&rows);
    Ok(Staged {
        wire: StagedImport {
            id: input.id,
            file: input.file,
            sheet: input.grid.sheet.clone(),
            sheets: input.sheets,
            plan: input.plan.clone(),
            candidates: input.candidates,
            rows,
            summary,
            origin: StagingOrigin::Datei,
            entities: None,
        },
        values,
    })
}

fn interpret(grid: &Grid, plan: &ImportPlan, layout: &Layout) -> Vec<Option<Read>> {
    plan.columns
        .iter()
        .map(|binding| {
            if binding.field == FieldKind::Ignorieren {
                return None;
            }
            let rows = layout.first_data_row..=layout.last_data_row.min(grid.rows);
            let cells: Vec<(u32, &super::sheet::grid::RawCell)> = rows
                .filter_map(|row| grid.cell(binding.index, row).map(|cell| (row, cell)))
                .collect();
            let texts: Vec<(u32, &str)> = cells
                .iter()
                .filter(|(_, cell)| cell.number.is_none() && !cell.text.trim().is_empty())
                .map(|(row, cell)| (*row, cell.text.as_str()))
                .collect();
            let hints: Vec<bool> = cells.iter().map(|(_, cell)| cell.date_format).collect();
            let (interpretation, question) =
                read_column(binding, &texts, &hints, Confirmed::default());
            Some(Read {
                interpretation,
                question,
            })
        })
        .collect()
}

fn stage_row(
    input: &StageInput<'_>,
    interpretations: &[Option<Read>],
    row: u32,
    seen_keys: &mut HashSet<String>,
    partners: &mut resolve::PartnerMemo,
) -> Option<(StagedRow, RowValues)> {
    let mut cells = Vec::new();
    let mut issues = Vec::new();
    let mut values: Vec<(FieldKind, Value)> = Vec::new();
    let mut rejected = false;

    for (position, binding) in input.plan.columns.iter().enumerate() {
        let cell = input.grid.cell(binding.index, row);
        let raw = cell.map(|cell| cell.text.clone()).unwrap_or_default();

        let cell = cell.filter(|cell| !(input.master && is_master_blank(binding.field, cell)));
        let raw = if cell.is_none() { String::new() } else { raw };

        let Some(read) = interpretations.get(position).and_then(Option::as_ref) else {
            cells.push(StagedCell {
                column: binding.index,
                field: binding.field,
                raw,
                parsed: String::new(),
                ok: true,
            });
            continue;
        };

        match parse_cell(&read.interpretation, input.plan, cell) {
            Ok(Parsed { value, warning }) => {
                if let Some(message) = warning {
                    issues.push(issue(row, binding, &raw, message, Severity::Warnung));
                }
                if let Some(question) = read.question.as_ref().filter(|q| q.touches(row)) {
                    let message = format!("{} Bitte Spalte prüfen.", question.reason);
                    issues.push(issue(row, binding, &raw, message, Severity::Warnung));
                }
                cells.push(StagedCell {
                    column: binding.index,
                    field: binding.field,
                    raw,
                    parsed: format::styled(&value, input.db.settings().wagennummer),
                    ok: true,
                });
                values.push((binding.field, value));
            }
            Err(message) => {
                if binding.field.required() || input.master {
                    rejected = true;
                }
                issues.push(issue(row, binding, &raw, message, Severity::Fehler));
                cells.push(StagedCell {
                    column: binding.index,
                    field: binding.field,
                    raw,
                    parsed: String::new(),
                    ok: false,
                });
            }
        }
    }

    // A row where every mapped cell is blank is spacing, not data.
    if cells.iter().all(|cell| cell.raw.trim().is_empty()) {
        return None;
    }

    let uic = match value_of(&values, FieldKind::Wagennummer) {
        Some(Value::Uic(uic)) => Some(uic.as_str().to_string()),
        _ => None,
    };
    let wagen = resolve::wagen(input.db, uic.as_deref());
    let werkstatt = partners.find(
        input.db,
        PartnerRolle::Werkstatt,
        raw_of(&cells, FieldKind::Werkstatt),
    );
    let halter = partners.find(
        input.db,
        PartnerRolle::Halter,
        raw_of(&cells, FieldKind::Halter),
    );
    let eigentuemer = partners.find(
        input.db,
        PartnerRolle::Eigentuemer,
        raw_of(&cells, FieldKind::Eigentuemer),
    );
    let sender = sender_of(input, &werkstatt);
    let radsatz = resolve::find_radsatz(
        input.db,
        raw_of(&cells, FieldKind::Radsatznummer),
        sender.as_deref(),
    );

    let key = dedupe_key(
        &uic,
        &values,
        raw_of(&cells, FieldKind::Werkstatt),
        raw_of(&cells, FieldKind::Radsatznummer),
    );
    let duplicate = input.db.event_exists(&key) || !seen_keys.insert(key.clone());

    let status = if rejected {
        RowStatus::Rejected
    } else if duplicate {
        RowStatus::Duplicate
    } else if wagen.needs_input()
        || werkstatt.needs_input()
        || halter.needs_input()
        || eigentuemer.needs_input()
        || radsatz.needs_input()
    {
        RowStatus::NeedsInput
    } else {
        RowStatus::Ready
    };

    Some((
        StagedRow {
            row,
            status,
            cells,
            wagen,
            werkstatt,
            halter,
            eigentuemer,
            radsatz,
            issues,
            sender,
        },
        RowValues {
            row,
            values,
            dedupe_key: key,
        },
    ))
}

pub(super) fn parse_cell(
    interpretation: &Interpretation,
    plan: &ImportPlan,
    cell: Option<&super::sheet::grid::RawCell>,
) -> super::sanitise::Parse {
    let raw = cell.map(|cell| cell.text.as_str()).unwrap_or("");
    let stored_number = cell.and_then(|cell| cell.number);

    let field = interpretation.binding.field;
    match field {
        FieldKind::Wagennummer => super::sanitise::wagen::parse(raw),
        FieldKind::Betrag => match stored_number {
            Some(amount) => Ok(Parsed::plain(Value::Money((amount * 100.0).round() as i64))),
            None => number::parse_money(raw, interpretation.decimal.value),
        },
        FieldKind::TelematikLaufleistung | FieldKind::TelematikEnergie => match stored_number {
            Some(amount) => number::count(amount)
                .map(|value| Parsed::plain(Value::Zahl(value)))
                .ok_or_else(|| format!("Die Zahl „{raw}“ ist zu groß.")),
            None => number::parse_count(raw, interpretation.decimal.value),
        },
        FieldKind::TelematikZeitpunkt => match stored_number {
            Some(serial) if is_serial(interpretation, serial) => {
                date::zeitpunkt_from_serial(serial, plan.date1904)
                    .map(|value| Parsed::plain(Value::Zeitpunkt(value)))
            }
            _ => date::parse_zeitpunkt(raw, interpretation.date_order.value),
        },
        _ if reads_a_date(field) => match stored_number {
            Some(serial) if is_serial(interpretation, serial) => {
                date::from_serial(serial, plan.date1904)
                    .map(|value| Parsed::plain(Value::Date(value)))
            }
            _ => date::parse_text(raw, interpretation.date_order.value),
        },
        FieldKind::Ausgesetzt | FieldKind::Beladen => text::parse_flag(raw),
        FieldKind::Ignorieren => text::parse(""),
        _ => text::parse(raw),
    }
}

fn is_master_blank(field: FieldKind, cell: &super::sheet::grid::RawCell) -> bool {
    reads_a_date(field) && cell.number == Some(0.0)
}

fn is_serial(interpretation: &Interpretation, serial: f64) -> bool {
    interpretation.dates || date::PLAUSIBLE_SERIALS.contains(&(serial.floor() as i64))
}

fn issue(
    row: u32,
    binding: &ColumnBinding,
    raw: &str,
    message: String,
    severity: Severity,
) -> CellIssue {
    CellIssue {
        row,
        column: binding.header.clone(),
        raw: raw.to_string(),
        message,
        severity,
    }
}

fn raw_of(cells: &[StagedCell], field: FieldKind) -> Option<&str> {
    cells
        .iter()
        .find(|cell| cell.field == field)
        .map(|cell| cell.raw.as_str())
}

/// WHO SENT THE FILE, which is what scopes a Radsatznummer — see
/// `resolve::radsatz`. The template's partner is the reliable answer because the
/// user bound it once; a row's Werkstatt is the fallback and only when CONFIRMED,
/// since a `Likely` is a suggestion and would scope an alias to a guess.
fn sender_of(input: &StageInput<'_>, werkstatt: &Resolution) -> Option<String> {
    let from_template = input
        .plan
        .template_id
        .as_deref()
        .and_then(|id| input.db.template(id))
        .and_then(|template| template.partner_id.clone());
    if from_template.is_some() {
        return from_template;
    }
    match werkstatt {
        Resolution::Known { id, .. } => Some(id.clone()),
        _ => None,
    }
}

fn dedupe_key(
    nummer: &Option<String>,
    values: &[(FieldKind, Value)],
    werkstatt: Option<&str>,
    radsatz: Option<&str>,
) -> String {
    let find = |wanted: FieldKind| {
        values
            .iter()
            .find(|(field, _)| *field == wanted)
            .map(|(_, value)| format::value(value))
            .unwrap_or_default()
    };
    let mut parts = vec![
        nummer.as_deref().unwrap_or("").to_string(),
        find(FieldKind::Datum),
        resolve::partner::match_key(werkstatt.unwrap_or("")),
        find(FieldKind::Leistung),
        find(FieldKind::Betrag),
        resolve::radsatz::match_key(radsatz.unwrap_or("")),
        find(FieldKind::EingebautAm),
        find(FieldKind::AusgebautAm),
    ];
    let zustand: Vec<String> = ZUSTAND_KEYS.iter().map(|field| find(*field)).collect();
    if zustand.iter().any(|part| !part.is_empty()) {
        parts.extend(zustand);
    }
    let parts: Vec<&str> = parts.iter().map(String::as_str).collect();
    hash::join(&parts)
}

const ZUSTAND_KEYS: [FieldKind; 5] = [
    FieldKind::Bestellnummer,
    FieldKind::SchadenGemeldetAm,
    FieldKind::Schadcode,
    FieldKind::Pruefart,
    FieldKind::PruefungFaelligAm,
];

fn summarise(rows: &[StagedRow]) -> StagedSummary {
    let mut summary = StagedSummary {
        total: rows.len() as u32,
        ..StagedSummary::default()
    };
    for row in rows {
        match row.status {
            RowStatus::Ready => summary.ready += 1,
            RowStatus::NeedsInput => summary.needs_input += 1,
            RowStatus::Duplicate => summary.duplicates += 1,
            RowStatus::Rejected => summary.rejected += 1,
        }
        if matches!(row.wagen, Resolution::New { .. }) {
            summary.neue_wagen += 1;
        }
        if matches!(row.radsatz, Resolution::New { .. }) {
            summary.neue_radsaetze += 1;
        }
        for resolution in [&row.werkstatt, &row.halter, &row.eigentuemer] {
            if matches!(resolution, Resolution::New { .. }) {
                summary.neue_partner += 1;
            }
        }
    }
    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TempDir;
    use crate::trains::model::{Partner, Wagen};
    use crate::trains::sheet::layout::LayoutHint;
    use crate::trains::sheet::readers::ReaderKind;

    fn grid() -> Grid {
        Grid::from_text(
            "Tabelle1",
            &[
                &["Wagennummer", "Datum", "Werkstatt", "Leistung", "Kosten"],
                &[
                    "31 80 4740 123-4",
                    "31.12.2025",
                    "Fa. Müller GmbH",
                    "Bremsprobe",
                    "1.234,56",
                ],
                &[
                    "21 81 2471 217-3",
                    "01.01.2026",
                    "Bahnwerk Nord",
                    "Radsatz",
                    "987,00",
                ],
            ],
        )
    }

    fn plan(fields: &[(u32, &str, FieldKind)]) -> ImportPlan {
        ImportPlan {
            reader: ReaderKind::HeaderRow,
            layout: LayoutHint {
                header_row: Some(1),
                first_data_row: 2,
                last_data_row: None,
            },
            columns: fields
                .iter()
                .map(|(index, header, field)| ColumnBinding {
                    header: (*header).into(),
                    index: *index,
                    field: *field,
                    decimal: None,
                    date_order: None,
                })
                .collect(),
            template_id: None,
            date1904: false,
            pruefart: None,
        }
    }

    fn full_plan() -> ImportPlan {
        plan(&[
            (1, "Wagennummer", FieldKind::Wagennummer),
            (2, "Datum", FieldKind::Datum),
            (3, "Werkstatt", FieldKind::Werkstatt),
            (4, "Leistung", FieldKind::Leistung),
            (5, "Kosten", FieldKind::Betrag),
        ])
    }

    fn run(grid: &Grid, plan: &ImportPlan, db: &TrainsDb) -> AppResult<StagedImport> {
        run_full(grid, plan, db).map(|staged| staged.wire)
    }

    fn run_full(grid: &Grid, plan: &ImportPlan, db: &TrainsDb) -> AppResult<Staged> {
        stage(StageInput {
            id: "s1".into(),
            file: "monat.xlsx".into(),
            sheets: vec!["Tabelle1".into()],
            candidates: Vec::new(),
            grid,
            plan,
            db,
            master: false,
        })
    }

    fn empty_db(label: &str) -> (TempDir, TrainsDb) {
        let folder = TempDir::new(label);
        let db = TrainsDb::load(&folder.config()).unwrap();
        (folder, db)
    }

    /// The mapping screen re-stages on every pick, so a half-mapped plan is the
    /// NORMAL state. Refusing it here made the first pick fail and nothing
    /// mappable at all; `commit` is where required-ness is checked.
    #[test]
    fn a_half_mapped_plan_stages_instead_of_failing() {
        let (_f, db) = empty_db("stage-partial");
        let staged = run(
            &grid(),
            &plan(&[(1, "Wagennummer", FieldKind::Wagennummer)]),
            &db,
        )
        .unwrap();
        assert_eq!(staged.summary.total, 2);
    }

    /// The columns that most need sample values are the UNMAPPED ones — that is
    /// how the user works out what they are.
    #[test]
    fn an_ignored_column_still_carries_its_raw_text() {
        let (_f, db) = empty_db("stage-ignored-raw");
        let staged = run(
            &grid(),
            &plan(&[
                (1, "Wagennummer", FieldKind::Wagennummer),
                (3, "Werkstatt", FieldKind::Ignorieren),
            ]),
            &db,
        )
        .unwrap();
        let cell = staged.rows[0]
            .cells
            .iter()
            .find(|cell| cell.column == 3)
            .expect("the ignored column is still there");
        assert_eq!(cell.raw, "Fa. Müller GmbH");
        assert_eq!(cell.parsed, "");
    }

    /// Cells are matched by COLUMN, so two columns mapped to one field stay
    /// distinguishable rather than merging.
    #[test]
    fn every_cell_names_the_column_it_came_from() {
        let (_f, db) = empty_db("stage-columns");
        let staged = run(&grid(), &full_plan(), &db).unwrap();
        let columns: Vec<u32> = staged.rows[0]
            .cells
            .iter()
            .map(|cell| cell.column)
            .collect();
        assert_eq!(columns, [1, 2, 3, 4, 5]);
    }

    #[test]
    fn every_data_row_is_staged_with_what_it_parsed_to() {
        let (_f, db) = empty_db("stage-rows");
        let staged = run(&grid(), &full_plan(), &db).unwrap();
        assert_eq!(staged.summary.total, 2);
        assert_eq!(staged.rows[0].row, 2);

        let cells = &staged.rows[0].cells;
        let parsed = |field: FieldKind| {
            cells
                .iter()
                .find(|c| c.field == field)
                .unwrap()
                .parsed
                .as_str()
        };
        assert_eq!(parsed(FieldKind::Wagennummer), "318047401234");
        assert_eq!(parsed(FieldKind::Datum), "31.12.2025");
        assert_eq!(parsed(FieldKind::Betrag), "1234,56");
    }

    /// Both halves side by side is what the preview is for.
    #[test]
    fn a_cell_keeps_the_raw_text_beside_what_it_became() {
        let (_f, db) = empty_db("stage-raw");
        let staged = run(&grid(), &full_plan(), &db).unwrap();
        let cell = staged.rows[0]
            .cells
            .iter()
            .find(|c| c.field == FieldKind::Wagennummer)
            .unwrap();
        assert_eq!(cell.raw, "31 80 4740 123-4");
        assert_eq!(cell.parsed, "318047401234");
        assert!(cell.ok);
    }

    /// The number in the original request has a wrong check digit. It is still
    /// staged — with a warning, and needing confirmation because it is unknown.
    #[test]
    fn a_wrong_check_digit_warns_but_does_not_reject_the_row() {
        let (_f, db) = empty_db("stage-checkdigit");
        let staged = run(&grid(), &full_plan(), &db).unwrap();
        let row = &staged.rows[0];
        assert_ne!(row.status, RowStatus::Rejected);
        assert!(row
            .issues
            .iter()
            .any(|issue| issue.message.contains("Prüfziffer")));
    }

    #[test]
    fn a_row_whose_required_cell_cannot_be_read_is_rejected_and_the_rest_survive() {
        let broken = Grid::from_text(
            "Tabelle1",
            &[
                &["Wagennummer", "Datum"],
                &["kein Wagen", "31.12.2025"],
                &["21 81 2471 217-3", "01.01.2026"],
            ],
        );
        let (_f, db) = empty_db("stage-rejected");
        let staged = run(
            &broken,
            &plan(&[
                (1, "Wagennummer", FieldKind::Wagennummer),
                (2, "Datum", FieldKind::Datum),
            ]),
            &db,
        )
        .unwrap();
        assert_eq!(staged.summary.total, 2);
        assert_eq!(staged.summary.rejected, 1);
        assert_eq!(staged.rows[0].status, RowStatus::Rejected);
        assert!(staged.rows[0]
            .issues
            .iter()
            .any(|i| i.severity == Severity::Fehler));
        assert_ne!(staged.rows[1].status, RowStatus::Rejected);
    }

    #[test]
    fn an_unknown_wagen_and_workshop_make_the_row_need_input() {
        let (_f, db) = empty_db("stage-needs");
        let staged = run(&grid(), &full_plan(), &db).unwrap();
        assert_eq!(staged.summary.needs_input, 2);
        assert_eq!(staged.summary.neue_wagen, 2);
        assert_eq!(staged.summary.neue_partner, 2);
        assert_eq!(staged.rows[0].status, RowStatus::NeedsInput);
    }

    #[test]
    fn known_entities_make_a_row_ready() {
        let folder = TempDir::new("stage-ready");
        let mut db = TrainsDb::load(&folder.config()).unwrap();
        db.transaction(|tx| {
            for (id, uic) in [("w1", "318047401234"), ("w2", "218124712173")] {
                tx.put_wagen(Wagen {
                    id: id.into(),
                    nummer: uic.into(),
                    halter_id: None,
                    eigentuemer_id: None,
                    bauart: None,
                    bemerkung: None,
                    created_at: "2026-08-16".into(),
                    source: None,
                });
            }
            for (id, name) in [("p1", "Müller GmbH"), ("p2", "Bahnwerk Nord")] {
                tx.put_partner(Partner {
                    id: id.into(),
                    rollen: vec![PartnerRolle::Werkstatt],
                    name: name.into(),
                    match_key: crate::trains::resolve::partner::match_key(name),
                    aliases: Vec::new(),
                    bemerkung: None,
                    created_at: "2026-08-16".into(),
                });
            }
            Ok(())
        })
        .unwrap();

        let staged = run(&grid(), &full_plan(), &db).unwrap();
        assert_eq!(staged.summary.ready, 2, "{:?}", staged.rows[0]);
        assert_eq!(staged.summary.neue_wagen, 0);
    }

    /// Two identical rows in ONE file collide with each other, not just with the
    /// store — so re-sending a file cannot double an event either way.
    #[test]
    fn an_identical_row_twice_in_one_file_is_flagged_as_duplicate() {
        let repeated = Grid::from_text(
            "Tabelle1",
            &[
                &["Wagennummer", "Datum"],
                &["21 81 2471 217-3", "01.01.2026"],
                &["21 81 2471 217-3", "01.01.2026"],
            ],
        );
        let (_f, db) = empty_db("stage-dupe");
        let staged = run(
            &repeated,
            &plan(&[
                (1, "Wagennummer", FieldKind::Wagennummer),
                (2, "Datum", FieldKind::Datum),
            ]),
            &db,
        )
        .unwrap();
        assert_eq!(staged.summary.duplicates, 1);
        assert_eq!(staged.rows[0].status, RowStatus::NeedsInput);
        assert_eq!(staged.rows[1].status, RowStatus::Duplicate);
    }

    // Shape of a real wheelset-monitoring export: one row per FITTED radsatz,
    // four per wagen, no Datum, Leistung or Betrag. Four radsaetze on one wagen
    // are four fittings, not one fitting sent four times.
    #[test]
    fn four_radsaetze_on_one_wagen_are_not_duplicates_of_each_other() {
        let fitted = Grid::from_text(
            "Radsatzmonitoring",
            &[
                &["Wagennr.", "Radsatznummer", "Einbaudatum"],
                &["21 81 2471 217-3", "AL240720", "02.08.2024"],
                &["21 81 2471 217-3", "AL240719", "02.08.2024"],
                &["21 81 2471 217-3", "AL240710", "02.08.2024"],
                &["21 81 2471 217-3", "AL240717", "02.08.2024"],
            ],
        );
        let (_f, db) = empty_db("stage-fitted");
        let staged = run(
            &fitted,
            &plan(&[
                (1, "Wagennr.", FieldKind::Wagennummer),
                (2, "Radsatznummer", FieldKind::Radsatznummer),
                (3, "Einbaudatum", FieldKind::EingebautAm),
            ]),
            &db,
        )
        .unwrap();
        assert_eq!(staged.summary.duplicates, 0, "{:?}", staged.rows);
    }

    #[test]
    fn a_blank_row_is_spacing_and_is_not_staged() {
        let spaced = Grid::from_text(
            "Tabelle1",
            &[
                &["Wagennummer", "Datum"],
                &["21 81 2471 217-3", "01.01.2026"],
                &["", ""],
                &["31 80 4740 123-4", "02.01.2026"],
            ],
        );
        let (_f, db) = empty_db("stage-blank");
        let staged = run(
            &spaced,
            &plan(&[
                (1, "Wagennummer", FieldKind::Wagennummer),
                (2, "Datum", FieldKind::Datum),
            ]),
            &db,
        )
        .unwrap();
        assert_eq!(staged.summary.total, 2);
    }

    /// The ambiguous-decimal case: every cell is `d.ddd`, so nothing in the
    /// column settles it and every row says so.
    #[test]
    fn an_undecidable_amount_column_warns_on_every_row() {
        let ambiguous = Grid::from_text(
            "Tabelle1",
            &[
                &["Wagennummer", "Datum", "Kosten"],
                &["21 81 2471 217-3", "01.01.2026", "1.234"],
                &["31 80 4740 123-4", "02.01.2026", "5.678"],
            ],
        );
        let (_f, db) = empty_db("stage-ambiguous");
        let staged = run(
            &ambiguous,
            &plan(&[
                (1, "Wagennummer", FieldKind::Wagennummer),
                (2, "Datum", FieldKind::Datum),
                (3, "Kosten", FieldKind::Betrag),
            ]),
            &db,
        )
        .unwrap();
        for row in &staged.rows {
            assert!(
                row.issues
                    .iter()
                    .any(|i| i.message.contains("Dezimaltrennzeichen")),
                "{:?}",
                row.issues
            );
        }
        let amount = |row: &StagedRow| {
            row.cells
                .iter()
                .find(|c| c.field == FieldKind::Betrag)
                .unwrap()
                .parsed
                .clone()
        };
        assert_eq!(amount(&staged.rows[0]), "1234,00");
    }

    /// One conclusive cell rescues the column, and then nothing warns.
    #[test]
    fn one_conclusive_amount_settles_the_column_and_silences_the_warning() {
        let settled = Grid::from_text(
            "Tabelle1",
            &[
                &["Wagennummer", "Datum", "Kosten"],
                &["21 81 2471 217-3", "01.01.2026", "1.234"],
                &["31 80 4740 123-4", "02.01.2026", "5.678,90"],
            ],
        );
        let (_f, db) = empty_db("stage-settled");
        let staged = run(
            &settled,
            &plan(&[
                (1, "Wagennummer", FieldKind::Wagennummer),
                (2, "Datum", FieldKind::Datum),
                (3, "Kosten", FieldKind::Betrag),
            ]),
            &db,
        )
        .unwrap();
        for row in &staged.rows {
            assert!(!row
                .issues
                .iter()
                .any(|i| i.message.contains("Dezimaltrennzeichen")));
        }
    }

    /// A column holding BOTH readings — `13/01` can only be day-first, `01/13`
    /// only month-first — is the case the warning exists for. `infer_date_order`
    /// answers `None` there, which must stay uncertain rather than fall back to
    /// a confident default and say nothing — on the row whose value depends on
    /// the answer.
    #[test]
    fn a_column_contradicting_itself_on_day_and_month_still_warns() {
        let contradictory = Grid::from_text(
            "Tabelle1",
            &[
                &["Wagennummer", "Datum"],
                &["21 81 2471 217-3", "13/01/2026"],
                &["21 81 2471 217-3", "01/13/2026"],
                &["21 81 2471 217-3", "03/04/2026"],
            ],
        );
        let (_f, db) = empty_db("stage-contradiction");
        let staged = run(
            &contradictory,
            &plan(&[
                (1, "Wagennummer", FieldKind::Wagennummer),
                (2, "Datum", FieldKind::Datum),
            ]),
            &db,
        )
        .unwrap();

        assert!(
            staged
                .rows
                .iter()
                .flat_map(|row| &row.issues)
                .any(|issue| issue.row == 4 && issue.message.contains("beiden Reihenfolgen")),
            "{:?}",
            staged.rows
        );
    }

    #[test]
    fn an_ignored_column_contributes_nothing() {
        let (_f, db) = empty_db("stage-ignore");
        let mut plan = full_plan();
        plan.columns[3].field = FieldKind::Ignorieren;
        let staged = run(&grid(), &plan, &db).unwrap();
        assert!(staged.rows[0]
            .cells
            .iter()
            .all(|cell| cell.field != FieldKind::Leistung));
        assert!(staged.rows[0]
            .cells
            .iter()
            .any(|cell| cell.column == 4 && cell.field == FieldKind::Ignorieren));
    }
}
