// ─── why ────────────────────────────────────────────────────────
// One sheet of the customer's master, cleaned of what nothing in it can mean —
// and of nothing else. The workbook is a hub of lookups the customer built, so
// the rules are about what a change could BREAK, not about what would be tidier:
//
//   • A FORMULA INSIDE THE DATA IS NEVER TOUCHED. Not its text, not its cached
//     value, not its cell — `is_formula()` covers shared-formula children too.
//   • NO ROW MOVES. Removing a row inside the data would shift every reference
//     below it. Only the tail is cut: cells holding nothing, below the last row
//     that holds a value or a formula — styled blanks and their row records,
//     which is what makes a sheet „reach“ row 1,048,576 without content.
//   • A TAIL of rows holding nothing but `0` or `#NV` — in the real master a
//     formula pulled down to the end of the sheet — keeps its first three rows
//     and loses the rest (Martin's call, 2026-10-04). Three rows keep the
//     pattern a person extends by dragging; the other million are what makes
//     every read of the sheet cost seconds. It is cut only when nothing but
//     such rows follows the last real one, and it is counted apart
//     (`tail_rows_cut`, from `formula_tail`) so the report SAYS formulas went.
//     A shared formula whose range reached into the cut ends at the last kept
//     row, or Excel would read a range of formulas that no longer exist.
//   • A value is only RETYPED where its column already says what it is. Text in
//     a column whose cells are mostly numbers becomes the number it spells —
//     which mends a VLOOKUP keyed on it — and text in a column mostly holding
//     dates becomes that date, in the column's own date format. Only spellings
//     that read ONE way: no leading zero (an identifier, not a number), no
//     `1,234` (thousands or decimals?), no `01/02/2025`, no two-digit year.
//     Whatever looks like a number or a date and is not that unambiguous is a
//     NOTE, never a guess.
//   • Surrounding whitespace in text goes, NBSP included — lookups miss on it.
//
// The header row is left as it is: other parts of the app bind sheets by header
// text, and a formula may MATCH on it.
// ────────────────────────────────────────────────────────────────

use std::collections::{BTreeSet, HashMap};

use umya_spreadsheet::{CellRawValue, Worksheet};

use crate::trains::model::{
    DateOrder, MasterFileChange, MasterFileNote, MasterFileRule, MasterFileSheet,
};
use crate::trains::sanitise::{date, wagen, Value};
use crate::trains::sheet::grid;

const EXAMPLES: usize = 5;
const NOTES: usize = 50;
const TAIL_KEPT: u32 = 3;
const MAX_DIGITS: usize = 15;

pub struct Outcome {
    pub report: MasterFileSheet,
    pub changes: Vec<MasterFileChange>,
    pub notes: Vec<MasterFileNote>,
}

#[derive(Default, Clone)]
struct Shape {
    numbers: u32,
    dates: u32,
    texts: u32,
    date_format: Option<String>,
}

impl Shape {
    fn numeric(&self) -> bool {
        self.numbers > self.texts && self.numbers > self.dates
    }

    fn dated(&self) -> bool {
        self.dates > self.texts && self.dates > self.numbers && self.date_format.is_some()
    }
}

enum Retype {
    Number(f64),
    Date(f64),
}

