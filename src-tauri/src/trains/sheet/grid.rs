// ─── why ────────────────────────────────────────────────────────
// The only file in `trains/` that knows umya on the READ side. Everything above
// it works on a `Grid`, which is why a reader can be written and tested without
// a workbook anywhere near it.
//
// Coordinates are `(col, row)` numbers and NEVER strings. `doc/xlsx/address.rs`
// records that umya's `From<&str> for CellCoordinates` unwraps a half-parsed
// address and PANICS, and a panic is not an `AppError`, so the German dialog is
// lost. Here that trap does not need guarding against: there is no code path
// that could build a coordinate string.
//
// The bounds come from the CELLS THAT HOLD SOMETHING, not from
// `highest_column_and_row()`. Measured (docs/state.md): a sheet with three real
// cells plus one empty-but-formatted cell at (20, 500) reports its used range as
// (20, 500). Trusting that is an allocation three orders of magnitude past the
// data on a file that looks like forty rows in Excel. `MAX_ROWS` and `MAX_COLS`
// bound what is left: past those the file is not a hand-kept maintenance list
// any more, and saying so beats allocating for it.
//
// A `RawCell` carries all three forms because each answers a different question.
// `text` is `value()` — the raw stringification, NOT `formatted_value()`, which
// does not apply the number format and answers `45000` for a formatted date.
// `number` is `value_number()`, `Some` only when Excel stored a number; when it
// is, re-parsing `text` is always a mistake, because that stringification uses a
// `.` decimal whatever the file's own style is. `date_format` is a HINT about
// this one cell, aggregated by `sanitise::column::looks_like_dates` — real files
// carry it on some rows of a date column and not others.
//
// The grid is dense so that "is row 7 mostly empty" is answerable: with a sparse
// map a lookup miss means both "blank" and "outside the data", and a reader has
// to tell those apart. Indices are 1-based, like the numbers on Excel's rulers.
//
// `from_text` is the readers' test builder, and it calls a cell numeric ONLY for
// a plain run of ASCII digits. Handing `"1.234"` to Rust's f64 parser would make
// a German-formatted text cell look like a stored number — the exact confusion
// the real `number` field exists to prevent, faked into the fixtures.
//
// `read` parses ONE sheet: `lazy_read` loads the sheet list and the shared
// strings, and only the chosen sheet is deserialised. A full `read` builds every
// sheet the sender's file holds, and those files are not tidy — measured on a
// real 28-sheet workbook with two sheets filled down to row 1,048,576: 6.5 s and
// 2.6 GB for the full read against 0.4 s and 300 MB for one sheet. The index is
// resolved here rather than by `read_sheet_by_name`, which unwraps an unknown
// name. Both umya calls run inside `AppError::reading`, because umya's parser
// has panicked on real files before.
//
// `read` takes a PATH and `from_worksheet` takes a workbook already open. The
// master exporter has to hold a writable `Workbook` anyway, and going back to
// the path for the grid parsed the same file a second time — double the parse
// and roughly double the peak memory, on the one file whose loss would end the
// project.
//
// `is_date_format` runs on EVERY non-empty cell, so it neither lowercases nor
// allocates: `eq_ignore_ascii_case` for the early out and a per-character
// `to_ascii_uppercase` in the loop. A `to_uppercase()` up front was one `String`
// per cell of the sheet.
//
// `is_date_format` decides on the format CODE rather than the built-in id,
// because the ids that matter are mostly custom — the probe saw 164 (`GENERAL`),
// 165 (`MM/DD/YY`) and 176 (`DD.MM.YYYY`) in two ordinary files. A `Y` or `D`
// outside a literal or a bracket means a date; `M` alone does not, because it is
// minutes in `h:mm:ss` and a time is not a date in this model.
// ────────────────────────────────────────────────────────────────

use std::path::Path;

use crate::error::{AppError, AppResult};

pub const MAX_ROWS: u32 = 100_000;
pub const MAX_COLS: u32 = 1_024;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RawCell {
    pub text: String,
    pub number: Option<f64>,
    pub date_format: bool,
}

