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
//
// The guided import is a second held slot, `cleaning`, beside `staging`: the
// file being cleaned and the cleaned file being previewed are two different
// files, and one slot would have the second overwrite the first mid-flow.
// `write_clean` is where one becomes the other — it re-runs the cleaning with
// the decisions it was SENT rather than trusting the last report, refuses while
// anything is open (`clean::ready`), writes the copy and stages THAT with the
// confirmed plan. `commit_import` then teaches the plan's readings to its
// template — for a guided import and a manual one alike, since a reading the
// user set in the mapper is just as confirmed.
// ────────────────────────────────────────────────────────────────

use std::path::{Path, PathBuf};

use tauri::State;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::model::ClientReport;
use crate::picker;
use crate::state::AppState;
use crate::trains::db::TrainsDb;
use crate::trains::model::{
    CleanDecisions, CommitDecisions, ImportPlan, Partner, Radsatz, TrainsData, Wagen,
};
use crate::trains::sheet::grid;
use crate::trains::stage::{stage, HeldImport, StageInput};
use crate::trains::{clean, commit, export, recognise, scan};

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
    let Some(path) = picker::file(&window, "Datei importieren", None, Some(picker::EXCEL)) else {
        return Ok(TrainsData::nothing());
    };
    read_and_stage(&path, None, &state)
}

#[tauri::command(async)]
pub fn stage_import_path(path: String, state: State<'_, AppState>) -> AppResult<TrainsData> {
    read_and_stage(Path::new(&path), None, &state)
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
    let mut report = commit::commit(&mut db, &held.wire, &held.values, &decisions)?.report();

    if let Some(template_id) = &held.wire.plan.template_id {
        if let Some(line) = commit::learn(&mut db, template_id, &held.wire.plan)? {
            report.messages.push(line);
        }
    }
    Ok(everything(&db).report(report))
}

#[tauri::command(async)]
pub fn pick_import_folder(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
) -> AppResult<TrainsData> {
    Ok(match picker::folder(&window, "Ordner importieren") {
        Some(folder) => scanned(&[folder], &state),
        None => TrainsData::nothing(),
    })
}

#[tauri::command(async)]
pub fn pick_import_files(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
) -> AppResult<TrainsData> {
    Ok(
        match picker::files(&window, "Dateien importieren", Some(picker::EXCEL)) {
            Some(files) => scanned(&files, &state),
            None => TrainsData::nothing(),
        },
    )
}

#[tauri::command(async)]
pub fn scan_import_paths(paths: Vec<String>, state: State<'_, AppState>) -> AppResult<TrainsData> {
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    Ok(scanned(&paths, &state))
}

#[tauri::command(async)]
pub fn clean_file(
    path: String,
    sheet: String,
    template_id: String,
    state: State<'_, AppState>,
) -> AppResult<TrainsData> {
    let template = state
        .trains()
        .template(&template_id)
        .ok_or_else(|| AppError::Report(vec!["Die gewählte Vorlage gibt es nicht mehr.".into()]))?;
    let held = clean::open(Path::new(&path), &sheet, &template)?;
    let report = held.run(&CleanDecisions::default())?.report;
    *state.cleaning() = Some(held);
    Ok(TrainsData::nothing().cleaning(report))
}

#[tauri::command(async)]
pub fn reclean_file(
    decisions: CleanDecisions,
    state: State<'_, AppState>,
) -> AppResult<TrainsData> {
    let held = state.cleaning();
    let held = held.as_ref().ok_or_else(stale)?;
    Ok(TrainsData::nothing().cleaning(held.run(&decisions)?.report))
}

#[tauri::command(async)]
pub fn write_clean(decisions: CleanDecisions, state: State<'_, AppState>) -> AppResult<TrainsData> {
    let (cleaned, path, sheet) = {
        let held = state.cleaning();
        let held = held.as_ref().ok_or_else(stale)?;
        (
            held.run(&decisions)?,
            held.path.clone(),
            held.grid.sheet.clone(),
        )
    };
    clean::ready(&cleaned.report)?;

    let folder = state
        .config
        .output_path
        .join(format!("bereinigt-{}", crate::trains::clock::today_iso()));
    let written = clean::write::write(&path, &sheet, &cleaned.changes, &folder)?;

    let source = grid::read(&written, Some(&sheet))?;
    let staged = stage_source(&written, source, Vec::new(), cleaned.confirmed, &state)?;
    *state.cleaning() = None;

    Ok(staged.report(ClientReport {
        headline: "Bereinigte Datei wurde geschrieben".into(),
        messages: vec![
            format!("Datei wurde erstellt: {}", crate::doc::file_name(&written)),
            format!(
                "{} Änderung(en) im Blatt „{}“ protokolliert.",
                cleaned.changes.len(),
                clean::write::PROTOCOL
            ),
        ],
        message_folder: Some(folder.to_string_lossy().into_owned()),
    }))
}

#[tauri::command]
pub fn discard_clean(state: State<'_, AppState>) -> AppResult<TrainsData> {
    *state.cleaning() = None;
    Ok(TrainsData::nothing())
}

fn scanned(paths: &[PathBuf], state: &AppState) -> TrainsData {
    let templates = state.trains().templates();
    TrainsData::nothing().scan(scan::scan(paths, &templates))
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
    if crate::trains::builtin::is_builtin(&id) {
        return Err(AppError::Report(vec![
            "Mitgelieferte Vorlagen können nicht entfernt werden.".into(),
        ]));
    }
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
    let (plan, candidates) = recognise::detected(&source.grid)?;
    let plan = recognise::apply(&state.trains().templates(), &plan).unwrap_or(plan);
    stage_source(path, source, candidates, plan, state)
}

fn stage_source(
    path: &Path,
    source: grid::Source,
    candidates: Vec<crate::trains::sheet::layout::Candidate>,
    plan: ImportPlan,
    state: &AppState,
) -> AppResult<TrainsData> {
    let file = path.to_string_lossy().into_owned();
    let staging_id = Uuid::new_v4().to_string();

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

fn stale() -> AppError {
    AppError::Report(vec![
        "Die Vorschau ist nicht mehr aktuell.".into(),
        "Bitte den Import erneut starten.".into(),
    ])
}
