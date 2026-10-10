// ─── why ────────────────────────────────────────────────────────
// The customer's master workbook as a FILE the app owns: picked, copied in,
// cleaned, and kept as versions. It is THE master — `trains/master/` binds,
// mirrors and exports against its current version (`bindings::follow`). Still
// no Dokument: a master is never imported by the walk nor
// exported into itself.
//
// ONLY ONE EXISTS. The customer produces it again and again, so each pick is a
// new VERSION of the one master, not a second master; `versions[0]` is the
// latest and the older ones stay as history. Identity is the original's
// content hash, as for a Dokument — the same bytes picked twice are refused.
//
// Plan/apply with the files on disk. `clean` copies the original into
// `masterdatei/<id>/` BEFORE reading it (`dokument::adopt`, the landing-zone
// rule), cleans every sheet (`clean::sheet`) and writes the cleaned copy and
// its `protokoll.json` — a whole workbook, so holding it in memory between two
// commands would hold gigabytes. The result is `pending` until the user takes
// it over (`accept`) or drops it (`discard`, which removes the folder). A new
// pick replaces an untaken one.
//
// `clean` takes no store: the caller fetches folder and hashes and lets go of
// the lock before the workbook is read, which takes seconds on the real file.
// A SHEET WITH NOTHING TO CLEAN IS COPIED BYTE FOR BYTE. umya loses small things
// in every sheet it deserialises and writes back (`docs/footguns.md`, „Writing a
// workbook back“), so `clean_adopted` reads twice: a probe book cleans every
// sheet and throws it away, and only the sheets that changed are deserialised
// in the book that is written. On the real master that is a dozen of 28.
//
// THE MVP WRITES THE CUSTOMER'S OWN FILE (`pfad`, set by `set_target`, no copy,
// no cleaning). When set it is THE master and `bindings::follow` prefers it over
// any version; the versions and the cleaning stay for the ERP's older path.
// `write_in_place` never writes without a backup: it first opens the file for
// writing — Excel's lock on Windows fails HERE, before anything is copied — then
// copies it into `Sicherungen/` beside it, and only then replaces it through
// `write_book`'s temp + rename. A failed copy writes nothing. See
// `docs/decisions.md`, „MVP: In die Kunden-Master schreiben“.
// ────────────────────────────────────────────────────────────────

mod clean;
pub mod commands;

use std::path::{Path, PathBuf};

use serde::Serialize;

use super::db::{remove_folder, TrainsDb};
use super::dokument;
use super::model::{
    MasterFile, MasterFileChange, MasterFileNote, MasterFileReport, MasterFileTotals,
    MasterFileVersion,
};
use crate::error::{AppError, AppResult};

const PROTOCOL_FILE: &str = "protokoll.json";
const SICHERUNGEN: &str = "Sicherungen";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Protocol<'a> {
    sheet: &'a str,
    changes: &'a [MasterFileChange],
    notes: &'a [MasterFileNote],
}

pub fn known(file: &MasterFile, hash: &str) -> Option<String> {
    file.versions
        .iter()
        .chain(&file.pending)
        .find(|version| version.original_hash == hash)
        .map(|version| version.bereinigt_am.clone())
}

pub fn clean(original: &Path, root: &Path, stamp: &str) -> AppResult<MasterFileVersion> {
    if !is_xlsx(original) {
        return Err(AppError::Report(vec![
            "Nur Excel-Dateien (.xlsx) können als Master-Datei bereinigt werden.".into(),
        ]));
    }
    let adopted = dokument::adopt(original, root)?;
    match clean_adopted(&adopted, stamp) {
        Ok(version) => Ok(version),
        Err(error) => {
            dokument::discard(&adopted.folder);
            Err(error)
        }
    }
}

