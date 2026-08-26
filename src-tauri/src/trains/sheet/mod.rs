// ─── why ────────────────────────────────────────────────────────
// A file becomes a `Grid`, and a reader says WHERE in that grid the data is.
// Those are two separable questions, and keeping them separate is what makes a
// second reader cheap: `grid` knows umya and nothing about tables, a reader
// knows tables and nothing about umya.
//
// A reader never produces VALUES. It answers "which cells are the data" and
// stops — sanitising happens afterwards, in `sanitise`, on the cells the reader
// pointed at. That is why a shape as different as a one-record-per-sheet form
// can be added without anything downstream changing.
// ────────────────────────────────────────────────────────────────

pub mod grid;
pub mod layout;
pub mod readers;

#[cfg(test)]
mod tests {
    use super::grid;
    use super::readers;
    use crate::testing::fixture_xlsx;

    /// The whole seam against a workbook nobody wrote for this app: read it,
    /// score its rows, pick a header, and build the layout. The scoring weights
    /// are judgement and will move — so this asserts WHICH row wins and what
    /// came out, never the number it won by.
    #[test]
    fn the_seam_reads_a_real_workbook_end_to_end() {
        let source = grid::read(&fixture_xlsx(), None).expect("fixture readable");
        assert_eq!(source.sheets, ["Sheet1"]);
        assert_eq!(source.grid.rows, 1001);
        assert_eq!(source.grid.cols, 8);

        let candidates = readers::detect(&source.grid);
        let chosen = readers::choose(&candidates).expect("a header row is clear here");
        assert_eq!(chosen.hint.header_row, Some(1));
        assert_eq!(chosen.reader, readers::ReaderKind::HeaderRow);

        let layout = chosen
            .reader
            .apply(&source.grid, chosen.hint)
            .expect("layout builds");
        assert_eq!(layout.first_data_row, 2);
        assert_eq!(layout.last_data_row, 1001);

        let headers: Vec<&str> = layout
            .columns
            .iter()
            .map(|slot| slot.header.as_str())
            .collect();
        assert_eq!(
            headers,
            [
                "Spalte A",
                "First Name",
                "Last Name",
                "Gender",
                "Country",
                "Age",
                "Date",
                "Id"
            ]
        );
    }

    /// The date column is stored as text with a date number format on only some
    /// rows — so the per-cell hint disagrees down one column, and the aggregate
    /// is what has to answer.
    #[test]
    fn the_date_column_is_only_recognisable_in_aggregate() {
        let source = grid::read(&fixture_xlsx(), None).expect("fixture readable");
        let hints: Vec<bool> = source
            .grid
            .column(7, 2, 1001)
            .iter()
            .map(|cell| cell.date_format)
            .collect();

        assert!(hints.iter().any(|hint| *hint), "some rows carry the format");
        assert!(!hints.iter().all(|hint| *hint), "and some do not");
        assert!(crate::trains::sanitise::column::looks_like_dates(&hints));
    }

    #[test]
    fn asking_for_a_sheet_that_is_not_there_names_the_ones_that_are() {
        let error = grid::read(&fixture_xlsx(), Some("Tabelle9")).unwrap_err();
        let messages = error.into_messages();
        assert!(messages[0].contains("Tabelle9"), "{}", messages[0]);
        assert!(messages[1].contains("Sheet1"), "{}", messages[1]);
    }
}
