// ─── why ────────────────────────────────────────────────────────
// Which saved template a file IS, decided from its header row. This replaced an
// exact hash of the sheet name plus every header in order, and the hash was
// wrong for the files that actually arrive: a portal export grows a column next
// month and is still the same export.
//
// A file matches a template when every header the template MAPS is present.
// Extra columns never count against it, and the template's ignored headers are
// not asked for — a sender dropping a column nobody imports has not changed
// anything this program reads.
//
// SEVERAL MATCHES ARE A QUESTION, never a pick. A template that maps only the
// Wagennummer matches nearly every file; deciding by "most columns" would be a
// guess presented as recognition, and a wrong template is a wrong import. So
// `apply` applies only a SINGLE match, and the scan lists the rest for the user.
//
// `rebind` exists because a binding's `index` is positional. Cloning a template
// plan onto a file with one extra column in front would read every column one to
// the left. Bindings are therefore carried over BY HEADER onto the layout
// detected in the file at hand, and the detected reader and layout win: the
// template's header row was the header row of a different file.
//
// Headers are compared normalised — case, spacing and punctuation are how a
// sender's export wobbles between months, and none of it changes what a column
// is.
// ────────────────────────────────────────────────────────────────

use super::model::{ColumnBinding, FieldKind, ImportPlan, ImportTemplate};
use super::sheet::grid::Grid;
use super::sheet::layout::Candidate;
use super::sheet::readers;
use crate::error::{AppError, AppResult};

pub fn detected(grid: &Grid) -> AppResult<(ImportPlan, Vec<Candidate>)> {
    let candidates = readers::detect(grid);
    let chosen = readers::choose(&candidates)
        .or_else(|| candidates.first())
        .ok_or_else(|| {
            AppError::Report(vec![
                "In dieser Arbeitsmappe wurde keine Tabelle gefunden.".into(),
                "Bitte Kopfzeile und erste Datenzeile von Hand festlegen.".into(),
            ])
        })?;
    let layout = chosen.reader.apply(grid, chosen.hint)?;
    let plan = ImportPlan {
        reader: chosen.reader,
        layout: chosen.hint,
        columns: layout
            .columns
            .iter()
            .map(|slot| ColumnBinding {
                header: slot.header.clone(),
                index: slot.index,
                field: FieldKind::Ignorieren,
                decimal: None,
                date_order: None,
            })
            .collect(),
        template_id: None,
        date1904: false,
    };
    Ok((plan, candidates))
}

pub fn normalise(text: &str) -> String {
    super::sanitise::text::normalise(text)
        .to_lowercase()
        .chars()
        .filter(|character| character.is_alphanumeric() || *character == ' ')
        .collect()
}

fn matches(template: &ImportTemplate, columns: &[ColumnBinding]) -> bool {
    let present: Vec<String> = columns
        .iter()
        .map(|column| normalise(&column.header))
        .collect();
    let mut mapped = template
        .plan
        .columns
        .iter()
        .filter(|binding| binding.field != FieldKind::Ignorieren)
        .peekable();
    mapped.peek().is_some() && mapped.all(|binding| present.contains(&normalise(&binding.header)))
}

pub fn matching<'a>(
    templates: &'a [ImportTemplate],
    columns: &[ColumnBinding],
) -> Vec<&'a ImportTemplate> {
    templates
        .iter()
        .filter(|template| matches(template, columns))
        .collect()
}

pub fn rebind(template: &ImportTemplate, detected: &ImportPlan) -> ImportPlan {
    let mut taken = vec![false; template.plan.columns.len()];
    let columns = detected
        .columns
        .iter()
        .map(|column| {
            let header = normalise(&column.header);
            let found = template
                .plan
                .columns
                .iter()
                .enumerate()
                .find(|(position, binding)| {
                    !taken[*position]
                        && binding.field != FieldKind::Ignorieren
                        && normalise(&binding.header) == header
                });
            match found {
                Some((position, binding)) => {
                    taken[position] = true;
                    ColumnBinding {
                        header: column.header.clone(),
                        index: column.index,
                        field: binding.field,
                        decimal: binding.decimal,
                        date_order: binding.date_order,
                    }
                }
                None => column.clone(),
            }
        })
        .collect();

    ImportPlan {
        reader: detected.reader,
        layout: detected.layout,
        columns,
        template_id: Some(template.id.clone()),
        date1904: detected.date1904,
    }
}

