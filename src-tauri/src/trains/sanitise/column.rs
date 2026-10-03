// ─── why ────────────────────────────────────────────────────────
// `1.234` is one thousand two hundred and thirty-four in German and one point
// two three four in English, and NOTHING IN THE CELL DECIDES WHICH. A per-cell
// heuristic is a coin flip dressed as a rule, and worse, it flips independently
// per row — so one column silently mixes both readings and a total comes out
// wrong on some rows and right on others. Plausible, mixed, and invisible.
//
// A column has evidence a cell does not, because the sender was internally
// consistent even where they agreed with nobody else. So the question is asked
// once for the whole column, the first conclusive cell settles it, and where
// nothing is conclusive the answer is a DEFAULT THAT SAYS SO — which is what
// lets the preview offer one control that re-reads the entire column.
//
// `infer_decimal` walks the column and takes the first cell that settles it:
//   1. both `.` and `,` present — the LAST of the two is the decimal separator
//   2. one separator whose trailing group is not exactly 3 digits — it is the
//      decimal one, since a thousands group is always three
//   3. the same separator twice — it can only be grouping
// Anything left is `d{1,3}[.,]d{3}` in every cell, which is genuinely
// undecidable, and gets the default with `certain: false`. A separator with
// nothing numeric after it decides nothing: that is a broken value, and judging
// it belongs to the parser.
//
// `infer_date_order` reads the same way — a component over 12 can only be a day.
// Dotted dates are excluded from the evidence because `31.12.2025` is German
// whatever the sender does with slashes, so it says nothing about the case in
// question. Year-first dates (`2025-12-31`) are excluded for the same reason: a
// leading year is not a day over twelve, and counting it as one settled every
// column holding a single ISO date as "day first" without a word. A column that
// forces BOTH readings contradicts itself, and `None`
// says so rather than resolving it: that is a fact the user needs.
//
// The German default is not a preference, it is the house language of every
// sender this was written for, and it applies only where the file said nothing.
// ────────────────────────────────────────────────────────────────

use super::text;
use crate::trains::model::{DateOrder, DecimalStyle};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Inference<T> {
    pub value: T,
    pub certain: bool,
}

impl<T> Inference<T> {
    pub fn certain(value: T) -> Self {
        Self {
            value,
            certain: true,
        }
    }

    pub fn assumed(value: T) -> Self {
        Self {
            value,
            certain: false,
        }
    }
}

const DEFAULT_DECIMAL: DecimalStyle = DecimalStyle::German;
const DEFAULT_DATE_ORDER: DateOrder = DateOrder::DayFirst;

pub fn infer_decimal(values: &[&str]) -> Inference<DecimalStyle> {
    for raw in values {
        let value = text::normalise(raw);
        if value.is_empty() {
            continue;
        }
        let dots = value.matches('.').count();
        let commas = value.matches(',').count();

        if dots > 0 && commas > 0 {
            let last_dot = value.rfind('.').unwrap_or(0);
            let last_comma = value.rfind(',').unwrap_or(0);
            return Inference::certain(if last_comma > last_dot {
                DecimalStyle::German
            } else {
                DecimalStyle::English
            });
        }

        if dots > 1 {
            return Inference::certain(DecimalStyle::German);
        }
        if commas > 1 {
            return Inference::certain(DecimalStyle::English);
        }

        if let Some(style) = by_group_length(&value, '.', DecimalStyle::English) {
            return Inference::certain(style);
        }
        if let Some(style) = by_group_length(&value, ',', DecimalStyle::German) {
            return Inference::certain(style);
        }
    }
    Inference::assumed(DEFAULT_DECIMAL)
}

fn by_group_length(
    value: &str,
    separator: char,
    decimal_style: DecimalStyle,
) -> Option<DecimalStyle> {
    let (_, tail) = value.split_once(separator)?;
    let group: String = tail.chars().take_while(char::is_ascii_digit).collect();
    if group.is_empty() || !tail.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    (group.len() != 3).then_some(decimal_style)
}

pub fn infer_date_order(values: &[&str]) -> Option<Inference<DateOrder>> {
    let mut day_first = false;
    let mut month_first = false;

    for raw in values {
        let value = text::normalise(raw);
        let Some((first, second)) = leading_pair(&value) else {
            continue;
        };
        if first > 12 {
            day_first = true;
        }
        if second > 12 {
            month_first = true;
        }
    }

    match (day_first, month_first) {
        (true, true) => None,
        (true, false) => Some(Inference::certain(DateOrder::DayFirst)),
        (false, true) => Some(Inference::certain(DateOrder::MonthFirst)),
        (false, false) => Some(Inference::assumed(DEFAULT_DATE_ORDER)),
    }
}

