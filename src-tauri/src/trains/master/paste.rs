// ─── why ────────────────────────────────────────────────────────
// One source table into one master worksheet. The sheet is the customer's paste
// target, and the dashboard reads it POSITIONALLY — `VLOOKUP(A2,Telematik!A:K,
// 10,0)` means "the tenth column", not "Stadt" — so values go into the master's
// columns, found by HEADER, and the master's column order is never touched.
//
// Every master column is one of three things, decided once per sheet:
//   • matched  its header (or the alias the binding names for it) is a source
//              header, and the source owns it. A source column the binding
//              aliases elsewhere, or `ignored`, is never matched BY NAME too:
//              one document column lands in one master column, and „nicht
//              übertragen“ holds even where the sheet has a column of that name
//   • formula  a data row holds a formula — the customer's own derived column
//              (`TODAY()-D`, a lookup into a sibling sheet) — re-emitted per row
//   • hand     neither: something a person keeps in the sheet by hand
//
// A new row gets each formula column as a PLAIN formula copied from the
// column's first formula, shifted by umya's own `set_coordinate`, which moves
// relative references and leaves `$A$1` and whole-column `A:A` alone. Cached
// results stay stale on purpose: the written book carries an older `calcId`, so
// Excel recalculates everything on open (measured, footguns.md).
//
// THE UPDATE IS INCREMENTAL, ALWAYS (decisions.md): rows are matched by KEY, a
// known key has its matched cells overwritten — formula and hand columns of
// that row are never touched — and nothing is ever deleted. An unknown key is
// appended only with `append`; otherwise it is counted and said. A document
// that names one key twice cannot say which of its rows is the update, so the
// sheet is refused rather than letting the last row win silently; a key the
// SHEET holds twice updates its first row, and that is said too.
//
// A key that reads as a Wagennummer — twelve digits once spaces, dashes and
// dots are gone — is compared by its digits: the cleaned copy writes the
// grouped spelling the user chose, the master usually holds a number.
//
// A matched column keeps the master's TYPE. The real exports deliver
// `"180028676"` and `"6715.0"` as text where the pasted sheet holds numbers —
// Excel converted them on paste — and a text key misses a number in every
// VLOOKUP. So where most of a column's existing cells are numbers, a plain
// decimal string goes in as a number. Only in that direction: a number written
// into a text column could be a date serial, and `46223` is no better than it.
//
// The header row is row 1. Every source sheet of the real master has it there,
// and a guessed header row in somebody else's workbook is the wrong place to be
// clever. New rows take each column's style from row 2, so a date column stays
// formatted as dates.
//
// `structure` is the same classification without the write, for the export
// wizard's Abgleich: which master columns have a source (`pairs`, the source
// named), which are kept by hand, and which source columns land nowhere — a
// renamed or dropped column.
// `Outcome.columns` names what a write may have changed (formula columns are
// re-emitted, not changed), so the wizard's cell diff looks only there.
// ────────────────────────────────────────────────────────────────

use std::collections::{BTreeMap, HashMap};

use umya_spreadsheet::{Cell, Style, Worksheet};

use super::source::{Out, Table};
use crate::error::{AppError, AppResult};
use crate::trains::model::{MasterAlias, MasterBinding};
use crate::trains::sheet::grid;

#[derive(Debug, Default, PartialEq)]
pub struct Outcome {
    pub line: String,
    pub notes: Vec<String>,
    pub columns: Vec<u32>,
    pub key: Option<u32>,
}

#[derive(Debug, Default, PartialEq)]
pub struct Structure {
    pub matched: Vec<String>,
    pub pairs: Vec<MasterAlias>,
    pub hand: Vec<String>,
    pub unmatched: Vec<String>,
}

enum Kind {
    Matched { index: usize, numeric: bool },
    Formula(Box<Cell>),
    Hand(String),
}

struct Classified {
    master: grid::Grid,
    columns: BTreeMap<u32, Kind>,
    last_row: u32,
}

struct Layout {
    columns: BTreeMap<u32, Kind>,
    key: Option<(u32, usize)>,
    last_row: u32,
    styles: HashMap<u32, Style>,
}

pub fn write(
    worksheet: &mut Worksheet,
    binding: &MasterBinding,
    table: &Table,
    append: bool,
) -> AppResult<Outcome> {
    let layout = layout(worksheet, binding, table)?;
    let mut outcome = incremental(worksheet, &layout, binding, table, append)?;
    outcome.columns = layout
        .columns
        .iter()
        .filter(|(_, kind)| !matches!(kind, Kind::Formula(_)))
        .map(|(col, _)| *col)
        .collect();
    outcome.key = layout.key.map(|(col, _)| col);
    Ok(outcome)
}