pub fn apply(templates: &[ImportTemplate], detected: &ImportPlan) -> Option<ImportPlan> {
    match matching(templates, &detected.columns).as_slice() {
        [only] => Some(rebind(only, detected)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trains::sheet::layout::LayoutHint;
    use crate::trains::sheet::readers::ReaderKind;

    fn bindings(columns: &[(&str, FieldKind)]) -> Vec<ColumnBinding> {
        columns
            .iter()
            .enumerate()
            .map(|(index, (header, field))| ColumnBinding {
                header: (*header).into(),
                index: index as u32 + 1,
                field: *field,
                decimal: None,
                date_order: None,
            })
            .collect()
    }

    fn plan(columns: &[(&str, FieldKind)]) -> ImportPlan {
        ImportPlan {
            reader: ReaderKind::HeaderRow,
            layout: LayoutHint {
                header_row: Some(1),
                first_data_row: 2,
                last_data_row: None,
            },
            columns: bindings(columns),
            template_id: None,
            date1904: false,
        }
    }

    fn detected(headers: &[&str]) -> ImportPlan {
        let columns: Vec<(&str, FieldKind)> = headers
            .iter()
            .map(|header| (*header, FieldKind::Ignorieren))
            .collect();
        plan(&columns)
    }

    fn template(id: &str, columns: &[(&str, FieldKind)]) -> ImportTemplate {
        ImportTemplate {
            id: id.into(),
            name: id.into(),
            plan: plan(columns),
            partner_id: None,
            origin: None,
            builtin: false,
            created_at: String::new(),
        }
    }

    fn feed() -> ImportTemplate {
        template(
            "feed",
            &[
                ("Wagen", FieldKind::Wagennummer),
                ("Bestellnr", FieldKind::Ignorieren),
                ("Ausgang", FieldKind::Datum),
            ],
        )
    }

    /// Next month's portal export has one column more and is the same export.
    #[test]
    fn an_extra_column_does_not_change_what_a_file_is() {
        let file = detected(&["Wagen", "Neu", "Ausgang"]);
        assert!(matches(&feed(), &file.columns));
    }

    #[test]
    fn a_missing_mapped_header_is_not_a_match() {
        let file = detected(&["Wagen", "Bestellnr"]);
        assert!(!matches(&feed(), &file.columns));
    }

    /// Nobody imports the order number, so a file without it reads the same.
    #[test]
    fn a_missing_ignored_header_still_matches() {
        let file = detected(&["Wagen", "Ausgang"]);
        assert!(matches(&feed(), &file.columns));
    }

    #[test]
    fn header_wobble_does_not_change_the_match() {
        let file = detected(&[" WAGEN ", "ausgang:"]);
        assert!(matches(&feed(), &file.columns));
    }

    /// A template mapping nothing would match every file there is.
    #[test]
    fn a_template_that_maps_nothing_matches_nothing() {
        let empty = template("leer", &[("Wagen", FieldKind::Ignorieren)]);
        assert!(!matches(&empty, &detected(&["Wagen"]).columns));
    }

    /// The bug the positional index would cause: a column inserted in front
    /// must not shift every binding one to the left.
    #[test]
    fn rebinding_follows_the_header_when_columns_move() {
        let file = detected(&["Neu", "Ausgang", "Wagen"]);
        let rebound = rebind(&feed(), &file);
        let field_at = |header: &str| {
            rebound
                .columns
                .iter()
                .find(|column| column.header == header)
                .map(|column| (column.index, column.field))
        };
        assert_eq!(field_at("Wagen"), Some((3, FieldKind::Wagennummer)));
        assert_eq!(field_at("Ausgang"), Some((2, FieldKind::Datum)));
        assert_eq!(field_at("Neu"), Some((1, FieldKind::Ignorieren)));
        assert_eq!(rebound.template_id.as_deref(), Some("feed"));
    }

    /// A confirmed reading is the point of saving it — it has to survive.
    #[test]
    fn rebinding_keeps_a_saved_reading() {
        let mut saved = feed();
        saved.plan.columns[2].date_order = Some(crate::trains::model::DateOrder::MonthFirst);
        let rebound = rebind(&saved, &detected(&["Ausgang", "Wagen"]));
        assert_eq!(
            rebound.columns[0].date_order,
            Some(crate::trains::model::DateOrder::MonthFirst)
        );
    }

    /// The file's own header row wins over the one the template was saved from.
    #[test]
    fn rebinding_keeps_the_detected_layout() {
        let mut file = detected(&["Wagen", "Ausgang"]);
        file.layout.header_row = Some(4);
        file.layout.first_data_row = 5;
        assert_eq!(rebind(&feed(), &file).layout, file.layout);
    }

    #[test]
    fn two_matching_templates_are_a_question_not_a_pick() {
        let narrow = template("schmal", &[("Wagen", FieldKind::Wagennummer)]);
        let templates = vec![feed(), narrow];
        let file = detected(&["Wagen", "Ausgang"]);
        assert_eq!(matching(&templates, &file.columns).len(), 2);
        assert_eq!(apply(&templates, &file), None);
    }

    #[test]
    fn a_single_match_is_applied() {
        let templates = vec![feed()];
        let applied = apply(&templates, &detected(&["Wagen", "Ausgang"])).unwrap();
        assert_eq!(applied.template_id.as_deref(), Some("feed"));
    }
}
