// ─── why ────────────────────────────────────────────────────────
// The master workbook is the USER'S document. It has formulas, colours, filters,
// notes and columns this program invented no meaning for, and the only reason
// this can touch it at all is that umya round-trips everything it does not
// understand — which is the requirement `docs/decisions.md` chose the crate for.
//
// Three rules, and they are the whole module:
//   • UPDATE the cells this app owns, by header name, and touch nothing else
//   • APPEND a row whose key is not there yet
//   • NEVER DELETE A ROW. A wagen removed in the app stays in the master.
//     Deleting a human's row is unrecoverable and no automatic rule is worth it.
//
// The header row is found with the SAME `sheet` seam the import uses, so the
// master's own layout can drift — a column moved or inserted — without a code
// change here. That is the payoff of the reader being a general thing rather
// than an import-only thing.
//
// The write is temp-and-rename, the same atomicity `db` gives the stores, and a
// timestamped `.bak` is taken before the first write of a session. That is not
// paranoia: `docs/state.md` records that the XLSX path had never executed until
// this feature, and this is the one file whose loss would end the project.
//
// `.xlsm` is refused outright. umya has no VBA story, and silently dropping a
// macro project presents to the user as "my buttons are gone".
//
// The file is parsed ONCE: the writable `Workbook` has to be held anyway, so the
// grid is built from the worksheet already in hand rather than by handing the
// path back to `grid::read`. The columns are `export::INSTANDHALTUNG_COLUMNS` and the
// values `export::instandhaltung_row`, shared with the ERP sheet — the cells here are
// located BY HEADER TEXT, so a list that drifted from the ERP's would silently
// append a second set of columns instead of updating in place.
// ────────────────────────────────────────────────────────────────

use std::collections::HashMap;
use std::path::Path;

use umya_spreadsheet::Workbook;

use crate::error::{AppError, AppResult};
use crate::trains::db::TrainsDb;
use crate::trains::export::{instandhaltung_row, INSTANDHALTUNG_COLUMNS};
use crate::trains::sheet::grid;
use crate::trains::sheet::readers;

const SHEET: &str = "Wartungen";
const KEY_HEADER: &str = "Id";

pub fn upsert(db: &TrainsDb, path: &Path) -> AppResult<String> {
    if path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("xlsm"))
    {
        return Err(AppError::Report(vec![
            "Die Master-Datei ist eine .xlsm-Datei mit Makros.".into(),
            "Diese kann das Programm nicht schreiben, ohne die Makros zu verlieren.".into(),
        ]));
    }
    if !path.is_file() {
        return Ok(format!(
            "Keine Master-Datei unter {} — übersprungen.",
            path.display()
        ));
    }

    backup(path)?;

    let mut book = umya_spreadsheet::reader::xlsx::read(path).map_err(|error| {
        AppError::detail(
            format!(
                "Die Master-Datei {} konnte nicht gelesen werden.",
                crate::doc::file_name(path)
            ),
            error,
        )
    })?;

    let master_grid = {
        let worksheet = book
            .sheet_collection_no_check()
            .iter()
            .find(|sheet| sheet.name() == SHEET)
            .ok_or_else(|| {
                AppError::Report(vec![
                    format!("Die Master-Datei hat keine Arbeitsmappe „{SHEET}“."),
                    "Bitte eine Arbeitsmappe mit diesem Namen anlegen.".into(),
                ])
            })?;
        grid::from_worksheet(worksheet)?
    };

    let candidates = readers::detect(&master_grid);
    let hint = candidates
        .first()
        .map(|candidate| candidate.hint)
        .unwrap_or(crate::trains::sheet::layout::LayoutHint {
            header_row: Some(1),
            first_data_row: 2,
            last_data_row: None,
        });
    let header_row = hint.header_row.unwrap_or(1);

    let mut columns: HashMap<&str, u32> = HashMap::new();
    let mut next_column = master_grid.cols + 1;
    for owned in INSTANDHALTUNG_COLUMNS {
        let found = (1..=master_grid.cols)
            .find(|index| master_grid.text(*index, header_row).trim() == owned);
        columns.insert(
            owned,
            found.unwrap_or_else(|| {
                let index = next_column;
                next_column += 1;
                index
            }),
        );
    }

    let key_column = columns[KEY_HEADER];
    let mut rows_by_key: HashMap<String, u32> = HashMap::new();
    for row in hint.first_data_row..=master_grid.rows {
        let key = master_grid.text(key_column, row).trim().to_string();
        if !key.is_empty() {
            rows_by_key.insert(key, row);
        }
    }

    let mut updated = 0_u32;
    let mut appended = 0_u32;
    let mut next_row = master_grid.rows + 1;

    let sheet = book
        .sheet_by_name_mut(SHEET)
        .map_err(|error| AppError::detail(format!("Die Arbeitsmappe „{SHEET}“ fehlt."), error))?;

    for owned in INSTANDHALTUNG_COLUMNS {
        let column = columns[owned];
        if master_grid.text(column, header_row).trim() != owned {
            sheet.cell_mut((column, header_row)).set_value(owned);
        }
    }

    for event in db.instandhaltungen() {
        let row = match rows_by_key.get(&event.id) {
            Some(row) => {
                updated += 1;
                *row
            }
            None => {
                appended += 1;
                let row = next_row;
                next_row += 1;
                row
            }
        };
        for (owned, value) in INSTANDHALTUNG_COLUMNS
            .iter()
            .zip(instandhaltung_row(db, event).iter())
        {
            sheet.cell_mut((columns[owned], row)).set_value(value);
        }
    }

    write_atomically(&book, path)?;
    Ok(format!(
        "Master-Datei aktualisiert: {updated} Zeile(n) überschrieben, {appended} angehängt."
    ))
}

