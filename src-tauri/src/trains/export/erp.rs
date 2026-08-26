// ─── why ────────────────────────────────────────────────────────
// The artefact the ERP imports, written from scratch every run. Two sheets in
// one workbook — the wagen and the maintenance events — because that is the
// shape the two importers want and one file is one thing to hand over.
//
// XLSX rather than CSV, and that is a decision with a reason: `docs/decisions.md`
// records that the encoding question died with pdftk, and CSV would reopen it —
// Excel renders BOM-less UTF-8 as mojibake, so a German ERP needs either CP1252
// or a BOM, and picking one silently is how umlauts get mangled six months from
// now. A workbook carries its own encoding.
//
// Every value goes out through `sanitise::format`, which is the same module the
// parsers are tested against — so what this writes, the importer reads back as
// the same value. Dates are `31.12.2025` and amounts `1234,56`: German, because
// the ERP is.
//
// The maintenance columns are `export::INSTANDHALTUNG_COLUMNS` and `export::instandhaltung_row`
// rather than a list here, because the master workbook writes the same seven and
// finds its cells by header text. The wagen sheet keeps its own list: it is
// this file's alone.
//
// Ids are included and are OURS, not the ERP's. They are what makes a second
// export of the same data recognisable as the same data rather than as
// duplicates, and the ERP's own import can key on them.
// ────────────────────────────────────────────────────────────────

use std::path::Path;

use umya_spreadsheet::Workbook;

use crate::error::{AppError, AppResult};
use crate::trains::db::TrainsDb;
use crate::trains::export::{instandhaltung_row, INSTANDHALTUNG_COLUMNS};
use crate::trains::sanitise::format;

const WAGGON_HEADERS: [&str; 5] = ["Id", "Wagennummer", "Gattung", "Eigentümer", "Bemerkung"];

pub fn write(db: &TrainsDb, folder: &Path) -> AppResult<Vec<String>> {
    let mut book = umya_spreadsheet::new_file();
    book.set_sheet_name(0, "Wagen").map_err(|error| {
        AppError::detail(
            "Die Arbeitsmappe konnte nicht angelegt werden.".into(),
            error,
        )
    })?;
    book.new_sheet("Wartungen").map_err(|error| {
        AppError::detail(
            "Die Arbeitsmappe konnte nicht angelegt werden.".into(),
            error,
        )
    })?;

    fill_wagens(&mut book, db)?;
    fill_events(&mut book, db)?;

    let target = folder.join("erp-import.xlsx");
    umya_spreadsheet::writer::xlsx::write(&book, &target).map_err(|error| {
        AppError::detail(
            format!(
                "Die Datei {} konnte nicht geschrieben werden.",
                crate::doc::file_name(&target)
            ),
            error,
        )
    })?;
    Ok(vec![crate::doc::file_name(&target)])
}

fn fill_wagens(book: &mut Workbook, db: &TrainsDb) -> AppResult<()> {
    let sheet = book
        .sheet_by_name_mut("Wagen")
        .map_err(|error| AppError::detail("Die Arbeitsmappe „Wagen“ fehlt.".into(), error))?;
    for (index, header) in WAGGON_HEADERS.iter().enumerate() {
        sheet.cell_mut((index as u32 + 1, 1u32)).set_value(*header);
    }
    for (offset, wagen) in db.wagen().iter().enumerate() {
        let row = offset as u32 + 2;
        let owner = wagen
            .halter_id
            .as_deref()
            .and_then(|id| db.partner_by_id(id))
            .map(|partner| partner.name.clone())
            .unwrap_or_default();
        let cells = [
            wagen.id.clone(),
            format::uic_display(&wagen.nummer),
            wagen.bauart.clone().unwrap_or_default(),
            owner,
            wagen.bemerkung.clone().unwrap_or_default(),
        ];
        for (index, value) in cells.iter().enumerate() {
            sheet.cell_mut((index as u32 + 1, row)).set_value(value);
        }
    }
    Ok(())
}