pub fn structure(
    worksheet: &Worksheet,
    binding: &MasterBinding,
    table: &Table,
) -> AppResult<Structure> {
    let classified = classify(worksheet, binding, table)?;
    let mut structure = Structure::default();
    let mut used = Vec::new();
    for (col, kind) in &classified.columns {
        match kind {
            Kind::Matched { index, .. } => {
                used.push(*index);
                let master = classified.master.text(*col, 1).trim().to_string();
                structure.pairs.push(MasterAlias {
                    master: master.clone(),
                    source: table.headers[*index].clone(),
                });
                structure.matched.push(master);
            }
            Kind::Hand(header) => structure.hand.push(header.clone()),
            Kind::Formula(_) => {}
        }
    }
    structure.unmatched = table
        .headers
        .iter()
        .enumerate()
        .filter(|(index, header)| !header.is_empty() && !used.contains(index))
        .map(|(_, header)| header.clone())
        .collect();
    Ok(structure)
}

fn classify(
    worksheet: &Worksheet,
    binding: &MasterBinding,
    table: &Table,
) -> AppResult<Classified> {
    let master = grid::from_worksheet(worksheet)?;

    let mut formulas: BTreeMap<u32, Cell> = BTreeMap::new();
    let mut last_row = master.rows.max(1);
    let mut last_col = master.cols;
    for cell in worksheet.cells() {
        let (col, row) = (cell.coordinate().col_num(), cell.coordinate().row_num());
        if row < 2 || cell.formula().is_empty() {
            continue;
        }
        last_row = last_row.max(row);
        last_col = last_col.max(col);
        let earlier = formulas
            .get(&col)
            .is_some_and(|known| known.coordinate().row_num() < row);
        if !earlier {
            formulas.insert(col, cell.clone());
        }
    }

    let source_of = |header: &str| -> Option<String> {
        if let Some(alias) = binding
            .aliases
            .iter()
            .find(|alias| alias.master.trim() == header)
        {
            return Some(alias.source.trim().to_string());
        }
        let taken = binding
            .aliases
            .iter()
            .any(|alias| alias.source.trim() == header)
            || binding
                .ignored
                .iter()
                .any(|ignored| ignored.trim() == header);
        (!taken).then(|| header.to_string())
    };

    let mut columns = BTreeMap::new();
    for col in 1..=last_col {
        let header = master.text(col, 1).trim().to_string();
        let kind = if let Some(index) = (!header.is_empty())
            .then(|| source_of(&header))
            .flatten()
            .and_then(|source| table.column(&source))
        {
            Kind::Matched {
                index,
                numeric: numeric(&master, col),
            }
        } else if let Some(template) = formulas.remove(&col) {
            Kind::Formula(Box::new(template))
        } else if !header.is_empty() {
            Kind::Hand(header)
        } else {
            continue;
        };
        columns.insert(col, kind);
    }

    Ok(Classified {
        master,
        columns,
        last_row,
    })
}

fn layout(worksheet: &Worksheet, binding: &MasterBinding, table: &Table) -> AppResult<Layout> {
    let Classified {
        master,
        columns,
        last_row,
    } = classify(worksheet, binding, table)?;
    let sheet = &binding.sheet;

    if !columns
        .values()
        .any(|kind| matches!(kind, Kind::Matched { .. }))
    {
        return Err(AppError::Report(vec![format!(
            "„{sheet}“ hat keine Spalte, deren Überschrift in „{}“ vorkommt.",
            table.name
        )]));
    }

    let key = match binding
        .key
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty())
    {
        None => None,
        Some(key) => {
            let col = (1..=master.cols)
                .find(|col| master.text(*col, 1).trim() == key)
                .ok_or_else(|| {
                    AppError::Report(vec![format!(
                        "„{sheet}“ hat keine Spalte „{key}“ für den Schlüssel."
                    )])
                })?;
            match columns.get(&col) {
                Some(Kind::Matched { index, .. }) => Some((col, *index)),
                _ => {
                    return Err(AppError::Report(vec![format!(
                        "Die Schlüsselspalte „{key}“ in „{sheet}“ hat keine Quelle in „{}“.",
                        table.name
                    )]))
                }
            }
        }
    };

    let styles = columns
        .keys()
        .filter_map(|col| {
            worksheet
                .cell((*col, 2u32))
                .map(|cell| (*col, cell.style().clone()))
        })
        .collect();

    Ok(Layout {
        columns,
        key,
        last_row,
        styles,
    })
}

