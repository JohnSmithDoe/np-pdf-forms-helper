// ─── why ────────────────────────────────────────────────────────
// Text to a number, given a decimal style the CALLER decided. This module never
// guesses: `column.rs` looks at the whole column and hands the answer down,
// because a per-cell guess flips independently per row and silently mixes two
// readings inside one column.
//
// The style is an argument rather than a lookup for the same reason nothing here
// reads a locale — the question "is `1.234` one thousand or one point two" is
// answered by the file, not by the machine.
//
// A cell Excel stored as a NUMBER never reaches this module: `value_number()`
// already answered, and re-parsing its stringification is how a German column
// gets read as English. See `docs/state.md`.
//
// What the walk accepts, and why each one is in the list:
//   • a lone `-` or `–` is how a spreadsheet writes "nothing here", so it is
//     `Empty` rather than a minus sign with no digits
//   • `(1.234,00)` is accounting's negative and `1234-` is some exports'; both
//     are stripped before the digit walk so it only ever sees digits
//   • grouping marks carry no value — including every space character and the
//     Swiss apostrophe in all three spellings: `'`, the typographic `’`, and the
//     PRIME `′` a real fleet export used for every km reading
//   • a SECOND decimal separator means this is not a number in this style, and
//     saying so beats inventing one of the two possible readings
//
// There is deliberately NO plain-number parser. Every number this app reads is
// an amount, and `parse_money` rounds to cents exactly once so no `f64` reaches
// the store or the workbook. A second entry point returning a float would be the
// one somebody reaches for by accident. Its magnitude bound is not decoration: `as i64` on an
// out-of-range float saturates SILENTLY, so a nonsense cell would otherwise land
// in the store as a real amount.
// ────────────────────────────────────────────────────────────────

use super::text;
use super::{Parse, Parsed, Value};
use crate::trains::model::DecimalStyle;

const CURRENCY: [&str; 5] = ["€", "EUR", "CHF", "$", "USD"];
const APOSTROPHES: [char; 3] = ['\'', '\u{2019}', '\u{2032}'];

impl DecimalStyle {
    fn decimal(self) -> char {
        match self {
            DecimalStyle::German => ',',
            DecimalStyle::English => '.',
        }
    }

    fn grouping(self) -> char {
        match self {
            DecimalStyle::German => '.',
            DecimalStyle::English => ',',
        }
    }
}

pub fn parse_money(raw: &str, style: DecimalStyle) -> Parse {
    let Some(number) = to_f64(raw, style)? else {
        return Ok(Parsed::plain(Value::Empty));
    };
    let cents = (number * 100.0).round();
    if !cents.is_finite() || cents.abs() > i64::MAX as f64 {
        return Err(format!(
            "Der Betrag „{}“ ist zu groß.",
            text::normalise(raw)
        ));
    }
    Ok(Parsed::plain(Value::Money(cents as i64)))
}

fn to_f64(raw: &str, style: DecimalStyle) -> Result<Option<f64>, String> {
    let trimmed = text::normalise(raw);
    if trimmed.is_empty() || trimmed == "-" || trimmed == "–" {
        return Ok(None);
    }

    let (body, negative) = strip_sign(&strip_currency(&trimmed));
    if body.is_empty() {
        return Err(invalid(&trimmed));
    }

    let mut digits = String::with_capacity(body.len());
    let mut seen_decimal = false;
    for character in body.chars() {
        if character.is_ascii_digit() {
            digits.push(character);
        } else if character == style.decimal() {
            if seen_decimal {
                return Err(invalid(&trimmed));
            }
            seen_decimal = true;
            digits.push('.');
        } else if character == style.grouping()
            || text::is_space(character)
            || APOSTROPHES.contains(&character)
        {
            continue;
        } else {
            return Err(invalid(&trimmed));
        }
    }

    if digits.is_empty() || digits == "." {
        return Err(invalid(&trimmed));
    }

    let parsed: f64 = digits.parse().map_err(|_| invalid(&trimmed))?;
    Ok(Some(if negative { -parsed } else { parsed }))
}

fn strip_sign(body: &str) -> (String, bool) {
    let body = body.trim();
    if let Some(inner) = body.strip_prefix('(').and_then(|b| b.strip_suffix(')')) {
        return (inner.trim().to_string(), true);
    }
    if let Some(rest) = body.strip_prefix('-').or_else(|| body.strip_prefix('−')) {
        return (rest.trim().to_string(), true);
    }
    if let Some(rest) = body.strip_suffix('-').or_else(|| body.strip_suffix('−')) {
        return (rest.trim().to_string(), true);
    }
    if let Some(rest) = body.strip_prefix('+') {
        return (rest.trim().to_string(), false);
    }
    (body.to_string(), false)
}

