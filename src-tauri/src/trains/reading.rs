// ─── why ────────────────────────────────────────────────────────
// How ONE column is read, decided once before any row, and shared by the two
// paths that read a sender's file: `stage` (the preview) and `clean` (the
// cleaned copy). They used to decide separately and drifted — the cleaner
// learned to doubt a saved reading the file contradicts, staging silently let it
// win — so the same file read two ways depending on the button pressed.
//
// The decimal style is consulted only for a Betrag and the date order only for
// a date; any other column carries a certain default, so it can never raise a
// question nobody asked.
//
// A reading has three sources, in this order: what the user CONFIRMED for this
// file, what the TEMPLATE saved, what the column's own evidence says. There is a
// QUESTION when nothing was saved and the evidence is not conclusive, or when
// something was saved and conclusive evidence says otherwise — never
// auto-resolve when another reading is possible. A contradictory date column
// (both orders forced) is its own reason, because it is the column the question
// exists for.
//
// AND THERE IS NO QUESTION UNLESS A CELL ACTUALLY READS DIFFERENTLY under the
// alternative. An undecided column of `12`, `15` or of `31.12.2025` comes out the
// same either way; asking about it is noise that teaches users to click
// questions away. `differs` is therefore the evidence for the question and the
// rows it touches — staging warns on exactly those rows, cleaning shows them as
// the card's examples.
//
// Evidence is taken from cells stored as TEXT. A cell Excel stored as a number
// is read from the number, and its text — a `.` decimal whatever the sender's
// style — says nothing about how the sender writes.
// ────────────────────────────────────────────────────────────────

use std::collections::HashSet;

use super::model::{CardExample, ColumnBinding, DateOrder, DecimalStyle, FieldKind, Reading};
use super::sanitise::column::{self, Inference};
use super::sanitise::{date, format, number, Parse};

pub struct Interpretation {
    pub binding: ColumnBinding,
    pub decimal: Inference<DecimalStyle>,
    pub date_order: Inference<DateOrder>,
    pub dates: bool,
}

pub struct Question {
    pub reading: Reading,
    pub reason: String,
    pub differs: Vec<CardExample>,
    rows: HashSet<u32>,
}

impl Question {
    pub fn touches(&self, row: u32) -> bool {
        self.rows.contains(&row)
    }

    pub fn example(&self, row: u32) -> Option<&CardExample> {
        self.touches(row)
            .then(|| self.differs.iter().find(|example| example.row == row))
            .flatten()
    }
}

#[derive(Default, Clone, Copy)]
pub struct Confirmed {
    pub decimal: Option<DecimalStyle>,
    pub date_order: Option<DateOrder>,
}

pub fn reads_a_date(field: FieldKind) -> bool {
    matches!(
        field,
        FieldKind::Datum | FieldKind::EingebautAm | FieldKind::AusgebautAm
    )
}

