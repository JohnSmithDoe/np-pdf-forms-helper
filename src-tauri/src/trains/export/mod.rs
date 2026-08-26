// ─── why ────────────────────────────────────────────────────────
// One run, two artefacts: a fresh workbook the ERP can import, and an upsert
// into the master workbook the user owns. `Run { messages, failed }` mirrors
// `filler::export` so the report reads the same and the existing dialog rule —
// a `messageFolder` or more than one line — routes it without knowing trains
// exists.
//
// The two halves are not symmetric and must not be written as if they were. The
// ERP file is OURS: generated from scratch every time, disposable, and nobody
// loses anything if it is wrong. The master is the USER'S: it has formulas,
// colours, filters and columns this program invented no meaning for, so it is
// updated in place, never rebuilt, and never has a row deleted from it.
//
// A failed master update does not fail the run. The ERP file is the thing the
// user came for, and refusing to produce it because a workbook was open in Excel
// would be the tool getting in the way.
//
// `INSTANDHALTUNG_COLUMNS` and `instandhaltung_row` live HERE because both halves write the same
// seven columns and the master locates its cells BY HEADER NAME. Two copies of
// the list is not a tidiness question: the moment they drift, the master stops
// updating in place and silently starts appending a second set of columns beside
// the first.
//
// `Run::report` is on the run and not in the command, for the reason
// `filler::export` records: it reads `failed`, and a `#[tauri::command]` cannot
// be exercised by `cargo test`.
// ────────────────────────────────────────────────────────────────

pub mod erp;
pub mod master;

use std::path::Path;

use super::db::TrainsDb;
use super::model::Instandhaltung;
use super::sanitise::format;
use crate::error::AppResult;
use crate::model::ClientReport;

pub const INSTANDHALTUNG_COLUMNS: [&str; 7] = [
    "Id",
    "Wagennummer",
    "Datum",
    "Werkstatt",
    "Leistung",
    "Betrag",
    "Bemerkung",
];

pub fn instandhaltung_row(db: &TrainsDb, event: &Instandhaltung) -> [String; 7] {
    [
        event.id.clone(),
        db.wagen_by_id(&event.wagen_id)
            .map(|wagen| format::uic_display(&wagen.nummer))
            .unwrap_or_default(),
        event
            .datum
            .as_deref()
            .map(format::iso_date)
            .unwrap_or_default(),
        event
            .werkstatt_id
            .as_deref()
            .and_then(|id| db.partner_by_id(id))
            .map(|partner| partner.name.clone())
            .unwrap_or_default(),
        event.leistung.clone(),
        event.betrag_cent.map(format::money).unwrap_or_default(),
        event.bemerkung.clone().unwrap_or_default(),
    ]
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

pub fn run(db: &TrainsDb, folder: &Path, master_file: &Path) -> AppResult<Run> {
    let mut run = Run::default();

    std::fs::create_dir_all(folder).map_err(|error| crate::error::AppError::io(folder, error))?;
    run.messages
        .push(format!("Ordner wurde erstellt: {}", folder.display()));

    match erp::write(db, folder) {
        Ok(files) => {
            for file in files {
                run.messages.push(format!("Datei wurde erstellt: {file}"));
            }
        }
        Err(error) => {
            run.failed += 1;
            run.messages.extend(error.into_messages());
        }
    }

    match master::upsert(db, master_file) {
        Ok(summary) => run.messages.push(summary),
        Err(error) => {
            run.failed += 1;
            run.messages
                .push("Die Master-Datei konnte nicht aktualisiert werden.".into());
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