fn backup(path: &Path) -> AppResult<()> {
    let stamp = crate::trains::clock::today_iso();
    let backup = path.with_extension(format!("{stamp}.bak.xlsx"));
    if backup.exists() {
        return Ok(());
    }
    std::fs::copy(path, &backup)
        .map(|_| ())
        .map_err(|error| AppError::io(&backup, error))
}

fn write_atomically(book: &Workbook, path: &Path) -> AppResult<()> {
    let temp = path.with_extension("tmp.xlsx");
    umya_spreadsheet::writer::xlsx::write(book, &temp).map_err(|error| {
        AppError::detail(
            format!(
                "Die Master-Datei {} konnte nicht geschrieben werden.",
                crate::doc::file_name(path)
            ),
            error,
        )
    })?;
    std::fs::rename(&temp, path).map_err(|error| AppError::io(path, error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TempDir;
    use crate::trains::model::{Instandhaltung, Provenance, Wagen};

    fn seeded(label: &str) -> (TempDir, TrainsDb) {
        let folder = TempDir::new(label);
        let mut db = TrainsDb::load(&folder.config()).unwrap();
        db.transaction(|tx| {
            tx.put_wagen(Wagen {
                id: "w1".into(),
                nummer: "218124712173".into(),
                halter_id: None,
                eigentuemer_id: None,
                bauart: None,
                bemerkung: None,
                created_at: "2026-08-16".into(),
                source: None,
            });
            tx.put_instandhaltung(Instandhaltung {
                id: "e1".into(),
                wagen_id: "w1".into(),
                werkstatt_id: None,
                radsatz_id: None,
                datum: Some("2025-12-31".into()),
                leistung: "Bremsprobe".into(),
                betrag_cent: Some(123_456),
                bemerkung: None,
                dedupe_key: "k1".into(),
                source: Provenance {
                    file: "monat.xlsx".into(),
                    sheet: "Tabelle1".into(),
                    row: 2,
                    imported_at: "2026-08-16".into(),
                },
            });
            Ok(())
        })
        .unwrap();
        (folder, db)
    }

    /// A master with a hand-added column and a hand-written bemerkung in it. After an
    /// upsert both have to still be there.
    fn master(path: &Path, existing_key: Option<&str>) {
        let mut book = umya_spreadsheet::new_file();
        book.set_sheet_name(0, SHEET).unwrap();
        let sheet = book.sheet_by_name_mut(SHEET).unwrap();
        for (index, header) in ["Id", "Wagennummer", "Datum", "Eigene Spalte"]
            .iter()
            .enumerate()
        {
            sheet.cell_mut((index as u32 + 1, 1u32)).set_value(*header);
        }
        if let Some(key) = existing_key {
            sheet.cell_mut((1u32, 2u32)).set_value(key);
            sheet.cell_mut((2u32, 2u32)).set_value("alt");
            sheet.cell_mut((4u32, 2u32)).set_value("von Hand");
        }
        umya_spreadsheet::writer::xlsx::write(&book, path).unwrap();
    }

    #[test]
    fn a_missing_master_is_skipped_rather_than_created() {
        let (folder, db) = seeded("master-missing");
        let message = upsert(&db, &folder.join("gibtsnicht.xlsx")).unwrap();
        assert!(message.contains("übersprungen"), "{message}");
    }

    #[test]
    fn a_macro_workbook_is_refused_before_anything_is_read() {
        let (folder, db) = seeded("master-xlsm");
        let error = upsert(&db, &folder.join("master.xlsm")).unwrap_err();
        assert!(error.into_messages()[0].contains("Makros"));
    }

    #[test]
    fn a_new_event_is_appended() {
        let (folder, db) = seeded("master-append");
        let path = folder.join("master.xlsx");
        master(&path, None);
        let message = upsert(&db, &path).unwrap();
        assert!(message.contains("1 angehängt"), "{message}");

        let source = grid::read(&path, Some(SHEET)).unwrap();
        assert_eq!(source.grid.text(1, 2), "e1");
        assert_eq!(source.grid.text(2, 2), "21 81 2471 217-3");
        assert_eq!(source.grid.text(3, 2), "31.12.2025");
    }

    /// The rule the whole module exists for: the app's cells change, everything
    /// else in the row is left exactly as the user left it.
    #[test]
    fn an_existing_row_is_updated_in_place_and_keeps_its_own_columns() {
        let (folder, db) = seeded("master-update");
        let path = folder.join("master.xlsx");
        master(&path, Some("e1"));
        let message = upsert(&db, &path).unwrap();
        assert!(message.contains("1 Zeile(n) überschrieben"), "{message}");

        let source = grid::read(&path, Some(SHEET)).unwrap();
        assert_eq!(source.grid.rows, 2, "no row was added");
        assert_eq!(
            source.grid.text(2, 2),
            "21 81 2471 217-3",
            "ours is updated"
        );
        assert_eq!(source.grid.text(4, 2), "von Hand", "theirs is untouched");
    }

    /// A column the app owns but the file does not have yet is added, with its
    /// header, after the last used one.
    #[test]
    fn a_missing_owned_column_is_appended_with_its_header() {
        let (folder, db) = seeded("master-column");
        let path = folder.join("master.xlsx");
        master(&path, None);
        upsert(&db, &path).unwrap();

        let source = grid::read(&path, Some(SHEET)).unwrap();
        let headers: Vec<String> = (1..=source.grid.cols)
            .map(|index| source.grid.text(index, 1).to_string())
            .collect();
        assert!(headers.contains(&"Betrag".to_string()), "{headers:?}");
        assert!(
            headers.contains(&"Eigene Spalte".to_string()),
            "{headers:?}"
        );
    }

    #[test]
    fn a_backup_is_taken_before_the_first_write() {
        let (folder, db) = seeded("master-backup");
        let path = folder.join("master.xlsx");
        master(&path, None);
        upsert(&db, &path).unwrap();

        let backups: Vec<_> = std::fs::read_dir(folder.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains(".bak."))
            .collect();
        assert_eq!(backups.len(), 1);
    }

    #[test]
    fn a_master_without_the_expected_sheet_says_so_in_german() {
        let (folder, db) = seeded("master-nosheet");
        let path = folder.join("master.xlsx");
        let mut book = umya_spreadsheet::new_file();
        book.set_sheet_name(0, "Irgendwas").unwrap();
        umya_spreadsheet::writer::xlsx::write(&book, &path).unwrap();

        let error = upsert(&db, &path).unwrap_err();
        assert!(error.into_messages()[0].contains(SHEET));
    }
}
