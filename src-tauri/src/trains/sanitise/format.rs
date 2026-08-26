// ─── why ────────────────────────────────────────────────────────
// The inverse of the parsers, and a sibling of them rather than a helper buried
// in the exporter — because the two halves have to agree and the way to keep
// them agreeing is to test them against each other.
//
// It renders German, and that is the only spelling it renders: the ERP is
// German, the operator is German, and the parsers accept German. A round trip
// through here and back must land on the same value, which is exactly what the
// tests at the bottom assert.
//
// This is also what the PREVIEW uses. The Phase 0 probe found that umya's
// `formatted_value()` does NOT apply a cell's number format — a date serial
// comes back as `45000` — so "what the user sees in Excel" has to be rendered
// here rather than read off the file. One renderer, used by both sides.
//
// `iso_date` is the same rendering for a date already stored as `2025-12-31` —
// the stores keep ISO, so the exporters would otherwise each carry their own
// split-and-reorder, which is what they did.
//
// `money` is always two decimals, always a comma, and NEVER a grouping mark: a
// grouping mark is for reading and this output is for re-importing. `uic` is the
// grouping people read a wagen number in, derived every time and never stored;
// `uic_display` takes bare digits for the messages that have to name a number
// before a `Uic` exists, and hands back anything that is not twelve digits
// untouched rather than slicing it — it runs inside error paths, where panicking
// is the worst available outcome. `Value::Empty` renders as nothing rather than
// as a word, because a blank cell should look blank.
// ────────────────────────────────────────────────────────────────

use super::{Date, Uic, Value};

pub fn date(value: Date) -> String {
    format!("{:02}.{:02}.{:04}", value.day, value.month, value.year)
}

pub fn iso_date(iso: &str) -> String {
    match iso.split('-').collect::<Vec<&str>>().as_slice() {
        [year, month, day] => format!("{day}.{month}.{year}"),
        _ => iso.to_string(),
    }
}

pub fn money(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let absolute = cents.unsigned_abs();
    format!("{sign}{},{:02}", absolute / 100, absolute % 100)
}

pub fn uic(value: &Uic) -> String {
    uic_display(value.as_str())
}

pub fn uic_display(digits: &str) -> String {
    if digits.len() != 12 || !digits.chars().all(|c| c.is_ascii_digit()) {
        return digits.to_string();
    }
    format!(
        "{} {} {} {}-{}",
        &digits[0..2],
        &digits[2..4],
        &digits[4..8],
        &digits[8..11],
        &digits[11..12]
    )
}

pub fn value(input: &Value) -> String {
    match input {
        Value::Empty => String::new(),
        Value::Text(text) => text.clone(),
        Value::Money(cents) => money(*cents),
        Value::Date(date_value) => date(*date_value),
        Value::Uic(uic_value) => uic(uic_value),
    }
}

#[cfg(test)]
mod tests {
    use super::super::{date as date_parser, number as number_parser, wagen};
    use super::*;
    use crate::trains::model::{DateOrder, DecimalStyle};

    #[test]
    fn renders_the_german_forms() {
        assert_eq!(date(Date::new(2025, 1, 5).unwrap()), "05.01.2025");
        assert_eq!(money(123_456), "1234,56");
        assert_eq!(money(-1999), "-19,99");
        assert_eq!(money(5), "0,05");
        assert_eq!(money(0), "0,00");
    }

    /// The stored form is ISO and the rendered form is German. Anything that is
    /// not three dash-separated parts comes back untouched, for the same reason
    /// `uic_display` does not slice a short number.
    #[test]
    fn a_stored_iso_date_renders_german() {
        assert_eq!(iso_date("2025-12-31"), "31.12.2025");
        assert_eq!(iso_date("2025-01-05"), "05.01.2025");
        assert_eq!(iso_date("kaputt"), "kaputt");
        assert_eq!(iso_date(""), "");
    }

    #[test]
    fn a_wagen_number_reads_in_its_grouping() {
        assert_eq!(uic_display("318047401234"), "31 80 4740 123-4");
    }

    /// Anything that is not twelve digits comes back untouched rather than
    /// sliced — this runs inside error messages, where panicking is the worst
    /// available outcome.
    #[test]
    fn a_short_number_is_not_sliced() {
        assert_eq!(uic_display("3180"), "3180");
        assert_eq!(uic_display(""), "");
    }

    /// The property that matters: what this module writes, the parsers read
    /// back to the same value.
    #[test]
    fn dates_round_trip_through_the_parser() {
        for (year, month, day) in [(2025, 12, 31), (2000, 2, 29), (1999, 1, 1)] {
            let original = Date::new(year, month, day).unwrap();
            let parsed =
                date_parser::parse_text(&date(original), DateOrder::DayFirst).expect("re-reads");
            assert_eq!(parsed.value, Value::Date(original));
            assert!(parsed.warning.is_none(), "a full year needs no warning");
        }
    }

    #[test]
    fn money_round_trips_through_the_parser() {
        for cents in [0_i64, 5, -1999, 123_456, -100_000_000] {
            let parsed =
                number_parser::parse_money(&money(cents), DecimalStyle::German).expect("re-reads");
            assert_eq!(parsed.value, Value::Money(cents), "{cents}");
        }
    }

    #[test]
    fn wagen_numbers_round_trip_through_the_parser() {
        let parsed = wagen::parse(&uic_display("218124712173")).expect("re-reads");
        assert_eq!(value(&parsed.value), "21 81 2471 217-3");
    }
}
