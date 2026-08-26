// ─── why ────────────────────────────────────────────────────────
// UIC wagen numbers: twelve digits, of which the last is a check digit.
//
// The canonical form is the BARE TWELVE DIGITS and nothing else is ever stored
// or matched on. `31 80 4740 123-4`, `31804740123-4` and `318047401234` are one
// wagen, and one representation is what makes that true without a normalising
// step at every comparison. The grouped form people read is derived on the way
// out, in `format.rs`.
//
// A WRONG CHECK DIGIT IS A WARNING, NEVER A REJECTION. Fleets carry typo'd
// numbers in circulation, on paper and in the customer's own ERP; a tool that
// refuses them is a tool that gets worked around. The real typo detector is a
// bad check digit AND no matching known wagen — `resolve/` combines the two,
// because neither on its own is evidence.
//
// The check digit is UIC leaflet 913, a Luhn mod-10 over the first eleven
// digits: numbering from the left, positions 1, 3, 5, 7, 9 and 11 are doubled
// and the rest taken as they are; a product over nine is replaced by its digit
// sum; the check digit is whatever takes the total to the next multiple of ten.
// Because eleven is odd, "double the odd positions from the left" and the
// leaflet's "alternate 2,1 from the right beginning with 2" are the same
// pattern — but ONLY for exactly eleven digits, which is why the length is
// established before the sum is ever computed.
//
// Eleven digits get the check digit computed and a warning; twelve get theirs
// verified. Any other count is an error. Separators are noise and are dropped,
// but anything that is NOT a separator means the cell is not a wagen number in
// need of tidying — it is something else, and stripping it down to whatever
// digits it happens to contain would invent a wagen.
// ────────────────────────────────────────────────────────────────

use super::text;
use super::{Parse, Parsed, Value};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Uic(String);

impl Uic {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub fn check_digit(digits: &[u8]) -> u8 {
    debug_assert_eq!(digits.len(), 11, "the check digit covers eleven digits");
    let total: u32 = digits
        .iter()
        .enumerate()
        .map(|(index, digit)| {
            let product = u32::from(*digit) * if index % 2 == 0 { 2 } else { 1 };
            if product > 9 {
                product - 9
            } else {
                product
            }
        })
        .sum();
    ((10 - total % 10) % 10) as u8
}

pub fn parse(raw: &str) -> Parse {
    let trimmed = text::normalise(raw);
    if trimmed.is_empty() {
        return Ok(Parsed::plain(Value::Empty));
    }

    let digits: Vec<u8> = trimmed
        .chars()
        .filter(char::is_ascii_digit)
        .map(|character| character as u8 - b'0')
        .collect();

    let noise = trimmed.chars().any(|c| {
        !c.is_ascii_digit() && !matches!(c, '-' | '–' | '.' | '/' | '_') && !text::is_space(c)
    });

    match digits.len() {
        11 if !noise => {
            let computed = check_digit(&digits);
            let uic = canonical(&digits, computed);
            Ok(Parsed::warned(
                Value::Uic(Uic(uic.clone())),
                format!(
                    "Prüfziffer fehlte und wurde ergänzt: {}.",
                    super::format::uic_display(&uic)
                ),
            ))
        }
        12 if !noise => {
            let computed = check_digit(&digits[..11]);
            let found = digits[11];
            let uic = canonical(&digits[..11], found);
            let value = Value::Uic(Uic(uic));
            Ok(if computed == found {
                Parsed::plain(value)
            } else {
                Parsed::warned(
                    value,
                    format!(
                        "Prüfziffer stimmt nicht (erwartet {computed}, gefunden {found}). Die Zeile wird trotzdem übernommen."
                    ),
                )
            })
        }
        found => Err(format!(
            "„{trimmed}“ ist keine gültige Wagennummer (12 Ziffern erwartet, {found} gefunden)."
        )),
    }
}

fn canonical(first_eleven: &[u8], check: u8) -> String {
    let mut out = String::with_capacity(12);
    for digit in first_eleven {
        out.push((b'0' + digit) as char);
    }
    out.push((b'0' + check) as char);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digits(text: &str) -> Vec<u8> {
        text.chars()
            .filter(char::is_ascii_digit)
            .map(|c| c as u8 - b'0')
            .collect()
    }

    fn uic(raw: &str) -> String {
        match parse(raw).expect("parses").value {
            Value::Uic(uic) => uic.as_str().to_string(),
            other => panic!("expected a uic, got {other:?}"),
        }
    }

    /// The worked example out of the leaflet: 21 81 2471 217 checks to 3.
    #[test]
    fn reproduces_the_published_example() {
        assert_eq!(check_digit(&digits("21812471217")), 3);
    }

    /// A total that is already a multiple of ten checks to 0, not to 10.
    #[test]
    fn a_round_total_checks_to_zero() {
        // 00 00 0000 000 sums to 0.
        assert_eq!(check_digit(&digits("00000000000")), 0);
    }

    /// Every separator style the same wagen gets written in.
    #[test]
    fn one_wagen_has_exactly_one_stored_form() {
        assert_eq!(uic("21 81 2471 217-3"), "218124712173");
        assert_eq!(uic("21812471217-3"), "218124712173");
        assert_eq!(uic("218124712173"), "218124712173");
        assert_eq!(uic("21.81.2471.217/3"), "218124712173");
    }

    #[test]
    fn a_correct_check_digit_passes_without_comment() {
        assert!(parse("21 81 2471 217-3").unwrap().warning.is_none());
    }

    /// The number in the original request. The algorithm says -3, so -4 is
    /// wrong — and it is still accepted.
    #[test]
    fn a_wrong_check_digit_is_accepted_with_a_warning() {
        assert_eq!(check_digit(&digits("31804740123")), 3);
        let parsed = parse("31 80 4740 123-4").unwrap();
        assert_eq!(parsed.value, Value::Uic(Uic("318047401234".into())));
        let warning = parsed.warning.expect("warned");
        assert!(warning.contains("erwartet 3"), "{warning}");
        assert!(warning.contains("gefunden 4"), "{warning}");
    }

    #[test]
    fn eleven_digits_get_the_check_digit_computed_and_flagged() {
        let parsed = parse("21 81 2471 217").unwrap();
        assert_eq!(parsed.value, Value::Uic(Uic("218124712173".into())));
        assert!(parsed.warning.expect("warned").contains("ergänzt"));
    }

    #[test]
    fn the_wrong_length_is_refused_and_the_count_named() {
        let error = parse("3180474").unwrap_err();
        assert!(error.contains("7 gefunden"), "{error}");
    }

    /// A cell holding a sentence is not a wagen number with stray punctuation.
    #[test]
    fn text_around_the_digits_is_refused_rather_than_stripped() {
        assert!(parse("Wagen Nr 218124712173 (Halter X)").is_err());
    }

    #[test]
    fn a_blank_cell_is_empty_rather_than_an_error() {
        assert_eq!(parse("  ").unwrap().value, Value::Empty);
    }
}
