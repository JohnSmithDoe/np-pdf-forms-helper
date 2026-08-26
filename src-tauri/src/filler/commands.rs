// ─── why ────────────────────────────────────────────────────────
// The frontend API, and only that. Plain request/response: what a command
// returns IS the response, and `Err` rejects the same promise the caller is
// awaiting. There is no broadcast, so a failure reaches only the caller that
// caused it.
//
// The work behind the two big commands lives in `import` and `export`; what
// stays here is what needs a window — the native pickers and opening a file.
//
// Commands that cannot touch a list leave it out of `ClientData` entirely —
// see the `Option` note on that type. Where a command sends one list and not
// the other, that is the decision, not an omission.
//
// `DocumentSource` lives HERE and not in `model.rs`, because it is an argument
// to one command rather than part of the stored shape — nothing serialises it
// into `data.db`, and the renderer mirrors it in `filler.backend.ts`'s command
// union for the same reason. It replaced a `whole_folder: bool`: there are three
// ways in now, and a second boolean beside the first would make two of the four
// combinations mean nothing.
// ────────────────────────────────────────────────────────────────

use std::path::{Path, PathBuf};

use serde::Deserialize;
use tauri::State;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use super::{export, import};
use crate::config::AppConfig;
use crate::doc;
use crate::error::{AppError, AppResult};
use crate::model::{ClientData, ClientReport, DocumentKind, MappedDocument, MappedInput, Profile};
use crate::state::AppState;

// No report. The welcome text used to ride this response, which worked only
// while the page that subscribed to `report$` was also the caller. The load now
// happens in a route resolver, one phase before any page exists, so a report
// here would be emitted to nobody — silently. The version moved to `/about`.
#[tauri::command]
pub fn get_client_data(state: State<'_, AppState>) -> AppResult<ClientData> {
    let db = state.db();
    Ok(ClientData::nothing()
        .documents(db.documents())
        .profiles(db.profiles()))
}

#[tauri::command]
pub fn save_document(
    document: MappedDocument,
    state: State<'_, AppState>,
) -> AppResult<ClientData> {
    let mut db = state.db();
    db.update_document(document, false)?;
    Ok(ClientData::nothing()
        .documents(db.documents())
        .profiles(db.profiles())
        .report(ClientReport::headline(
            "Dokument wurde erfolgreich gespeichert",
        )))
}

#[tauri::command]
pub fn remove_document(id: String, state: State<'_, AppState>) -> AppResult<ClientData> {
    let mut db = state.db();
    db.remove_document(&id)?;
    Ok(ClientData::nothing()
        .documents(db.documents())
        .profiles(db.profiles())
        .report(ClientReport::headline(
            "Dokument wurde erfolgreich entfernt",
        )))
}

#[tauri::command]
pub fn reset_app(state: State<'_, AppState>) -> AppResult<ClientData> {
    let mut db = state.db();
    db.reset()?;
    Ok(ClientData::nothing()
        .documents(db.documents())
        .profiles(db.profiles())
        .report(ClientReport::headline(
            "System wurde erfolgreich zurückgesetzt",
        )))
}

// No `documents`: saving profiles cannot change the document list, and sending
// it anyway would tell the store to replace what it holds for no reason.
#[tauri::command]
pub fn save_profiles(profiles: Vec<Profile>, state: State<'_, AppState>) -> AppResult<ClientData> {
    let mut db = state.db();
    db.update_profiles(profiles)?;
    Ok(ClientData::nothing()
        .profiles(db.profiles())
        .report(ClientReport::headline(
            "Profile wurden erfolgreich aktualisiert",
        )))
}

// The opener plugin hands the path to the OS as an argument, never to a shell,
// so a filename holding `&` or `"` is a filename and not a command line.
#[tauri::command]
pub fn open_file(app: tauri::AppHandle, filename: String) -> AppResult<ClientData> {
    open(&app, &filename)
}

// The frontend sends the run subfolder off a `ClientReport`, or '' when the
// button is pressed outside a run — so a path that is not there is normal, not
// an error, and falls back to the output root. Without the fallback the empty
// string is just a path that does not exist, and the button answers "Die Datei
// konnte nicht geöffnet werden."
#[tauri::command]
pub fn open_output_folder(
    app: tauri::AppHandle,
    folder: String,
    state: State<'_, AppState>,
) -> AppResult<ClientData> {
    let target = folder_to_open(&folder, &state.config);
    open(&app, &target.to_string_lossy())
}

