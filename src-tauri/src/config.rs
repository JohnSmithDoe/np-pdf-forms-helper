// ─── why ────────────────────────────────────────────────────────
// Resolution order per key: `.npconfig` → `APP_*` env var → default under
// `./data`. The base is the process WORKING directory, not the executable's —
// installs put `.npconfig` and `data/` beside the exe and launch it from there,
// so resolving anywhere else would orphan their data.
//
// `DB_FILE` and `PROFILE_FILE` read their OWN env vars (`APP_DB_FILE` /
// `APP_PROFILE_FILE`); one shared variable would aim both databases at one file.
// Unknown `.npconfig` keys are ignored, so a config file carrying keys this
// build does not know still loads.
//
// There is no `MASTER_FILE` any more. The trains master is picked in the app and
// stored with the trains settings: the app only ever writes a dated COPY beside
// it, so nobody can discover their file being edited by watching it change, and
// a Citrix desk has no comfortable way to edit `.npconfig`. An old config that
// still names the key loads, because unknown keys are ignored.
// ────────────────────────────────────────────────────────────────

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{AppError, AppResult};

const CONFIG_FILE: &str = ".npconfig";

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", default)]
struct ConfigFile {
    data_path: Option<String>,
    cache_path: Option<String>,
    output_path: Option<String>,
    db_file: Option<String>,
    profile_file: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub data_path: PathBuf,
    pub cache_path: PathBuf,
    pub output_path: PathBuf,
    pub db_file: PathBuf,
    pub profile_file: PathBuf,
}

impl AppConfig {
    pub fn load() -> AppResult<Self> {
        let base = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let file = read_config_file(&base.join(CONFIG_FILE));
        let config = Self::resolve_all(&base, file, |key| std::env::var(key).ok());
        config.ensure_folders()?;
        Ok(config)
    }

    // The precedence rule itself, with the two things it reads passed IN: the
    // base folder and a lookup for the environment. Both are process-global at
    // the call site above — `current_dir` and `std::env::var` — and `cargo test`
    // runs tests in parallel threads of ONE process, so a test that set either
    // would be visible to every other test running at that moment.
    fn resolve_all(base: &Path, file: ConfigFile, env: impl Fn(&str) -> Option<String>) -> Self {
        let data_path = resolve(file.data_path, "APP_DATA", &env, || base.join("data"));
        Self {
            cache_path: resolve(file.cache_path, "APP_CACHE", &env, || {
                data_path.join("cache")
            }),
            output_path: resolve(file.output_path, "APP_OUTPUT", &env, || {
                data_path.join("out")
            }),
            db_file: resolve(file.db_file, "APP_DB_FILE", &env, || {
                data_path.join("data.db")
            }),
            profile_file: resolve(file.profile_file, "APP_PROFILE_FILE", &env, || {
                data_path.join("profiles.db")
            }),
            data_path,
        }
    }

    fn ensure_folders(&self) -> AppResult<()> {
        for folder in [&self.data_path, &self.cache_path, &self.output_path] {
            std::fs::create_dir_all(folder).map_err(|source| AppError::io(folder, source))?;
        }
        Ok(())
    }
}

// A `.npconfig` that is missing or unreadable is not an error — the app has a
// full set of defaults and the file is optional. A CORRUPT one is worth a line
// on stderr, because the user's overrides are being silently ignored.
fn read_config_file(path: &Path) -> ConfigFile {
    let Ok(content) = std::fs::read_to_string(path) else {
        return ConfigFile::default();
    };
    parse_config_file(&content, path)
}

fn parse_config_file(content: &str, path: &Path) -> ConfigFile {
    serde_json::from_str(content).unwrap_or_else(|error| {
        eprintln!(
            "{} ist beschädigt und wird ignoriert: {error}",
            path.display()
        );
        ConfigFile::default()
    })
}

