// ─── why ────────────────────────────────────────────────────────
// Civil dates: a year, a month and a day. No time, no zone, no clock.
//
// That is not minimalism, it is the requirement. A maintenance record happened
// on the 31st; attaching a timezone to it means that in half of Europe it
// happened on the 30th, and the conversion that does it is exactly the kind of
// helpfulness a date library invites.
//
// `days_from_civil` / `civil_from_days` are Howard Hinnant's, whose only
// subtlety is shifting the year to start in March so the leap day lands at the
// END of the cycle and needs no special case.
//
// Excel's serials carry a bug that is now a format: 1900 is treated as a leap
// year, so serial 60 is a 29 February that never existed and everything after it
// is one day further along than the arithmetic says. Hence two epochs rather
// than one epoch and a fudge — 1899-12-30 at or above serial 61, 1899-12-31 at
// or below 59, and 60 itself an error. The Macintosh system starts at 1904-01-01
// and never had the bug, so it is one clean addition. A time of day is dropped
// rather than rounded: this model has no time.
//
// `PLAUSIBLE_SERIALS` is 1990-01-01 to 2100-01-01, for the one case where a
// number sits in a column with no other evidence that it is a date at all — a
// bare `45000` is more likely a cost than a date.
//
// Reading a WRITTEN date, in order:
//   • a four-digit first component is ISO, which is unambiguous by construction
//   • `.` is German and always day-first — no locale writes `12.31.2025` — so
//     the caller's `order` is consulted only for `/` and `-`
//   • a two-digit year maps 00–79 to 20xx and 80–99 to 19xx, and ALWAYS carries
//     a warning: the file did not say, and a wrong century is invisible
//
// `finish` always takes `(year, month, day)`; both callers reorder on the way
// in, so nothing there has to infer which number is which. `to_iso` is the wire
// and storage form because it sorts lexicographically, which is a real property
// of the format and the reason the wire carries a string and not three numbers.
// ────────────────────────────────────────────────────────────────

use super::text;
use super::{Parse, Parsed, Value};
use crate::trains::model::DateOrder;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

pub const PLAUSIBLE_SERIALS: std::ops::RangeInclusive<i64> = 32874..=73050;

impl Date {
    pub fn new(year: i32, month: u8, day: u8) -> Option<Self> {
        if !(1..=12).contains(&month) || day < 1 || day > days_in_month(year, month) {
            return None;
        }
        Some(Self { year, month, day })
    }

    pub fn to_iso(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

pub fn is_leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

pub fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(year) => 29,
        2 => 28,
        _ => 0,
    }
}

