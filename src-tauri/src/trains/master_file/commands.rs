// ─── why ────────────────────────────────────────────────────────
// The master file's API, beside its module rather than in `trains::commands`:
// it shares nothing with the rest of trains' surface, and keeping it apart is
// the point. `main.rs`'s `generate_handler!` still indexes it.
//
// `clean_master_file` is a picker AND seconds of work, so it is
// `#[tauri::command(async)]` on a sync fn, and it lets go of the store while the
// workbook is read: the lock is taken for the duplicate check and again for
// `hold`, never across the cleaning. `accept_master_file` reads the taken-over
// version's headers once (`bindings::sync`), so the master settings page and the
// export open on it without a second wait. The decision is a free fn over `&AppState`
// (`clean_picked`) so a test can walk a file in without a window.
//
// `pick_master_target` is the MVP's way in: it only RECORDS the customer's path
// (`target_picked`) and reads its headers once, so the export opens on it.
// ────────────────────────────────────────────────────────────────

use std::path::Path;

use tauri::State;

use crate::error::{AppError, AppResult};
use crate::picker;
use crate::state::AppState;
use crate::trains::clock::today_iso;
use crate::trains::dokument;
use crate::trains::model::TrainsData;

#[tauri::command]
pub fn get_master_file(state: State<'_, AppState>) -> AppResult<TrainsData> {
    Ok(TrainsData::nothing().master_file(state.trains().master_file().clone()))
}

#[tauri::command(async)]
pub fn clean_master_file(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
) -> AppResult<TrainsData> {
    let Some(path) = picker::file(&window, "Master-Datei wählen", None, Some(picker::EXCEL))
    else {
        return Ok(TrainsData::nothing());
    };
    clean_picked(&path, &state)
}

#[tauri::command(async)]
pub fn pick_master_target(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
) -> AppResult<TrainsData> {
    let Some(path) = picker::file(&window, "Master-Datei wählen", None, Some(picker::EXCEL))
    else {
        return Ok(TrainsData::nothing());
    };
    target_picked(&path, &state)
}

#[tauri::command(async)]
pub fn accept_master_file(state: State<'_, AppState>) -> AppResult<TrainsData> {
    let mut db = state.trains();
    super::accept(&mut db, &today_iso())?;
    crate::trains::master::bindings::sync(&mut db, false)?;
    Ok(TrainsData::nothing().master_file(db.master_file().clone()))
}

#[tauri::command(async)]
pub fn discard_master_file(state: State<'_, AppState>) -> AppResult<TrainsData> {
    let mut db = state.trains();
    super::discard(&mut db)?;
    Ok(TrainsData::nothing().master_file(db.master_file().clone()))
}

fn target_picked(path: &Path, state: &AppState) -> AppResult<TrainsData> {
    let mut db = state.trains();
    super::set_target(&mut db, path)?;
    crate::trains::master::bindings::sync(&mut db, false)?;
    Ok(TrainsData::nothing()
        .master_file(db.master_file().clone())
        .master(crate::trains::master::view(db.master())))
}

fn clean_picked(path: &Path, state: &AppState) -> AppResult<TrainsData> {
    let hash = dokument::hash_of(path)?;
    let root = {
        let db = state.trains();
        if let Some(when) = super::known(db.master_file(), &hash) {
            return Err(AppError::Report(vec![
                format!(
                    "„{}“ wurde bereits am {when} bereinigt.",
                    crate::doc::file_name(path)
                ),
                "Diese Fassung der Master-Datei liegt schon vor.".into(),
            ]));
        }
        db.master_file_folder()
    };
    let version = super::clean(path, &root, &today_iso())?;
    let mut db = state.trains();
    super::hold(&mut db, version)?;
    Ok(TrainsData::nothing().master_file(db.master_file().clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{workbook, TempDir};

    #[test]
    fn the_same_bytes_are_refused_once_they_are_a_version() {
        let folder = TempDir::new("masterfile-commands");
        let state = folder.state();
        let original = workbook(&folder, "Master.xlsx", &[("Liste", &[&["Wagennummer"]])]);

        let first = clean_picked(&original, &state).unwrap();
        assert!(first.master_file.unwrap().pending.is_some());
        let error = clean_picked(&original, &state).unwrap_err();
        assert!(error.into_messages()[1].contains("schon vor"));
    }

    #[test]
    fn a_picked_target_becomes_the_master_the_export_reads() {
        let folder = TempDir::new("masterfile-target-picked");
        let state = folder.state();
        let original = workbook(&folder, "Master.xlsx", &[("Liste", &[&["Wagennummer"]])]);

        let data = target_picked(&original, &state).unwrap();
        let path = original.to_string_lossy().into_owned();
        assert_eq!(
            data.master_file.unwrap().pfad.as_deref(),
            Some(path.as_str())
        );
        let db = state.trains();
        assert_eq!(db.master().file.as_deref(), Some(path.as_str()));
        assert_eq!(db.master().scan.as_ref().unwrap().sheets[0].name, "Liste");
    }
}