pub fn sheet(worksheet: &mut Worksheet) -> Outcome {
    let (rows_cut, tail_rows_cut, formula_tail) = cut_tail(worksheet);
    let headers = headers(worksheet);
    let shapes = shapes(worksheet);

    let mut changes = Vec::new();
    let mut notes = Vec::new();
    for (col, row, raw) in texts(worksheet) {
        let header = headers.get(&col).cloned().unwrap_or_default();
        let shape = shapes.get(&col).cloned().unwrap_or_default();
        let trimmed = trim(&raw);

        let (retype, note) = if shape.numeric() {
            match number(trimmed) {
                Some(value) => (Some(Retype::Number(value)), None),
                None if numberish(trimmed) => (None, Some("Zahl als Text, nicht eindeutig lesbar")),
                None => (None, None),
            }
        } else if shape.dated() {
            match serial(trimmed) {
                Some(value) => (Some(Retype::Date(value)), None),
                None if dateish(trimmed) => (None, Some("Datum als Text, Lesart nicht eindeutig")),
                None => (None, None),
            }
        } else {
            (None, None)
        };

        if let Some(reason) = note {
            notes.push(MasterFileNote {
                row,
                column: col,
                header: header.clone(),
                raw: raw.clone(),
                reason: reason.into(),
            });
        }

        let cell = worksheet.cell_mut((col, row));
        let (clean, rule) = match retype {
            Some(Retype::Number(value)) => {
                cell.set_value_number(value);
                (number_text(value), MasterFileRule::Number)
            }
            Some(Retype::Date(value)) => {
                cell.set_value_number(value);
                let dated = cell
                    .style()
                    .number_format()
                    .is_some_and(|format| grid::is_date_format(format.format_code()));
                if let (false, Some(code)) = (dated, &shape.date_format) {
                    cell.style_mut()
                        .number_format_mut()
                        .set_format_code(code.as_str());
                }
                (trimmed.to_string(), MasterFileRule::Date)
            }
            None if trimmed != raw => {
                if trimmed.is_empty() {
                    cell.set_blank();
                } else {
                    cell.set_value_string(trimmed);
                }
                (trimmed.to_string(), MasterFileRule::Trimmed)
            }
            None => continue,
        };
        changes.push(MasterFileChange {
            row,
            column: col,
            header,
            raw,
            clean,
            rule,
        });
    }

    let count = |rule| changes.iter().filter(|change| change.rule == rule).count() as u32;
    let mut examples = Vec::new();
    for rule in [
        MasterFileRule::Number,
        MasterFileRule::Date,
        MasterFileRule::Trimmed,
    ] {
        examples.extend(
            changes
                .iter()
                .filter(|change| change.rule == rule)
                .take(EXAMPLES)
                .cloned(),
        );
    }
    let report = MasterFileSheet {
        sheet: worksheet.name().to_string(),
        rows_cut,
        tail_rows_cut,
        formula_tail,
        trimmed: count(MasterFileRule::Trimmed),
        numbers: count(MasterFileRule::Number),
        dates: count(MasterFileRule::Date),
        examples,
        notes: notes.iter().take(NOTES).cloned().collect(),
        note_count: notes.len() as u32,
    };
    Outcome {
        report,
        changes,
        notes,
    }
}

fn cut_tail(worksheet: &mut Worksheet) -> (u32, u32, Option<u32>) {
    let mut last_kept = 0;
    let mut last_real = 0;
    for cell in worksheet.cells() {
        let row = cell.coordinate().row_num();
        let value = cell.value();
        let value = value.trim();
        if !value.is_empty() && value != "0" && !grid::is_error(value) {
            last_real = last_real.max(row);
        }
        if !value.is_empty() || cell.is_formula() {
            last_kept = last_kept.max(row);
        }
    }
    let tail = last_kept > last_real + TAIL_KEPT;
    let boundary = if tail {
        last_real + TAIL_KEPT
    } else {
        last_kept
    };

    let below: Vec<(u32, u32)> = worksheet
        .cells()
        .into_iter()
        .map(|cell| (cell.coordinate().col_num(), cell.coordinate().row_num()))
        .filter(|(_, row)| *row > boundary)
        .collect();
    let mut rows: BTreeSet<u32> = below.iter().map(|(_, row)| *row).collect();
    for coordinate in below {
        worksheet.remove_cell(coordinate);
    }
    let dimensions = worksheet.row_dimensions_to_hashmap_mut();
    rows.extend(dimensions.keys().copied().filter(|row| *row > boundary));
    dimensions.retain(|row, _| *row <= boundary);
    if tail {
        end_shared_ranges(worksheet, boundary);
    }

    let tail_rows = rows.iter().filter(|row| **row <= last_kept).count() as u32;
    (
        rows.len() as u32 - tail_rows,
        tail_rows,
        tail.then_some(last_real + 1),
    )
}

fn end_shared_ranges(worksheet: &mut Worksheet, last: u32) {
    let reaching: Vec<((u32, u32), String)> = worksheet
        .cells()
        .into_iter()
        .filter_map(|cell| {
            let reference = cell.formula_obj()?.reference();
            let clamped = clamp(reference, last)?;
            Some((
                (cell.coordinate().col_num(), cell.coordinate().row_num()),
                clamped,
            ))
        })
        .collect();
    for (coordinate, reference) in reaching {
        let cell = worksheet.cell_mut(coordinate);
        if let Some(mut formula) = cell.formula_obj().cloned() {
            formula.set_reference(reference);
            cell.cell_value_mut().set_formula_obj(formula);
        }
    }
}

