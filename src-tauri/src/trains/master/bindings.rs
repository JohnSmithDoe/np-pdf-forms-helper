// ─── why ────────────────────────────────────────────────────────
// Every sheet of the master is bound without a click. A binding is DERIVED
// from the sheet's header row: the kind that claims it (`kinds::recognise`),
// and as export source the one template whose mapped headers it carries
// (`recognise::matching`, and only when exactly one does). A sheet no kind
// claims is bound view-only — it still gets a sheet view, it is just not
// imported. What the user changes by hand stays: `auto` marks the bindings
// nobody touched, and only those are re-derived — on EVERY `sync`, from the
// stored header row, so a better default in a program update reaches an
// existing install and not only a fresh one. A sheet not bound yet is added;
// a binding with `auto: false` is never rewritten. The file is written only
// when a derivation actually changed.
//
// A shipped template may know its master sheet better (`builtin::master_hint`):
// headers the master spells differently are read back to the template's
// spelling for recognition, and the binding starts with that template's key
// and those respellings as aliases — so the Radsatz-Monitoring sheet is the
// template's, keyed by `Radsatz ID` ← `RadsatzID`, on a fresh install without a
// click. A user copy takes the hint of the built-in it came from.
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
use crate::trains::builtin;
use crate::trains::db::TrainsDb;
use crate::trains::model::{
    ColumnBinding, FieldKind, ImportTemplate, MasterAlias, MasterBinding, MasterScan,
    MasterSettings, MasterSheet,
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
        return rederive(db, settings);
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
    settings.scan = Some(scan);
    let changed = derive(&mut settings, &db.templates());
    if changed || !fresh || reset {
        db.save_master(settings)?;
    }
    Ok(())
}

fn rederive(db: &mut TrainsDb, mut settings: MasterSettings) -> AppResult<()> {
    if derive(&mut settings, &db.templates()) {
        db.save_master(settings)?;
    }
    Ok(())
}

fn derive(settings: &mut MasterSettings, templates: &[ImportTemplate]) -> bool {
    let Some(scan) = settings.scan.as_ref() else {
        return false;
    };
    let mut changed = false;
    for sheet in &scan.sheets {
        let derived = default(sheet, templates);
        match settings
            .bindings
            .iter_mut()
            .find(|bound| bound.sheet == sheet.name)
        {
            Some(bound) if bound.auto && *bound != derived => {
                *bound = derived;
                changed = true;
            }
            Some(_) => {}
            None => {
                settings.bindings.push(derived);
                changed = true;
            }
        }
    }
    changed
}

pub fn default(sheet: &MasterSheet, templates: &[ImportTemplate]) -> MasterBinding {
    let template_id = source_template(&sheet.headers, templates)
        .or_else(|| respelled_template(&sheet.headers, templates))
        .unwrap_or_default();
    let mut binding = MasterBinding {
        sheet: sheet.name.clone(),
        template_id,
        kind: kinds::recognise(&sheet.headers),
        key: None,
        aliases: Vec::new(),
        ignored: Vec::new(),
        remove_for: Vec::new(),
        auto: true,
    };
    if let Some(hint) = hint(&binding.template_id, templates) {
        let has = |name: &str| sheet.headers.iter().any(|own| own.trim() == name);
        binding.aliases = hint
            .renamed
            .iter()
            .filter(|(_, master)| has(master))
            .map(|(source, master)| MasterAlias {
                master: master.to_string(),
                source: source.to_string(),
            })
            .collect();
        let key = binding
            .aliases
            .iter()
            .find(|alias| alias.source == hint.key)
            .map_or(hint.key, |alias| alias.master.as_str());
        binding.key = has(key).then(|| key.to_string());
    }
    binding
}

fn hint(template_id: &str, templates: &[ImportTemplate]) -> Option<builtin::MasterHint> {
    let template = templates
        .iter()
        .find(|template| template.id == template_id)?;
    builtin::master_hint(template.origin.as_deref().unwrap_or(&template.id))
}