// `||` in the original treats "" as unset, so an empty override must not win —
// and must not swallow the NEXT source either: an empty `.npconfig` value falls
// through to the env var, exactly as `file || env || default` did.
fn resolve(
    from_file: Option<String>,
    env_key: &str,
    env: impl Fn(&str) -> Option<String>,
    fallback: impl FnOnce() -> PathBuf,
) -> PathBuf {
    from_file
        .filter(|value| !value.is_empty())
        .or_else(|| env(env_key))
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(fallback)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "/opt/npdh";

    fn no_env(_: &str) -> Option<String> {
        None
    }

    fn parse(content: &str) -> ConfigFile {
        parse_config_file(content, Path::new(".npconfig"))
    }

    fn resolved(content: &str, env: impl Fn(&str) -> Option<String>) -> AppConfig {
        AppConfig::resolve_all(Path::new(BASE), parse(content), env)
    }

    // ─── the defaults ─────────────────────────────────────────────

    #[test]
    fn everything_defaults_under_data_beside_the_working_directory() {
        let config = resolved("{}", no_env);
        assert_eq!(config.data_path, Path::new("/opt/npdh/data"));
        assert_eq!(config.cache_path, Path::new("/opt/npdh/data/cache"));
        assert_eq!(config.output_path, Path::new("/opt/npdh/data/out"));
        assert_eq!(config.db_file, Path::new("/opt/npdh/data/data.db"));
        assert_eq!(config.profile_file, Path::new("/opt/npdh/data/profiles.db"));
    }

    // `data_path` is resolved FIRST and the rest default beneath it, so moving
    // the data folder moves everything that was not named separately.
    #[test]
    fn moving_the_data_path_moves_every_default_beneath_it() {
        let config = resolved(r#"{ "DATA_PATH": "/mnt/share" }"#, no_env);
        assert_eq!(config.cache_path, Path::new("/mnt/share/cache"));
        assert_eq!(config.output_path, Path::new("/mnt/share/out"));
        assert_eq!(config.db_file, Path::new("/mnt/share/data.db"));
        assert_eq!(config.profile_file, Path::new("/mnt/share/profiles.db"));
    }

    // ─── the precedence ladder ────────────────────────────────────

    #[test]
    fn the_file_beats_the_environment() {
        let config = resolved(r#"{ "OUTPUT_PATH": "/aus/datei" }"#, |_| {
            Some("/aus/env".into())
        });
        assert_eq!(config.output_path, Path::new("/aus/datei"));
    }

    #[test]
    fn the_environment_beats_the_default() {
        let config = resolved("{}", |key| (key == "APP_OUTPUT").then(|| "/aus/env".into()));
        assert_eq!(config.output_path, Path::new("/aus/env"));
        assert_eq!(config.cache_path, Path::new("/opt/npdh/data/cache"));
    }

    // `||` in the original JavaScript treated "" as unset, so an empty override
    // must not win — and must not swallow the next source either.
    #[test]
    fn an_empty_file_value_falls_through_to_the_environment() {
        let config = resolved(r#"{ "OUTPUT_PATH": "" }"#, |_| Some("/aus/env".into()));
        assert_eq!(config.output_path, Path::new("/aus/env"));
    }

    #[test]
    fn an_empty_environment_value_falls_through_to_the_default() {
        let config = resolved(r#"{ "OUTPUT_PATH": "" }"#, |_| Some(String::new()));
        assert_eq!(config.output_path, Path::new("/opt/npdh/data/out"));
    }

    // THE documented bug: one shared `APP_CONFIG` aimed both databases at one
    // file. Each key reads its own variable.
    #[test]
    fn the_two_databases_read_their_own_environment_keys() {
        let config = resolved("{}", |key| match key {
            "APP_DB_FILE" => Some("/mnt/dokumente.json".into()),
            "APP_PROFILE_FILE" => Some("/mnt/profile.json".into()),
            _ => None,
        });
        assert_eq!(config.db_file, Path::new("/mnt/dokumente.json"));
        assert_eq!(config.profile_file, Path::new("/mnt/profile.json"));
    }

    #[test]
    fn the_two_databases_read_their_own_file_keys() {
        let config = resolved(
            r#"{ "DB_FILE": "/mnt/dokumente.json", "PROFILE_FILE": "/mnt/profile.json" }"#,
            no_env,
        );
        assert_eq!(config.db_file, Path::new("/mnt/dokumente.json"));
        assert_eq!(config.profile_file, Path::new("/mnt/profile.json"));
    }

    // ─── reading the file ─────────────────────────────────────────

    // An old config carrying keys this build no longer knows — `PDFTK_EXE`,
    // `ENCODING` — must still load rather than being treated as corrupt.
    #[test]
    fn unknown_keys_are_ignored() {
        let file = parse(r#"{ "PDFTK_EXE": "C:\\pdftk.exe", "ENCODING": "latin1" }"#);
        assert!(file.data_path.is_none());

        let config = resolved(r#"{ "ENCODING": "latin1", "OUTPUT_PATH": "/aus" }"#, no_env);
        assert_eq!(config.output_path, Path::new("/aus"));
    }

    // A corrupt `.npconfig` falls back to the full set of defaults rather than
    // stopping the app — the overrides are lost, and stderr says so.
    #[test]
    fn a_corrupt_file_falls_back_to_the_defaults() {
        let config = AppConfig::resolve_all(Path::new(BASE), parse("{kaputt"), no_env);
        assert_eq!(config.data_path, Path::new("/opt/npdh/data"));
    }

    #[test]
    fn a_missing_file_is_not_an_error() {
        let file = read_config_file(Path::new("/nirgends/.npconfig"));
        assert!(file.data_path.is_none());
        assert!(file.db_file.is_none());
    }

    // The keys are SCREAMING_SNAKE_CASE on disk. A camelCase spelling is simply
    // an unknown key, not an alternative.
    #[test]
    fn the_file_keys_are_screaming_snake_case() {
        assert!(parse(r#"{ "dataPath": "/mnt" }"#).data_path.is_none());
        assert_eq!(
            parse(r#"{ "DATA_PATH": "/mnt" }"#).data_path.as_deref(),
            Some("/mnt")
        );
    }
}