fn clamp(reference: &str, last: u32) -> Option<String> {
    let (start, end) = reference.split_once(':')?;
    let digits = end.find(|c: char| c.is_ascii_digit())?;
    let (column, row) = end.split_at(digits);
    let row: u32 = row.parse().ok()?;
    (row > last).then(|| format!("{start}:{column}{last}"))
}

fn headers(worksheet: &Worksheet) -> HashMap<u32, String> {
    worksheet
        .cells()
        .into_iter()
        .filter(|cell| cell.coordinate().row_num() == 1)
        .map(|cell| (cell.coordinate().col_num(), cell.value().trim().to_string()))
        .collect()
}

fn shapes(worksheet: &Worksheet) -> HashMap<u32, Shape> {
    let mut shapes: HashMap<u32, Shape> = HashMap::new();
    for cell in worksheet.cells() {
        if cell.coordinate().row_num() < 2 {
            continue;
        }
        let shape = shapes.entry(cell.coordinate().col_num()).or_default();
        let format = cell
            .style()
            .number_format()
            .map(|format| format.format_code())
            .filter(|code| grid::is_date_format(code));
        match cell.raw_value() {
            CellRawValue::Numeric(_) => match format {
                Some(code) => {
                    shape.dates += 1;
                    shape.date_format.get_or_insert_with(|| code.to_string());
                }
                None => shape.numbers += 1,
            },
            CellRawValue::String(text) if !text.trim().is_empty() => shape.texts += 1,
            CellRawValue::RichText(_) => shape.texts += 1,
            _ => {}
        }
    }
    shapes
}

fn texts(worksheet: &Worksheet) -> Vec<(u32, u32, String)> {
    let mut texts: Vec<(u32, u32, String)> = worksheet
        .cells()
        .into_iter()
        .filter(|cell| cell.coordinate().row_num() >= 2 && !cell.is_formula())
        .filter_map(|cell| match cell.raw_value() {
            CellRawValue::String(text) => Some((
                cell.coordinate().col_num(),
                cell.coordinate().row_num(),
                text.to_string(),
            )),
            _ => None,
        })
        .collect();
    texts.sort_by_key(|(col, row, _)| (*row, *col));
    texts
}

fn trim(raw: &str) -> &str {
    raw.trim_matches(|c: char| c.is_whitespace() || c == '\u{200b}' || c == '\u{feff}')
}

fn number(text: &str) -> Option<f64> {
    let unsigned = text.strip_prefix('-').unwrap_or(text);
    let (whole, fraction) = match unsigned.find(['.', ',']) {
        Some(at) => (&unsigned[..at], Some(&unsigned[at + 1..])),
        None => (unsigned, None),
    };
    let digits = |part: &str| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit());
    let plain = digits(whole)
        && (whole == "0" || !whole.starts_with('0'))
        && whole.len() + fraction.map_or(0, str::len) <= MAX_DIGITS
        && fraction.is_none_or(|fraction| digits(fraction) && fraction.len() != 3);
    if plain {
        return text.replace(',', ".").parse().ok();
    }
    match wagen::parse(text) {
        Ok(parsed) if parsed.warning.is_none() => match parsed.value {
            Value::Uic(uic) => uic.as_str().parse().ok(),
            _ => None,
        },
        _ => None,
    }
}

fn numberish(text: &str) -> bool {
    text.chars().any(|c| c.is_ascii_digit())
        && text
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '.' | ',' | '-' | ' ' | '\''))
}