fn clean_adopted(adopted: &dokument::Adopted, stamp: &str) -> AppResult<MasterFileVersion> {
    let headline = || {
        format!(
            "Die Master-Datei {} konnte nicht gelesen werden.",
            adopted.name
        )
    };
    let open = || {
        AppError::reading(headline(), || {
            umya_spreadsheet::reader::xlsx::lazy_read(&adopted.path)
        })
    };
    let mut probe = open()?;
    let count = probe.sheet_collection_no_check().len();
    if count == 0 {
        return Err(AppError::Report(vec![
            "Die Master-Datei enthält keine Arbeitsmappen.".into(),
        ]));
    }

    let mut outcomes = Vec::with_capacity(count);
    for index in 0..count {
        let worksheet = sheet(&mut probe, index, headline)?;
        outcomes.push(clean::sheet(worksheet));
        *worksheet = umya_spreadsheet::Worksheet::default();
    }
    drop(probe);

    let mut book = open()?;
    for (index, outcome) in outcomes.iter_mut().enumerate() {
        if outcome.report.rows_cut + outcome.report.tail_rows_cut > 0 || !outcome.changes.is_empty()
        {
            *outcome = clean::sheet(sheet(&mut book, index, headline)?);
        }
    }
    let (sheets, protocol): (Vec<_>, Vec<_>) = outcomes
        .into_iter()
        .map(|outcome| (outcome.report, (outcome.changes, outcome.notes)))
        .unzip();

    let stem = Path::new(&adopted.name).file_stem().map_or_else(
        || "Master".into(),
        |stem| stem.to_string_lossy().into_owned(),
    );
    let cleaned = adopted.folder.join(format!("{stem} bereinigt.xlsx"));
    crate::doc::write_book(
        &book,
        &cleaned,
        format!("Die bereinigte Master-Datei {stem} konnte nicht geschrieben werden."),
    )?;

    let lines: Vec<Protocol> = sheets
        .iter()
        .zip(&protocol)
        .map(|(sheet, (changes, notes))| Protocol {
            sheet: &sheet.sheet,
            changes,
            notes,
        })
        .collect();
    let path = adopted.folder.join(PROTOCOL_FILE);
    let json = serde_json::to_vec(&lines).map_err(|error| AppError::json(&path, error))?;
    std::fs::write(&path, json).map_err(|error| AppError::io(&path, error))?;

    let totals = sheets
        .iter()
        .fold(MasterFileTotals::default(), |sum, sheet| MasterFileTotals {
            rows_cut: sum.rows_cut + sheet.rows_cut,
            tail_rows_cut: sum.tail_rows_cut + sheet.tail_rows_cut,
            trimmed: sum.trimmed + sheet.trimmed,
            numbers: sum.numbers + sheet.numbers,
            dates: sum.dates + sheet.dates,
            notes: sum.notes + sheet.note_count,
        });
    Ok(MasterFileVersion {
        id: adopted.id.clone(),
        name: adopted.name.clone(),
        folder: adopted.folder.to_string_lossy().into_owned(),
        original: adopted.path.to_string_lossy().into_owned(),
        cleaned: cleaned.to_string_lossy().into_owned(),
        original_hash: adopted.hash.clone(),
        cleaned_hash: dokument::hash_of(&cleaned)?,
        bereinigt_am: stamp.to_string(),
        uebernommen_am: None,
        quelle: None,
        report: MasterFileReport { sheets, totals },
    })
}

fn sheet(
    book: &mut umya_spreadsheet::Workbook,
    index: usize,
    headline: impl Fn() -> String,
) -> AppResult<&mut umya_spreadsheet::Worksheet> {
    AppError::reading(headline(), || {
        book.read_sheet(index);
        Ok::<_, std::convert::Infallible>(())
    })?;
    book.sheet_mut(index)
        .map_err(|error| AppError::detail(headline(), error))
}

pub fn hold(db: &mut TrainsDb, version: MasterFileVersion) -> AppResult<()> {
    let mut file = db.master_file().clone();
    let folder = version.folder.clone();
    let previous = file.pending.replace(version);
    if let Err(error) = db.save_master_file(file) {
        dokument::discard(Path::new(&folder));
        return Err(error);
    }
    if let Some(previous) = previous {
        remove_folder(Path::new(&previous.folder))?;
    }
    Ok(())
}

pub fn accept(db: &mut TrainsDb, stamp: &str) -> AppResult<()> {
    let mut file = db.master_file().clone();
    let Some(mut version) = file.pending.take() else {
        return Err(nothing_pending());
    };
    version.uebernommen_am = Some(stamp.to_string());
    file.versions.insert(0, version);
    db.save_master_file(file)
}

pub fn set_target(db: &mut TrainsDb, path: &Path) -> AppResult<()> {
    if !is_xlsx(path) {
        return Err(AppError::Report(vec![
            "Nur Excel-Dateien (.xlsx) können als Master-Datei gewählt werden.".into(),
        ]));
    }
    if !path.is_file() {
        return Err(AppError::Report(vec![format!(
            "Die Master-Datei {} gibt es nicht.",
            path.display()
        )]));
    }
    let mut file = db.master_file().clone();
    file.pfad = Some(path.to_string_lossy().into_owned());
    db.save_master_file(file)
}

