// ─── why ────────────────────────────────────────────────────────
// Every message here is user-facing GERMAN and is shown verbatim in a dialog.
// A command returns `Result<T, AppError>` and Tauri serialises the error as
// `{ "messages": [...] }`, so a line break is structure rather than punctuation
// nobody may type into a message.
//
// Nothing swallows an `io::Error`: `?` makes a failed save loud, because a save
// that reports success and loses the data is the worse failure.
//
// `AppError::reading` is the boundary around a PARSER this app does not own.
// umya 3.0.1 panicked on shared formulas with whole-column ranges (`A:A`) — a
// real customer workbook, every sheet — and a panic is not an `AppError`: the
// command never answers and the window keeps spinning. So every umya and lopdf
// read runs inside `catch_unwind`, and a panic becomes the same German headline
// a parse error gets, with the panic text as the cause line.
//
// `AssertUnwindSafe` is honest here: the closure only builds a value from a file,
// and on a panic that value is dropped, so no half-mutated state escapes. It
// relies on the default `panic = "unwind"` — a `panic = "abort"` in a Cargo
// profile would silently turn this boundary back into a crash.
// ────────────────────────────────────────────────────────────────

use std::fmt::Display;
use std::panic::{self, AssertUnwindSafe};
use std::path::PathBuf;

use serde::{ser::SerializeStruct, Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Die gewählte Datei konnte nicht gelesen werden. Bitte wenden Sie sich an Ihren persönlichen Ansprechpartner für IT-Probleme.")]
    FileUnreadable,

    #[error("Dieses Dokument existiert bereits. Bitte keine doppelten Dokumente anlegen. Bei Problemen entferne die alte Vorlage und beginne von vorne.")]
    DocumentExists,

    #[error("Dieses Dokument existiert nicht mehr. Bitte wenden Sie sich an Ihren persönlichen Ansprechpartner für IT-Probleme.")]
    DocumentMissing,

    #[error("Es kann hier nur eine Änderung des Dateinamens vorgenommen werden. Eine Änderung des Vorlagen-Typs geht nur über löschen und neu anlegen.")]
    DocumentTypeChanged,

    #[error("Die Datei {path} konnte nicht gelesen oder geschrieben werden. ({source})")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Die Datei {path} ist beschädigt und konnte nicht gelesen werden. ({source})")]
    Json {
        path: String,
        #[source]
        source: serde_json::Error,
    },

    /// Already user-ready lines — the export path collects them as it goes.
    #[error("{}", .0.join(" "))]
    Report(Vec<String>),
}