fn incremental(
    worksheet: &mut Worksheet,
    layout: &Layout,
    binding: &MasterBinding,
    table: &Table,
    append: bool,
) -> AppResult<Outcome> {
    let sheet = &binding.sheet;
    let Some((key_col, key_index)) = layout.key else {
        return Err(AppError::Report(vec![format!(
            "„{sheet}“ braucht eine Schlüsselspalte, über die Zeilen zugeordnet werden."
        )]));
    };

    let mut seen: HashMap<String, u32> = HashMap::new();
    for values in &table.rows {
        let key = match_key(&values[key_index].key());
        if !key.is_empty() {
            *seen.entry(key).or_default() += 1;
        }
    }
    let mut twice: Vec<&String> = seen
        .iter()
        .filter(|(_, count)| **count > 1)
        .map(|(key, _)| key)
        .collect();
    if !twice.is_empty() {
        twice.sort();
        return Err(AppError::Report(vec![format!(
            "„{sheet}“: Der Schlüssel „{}“ ist in „{}“ nicht eindeutig — {} Wert(e) kommen mehrfach vor, z. B. „{}“. Bitte eine eindeutige Schlüsselspalte wählen.",
            table.headers[key_index],
            table.name,
            twice.len(),
            twice[0]
        )]));
    }

    let master = grid::from_worksheet(worksheet)?;
    let mut rows: HashMap<String, u32> = HashMap::new();
    let mut doubled = 0_u32;
    for row in 2..=layout.last_row {
        let key = match_key(master.text(key_col, row));
        if key.is_empty() {
            continue;
        }
        match rows.entry(key) {
            std::collections::hash_map::Entry::Occupied(_) => doubled += 1,
            std::collections::hash_map::Entry::Vacant(free) => {
                free.insert(row);
            }
        }
    }

    let (mut appended, mut updated, mut skipped, mut keyless) = (0_u32, 0_u32, 0_u32, 0_u32);
    let mut next = layout.last_row + 1;
    for values in &table.rows {
        let key = match_key(&values[key_index].key());
        if key.is_empty() {
            keyless += 1;
            continue;
        }
        match rows.get(&key) {
            Some(row) => {
                for (col, kind) in &layout.columns {
                    if *col == key_col {
                        continue;
                    }
                    if let Kind::Matched { index, numeric } = kind {
                        put(
                            worksheet,
                            *col,
                            *row,
                            &typed(&values[*index], *numeric),
                            None,
                        );
                    }
                }
                updated += 1;
            }
            None if append => {
                write_new_row(worksheet, layout, next, values);
                rows.insert(key, next);
                next += 1;
                appended += 1;
            }
            None => skipped += 1,
        }
    }
    if appended > 0 {
        stretch_filter(worksheet, next - 1);
    }

    let mut notes = Vec::new();
    if skipped > 0 {
        notes.push(format!(
            "„{sheet}“: {skipped} Zeile(n) mit einem Schlüssel, den das Blatt nicht hat, wurden nicht angehängt."
        ));
    }
    if doubled > 0 {
        notes.push(format!(
            "„{sheet}“: {doubled} Schlüssel stehen im Blatt mehrfach; aktualisiert wurde jeweils die erste Zeile."
        ));
    }
    if keyless > 0 {
        notes.push(format!(
            "„{sheet}“: {keyless} Zeile(n) ohne Schlüssel wurden übersprungen."
        ));
    }
    Ok(Outcome {
        line: format!(
            "„{sheet}“: {updated} Zeile(n) aktualisiert, {appended} angehängt aus „{}“.",
            table.name
        ),
        notes,
        ..Outcome::default()
    })
}

pub fn match_key(text: &str) -> String {
    let text = text.trim();
    let digits: String = text
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-' && *c != '.')
        .collect();
    if digits.len() == 12 && digits.chars().all(|c| c.is_ascii_digit()) {
        digits
    } else {
        text.to_string()
    }
}