fn leading_pair(value: &str) -> Option<(u32, u32)> {
    let separator = ['/', '-'].into_iter().find(|s| value.contains(*s))?;
    let mut parts = value.split(separator);
    let leading = parts.next()?.trim();
    if leading.len() > 2 {
        return None;
    }
    let first: u32 = leading.parse().ok()?;
    let second: u32 = parts.next()?.trim().parse().ok()?;
    parts.next()?;
    Some((first, second))
}

pub fn looks_like_dates(hints: &[bool]) -> bool {
    hints.iter().any(|hint| *hint)
}

#[cfg(test)]
mod tests {
    use super::*;
    use DecimalStyle::{English, German};

    fn decimal(values: &[&str]) -> Inference<DecimalStyle> {
        infer_decimal(values)
    }

    #[test]
    fn both_separators_in_one_cell_settle_it() {
        assert_eq!(decimal(&["1.234,56"]), Inference::certain(German));
        assert_eq!(decimal(&["1,234.56"]), Inference::certain(English));
    }

    #[test]
    fn a_trailing_group_that_is_not_three_digits_settles_it() {
        assert_eq!(decimal(&["1.23"]), Inference::certain(English));
        assert_eq!(decimal(&["12,3456"]), Inference::certain(German));
    }

    #[test]
    fn a_repeated_separator_is_grouping() {
        assert_eq!(decimal(&["1.234.567"]), Inference::certain(German));
        assert_eq!(decimal(&["1,234,567"]), Inference::certain(English));
    }

    /// The undecidable shape, and the whole reason `certain` exists.
    #[test]
    fn three_digit_groups_alone_are_not_decidable() {
        let inferred = decimal(&["1.234", "5.678", "9.012"]);
        assert_eq!(inferred.value, DEFAULT_DECIMAL);
        assert!(!inferred.certain);
    }

    /// One conclusive cell rescues a column full of ambiguous ones — which is
    /// exactly what a per-cell rule cannot do.
    #[test]
    fn one_conclusive_cell_decides_the_whole_column() {
        let inferred = decimal(&["1.234", "5.678", "9.012,50"]);
        assert_eq!(inferred, Inference::certain(German));

        let inferred = decimal(&["1.234", "5.678", "9,012.50"]);
        assert_eq!(inferred, Inference::certain(English));
    }

    #[test]
    fn blank_and_non_numeric_cells_are_skipped_not_counted() {
        assert_eq!(decimal(&["", "  ", "1.23"]), Inference::certain(English));
    }

    #[test]
    fn a_column_with_no_separators_at_all_falls_back() {
        assert!(!decimal(&["1234", "5678"]).certain);
    }

    #[test]
    fn a_day_over_twelve_settles_the_date_order() {
        assert_eq!(
            infer_date_order(&["03/04/2025", "31/12/2025"]),
            Some(Inference::certain(DateOrder::DayFirst))
        );
        assert_eq!(
            infer_date_order(&["03/04/2025", "12/31/2025"]),
            Some(Inference::certain(DateOrder::MonthFirst))
        );
    }

    /// A column that forces both readings is broken, and saying so beats
    /// picking one.
    #[test]
    fn a_column_that_contradicts_itself_answers_none() {
        assert_eq!(infer_date_order(&["31/12/2025", "12/31/2025"]), None);
    }

    #[test]
    fn an_undecidable_date_column_falls_back_and_says_so() {
        let inferred = infer_date_order(&["03/04/2025", "05/06/2025"]).expect("no conflict");
        assert_eq!(inferred.value, DEFAULT_DATE_ORDER);
        assert!(!inferred.certain);
    }

    /// Dotted dates carry no information about how this sender writes slashes.
    #[test]
    fn dotted_dates_are_not_evidence_about_order() {
        let inferred = infer_date_order(&["31.12.2025"]).expect("no conflict");
        assert!(!inferred.certain);
    }

    /// 2025 is a year, not a day over twelve, so it settles nothing.
    #[test]
    fn year_first_dates_are_not_evidence_about_order() {
        let inferred = infer_date_order(&["03/04/2025", "2025-05-06"]).expect("no conflict");
        assert!(!inferred.certain);
    }

    /// The inconsistency the Phase 0 probe found in a real file.
    #[test]
    fn one_date_formatted_cell_marks_the_column() {
        assert!(looks_like_dates(&[true, false, false]));
        assert!(!looks_like_dates(&[false, false]));
    }
}
