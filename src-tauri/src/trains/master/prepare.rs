// ─── why ────────────────────────────────────────────────────────
// The master is READ from a trimmed copy, the „Lesekopie“. umya has no partial
// read: `read_sheet` deserialises a whole sheet, and five of the customer's
// sheets are filled down to row 1,048,576 — a formula pulled to the end of the
// sheet, a `0` or a cached `#N/A` in every row below the data (the largest is
// 72 MB of XML). Measured on the real file: reading every sheet once costs
// 6.2 s and ~1.7 GB. Every staging and every sheet view paid its sheet's share
// again. `copy` pays it ONCE per version of the file and writes a book whose
// sheets end where their data ends.
//
// The cut is at the last row holding anything but empty, `0` or a formula
// error — the rule `grid::from_master` applies, which keeps trimming as a guard.
// Everything below goes: `remove_cell` per cell, and the row records with it,
// or the copy would still declare a million empty rows. Not umya's `cleanup()`,
// which stops at the first visible `0`, and not `remove_row`, which rewrites
// every formula reference in the book for each row it removes. Nothing above
// the cut is touched, so a cell reads the same from the copy as from the
// original: umya writes a formula cell's cached value back out with it.
//
// The copy is INTERNAL, under `data/trains/master/`, never beside the
// customer's file — a second workbook in their folder would be opened, edited
// and lost. One copy at a time: the folder is emptied before a new one is
// written, so a new version leaves no copy of the old one behind. It is
// called „Lesekopie“ and not „bereinigt“, because Bereinigen is the cleaning
// walk and this is not one.
//
// Only READERS use the copy — the import's `stage_sheet` and the sheet views.
// The export builds a new version from the CURRENT VERSION itself and must carry
// every row the customer has, so it never opens this one; provenance keeps naming the
// original file and the customer's sheet.
// ────────────────────────────────────────────────────────────────

use std::path::{Path, PathBuf};

use umya_spreadsheet::Worksheet;

use super::book;
use crate::error::{AppError, AppResult};
use crate::trains::db::{remove_folder, TrainsDb};
use crate::trains::model::MasterSheet;
use crate::trains::sheet::grid;

pub fn copy_path(db: &TrainsDb, original: &Path) -> PathBuf {
    let stem = original.file_stem().unwrap_or_default().to_string_lossy();
    db.master_folder().join(format!("{stem} Lesekopie.xlsx"))
}

pub fn existing(db: &TrainsDb) -> AppResult<PathBuf> {
    let original = db.master().file.as_deref().map(Path::new).ok_or_else(|| {
        AppError::Report(vec!["Es ist noch keine Master-Datei übernommen.".into()])
    })?;
    let copy = copy_path(db, original);
    if copy.is_file() {
        return Ok(copy);
    }
    Err(AppError::Report(vec![
        "Die Master-Datei ist noch nicht eingelesen.".into(),
        "Bitte die Master-Einstellungen einmal öffnen, dann wird sie eingelesen.".into(),
    ]))
}

pub fn copy(db: &TrainsDb, original: &Path) -> AppResult<Vec<MasterSheet>> {
    let mut workbook = book::open(original)?;
    let count = book::names(&workbook).len();
    if count == 0 {
        return Err(AppError::Report(vec![
            "Die Master-Datei enthält keine Arbeitsmappen.".into(),
        ]));
    }

    let mut sheets = Vec::with_capacity(count);
    for index in 0..count {
        book::deserialise(&mut workbook, index, original)?;
        let worksheet = workbook.sheet_mut(index).map_err(|error| {
            AppError::detail(
                format!(
                    "Die Master-Datei {} konnte nicht gelesen werden.",
                    crate::doc::file_name(original)
                ),
                error,
            )
        })?;
        trim(worksheet);
        let head = grid::head(worksheet)?;
        sheets.push(MasterSheet {
            headers: (1..=head.cols)
                .map(|col| head.text(col, 1).trim().to_string())
                .filter(|text| !text.is_empty())
                .collect(),
            name: head.sheet,
        });
    }

    let folder = db.master_folder();
    remove_folder(&folder)?;
    std::fs::create_dir_all(&folder).map_err(|error| AppError::io(&folder, error))?;
    crate::doc::write_book(
        &workbook,
        &copy_path(db, original),
        "Die Lesekopie der Master-Datei konnte nicht geschrieben werden.".into(),
    )?;
    Ok(sheets)
}