fn write_new_row(worksheet: &mut Worksheet, layout: &Layout, row: u32, values: &[Out]) {
    for (col, kind) in &layout.columns {
        match kind {
            Kind::Matched { index, numeric } => put(
                worksheet,
                *col,
                row,
                &typed(&values[*index], *numeric),
                layout.styles.get(col),
            ),
            Kind::Formula(template) => {
                let mut cell = (**template).clone();
                cell.set_coordinate((*col, row));
                let formula = cell.formula().to_string();
                cell.set_formula(formula);
                worksheet.set_cell(cell);
            }
            Kind::Hand(_) => {}
        }
    }
}

fn numeric(master: &grid::Grid, col: u32) -> bool {
    let (mut numbers, mut texts) = (0_u32, 0_u32);
    for cell in master.column(col, 2, master.rows) {
        if cell.number.is_some() {
            numbers += 1;
        } else if !cell.is_empty() {
            texts += 1;
        }
    }
    numbers > texts
}

fn typed(value: &Out, numeric: bool) -> Out {
    match value {
        Out::Text(text) if numeric && plain_decimal(text.trim()) => text
            .trim()
            .parse::<f64>()
            .map_or_else(|_| value.clone(), Out::Number),
        _ => value.clone(),
    }
}

fn plain_decimal(text: &str) -> bool {
    let digits = text.strip_prefix('-').unwrap_or(text);
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, "0"));
    !whole.is_empty()
        && !fraction.is_empty()
        && whole.chars().all(|c| c.is_ascii_digit())
        && fraction.chars().all(|c| c.is_ascii_digit())
}

fn put(worksheet: &mut Worksheet, col: u32, row: u32, value: &Out, style: Option<&Style>) {
    let cell = worksheet.cell_mut((col, row));
    if let Some(style) = style {
        cell.set_style(style.clone());
    }
    match value {
        Out::Empty => cell.set_blank(),
        Out::Number(number) => cell.set_value_number(*number),
        Out::Text(text) => cell.set_value_string(text.as_str()),
    };
}