fn strip_currency(body: &str) -> String {
    let mut out = body.trim().to_string();
    for _ in 0..2 {
        for mark in CURRENCY {
            if let Some(rest) = out.strip_prefix(mark) {
                out = rest.trim().to_string();
            }
            if let Some(rest) = out.strip_suffix(mark) {
                out = rest.trim().to_string();
            }
        }
    }
    out
}

fn invalid(raw: &str) -> String {
    format!("„{raw}“ ist keine Zahl.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use DecimalStyle::{English, German};

    fn number(raw: &str, style: DecimalStyle) -> f64 {
        cents(raw, style) as f64 / 100.0
    }

    fn cents(raw: &str, style: DecimalStyle) -> i64 {
        match parse_money(raw, style).expect("parses").value {
            Value::Money(value) => value,
            other => panic!("expected money, got {other:?}"),
        }
    }

    #[test]
    fn reads_the_two_house_styles() {
        assert_eq!(number("1.234,56", German), 1234.56);
        assert_eq!(number("1,234.56", English), 1234.56);
    }

    /// The whole reason the style is an argument: one string, two readings a
    /// factor of a thousand apart, and only the column knows which.
    #[test]
    fn the_same_text_reads_differently_under_each_style() {
        assert_eq!(cents("1.234", German), 123_400);
        assert_eq!(cents("1.234", English), 123);
    }

    #[test]
    fn accepts_every_space_character_as_grouping() {
        assert_eq!(number("1 234,56", German), 1234.56);
        assert_eq!(number("1\u{00a0}234,56", German), 1234.56);
        assert_eq!(number("1\u{202f}234,56", German), 1234.56);
        assert_eq!(number("1'234.56", English), 1234.56);
    }

    // `′` is U+2032 PRIME, not an apostrophe at all — but it is what a real
    // export wrote 283 times, and under either style it can only be grouping.
    #[test]
    fn accepts_every_apostrophe_spelling_as_grouping() {
        assert_eq!(number("67\u{2032}543", German), 67_543.0);
        assert_eq!(number("67\u{2032}543", English), 67_543.0);
        assert_eq!(number("1\u{2019}234.56", English), 1234.56);
        assert_eq!(number("1\u{2032}234\u{2032}567,5", German), 1_234_567.5);
    }

    #[test]
    fn strips_currency_on_either_side() {
        assert_eq!(number("€ 1.234,56", German), 1234.56);
        assert_eq!(number("1.234,56 EUR", German), 1234.56);
        assert_eq!(number("CHF 1'234.56", English), 1234.56);
    }

    #[test]
    fn reads_the_three_ways_a_negative_is_written() {
        assert_eq!(number("-1.234,56", German), -1234.56);
        assert_eq!(number("1.234,56-", German), -1234.56);
        assert_eq!(number("(1.234,56)", German), -1234.56);
    }

    #[test]
    fn a_blank_and_a_lone_dash_are_empty_rather_than_zero() {
        assert_eq!(parse_money("", German).unwrap().value, Value::Empty);
        assert_eq!(parse_money("  ", German).unwrap().value, Value::Empty);
        assert_eq!(parse_money("-", German).unwrap().value, Value::Empty);
    }

    #[test]
    fn refuses_what_is_not_a_number_and_names_it() {
        let error = parse_money("keine Angabe", German).unwrap_err();
        assert!(error.contains("keine Angabe"), "{error}");
    }

    /// Two decimal separators are not a number in either style — better to say
    /// so than to pick one of the two possible readings.
    #[test]
    fn refuses_a_second_decimal_separator() {
        assert!(parse_money("1.2.3", English).is_err());
        assert!(parse_money("1,2,3", German).is_err());
    }

    /// Under English `,` is grouping, so this is 1234 and not 12.34. It is the
    /// mirror of the ambiguity test above and the reason the column decides.
    #[test]
    fn a_separator_in_the_other_role_is_read_as_grouping() {
        assert_eq!(number("12,34", English), 1234.0);
    }

    #[test]
    fn money_is_cents_and_rounds_once() {
        assert_eq!(cents("1.234,56", German), 123_456);
        assert_eq!(cents("0,1", German), 10);
        assert_eq!(cents("-19,99", German), -1999);
        // The case that makes f64 money visible in a workbook.
        assert_eq!(cents("1234,565", German), 123_457);
    }

    #[test]
    fn an_absurd_amount_is_refused_rather_than_saturated() {
        assert!(parse_money("999999999999999999999", German).is_err());
    }
}