fn trim(worksheet: &mut Worksheet) {
    let last = worksheet
        .cells()
        .into_iter()
        .filter(|cell| holds_data(&cell.value()))
        .map(|cell| cell.coordinate().row_num())
        .max()
        .unwrap_or(0);
    let below: Vec<(u32, u32)> = worksheet
        .cells()
        .into_iter()
        .map(|cell| (cell.coordinate().col_num(), cell.coordinate().row_num()))
        .filter(|(_, row)| *row > last)
        .collect();
    for coordinate in below {
        worksheet.remove_cell(coordinate);
    }
    worksheet
        .row_dimensions_to_hashmap_mut()
        .retain(|row, _| *row <= last);
}

fn holds_data(text: &str) -> bool {
    let text = text.trim();
    !text.is_empty() && text != "0" && !grid::is_error(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{workbook, TempDir};

    fn db(folder: &TempDir) -> TrainsDb {
        TrainsDb::load(&folder.config()).unwrap()
    }

    fn read(path: &Path, sheet: &str) -> Worksheet {
        let mut book = umya_spreadsheet::reader::xlsx::read(path).unwrap();
        book.sheet_by_name_mut(sheet).unwrap().clone()
    }

    fn rows(worksheet: &Worksheet) -> u32 {
        worksheet
            .cells()
            .iter()
            .map(|cell| cell.coordinate().row_num())
            .max()
            .unwrap_or(0)
    }

    fn master(folder: &TempDir) -> PathBuf {
        workbook(
            folder,
            "Master.xlsx",
            &[
                (
                    "Bestand",
                    &[
                        &["radsatz", "einbau_am"],
                        &["RS1", "#45600"],
                        // An inner zero is data; only the tail is cut.
                        &["#0", "#0"],
                        &["RS2", "#45500"],
                        &["#0", "#0"],
                        &["#N/A", "#0"],
                        &["", "#0"],
                    ],
                ),
                ("Liste", &[&["Wagennummer"], &["#218124712173"]]),
            ],
        )
    }

    #[test]
    fn the_tail_is_cut_and_every_real_row_reads_as_before() {
        let folder = TempDir::new("prepare-trim");
        let db = db(&folder);
        let original = master(&folder);
        let sheets = copy(&db, &original).unwrap();

        let copy = read(&copy_path(&db, &original), "Bestand");
        assert_eq!(rows(&copy), 4, "rows 5 to 7 are tail");
        let before = read(&original, "Bestand");
        for row in 1..=4 {
            for col in 1..=2 {
                assert_eq!(
                    copy.value((col, row)),
                    before.value((col, row)),
                    "({col}, {row})"
                );
            }
        }
        assert_eq!(sheets[0].headers, ["radsatz", "einbau_am"]);
    }

    #[test]
    fn a_sheet_without_a_tail_is_unchanged() {
        let folder = TempDir::new("prepare-untouched");
        let db = db(&folder);
        let original = master(&folder);
        let sheets = copy(&db, &original).unwrap();

        let copy = read(&copy_path(&db, &original), "Liste");
        assert_eq!(rows(&copy), 2);
        assert_eq!(copy.value((1, 2)), "218124712173");
        assert_eq!(sheets[1].name, "Liste");
        assert_eq!(sheets[1].headers, ["Wagennummer"]);
    }

    #[test]
    fn the_copy_lives_in_the_data_folder_and_the_original_is_not_written() {
        let folder = TempDir::new("prepare-where");
        let db = db(&folder);
        let original = master(&folder);
        let bytes = std::fs::read(&original).unwrap();
        copy(&db, &original).unwrap();

        let path = existing(&db);
        assert!(path.is_err(), "no file chosen yet");
        let copy = copy_path(&db, &original);
        assert!(copy.starts_with(db.master_folder()), "{}", copy.display());
        assert!(copy.is_file());
        assert_eq!(std::fs::read(&original).unwrap(), bytes);
        let beside: Vec<_> = std::fs::read_dir(original.parent().unwrap())
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_name().to_string_lossy().contains("Lesekopie"))
            .collect();
        assert!(beside.is_empty(), "nothing written beside the original");
    }

    #[test]
    fn a_new_file_leaves_no_copy_of_the_old_one_behind() {
        let folder = TempDir::new("prepare-one");
        let db = db(&folder);
        let first = master(&folder);
        copy(&db, &first).unwrap();
        let second = workbook(&folder, "Neu.xlsx", &[("Liste", &[&["Wagennummer"]])]);
        copy(&db, &second).unwrap();

        let names: Vec<String> = std::fs::read_dir(db.master_folder())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["Neu Lesekopie.xlsx"]);
    }

    #[test]
    fn a_reset_removes_the_copy() {
        let folder = TempDir::new("prepare-reset");
        let mut db = db(&folder);
        let original = master(&folder);
        copy(&db, &original).unwrap();
        db.reset().unwrap();
        assert!(!db.master_folder().exists());
    }
}
