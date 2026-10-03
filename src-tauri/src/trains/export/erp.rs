// ─── why ────────────────────────────────────────────────────────
// The artefact the ERP imports, written from scratch every run. One workbook —
// the wagen, the maintenance events, the wheelsets and their fittings — because
// one file is one thing to hand over.
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
// The wheelsets go out as two more sheets: `Radsätze`, one row per wheelset with
// the wagen it is fitted to NOW, and `Einbauten`, the FULL fitting history.
// Both questions are asked of this data — "what is on wagen X" and "where has
// wheelset Y been" — and the history answers the first as well (an empty
// `Ausgebaut am`), so it is complete rather than filtered to the open fittings.
// Every sheet goes through `export::fill`, which writes TEXT — see there.
//
// Ids are included and are OURS, not the ERP's. They are what makes a second
// export of the same data recognisable as the same data rather than as
// duplicates, and the ERP's own import can key on them.
// ────────────────────────────────────────────────────────────────

use std::collections::HashMap;
use std::path::Path;

use crate::error::{AppError, AppResult};
use crate::trains::db::TrainsDb;
use crate::trains::export::{fill, instandhaltung_row, INSTANDHALTUNG_COLUMNS};
use crate::trains::sanitise::format;

const WAGGON_HEADERS: [&str; 5] = ["Id", "Wagennummer", "Gattung", "Eigentümer", "Bemerkung"];

const RADSATZ_HEADERS: [&str; 7] = [
    "Id",
    "Radsatznummer",
    "Radsatz-ID",
    "Radsatzwellennummer",
    "Bauart",
    "Eingebaut in Wagen",
    "Bemerkung",
];

const EINBAU_HEADERS: [&str; 6] = [
    "Id",
    "Radsatznummer",
    "Wagennummer",
    "Position",
    "Eingebaut am",
    "Ausgebaut am",
];