pub fn days_from_civil(year: i32, month: u8, day: u8) -> i64 {
    let year = year as i64 - if month <= 2 { 1 } else { 0 };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400; // [0, 399]
    let month = month as i64;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day as i64 - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

pub fn civil_from_days(days: i64) -> Date {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097; // [0, 146096]
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153; // [0, 11], March = 0
    let day = (day_of_year - (153 * month_prime + 2) / 5 + 1) as u8;
    let month = (month_prime + if month_prime < 10 { 3 } else { -9 }) as u8;
    Date {
        year: (year + i64::from(month <= 2)) as i32,
        month,
        day,
    }
}

pub fn from_serial(serial: f64, date_1904: bool) -> Result<Date, String> {
    if !serial.is_finite() {
        return Err("Der Datumswert ist keine Zahl.".into());
    }
    let whole = serial.floor() as i64;
    if date_1904 {
        if whole < 0 {
            return Err(format!("Der Datumswert {whole} ist kein Datum."));
        }
        return Ok(civil_from_days(days_from_civil(1904, 1, 1) + whole));
    }
    match whole {
        60 => Err(
            "Der Datumswert 60 entspricht dem 29.02.1900, den es nicht gibt. Bitte die Zelle prüfen."
                .into(),
        ),
        0 => Err("Der Datumswert 0 ist kein Datum.".into()),
        serial if serial < 0 => Err(format!("Der Datumswert {serial} ist kein Datum.")),
        serial if serial >= 61 => Ok(civil_from_days(days_from_civil(1899, 12, 30) + serial)),
        serial => Ok(civil_from_days(days_from_civil(1899, 12, 31) + serial)),
    }
}

const SEPARATORS: [char; 3] = ['.', '-', '/'];

pub fn parse_text(raw: &str, order: DateOrder) -> Parse {
    let trimmed = text::normalise(raw);
    if trimmed.is_empty() {
        return Ok(Parsed::plain(Value::Empty));
    }

    let Some((first, second, third, separator)) = split(&trimmed) else {
        return Err(invalid(&trimmed));
    };

    if first.len == 4 {
        return finish(first.value, second.value, third.value, &trimmed, None);
    }

    let day_first = separator == '.' || order == DateOrder::DayFirst;
    let (day, month) = if day_first {
        (first, second)
    } else {
        (second, first)
    };

    let (year, warning) = match third.len {
        4 => (third.value, None),
        1 | 2 => {
            let year = if third.value <= 79 {
                2000 + third.value
            } else {
                1900 + third.value
            };
            (
                year,
                Some(format!(
                    "Zweistellige Jahreszahl „{}“ als {year} gelesen.",
                    third.raw
                )),
            )
        }
        _ => return Err(invalid(&trimmed)),
    };

    finish(year, month.value, day.value, &trimmed, warning)
}

fn finish(year: i32, month: i32, day: i32, raw: &str, warning: Option<String>) -> Parse {
    let (Ok(month), Ok(day)) = (u8::try_from(month), u8::try_from(day)) else {
        return Err(invalid(raw));
    };
    let Some(date) = Date::new(year, month, day) else {
        return Err(format!("„{raw}“ ist kein gültiges Datum."));
    };
    Ok(match warning {
        Some(line) => Parsed::warned(Value::Date(date), line),
        None => Parsed::plain(Value::Date(date)),
    })
}

struct Part {
    value: i32,
    len: usize,
    raw: String,
}

fn split(raw: &str) -> Option<(Part, Part, Part, char)> {
    let separator = SEPARATORS.into_iter().find(|s| raw.contains(*s))?;
    let mut parts = raw.split(separator);
    let first = part(parts.next()?)?;
    let second = part(parts.next()?)?;
    let third = part(parts.next()?)?;
    if parts.next().is_some() {
        return None;
    }
    Some((first, second, third, separator))
}

fn part(raw: &str) -> Option<Part> {
    let raw = raw.trim();
    if raw.is_empty() || !raw.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(Part {
        value: raw.parse().ok()?,
        len: raw.len(),
        raw: raw.to_string(),
    })
}

fn invalid(raw: &str) -> String {
    format!("„{raw}“ ist kein Datum. Erwartet wird z. B. 31.12.2025.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use DateOrder::{DayFirst, MonthFirst};

    fn date(raw: &str, order: DateOrder) -> Date {
        match parse_text(raw, order).expect("parses").value {
            Value::Date(date) => date,
            other => panic!("expected a date, got {other:?}"),
        }
    }

    #[test]
    fn civil_round_trips_across_leap_boundaries() {
        for (year, month, day) in [
            (1970, 1, 1),
            (1899, 12, 30),
            (1900, 3, 1),
            (2000, 2, 29),
            (2023, 3, 15),
            (2100, 3, 1),
        ] {
            let days = days_from_civil(year, month, day);
            assert_eq!(
                civil_from_days(days),
                Date { year, month, day },
                "{year}-{month}-{day}"
            );
        }
    }

    #[test]
    fn the_epoch_is_day_zero() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
    }

    #[test]
    fn leap_years_follow_the_gregorian_rule() {
        assert!(is_leap(2000));
        assert!(!is_leap(1900));
        assert!(is_leap(2024));
        assert!(!is_leap(2023));
    }

    /// The value the Phase 0 probe wrote into a real workbook.
    #[test]
    fn serial_45000_is_the_15th_of_march_2023() {
        assert_eq!(
            from_serial(45000.0, false).unwrap(),
            Date::new(2023, 3, 15).unwrap()
        );
    }

    /// Serial 61 is 1900-03-01 and serial 59 is 1900-02-28 — the two sides of
    /// the phantom leap day, and the reason for two epochs.
    #[test]
    fn the_1900_leap_bug_is_handled_on_both_sides() {
        assert_eq!(
            from_serial(59.0, false).unwrap(),
            Date::new(1900, 2, 28).unwrap()
        );
        assert_eq!(
            from_serial(61.0, false).unwrap(),
            Date::new(1900, 3, 1).unwrap()
        );
        assert!(from_serial(60.0, false).is_err());
    }

    #[test]
    fn a_time_of_day_is_dropped_rather_than_rounded() {
        assert_eq!(
            from_serial(45000.99, false).unwrap(),
            Date::new(2023, 3, 15).unwrap()
        );
    }

    #[test]
    fn reads_the_german_and_iso_forms() {
        assert_eq!(
            date("31.12.2025", DayFirst),
            Date::new(2025, 12, 31).unwrap()
        );
        assert_eq!(
            date("2025-12-31", DayFirst),
            Date::new(2025, 12, 31).unwrap()
        );
    }

    /// The ambiguity the column has to settle.
    #[test]
    fn slashes_follow_the_order_the_column_decided() {
        assert_eq!(date("03/04/2025", DayFirst), Date::new(2025, 4, 3).unwrap());
        assert_eq!(
            date("03/04/2025", MonthFirst),
            Date::new(2025, 3, 4).unwrap()
        );
    }

    /// A dotted date is German whatever the column inferred — no locale writes
    /// `12.31.2025`.
    #[test]
    fn dots_are_day_first_regardless_of_the_order() {
        assert_eq!(
            date("31.12.2025", MonthFirst),
            Date::new(2025, 12, 31).unwrap()
        );
    }

    #[test]
    fn a_two_digit_year_is_read_and_flagged() {
        let parsed = parse_text("31.12.25", DayFirst).unwrap();
        assert_eq!(parsed.value, Value::Date(Date::new(2025, 12, 31).unwrap()));
        assert!(parsed.warning.is_some());

        let parsed = parse_text("31.12.99", DayFirst).unwrap();
        assert_eq!(parsed.value, Value::Date(Date::new(1999, 12, 31).unwrap()));
    }

    #[test]
    fn refuses_a_day_that_does_not_exist() {
        assert!(parse_text("30.02.2025", DayFirst).is_err());
        assert!(parse_text("29.02.2023", DayFirst).is_err());
        assert!(parse_text("29.02.2024", DayFirst).is_ok());
    }

    #[test]
    fn refuses_what_is_not_a_date_and_names_the_expected_form() {
        let error = parse_text("nächste Woche", DayFirst).unwrap_err();
        assert!(error.contains("31.12.2025"), "{error}");
        assert!(parse_text("31.12", DayFirst).is_err());
        assert!(parse_text("1.2.3.4", DayFirst).is_err());
    }

    #[test]
    fn a_blank_cell_is_empty_rather_than_an_error() {
        assert_eq!(parse_text("  ", DayFirst).unwrap().value, Value::Empty);
    }

    #[test]
    fn iso_renders_sortably() {
        assert_eq!(Date::new(2025, 1, 5).unwrap().to_iso(), "2025-01-05");
        let mut dates = [
            Date::new(2025, 12, 31).unwrap(),
            Date::new(2025, 1, 5).unwrap(),
        ];
        dates.sort();
        assert_eq!(dates[0].to_iso(), "2025-01-05");
    }
}
