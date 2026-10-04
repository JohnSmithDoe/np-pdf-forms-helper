// ─── why ────────────────────────────────────────────────────────
// Every sheet of the master is bound without a click. A binding is DERIVED
// from the sheet's header row: the kind that claims it (`kinds::recognise`),
// and as export source the one template whose mapped headers it carries
// (`recognise::matching`, and only when exactly one does). A sheet no kind
// claims is bound view-only — it still gets a sheet view, it is just not
// imported. What the user changes by hand stays: `sync` only ADDS bindings for
// sheets not bound yet, and `auto` marks the ones nobody touched.
//
// ONLY MAPPING, IMPORT, EXPORT AND A SHEET VIEW READ THE FILE. `sync` runs when
// a version is taken over or written, on „Standardzuordnung“, when an import
// starts, when the export wizard opens, when a sheet view opens, and for the
// settings page — where it is one `stat` unless the version is new.
// Reading a header means deserialising the whole sheet — umya has no partial
// read — and five of the customer's sheets are filled down to row 1,048,576
// (up to 72 MB of XML each). So the read that takes the headers also writes the
// trimmed read copy every later reader opens (`prepare`), and even on those
// occasions it is skipped while the file's modification time matches the
// stored one and the copy is still there. „Standardzuordnung“ on an unchanged
// file rebinds from the stored headers without opening the workbook at all.
//
// THE FILE IS THE CLIENT MASTER, never one picked here: `follow` points
// `MasterSettings.file` at the cleaned copy of `MasterFile.versions[0]` and is
// the only writer of that field. No version, no master. A new version is the
// same workbook again, so the bindings stay — matched by sheet name, and a sheet
// it gained is bound by default — while the scan belongs to the old path and
// goes, which is what makes the next `sync` read the new one.
// ────────────────────────────────────────────────────────────────

use std::path::Path;

use super::{kinds, prepare};
use crate::error::AppResult;
use crate::trains::db::TrainsDb;
use crate::trains::model::{
    ColumnBinding, FieldKind, ImportTemplate, MasterBinding, MasterMode, MasterScan, MasterSheet,
};
use crate::trains::recognise;

pub fn follow(db: &mut TrainsDb) -> AppResult<()> {
    let current = db
        .master_file()
        .versions
        .first()
        .map(|version| version.cleaned.clone());
    if db.master().file == current {
        return Ok(());
    }
    let mut settings = db.master().clone();
    settings.file = current;
    settings.scan = None;
    db.save_master(settings)
}

pub fn sync(db: &mut TrainsDb, reset: bool) -> AppResult<()> {
    follow(db)?;
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
        .is_some_and(|scan| scan.modified == modified)
        && prepare::copy_path(db, path).is_file();
    if fresh && !reset {
        return Ok(());
    }

    let scan = match settings.scan.take() {
        Some(scan) if fresh => scan,
        _ => MasterScan {
            modified,
            sheets: prepare::copy(db, path)?,
        },
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
        template_id: source_template(&sheet.headers, templates).unwrap_or_default(),
        kind: kinds::recognise(&sheet.headers),
        mode: MasterMode::Snapshot,
        key: None,
        aliases: Vec::new(),
        ignored: Vec::new(),
        auto: true,
    }
}

fn source_template(headers: &[String], templates: &[ImportTemplate]) -> Option<String> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{workbook, TempDir};
    use crate::trains::model::SheetKind;

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
        crate::testing::client_master(&mut db, &file);
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
    fn a_shipped_template_whose_headers_the_sheet_carries_becomes_its_source_template() {
        let folder = TempDir::new("bindings-source");
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

    #[test]
    fn standardzuordnung_on_an_unchanged_file_rebinds_from_the_stored_headers() {
        let folder = TempDir::new("bindings-reset-cached");
        let mut db = master(&folder);
        sync(&mut db, false).unwrap();
        let mut settings = db.master().clone();
        settings.scan.as_mut().unwrap().sheets.truncate(1);
        db.save_master(settings).unwrap();

        sync(&mut db, true).unwrap();
        assert_eq!(db.master().bindings.len(), 1, "the workbook was not opened");
    }

    #[test]
    fn a_missing_read_copy_is_made_again_although_the_file_is_unchanged() {
        let folder = TempDir::new("bindings-copy-gone");
        let mut db = master(&folder);
        sync(&mut db, false).unwrap();
        let path = prepare::existing(&db).unwrap();
        std::fs::remove_file(&path).unwrap();

        sync(&mut db, false).unwrap();
        assert!(path.is_file());
        assert_eq!(db.master().scan.as_ref().unwrap().sheets.len(), 6);
    }
}
