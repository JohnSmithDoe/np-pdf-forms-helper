// ─── why ────────────────────────────────────────────────────────
// A stable signature for "a file shaped like this one", so the second monthly
// list from a werkstatt finds its saved mapping instead of asking again. That is
// the payoff the whole import is arranged around: file one costs a mapping, file
// two costs nothing.
//
// It is taken from the COLUMN HEADERS and the sheet name, normalised the way a
// partner name is — case, spacing and punctuation are how a sender's export
// wobbles between months, and none of it changes what the file is. Column ORDER
// is kept, because a sender who reorders their export has genuinely changed it.
//
// This is NOT a security boundary and NOT unique. Two werkstaetten using the same
// off-the-shelf template collide, and that is accepted: a template is named,
// visible and editable, and a wrong one shows up in the preview before anything
// is written. Do not harden it into a hash nobody can debug.
//
// It takes the SHEET NAME AND THE COLUMNS, not a `StagedImport`. The question has
// to be asked before anything is staged — `commands::apply_template` runs while
// the plan is still being built — and the wider signature made the caller
// fabricate an empty `StagedImport` to ask it.
// ────────────────────────────────────────────────────────────────

use super::hash;
use super::model::ColumnBinding;

pub fn of(sheet: &str, columns: &[ColumnBinding]) -> String {
    let mut parts: Vec<String> = vec![normalise(sheet)];
    parts.extend(columns.iter().map(|column| normalise(&column.header)));
    hash::join(&parts.iter().map(String::as_str).collect::<Vec<_>>())
}

fn normalise(text: &str) -> String {
    super::sanitise::text::normalise(text)
        .to_lowercase()
        .chars()
        .filter(|character| character.is_alphanumeric() || *character == ' ')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trains::model::FieldKind;

    fn columns(headers: &[&str]) -> Vec<ColumnBinding> {
        headers
            .iter()
            .enumerate()
            .map(|(index, header)| ColumnBinding {
                header: (*header).into(),
                index: index as u32 + 1,
                field: FieldKind::Ignorieren,
                decimal: None,
                date_order: None,
            })
            .collect()
    }

    /// The same export next month, with the wobble a real one has.
    #[test]
    fn casing_spacing_and_punctuation_do_not_change_the_signature() {
        assert_eq!(
            of("Tabelle1", &columns(&["Wagennummer", "Datum", "Kosten"])),
            of("tabelle1", &columns(&[" WAGENNUMMER ", "Datum:", "Kosten"]))
        );
    }

    #[test]
    fn a_different_set_of_columns_is_a_different_file() {
        assert_ne!(
            of("Tabelle1", &columns(&["Wagennummer", "Datum"])),
            of("Tabelle1", &columns(&["Wagennummer", "Datum", "Kosten"]))
        );
    }

    /// A sender who reorders their export has genuinely changed it.
    #[test]
    fn reordering_the_columns_is_a_different_file() {
        assert_ne!(
            of("Tabelle1", &columns(&["Wagennummer", "Datum"])),
            of("Tabelle1", &columns(&["Datum", "Wagennummer"]))
        );
    }

    #[test]
    fn a_different_sheet_name_is_a_different_file() {
        assert_ne!(
            of("Januar", &columns(&["Wagennummer"])),
            of("Februar", &columns(&["Wagennummer"]))
        );
    }
}