#[derive(Debug)]
pub struct Written {
    pub target: PathBuf,
    pub sicherung: PathBuf,
}

pub fn write_in_place(
    book: &umya_spreadsheet::Workbook,
    target: &Path,
    stamp: &str,
) -> AppResult<Written> {
    let name = crate::doc::file_name(target);
    std::fs::OpenOptions::new()
        .write(true)
        .open(target)
        .map_err(|error| {
            AppError::detail(
                format!(
                    "Die Master-Datei {name} kann nicht geschrieben werden. Ist sie noch in Excel geöffnet?"
                ),
                error,
            )
        })?;
    let sicherung = backup(target, stamp)?;
    crate::doc::write_book(
        book,
        target,
        format!("Die Master-Datei {name} konnte nicht geschrieben werden."),
    )?;
    Ok(Written {
        target: target.to_path_buf(),
        sicherung,
    })
}

fn backup(target: &Path, stamp: &str) -> AppResult<PathBuf> {
    let headline = || {
        "Die Sicherung der Master-Datei konnte nicht angelegt werden. Es wurde nichts geschrieben."
            .to_string()
    };
    let folder = target
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(SICHERUNGEN);
    std::fs::create_dir_all(&folder).map_err(|error| AppError::detail(headline(), error))?;
    let stem = target.file_stem().map_or_else(
        || "Master".into(),
        |stem| stem.to_string_lossy().into_owned(),
    );
    let path = crate::doc::free_path(&folder, &format!("{stem} {stamp}.xlsx"));
    std::fs::copy(target, &path).map_err(|error| AppError::detail(headline(), error))?;
    Ok(path)
}

fn is_xlsx(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("xlsx"))
}

pub fn discard(db: &mut TrainsDb) -> AppResult<()> {
    let mut file = db.master_file().clone();
    let Some(version) = file.pending.take() else {
        return Ok(());
    };
    db.save_master_file(file)?;
    remove_folder(Path::new(&version.folder))
}

