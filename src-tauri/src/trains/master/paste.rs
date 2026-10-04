// ─── why ────────────────────────────────────────────────────────
// One source table into one master worksheet. The sheet is the customer's paste
// target, and the dashboard reads it POSITIONALLY — `VLOOKUP(A2,Telematik!A:K,
// 10,0)` means "the tenth column", not "Stadt" — so values go into the master's
// columns, found by HEADER, and the master's column order is never touched.
//
// Every master column is one of three things, decided once per sheet:
//   • matched  its header (or the alias the binding names for it) is a source
//              header, and the source owns it
//   • formula  a data row holds a formula — the customer's own derived column
//              (`TODAY()-D`, a lookup into a sibling sheet) — re-emitted per row
//   • hand     neither: something a person keeps in the sheet by hand
//
// A formula column is rewritten as PLAIN per-row formulas, every row of it.
// Excel stores a filled-down column as one shared formula anchored in its first
// cell; clearing a snapshot's stale rows can remove that anchor while children
// still point at it. The row is shifted by umya's own `set_coordinate`, which
// moves relative references and leaves `$A$1` and whole-column `A:A` alone.
// Cached results stay stale on purpose: the written book carries an older
// `calcId`, so Excel recalculates everything on open (measured, footguns.md).
//
// The two modes are the two shapes a sender's file comes in (decisions.md):
//   • snapshot  the file IS the current state. Rows 2… are cleared and written
//               fresh; a hand column's cells travel with their row's KEY if the
//               binding names one, otherwise they are gone and the report says so
//   • feed      the file is the latest slice of a history. Upsert by key: a known
//               key updates the matched cells, a new one is appended, nothing is
//               ever deleted
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
// wizard's Abgleich: which master columns have a source, which are kept by
// hand, and which source columns land nowhere — a renamed or dropped column.
// `Outcome.columns` names what a write may have changed (formula columns are
// re-emitted, not changed), so the wizard's cell diff looks only there.
// ────────────────────────────────────────────────────────────────

use std::collections::{BTreeMap, HashMap};

use umya_spreadsheet::{Cell, Style, Worksheet};

use super::source::{Out, Table};
use crate::error::{AppError, AppResult};
use crate::trains::model::{MasterBinding, MasterMode};
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
    last_col: u32,
}

struct Layout {
    columns: BTreeMap<u32, Kind>,
    key: Option<(u32, usize)>,
    last_row: u32,
    last_col: u32,
    styles: HashMap<u32, Style>,
}

pub fn write(
    worksheet: &mut Worksheet,
    binding: &MasterBinding,
    table: &Table,
) -> AppResult<Outcome> {
    let layout = layout(worksheet, binding, table)?;
    let mut outcome = match binding.mode {
        MasterMode::Snapshot => snapshot(worksheet, &layout, binding, table),
        MasterMode::Feed => feed(worksheet, &layout, binding, table)?,
    };
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
                structure
                    .matched
                    .push(classified.master.text(*col, 1).trim().to_string());
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

    let source_of = |header: &str| -> String {
        binding
            .aliases
            .iter()
            .find(|alias| alias.master.trim() == header)
            .map_or(header, |alias| alias.source.trim())
            .to_string()
    };

    let mut columns = BTreeMap::new();
    for col in 1..=last_col {
        let header = master.text(col, 1).trim().to_string();
        let kind = if let Some(index) = (!header.is_empty())
            .then(|| table.column(&source_of(&header)))
            .flatten()
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
        last_col,
    })
}

fn layout(worksheet: &Worksheet, binding: &MasterBinding, table: &Table) -> AppResult<Layout> {
    let Classified {
        master,
        columns,
        last_row,
        last_col,
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
        last_col,
        styles,
    })
}

fn snapshot(
    worksheet: &mut Worksheet,
    layout: &Layout,
    binding: &MasterBinding,
    table: &Table,
) -> Outcome {
    let master = grid::from_worksheet(worksheet).ok();
    let mut kept: HashMap<String, Vec<Cell>> = HashMap::new();
    if let (Some((key_col, _)), Some(master)) = (layout.key, master.as_ref()) {
        for row in 2..=layout.last_row {
            let key = master.text(key_col, row).trim().to_string();
            if key.is_empty() {
                continue;
            }
            for (col, kind) in &layout.columns {
                if let (Kind::Hand(_), Some(cell)) = (kind, worksheet.cell((*col, row))) {
                    kept.entry(key.clone()).or_default().push(cell.clone());
                }
            }
        }
    }

    for row in 2..=layout.last_row {
        for col in 1..=layout.last_col {
            worksheet.remove_cell((col, row));
        }
    }

    for (offset, values) in table.rows.iter().enumerate() {
        let row = offset as u32 + 2;
        write_new_row(worksheet, layout, row, values);
        if let Some((_, index)) = layout.key {
            for mut cell in kept.remove(&values[index].key()).unwrap_or_default() {
                let col = cell.coordinate().col_num();
                cell.set_coordinate((col, row));
                worksheet.set_cell(cell);
            }
        }
    }
    let last = table.rows.len() as u32 + 1;
    stretch_filter(worksheet, last);

    let mut notes = Vec::new();
    for kind in layout.columns.values() {
        if let Kind::Hand(header) = kind {
            notes.push(match layout.key {
                Some(_) => format!(
                    "„{}“: Die Spalte „{header}“ hat keine Quelle; ihre Werte sind über den Schlüssel mitgewandert.",
                    binding.sheet
                ),
                None => format!(
                    "„{}“: Die Spalte „{header}“ hat keine Quelle und ist jetzt leer.",
                    binding.sheet
                ),
            });
        }
    }
    if !kept.is_empty() {
        notes.push(format!(
            "„{}“: Handeinträge zu {} Schlüssel(n), die in der neuen Datei fehlen, wurden nicht übernommen.",
            binding.sheet,
            kept.len()
        ));
    }

    Outcome {
        line: format!(
            "„{}“: {} Zeile(n) aus „{}“ übernommen (vorher {}).",
            binding.sheet,
            table.rows.len(),
            table.name,
            layout.last_row - 1
        ),
        notes,
        ..Outcome::default()
    }
}

