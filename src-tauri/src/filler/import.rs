// ─── why ────────────────────────────────────────────────────────
// Linking documents: the policy half of `add_documents`, with no Tauri in it.
// Picking the file or the folder needs a window and stays in `commands`; what
// happens to the files once picked does not.
//
// `folder` is `many` over a sorted directory listing, not a second loop. The
// per-file rule — skip the broken one, keep the rest, fail only on nothing
// linked — has to be identical whether the user multi-selected the files or
// pointed at the folder holding them, and two copies of it would eventually
// disagree about which of those is an error.
// ────────────────────────────────────────────────────────────────

use std::path::{Path, PathBuf};

use crate::doc;
use crate::error::{AppError, AppResult};
use crate::model::MappedDocument;
use crate::state::AppState;

pub fn one(filename: &Path, auto_map_fields: bool, state: &AppState) -> AppResult<MappedDocument> {
    if state.db().contains_filename(filename) {
        return Err(AppError::DocumentExists);
    }
    let document = doc::add(filename, auto_map_fields, &state.config)?;
    state.db().add_document(document.clone())?;
    Ok(document)
}

/// The report lines for a batch of files. Errors only when nothing linked at all.
pub fn many(files: &[PathBuf], auto_map_fields: bool, state: &AppState) -> AppResult<Vec<String>> {
    // One unreadable or already-linked file must not abandon the rest — each
    // `one` has ALREADY persisted the documents before it, so failing the whole
    // gesture would report an error over a half-imported batch. The export path
    // collects per-document failures the same way.
    let mut messages = Vec::new();
    let mut added = 0;
    for file in files {
        match one(file, auto_map_fields, state) {
            Ok(_) => added += 1,
            Err(error) => {
                messages.push(format!("{} wurde übersprungen:", doc::file_name(file)));
                messages.extend(error.into_messages());
            }
        }
    }

    // Nothing linked at all is a failure, not a report with a green headline.
    if added == 0 && !messages.is_empty() {
        return Err(AppError::Report(messages));
    }
    messages.insert(0, format!("{added} Dokument(e) wurden hinzugefügt."));
    Ok(messages)
}

