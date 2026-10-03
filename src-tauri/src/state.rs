// ─── why ────────────────────────────────────────────────────────
// What every command shares. It lives outside any workflow so the halves that
// use it — `filler::import`, `filler::export`, `trains::commit` — do not have to
// depend on an API surface to reach a store. That is also why this is the one
// shared module allowed to name a workflow's database: it is the composition
// root's handle, and naming both is its job.
//
// A poisoned lock means another command panicked mid-mutation. The store is
// still structurally fine — it is rebuilt from its JSON on the next start — so
// recovering beats taking the window down with a second panic.
//
// The staged import is held HERE rather than round-tripped through the renderer.
// Re-reading the file every time the user nudges a header row would re-parse a
// thousand rows for a click, and the typed values behind the preview are not the
// frontend's to hold: it gets the preview, and sends back decisions about it.
// Nothing here is written to disk, so a crash costs a preview, which is a
// re-pick.
//
// `HeldImport` itself lives in `trains::stage`, not here. The licence above is
// about the STORES, which two halves of a workflow reach independently; the held
// preview has exactly one reader, `trains::commands`, so describing its shape
// here would make the shared composition root name four trains types to no end.
// ────────────────────────────────────────────────────────────────

use std::sync::Mutex;

use crate::config::AppConfig;
use crate::filler::db::Database;
use crate::trains::clean::HeldClean;
use crate::trains::db::TrainsDb;
use crate::trains::stage::HeldImport;

pub struct AppState {
    pub config: AppConfig,
    pub db: Mutex<Database>,
    pub trains: Mutex<TrainsDb>,
    pub staging: Mutex<Option<HeldImport>>,
    pub cleaning: Mutex<Option<HeldClean>>,
}

impl AppState {
    pub fn db(&self) -> std::sync::MutexGuard<'_, Database> {
        self.db
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn trains(&self) -> std::sync::MutexGuard<'_, TrainsDb> {
        self.trains
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn staging(&self) -> std::sync::MutexGuard<'_, Option<HeldImport>> {
        self.staging
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn cleaning(&self) -> std::sync::MutexGuard<'_, Option<HeldClean>> {
        self.cleaning
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