fn stretch_filter(worksheet: &mut Worksheet, last_row: u32) {
    let Some(filter) = worksheet.auto_filter_mut() else {
        return;
    };
    let range = filter.range().range();
    let Some((start, end)) = range.split_once(':') else {
        return;
    };
    let end_col: String = end
        .chars()
        .take_while(|c| c.is_ascii_alphabetic() || *c == '$')
        .collect();
    if end_col.is_empty() {
        return;
    }
    filter
        .range_mut()
        .set_range(format!("{start}:{end_col}{}", last_row.max(2)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trains::model::MasterAlias;

    fn table(headers: &[&str], rows: &[&[Out]]) -> Table {
        Table {
            name: "quelle.xlsx".into(),
            headers: headers.iter().map(|header| header.to_string()).collect(),
            rows: rows.iter().map(|row| row.to_vec()).collect(),
        }
    }

    fn binding(key: Option<&str>) -> MasterBinding {
        MasterBinding {
            sheet: "Blatt".into(),
            template_id: "t".into(),
            kind: None,
            key: key.map(str::to_string),
            aliases: vec![],
            ignored: Vec::new(),
            auto: false,
        }
    }

    fn n(value: f64) -> Out {
        Out::Number(value)
    }

    fn t(value: &str) -> Out {
        Out::Text(value.into())
    }

    /// Wagen | (formula) | Stadt | Notiz — the shape of the real Telematik sheet:
    /// a formula column without a header, and a column somebody types into.
    fn worksheet() -> Worksheet {
        let mut sheet = Worksheet::default();
        for (col, header) in [(1u32, "Wagen"), (3, "Stadt"), (4, "Notiz")] {
            sheet.cell_mut((col, 1u32)).set_value_string(header);
        }
        for (row, wagen, city, note) in [(2u32, 1.0, "Alt", "prüfen"), (3, 2.0, "Alt", "")] {
            sheet.cell_mut((1u32, row)).set_value_number(wagen);
            sheet
                .cell_mut((2u32, row))
                .set_formula(format!("TODAY()-A{row}+$A$1+COUNT(A:A)"));
            sheet.cell_mut((3u32, row)).set_value_string(city);
            if !note.is_empty() {
                sheet.cell_mut((4u32, row)).set_value_string(note);
            }
        }
        sheet
    }

    fn value(sheet: &Worksheet, col: u32, row: u32) -> String {
        sheet
            .cell((col, row))
            .map(|cell| cell.value().to_string())
            .unwrap_or_default()
    }

    #[test]
    fn known_keys_are_updated_new_ones_appended_and_nothing_deleted() {
        let mut sheet = worksheet();
        let source = table(
            &["Wagen", "Stadt"],
            &[&[n(2.0), t("Neuhof")], &[n(7.0), t("Fulda")]],
        );
        let outcome = write(&mut sheet, &binding(Some("Wagen")), &source, true).unwrap();

        assert_eq!(
            value(&sheet, 3, 2),
            "Alt",
            "wagen 1 is not in the file and stays"
        );
        assert_eq!(
            value(&sheet, 4, 2),
            "prüfen",
            "a hand cell is never touched"
        );
        assert_eq!(
            value(&sheet, 3, 3),
            "Neuhof",
            "wagen 2 was updated in place"
        );
        assert_eq!(
            sheet.cell((2u32, 3u32)).unwrap().formula(),
            "TODAY()-A3+$A$1+COUNT(A:A)",
            "an updated row keeps its formula"
        );
        assert_eq!(sheet.cell((1u32, 4u32)).unwrap().value_number(), Some(7.0));
        // Relative row moved, `$A$1` and the whole column `A:A` did not.
        assert_eq!(
            sheet.cell((2u32, 4u32)).unwrap().formula(),
            "TODAY()-A4+$A$1+COUNT(A:A)"
        );
        assert!(
            outcome
                .line
                .contains("1 Zeile(n) aktualisiert, 1 angehängt"),
            "{}",
            outcome.line
        );
    }

    #[test]
    fn without_append_an_unknown_key_is_counted_not_written() {
        let mut sheet = worksheet();
        let source = table(
            &["Wagen", "Stadt"],
            &[&[n(2.0), t("Neuhof")], &[n(7.0), t("Fulda")]],
        );
        let outcome = write(&mut sheet, &binding(Some("Wagen")), &source, false).unwrap();
        assert_eq!(value(&sheet, 3, 3), "Neuhof");
        assert!(sheet.cell((1u32, 4u32)).is_none());
        assert!(
            outcome.notes[0].contains("1 Zeile(n)"),
            "{:?}",
            outcome.notes
        );
    }

    #[test]
    fn a_wagennummer_key_matches_by_its_digits() {
        let mut sheet = Worksheet::default();
        sheet.cell_mut((1u32, 1u32)).set_value_string("Wagen");
        sheet.cell_mut((2u32, 1u32)).set_value_string("Stadt");
        sheet
            .cell_mut((1u32, 2u32))
            .set_value_number(338_506_591_522.0);
        sheet.cell_mut((2u32, 2u32)).set_value_string("Alt");
        let source = table(&["Wagen", "Stadt"], &[&[t("3385 0659 152-2"), t("Neuhof")]]);
        write(&mut sheet, &binding(Some("Wagen")), &source, false).unwrap();
        assert_eq!(value(&sheet, 2, 2), "Neuhof");
    }

    #[test]
    fn a_key_the_document_names_twice_refuses_the_sheet() {
        let mut sheet = worksheet();
        let source = table(
            &["Wagen", "Stadt"],
            &[&[n(2.0), t("Neuhof")], &[n(2.0), t("Fulda")]],
        );
        let error = write(&mut sheet, &binding(Some("Wagen")), &source, true).unwrap_err();
        assert!(error.into_messages()[0].contains("nicht eindeutig"));
        assert_eq!(value(&sheet, 3, 3), "Alt");
    }

    #[test]
    fn without_a_key_the_sheet_is_refused() {
        let mut sheet = worksheet();
        let source = table(&["Wagen"], &[&[n(2.0)]]);
        let error = write(&mut sheet, &binding(None), &source, true).unwrap_err();
        assert!(error.into_messages()[0].contains("Schlüsselspalte"));
    }

    #[test]
    fn an_alias_maps_a_renamed_master_column_onto_its_source() {
        let mut sheet = worksheet();
        let mut bound = binding(Some("Wagen"));
        bound.aliases.push(MasterAlias {
            master: "Stadt".into(),
            source: "ort".into(),
        });
        let source = table(&["Wagen", "ort"], &[&[n(2.0), t("Neuhof")]]);
        write(&mut sheet, &bound, &source, false).unwrap();
        assert_eq!(value(&sheet, 3, 3), "Neuhof");
    }

    #[test]
    fn a_sheet_sharing_no_header_with_the_source_is_refused_untouched() {
        let mut sheet = worksheet();
        let source = table(&["Etwas anderes"], &[&[t("x")]]);
        let error = write(&mut sheet, &binding(None), &source, true).unwrap_err();
        assert!(error.into_messages()[0].contains("keine Spalte"));
        assert_eq!(value(&sheet, 3, 2), "Alt");
    }

    #[test]
    fn the_structure_names_matched_hand_kept_and_unplaced_columns() {
        let sheet = worksheet();
        let source = table(&["Wagen", "Ort", "Extra"], &[&[n(2.0), t("x"), t("y")]]);
        let structure = structure(&sheet, &binding(None), &source).unwrap();
        assert_eq!(structure.matched, ["Wagen"]);
        // The formula column has no header and is neither.
        assert_eq!(structure.hand, ["Stadt", "Notiz"]);
        assert_eq!(structure.unmatched, ["Ort", "Extra"]);
    }

    // A pair says which document column feeds each master column, aliased or
    // matched by name.
    #[test]
    fn the_structure_names_each_pair_with_its_source() {
        let sheet = worksheet();
        let mut bound = binding(None);
        bound.aliases.push(MasterAlias {
            master: "Stadt".into(),
            source: "ort".into(),
        });
        let source = table(&["Wagen", "ort"], &[&[n(2.0), t("x")]]);
        let structure = structure(&sheet, &bound, &source).unwrap();
        assert_eq!(
            structure.pairs,
            [
                MasterAlias {
                    master: "Wagen".into(),
                    source: "Wagen".into(),
                },
                MasterAlias {
                    master: "Stadt".into(),
                    source: "ort".into(),
                },
            ]
        );
    }

    // A source column paired elsewhere lands there ONLY: the master column of
    // its own name is left to whoever keeps it by hand.
    #[test]
    fn a_source_paired_elsewhere_is_not_also_matched_by_name() {
        let mut sheet = worksheet();
        let mut bound = binding(Some("Wagen"));
        bound.aliases.push(MasterAlias {
            master: "Notiz".into(),
            source: "Stadt".into(),
        });
        let source = table(&["Wagen", "Stadt"], &[&[n(2.0), t("Neuhof")]]);
        assert_eq!(structure(&sheet, &bound, &source).unwrap().hand, ["Stadt"]);
        write(&mut sheet, &bound, &source, false).unwrap();
        assert_eq!(value(&sheet, 4, 3), "Neuhof");
        assert_eq!(value(&sheet, 3, 3), "Alt");
    }

    // „nicht übertragen“ holds even where the sheet has a column of that name.
    #[test]
    fn an_ignored_source_is_not_written_into_its_namesake() {
        let mut sheet = worksheet();
        let mut bound = binding(Some("Wagen"));
        bound.ignored.push("Stadt".into());
        let source = table(&["Wagen", "Stadt"], &[&[n(2.0), t("Neuhof")]]);
        assert_eq!(
            structure(&sheet, &bound, &source).unwrap().unmatched,
            ["Stadt"]
        );
        write(&mut sheet, &bound, &source, false).unwrap();
        assert_eq!(value(&sheet, 3, 3), "Alt");
    }

    #[test]
    fn the_autofilter_grows_with_the_rows() {
        let mut sheet = worksheet();
        sheet.set_auto_filter("A1:D3");
        let source = table(&["Wagen"], &[&[n(1.0)], &[n(2.0)], &[n(3.0)], &[n(4.0)]]);
        write(&mut sheet, &binding(Some("Wagen")), &source, true).unwrap();
        assert_eq!(sheet.auto_filter().unwrap().range().range(), "A1:D5");
    }

    #[test]
    fn a_numeric_master_column_turns_plain_decimal_text_into_numbers_and_nothing_else() {
        let mut sheet = worksheet();
        // Wagen is numeric in the master, Stadt is text: an export delivering
        // both as text must not leave a text key the dashboard cannot find.
        let source = table(
            &["Wagen", "Stadt"],
            &[
                &[t("180028676"), t("12")],
                &[t("6715.0"), t("1.234")],
                &[t("46′559"), t("x")],
            ],
        );
        write(&mut sheet, &binding(Some("Wagen")), &source, true).unwrap();
        assert_eq!(
            sheet.cell((1u32, 4u32)).unwrap().value_number(),
            Some(180_028_676.0)
        );
        assert_eq!(
            sheet.cell((1u32, 5u32)).unwrap().value_number(),
            Some(6715.0)
        );
        assert_eq!(
            sheet.cell((1u32, 6u32)).unwrap().data_type(),
            "s",
            "not a plain decimal"
        );
        assert_eq!(
            sheet.cell((3u32, 4u32)).unwrap().data_type(),
            "s",
            "a text column stays text"
        );
    }
}
