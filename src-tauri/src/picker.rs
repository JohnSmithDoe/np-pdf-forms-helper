// ─── why ────────────────────────────────────────────────────────
// The native pickers, shared by both workflows. `filler` and `trains` are sealed
// from each other, so the one piece of window code both need sits at the top
// level beside `doc/`, rather than being written twice and drifting.
//
// Every picker BLOCKS, which is why its callers are `#[tauri::command(async)]`
// on a sync fn — see `filler::commands`.
//
// An empty multi-selection is a cancelled picker, not a batch of nothing: without
// the check a caller would answer "0 Dokument(e) wurden hinzugefügt." to a dialog
// the user dismissed. And the picker can answer with a URL on mobile; on desktop
// it never does, and a path is the only thing the backend can use.
// ────────────────────────────────────────────────────────────────

use std::path::{Path, PathBuf};

use tauri_plugin_dialog::{DialogExt, FileDialogBuilder};

pub const EXCEL: (&str, &[&str]) = ("Excel", &["xlsx"]);

fn dialog(
    window: &tauri::WebviewWindow,
    title: &str,
    filter: Option<(&str, &[&str])>,
) -> FileDialogBuilder<tauri::Wry> {
    let picker = window.dialog().file().set_title(title).set_parent(window);
    match filter {
        Some((name, extensions)) => picker.add_filter(name, extensions),
        None => picker,
    }
}

pub fn file(
    window: &tauri::WebviewWindow,
    title: &str,
    folder: Option<&Path>,
    filter: Option<(&str, &[&str])>,
) -> Option<PathBuf> {
    let mut picker = dialog(window, title, filter);
    if let Some(folder) = folder {
        picker = picker.set_directory(folder);
    }
    picker.blocking_pick_file().and_then(into_path)
}

pub fn files(
    window: &tauri::WebviewWindow,
    title: &str,
    filter: Option<(&str, &[&str])>,
) -> Option<Vec<PathBuf>> {
    let picked: Vec<PathBuf> = dialog(window, title, filter)
        .blocking_pick_files()?
        .into_iter()
        .filter_map(into_path)
        .collect();
    (!picked.is_empty()).then_some(picked)
}

pub fn folder(window: &tauri::WebviewWindow, title: &str) -> Option<PathBuf> {
    dialog(window, title, None)
        .blocking_pick_folder()
        .and_then(into_path)
}

fn into_path(picked: tauri_plugin_dialog::FilePath) -> Option<PathBuf> {
    picked.into_path().ok()
}
