// ─── why ────────────────────────────────────────────────────────
// The composition root, and the one place that names every workflow.
//
// `generate_handler!` IS the index of what commands exist. Each workflow keeps
// its own `commands` module, so this list is the only file to read to learn the
// whole API surface.
//
// The module tree splits by what a reader has to know:
//   config, error, state, model, doc   shared — the shell and the document layer
//   about                              the app itself: version and resolved paths
//   filler, trains                     one folder per workflow, sealed from each
//                                      other and reaching only into the shared half
//
// Release builds on Windows detach from the console, or double-clicking the exe
// would flash a terminal behind the window. The cost is that the startup
// diagnostics below are only visible when run from a shell.
//
// Loading happens BEFORE the window exists, so a failure in `load_state` has
// nowhere to render. Both causes are rare — an uncreatable data folder, or a
// corrupt `data.db` — and go to stderr; a real error dialog is still owed.
//
// ────────────────────────────────────────────────────────────────

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod about;
mod config;
mod doc;
mod error;
mod filler;
mod model;
mod picker;
mod state;
#[cfg(test)]
mod testing;
mod trains;

use std::sync::Mutex;

use crate::config::AppConfig;
use crate::error::AppResult;
use crate::filler::db::Database;
use crate::state::AppState;
use crate::trains::db::TrainsDb;

fn main() {
    let state = match load_state() {
        Ok(state) => state,
        Err(error) => {
            eprintln!("npDokumentenhilfe konnte nicht starten: {error}");
            std::process::exit(1);
        }
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            about::app_info,
            filler::commands::get_client_data,
            filler::commands::add_documents,
            filler::commands::remap_document,
            filler::commands::save_document,
            filler::commands::remove_document,
            filler::commands::reset_app,
            filler::commands::save_profiles,
            filler::commands::create_documents,
            filler::commands::open_file,
            filler::commands::open_output_folder,
            trains::commands::get_trains_data,
            trains::commands::query_events,
            trains::commands::stage_import,
            trains::commands::stage_import_path,
            trains::commands::restage_import,
            trains::commands::restage_sheet,
            trains::commands::discard_import,
            trains::commands::stage_document,
            trains::commands::commit_document,
            trains::commands::save_template,
            trains::commands::save_waggon,
            trains::commands::remove_waggon,
            trains::commands::save_wheelset,
            trains::commands::remove_wheelset,
            trains::commands::save_partner,
            trains::commands::remove_partner,
            trains::commands::remove_template,
            trains::commands::save_trains_settings,
            trains::commands::reset_trains,
            trains::commands::create_trains_export,
            trains::commands::get_master,
            trains::commands::save_master,
            trains::commands::open_master_export,
            trains::commands::preview_master_export,
            trains::commands::write_master_export,
            trains::commands::get_master_sheet,
            trains::commands::get_entity_detail,
            trains::commands::get_telematik,
            trains::commands::set_farbe,
            trains::commands::reset_master_bindings,
            trains::commands::start_master_import,
            trains::commands::import_master_all,
            trains::commands::stage_master_sheet,
            trains::master_file::commands::get_master_file,
            trains::master_file::commands::clean_master_file,
            trains::master_file::commands::pick_master_target,
            trains::master_file::commands::accept_master_file,
            trains::master_file::commands::discard_master_file,
            trains::commands::pick_import_folder,
            trains::commands::pick_import_files,
            trains::commands::scan_import_paths,
            trains::commands::clean_file,
            trains::commands::reclean_file,
            trains::commands::write_clean,
            trains::commands::discard_clean,
        ])
        .run(tauri::generate_context!())
        .expect("Tauri konnte nicht gestartet werden");
}

fn load_state() -> AppResult<AppState> {
    let config = AppConfig::load()?;
    let db = Database::load(&config)?;
    let trains = TrainsDb::load(&config)?;
    Ok(AppState {
        config,
        db: Mutex::new(db),
        trains: Mutex::new(trains),
        staging: Mutex::new(None),
        cleaning: Mutex::new(None),
    })
}