fn feed(
    worksheet: &mut Worksheet,
    layout: &Layout,
    binding: &MasterBinding,
    table: &Table,
) -> AppResult<Outcome> {
    let Some((key_col, key_index)) = layout.key else {
        return Err(AppError::Report(vec![format!(
            "„{}“ wird fortlaufend ergänzt und braucht dafür eine Schlüsselspalte.",
            binding.sheet
        )]));
    };
    let master = grid::from_worksheet(worksheet)?;
    let mut rows: HashMap<String, u32> = (2..=layout.last_row)
        .map(|row| (master.text(key_col, row).trim().to_string(), row))
        .filter(|(key, _)| !key.is_empty())
        .collect();

    let (mut appended, mut updated, mut keyless) = (0_u32, 0_u32, 0_u32);
    let mut next = layout.last_row + 1;
    for values in &table.rows {
        let key = values[key_index].key();
        if key.is_empty() {
            keyless += 1;
            continue;
        }
        match rows.get(&key) {
            Some(row) => {
                for (col, kind) in &layout.columns {
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
            None => {
                write_new_row(worksheet, layout, next, values);
                rows.insert(key, next);
                next += 1;
                appended += 1;
            }
        }
    }
    stretch_filter(worksheet, next - 1);

    let mut notes = Vec::new();
    if keyless > 0 {
        notes.push(format!(
            "„{}“: {keyless} Zeile(n) ohne Schlüssel wurden übersprungen.",
            binding.sheet
        ));
    }
    Ok(Outcome {
        line: format!(
            "„{}“: {appended} Zeile(n) angehängt, {updated} aktualisiert aus „{}“.",
            binding.sheet, table.name
        ),
        notes,
        ..Outcome::default()
    })
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

    fn binding(mode: MasterMode, key: Option<&str>) -> MasterBinding {
        MasterBinding {
            sheet: "Blatt".into(),
            template_id: "t".into(),
            kind: None,
            mode,
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
    fn a_snapshot_replaces_the_rows_and_carries_the_formula_down_with_its_row() {
        let mut sheet = worksheet();
        let source = table(
            &["Wagen", "Stadt"],
            &[
                &[n(2.0), t("Neuhof")],
                &[n(3.0), t("Fulda")],
                &[n(4.0), t("Kassel")],
            ],
        );
        let outcome = write(&mut sheet, &binding(MasterMode::Snapshot, None), &source).unwrap();

        assert_eq!(sheet.cell((1u32, 2u32)).unwrap().value_number(), Some(2.0));
        assert_eq!(value(&sheet, 3, 4), "Kassel");
        // Relative row moved, `$A$1` and the whole column `A:A` did not.
        assert_eq!(
            sheet.cell((2u32, 4u32)).unwrap().formula(),
            "TODAY()-A4+$A$1+COUNT(A:A)"
        );
        assert_eq!(
            sheet.cell((2u32, 2u32)).unwrap().formula(),
            "TODAY()-A2+$A$1+COUNT(A:A)"
        );
        assert!(outcome.line.contains("3 Zeile(n)"), "{}", outcome.line);
        assert!(outcome
            .notes
            .iter()
            .any(|note| note.contains("„Notiz“") && note.contains("leer")));
        assert_eq!(
            value(&sheet, 4, 2),
            "",
            "a hand column without a key is cleared"
        );
    }

    #[test]
    fn a_shorter_snapshot_clears_the_stale_rows_formula_included() {
        let mut sheet = worksheet();
        let source = table(&["Wagen", "Stadt"], &[&[n(9.0), t("Neuhof")]]);
        write(&mut sheet, &binding(MasterMode::Snapshot, None), &source).unwrap();
        assert!(sheet.cell((1u32, 3u32)).is_none());
        assert!(sheet.cell((2u32, 3u32)).is_none());
    }

    #[test]
    fn with_a_key_a_hand_cell_travels_with_its_row() {
        let mut sheet = worksheet();
        // Wagen 1 is gone, wagen 2 moves up to row 2: the note on wagen 1 has
        // nowhere to go, and nothing may land on the wrong wagen.
        let source = table(
            &["Wagen", "Stadt"],
            &[&[n(2.0), t("Neuhof")], &[n(1.0), t("Fulda")]],
        );
        let outcome = write(
            &mut sheet,
            &binding(MasterMode::Snapshot, Some("Wagen")),
            &source,
        )
        .unwrap();
        assert_eq!(value(&sheet, 4, 2), "");
        assert_eq!(
            value(&sheet, 4, 3),
            "prüfen",
            "the note followed wagen 1 to row 3"
        );
        assert!(outcome
            .notes
            .iter()
            .any(|note| note.contains("mitgewandert")));
    }

    #[test]
    fn a_feed_appends_new_keys_updates_known_ones_and_deletes_nothing() {
        let mut sheet = worksheet();
        let source = table(
            &["Wagen", "Stadt"],
            &[&[n(2.0), t("Neuhof")], &[n(7.0), t("Fulda")]],
        );
        let outcome = write(
            &mut sheet,
            &binding(MasterMode::Feed, Some("Wagen")),
            &source,
        )
        .unwrap();

        assert_eq!(
            value(&sheet, 3, 2),
            "Alt",
            "wagen 1 is not in the file and stays"
        );
        assert_eq!(value(&sheet, 4, 2), "prüfen");
        assert_eq!(
            value(&sheet, 3, 3),
            "Neuhof",
            "wagen 2 was updated in place"
        );
        assert_eq!(sheet.cell((1u32, 4u32)).unwrap().value_number(), Some(7.0));
        assert_eq!(
            sheet.cell((2u32, 4u32)).unwrap().formula(),
            "TODAY()-A4+$A$1+COUNT(A:A)"
        );
        assert!(
            outcome
                .line
                .contains("1 Zeile(n) angehängt, 1 aktualisiert"),
            "{}",
            outcome.line
        );
    }

    #[test]
    fn a_feed_without_a_key_is_refused() {
        let mut sheet = worksheet();
        let source = table(&["Wagen"], &[&[n(2.0)]]);
        let error = write(&mut sheet, &binding(MasterMode::Feed, None), &source).unwrap_err();
        assert!(error.into_messages()[0].contains("Schlüsselspalte"));
    }

    #[test]
    fn an_alias_maps_a_renamed_master_column_onto_its_source() {
        let mut sheet = worksheet();
        let mut bound = binding(MasterMode::Snapshot, None);
        bound.aliases.push(MasterAlias {
            master: "Stadt".into(),
            source: "ort".into(),
        });
        let source = table(&["Wagen", "ort"], &[&[n(2.0), t("Neuhof")]]);
        write(&mut sheet, &bound, &source).unwrap();
        assert_eq!(value(&sheet, 3, 2), "Neuhof");
    }

    #[test]
    fn a_sheet_sharing_no_header_with_the_source_is_refused_untouched() {
        let mut sheet = worksheet();
        let source = table(&["Etwas anderes"], &[&[t("x")]]);
        let error = write(&mut sheet, &binding(MasterMode::Snapshot, None), &source).unwrap_err();
        assert!(error.into_messages()[0].contains("keine Spalte"));
        assert_eq!(value(&sheet, 3, 2), "Alt");
    }

    #[test]
    fn the_structure_names_matched_hand_kept_and_unplaced_columns() {
        let sheet = worksheet();
        let source = table(&["Wagen", "Ort", "Extra"], &[&[n(2.0), t("x"), t("y")]]);
        let structure = structure(&sheet, &binding(MasterMode::Snapshot, None), &source).unwrap();
        assert_eq!(structure.matched, ["Wagen"]);
        // The formula column has no header and is neither.
        assert_eq!(structure.hand, ["Stadt", "Notiz"]);
        assert_eq!(structure.unmatched, ["Ort", "Extra"]);
    }

    #[test]
    fn the_autofilter_grows_with_the_rows() {
        let mut sheet = worksheet();
        sheet.set_auto_filter("A1:D3");
        let source = table(&["Wagen"], &[&[n(1.0)], &[n(2.0)], &[n(3.0)], &[n(4.0)]]);
        write(&mut sheet, &binding(MasterMode::Snapshot, None), &source).unwrap();
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
        write(&mut sheet, &binding(MasterMode::Snapshot, None), &source).unwrap();
        assert_eq!(
            sheet.cell((1u32, 2u32)).unwrap().value_number(),
            Some(180_028_676.0)
        );
        assert_eq!(
            sheet.cell((1u32, 3u32)).unwrap().value_number(),
            Some(6715.0)
        );
        assert_eq!(
            sheet.cell((1u32, 4u32)).unwrap().data_type(),
            "s",
            "not a plain decimal"
        );
        assert_eq!(
            sheet.cell((3u32, 2u32)).unwrap().data_type(),
            "s",
            "a text column stays text"
        );
    }
}
