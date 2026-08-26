// ─── why ────────────────────────────────────────────────────────
// One command, belonging to no workflow: what the app is and where it keeps
// things. It sits at the top level beside `config` and `state` for that reason —
// filing it under `filler` would make the version a property of one workflow.
//
// It exists because the version used to reach the UI only on the welcome report
// riding `get_client_data`, and that report is gone: the load moved into a route
// resolver, which runs before any page can be listening. Asking for the version
// is now a question with an answer rather than a message that has to be caught.
//
// The PATHS are the point as much as the version. Running unknown software on a
// managed desktop has to be approved, and the questions asked are always the
// same three — what it writes, where, and whether it talks to anything. The
// first two are answerable only by the process that resolved them, because
// `AppConfig` is relative to the WORKING directory and so differs between
// `tauri dev` and an installed build.
//
// `info` is a free function over `AppConfig` and the command is one line over
// it, for the reason every other testable decision here is: a
// `State<'_, AppState>` cannot be built outside a running app, so anything left
// inside the command body is reachable only through Playwright.
//
// `CARGO_PKG_VERSION` is read at compile time from `Cargo.toml`, so there is no
// constant here to keep in step with a release.
// ────────────────────────────────────────────────────────────────

use serde::Serialize;
use tauri::State;

use crate::config::AppConfig;
use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub data_path: String,
    pub output_path: String,
    pub cache_path: String,
}

#[tauri::command]
pub fn app_info(state: State<'_, AppState>) -> AppInfo {
    info(&state.config)
}

fn info(config: &AppConfig) -> AppInfo {
    AppInfo {
        name: "npDokumentenhilfe".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        data_path: config.data_path.to_string_lossy().into_owned(),
        output_path: config.output_path.to_string_lossy().into_owned(),
        cache_path: config.cache_path.to_string_lossy().into_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TempDir;

    // The version is the crate's own, not a string anybody edits by hand — the
    // release checklist would otherwise have a third place in it.
    #[test]
    fn the_version_is_the_crate_version() {
        let temp = TempDir::new("about-version");
        let config = temp.state().config;
        assert_eq!(info(&config).version, env!("CARGO_PKG_VERSION"));
        assert!(!info(&config).version.is_empty());
    }

    // The RESOLVED paths, not the configured defaults: an approval asks where
    // this build writes, and dev and an installed build differ.
    #[test]
    fn the_paths_are_the_resolved_ones() {
        let temp = TempDir::new("about-paths");
        let config = temp.state().config;
        let reported = info(&config);
        assert_eq!(reported.data_path, config.data_path.to_string_lossy());
        assert_eq!(reported.output_path, config.output_path.to_string_lossy());
        assert_eq!(reported.cache_path, config.cache_path.to_string_lossy());
        assert!(reported.data_path.contains("about-paths"));
    }
}