/// The report lines for a whole folder. Errors only when nothing linked at all.
pub fn folder(folder: &Path, auto_map_fields: bool, state: &AppState) -> AppResult<Vec<String>> {
    let entries = std::fs::read_dir(folder).map_err(|error| AppError::io(folder, error))?;
    // Filtered and sorted: a sub-DIRECTORY would link as a resource document
    // whose export copy can never succeed, and the order the OS hands back is
    // arbitrary while insertion order IS the order the UI lists documents in.
    let mut files: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_file())
        .collect();
    files.sort();

    many(&files, auto_map_fields, state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TempDir;

    fn names(state: &AppState) -> Vec<String> {
        state
            .db()
            .documents()
            .iter()
            .map(|document| document.name.clone())
            .collect()
    }

    // ─── one file ─────────────────────────────────────────────────

    #[test]
    fn linking_one_file_stores_it() {
        let temp = TempDir::new("import-one");
        let state = temp.state();
        let file = temp.write("anhang.txt", "inhalt");

        let document = one(&file, false, &state).unwrap();
        assert_eq!(document.name, "anhang.txt");
        assert_eq!(names(&state), ["anhang.txt"]);
    }

    // The same FILE, not the same document — a second link would give the user
    // two rows that fill from one original.
    #[test]
    fn linking_the_same_file_twice_is_refused() {
        let temp = TempDir::new("import-duplicate");
        let state = temp.state();
        let file = temp.write("anhang.txt", "inhalt");

        one(&file, false, &state).unwrap();
        assert!(matches!(
            one(&file, false, &state),
            Err(AppError::DocumentExists)
        ));
        assert_eq!(state.db().documents().len(), 1);
    }

    #[test]
    fn linking_a_file_that_is_gone_is_unreadable() {
        let temp = TempDir::new("import-missing");
        let state = temp.state();
        assert!(matches!(
            one(&temp.join("weg.txt"), false, &state),
            Err(AppError::FileUnreadable)
        ));
    }

    // ─── a hand-picked batch ──────────────────────────────────────

    // What a multi-select picker hands back: files that share no folder, and an
    // order the user chose rather than one `read_dir` produced. `many` keeps
    // that order, because insertion order IS the UI's document order.
    #[test]
    fn a_batch_links_the_files_in_the_order_given() {
        let temp = TempDir::new("import-many");
        let state = temp.state();
        let files = vec![
            temp.write("hier/zebra.txt", "inhalt"),
            temp.write("dort/alpha.txt", "inhalt"),
        ];

        let messages = many(&files, false, &state).unwrap();
        assert_eq!(names(&state), ["zebra.txt", "alpha.txt"]);
        assert_eq!(messages, ["2 Dokument(e) wurden hinzugefügt."]);
    }

    // The same per-file rule as a folder: skip the broken one, keep the rest.
    #[test]
    fn a_batch_skips_what_it_cannot_link_and_keeps_the_rest() {
        let temp = TempDir::new("import-many-partial");
        let state = temp.state();
        let files = vec![temp.write("gut.txt", "inhalt"), temp.join("weg.txt")];

        let messages = many(&files, false, &state).unwrap();
        assert_eq!(names(&state), ["gut.txt"]);
        assert_eq!(messages[0], "1 Dokument(e) wurden hinzugefügt.");
        assert!(messages[1].contains("weg.txt"));
    }

    // Nothing linked at all is a failure, not a green headline over a skip list.
    #[test]
    fn a_batch_that_links_nothing_is_an_error() {
        let temp = TempDir::new("import-many-empty");
        let state = temp.state();
        let files = vec![temp.join("weg.txt")];

        assert!(matches!(
            many(&files, false, &state),
            Err(AppError::Report(_))
        ));
    }

    // ─── a whole folder ───────────────────────────────────────────

    // Insertion order IS the UI's document order, and the order the OS hands
    // back from `read_dir` is arbitrary.
    #[test]
    fn a_folder_links_its_files_in_sorted_order() {
        let temp = TempDir::new("import-folder");
        let state = temp.state();
        let source = temp.join("quelle");
        for name in ["zebra.txt", "alpha.txt", "mitte.txt"] {
            temp.write(&format!("quelle/{name}"), "inhalt");
        }

        let messages = folder(&source, false, &state).unwrap();
        assert_eq!(names(&state), ["alpha.txt", "mitte.txt", "zebra.txt"]);
        assert_eq!(messages, ["3 Dokument(e) wurden hinzugefügt."]);
    }

    // A sub-DIRECTORY would link as a resource document whose export copy can
    // never succeed.
    #[test]
    fn sub_directories_are_skipped_without_a_word() {
        let temp = TempDir::new("import-subdir");
        let state = temp.state();
        let source = temp.join("quelle");
        temp.write("quelle/anhang.txt", "inhalt");
        temp.write("quelle/tiefer/versteckt.txt", "inhalt");

        let messages = folder(&source, false, &state).unwrap();
        assert_eq!(names(&state), ["anhang.txt"]);
        assert_eq!(messages, ["1 Dokument(e) wurden hinzugefügt."]);
    }

    // One unreadable or already-linked file must not abandon the rest: each
    // `one` has ALREADY persisted the documents before it.
    #[test]
    fn an_already_linked_file_is_skipped_and_the_rest_survive() {
        let temp = TempDir::new("import-partial");
        let state = temp.state();
        let source = temp.join("quelle");
        let taken = temp.write("quelle/alpha.txt", "inhalt");
        temp.write("quelle/zebra.txt", "inhalt");
        one(&taken, false, &state).unwrap();

        let messages = folder(&source, false, &state).unwrap();
        assert_eq!(names(&state), ["alpha.txt", "zebra.txt"]);
        assert_eq!(messages[0], "1 Dokument(e) wurden hinzugefügt.");
        assert!(
            messages[1].contains("alpha.txt wurde übersprungen"),
            "{messages:?}"
        );
        assert!(
            messages.len() > 2,
            "the skip carries its reason: {messages:?}"
        );
    }

    // Nothing linked at all is a failure, not a report with a green headline.
    #[test]
    fn a_folder_where_nothing_links_is_an_error() {
        let temp = TempDir::new("import-nothing");
        let state = temp.state();
        let source = temp.join("quelle");
        let taken = temp.write("quelle/alpha.txt", "inhalt");
        one(&taken, false, &state).unwrap();

        let error = folder(&source, false, &state).unwrap_err();
        assert!(error.into_messages()[0].contains("alpha.txt"));
    }

    // An EMPTY folder produces no skip lines, so it reports "0 hinzugefügt"
    // rather than erroring — nothing went wrong, there was nothing to link.
    #[test]
    fn an_empty_folder_reports_zero_rather_than_failing() {
        let temp = TempDir::new("import-empty");
        let state = temp.state();
        let source = temp.join("quelle");
        std::fs::create_dir_all(&source).unwrap();

        assert_eq!(
            folder(&source, false, &state).unwrap(),
            ["0 Dokument(e) wurden hinzugefügt."]
        );
    }

    #[test]
    fn a_folder_that_is_not_there_is_an_error() {
        let temp = TempDir::new("import-no-folder");
        let state = temp.state();
        assert!(folder(&temp.join("weg"), false, &state).is_err());
    }
}
