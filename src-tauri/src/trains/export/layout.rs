// ─── why ────────────────────────────────────────────────────────
// How a generated sheet LOOKS, in one place: a header row, typed cells, a
// number format per column, a frozen header and an AutoFilter. Every sheet of
// the master overview goes through here, so they cannot drift apart in style.
//
// Cells are TYPED, unlike `export::fill`'s text: the overview computes with
// them. A date is a serial with a date format, or `MINIFS` and `HEUTE()` have
// nothing to compare; a Wagennummer is a NUMBER in the compact spelling, because
// that is what the customer's pasted portal exports carry and a text key never
// matches a number in `SVERWEIS`. The grouped spelling is text by nature and
// stays text — the overview's own key column is written the same way, so the
// lookups agree either way.
//
// A Radsatznummer, a Schadcode (`3.3.4`) or a Bestellnummer is Text and gets
// the `@` format, or Excel turns it into a number or a date on the first edit.
//
// A conditional fill sets fgColor AND bgColor: a differential format reads the
// background where a cell style reads the foreground, and Excel and LibreOffice
// do not agree which one a solid `dxf` fill takes.
//
// Zero is hidden in the date and count formats (`;;`): `MINIFS` answers 0 when
// nothing matches, and 0 as a date is the 0th of January 1900.
// ────────────────────────────────────────────────────────────────

use umya_spreadsheet::structs::{
    ConditionalFormatValues, ConditionalFormatting, ConditionalFormattingRule, Formula, Pane,
    PaneStateValues, PaneValues, PatternValues, SequenceOfReferences, SheetView, Style,
};
use umya_spreadsheet::Worksheet;

use crate::trains::model::UicStyle;
use crate::trains::sanitise::date::days_from_civil;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Text,
    Wagen,
    Date,
    Moment,
    Count,
    Integer,
    Money,
}