fn nothing_pending() -> AppError {
    AppError::Report(vec![
        "Es liegt keine bereinigte Master-Datei zur Übernahme vor.".into(),
        "Bitte die Master-Datei erneut wählen.".into(),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{workbook, TempDir};

    fn db(folder: &TempDir) -> TrainsDb {
        TrainsDb::load(&folder.config()).unwrap()
    }

    fn master(folder: &TempDir, name: &str) -> std::path::PathBuf {
        workbook(
            folder,
            name,
            &[
                (
                    "Liste",
                    &[
                        &["Wagennummer", "Stadt"],
                        &["#338506591522", " Köln "],
                        &["#338506591530", "Bonn"],
                        &["3385 0659 152-2", "Aachen"],
                    ],
                ),
                ("Notizen", &[&["Text"], &["frei"]]),
            ],
        )
    }

    #[test]
    fn a_pick_is_copied_cleaned_and_held_until_taken_over() {
        let folder = TempDir::new("masterfile-clean");
        let original = master(&folder, "Master.xlsx");
        let before = std::fs::read(&original).unwrap();
        let mut db = db(&folder);

        let version = clean(&original, &db.master_file_folder(), "2026-10-04").unwrap();
        assert!(Path::new(&version.cleaned).is_file());
        assert!(Path::new(&version.folder).join(PROTOCOL_FILE).is_file());
        assert_eq!(version.report.sheets.len(), 2);
        assert_eq!(version.report.totals.numbers, 1);
        // The customer's file is only ever read.
        assert_eq!(std::fs::read(&original).unwrap(), before);

        hold(&mut db, version.clone()).unwrap();
        assert!(db.master_file().versions.is_empty());
        accept(&mut db, "2026-10-04").unwrap();
        assert_eq!(db.master_file().versions[0].id, version.id);
        assert_eq!(db.master_file().pending, None);
        assert_eq!(
            known(db.master_file(), &version.original_hash).as_deref(),
            Some("2026-10-04")
        );
    }

    #[test]
    fn a_new_version_goes_first_and_the_old_one_stays() {
        let folder = TempDir::new("masterfile-versions");
        let mut db = db(&folder);
        for name in ["Master 1.xlsx", "Master 2.xlsx"] {
            let version = clean(
                &master(&folder, name),
                &db.master_file_folder(),
                "2026-10-04",
            );
            hold(&mut db, version.unwrap()).unwrap();
            accept(&mut db, "2026-10-04").unwrap();
        }
        let names: Vec<&str> = db
            .master_file()
            .versions
            .iter()
            .map(|version| version.name.as_str())
            .collect();
        assert_eq!(names, vec!["Master 2.xlsx", "Master 1.xlsx"]);
    }

    #[test]
    fn discarding_removes_the_pending_copy() {
        let folder = TempDir::new("masterfile-discard");
        let mut db = db(&folder);
        let version = clean(
            &master(&folder, "Master.xlsx"),
            &db.master_file_folder(),
            "x",
        )
        .unwrap();
        let copy = version.folder.clone();
        hold(&mut db, version).unwrap();
        discard(&mut db).unwrap();
        assert!(!Path::new(&copy).exists());
        assert!(accept(&mut db, "x").is_err());
    }

    #[test]
    fn a_new_pick_replaces_an_untaken_one() {
        let folder = TempDir::new("masterfile-replace");
        let mut db = db(&folder);
        let first = clean(&master(&folder, "A.xlsx"), &db.master_file_folder(), "x").unwrap();
        let first_folder = first.folder.clone();
        hold(&mut db, first).unwrap();
        let second = clean(&master(&folder, "B.xlsx"), &db.master_file_folder(), "x").unwrap();
        hold(&mut db, second).unwrap();
        assert!(!Path::new(&first_folder).exists());
        assert_eq!(db.master_file().pending.as_ref().unwrap().name, "B.xlsx");
    }

    #[test]
    fn a_target_is_the_customers_own_path_and_nothing_is_copied() {
        let folder = TempDir::new("masterfile-target");
        let original = master(&folder, "Master.xlsx");
        let mut db = db(&folder);
        set_target(&mut db, &original).unwrap();
        assert_eq!(
            db.master_file().pfad.as_deref(),
            Some(original.to_string_lossy().as_ref())
        );
        assert!(!db.master_file_folder().exists());
        assert!(set_target(&mut db, &folder.write("Master.xlsm", "x")).is_err());
        assert!(set_target(&mut db, &folder.join("fehlt.xlsx")).is_err());
    }

    #[test]
    fn writing_in_place_backs_the_old_file_up_first() {
        let folder = TempDir::new("masterfile-in-place");
        let original = master(&folder, "Master.xlsx");
        let before = std::fs::read(&original).unwrap();
        let mut book = umya_spreadsheet::reader::xlsx::read(&original).unwrap();
        book.sheet_by_name_mut("Liste")
            .unwrap()
            .cell_mut("B2")
            .set_value("Dortmund");

        let written = write_in_place(&book, &original, "2026-10-10 120000").unwrap();
        assert_eq!(
            written.sicherung,
            folder
                .join("Sicherungen")
                .join("Master 2026-10-10 120000.xlsx")
        );
        assert_eq!(std::fs::read(&written.sicherung).unwrap(), before);
        let after = umya_spreadsheet::reader::xlsx::read(&original).unwrap();
        assert_eq!(
            after.sheet_by_name("Liste").unwrap().value("B2"),
            "Dortmund"
        );

        // A second write the same second does not overwrite the first backup.
        let again = write_in_place(&book, &original, "2026-10-10 120000").unwrap();
        assert_ne!(again.sicherung, written.sicherung);
        assert_eq!(std::fs::read(&written.sicherung).unwrap(), before);
    }

    #[test]
    fn without_a_backup_nothing_is_written() {
        let folder = TempDir::new("masterfile-no-backup");
        let original = master(&folder, "Master.xlsx");
        let before = std::fs::read(&original).unwrap();
        // A FILE where the backup folder should go makes the backup fail.
        folder.write("Sicherungen", "kein Ordner");
        let book = umya_spreadsheet::reader::xlsx::read(&original).unwrap();
        let error = write_in_place(&book, &original, "x").unwrap_err();
        assert!(error.into_messages()[0].contains("nichts geschrieben"));
        assert_eq!(std::fs::read(&original).unwrap(), before);
    }

    #[test]
    fn a_workbook_that_is_not_xlsx_is_refused_before_anything_is_copied() {
        let folder = TempDir::new("masterfile-xlsm");
        let original = folder.write("Master.xlsm", "x");
        let root = folder.join("masterdatei");
        assert!(clean(&original, &root, "x").is_err());
        assert!(!root.exists());
    }
}
