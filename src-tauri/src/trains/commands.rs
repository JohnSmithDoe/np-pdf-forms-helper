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
// Cleaning is a second held slot, `cleaning`, beside `staging`: the file being
// cleaned and a document being imported are different things, and one slot
// would have either overwrite the other. `clean_file` ADOPTS the original before
// reading it — copies it into the document's own folder, refusing bytes already
// owned — so what is cleaned is what is stored. `write_clean` is the cleaning's
// commit point: it re-runs the cleaning with the decisions it was SENT rather
// than trusting the last report, refuses while anything is open
// (`clean::ready`), writes the copy and its protocol, and files the record and
// what the template learned in ONE transaction. It stages nothing: importing is
// a separate, later act, started from the document by `stage_document`.
//
// `commit_document` takes one decision per ENTITY and lets `entities::expand`
// turn them into the per-row decisions `commit` checks. There is no other way
// in any more — `commit_import` went with the manual import, and the mapper's
// only exit is `save_template`, after which the file is cleaned like any other.
//
// Archiving HIDES a Dokument and deletes nothing. Every archive command answers
// with both lists, `dokumente` without the archive and `archiv` with only it,
// so which list a Dokument belongs to is decided here and never in Angular.
// ────────────────────────────────────────────────────────────────

use std::path::{Path, PathBuf};

use tauri::State;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::model::ClientReport;
use crate::picker;
use crate::state::AppState;
use crate::trains::db::TrainsDb;
use crate::trains::dokument::{self, Cleaning, Filed};
use crate::trains::model::{
    CleanDecisions, EntityDecisions, EntityRef, Farbe, Farben, ImportPlan, MasterExportRequest,
    MasterSettings, Partner, Radsatz, StagingOrigin, TrainsData, TrainsSettings, Vorhanden, Wagen,
};
use crate::trains::sheet::grid;
use crate::trains::stage::{stage, HeldImport, StageInput};
use crate::trains::{
    clean, clock, commit, detail, entities, export, farbe, master, recognise, scan, telematik,
    template,
};

