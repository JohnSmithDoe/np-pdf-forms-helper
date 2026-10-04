// ─── why ────────────────────────────────────────────────────────
// A filed Dokument as the master wants it: its header row plus every data row,
// each cell TYPED. The master's lookups are keyed on Wagennummern stored as
// numbers and its dates are serials, so a text `338506590011` or `31.12.2025`
// would make every VLOOKUP over the sheet answer `#N/A`. That is the whole reason
// this is not a cell copy.
//
// The source is the CLEANED copy, never the original. Every mapped value in it is
// canonical — twelve digits (or the grouped spelling), `dd.mm.yyyy`, `1234,56` —
// so ONE fixed reading per kind converts it exactly, and the column questions
// `reading` asks of a sender's file have nothing left to ask here. A cleaned copy
// edited since it was filed is refused, like `dokument::importable` refuses it:
// the master gets what was reviewed or nothing.
//
// A mapped cell that does not convert goes in as its text, never dropped. An
// unmapped column is copied as stored: a number stays a number, text stays text.
// ────────────────────────────────────────────────────────────────

use std::path::PathBuf;

use crate::error::{AppError, AppResult};
use crate::trains::dokument;
use crate::trains::model::{DateOrder, DecimalStyle, Dokument, FieldKind};
use crate::trains::reading::reads_a_date;
use crate::trains::sanitise::{date, number, wagen, Date, Value};
use crate::trains::sheet::grid::{self, Grid, RawCell};

#[derive(Debug, Clone, PartialEq)]
pub enum Out {
    Empty,
    Number(f64),
    Text(String),
}

impl Out {
    pub fn key(&self) -> String {
        match self {
            Out::Empty => String::new(),
            Out::Number(value) => number_text(*value),
            Out::Text(text) => text.trim().to_string(),
        }
    }
}

pub fn number_text(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        value.to_string()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub name: String,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<Out>>,
}

impl Table {
    pub fn column(&self, header: &str) -> Option<usize> {
        self.headers
            .iter()
            .position(|candidate| candidate == header)
    }
}

pub fn load(dokument: &Dokument) -> AppResult<Table> {
    let cleaned = PathBuf::from(&dokument.cleaned);
    if dokument::hash_of(&cleaned)? != dokument.cleaned_hash {
        return Err(AppError::Report(vec![format!(
            "Die bereinigte Datei von „{}“ wurde seit dem Bereinigen verändert und wird nicht übernommen.",
            dokument.name
        )]));
    }
    let source = grid::read(&cleaned, Some(&dokument.sheet))?;
    Ok(table(dokument, &source.grid))
}

fn table(dokument: &Dokument, grid: &Grid) -> Table {
    let layout = &dokument.plan.layout;
    let header_row = layout.header_row.unwrap_or(1);
    let fields: Vec<FieldKind> = (1..=grid.cols)
        .map(|col| {
            dokument
                .plan
                .columns
                .iter()
                .find(|binding| binding.index == col)
                .map_or(FieldKind::Ignorieren, |binding| binding.field)
        })
        .collect();
    let last = layout.last_data_row.unwrap_or(grid.rows).min(grid.rows);
    let rows = (layout.first_data_row..=last)
        .filter(|row| grid.row(*row).iter().any(|cell| !cell.is_empty()))
        .map(|row| {
            (1..=grid.cols)
                .map(|col| {
                    typed(
                        fields[col as usize - 1],
                        grid.cell(col, row),
                        dokument.plan.date1904,
                    )
                })
                .collect()
        })
        .collect();
    Table {
        name: dokument.name.clone(),
        headers: (1..=grid.cols)
            .map(|col| grid.text(col, header_row).trim().to_string())
            .collect(),
        rows,
    }
}

pub fn typed(field: FieldKind, cell: Option<&RawCell>, date1904: bool) -> Out {
    let Some(cell) = cell.filter(|cell| !cell.is_empty()) else {
        return Out::Empty;
    };
    let converted = match field {
        FieldKind::Wagennummer => match wagen::parse(&cell.text).map(|parsed| parsed.value) {
            Ok(Value::Uic(uic)) => uic.as_str().parse::<f64>().ok().map(Out::Number),
            _ => None,
        },
        field if reads_a_date(field) => match cell.number {
            Some(serial) => Some(Out::Number(serial)),
            None => {
                match date::parse_text(&cell.text, DateOrder::DayFirst).map(|parsed| parsed.value) {
                    Ok(Value::Date(value)) => Some(Out::Number(serial(value, date1904))),
                    _ => None,
                }
            }
        },
        FieldKind::Betrag => match cell.number {
            Some(amount) => Some(Out::Number(amount)),
            None => match number::parse_money(&cell.text, DecimalStyle::German)
                .map(|parsed| parsed.value)
            {
                Ok(Value::Money(cents)) => Some(Out::Number(cents as f64 / 100.0)),
                _ => None,
            },
        },
        _ => None,
    };
    converted.unwrap_or_else(|| match cell.number {
        Some(value) => Out::Number(value),
        None => Out::Text(cell.text.clone()),
    })
}

fn serial(value: Date, date1904: bool) -> f64 {
    let days = date::days_from_civil(value.year, value.month, value.day);
    let epoch = if date1904 {
        date::days_from_civil(1904, 1, 1)
    } else {
        date::days_from_civil(1899, 12, 30)
    };
    (days - epoch) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(raw: &str) -> RawCell {
        RawCell {
            text: raw.into(),
            number: None,
            date_format: false,
        }
    }

    fn stored(value: f64) -> RawCell {
        RawCell {
            text: number_text(value),
            number: Some(value),
            date_format: false,
        }
    }

    #[test]
    fn a_wagennummer_goes_in_as_the_number_every_lookup_is_keyed_on() {
        for raw in ["338506591522", "3385 0659 152-2"] {
            assert_eq!(
                typed(FieldKind::Wagennummer, Some(&text(raw)), false),
                Out::Number(338_506_591_522.0)
            );
        }
    }

    #[test]
    fn a_cleaned_date_becomes_the_serial_excel_stores() {
        // 2026-07-20 is serial 46223 — a value the real master holds.
        assert_eq!(
            typed(FieldKind::Datum, Some(&text("20.07.2026")), false),
            Out::Number(46_223.0)
        );
        assert_eq!(
            typed(FieldKind::EingebautAm, Some(&stored(46_223.0)), false),
            Out::Number(46_223.0)
        );
    }

    #[test]
    fn a_cleaned_amount_becomes_a_number() {
        assert_eq!(
            typed(FieldKind::Betrag, Some(&text("1234,56")), false),
            Out::Number(1234.56)
        );
    }

    #[test]
    fn what_does_not_convert_is_kept_as_text_and_unmapped_cells_keep_their_type() {
        assert_eq!(
            typed(FieldKind::Datum, Some(&text("demnächst")), false),
            Out::Text("demnächst".into())
        );
        assert_eq!(
            typed(FieldKind::Ignorieren, Some(&stored(98.0)), false),
            Out::Number(98.0)
        );
        assert_eq!(
            typed(FieldKind::Ignorieren, Some(&text("Neuhof")), false),
            Out::Text("Neuhof".into())
        );
        assert_eq!(
            typed(FieldKind::Ignorieren, Some(&text("  ")), false),
            Out::Empty
        );
    }

    #[test]
    fn a_key_reads_the_same_whether_it_was_stored_as_a_number_or_typed() {
        assert_eq!(Out::Number(338_506_590_011.0).key(), "338506590011");
        assert_eq!(Out::Text(" 24094-26/01 ".into()).key(), "24094-26/01");
    }
}
