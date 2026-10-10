// ─── why ────────────────────────────────────────────────────────
// Filling and writing a run of documents: the whole of what `create_documents`
// does apart from answering the frontend. No Tauri in it.
//
// One document failing must not abandon the rest, so a failure becomes lines in
// the report instead of an early return. Only an unusable output folder stops
// the run.
// ────────────────────────────────────────────────────────────────

use std::path::Path;

use crate::doc;
use crate::error::{AppError, AppResult};
use crate::model::{ClientReport, MappedDocument, MappedInput};

/// What a run produced. `refreshed` carries the documents whose original had a
/// newer timestamp than the store knew — the caller persists them.
#[derive(Debug)]
pub struct Run {
    pub messages: Vec<String>,
    pub failed: usize,
    pub refreshed: Vec<MappedDocument>,
}

impl Run {
    /// The headline carries the outcome: a run that lost a document must not be
    /// announced as a success and contradicted by a line further down. It lives
    /// here rather than in the command because it reads `failed`, which only
    /// this module knows how to count.
    pub fn report(&self, output_folder: &Path) -> ClientReport {
        ClientReport {
            headline: if self.failed == 0 {
                "Dokumente wurden erfolgreich erstellt".into()
            } else {
                "Dokumente wurden mit Fehlern erstellt".into()
            },
            messages: self.messages.clone(),
            message_folder: Some(output_folder.to_string_lossy().into_owned()),
        }
    }
}

pub fn run(
    output_folder: &Path,
    documents: Vec<MappedDocument>,
    inputs: &[MappedInput],
) -> AppResult<Run> {
    let mut messages = create_output_folder(output_folder)?;
    let mut refreshed = Vec::new();
    let mut failed = 0;

    for document in documents {
        match doc::create(&document, output_folder, inputs, &mut messages) {
            Ok(export) => {
                messages.push(format!("Dokument wurden erstellt: {}", export.basename));
                if export.mtime != document.mtime {
                    refreshed.push(MappedDocument {
                        mtime: export.mtime,
                        ..document
                    });
                }
            }
            Err(error) => {
                failed += 1;
                report_failure(&document, inputs, error, &mut messages);
            }
        }
    }

    messages.push(if failed == 0 {
        "Alle Dokumente wurden erfolgreich erstellt.".into()
    } else {
        format!(
            "{failed} Dokument(e) konnten nicht erstellt werden. Bitte die Meldungen oben prüfen."
        )
    });
    Ok(Run {
        messages,
        failed,
        refreshed,
    })
}

// Refusing to write into a folder that already exists is the app's only guard
// against overwriting an earlier export — the folder name ends in a suffix the
// user types, and typing the same one twice is easy.
//
// The advice names a SECOND, because that is the resolution of the name the
// renderer builds (`run-folder.utility.ts`): only a second run inside the same
// second can collide on the timestamp alone.
fn create_output_folder(folder: &Path) -> AppResult<Vec<String>> {
    if folder.exists() {
        return Err(AppError::Report(vec![
            format!("Ordner existierte bereits: {}", folder.display()),
            "Erstellen wurde abgebrochen damit keine Daten überschrieben werden.".into(),
            "Bitte warte einen Moment, lösche den Ordner oder verwende ein anderes Suffix.".into(),
        ]));
    }
    std::fs::create_dir_all(folder).map_err(|error| AppError::io(folder, error))?;
    Ok(vec![format!("Ordner wurde erstellt: {}", folder.display())])
}

