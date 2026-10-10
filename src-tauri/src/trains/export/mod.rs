// ─── why ────────────────────────────────────────────────────────
// „Export erstellen“: the master overview, a workbook built fresh from the
// Schattensystem every run — an „Übersicht“ that computes, and one data sheet
// per entity it computes from. It is OURS and disposable, and it never touches
// the customer's own master: that file is the user's, with formulas, colours
// and columns this program gives no meaning to, and `trains/master/` writes it
// by a different rule. This replaced the ERP workbook, which no ERP ever read.
//
// The file is named `Master-Übersicht.xlsx` in a dated folder, and is written
// through `doc::write_book`, temp + rename, so a failed write leaves no half
// file under the real name.
//
// `fill` stays: it writes a header row and rows of TEXT (`set_value_string`) for
// the cleaned copies `clean::write` produces, where a wheelset number made of
// digits must not become a number Excel strips the leading zeros from. The
// overview needs typed cells instead, and those live in `layout`.
//
// `Run { messages, failed }` mirrors `filler::export` so the report reads the
// same, and `Run::report` is on the run rather than in the command because a
// `#[tauri::command]` cannot be exercised by `cargo test`.
// ────────────────────────────────────────────────────────────────

mod layout;
mod overview;
mod rows;
mod sheets;

use std::path::Path;

use umya_spreadsheet::Worksheet;

use super::db::TrainsDb;
use super::sanitise::date::Date;
use crate::error::{AppError, AppResult};
use crate::model::ClientReport;

pub const FILE: &str = "Master-Übersicht.xlsx";

