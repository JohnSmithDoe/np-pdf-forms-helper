// ─── why ────────────────────────────────────────────────────────
// The trains API, and only that. It lives here rather than in `filler::commands`
// for the reason `doc/` has one folder per format: the workflow being worked on
// is the folder being worked in. `main.rs`'s `generate_handler!` remains the one
// index of what commands exist.
//
// `#[tauri::command(async)]` on a SYNCHRONOUS fn wherever a picker or real work
// is involved, for the two reasons `filler::commands` already records: a plain
// command runs on the main thread, where a blocking native dialog deadlocks the
// loop it needs, and a real `async fn` cannot hold a `Mutex` guard across an
// await. Reading a thousand-row workbook has the same problem from the other
// end — on the main thread it is a frozen window.
//
// `stage_import` reads the file ONCE and holds the grid; `restage_import` reparses
// what is held. That is why nudging a header row is instant and why the file on
// disk cannot change under a preview mid-decision.
//
// A cancelled picker is not a failure and gets no report — same as `add_documents`.
// ────────────────────────────────────────────────────────────────

use std::path::{Path, PathBuf};

use tauri::State;
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::model::ClientReport;
use crate::state::AppState;
use crate::trains::db::TrainsDb;
use crate::trains::model::{CommitDecisions, ImportPlan, Partner, Radsatz, TrainsData, Wagen};
use crate::trains::sheet::{grid, readers};
use crate::trains::stage::{stage, HeldImport, StageInput};
use crate::trains::{commit, export, fingerprint};

fn everything(db: &TrainsDb) -> TrainsData {
    TrainsData::nothing()
        .wagen(db.wagen())
        .partner(db.partner())
        .radsaetze(db.radsaetze())
        .einbauten(db.einbauten())
        .templates(db.templates())
        .counts(db.counts())
}

#[tauri::command]
pub fn get_trains_data(state: State<'_, AppState>) -> AppResult<TrainsData> {
    Ok(everything(&state.trains()))
}

#[tauri::command]
pub fn query_events(
    wagen_id: Option<String>,
    offset: u32,
    limit: u32,
    state: State<'_, AppState>,
) -> AppResult<TrainsData> {
    let db = state.trains();
    let (rows, total) = db.instandhaltungen_page(wagen_id.as_deref(), offset, limit.clamp(1, 500));
    Ok(
        TrainsData::nothing().instandhaltungen(crate::trains::model::InstandhaltungPage {
            rows,
            total,
            offset,
        }),
    )
}

#[tauri::command(async)]
pub fn stage_import(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
) -> AppResult<TrainsData> {
    let Some(path) = pick_file(&window) else {
        return Ok(TrainsData::nothing());
    };
    read_and_stage(&path, None, &state)
}

#[tauri::command(async)]
pub fn restage_import(plan: ImportPlan, state: State<'_, AppState>) -> AppResult<TrainsData> {
    let staging_id = Uuid::new_v4().to_string();
    let (wire, values) = {
        let held = state.staging();
        let held = held.as_ref().ok_or_else(stale)?;
        let staged = stage(StageInput {
            id: staging_id,
            file: held.wire.file.clone(),
            sheets: held.wire.sheets.clone(),
            candidates: held.wire.candidates.clone(),
            grid: &held.grid,
            plan: &plan,
            db: &state.trains(),
        })?;
        (staged.wire, staged.values)
    };

    let mut held = state.staging();
    let held = held.as_mut().ok_or_else(stale)?;
    held.wire = wire.clone();
    held.values = values;
    Ok(TrainsData::nothing().staging(wire))
}

#[tauri::command(async)]
pub fn restage_sheet(sheet: String, state: State<'_, AppState>) -> AppResult<TrainsData> {
    let path = {
        let held = state.staging();
        PathBuf::from(&held.as_ref().ok_or_else(stale)?.wire.file)
    };
    read_and_stage(&path, Some(&sheet), &state)
}

#[tauri::command]
pub fn discard_import(state: State<'_, AppState>) -> AppResult<TrainsData> {
    *state.staging() = None;
    Ok(TrainsData::nothing().counts(state.trains().counts()))
}

#[tauri::command(async)]
pub fn commit_import(
    decisions: CommitDecisions,
    state: State<'_, AppState>,
) -> AppResult<TrainsData> {
    let held = state.staging();
    let held = held.as_ref().ok_or_else(stale)?;
    let mut db = state.trains();
    let run = commit::commit(&mut db, &held.wire, &held.values, &decisions)?;

    Ok(everything(&db).report(run.report()))
}

#[tauri::command]
pub fn save_waggon(wagen: Wagen, state: State<'_, AppState>) -> AppResult<TrainsData> {
    let mut db = state.trains();
    db.transaction(|tx| {
        tx.put_wagen(wagen);
        Ok(())
    })?;
    Ok(TrainsData::nothing()
        .wagen(db.wagen())
        .counts(db.counts())
        .report(ClientReport::headline("Wagen wurde gespeichert")))
}

#[tauri::command]
pub fn remove_waggon(id: String, state: State<'_, AppState>) -> AppResult<TrainsData> {
    let mut db = state.trains();
    db.transaction(|tx| {
        tx.remove_wagen(&id);
        Ok(())
    })?;
    Ok(TrainsData::nothing()
        .wagen(db.wagen())
        .einbauten(db.einbauten())
        .counts(db.counts())
        .report(ClientReport::headline("Wagen wurde entfernt")))
}