fn report_failure(
    document: &MappedDocument,
    inputs: &[MappedInput],
    error: AppError,
    messages: &mut Vec<String>,
) {
    messages.push(format!(
        "Beim erstellen von: {} ist ein Fehler aufgetreten.",
        document.name
    ));
    messages.extend(error.into_messages());
    messages.push(format!(
        "Folgende Felder wurden erwartet: {}",
        document
            .mapped_fields()
            .iter()
            .map(|field| field.mapped_name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    ));
    messages.push(format!(
        "Debug Info:{}",
        inputs
            .iter()
            .map(|input| input.identifiers.join(", "))
            .collect::<Vec<_>>()
            .join("; ")
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{input, pdf_document, resource_document, TempDir};

    /// A resource document whose stored mtime already matches its file, so the
    /// "original was changed" warning stays out of the way.
    fn linked(folder: &TempDir, id: &str, name: &str) -> MappedDocument {
        let path = folder.write(name, "inhalt");
        let mut document = resource_document(id, &path);
        document.mtime = crate::doc::file_mtime_ms(&path).unwrap();
        document
    }

    fn joined(messages: &[String]) -> String {
        messages.join("\n")
    }

    // ─── the output folder ────────────────────────────────────────

    // Refusing to write into a folder that already exists is the app's only
    // guard against overwriting an earlier export.
    #[test]
    fn an_existing_output_folder_stops_the_run() {
        let folder = TempDir::new("export-existing");
        let output = folder.join("run");
        std::fs::create_dir_all(&output).unwrap();

        let error = run(&output, Vec::new(), &[]).unwrap_err();
        let messages = error.into_messages();
        assert_eq!(messages.len(), 3);
        assert!(
            messages[0].contains("existierte bereits"),
            "{}",
            messages[0]
        );
    }

    #[test]
    fn an_output_folder_that_is_a_file_stops_the_run() {
        let folder = TempDir::new("export-file");
        let output = folder.write("run", "");
        assert!(run(&output, Vec::new(), &[]).is_err());
    }

    #[test]
    fn the_folder_is_created_and_reported_first() {
        let folder = TempDir::new("export-create");
        let output = folder.join("run");
        let done = run(&output, Vec::new(), &[]).unwrap();

        assert!(output.is_dir());
        assert!(
            done.messages[0].contains("Ordner wurde erstellt"),
            "{:?}",
            done.messages
        );
        assert_eq!(done.failed, 0);
    }

    // ─── a run ────────────────────────────────────────────────────

    #[test]
    fn every_document_is_written_and_named_in_the_report() {
        let folder = TempDir::new("export-run");
        let documents = vec![
            linked(&folder, "a", "eins.txt"),
            linked(&folder, "b", "zwei.txt"),
        ];
        let output = folder.join("run");

        let done = run(&output, documents, &[]).unwrap();
        assert_eq!(done.failed, 0);
        assert!(output.join("eins.txt").is_file());
        assert!(output.join("zwei.txt").is_file());
        assert!(joined(&done.messages).contains("Dokument wurden erstellt: eins.txt"));
        assert!(done.messages.last().unwrap().contains("Alle Dokumente"));
    }

    // Two linked documents can share a basename while living in different
    // folders, and the second must not overwrite the first.
    #[test]
    fn a_shared_basename_is_written_beside_the_first_not_over_it() {
        let folder = TempDir::new("export-collision");
        let first = folder.write("anhang.txt", "erst");
        let second = folder.write("unterordner/anhang.txt", "dann");
        let documents = vec![
            resource_document("a", &first),
            resource_document("b", &second),
        ];
        let output = folder.join("run");

        let done = run(&output, documents, &[]).unwrap();
        assert_eq!(done.failed, 0);
        assert_eq!(
            std::fs::read_to_string(output.join("anhang.txt")).unwrap(),
            "erst"
        );
        assert_eq!(
            std::fs::read_to_string(output.join("anhang (2).txt")).unwrap(),
            "dann"
        );
        // The report names what was actually WRITTEN, not what was asked for.
        assert!(joined(&done.messages).contains("anhang (2).txt"));
    }

    // One document failing must not abandon the rest.
    #[test]
    fn a_failing_document_becomes_report_lines_and_the_run_goes_on() {
        let folder = TempDir::new("export-partial");
        let documents = vec![
            resource_document("weg", &folder.join("gibtsnicht.txt")),
            linked(&folder, "b", "zwei.txt"),
        ];
        let output = folder.join("run");

        let done = run(&output, documents, &[]).unwrap();
        assert_eq!(done.failed, 1);
        assert!(output.join("zwei.txt").is_file());

        let report = joined(&done.messages);
        assert!(
            report.contains("Beim erstellen von: gibtsnicht.txt"),
            "{report}"
        );
        assert!(
            report.contains("Dokument wurden erstellt: zwei.txt"),
            "{report}"
        );
        assert!(
            done.messages
                .last()
                .unwrap()
                .contains("1 Dokument(e) konnten nicht erstellt werden"),
            "{report}"
        );
    }

    #[test]
    fn a_failure_lists_the_expected_fields_and_the_inputs() {
        let folder = TempDir::new("export-failure-detail");
        let documents = vec![pdf_document(
            "a",
            &folder.join("gibtsnicht.pdf"),
            &[("f1", "Vorname"), ("f2", "Ort")],
        )];
        let inputs = [input("Berlin", &["f2"])];

        let done = run(&folder.join("run"), documents, &inputs).unwrap();
        let report = joined(&done.messages);
        assert!(
            report.contains("Folgende Felder wurden erwartet: Vorname, Ort"),
            "{report}"
        );
        assert!(report.contains("Debug Info:f2"), "{report}");
    }

    // ─── refreshed timestamps ─────────────────────────────────────

    // The refreshed mtime is what the caller persists, or the "original was
    // changed" warning returns on every start.
    #[test]
    fn an_original_newer_than_the_store_is_reported_and_handed_back() {
        let folder = TempDir::new("export-refresh");
        // `resource_document` stores mtime 0.0, which every real file beats.
        let documents = vec![resource_document(
            "a",
            &folder.write("anhang.txt", "inhalt"),
        )];

        let done = run(&folder.join("run"), documents, &[]).unwrap();
        assert_eq!(done.failed, 0);
        assert_eq!(done.refreshed.len(), 1);
        assert!(done.refreshed[0].mtime > 0.0);
        assert!(joined(&done.messages).contains("WARNUNG"));
    }

    #[test]
    fn an_unchanged_original_is_neither_warned_about_nor_refreshed() {
        let folder = TempDir::new("export-unchanged");
        let done = run(
            &folder.join("run"),
            vec![linked(&folder, "a", "eins.txt")],
            &[],
        )
        .unwrap();
        assert!(done.refreshed.is_empty());
        assert!(!joined(&done.messages).contains("WARNUNG"));
    }

    // ─── the report ───────────────────────────────────────────────

    // A run that lost a document must not be announced as a success and
    // contradicted by a line further down.
    #[test]
    fn the_headline_carries_the_outcome() {
        let clean = Run {
            messages: Vec::new(),
            failed: 0,
            refreshed: Vec::new(),
        };
        assert_eq!(
            clean.report(Path::new("/tmp/out/run")).headline,
            "Dokumente wurden erfolgreich erstellt"
        );

        let broken = Run { failed: 1, ..clean };
        assert_eq!(
            broken.report(Path::new("/tmp/out/run")).headline,
            "Dokumente wurden mit Fehlern erstellt"
        );
    }

    // The folder is what the report's button opens, so it is always attached —
    // a run with failures still produced one.
    #[test]
    fn the_report_always_carries_the_output_folder() {
        let run = Run {
            messages: vec!["eins".into()],
            failed: 1,
            refreshed: Vec::new(),
        };
        let report = run.report(Path::new("/tmp/out/run"));
        assert_eq!(report.message_folder.as_deref(), Some("/tmp/out/run"));
        assert_eq!(report.messages, ["eins"]);
    }
}