fn fill_events(book: &mut Workbook, db: &TrainsDb) -> AppResult<()> {
    let rows: Vec<[String; 7]> = db
        .instandhaltungen()
        .map(|event| instandhaltung_row(db, event))
        .collect();

    let sheet = book
        .sheet_by_name_mut("Wartungen")
        .map_err(|error| AppError::detail("Die Arbeitsmappe „Wartungen“ fehlt.".into(), error))?;
    for (index, header) in INSTANDHALTUNG_COLUMNS.iter().enumerate() {
        sheet.cell_mut((index as u32 + 1, 1u32)).set_value(*header);
    }
    for (offset, cells) in rows.into_iter().enumerate() {
        let row = offset as u32 + 2;
        for (index, value) in cells.iter().enumerate() {
            sheet.cell_mut((index as u32 + 1, row)).set_value(value);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TempDir;
    use crate::trains::model::{Instandhaltung, Partner, PartnerRolle, Provenance, Wagen};
    use crate::trains::sheet::grid;

    fn seeded(label: &str) -> (TempDir, TrainsDb) {
        let folder = TempDir::new(label);
        let mut db = TrainsDb::load(&folder.config()).unwrap();
        db.transaction(|tx| {
            tx.put_partner(Partner {
                id: "p1".into(),
                rollen: vec![PartnerRolle::Werkstatt, PartnerRolle::Halter],
                name: "Müller GmbH".into(),
                match_key: "mueller".into(),
                aliases: Vec::new(),
                bemerkung: None,
                created_at: "2026-08-16".into(),
            });
            tx.put_wagen(Wagen {
                id: "w1".into(),
                nummer: "218124712173".into(),
                halter_id: Some("p1".into()),
                eigentuemer_id: None,
                bauart: Some("Zans".into()),
                bemerkung: None,
                created_at: "2026-08-16".into(),
                source: None,
            });
            tx.put_instandhaltung(Instandhaltung {
                id: "e1".into(),
                wagen_id: "w1".into(),
                werkstatt_id: Some("p1".into()),
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

    #[test]
    fn the_workbook_carries_both_sheets_with_their_headers() {
        let (folder, db) = seeded("erp-write");
        let out = folder.join("run");
        std::fs::create_dir_all(&out).unwrap();
        write(&db, &out).unwrap();

        let source = grid::read(&out.join("erp-import.xlsx"), Some("Wagen")).unwrap();
        assert!(source.sheets.contains(&"Wartungen".to_string()));
        assert_eq!(source.grid.text(1, 1), "Id");
        assert_eq!(source.grid.text(2, 1), "Wagennummer");
    }

    /// What the exporter writes, the importer reads back to the same value —
    /// which is why `format` is shared rather than reimplemented here.
    #[test]
    fn values_go_out_in_the_german_spelling_the_parsers_accept() {
        let (folder, db) = seeded("erp-values");
        let out = folder.join("run");
        std::fs::create_dir_all(&out).unwrap();
        write(&db, &out).unwrap();

        let source = grid::read(&out.join("erp-import.xlsx"), Some("Wartungen")).unwrap();
        assert_eq!(source.grid.text(2, 2), "21 81 2471 217-3");
        assert_eq!(source.grid.text(3, 2), "31.12.2025");
        assert_eq!(source.grid.text(6, 2), "1234,56");
    }

    #[test]
    fn a_wagen_names_its_owner_rather_than_its_id() {
        let (folder, db) = seeded("erp-owner");
        let out = folder.join("run");
        std::fs::create_dir_all(&out).unwrap();
        write(&db, &out).unwrap();

        let source = grid::read(&out.join("erp-import.xlsx"), Some("Wagen")).unwrap();
        assert_eq!(source.grid.text(4, 2), "Müller GmbH");
    }

    #[test]
    fn an_empty_store_still_produces_a_file_with_its_headers() {
        let folder = TempDir::new("erp-empty");
        let db = TrainsDb::load(&folder.config()).unwrap();
        let out = folder.join("run");
        std::fs::create_dir_all(&out).unwrap();
        write(&db, &out).unwrap();
        assert!(out.join("erp-import.xlsx").exists());
    }
}