fn everything(db: &TrainsDb) -> TrainsData {
    TrainsData::nothing()
        .wagen(db.wagen())
        .partner(db.partner())
        .radsaetze(db.radsaetze())
        .einbauten(db.einbauten())
        .zustand(db.zustand().clone())
        .markierungen(db.markierungen().clone())
        .templates(db.templates())
        .dokumente(db.dokumente())
        .settings(db.settings())
        .counts(db.counts())
        .master_import_run(db.master().import_run.clone())
        .master_file(db.master_file().clone())
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
            master: false,
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
pub fn stage_document(id: String, state: State<'_, AppState>) -> AppResult<TrainsData> {
    stage_owned(&id, &state)
}

fn stage_owned(id: &str, state: &AppState) -> AppResult<TrainsData> {
    let found = state.trains().dokument(id).cloned();
    let dokument =
        found.ok_or_else(|| AppError::Report(vec!["Das Dokument gibt es nicht mehr.".into()]))?;
    let cleaned = dokument::importable(&dokument)?;
    let protocol = match cleaned.parent() {
        Some(folder) => dokument::read_protocol(folder)?,
        None => Vec::new(),
    };
    let source = grid::read(&cleaned, Some(&dokument.sheet))?;

    let mut staged = stage(StageInput {
        id: Uuid::new_v4().to_string(),
        file: dokument.name.clone(),
        sheets: source.sheets.clone(),
        candidates: Vec::new(),
        grid: &source.grid,
        plan: &dokument.plan,
        db: &state.trains(),
        master: false,
    })?;
    staged.wire.origin = StagingOrigin::Dokument {
        id: dokument.id.clone(),
    };
    staged.wire.entities = Some(entities::group(&staged.wire, &protocol));

    let wire = staged.wire.clone();
    *state.staging() = Some(HeldImport {
        grid: source.grid,
        wire: staged.wire,
        values: staged.values,
        farben: Farben::default(),
    });
    Ok(TrainsData::nothing()
        .staging(wire)
        .counts(state.trains().counts()))
}

#[tauri::command(async)]
pub fn commit_document(
    decisions: EntityDecisions,
    state: State<'_, AppState>,
) -> AppResult<TrainsData> {
    commit_owned(&decisions, &state)
}

fn commit_owned(decisions: &EntityDecisions, state: &AppState) -> AppResult<TrainsData> {
    let mut held = state.staging();
    let staging = held.as_ref().ok_or_else(stale)?;
    if staging.wire.origin == StagingOrigin::Datei {
        return Err(AppError::Report(vec![
            "Importiert wird nur ein bereinigtes Dokument.".into(),
        ]));
    }
    let rows = entities::expand(&staging.wire, decisions);
    let mut db = state.trains();
    let report = commit::commit(&mut db, &staging.wire, &staging.values, &rows)?.report();
    if let StagingOrigin::Master { sheet } = &staging.wire.origin {
        farbe::merge_master(&mut db, &staging.farben)?;
        master::mirror::done(&mut db, sheet)?;
    }
    *held = None;
    Ok(everything(&db).report(report))
}

#[tauri::command]
pub fn save_template(name: String, state: State<'_, AppState>) -> AppResult<TrainsData> {
    let mut held = state.staging();
    let plan = held.as_ref().ok_or_else(stale)?.wire.plan.clone();
    let template = template::save(&name, &plan)?;
    let saved = template.name.clone();
    let mut db = state.trains();
    db.transaction(|tx| {
        tx.put_template(template);
        Ok(())
    })?;
    *held = None;
    Ok(TrainsData::nothing()
        .templates(db.templates())
        .report(ClientReport::headline(format!(
            "Vorlage „{saved}“ wurde gespeichert"
        ))))
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
    adopt_and_clean(Path::new(&path), &sheet, &template_id, &state)
}

fn adopt_and_clean(
    path: &Path,
    sheet: &str,
    template_id: &str,
    state: &AppState,
) -> AppResult<TrainsData> {
    let hash = dokument::hash_of(path)?;
    let (template, root, uic) = {
        let db = state.trains();
        if let Some(owned) = db.dokument_by_hash(&hash) {
            return Err(AppError::Report(vec![
                format!(
                    "„{}“ wurde bereits am {} bereinigt.",
                    crate::doc::file_name(path),
                    owned.bereinigt_am
                ),
                format!("Es liegt unter den Dokumenten als „{}“.", owned.name),
            ]));
        }
        let template = db.template(template_id).ok_or_else(|| {
            AppError::Report(vec!["Die gewählte Vorlage gibt es nicht mehr.".into()])
        })?;
        (template, db.dokumente_folder(), db.settings().wagennummer)
    };

    let adopted = dokument::adopt(path, &root)?;
    let opened = clean::open(&adopted.path, sheet, &template, uic)
        .and_then(|held| held.run(&CleanDecisions::default()).map(|run| (held, run)));
    let (held, run) = match opened {
        Ok(opened) => opened,
        Err(error) => {
            dokument::discard(&adopted.folder);
            return Err(error);
        }
    };

    let previous = state.cleaning().replace(Cleaning { adopted, held });
    if let Some(previous) = previous {
        dokument::discard(&previous.adopted.folder);
    }
    Ok(TrainsData::nothing().cleaning(run.report))
}

#[tauri::command(async)]
pub fn reclean_file(
    decisions: CleanDecisions,
    state: State<'_, AppState>,
) -> AppResult<TrainsData> {
    let cleaning = state.cleaning();
    let cleaning = cleaning.as_ref().ok_or_else(stale)?;
    Ok(TrainsData::nothing().cleaning(cleaning.held.run(&decisions)?.report))
}

#[tauri::command(async)]
pub fn write_clean(decisions: CleanDecisions, state: State<'_, AppState>) -> AppResult<TrainsData> {
    file_cleaned(&decisions, &state)
}

fn file_cleaned(decisions: &CleanDecisions, state: &AppState) -> AppResult<TrainsData> {
    let mut slot = state.cleaning();
    let cleaning = slot.as_ref().ok_or_else(stale)?;
    let cleaned = cleaning.held.run(decisions)?;
    clean::ready(&cleaned.report)?;

    let adopted = &cleaning.adopted;
    let sheet = cleaning.held.grid.sheet.clone();
    let written = clean::write::write(&adopted.path, &sheet, &cleaned.changes, &adopted.folder)?;

    let mut db = state.trains();
    let filed = dokument::write_protocol(&adopted.folder, &cleaned.changes)
        .and_then(|()| {
            dokument::record(
                adopted,
                Filed {
                    sheet: &sheet,
                    template_id: &cleaned.report.template_id,
                    template_name: &cleaned.report.template_name,
                    plan: cleaned.confirmed.clone(),
                    cleaned: &written,
                    summary: cleaned.report.summary,
                    stamp: &crate::trains::clock::today_iso(),
                },
            )
        })
        .and_then(|record| {
            let lesson = template::learned(&db, &cleaned.report.template_id, &cleaned.confirmed);
            let line = lesson.as_ref().map(|(_, line)| line.clone());
            db.transaction(|tx| {
                tx.put_dokument(record);
                if let Some((learned, _)) = lesson {
                    tx.put_template(learned);
                }
                Ok(())
            })
            .map(|()| line)
        });
    let learned = match filed {
        Ok(line) => line,
        Err(error) => {
            let _ = std::fs::remove_file(&written);
            let _ = std::fs::remove_file(adopted.folder.join(dokument::PROTOCOL_FILE));
            return Err(error);
        }
    };

    let mut messages = vec![format!(
        "{} Änderung(en) im Blatt „{}“ der bereinigten Datei protokolliert.",
        cleaned.changes.len(),
        clean::write::PROTOCOL
    )];
    messages.extend(learned);
    let name = adopted.name.clone();
    *slot = None;

    Ok(TrainsData::nothing()
        .dokumente(db.dokumente())
        .templates(db.templates())
        .counts(db.counts())
        .report(ClientReport {
            headline: format!("„{name}“ wurde bereinigt"),
            messages,
            message_folder: None,
        }))
}

#[tauri::command]
pub fn get_dokument_archiv(state: State<'_, AppState>) -> AppResult<TrainsData> {
    Ok(listing(&state.trains()))
}

#[tauri::command]
pub fn archive_dokument(id: String, state: State<'_, AppState>) -> AppResult<TrainsData> {
    archive(Some(&id), &state)
}

#[tauri::command]
pub fn archive_all_dokumente(state: State<'_, AppState>) -> AppResult<TrainsData> {
    archive(None, &state)
}

#[tauri::command]
pub fn restore_dokument(id: String, state: State<'_, AppState>) -> AppResult<TrainsData> {
    restore(&id, &state)
}

fn listing(db: &TrainsDb) -> TrainsData {
    TrainsData::nothing()
        .dokumente(db.dokumente())
        .archiv(db.archiv())
        .counts(db.counts())
}

fn dokument_gone() -> AppError {
    AppError::Report(vec!["Das Dokument gibt es nicht mehr.".into()])
}

fn archive(id: Option<&str>, state: &AppState) -> AppResult<TrainsData> {
    let mut db = state.trains();
    let (ids, name) = match id {
        Some(id) => {
            let dokument = db.dokument(id).ok_or_else(dokument_gone)?;
            (vec![dokument.id.clone()], Some(dokument.name.clone()))
        }
        None => (
            db.dokumente()
                .into_iter()
                .map(|dokument| dokument.id)
                .collect(),
            None,
        ),
    };
    let stamp = clock::today_iso();
    let archived = db.transaction(|tx| Ok(tx.archive(&ids, &stamp)))?;
    let headline = match (name, archived) {
        (Some(name), _) => format!("„{name}“ wurde archiviert"),
        (None, 0) => "Keine Dokumente zum Archivieren".to_string(),
        (None, 1) => "1 Dokument wurde archiviert".to_string(),
        (None, count) => format!("{count} Dokumente wurden archiviert"),
    };
    Ok(listing(&db).report(ClientReport::headline(headline)))
}

fn restore(id: &str, state: &AppState) -> AppResult<TrainsData> {
    let mut db = state.trains();
    let name = db.dokument(id).ok_or_else(dokument_gone)?.name.clone();
    db.transaction(|tx| Ok(tx.restore(id)))?;
    Ok(listing(&db).report(ClientReport::headline(format!(
        "„{name}“ wurde wiederhergestellt"
    ))))
}

#[tauri::command]
pub fn discard_clean(state: State<'_, AppState>) -> AppResult<TrainsData> {
    abandon_cleaning(&state);
    Ok(TrainsData::nothing())
}

fn abandon_cleaning(state: &AppState) {
    if let Some(cleaning) = state.cleaning().take() {
        dokument::discard(&cleaning.adopted.folder);
    }
}

fn scanned(paths: &[PathBuf], state: &AppState) -> TrainsData {
    let db = state.trains();
    let owned = |hash: &str| {
        db.dokument_by_hash(hash).map(|dokument| Vorhanden {
            dokument_id: dokument.id.clone(),
            bereinigt_am: dokument.bereinigt_am.clone(),
            importiert_am: dokument.importiert_am.clone(),
            archiviert_am: dokument.archiviert_am.clone(),
        })
    };
    TrainsData::nothing().scan(scan::scan(paths, &db.templates(), &owned))
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
pub fn save_trains_settings(
    settings: TrainsSettings,
    state: State<'_, AppState>,
) -> AppResult<TrainsData> {
    let mut db = state.trains();
    db.save_settings(settings)?;
    Ok(TrainsData::nothing()
        .settings(db.settings())
        .report(ClientReport::headline("Einstellungen wurden gespeichert")))
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
    *state.staging() = None;
    *state.cleaning() = None;
    db.reset()?;
    Ok(everything(&db).report(ClientReport::headline("Zugdaten wurden zurückgesetzt")))
}

#[tauri::command(async)]
pub fn create_trains_export(state: State<'_, AppState>) -> AppResult<TrainsData> {
    let folder = state
        .config
        .output_path
        .join(format!("trains-{}", crate::trains::clock::today_iso()));
    let run = export::run(&state.trains(), &folder)?;

    Ok(TrainsData::nothing().report(run.report(&folder)))
}

#[tauri::command(async)]
pub fn get_master(state: State<'_, AppState>) -> AppResult<TrainsData> {
    let mut db = state.trains();
    master::bindings::sync(&mut db, false)?;
    Ok(TrainsData::nothing().master(master::view(db.master())))
}

#[tauri::command(async)]
pub fn reset_master_bindings(state: State<'_, AppState>) -> AppResult<TrainsData> {
    let mut db = state.trains();
    master::bindings::sync(&mut db, true)?;
    Ok(TrainsData::nothing().master(master::view(db.master())))
}

#[tauri::command(async)]
pub fn save_master(settings: MasterSettings, state: State<'_, AppState>) -> AppResult<TrainsData> {
    let mut db = state.trains();
    let settings = MasterSettings {
        file: db.master().file.clone(),
        import_run: db.master().import_run.clone(),
        scan: db.master().scan.clone(),
        ..settings
    };
    db.save_master(settings.clone())?;
    Ok(TrainsData::nothing().master(master::view(&settings)))
}

#[tauri::command(async)]
pub fn get_master_sheet(sheet: String, state: State<'_, AppState>) -> AppResult<TrainsData> {
    let mut db = state.trains();
    Ok(TrainsData::nothing().master_sheet(master::sheet_view(&mut db, &sheet)))
}

#[tauri::command]
pub fn get_entity_detail(
    kind: EntityRef,
    id: String,
    state: State<'_, AppState>,
) -> AppResult<TrainsData> {
    let db = state.trains();
    Ok(TrainsData::nothing().entity_detail(detail::build(&db, kind, &id)?))
}

#[tauri::command]
pub fn set_farbe(
    kind: EntityRef,
    id: String,
    farbe: Option<Farbe>,
    state: State<'_, AppState>,
) -> AppResult<TrainsData> {
    let mut db = state.trains();
    farbe::set(&mut db, kind, &id, farbe)?;
    Ok(TrainsData::nothing().markierungen(db.markierungen().clone()))
}

#[tauri::command]
pub fn get_telematik(state: State<'_, AppState>) -> AppResult<TrainsData> {
    let db = state.trains();
    Ok(TrainsData::nothing().telematik(telematik::view(&db, clock::today())))
}

#[tauri::command(async)]
pub fn start_master_import(state: State<'_, AppState>) -> AppResult<TrainsData> {
    *state.staging() = None;
    let mut db = state.trains();
    master::mirror::start(&mut db, &crate::trains::clock::today_iso())?;
    Ok(everything(&db).master(master::view(db.master())))
}

#[tauri::command(async)]
pub fn import_master_all(state: State<'_, AppState>) -> AppResult<TrainsData> {
    *state.staging() = None;
    let mut db = state.trains();
    let report = master::import_all::run(&mut db, &crate::trains::clock::today_iso())?;
    Ok(everything(&db)
        .master(master::view(db.master()))
        .report(report))
}

#[tauri::command(async)]
pub fn stage_master_sheet(sheet: String, state: State<'_, AppState>) -> AppResult<TrainsData> {
    stage_master(&sheet, &state)
}

fn stage_master(sheet: &str, state: &AppState) -> AppResult<TrainsData> {
    let held = master::mirror::stage_sheet(&state.trains(), sheet)?;
    let wire = held.wire.clone();
    *state.staging() = Some(held);
    Ok(TrainsData::nothing()
        .staging(wire)
        .counts(state.trains().counts()))
}

#[tauri::command(async)]
pub fn open_master_export(id: String, state: State<'_, AppState>) -> AppResult<TrainsData> {
    let mut db = state.trains();
    let start = master::export::start(&mut db, &id)?;
    Ok(TrainsData::nothing().master_export_start(start))
}

#[tauri::command(async)]
pub fn preview_master_export(
    request: MasterExportRequest,
    state: State<'_, AppState>,
) -> AppResult<TrainsData> {
    let run = master::export::preview(&state.trains(), &request)?;
    Ok(TrainsData::nothing().master_export(run))
}

#[tauri::command(async)]
pub fn write_master_export(
    request: MasterExportRequest,
    state: State<'_, AppState>,
) -> AppResult<TrainsData> {
    let mut db = state.trains();
    let run = master::export::write(&mut db, &request, &crate::trains::clock::now_stamp())?;
    Ok(listing(&db)
        .master_export(run)
        .master(master::view(db.master()))
        .master_file(db.master_file().clone()))
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
        master: false,
    })?;

    let wire = staged.wire.clone();
    *state.staging() = Some(HeldImport {
        grid: source.grid,
        wire: staged.wire,
        values: staged.values,
        farben: Farben::default(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{workbook, TempDir};
    use crate::trains::model::{
        ColumnBinding, EntityChoice, EntityDecision, FieldKind, ImportPlan, ScanStatus,
    };
    use crate::trains::sheet::layout::LayoutHint;
    use crate::trains::sheet::readers::ReaderKind;

    const HEADERS: [&str; 4] = ["Wagennummer", "Datum", "Werkstatt", "Leistung"];

    fn monatsliste(state: &AppState) -> String {
        let plan = ImportPlan {
            reader: ReaderKind::HeaderRow,
            layout: LayoutHint {
                header_row: Some(1),
                first_data_row: 2,
                last_data_row: None,
            },
            columns: [
                FieldKind::Wagennummer,
                FieldKind::Datum,
                FieldKind::Werkstatt,
                FieldKind::Leistung,
            ]
            .into_iter()
            .zip(HEADERS)
            .enumerate()
            .map(|(position, (field, header))| ColumnBinding {
                header: header.into(),
                index: position as u32 + 1,
                field,
                decimal: None,
                date_order: None,
            })
            .collect(),
            template_id: None,
            date1904: false,
            pruefart: None,
        };
        let saved = template::save("Monatsliste", &plan).unwrap();
        let id = saved.id.clone();
        state
            .trains()
            .transaction(|tx| {
                tx.put_template(saved);
                Ok(())
            })
            .unwrap();
        id
    }

    fn sender_file(folder: &TempDir) -> PathBuf {
        workbook(
            folder,
            "monat.xlsx",
            &[(
                "Tabelle1",
                &[
                    &HEADERS,
                    &[
                        "218124712173",
                        "31.12.2025",
                        "Schienenbein Waggonwerk GmbH",
                        "Bremsprobe",
                    ],
                ],
            )],
        )
    }

    fn owned_folders(state: &AppState) -> usize {
        std::fs::read_dir(state.trains().dokumente_folder())
            .map(|entries| entries.count())
            .unwrap_or(0)
    }

    /// The whole life of a file: adopted, cleaned, filed — then, separately,
    /// staged per entity and imported, after which it is final.
    #[test]
    fn a_file_is_cleaned_into_a_document_and_imported_once() {
        let folder = TempDir::new("cmd-flow");
        let state = folder.state();
        let template = monatsliste(&state);
        let file = sender_file(&folder);

        let opened = adopt_and_clean(&file, "Tabelle1", &template, &state).unwrap();
        assert!(opened.cleaning.is_some());
        let filed = file_cleaned(&CleanDecisions::default(), &state).unwrap();
        assert!(state.cleaning().is_none());

        let dokument = filed.dokumente.unwrap().remove(0);
        assert_eq!(dokument.name, "monat.xlsx");
        assert_eq!(dokument.original_hash, dokument::hash_of(&file).unwrap());
        assert!(Path::new(&dokument.folder).is_dir());
        assert!(Path::new(&dokument.original).starts_with(&dokument.folder));
        assert!(Path::new(&dokument.original).is_file());
        assert!(Path::new(&dokument.cleaned).is_file());
        assert!(Path::new(&dokument.cleaned)
            .with_file_name(dokument::PROTOCOL_FILE)
            .is_file());
        assert_eq!(dokument.importiert_am, None);

        // Filing staged nothing: importing is a separate act.
        assert!(state.staging().is_none());

        let staged = stage_owned(&dokument.id, &state).unwrap().staging.unwrap();
        let groups = staged.entities.unwrap();
        assert_eq!(groups.wagen.len(), 1);
        assert_eq!(groups.partner.len(), 1);
        assert!(
            !groups.wagen[0].changes.is_empty(),
            "the Wagennummer's reformatting is shown on its group"
        );

        let decisions = EntityDecisions {
            staging_id: staged.id.clone(),
            partner: vec![EntityChoice {
                key: groups.partner[0].key.clone(),
                decision: EntityDecision::Create,
            }],
            wagen: vec![EntityChoice {
                key: groups.wagen[0].key.clone(),
                decision: EntityDecision::Create,
            }],
            radsaetze: Vec::new(),
            einbauten: Vec::new(),
            rows: vec![2],
        };
        let committed = commit_owned(&decisions, &state).unwrap();
        assert_eq!(committed.counts.unwrap().instandhaltungen, 1);
        assert!(state
            .trains()
            .dokument(&dokument.id)
            .unwrap()
            .importiert_am
            .is_some());

        let again = stage_owned(&dokument.id, &state).unwrap_err();
        assert!(again.into_messages()[0].contains("bereits"));
    }

    #[test]
    fn bytes_already_owned_are_refused_before_anything_is_copied() {
        let folder = TempDir::new("cmd-twice");
        let state = folder.state();
        let template = monatsliste(&state);
        let file = sender_file(&folder);
        adopt_and_clean(&file, "Tabelle1", &template, &state).unwrap();
        file_cleaned(&CleanDecisions::default(), &state).unwrap();

        let copy = folder.join("monat - Kopie.xlsx");
        std::fs::copy(&file, &copy).unwrap();
        let error = adopt_and_clean(&copy, "Tabelle1", &template, &state).unwrap_err();
        assert!(error.into_messages()[0].contains("bereits"));
        assert_eq!(owned_folders(&state), 1);
    }

    /// A cleaning the user walks away from leaves no folder, no record and
    /// nothing learned.
    #[test]
    fn an_abandoned_cleaning_leaves_nothing_behind() {
        let folder = TempDir::new("cmd-abandon");
        let state = folder.state();
        let template = monatsliste(&state);
        adopt_and_clean(&sender_file(&folder), "Tabelle1", &template, &state).unwrap();
        assert_eq!(owned_folders(&state), 1);

        abandon_cleaning(&state);
        assert_eq!(owned_folders(&state), 0);
        assert_eq!(state.trains().counts().dokumente, 0);
    }

    #[test]
    fn a_staging_from_the_mapper_cannot_be_imported() {
        let folder = TempDir::new("cmd-mapper");
        let state = folder.state();
        read_and_stage(&sender_file(&folder), None, &state).unwrap();
        let staging_id = state.staging().as_ref().unwrap().wire.id.clone();
        let error = commit_owned(
            &EntityDecisions {
                staging_id,
                partner: Vec::new(),
                wagen: Vec::new(),
                radsaetze: Vec::new(),
                einbauten: Vec::new(),
                rows: vec![2],
            },
            &state,
        )
        .unwrap_err();
        assert!(error.into_messages()[0].contains("bereinigtes Dokument"));
        assert_eq!(state.trains().counts().instandhaltungen, 0);
    }

    #[test]
    fn an_archived_document_leaves_the_list_but_keeps_its_bytes_owned() {
        let folder = TempDir::new("cmd-archive");
        let state = folder.state();
        let template = monatsliste(&state);
        let file = sender_file(&folder);
        adopt_and_clean(&file, "Tabelle1", &template, &state).unwrap();
        let filed = file_cleaned(&CleanDecisions::default(), &state).unwrap();
        let dokument = filed.dokumente.unwrap().remove(0);

        let archived = archive(Some(&dokument.id), &state).unwrap();
        assert_eq!(archived.dokumente.unwrap().len(), 0);
        assert_eq!(archived.archiv.unwrap()[0].id, dokument.id);
        assert_eq!(archived.counts.unwrap().dokumente, 0);
        assert!(archived.message.unwrap().headline.contains("archiviert"));
        assert!(Path::new(&dokument.folder).is_dir(), "hidden, not deleted");

        // The same bytes dropped again are still `Vorhanden` — and say why
        // the user will not find them in the list.
        let scan = scanned(std::slice::from_ref(&file), &state).scan.unwrap();
        assert_eq!(scan[0].status, ScanStatus::Vorhanden);
        assert!(scan[0]
            .message
            .as_deref()
            .unwrap()
            .contains("Archiviert am"));
        let error = adopt_and_clean(&file, "Tabelle1", &template, &state).unwrap_err();
        assert!(error.into_messages()[0].contains("bereits"));
        assert_eq!(owned_folders(&state), 1);

        let restored = restore(&dokument.id, &state).unwrap();
        assert_eq!(restored.dokumente.unwrap()[0].id, dokument.id);
        assert!(restored.archiv.unwrap().is_empty());
    }

    #[test]
    fn archiving_all_takes_every_listed_document() {
        let folder = TempDir::new("cmd-archive-all");
        let state = folder.state();
        let template = monatsliste(&state);
        adopt_and_clean(&sender_file(&folder), "Tabelle1", &template, &state).unwrap();
        file_cleaned(&CleanDecisions::default(), &state).unwrap();

        let all = archive(None, &state).unwrap();
        assert!(all.dokumente.unwrap().is_empty());
        assert_eq!(all.archiv.unwrap().len(), 1);
        assert_eq!(all.message.unwrap().headline, "1 Dokument wurde archiviert");

        let nothing = archive(None, &state).unwrap();
        assert_eq!(
            nothing.message.unwrap().headline,
            "Keine Dokumente zum Archivieren"
        );
    }

    #[test]
    fn archiving_an_unknown_document_is_an_error_not_a_silent_no_op() {
        let folder = TempDir::new("cmd-archive-gone");
        let state = folder.state();
        assert!(archive(Some("nope"), &state).is_err());
        assert!(restore("nope", &state).is_err());
    }
}