fn serial(text: &str) -> Option<f64> {
    let parts: Vec<&str> = text.split(['.', '-']).collect();
    let sizes: Vec<usize> = parts.iter().map(|part| part.len()).collect();
    let numeric = parts
        .iter()
        .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()));
    let dotted =
        text.contains('.') && !text.contains('-') && matches!(sizes[..], [1 | 2, 1 | 2, 4]);
    let iso = text.contains('-') && !text.contains('.') && sizes[..] == [4, 2, 2];
    if !numeric || !(dotted || iso) {
        return None;
    }
    match date::parse_text(text, DateOrder::DayFirst) {
        Ok(parsed) if parsed.warning.is_none() => match parsed.value {
            Value::Date(value) if (1900..=2100).contains(&value.year) => Some(
                (date::days_from_civil(value.year, value.month, value.day)
                    - date::days_from_civil(1899, 12, 30)) as f64,
            ),
            _ => None,
        },
        _ => None,
    }
}

fn dateish(text: &str) -> bool {
    text.chars().any(|c| c.is_ascii_digit())
        && text.contains(['.', '-', '/'])
        && text
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '/' | ' ' | ':'))
}

fn number_text(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn worksheet(rows: &[&[&str]]) -> Worksheet {
        let mut book = umya_spreadsheet::new_file();
        let sheet = book.sheet_mut(0).unwrap();
        for (row, cells) in rows.iter().enumerate() {
            for (col, value) in cells.iter().enumerate() {
                let cell = sheet.cell_mut((col as u32 + 1, row as u32 + 1));
                if let Some(formula) = value.strip_prefix('=') {
                    cell.set_formula(formula);
                    cell.set_formula_result_number(0);
                } else if let Some(number) =
                    value.strip_prefix('#').and_then(|n| n.parse::<f64>().ok())
                {
                    cell.set_value_number(number);
                } else if !value.is_empty() {
                    cell.set_value_string(*value);
                }
            }
        }
        sheet.clone()
    }

    fn date_cell(sheet: &mut Worksheet, col: u32, row: u32, serial: f64) {
        let cell = sheet.cell_mut((col, row));
        cell.set_value_number(serial);
        cell.style_mut()
            .number_format_mut()
            .set_format_code("DD.MM.YYYY");
    }

    fn text(sheet: &Worksheet, col: u32, row: u32) -> String {
        sheet
            .cell((col, row))
            .map(|cell| cell.value().to_string())
            .unwrap_or_default()
    }

    #[test]
    fn text_in_a_numeric_column_becomes_the_number_it_spells() {
        let mut sheet = worksheet(&[
            &["Wagennummer", "Auftrag"],
            &["#338506591522", "#180028676"],
            &["#338506591530", "#180028677"],
            &["3385 0659 152-2", "180028678"],
        ]);
        let outcome = super::sheet(&mut sheet);

        assert_eq!(outcome.report.numbers, 2);
        assert_eq!(
            sheet.cell((1, 4)).unwrap().value_number(),
            Some(338506591522.0)
        );
        assert_eq!(
            sheet.cell((2, 4)).unwrap().value_number(),
            Some(180028678.0)
        );
        assert_eq!(outcome.changes[0].header, "Wagennummer");
    }

    /// An identifier with a leading zero, or `1,234`, could mean two things.
    #[test]
    fn an_ambiguous_number_is_noted_and_left_as_text() {
        let mut sheet = worksheet(&[
            &["Betrag"],
            &["#1"],
            &["#2"],
            &["#3"],
            &["#4"],
            &["0123"],
            &["1,234"],
            &["12,5"],
        ]);
        let outcome = super::sheet(&mut sheet);

        assert_eq!(text(&sheet, 1, 6), "0123");
        assert_eq!(text(&sheet, 1, 7), "1,234");
        assert_eq!(sheet.cell((1, 8)).unwrap().value_number(), Some(12.5));
        assert_eq!(outcome.report.note_count, 2);
    }

    /// A text column stays text: `Bemerkung` holding a number is still prose.
    #[test]
    fn a_column_is_only_retyped_where_it_already_says_what_it_is() {
        let mut sheet = worksheet(&[&["Bemerkung"], &["neu"], &["alt"], &["42"]]);
        let outcome = super::sheet(&mut sheet);
        assert!(outcome.changes.is_empty());
        assert_eq!(text(&sheet, 1, 4), "42");
    }

    #[test]
    fn a_text_date_in_a_date_column_becomes_a_serial_in_the_columns_format() {
        let mut sheet = worksheet(&[
            &["Einbau"],
            &[""],
            &[""],
            &[""],
            &["31.12.2025"],
            &["01/02/2025"],
        ]);
        for row in 2..=4 {
            date_cell(&mut sheet, 1, row, 45000.0);
        }
        let outcome = super::sheet(&mut sheet);

        let cell = sheet.cell((1, 5)).unwrap();
        assert_eq!(cell.value_number(), Some(46022.0));
        assert_eq!(
            cell.style().number_format().unwrap().format_code(),
            "DD.MM.YYYY"
        );
        assert_eq!(outcome.report.dates, 1);
        // Day or month first? Not ours to decide.
        assert_eq!(text(&sheet, 1, 6), "01/02/2025");
        assert_eq!(outcome.report.note_count, 1);
    }

    #[test]
    fn surrounding_whitespace_goes_and_the_header_stays() {
        let mut sheet = worksheet(&[&[" Stadt "], &["  Köln\u{a0}"], &["Bonn"]]);
        let outcome = super::sheet(&mut sheet);

        assert_eq!(text(&sheet, 1, 1), " Stadt ");
        assert_eq!(text(&sheet, 1, 2), "Köln");
        assert_eq!(text(&sheet, 1, 3), "Bonn");
        assert_eq!(outcome.report.trimmed, 1);
    }

    #[test]
    fn a_formula_is_never_touched() {
        let mut sheet = worksheet(&[&["Wert"], &["#1"], &["#2"], &["=A2+A3"]]);
        let outcome = super::sheet(&mut sheet);
        assert!(outcome.changes.is_empty());
        assert_eq!(sheet.cell((1, 4)).unwrap().formula(), "A2+A3");
    }

    /// Styled blanks below the data go; nothing above them moves.
    #[test]
    fn only_empty_cells_below_the_last_row_are_cut() {
        let mut sheet = worksheet(&[&["Wert"], &["#1"], &[""], &["#0"]]);
        sheet
            .cell_mut((1, 50))
            .style_mut()
            .number_format_mut()
            .set_format_code("0.00");
        sheet.cell_mut((2, 60));
        let outcome = super::sheet(&mut sheet);

        assert_eq!(outcome.report.rows_cut, 2);
        assert!(sheet.cell((1, 50)).is_none());
        // A typed zero is a value: kept, and so is the row above it.
        assert_eq!(sheet.cell((1, 4)).unwrap().value_number(), Some(0.0));
        assert_eq!(outcome.report.formula_tail, None);
    }

    /// The real master's tail: a formula pulled to the end of the sheet.
    #[test]
    fn a_formula_tail_keeps_three_rows_and_loses_the_rest() {
        let mut sheet = worksheet(&[&["Wert", "Summe"], &["#7", "#7"]]);
        for row in 3..=1200 {
            let cell = sheet.cell_mut((2, row));
            cell.set_formula("A2*0");
            cell.set_formula_result_number(0);
        }
        sheet.cell_mut((1, 1300));
        let outcome = super::sheet(&mut sheet);

        assert_eq!(outcome.report.formula_tail, Some(3));
        assert_eq!(outcome.report.tail_rows_cut, 1195);
        assert_eq!(outcome.report.rows_cut, 1);
        assert_eq!(sheet.cell((2, 5)).unwrap().formula(), "A2*0");
        assert!(sheet.cell((2, 6)).is_none());
        assert!(sheet.cell((2, 1200)).is_none());
    }

    /// A row of real data after the zeros means they are not a tail.
    #[test]
    fn zeros_followed_by_data_are_not_a_tail() {
        let mut sheet = worksheet(&[
            &["Wert"],
            &["#7"],
            &["#0"],
            &["#0"],
            &["#0"],
            &["#0"],
            &["#8"],
        ]);
        let outcome = super::sheet(&mut sheet);
        assert_eq!(outcome.report.tail_rows_cut, 0);
        assert_eq!(outcome.report.formula_tail, None);
        assert_eq!(sheet.cell((1, 6)).unwrap().value_number(), Some(0.0));
    }

    #[test]
    fn a_shared_range_reaching_into_the_cut_ends_at_the_last_kept_row() {
        assert_eq!(clamp("C2:C1048576", 5).as_deref(), Some("C2:C5"));
        assert_eq!(clamp("C2:D4", 5), None);
        assert_eq!(clamp("", 5), None);
    }
}
