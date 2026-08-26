// ─── why ────────────────────────────────────────────────────────
// `mappedName` doubles as the cell address, in the format `$Tabelle1.A1`. The
// field dialog builds it and this module is the only thing that parses it back,
// which is why the format is written down in exactly one place.
// ────────────────────────────────────────────────────────────────

use crate::error::{AppError, AppResult};

// The leading marker is dropped and the FIRST '.' separates sheet from cell, so
// a sheet name holding a '.' cannot round-trip.
//
// The cell half is VALIDATED here rather than handed on. `umya`'s
// `From<&str> for CellCoordinates` ends in `col.unwrap(), row.unwrap()` over a
// regex whose column and row halves are BOTH optional, so an address like
// `$Tabelle1.` arrives as "" and panics — which loses the German dialog
// entirely, because a panic is not an `AppError`.
pub fn split(mapped_name: &str) -> AppResult<(&str, &str)> {
    let Some((sheet, cell)) = mapped_name.get(1..).and_then(|rest| rest.split_once('.')) else {
        return Err(invalid(mapped_name));
    };
    if sheet.is_empty() || !is_cell_reference(cell) {
        return Err(invalid(mapped_name));
    }
    Ok((sheet, cell))
}

/// Column letters followed by a row number, both required. `umya` accepts
/// either half alone and then unwraps the other.
fn is_cell_reference(cell: &str) -> bool {
    let letters = cell
        .chars()
        .take_while(char::is_ascii_alphabetic)
        .collect::<String>();
    // Letters are ASCII, so their count is also their byte length.
    let digits = &cell[letters.len()..];
    !letters.is_empty() && letters.len() <= 3 && digits.parse::<u32>().is_ok_and(|row| row >= 1)
}

fn invalid(mapped_name: &str) -> AppError {
    AppError::Report(vec![
        format!("Die Zell-Adresse „{mapped_name}“ ist ungültig. Erwartet wird $Tabelle1.A1."),
        "Bitte entferne das Feld und lege es neu an.".into(),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_the_documented_format() {
        assert_eq!(split("$Tabelle1.A1").unwrap(), ("Tabelle1", "A1"));
        assert_eq!(split("$Sheet.ZZ9999").unwrap(), ("Sheet", "ZZ9999"));
    }

    // A sheet name holding a '.' cannot round-trip, and the failure is loud:
    // the tail is parsed as the cell half and rejected there.
    #[test]
    fn a_dotted_sheet_name_is_an_error() {
        assert!(split("$Ta.belle.A1").is_err());
    }

    #[test]
    fn rejects_a_missing_separator() {
        assert!(split("$Tabelle1A1").is_err());
        assert!(split("").is_err());
    }

    // The leading marker is DROPPED, not checked — so a missing '$' silently
    // eats the first character of the sheet name rather than failing. Only the
    // field dialog writes these, and it always writes the marker.
    #[test]
    fn the_marker_is_dropped_unchecked() {
        assert_eq!(split("Tabelle1.A1").unwrap(), ("abelle1", "A1"));
    }

    #[test]
    fn rejects_an_empty_sheet_name() {
        assert!(split("$.A1").is_err());
    }

    // The reason the cell half is validated here at all: `umya` unwraps a
    // half-parsed address and panics, and a panic is not an `AppError`.
    #[test]
    fn rejects_a_half_written_cell() {
        assert!(split("$Tabelle1.").is_err());
        assert!(split("$Tabelle1.A").is_err());
        assert!(split("$Tabelle1.1").is_err());
    }

    #[test]
    fn rejects_a_zero_row_and_an_over_long_column() {
        assert!(split("$Tabelle1.A0").is_err());
        assert!(split("$Tabelle1.ABCD1").is_err());
        assert!(split("$Tabelle1.ABC1").is_ok());
    }

    #[test]
    fn rejects_trailing_junk_after_the_row() {
        assert!(split("$Tabelle1.A1B").is_err());
        assert!(split("$Tabelle1.A 1").is_err());
    }

    // `get(1..)` would panic on a byte index inside a character, and the marker
    // is whatever the field dialog wrote — so the multi-byte case is reachable.
    #[test]
    fn a_multi_byte_marker_is_an_error_not_a_panic() {
        assert!(split("§Tabelle1.A1").is_err());
        assert!(split("ä").is_err());
    }

    #[test]
    fn the_error_names_the_address_and_the_expected_format() {
        let messages = split("$kaputt").unwrap_err().into_messages();
        assert_eq!(messages.len(), 2);
        assert!(messages[0].contains("$kaputt"), "{}", messages[0]);
        assert!(messages[0].contains("$Tabelle1.A1"), "{}", messages[0]);
    }
}