impl Format {
    fn code(self) -> &'static str {
        match self {
            Format::Text => "@",
            Format::Wagen | Format::Integer => "0",
            Format::Date => "DD.MM.YYYY;;",
            Format::Moment => "DD.MM.YYYY hh:mm;;",
            Format::Count => "0;;",
            Format::Money => "#,##0.00",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Column {
    pub header: &'static str,
    pub format: Format,
    pub width: f64,
}

pub const fn column(header: &'static str, format: Format, width: f64) -> Column {
    Column {
        header,
        format,
        width,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    Empty,
    Text(String),
    Number(f64),
    Formula(String),
}

impl Cell {
    pub fn text(value: Option<impl Into<String>>) -> Cell {
        match value.map(Into::into) {
            Some(text) if !text.trim().is_empty() => Cell::Text(text),
            _ => Cell::Empty,
        }
    }

    pub fn date(iso: Option<&str>) -> Cell {
        iso.and_then(serial).map_or(Cell::Empty, Cell::Number)
    }

    pub fn number(value: Option<i64>) -> Cell {
        value.map_or(Cell::Empty, |value| Cell::Number(value as f64))
    }

    pub fn money(cents: Option<i64>) -> Cell {
        cents.map_or(Cell::Empty, |cents| Cell::Number(cents as f64 / 100.0))
    }

    pub fn yes_no(value: Option<bool>) -> Cell {
        value.map_or(Cell::Empty, |yes| {
            Cell::Text(if yes { "Ja" } else { "Nein" }.into())
        })
    }

    pub fn wagen(digits: &str, style: UicStyle) -> Cell {
        match (style, digits.parse::<f64>()) {
            (UicStyle::Compact, Ok(number)) => Cell::Number(number),
            _ => Cell::Text(crate::trains::sanitise::format::uic_in(digits, style)),
        }
    }
}

pub fn serial(iso: &str) -> Option<f64> {
    let (day, time) = iso.split_once(['T', ' ']).unwrap_or((iso, ""));
    let mut parts = day.split('-').map(str::parse::<i64>);
    let (Some(Ok(year)), Some(Ok(month)), Some(Ok(day)), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return None;
    };
    let days = days_from_civil(year as i32, month as u8, day as u8) - days_from_civil(1899, 12, 30);
    let seconds = time
        .split(':')
        .map(|part| part.parse::<f64>().unwrap_or(0.0))
        .zip([3600.0, 60.0, 1.0])
        .map(|(value, unit)| value * unit)
        .sum::<f64>();
    Some(days as f64 + seconds / 86_400.0)
}

pub fn letter(index: usize) -> String {
    let mut number = index + 1;
    let mut letters = Vec::new();
    while number > 0 {
        let rest = (number - 1) % 26;
        letters.push(b'A' + rest as u8);
        number = (number - 1) / 26;
    }
    letters.reverse();
    String::from_utf8(letters).unwrap_or_default()
}

pub fn letter_of(columns: &[Column], header: &str) -> String {
    let index = columns
        .iter()
        .position(|column| column.header == header)
        .unwrap_or_else(|| panic!("no column „{header}“"));
    letter(index)
}

pub fn put(sheet: &mut Worksheet, col: u32, row: u32, cell: &Cell, format: Format) {
    let target = sheet.cell_mut((col, row));
    match cell {
        Cell::Empty => {}
        Cell::Text(text) => {
            target.set_value_string(text.as_str());
        }
        Cell::Number(number) => {
            target.set_value_number(*number);
        }
        Cell::Formula(formula) => {
            target.set_formula(formula.as_str());
        }
    }
    target
        .style_mut()
        .number_format_mut()
        .set_format_code(format.code());
}

pub fn table(sheet: &mut Worksheet, top: u32, columns: &[Column], rows: &[Vec<Cell>], fill: &str) {
    header(sheet, top, columns, fill);
    for (offset, cells) in rows.iter().enumerate() {
        let row = top + 1 + offset as u32;
        for (index, (cell, column)) in cells.iter().zip(columns).enumerate() {
            put(sheet, index as u32 + 1, row, cell, column.format);
        }
    }
    let last = top + rows.len().max(1) as u32;
    sheet.set_auto_filter(format!("A{top}:{}{last}", letter(columns.len() - 1)));
}

pub fn header(sheet: &mut Worksheet, row: u32, columns: &[Column], fill: &str) {
    for (index, column) in columns.iter().enumerate() {
        heading(sheet, index as u32 + 1, row, column.header, fill);
        sheet
            .column_dimension_mut(&letter(index))
            .set_width(column.width);
    }
}

pub fn heading(sheet: &mut Worksheet, col: u32, row: u32, text: &str, fill: &str) {
    let cell = sheet.cell_mut((col, row));
    cell.set_value_string(text);
    let style = cell.style_mut();
    style.font_mut().set_bold(true);
    style.alignment_mut().set_wrap_text(true);
    style.set_background_color(fill);
}

pub fn freeze(sheet: &mut Worksheet, rows: u32, cols: u32) {
    let mut pane = Pane::default();
    if cols > 0 {
        pane.set_horizontal_split(cols as f64);
    }
    pane.set_vertical_split(rows as f64);
    pane.top_left_cell_mut()
        .set_coordinate(format!("{}{}", letter(cols as usize), rows + 1));
    pane.set_active_pane(PaneValues::BottomRight);
    pane.set_state(PaneStateValues::Frozen);
    let views = sheet.sheet_views_mut().sheet_view_list_mut();
    if views.is_empty() {
        views.push(SheetView::default());
    }
    views[0].set_pane(pane);
}

pub fn highlight(sheet: &mut Worksheet, range: &str, rule: &str, fill: &str) {
    let mut style = Style::default();
    style.set_background_color_with_pattern(fill, fill, PatternValues::Solid);
    let mut formula = Formula::default();
    formula.set_string_value(rule);
    let priority = sheet
        .conditional_formatting_collection()
        .iter()
        .map(|block| block.conditional_collection().len())
        .sum::<usize>() as i32
        + 1;
    let mut condition = ConditionalFormattingRule::default();
    condition
        .set_type(ConditionalFormatValues::Expression)
        .set_priority(priority)
        .set_style(style)
        .set_formula(formula);
    let mut references = SequenceOfReferences::default();
    references.set_sqref(range);
    let mut block = ConditionalFormatting::default();
    block.set_sequence_of_references(references);
    block.set_conditional_collection(vec![condition]);
    sheet.add_conditional_formatting_collection(block);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_date_becomes_the_serial_excel_counts_from_1899_12_30() {
        assert_eq!(serial("2023-03-15"), Some(45000.0));
    }

    #[test]
    fn a_moment_keeps_its_time_as_the_fraction() {
        assert_eq!(serial("2023-03-15T12:00:00"), Some(45000.5));
    }

    #[test]
    fn anything_that_is_not_an_iso_date_is_no_serial() {
        assert_eq!(serial("15.03.2023"), None);
        assert_eq!(serial(""), None);
    }

    #[test]
    fn column_letters_roll_over_after_z() {
        assert_eq!(letter(0), "A");
        assert_eq!(letter(25), "Z");
        assert_eq!(letter(26), "AA");
        assert_eq!(letter(27), "AB");
    }

    // The compact spelling must be a NUMBER: a pasted portal export keys on
    // numbers, and SVERWEIS never matches a number against text.
    #[test]
    fn a_compact_wagennummer_is_a_number_and_a_grouped_one_text() {
        assert_eq!(
            Cell::wagen("338506590011", UicStyle::Compact),
            Cell::Number(338506590011.0)
        );
        assert_eq!(
            Cell::wagen("338506590011", UicStyle::Grouped),
            Cell::Text("33 85 0659 001-1".into())
        );
    }

    #[test]
    fn blank_text_is_an_empty_cell() {
        assert_eq!(Cell::text(Some("  ")), Cell::Empty);
        assert_eq!(Cell::text(None::<String>), Cell::Empty);
    }
}