impl RawCell {
    pub fn is_empty(&self) -> bool {
        self.text.trim().is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct Grid {
    pub sheet: String,
    pub rows: u32,
    pub cols: u32,
    cells: Vec<RawCell>,
}

impl Grid {
    pub fn cell(&self, col: u32, row: u32) -> Option<&RawCell> {
        if col < 1 || row < 1 || col > self.cols || row > self.rows {
            return None;
        }
        let index = (row - 1) as usize * self.cols as usize + (col - 1) as usize;
        self.cells.get(index)
    }

    pub fn text(&self, col: u32, row: u32) -> &str {
        self.cell(col, row).map_or("", |cell| cell.text.as_str())
    }

    pub fn row(&self, row: u32) -> &[RawCell] {
        if row < 1 || row > self.rows {
            return &[];
        }
        let start = (row - 1) as usize * self.cols as usize;
        &self.cells[start..start + self.cols as usize]
    }

    pub fn column(&self, col: u32, first_row: u32, last_row: u32) -> Vec<&RawCell> {
        (first_row..=last_row.min(self.rows))
            .filter_map(|row| self.cell(col, row))
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.rows == 0 || self.cols == 0
    }

    #[cfg(test)]
    pub fn from_text(sheet: &str, rows: &[&[&str]]) -> Self {
        let cols = rows.iter().map(|row| row.len()).max().unwrap_or(0) as u32;
        let mut cells = Vec::with_capacity(rows.len() * cols as usize);
        for row in rows {
            for index in 0..cols as usize {
                let text = row.get(index).copied().unwrap_or("").to_string();
                let digits = text.strip_prefix('-').unwrap_or(&text);
                let number = (!digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()))
                    .then(|| text.parse::<f64>().ok())
                    .flatten();
                cells.push(RawCell {
                    text,
                    number,
                    date_format: false,
                });
            }
        }
        Self {
            sheet: sheet.to_string(),
            rows: rows.len() as u32,
            cols,
            cells,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Source {
    pub grid: Grid,
    pub sheets: Vec<String>,
}

pub fn read(path: &Path, sheet: Option<&str>) -> AppResult<Source> {
    let headline = || {
        format!(
            "Die Excel-Datei {} konnte nicht gelesen werden.",
            crate::doc::file_name(path)
        )
    };
    let mut book = AppError::reading(headline(), || {
        umya_spreadsheet::reader::xlsx::lazy_read(path)
    })?;

    let sheets: Vec<String> = book
        .sheet_collection_no_check()
        .iter()
        .map(|sheet| sheet.name().to_string())
        .collect();
    if sheets.is_empty() {
        return Err(AppError::Report(vec![
            "Die Excel-Datei enthält keine Arbeitsmappen.".into(),
        ]));
    }

    let index = match sheet {
        Some(wanted) => sheets
            .iter()
            .position(|name| name == wanted)
            .ok_or_else(|| {
                AppError::Report(vec![
                    format!("Die Arbeitsmappe „{wanted}“ gibt es in dieser Datei nicht."),
                    format!("Vorhanden sind: {}.", sheets.join(", ")),
                ])
            })?,
        None => 0,
    };

    AppError::reading(headline(), || {
        book.read_sheet(index);
        Ok::<_, std::convert::Infallible>(())
    })?;

    Ok(Source {
        grid: from_worksheet(&book.sheet_collection_no_check()[index])?,
        sheets,
    })
}

pub fn from_worksheet(worksheet: &umya_spreadsheet::Worksheet) -> AppResult<Grid> {
    let cells = worksheet.cells();

    let mut rows = 0_u32;
    let mut cols = 0_u32;
    for cell in &cells {
        if cell.value().trim().is_empty() {
            continue;
        }
        rows = rows.max(cell.coordinate().row_num());
        cols = cols.max(cell.coordinate().col_num());
    }

    if rows > MAX_ROWS {
        return Err(AppError::Report(vec![
            format!(
                "Die Arbeitsmappe „{}“ enthält {rows} Zeilen.",
                worksheet.name()
            ),
            format!("Verarbeitet werden höchstens {MAX_ROWS}. Bitte die Datei aufteilen."),
        ]));
    }
    cols = cols.min(MAX_COLS);

    let mut grid = vec![RawCell::default(); rows as usize * cols as usize];
    for cell in &cells {
        let (col, row) = (cell.coordinate().col_num(), cell.coordinate().row_num());
        if col < 1 || row < 1 || col > cols || row > rows {
            continue;
        }
        let index = (row - 1) as usize * cols as usize + (col - 1) as usize;
        grid[index] = RawCell {
            text: cell.value().to_string(),
            number: cell.value_number(),
            date_format: cell
                .style()
                .number_format()
                .is_some_and(|format| is_date_format(format.format_code())),
        };
    }

    Ok(Grid {
        sheet: worksheet.name().to_string(),
        rows,
        cols,
        cells: grid,
    })
}

pub fn is_date_format(code: &str) -> bool {
    if code.is_empty() || code.eq_ignore_ascii_case("GENERAL") {
        return false;
    }

    let mut in_quotes = false;
    let mut in_brackets = false;
    let mut escaped = false;
    for character in code.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        match character.to_ascii_uppercase() {
            '\\' => escaped = true,
            '"' => in_quotes = !in_quotes,
            '[' => in_brackets = true,
            ']' => in_brackets = false,
            'Y' | 'D' if !in_quotes && !in_brackets => return true,
            _ => {}
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_date_format_is_recognised_by_its_code() {
        assert!(is_date_format("DD.MM.YYYY"));
        assert!(is_date_format("MM/DD/YY"));
        assert!(is_date_format("d-mmm-yy"));
        assert!(is_date_format("YYYY-MM-DD HH:MM:SS"));
    }

    /// The codes the probe actually saw on non-date cells.
    #[test]
    fn general_and_plain_numbers_are_not_dates() {
        assert!(!is_date_format("GENERAL"));
        assert!(!is_date_format("General"));
        assert!(!is_date_format(""));
        assert!(!is_date_format("0.00"));
        assert!(!is_date_format("#,##0.00"));
    }

    /// A time is not a date here: `M` is minutes, and this model has no time.
    #[test]
    fn a_time_only_format_is_not_a_date() {
        assert!(!is_date_format("h:mm:ss"));
        assert!(!is_date_format("[h]:mm"));
    }

    /// A `D` inside a literal or a locale bracket is not a date token.
    #[test]
    fn letters_inside_literals_and_brackets_do_not_count() {
        assert!(!is_date_format(r#""Tag" 0"#));
        assert!(!is_date_format(r#"[$-407]0.00"#));
        assert!(!is_date_format(r#"0" DM""#));
    }

    #[test]
    fn a_grid_indexes_from_one_and_answers_nothing_outside_itself() {
        let grid = Grid::from_text("Tabelle1", &[&["a", "b"], &["c", "d"]]);
        assert_eq!(grid.text(1, 1), "a");
        assert_eq!(grid.text(2, 2), "d");
        assert_eq!(grid.cell(0, 1), None);
        assert_eq!(grid.cell(3, 1), None);
        assert_eq!(grid.cell(1, 3), None);
    }

    #[test]
    fn a_short_row_is_padded_so_every_row_is_the_same_width() {
        let grid = Grid::from_text("Tabelle1", &[&["a", "b", "c"], &["d"]]);
        assert_eq!(grid.cols, 3);
        assert_eq!(grid.row(2).len(), 3);
        assert!(grid.cell(3, 2).unwrap().is_empty());
    }

    #[test]
    fn a_column_reads_the_rows_it_was_asked_for_and_stops_at_the_end() {
        let grid = Grid::from_text("Tabelle1", &[&["kopf"], &["1"], &["2"]]);
        let values: Vec<&str> = grid
            .column(1, 2, 99)
            .iter()
            .map(|cell| cell.text.as_str())
            .collect();
        assert_eq!(values, ["1", "2"]);
    }
}