// Split out of the command so the rule can be exercised without a window: the
// command itself is nothing but this decision plus `open`.
fn folder_to_open(requested: &str, config: &AppConfig) -> PathBuf {
    let requested = PathBuf::from(requested);
    if requested.is_dir() {
        requested
    } else {
        config.output_path.clone()
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocumentSource {
    File,
    Files,
    Folder,
}

// ─── the picker commands ──────────────────────────────────────────
// `#[tauri::command(async)]` on a SYNCHRONOUS fn, and that matters twice over:
// a plain `#[tauri::command]` runs on the main thread, where a blocking native
// dialog would deadlock the very event loop it needs; and a real `async fn`
// cannot hold the `Mutex<Database>` guard across an await. Marking a sync fn
// `async` moves it to a worker thread and keeps both problems away.

#[tauri::command(async)]
pub fn add_documents(
    window: tauri::WebviewWindow,
    auto_map_fields: bool,
    source: DocumentSource,
    state: State<'_, AppState>,
) -> AppResult<ClientData> {
    let report = match source {
        DocumentSource::File => add_single(&window, auto_map_fields, &state)?,
        DocumentSource::Files => add_many(&window, auto_map_fields, &state)?,
        DocumentSource::Folder => add_folder(&window, auto_map_fields, &state)?,
    };
    let data = ClientData::nothing().documents(state.db().documents());
    // A cancelled picker is not a failure and gets no report.
    Ok(match report {
        Some(report) => data.report(report),
        None => data,
    })
}

#[tauri::command(async)]
pub fn remap_document(
    window: tauri::WebviewWindow,
    id: String,
    state: State<'_, AppState>,
) -> AppResult<ClientData> {
    let mut document = state
        .db()
        .get(&id)
        .cloned()
        .ok_or(AppError::DocumentMissing)?;
    let folder = Path::new(&document.filename)
        .parent()
        .map(Path::to_path_buf);

    let Some(filename) = pick_file(&window, "Dokument neu verknüpfen", folder.as_deref()) else {
        // Cancelled. Answer with the unchanged state, never nothing — the
        // renderer dereferences the response.
        let db = state.db();
        return Ok(ClientData::nothing()
            .documents(db.documents())
            .profiles(db.profiles()));
    };

    doc::remap(&mut document, &filename, &state.config)?;
    let name = document.name.clone();
    let mut db = state.db();
    // `force`: the field list may be identical while every id behind it was
    // regenerated, which no length comparison can see.
    db.update_document(document, true)?;
    Ok(ClientData::nothing()
        .documents(db.documents())
        .profiles(db.profiles())
        .report(ClientReport {
            headline: "Dokument wurde erfolgreich neu verknüpft".into(),
            messages: vec![format!("Dokument wurde neu verknüpft mit: {name}")],
            message_folder: None,
        }))
}

// Also `(async)`, for the other half of the reason: an export copies, rewrites
// and writes one file per document, and on the main thread that is the window
// frozen for the duration.
#[tauri::command(async)]
pub fn create_documents(
    export_folder: String,
    document_ids: Vec<String>,
    inputs: Vec<MappedInput>,
    state: State<'_, AppState>,
) -> AppResult<ClientData> {
    let output_folder = state.config.output_path.join(&export_folder);
    let documents = state.db().documents_by_ids(&document_ids);
    let run = export::run(&output_folder, documents, &inputs)?;
    let report = run.report(&output_folder);

    // The refreshed mtime is PERSISTED, not just held in memory, or the
    // "original was changed" warning returns on every start.
    let mut db = state.db();
    for document in run.refreshed {
        db.update_document(document, false)?;
    }

    Ok(ClientData::nothing()
        .documents(db.documents())
        .report(report))
}

fn add_single(
    window: &tauri::WebviewWindow,
    auto_map_fields: bool,
    state: &AppState,
) -> AppResult<Option<ClientReport>> {
    let Some(filename) = pick_file(window, "Dokument verknüpfen", None) else {
        return Ok(None);
    };
    let document = import::one(&filename, auto_map_fields, state)?;
    // Best effort: the preview is a convenience, and a viewer that refuses to
    // open must not undo a document that is already linked.
    if let DocumentKind::Pdf { previewfile, .. } = &document.kind {
        let _ = window.opener().open_path(previewfile, None::<&str>);
    }
    Ok(Some(ClientReport::headline(
        "Dokument wurde erfolgreich hinzugefügt",
    )))
}

fn add_many(
    window: &tauri::WebviewWindow,
    auto_map_fields: bool,
    state: &AppState,
) -> AppResult<Option<ClientReport>> {
    let Some(files) = pick_files(window, "Dokumente verknüpfen") else {
        return Ok(None);
    };
    Ok(Some(ClientReport {
        headline: "Dokumente wurden erfolgreich hinzugefügt".into(),
        messages: import::many(&files, auto_map_fields, state)?,
        message_folder: None,
    }))
}

fn add_folder(
    window: &tauri::WebviewWindow,
    auto_map_fields: bool,
    state: &AppState,
) -> AppResult<Option<ClientReport>> {
    let Some(folder) = pick_folder(window, "Ordner verknüpfen") else {
        return Ok(None);
    };
    Ok(Some(ClientReport {
        headline: "Ordner wurde erfolgreich hinzugefügt".into(),
        messages: import::folder(&folder, auto_map_fields, state)?,
        message_folder: None,
    }))
}

fn open(app: &tauri::AppHandle, path: &str) -> AppResult<ClientData> {
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|error| {
            AppError::detail("Die Datei konnte nicht geöffnet werden.".into(), error)
        })?;
    Ok(ClientData::nothing())
}

fn pick_file(window: &tauri::WebviewWindow, title: &str, folder: Option<&Path>) -> Option<PathBuf> {
    let mut picker = window.dialog().file().set_title(title).set_parent(window);
    if let Some(folder) = folder {
        picker = picker.set_directory(folder);
    }
    picker.blocking_pick_file().and_then(into_path)
}

// An empty selection is a cancelled picker, not a batch of nothing: without the
// check `import::many` would answer "0 Dokument(e) wurden hinzugefügt." to a
// dialog the user dismissed.
fn pick_files(window: &tauri::WebviewWindow, title: &str) -> Option<Vec<PathBuf>> {
    let picked: Vec<PathBuf> = window
        .dialog()
        .file()
        .set_title(title)
        .set_parent(window)
        .blocking_pick_files()?
        .into_iter()
        .filter_map(into_path)
        .collect();
    (!picked.is_empty()).then_some(picked)
}

fn pick_folder(window: &tauri::WebviewWindow, title: &str) -> Option<PathBuf> {
    window
        .dialog()
        .file()
        .set_title(title)
        .set_parent(window)
        .blocking_pick_folder()
        .and_then(into_path)
}

// The picker can also answer with a URL on mobile. On desktop it never does,
// and a path is the only thing the rest of the backend can use.
fn into_path(picked: tauri_plugin_dialog::FilePath) -> Option<PathBuf> {
    picked.into_path().ok()
}

// Only the decisions that were split out of a command are testable here: a
// `State` and a `WebviewWindow` cannot be constructed outside a running app, so
// everything else on this surface is covered through the Playwright fake.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TempDir;

    // The frontend sends the run subfolder off a `ClientReport`, or '' when the
    // button is pressed outside a run — so a path that is not there is normal
    // and falls back to the output root. Without the fallback the empty string
    // is just a path that does not exist, and the button answers "Die Datei
    // konnte nicht geöffnet werden."
    #[test]
    fn an_empty_folder_falls_back_to_the_output_root() {
        let temp = TempDir::new("open-empty");
        let config = temp.config();
        assert_eq!(folder_to_open("", &config), config.output_path);
    }

    #[test]
    fn a_folder_that_is_gone_falls_back_to_the_output_root() {
        let temp = TempDir::new("open-missing");
        let config = temp.config();
        let requested = temp.join("run").to_string_lossy().into_owned();
        assert_eq!(folder_to_open(&requested, &config), config.output_path);
    }

    // A FILE is not a folder either — the button opens folders.
    #[test]
    fn a_file_falls_back_to_the_output_root() {
        let temp = TempDir::new("open-file");
        let config = temp.config();
        let file = temp.write("anhang.txt", "inhalt");
        assert_eq!(
            folder_to_open(&file.to_string_lossy(), &config),
            config.output_path
        );
    }

    #[test]
    fn a_real_folder_is_opened_as_asked() {
        let temp = TempDir::new("open-real");
        let config = temp.config();
        let run = config.output_path.join("run");
        std::fs::create_dir_all(&run).unwrap();
        assert_eq!(folder_to_open(&run.to_string_lossy(), &config), run);
    }
}
