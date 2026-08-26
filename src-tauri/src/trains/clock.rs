// ─── why ────────────────────────────────────────────────────────
// The one place trains reads a clock, so that `sanitise` can promise it never
// does. A parser that consults the time parses the same file differently on two
// days, which is exactly the class of bug the rest of that module is arranged to
// avoid.
//
// UTC and a civil date, through the same `civil_from_days` everything else uses.
// No local time: `imported_at` is a stamp on a record, and converting it to a
// zone would make it read as the day before for half of Europe.
// ────────────────────────────────────────────────────────────────

use super::sanitise::date::{civil_from_days, Date};

const SECONDS_PER_DAY: u64 = 86_400;

pub fn today() -> Date {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);
    civil_from_days((seconds / SECONDS_PER_DAY) as i64)
}

pub fn today_iso() -> String {
    today().to_iso()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn today_is_a_plausible_civil_date() {
        let date = today();
        assert!(date.year >= 2026, "{date:?}");
        assert!((1..=12).contains(&date.month));
        assert!((1..=31).contains(&date.day));
    }

    #[test]
    fn the_stamp_is_iso_so_it_sorts() {
        let stamp = today_iso();
        assert_eq!(stamp.len(), 10);
        assert_eq!(stamp.matches('-').count(), 2);
    }
}