#[tauri::command]
pub fn save_wheelset(radsatz: Radsatz, state: State<'_, AppState>) -> AppResult<TrainsData> {
    let mut db = state.trains();
    let mut radsatz = radsatz;
    radsatz.match_key = crate::trains::resolve::radsatz::match_key(&radsatz.nummer);
    db.transaction(|tx| {
        tx.put_radsatz(radsatz);
        Ok(())
    })?;
    Ok(TrainsData::nothing()
        .radsaetze(db.radsaetze())
        .counts(db.counts())
        .report(ClientReport::headline("Radsatz wurde gespeichert")))
}

#[tauri::command]
pub fn remove_wheelset(id: String, state: State<'_, AppState>) -> AppResult<TrainsData> {
    let mut db = state.trains();
    db.transaction(|tx| {
        tx.remove_radsatz(&id);
        Ok(())
    })?;
    Ok(TrainsData::nothing()
        .radsaetze(db.radsaetze())
        .einbauten(db.einbauten())
        .counts(db.counts())
        .report(ClientReport::headline("Radsatz wurde entfernt")))
}

#[tauri::command]
pub fn save_partner(partner: Partner, state: State<'_, AppState>) -> AppResult<TrainsData> {
    let mut db = state.trains();
    let mut partner = partner;
    partner.match_key = crate::trains::resolve::partner::match_key(&partner.name);
    db.transaction(|tx| {
        tx.put_partner(partner);
        Ok(())
    })?;
    Ok(TrainsData::nothing()
        .partner(db.partner())
        .counts(db.counts())
        .report(ClientReport::headline("Partner wurde gespeichert")))
}

#[tauri::command]
pub fn remove_partner(id: String, state: State<'_, AppState>) -> AppResult<TrainsData> {
    let mut db = state.trains();
    db.transaction(|tx| {
        tx.remove_partner(&id);
        Ok(())
    })?;
    Ok(TrainsData::nothing()
        .wagen(db.wagen())
        .partner(db.partner())
        .counts(db.counts())
        .report(ClientReport::headline("Partner wurde entfernt")))
}

#[tauri::command]
pub fn remove_template(id: String, state: State<'_, AppState>) -> AppResult<TrainsData> {
    let mut db = state.trains();
    db.transaction(|tx| {
        tx.remove_template(&id);
        Ok(())
    })?;
    Ok(TrainsData::nothing()
        .templates(db.templates())
        .report(ClientReport::headline("Vorlage wurde entfernt")))
}

#[tauri::command]
pub fn reset_trains(state: State<'_, AppState>) -> AppResult<TrainsData> {
    let mut db = state.trains();
    db.reset()?;
    *state.staging() = None;
    Ok(everything(&db).report(ClientReport::headline("Zugdaten wurden zurückgesetzt")))
}

#[tauri::command(async)]
pub fn create_trains_export(state: State<'_, AppState>) -> AppResult<TrainsData> {
    let folder = state
        .config
        .output_path
        .join(format!("trains-{}", crate::trains::clock::today_iso()));
    let run = export::run(&state.trains(), &folder, &state.config.master_file)?;

    Ok(TrainsData::nothing().report(run.report(&folder)))
}

fn read_and_stage(path: &Path, sheet: Option<&str>, state: &AppState) -> AppResult<TrainsData> {
    let source = grid::read(path, sheet)?;
    let candidates = readers::detect(&source.grid);

    let chosen = readers::choose(&candidates)
        .or_else(|| candidates.first())
        .ok_or_else(|| {
            AppError::Report(vec![
                "In dieser Arbeitsmappe wurde keine Tabelle gefunden.".into(),
                "Bitte Kopfzeile und erste Datenzeile von Hand festlegen.".into(),
            ])
        })?;
    let layout = chosen.reader.apply(&source.grid, chosen.hint)?;
    let plan = ImportPlan {
        reader: chosen.reader,
        layout: chosen.hint,
        columns: layout
            .columns
            .iter()
            .map(|slot| crate::trains::model::ColumnBinding {
                header: slot.header.clone(),
                index: slot.index,
                field: crate::trains::model::FieldKind::Ignorieren,
                decimal: None,
                date_order: None,
            })
            .collect(),
        template_id: None,
        date1904: false,
    };

    let file = path.to_string_lossy().into_owned();
    let staging_id = Uuid::new_v4().to_string();

    // A template is applied before anything is staged, so a known sender's file
    // arrives already mapped rather than mapped and then re-read.
    let plan = apply_template(state, &source, &plan).unwrap_or(plan);

    let staged = stage(StageInput {
        id: staging_id.clone(),
        file: file.clone(),
        sheets: source.sheets.clone(),
        candidates: candidates.clone(),
        grid: &source.grid,
        plan: &plan,
        db: &state.trains(),
    })?;

    let wire = staged.wire.clone();
    *state.staging() = Some(HeldImport {
        grid: source.grid,
        wire: staged.wire,
        values: staged.values,
    });
    Ok(TrainsData::nothing()
        .staging(wire)
        .counts(state.trains().counts()))
}

fn apply_template(
    state: &AppState,
    source: &grid::Source,
    plan: &ImportPlan,
) -> Option<ImportPlan> {
    let signature = fingerprint::of(&source.grid.sheet, &plan.columns);
    let db = state.trains();
    let template = db.template_by_fingerprint(&signature)?;
    let mut applied = template.plan.clone();
    applied.template_id = Some(template.id.clone());
    Some(applied)
}

fn stale() -> AppError {
    AppError::Report(vec![
        "Die Vorschau ist nicht mehr aktuell.".into(),
        "Bitte den Import erneut starten.".into(),
    ])
}

fn pick_file(window: &tauri::WebviewWindow) -> Option<PathBuf> {
    window
        .dialog()
        .file()
        .set_title("Datei importieren")
        .add_filter("Excel", &["xlsx"])
        .set_parent(window)
        .blocking_pick_file()
        .and_then(|picked| picked.into_path().ok())
}