pub fn fill(sheet: &mut Worksheet, headers: &[&str], rows: Vec<Vec<String>>) {
    for (index, header) in headers.iter().enumerate() {
        sheet
            .cell_mut((index as u32 + 1, 1u32))
            .set_value_string(*header);
    }
    for (offset, cells) in rows.into_iter().enumerate() {
        let row = offset as u32 + 2;
        for (index, value) in cells.into_iter().enumerate() {
            sheet
                .cell_mut((index as u32 + 1, row))
                .set_value_string(value);
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Run {
    pub messages: Vec<String>,
    pub failed: usize,
}

impl Run {
    pub fn report(self, folder: &Path) -> ClientReport {
        ClientReport {
            headline: if self.failed == 0 {
                "Export wurde erfolgreich erstellt".into()
            } else {
                "Export wurde mit Fehlern erstellt".into()
            },
            messages: self.messages,
            message_folder: Some(folder.to_string_lossy().into_owned()),
        }
    }
}

pub fn run(db: &TrainsDb, folder: &Path) -> AppResult<Run> {
    let mut run = Run::default();

    std::fs::create_dir_all(folder).map_err(|error| AppError::io(folder, error))?;
    run.messages
        .push(format!("Ordner wurde erstellt: {}", folder.display()));

    match write(db, folder, super::clock::today()) {
        Ok(file) => run.messages.push(format!("Datei wurde erstellt: {file}")),
        Err(error) => {
            run.failed += 1;
            run.messages.extend(error.into_messages());
        }
    }

    run.messages.push(if run.failed == 0 {
        "Alle Dateien wurden erfolgreich erstellt.".into()
    } else {
        format!("{} Schritt(e) sind fehlgeschlagen.", run.failed)
    });
    Ok(run)
}

pub fn write(db: &TrainsDb, folder: &Path, today: Date) -> AppResult<String> {
    let created = || "Die Arbeitsmappe konnte nicht angelegt werden.".to_string();
    let mut book = umya_spreadsheet::new_file();
    book.set_sheet_name(0, overview::NAME)
        .map_err(|error| AppError::detail(created(), error))?;
    overview::write(&mut book.sheet_collection_mut()[0], db, today);

    for (sheet, rows) in sheets::all(db) {
        let target = book
            .new_sheet(sheet.name)
            .map_err(|error| AppError::detail(created(), error))?;
        layout::table(target, 1, sheet.columns, &rows, "FFD9D9D9");
        layout::freeze(target, 1, 0);
    }

    let target = folder.join(FILE);
    crate::doc::write_book(
        &book,
        &target,
        format!(
            "Die Datei {} konnte nicht geschrieben werden.",
            crate::doc::file_name(&target)
        ),
    )?;
    Ok(crate::doc::file_name(&target))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TempDir;
    use crate::trains::model::{
        Einbau, Partner, PartnerRolle, Provenance, Pruefung, Radsatz, Schadensmeldung,
        TelematikMeldung, TrainsSettings, UicStyle, Wagen,
    };
    use crate::trains::sheet::grid;
    use umya_spreadsheet::{reader, Workbook};

    const TODAY: Date = Date {
        year: 2026,
        month: 10,
        day: 10,
    };

    fn source() -> Provenance {
        Provenance {
            file: "master.xlsx".into(),
            sheet: "Tabelle1".into(),
            row: 2,
            imported_at: "2026-10-10".into(),
        }
    }

    fn seeded(label: &str) -> (TempDir, TrainsDb) {
        let folder = TempDir::new(label);
        let mut db = TrainsDb::load(&folder.config()).unwrap();
        db.transaction(|tx| {
            tx.put_partner(Partner {
                id: "p1".into(),
                rollen: vec![PartnerRolle::Halter],
                name: "Wagenmut AG".into(),
                match_key: "wagenmut".into(),
                aliases: Vec::new(),
                bemerkung: None,
                created_at: "2026-10-10".into(),
            });
            for (id, nummer) in [("w2", "338506590011"), ("w1", "218124712173")] {
                tx.put_wagen(Wagen {
                    id: id.into(),
                    nummer: nummer.into(),
                    halter_id: Some("p1".into()),
                    eigentuemer_id: None,
                    bauart: Some("Tanpps".into()),
                    bemerkung: None,
                    created_at: "2026-10-10".into(),
                    source: None,
                });
            }
            tx.put_radsatz(Radsatz {
                id: "r1".into(),
                nummer: "0123456".into(),
                match_key: "0123456".into(),
                aliases: Vec::new(),
                wellennummer: None,
                system_id: Some("180043025".into()),
                bauart: None,
                bemerkung: None,
                created_at: "2026-10-10".into(),
                source: None,
            });
            tx.put_einbau(Einbau {
                id: "e1".into(),
                radsatz_id: "r1".into(),
                wagen_id: "w1".into(),
                position: Some("3".into()),
                eingebaut_am: Some("2023-03-15".into()),
                ausgebaut_am: None,
                source: source(),
            });
            let zustand = tx.zustand_mut();
            zustand.meldungen.push(TelematikMeldung {
                id: "m1".into(),
                wagen_id: "w1".into(),
                geraet_id: None,
                zeitpunkt: "2023-03-15T12:00:00".into(),
                stadt: Some("Neuhof".into()),
                land: None,
                standort: None,
                laufleistung_km: None,
                energie_prozent: None,
                bewegung: None,
                source: source(),
            });
            zustand.schaeden.push(Schadensmeldung {
                id: "s1".into(),
                wagen_id: "w1".into(),
                gemeldet_am: Some("2026-09-29".into()),
                gemeldet_von: None,
                schadcode: Some("3.3.4".into()),
                notiz: None,
                ausgesetzt: Some(true),
                beladen: None,
                ausfuehrender: None,
                geplant_am: None,
                aktion: None,
                erledigt_am: None,
                source: source(),
            });
            zustand.pruefungen.push(Pruefung {
                id: "pr1".into(),
                wagen_id: "w1".into(),
                art: Some("P8".into()),
                faellig_am: Some("2026-03-31".into()),
                geplant_am: None,
                durchgefuehrt_am: None,
                status: None,
                bestellnummer: None,
                source: source(),
            });
            Ok(())
        })
        .unwrap();
        (folder, db)
    }

    fn written(folder: &TempDir, db: &TrainsDb) -> std::path::PathBuf {
        let out = folder.join("run");
        std::fs::create_dir_all(&out).unwrap();
        write(db, &out, TODAY).unwrap();
        out.join(FILE)
    }

    fn book(file: &Path) -> Workbook {
        reader::xlsx::read(file).unwrap()
    }

    #[test]
    fn the_overview_comes_first_then_one_sheet_per_entity() {
        let (folder, db) = seeded("export-sheets");
        let source = grid::read(&written(&folder, &db), None).unwrap();
        assert_eq!(
            source.sheets,
            [
                "Übersicht",
                "Wagen",
                "Radsätze",
                "Einbauten",
                "Instandhaltungen",
                "Telematik",
                "Schäden",
                "Aufträge",
                "Prüfungen"
            ]
        );
    }

    // The overview's lookups key on numbers, like the customer's pasted exports.
    #[test]
    fn a_compact_wagennummer_is_written_as_a_number_on_every_sheet() {
        let (folder, db) = seeded("export-key");
        let file = written(&folder, &db);
        for sheet in ["Wagen", "Einbauten"] {
            let grid = grid::read(&file, Some(sheet)).unwrap().grid;
            assert_eq!(
                grid.cell(1, 2).unwrap().number,
                Some(218124712173.0),
                "{sheet}"
            );
        }
    }

    #[test]
    fn a_grouped_wagennummer_is_text_in_the_overview_and_the_data_alike() {
        let (folder, mut db) = seeded("export-grouped");
        db.save_settings(TrainsSettings {
            wagennummer: UicStyle::Grouped,
        })
        .unwrap();
        let file = written(&folder, &db);
        let overview = grid::read(&file, Some("Übersicht")).unwrap().grid;
        let wagen = grid::read(&file, Some("Wagen")).unwrap().grid;
        assert_eq!(overview.text(1, 7), "21 81 2471 217-3");
        assert_eq!(wagen.text(1, 2), "21 81 2471 217-3");
        assert_eq!(wagen.cell(1, 2).unwrap().number, None);
    }

    #[test]
    fn dates_are_serials_with_a_date_format_and_numbers_made_of_digits_stay_text() {
        let (folder, db) = seeded("export-typed");
        let file = written(&folder, &db);
        let einbauten = grid::read(&file, Some("Einbauten")).unwrap().grid;
        let cell = einbauten.cell(4, 2).unwrap();
        assert_eq!(cell.number, Some(45000.0));
        assert!(cell.date_format);
        assert_eq!(einbauten.text(2, 2), "0123456", "leading zero kept");

        let schaeden = grid::read(&file, Some("Schäden")).unwrap().grid;
        assert_eq!(schaeden.text(4, 2), "3.3.4", "a Schadcode is no date");
        assert_eq!(schaeden.text(6, 2), "Ja");
    }

    #[test]
    fn a_telematik_meldung_keeps_its_time() {
        let (folder, db) = seeded("export-moment");
        let telematik = grid::read(&written(&folder, &db), Some("Telematik"))
            .unwrap()
            .grid;
        assert_eq!(telematik.cell(4, 2).unwrap().number, Some(45000.5));
        assert_eq!(telematik.text(5, 2), "Neuhof");
    }

    #[test]
    fn the_overview_has_one_row_per_wagen_sorted_by_number() {
        let (folder, db) = seeded("export-rows");
        let overview = grid::read(&written(&folder, &db), Some("Übersicht"))
            .unwrap()
            .grid;
        assert_eq!(overview.text(1, 6), "Wagennummer");
        assert_eq!(overview.cell(1, 7).unwrap().number, Some(218124712173.0));
        assert_eq!(overview.cell(1, 8).unwrap().number, Some(338506590011.0));
    }

    // Every formula names its column by LETTER; the letter has to land on the
    // header the formula means, or the overview answers from the wrong column.
    #[test]
    fn each_formula_reads_the_column_it_means() {
        let (folder, db) = seeded("export-formulas");
        let book = book(&written(&folder, &db));
        let overview = book.sheet_by_name("Übersicht").unwrap();
        let header = |sheet: &str, col: &str| {
            book.sheet_by_name(sheet)
                .unwrap()
                .cell(format!("{col}1").as_str())
                .map(|cell| cell.value().to_string())
                .unwrap_or_default()
        };

        let p8 = overview.cell("F7").unwrap().formula().to_string();
        assert_eq!(
            p8,
            "_xlfn.MINIFS('Prüfungen'!$C:$C,'Prüfungen'!$A:$A,$A7,'Prüfungen'!$B:$B,\"P8\",'Prüfungen'!$E:$E,\"\")"
        );
        assert_eq!(header("Prüfungen", "C"), "fällig am");
        assert_eq!(header("Prüfungen", "E"), "durchgeführt am");

        let schaeden = overview.cell("H7").unwrap().formula().to_string();
        assert!(schaeden.contains("'Schäden'!$K:$K"), "{schaeden}");
        assert_eq!(header("Schäden", "K"), "erledigt am");

        let standort = overview.cell("C8").unwrap().formula().to_string();
        assert_eq!(
            standort,
            "IFERROR(VLOOKUP($A8,'Telematik'!$A:$E,5,FALSE)&\"\",\"\")"
        );
        assert_eq!(header("Telematik", "E"), "Stadt");
    }

    #[test]
    fn the_hand_columns_are_there_and_empty() {
        let (folder, db) = seeded("export-hand");
        let overview = grid::read(&written(&folder, &db), Some("Übersicht"))
            .unwrap()
            .grid;
        assert_eq!(overview.text(15, 6), "Bemerkungen / notwendige Aktion");
        assert_eq!(overview.text(19, 6), "Notiz");
        assert_eq!(overview.text(15, 7), "");
    }

    #[test]
    fn overdue_and_silent_are_coloured_by_rule_not_by_hand() {
        let (folder, db) = seeded("export-colours");
        let book = book(&written(&folder, &db));
        let overview = book.sheet_by_name("Übersicht").unwrap();
        let ranges: Vec<String> = overview
            .conditional_formatting_collection()
            .iter()
            .map(|block| block.sequence_of_references().get_sqref())
            .collect();
        assert!(ranges.contains(&"D7:D8".to_string()), "{ranges:?}");
        assert!(ranges.contains(&"F7:F8".to_string()), "{ranges:?}");
        assert!(overview.auto_filter().is_some());
    }

    #[test]
    fn an_empty_store_still_produces_the_whole_workbook() {
        let folder = TempDir::new("export-empty");
        let db = TrainsDb::load(&folder.config()).unwrap();
        let source = grid::read(&written(&folder, &db), Some("Prüfungen")).unwrap();
        assert_eq!(source.sheets.len(), 9);
        assert_eq!(source.grid.text(1, 1), "Wagennummer");
    }

    #[test]
    fn the_report_names_the_folder_so_the_dialog_can_open_it() {
        let run = Run {
            messages: vec!["Datei wurde erstellt: Master-Übersicht.xlsx".into()],
            failed: 0,
        };
        let report = run.report(Path::new("/out/trains-2026-10-10"));
        assert_eq!(report.headline, "Export wurde erfolgreich erstellt");
        assert_eq!(
            report.message_folder.as_deref(),
            Some("/out/trains-2026-10-10")
        );
    }
}