pub fn write(db: &TrainsDb, folder: &Path) -> AppResult<Vec<String>> {
    let created = || "Die Arbeitsmappe konnte nicht angelegt werden.".to_string();
    let mut book = umya_spreadsheet::new_file();
    book.set_sheet_name(0, "Wagen")
        .map_err(|error| AppError::detail(created(), error))?;
    fill(
        &mut book.sheet_collection_mut()[0],
        &WAGGON_HEADERS,
        wagen_rows(db),
    );
    let events = db
        .instandhaltungen()
        .map(|event| instandhaltung_row(db, event).to_vec());
    for (name, headers, rows) in [
        (
            "Wartungen",
            &INSTANDHALTUNG_COLUMNS[..],
            events.collect::<Vec<_>>(),
        ),
        ("Radsätze", &RADSATZ_HEADERS[..], radsatz_rows(db)),
        ("Einbauten", &EINBAU_HEADERS[..], einbau_rows(db)),
    ] {
        let sheet = book
            .new_sheet(name)
            .map_err(|error| AppError::detail(created(), error))?;
        fill(sheet, headers, rows);
    }

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

fn wagen_nummer(db: &TrainsDb, id: &str) -> String {
    db.wagen_by_id(id)
        .map(|wagen| format::uic_display(&wagen.nummer))
        .unwrap_or_default()
}

fn date_of(iso: &Option<String>) -> String {
    iso.as_deref().map(format::iso_date).unwrap_or_default()
}

fn wagen_rows(db: &TrainsDb) -> Vec<Vec<String>> {
    db.wagen()
        .into_iter()
        .map(|wagen| {
            let owner = wagen
                .halter_id
                .as_deref()
                .and_then(|id| db.partner_by_id(id))
                .map(|partner| partner.name.clone())
                .unwrap_or_default();
            vec![
                wagen.id,
                format::uic_display(&wagen.nummer),
                wagen.bauart.unwrap_or_default(),
                owner,
                wagen.bemerkung.unwrap_or_default(),
            ]
        })
        .collect()
}

fn radsatz_rows(db: &TrainsDb) -> Vec<Vec<String>> {
    let einbauten = db.einbauten();
    let fitted: HashMap<&str, &str> = einbauten
        .iter()
        .filter(|einbau| einbau.is_open())
        .map(|einbau| (einbau.radsatz_id.as_str(), einbau.wagen_id.as_str()))
        .collect();
    db.radsaetze()
        .into_iter()
        .map(|radsatz| {
            let wagen = fitted
                .get(radsatz.id.as_str())
                .map(|wagen_id| wagen_nummer(db, wagen_id))
                .unwrap_or_default();
            vec![
                radsatz.id,
                radsatz.nummer,
                radsatz.system_id.unwrap_or_default(),
                radsatz.wellennummer.unwrap_or_default(),
                radsatz.bauart.unwrap_or_default(),
                wagen,
                radsatz.bemerkung.unwrap_or_default(),
            ]
        })
        .collect()
}

fn einbau_rows(db: &TrainsDb) -> Vec<Vec<String>> {
    db.einbauten()
        .into_iter()
        .map(|einbau| {
            vec![
                einbau.id.clone(),
                db.radsatz(&einbau.radsatz_id)
                    .map(|radsatz| radsatz.nummer.clone())
                    .unwrap_or_default(),
                wagen_nummer(db, &einbau.wagen_id),
                einbau.position.clone().unwrap_or_default(),
                date_of(&einbau.eingebaut_am),
                date_of(&einbau.ausgebaut_am),
            ]
        })
        .collect()
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

    fn fitted(db: &mut TrainsDb) {
        use crate::trains::model::{Einbau, Radsatz};
        let provenance = Provenance {
            file: "monitoring.xlsx".into(),
            sheet: "Tabelle1".into(),
            row: 2,
            imported_at: "2026-10-03".into(),
        };
        db.transaction(|tx| {
            tx.put_radsatz(Radsatz {
                id: "r1".into(),
                nummer: "0123456".into(),
                match_key: "0123456".into(),
                aliases: Vec::new(),
                wellennummer: None,
                system_id: Some("180043025".into()),
                bauart: None,
                bemerkung: None,
                created_at: "2026-10-03".into(),
                source: None,
            });
            tx.put_einbau(Einbau {
                id: "a1".into(),
                radsatz_id: "r1".into(),
                wagen_id: "w1".into(),
                position: Some("1".into()),
                eingebaut_am: Some("2024-01-02".into()),
                ausgebaut_am: Some("2025-03-04".into()),
                source: provenance.clone(),
            });
            tx.put_einbau(Einbau {
                id: "a2".into(),
                radsatz_id: "r1".into(),
                wagen_id: "w1".into(),
                position: Some("3".into()),
                eingebaut_am: Some("2025-03-05".into()),
                ausgebaut_am: None,
                source: provenance,
            });
            Ok(())
        })
        .unwrap();
    }

    /// Both questions — what is fitted now, where has it been — from the file.
    #[test]
    fn wheelsets_go_out_with_their_whole_fitting_history() {
        let (folder, mut db) = seeded("erp-radsaetze");
        fitted(&mut db);
        let out = folder.join("run");
        std::fs::create_dir_all(&out).unwrap();
        write(&db, &out).unwrap();
        let file = out.join("erp-import.xlsx");

        let radsaetze = grid::read(&file, Some("Radsätze")).unwrap().grid;
        assert_eq!(
            radsaetze.text(2, 2),
            "0123456",
            "digits stay text, leading zero kept"
        );
        assert_eq!(radsaetze.text(3, 2), "180043025");
        assert_eq!(radsaetze.text(6, 2), "21 81 2471 217-3");

        let einbauten = grid::read(&file, Some("Einbauten")).unwrap().grid;
        assert_eq!(einbauten.rows, 3, "header plus both fittings");
        assert_eq!(einbauten.text(5, 2), "02.01.2024");
        assert_eq!(einbauten.text(6, 2), "04.03.2025");
        assert_eq!(
            einbauten.text(6, 3),
            "",
            "the open fitting has no removal date"
        );
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
