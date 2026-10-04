// ─── why ────────────────────────────────────────────────────────
// Every sheet of the master is bound without a click. A binding is DERIVED
// from the sheet's header row: the kind that claims it (`kinds::recognise`),
// and as refresh source the one template whose mapped headers it carries
// (`recognise::matching`, and only when exactly one does). A sheet no kind
// claims is bound view-only — it still gets a sheet view, it is just not
// imported. What the user changes by hand stays: `sync` only ADDS bindings for
// sheets not bound yet, and `auto` marks the ones nobody touched.
//
// ONLY MAPPING AND IMPORT READ THE FILE. `sync` runs when a file is picked, on
// „Standardzuordnung“, and when an import starts — never for the settings
// page, which answers from what the last read stored (`MasterSettings.scan`).
// Reading a header means deserialising the whole sheet — umya has no partial
// read — and five of the customer's sheets are filled down to row 1,048,576
// (up to 72 MB of XML each). Even on those three occasions the scan is skipped
// when the file's modification time matches the stored one.
//
// A DIFFERENT file starts over: bindings belong to the workbook they were made
// for, so `reset` replaces them all with the defaults, and so does picking a new
// path. The same file read again keeps every binding the user set.
// ────────────────────────────────────────────────────────────────

use std::path::Path;

use super::kinds;
use crate::error::AppResult;
use crate::trains::db::TrainsDb;
use crate::trains::model::{
    ColumnBinding, FieldKind, ImportTemplate, MasterBinding, MasterMode, MasterScan, MasterSheet,
};
use crate::trains::recognise;
use crate::trains::sheet::grid;

pub fn sync(db: &mut TrainsDb, reset: bool) -> AppResult<()> {
    let mut settings = db.master().clone();
    let Some(path) = settings.file.as_deref().map(Path::new) else {
        return Ok(());
    };
    if !path.is_file() {
        return Ok(());
    }
    let modified = crate::doc::file_mtime_ms(path)? as u64;
    let fresh = settings
        .scan
        .as_ref()
        .is_some_and(|scan| scan.modified == modified);
    if fresh && !reset {
        return Ok(());
    }

    let scan = MasterScan {
        modified,
        sheets: scanned(path)?,
    };
    if reset {
        settings.bindings.clear();
    }
    let templates = db.templates();
    for sheet in &scan.sheets {
        if settings
            .bindings
            .iter()
            .all(|bound| bound.sheet != sheet.name)
        {
            settings.bindings.push(default(sheet, &templates));
        }
    }
    settings.scan = Some(scan);
    db.save_master(settings)
}

pub fn default(sheet: &MasterSheet, templates: &[ImportTemplate]) -> MasterBinding {
    MasterBinding {
        sheet: sheet.name.clone(),
        template_id: refresh_source(&sheet.headers, templates).unwrap_or_default(),
        kind: kinds::recognise(&sheet.headers),
        mode: MasterMode::Snapshot,
        key: None,
        aliases: Vec::new(),
        auto: true,
    }
}

fn refresh_source(headers: &[String], templates: &[ImportTemplate]) -> Option<String> {
    let columns: Vec<ColumnBinding> = headers
        .iter()
        .enumerate()
        .map(|(position, header)| ColumnBinding {
            header: header.clone(),
            index: position as u32 + 1,
            field: FieldKind::Ignorieren,
            decimal: None,
            date_order: None,
        })
        .collect();
    match recognise::matching(templates, &columns).as_slice() {
        [only] => Some(only.id.clone()),
        _ => None,
    }
}

fn scanned(path: &Path) -> AppResult<Vec<MasterSheet>> {
    Ok(grid::heads(path)?
        .into_iter()
        .map(|grid| MasterSheet {
            headers: (1..=grid.cols)
                .map(|col| grid.text(col, 1).trim().to_string())
                .filter(|text| !text.is_empty())
                .collect(),
            name: grid.sheet,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{workbook, TempDir};
    use crate::trains::model::{MasterSettings, SheetKind};

    fn master(folder: &TempDir) -> TrainsDb {
        let file = workbook(
            folder,
            "Master.xlsx",
            &[
                ("Übersicht", &[&["Wagennummer", "Bemerkungen"]]),
                (
                    "Einbau",
                    &[&["an_wagen", "radsatzid", "radsatz", "einbau_am", "pos"]],
                ),
                (
                    "Bestand",
                    &[&["radsatzid", "radsatz", "einbau_am", "an_wagen", "halter"]],
                ),
                ("Schilder", &[&["", "Wagennummer", "Schild wurde entfernt"]]),
                ("Telematik", &[&["Asset", "Anbaudatum"]]),
                ("Kontakte", &[&["Standort", "Ansprechpartner"]]),
            ],
        );
        let mut db = TrainsDb::load(&folder.config()).unwrap();
        db.save_master(MasterSettings {
            file: Some(file.to_string_lossy().into_owned()),
            ..MasterSettings::default()
        })
        .unwrap();
        db
    }

    #[test]
    fn every_sheet_is_bound_by_its_headers_in_workbook_order() {
        let folder = TempDir::new("bindings-default");
        let mut db = master(&folder);
        sync(&mut db, false).unwrap();

        let kinds: Vec<(&str, Option<SheetKind>)> = db
            .master()
            .bindings
            .iter()
            .map(|binding| (binding.sheet.as_str(), binding.kind))
            .collect();
        assert_eq!(
            kinds,
            [
                ("Übersicht", Some(SheetKind::Wagenliste)),
                ("Einbau", Some(SheetKind::RadsatzEinbau)),
                ("Bestand", Some(SheetKind::RadsatzBestand)),
                ("Schilder", Some(SheetKind::Wagenliste)),
                ("Telematik", Some(SheetKind::Wagenliste)),
                ("Kontakte", None),
            ]
        );
        assert!(db.master().bindings.iter().all(|binding| binding.auto));
    }

    #[test]
    fn a_shipped_template_whose_headers_the_sheet_carries_becomes_its_refresh_source() {
        let folder = TempDir::new("bindings-refresh");
        let mut db = master(&folder);
        sync(&mut db, false).unwrap();
        let telematik = &db.master().bindings[4];
        assert_eq!(telematik.template_id, "builtin:telematik");
        assert_eq!(db.master().bindings[5].template_id, "", "view-only");
    }

    #[test]
    fn a_hand_set_binding_survives_and_reset_brings_the_defaults_back() {
        let folder = TempDir::new("bindings-keep");
        let mut db = master(&folder);
        sync(&mut db, false).unwrap();
        let mut settings = db.master().clone();
        settings.bindings[0].kind = None;
        settings.bindings[0].auto = false;
        settings.scan = None;
        db.save_master(settings).unwrap();

        sync(&mut db, false).unwrap();
        assert_eq!(db.master().bindings[0].kind, None, "rescanned, but kept");
        assert_eq!(db.master().bindings.len(), 6, "nothing bound twice");

        sync(&mut db, true).unwrap();
        assert_eq!(db.master().bindings[0].kind, Some(SheetKind::Wagenliste));
    }

    #[test]
    fn an_unchanged_file_is_not_scanned_again() {
        let folder = TempDir::new("bindings-cache");
        let mut db = master(&folder);
        sync(&mut db, false).unwrap();
        let mut settings = db.master().clone();
        settings.scan.as_mut().unwrap().sheets.clear();
        db.save_master(settings).unwrap();

        sync(&mut db, false).unwrap();
        assert!(
            db.master().scan.as_ref().unwrap().sheets.is_empty(),
            "the cache was trusted, the workbook not opened"
        );
    }
}
