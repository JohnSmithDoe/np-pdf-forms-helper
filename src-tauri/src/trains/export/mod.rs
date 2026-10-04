// ─── why ────────────────────────────────────────────────────────
// The ERP artefact: `run` writes a fresh workbook the ERP can import.
// `Run { messages, failed }` mirrors `filler::export` so the report reads the
// same and the existing dialog rule — a `messageFolder` or more than one line —
// routes it without knowing trains exists.
//
// The customer's master workbook is NOT here, and the asymmetry is why: the ERP
// file is OURS — generated from scratch every time, disposable, and nobody loses
// anything if it is wrong. The master is the USER'S: formulas, colours, filters
// and columns this program invented no meaning for, read AND written — see
// `trains/master/`.
//
// `fill` writes a header row and rows of TEXT (`set_value_string`), for every
// sheet this program generates. `set_value` guesses a type, and turns a
// wheelset number made of digits — or a Gattung `4` — into a number Excel then
// reformats and strips the leading zeros from. The master is the exception that
// needs real numbers, and `master::source` types its cells itself.
//
// `Run::report` is on the run and not in the command, for the reason
// `filler::export` records: it reads `failed`, and a `#[tauri::command]` cannot
// be exercised by `cargo test`.
// ────────────────────────────────────────────────────────────────

pub mod erp;

use std::path::Path;

use umya_spreadsheet::Worksheet;

use super::db::TrainsDb;
use crate::error::AppResult;
use crate::model::ClientReport;

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

    run.messages.push(if run.failed == 0 {
        "Alle Dateien wurden erfolgreich erstellt.".into()
    } else {
        format!("{} Schritt(e) sind fehlgeschlagen.", run.failed)
    });
    Ok(run)
}