pub fn read_column(
    binding: &ColumnBinding,
    texts: &[(u32, &str)],
    hints: &[bool],
    confirmed: Confirmed,
) -> (Interpretation, Option<Question>) {
    let evidence: Vec<&str> = texts.iter().map(|(_, text)| *text).collect();
    let mut decimal = DecimalStyle::German;
    let mut date_order = DateOrder::DayFirst;
    let mut question = None;

    if binding.field == FieldKind::Betrag {
        let (chosen, reason) = settle(
            binding.decimal,
            column::infer_decimal(&evidence),
            confirmed.decimal,
        );
        decimal = chosen;
        let alternative = other_style(chosen);
        question = reason.and_then(|reason| {
            ask(
                texts,
                |raw| number::parse_money(raw, chosen),
                |raw| number::parse_money(raw, alternative),
                Reading::Decimal {
                    chosen,
                    alternative,
                },
                reason.text(
                    "Tausender- und Dezimaltrennzeichen sind in dieser Spalte nicht eindeutig.",
                    style_label(chosen),
                    style_label(alternative),
                ),
            )
        });
    } else if reads_a_date(binding.field) {
        let inferred = column::infer_date_order(&evidence);
        let mixed = inferred.is_none();
        let inferred = inferred.unwrap_or_else(|| Inference::assumed(DateOrder::DayFirst));
        let (chosen, reason) = settle(binding.date_order, inferred, confirmed.date_order);
        date_order = chosen;
        let alternative = other_order(chosen);
        let undecided = if mixed {
            "Die Spalte enthält Daten in beiden Reihenfolgen."
        } else {
            "Ob Tag oder Monat zuerst steht, ist in dieser Spalte nicht eindeutig."
        };
        question = reason.and_then(|reason| {
            ask(
                texts,
                |raw| date::parse_text(raw, chosen),
                |raw| date::parse_text(raw, alternative),
                Reading::DateOrder {
                    chosen,
                    alternative,
                },
                reason.text(undecided, order_label(chosen), order_label(alternative)),
            )
        });
    }

    (
        Interpretation {
            binding: binding.clone(),
            decimal: Inference::certain(decimal),
            date_order: Inference::certain(date_order),
            dates: column::looks_like_dates(hints),
        },
        question,
    )
}

enum Reason {
    Undecided,
    Contradicts,
}

impl Reason {
    fn text(&self, undecided: &str, chosen: &str, alternative: &str) -> String {
        match self {
            Reason::Undecided => {
                format!("{undecided} Gelesen {chosen}, möglich wäre auch {alternative}.")
            }
            Reason::Contradicts => format!(
                "Die Vorlage liest diese Spalte {chosen}, die Datei spricht für {alternative}."
            ),
        }
    }
}

fn settle<T: Copy + PartialEq>(
    saved: Option<T>,
    evidence: Inference<T>,
    confirmed: Option<T>,
) -> (T, Option<Reason>) {
    let reason = match saved {
        None if !evidence.certain => Some(Reason::Undecided),
        Some(saved) if evidence.certain && evidence.value != saved => Some(Reason::Contradicts),
        _ => None,
    };
    (confirmed.or(saved).unwrap_or(evidence.value), reason)
}

fn ask(
    texts: &[(u32, &str)],
    chosen: impl Fn(&str) -> Parse,
    alternative: impl Fn(&str) -> Parse,
    reading: Reading,
    reason: String,
) -> Option<Question> {
    let shown = |parse: Parse| match parse {
        Ok(parsed) => format::value(&parsed.value),
        Err(_) => "nicht lesbar".to_string(),
    };
    let differs: Vec<CardExample> = texts
        .iter()
        .filter_map(|(row, raw)| {
            let as_chosen = shown(chosen(raw));
            let as_alternative = shown(alternative(raw));
            (as_chosen != as_alternative).then(|| CardExample {
                row: *row,
                raw: raw.to_string(),
                chosen: as_chosen,
                alternative: Some(as_alternative),
                message: None,
            })
        })
        .collect();
    if differs.is_empty() {
        return None;
    }
    Some(Question {
        reading,
        reason,
        rows: differs.iter().map(|example| example.row).collect(),
        differs,
    })
}

fn other_style(style: DecimalStyle) -> DecimalStyle {
    match style {
        DecimalStyle::German => DecimalStyle::English,
        DecimalStyle::English => DecimalStyle::German,
    }
}

fn other_order(order: DateOrder) -> DateOrder {
    match order {
        DateOrder::DayFirst => DateOrder::MonthFirst,
        DateOrder::MonthFirst => DateOrder::DayFirst,
    }
}

fn style_label(style: DecimalStyle) -> &'static str {
    match style {
        DecimalStyle::German => "deutsch (1.234,56)",
        DecimalStyle::English => "englisch (1,234.56)",
    }
}

fn order_label(order: DateOrder) -> &'static str {
    match order {
        DateOrder::DayFirst => "Tag zuerst (31.12.)",
        DateOrder::MonthFirst => "Monat zuerst (12/31)",
    }
}