impl AppError {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into().display().to_string(),
            source,
        }
    }

    pub fn json(path: impl Into<PathBuf>, source: serde_json::Error) -> Self {
        Self::Json {
            path: path.into().display().to_string(),
            source,
        }
    }

    /// A German sentence the caller owns, plus the cause underneath it. The two
    /// are separate lines because the dialog renders one entry per line.
    pub fn detail(headline: String, cause: impl std::fmt::Display) -> Self {
        Self::Report(vec![headline, cause.to_string()])
    }

    pub fn reading<T, E: Display>(
        headline: String,
        read: impl FnOnce() -> Result<T, E>,
    ) -> AppResult<T> {
        match panic::catch_unwind(AssertUnwindSafe(read)) {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(error)) => Err(Self::detail(headline, error)),
            Err(payload) => {
                let cause = payload
                    .downcast_ref::<&str>()
                    .map(|text| text.to_string())
                    .or_else(|| payload.downcast_ref::<String>().cloned())
                    .unwrap_or_default();
                Err(Self::detail(
                    headline,
                    format!("Interner Fehler beim Lesen der Datei. {cause}"),
                ))
            }
        }
    }

    fn messages(&self) -> Vec<String> {
        match self {
            Self::Report(lines) => lines.clone(),
            other => vec![other.to_string()],
        }
    }

    /// For the export report, which folds a per-document failure into the lines
    /// it is already collecting instead of failing the whole run.
    pub fn into_messages(self) -> Vec<String> {
        self.messages()
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut error = serializer.serialize_struct("AppError", 1)?;
        error.serialize_field("messages", &self.messages())?;
        error.end()
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn io_error() -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Zugriff verweigert")
    }

    // Tauri serialises the error as `{ "messages": [...] }` and the renderer
    // unwraps exactly that shape into a `BackendError` — nothing else.
    #[test]
    fn every_variant_serialises_as_a_messages_object() {
        let value = serde_json::to_value(AppError::DocumentMissing).unwrap();
        assert_eq!(value["messages"].as_array().unwrap().len(), 1);
        assert_eq!(value.as_object().unwrap().len(), 1);
    }

    #[test]
    fn a_report_keeps_its_lines_separate() {
        let error = AppError::Report(vec!["Erste Zeile".into(), "Zweite Zeile".into()]);
        assert_eq!(
            serde_json::to_value(error).unwrap(),
            json!({ "messages": ["Erste Zeile", "Zweite Zeile"] })
        );
    }

    // A line break is STRUCTURE, not punctuation somebody may type into a
    // message: the dialog renders one entry per line.
    #[test]
    fn detail_is_a_headline_plus_the_cause_underneath() {
        let error = AppError::detail("Die Datei konnte nicht gelesen werden.".into(), io_error());
        let messages = error.into_messages();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0], "Die Datei konnte nicht gelesen werden.");
        assert!(
            messages[1].contains("Zugriff verweigert"),
            "{}",
            messages[1]
        );
    }

    #[test]
    fn an_io_error_names_the_path_and_the_cause_in_one_line() {
        let error = AppError::io("/tmp/data.db", io_error());
        let messages = error.into_messages();
        assert_eq!(messages.len(), 1);
        assert!(messages[0].contains("/tmp/data.db"), "{}", messages[0]);
        assert!(
            messages[0].contains("Zugriff verweigert"),
            "{}",
            messages[0]
        );
    }

    #[test]
    fn a_json_error_names_the_path_and_says_the_file_is_damaged() {
        let source = serde_json::from_str::<serde_json::Value>("{kaputt").unwrap_err();
        let messages = AppError::json("/tmp/data.db", source).into_messages();
        assert_eq!(messages.len(), 1);
        assert!(messages[0].contains("/tmp/data.db"), "{}", messages[0]);
        assert!(messages[0].contains("beschädigt"), "{}", messages[0]);
    }

    // The panic is the case the guard exists for: umya 3.0.1 unwrapped a `None`
    // inside its formula parser. The default hook still prints the panic to
    // stderr — installing a quiet one would be process-wide, and tests share a
    // process.
    #[test]
    fn a_panicking_reader_becomes_the_headline_plus_the_panic_text() {
        let error = AppError::reading("Die Datei konnte nicht gelesen werden.".into(), || {
            panic!("Formel ohne Zeile");
            #[allow(unreachable_code)]
            Ok::<u32, std::io::Error>(0)
        })
        .unwrap_err();
        let messages = error.into_messages();
        assert_eq!(messages[0], "Die Datei konnte nicht gelesen werden.");
        assert!(messages[1].contains("Formel ohne Zeile"), "{}", messages[1]);
    }

    #[test]
    fn a_reader_error_reads_like_detail_and_a_value_passes_through() {
        let error = AppError::reading("Kopfzeile".into(), || Err::<(), _>(io_error())).unwrap_err();
        assert_eq!(
            error.into_messages(),
            vec!["Kopfzeile".to_string(), "Zugriff verweigert".to_string()]
        );
        let value = AppError::reading("Kopfzeile".into(), || Ok::<_, std::io::Error>(7));
        assert_eq!(value.unwrap(), 7);
    }

    // Every message is user-facing GERMAN and shown verbatim, so an empty one
    // would reach the user as a blank dialog.
    #[test]
    fn no_variant_is_silent() {
        for error in [
            AppError::FileUnreadable,
            AppError::DocumentExists,
            AppError::DocumentMissing,
            AppError::DocumentTypeChanged,
            AppError::io("/tmp/a", io_error()),
        ] {
            let messages = error.into_messages();
            assert!(!messages.is_empty());
            assert!(messages.iter().all(|line| !line.trim().is_empty()));
        }
    }
}
