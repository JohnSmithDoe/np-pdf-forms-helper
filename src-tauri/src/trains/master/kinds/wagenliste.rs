// ─── why ────────────────────────────────────────────────────────
// Any sheet with a Wagen key column: the master's dashboard, and every pasted
// export or hand-kept list that is one row per Wagen. For now such a sheet
// contributes only the fleet — every Wagen it names. Its other columns are the
// later phases' (Wagenmeldung, Frist, Werkstattauftrag …).
//
// The KEY COLUMN is whichever of the portal's spellings the sheet uses, and it
// is not always column A — a running number can stand before it. `KEYS` holds
// the column names seen in the exports (generic report fields, no customer
// names), compared normalised, first match in sheet order wins. A sheet with
// none of them falls back to „Wagennummer“, which then simply finds nothing.
//
// Never written back: the dashboard is formulas over the other sheets plus the
// customer's own notes, and pasting rows into it would replace both.
// ────────────────────────────────────────────────────────────────

use crate::trains::builtin::shaped;
use crate::trains::model::{FieldKind, ImportTemplate};
use crate::trains::recognise::normalise;

pub const UPDATES: bool = false;

const KEYS: [&str; 6] = [
    "Wagennummer",
    "Wagennr.",
    "wagen",
    "wagen_nr",
    "transportmittelnr",
    "Asset",
];

pub fn claims(names: &[String]) -> bool {
    names.iter().any(|name| is_key(name))
}

pub fn template(headers: &[String]) -> ImportTemplate {
    let key = headers
        .iter()
        .find(|header| is_key(&normalise(header)))
        .map_or(KEYS[0], String::as_str);
    shaped(
        super::id("wagenliste"),
        "Master: Wagenliste",
        &[(key, FieldKind::Wagennummer)],
    )
}

fn is_key(name: &str) -> bool {
    KEYS.iter().any(|key| normalise(key) == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owned(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn the_key_column_is_found_wherever_it_stands() {
        let headers = owned(&["", "Wagennummer", "Schild wurde entfernt"]);
        let template = template(&headers);
        assert_eq!(template.plan.columns[0].header, "Wagennummer");

        let headers = owned(&["Asset", "Anbaudatum"]);
        assert_eq!(template_header(&headers), "Asset");
        let headers = owned(&["transportmittelnr", "naechste_revtyp"]);
        assert_eq!(template_header(&headers), "transportmittelnr");
    }

    fn template_header(headers: &[String]) -> String {
        template(headers).plan.columns[0].header.clone()
    }

    // A dashboard column merely STARTING with „Wagen“ is a note, not the key.
    #[test]
    fn a_column_that_only_begins_with_wagen_is_not_a_key() {
        let names: Vec<String> = ["Wagen BEBRA Schadwagengleis", "Wagen ist beladen"]
            .iter()
            .map(|name| normalise(name))
            .collect();
        assert!(!claims(&names));
    }
}