fn respelled_template(headers: &[String], templates: &[ImportTemplate]) -> Option<String> {
    let found: Vec<&ImportTemplate> = templates
        .iter()
        .filter(|template| {
            let Some(hint) = hint(&template.id, templates) else {
                return false;
            };
            let respelled: Vec<String> = headers
                .iter()
                .map(|header| {
                    hint.renamed
                        .iter()
                        .find(|(_, master)| *master == header.trim())
                        .map_or_else(|| header.clone(), |(source, _)| source.to_string())
                })
                .collect();
            source_template(&respelled, std::slice::from_ref(*template)).is_some()
        })
        .collect();
    match found.as_slice() {
        [only] => Some(only.id.clone()),
        _ => None,
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

    fn bound(folder: &TempDir, headers: &[&str]) -> MasterBinding {
        let file = workbook(folder, "Master.xlsx", &[("Monitoring", &[headers])]);
        let mut db = TrainsDb::load(&folder.config()).unwrap();
        crate::testing::client_master(&mut db, &file);
        sync(&mut db, false).unwrap();
        db.master().bindings[0].clone()
    }

    // The master spells the snapshot's `RadsatzID` as `Radsatz ID`; the sheet is
    // still the template's, keyed by that column — one row per Radsatz, so the
    // Wagennummer could never be the key.
    #[test]
    fn a_respelled_header_still_binds_the_template_with_its_key_and_alias() {
        let folder = TempDir::new("bindings-hint");
        let binding = bound(
            &folder,
            &[
                "Wagennr.",
                "gemeldet an",
                "Radsatz ID",
                "Radsatznummer",
                "Einbaudatum NACH letzter IS2/3",
            ],
        );
        assert_eq!(binding.template_id, "builtin:radsatz-monitoring");
        assert_eq!(binding.key.as_deref(), Some("Radsatz ID"));
        assert_eq!(
            binding.aliases,
            [MasterAlias {
                master: "Radsatz ID".into(),
                source: "RadsatzID".into(),
            }]
        );
    }

    #[test]
    fn the_template_spelling_binds_with_the_key_and_no_alias() {
        let folder = TempDir::new("bindings-hint-plain");
        let binding = bound(
            &folder,
            &[
                "Wagennr.",
                "RadsatzID",
                "Radsatznummer",
                "Einbaudatum NACH letzter IS2/3",
            ],
        );
        assert_eq!(binding.template_id, "builtin:radsatz-monitoring");
        assert_eq!(binding.key.as_deref(), Some("RadsatzID"));
        assert!(binding.aliases.is_empty());
    }

    // A binding derived by an older program — no template, no key — is derived
    // again on the next sync, without the file being rescanned; one the user
    // changed is left exactly as it is.
    #[test]
    fn an_untouched_binding_is_derived_again_and_a_hand_set_one_is_kept() {
        let folder = TempDir::new("bindings-rederive");
        let file = workbook(
            &folder,
            "Master.xlsx",
            &[
                (
                    "Monitoring",
                    &[&[
                        "Wagennr.",
                        "Radsatz ID",
                        "Radsatznummer",
                        "Einbaudatum NACH letzter IS2/3",
                    ]],
                ),
                ("Telematik", &[&["Asset", "Anbaudatum"]]),
            ],
        );
        let mut db = TrainsDb::load(&folder.config()).unwrap();
        crate::testing::client_master(&mut db, &file);
        sync(&mut db, false).unwrap();

        let mut settings = db.master().clone();
        let stale = &mut settings.bindings[0];
        stale.template_id = String::new();
        stale.key = None;
        stale.aliases.clear();
        let hand = &mut settings.bindings[1];
        hand.template_id = String::new();
        hand.auto = false;
        db.save_master(settings).unwrap();

        sync(&mut db, false).unwrap();
        let bindings = &db.master().bindings;
        assert_eq!(bindings[0].template_id, "builtin:radsatz-monitoring");
        assert_eq!(bindings[0].key.as_deref(), Some("Radsatz ID"));
        assert!(bindings[0].auto);
        assert_eq!(bindings[1].template_id, "", "hand-set, kept");
        assert!(!bindings[1].auto);
    }

    // A sheet missing one of the template's columns stays unbound, respelling
    // or not — the older Radsatz export has no Einbaudatum.
    #[test]
    fn a_respelled_sheet_missing_a_column_is_not_the_template() {
        let folder = TempDir::new("bindings-hint-missing");
        let binding = bound(&folder, &["Wagennr.", "Radsatz ID", "Radsatznummer"]);
        assert_eq!(binding.template_id, "");
        assert_eq!(binding.key, None);
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
