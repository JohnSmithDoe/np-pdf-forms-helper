// ─── why ────────────────────────────────────────────────────────
// What the three document services share, split out of the dispatcher so the
// arrows point one way: `mod` dispatches to {`pdf`, `xlsx`, `resource`}, and
// all four reach down here.
//
// This is NOT a trait and does not become one: the dispatcher calls these
// before the `match`, so the per-format half still cannot skip them.
//
// `list_files` and `write_book` are reached from `filler` and `trains` alike.
// A folder is listed FILES ONLY and SORTED: a sub-folder is not a document, and
// the order the OS hands back is arbitrary while insertion order is the order
// the UI lists things in. A workbook is written to a temp file and renamed, so a
// failed write never leaves a half-written file under the final name.
// ────────────────────────────────────────────────────────────────

use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult};
use crate::model::{MappedDocument, MappedInput};

/// The value the user typed for one mapped field. `inputs` is flat — one entry
/// per value, carrying every field id that shares it, which is how "same mapped
/// name, same value" is expressed on the wire. First match wins.
pub fn value_for<'a>(inputs: &'a [MappedInput], orig_id: &str) -> Option<&'a str> {
    inputs
        .iter()
        .find(|input| input.identifiers.iter().any(|id| id == orig_id))
        .map(|input| input.value.as_str())
}

pub fn list_files(folder: &Path) -> AppResult<Vec<PathBuf>> {
    let entries = std::fs::read_dir(folder).map_err(|error| AppError::io(folder, error))?;
    let mut files: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_file())
        .collect();
    files.sort();
    Ok(files)
}

pub fn write_book(
    book: &umya_spreadsheet::Workbook,
    path: &Path,
    headline: String,
) -> AppResult<()> {
    let temp = path.with_extension("tmp.xlsx");
    umya_spreadsheet::writer::xlsx::write(book, &temp)
        .map_err(|error| AppError::detail(headline, error))?;
    std::fs::rename(&temp, path).map_err(|error| AppError::io(path, error))
}

pub fn file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

/// Milliseconds since the epoch as a float — the unit `mtime` carries on the
/// wire and in `data.db`, and what the "original was changed" check compares.
pub fn mtime_ms(path: &Path) -> AppResult<f64> {
    let modified = std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .map_err(|error| AppError::io(path, error))?;
    let since = modified
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    Ok(since.as_secs() as f64 * 1000.0 + f64::from(since.subsec_nanos()) / 1e6)
}

// Two linked documents can share a filename while living in different folders —
// likely enough, given originals sit on network shares — and the second must not
// overwrite the first while being reported as created. The export folder is
// freshly created for this run, so anything already there IS this run.
pub fn free_path(folder: &Path, basename: &str) -> PathBuf {
    let candidate = folder.join(basename);
    if !candidate.exists() {
        return candidate;
    }
    let name = Path::new(basename);
    let stem = name.file_stem().unwrap_or_default().to_string_lossy();
    let extension = match name.extension() {
        Some(extension) => format!(".{}", extension.to_string_lossy()),
        None => String::new(),
    };
    (2..1000)
        .map(|index| folder.join(format!("{stem} ({index}){extension}")))
        .find(|candidate| !candidate.exists())
        .unwrap_or(candidate)
}

// Bit-exact float equality between two languages' timestamp arithmetic is not
// reachable, so under a millisecond counts as unchanged; a real edit moves the
// timestamp by orders of magnitude more.
pub fn warn_if_changed(document: &MappedDocument, mtime: f64, messages: &mut Vec<String>) {
    if (mtime - document.mtime).abs() < 1.0 {
        return;
    }
    messages.push("*******************".into());
    messages.push(format!(
        "WARNUNG: Die Original Datei [{}] wurde verändert!!!",
        document.name
    ));
    messages.push("Bitte überprüfe ob die Felder noch passen.".into());
    messages.push(
        "Wenn nicht lösche am besten das Dokument und füge es danach noch einmal neu hinzu.".into(),
    );
    messages.push("*******************".into());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{input, resource_document, TempDir};

    // ─── value_for ────────────────────────────────────────────────

    #[test]
    fn one_value_answers_for_every_identifier_sharing_it() {
        let inputs = [input("Berlin", &["a", "b"]), input("Köln", &["c"])];
        assert_eq!(value_for(&inputs, "a"), Some("Berlin"));
        assert_eq!(value_for(&inputs, "b"), Some("Berlin"));
        assert_eq!(value_for(&inputs, "c"), Some("Köln"));
    }

    #[test]
    fn an_unknown_id_has_no_value() {
        assert_eq!(value_for(&[input("Berlin", &["a"])], "z"), None);
        assert_eq!(value_for(&[], "a"), None);
    }

    #[test]
    fn first_match_wins() {
        let inputs = [input("erst", &["a"]), input("dann", &["a"])];
        assert_eq!(value_for(&inputs, "a"), Some("erst"));
    }

    // An empty value is a value — the services decide to skip it, this does not.
    #[test]
    fn an_empty_value_is_still_found() {
        assert_eq!(value_for(&[input("", &["a"])], "a"), Some(""));
    }

    // ─── file_name ────────────────────────────────────────────────

    #[test]
    fn file_name_is_the_last_component() {
        assert_eq!(file_name(Path::new("/tmp/formular.pdf")), "formular.pdf");
        assert_eq!(file_name(Path::new("formular.pdf")), "formular.pdf");
    }

    #[test]
    fn a_path_without_a_file_name_is_empty_not_a_panic() {
        assert_eq!(file_name(Path::new("/")), "");
        assert_eq!(file_name(Path::new("..")), "");
    }

    // ─── free_path ────────────────────────────────────────────────

    #[test]
    fn a_free_name_is_used_as_is() {
        let folder = TempDir::new("free-path");
        assert_eq!(
            free_path(folder.path(), "formular.pdf"),
            folder.join("formular.pdf")
        );
    }

    // Two documents can share a basename while living in different folders, and
    // the second must not silently overwrite the first.
    #[test]
    fn a_taken_name_counts_up_keeping_the_extension() {
        let folder = TempDir::new("free-path-taken");
        folder.write("formular.pdf", "");
        assert_eq!(
            free_path(folder.path(), "formular.pdf"),
            folder.join("formular (2).pdf")
        );

        folder.write("formular (2).pdf", "");
        assert_eq!(
            free_path(folder.path(), "formular.pdf"),
            folder.join("formular (3).pdf")
        );
    }

    #[test]
    fn a_taken_name_without_an_extension_gains_no_dot() {
        let folder = TempDir::new("free-path-bare");
        folder.write("liesmich", "");
        assert_eq!(
            free_path(folder.path(), "liesmich"),
            folder.join("liesmich (2)")
        );
    }

    // ─── warn_if_changed ──────────────────────────────────────────

    #[test]
    fn a_sub_millisecond_difference_is_not_a_change() {
        let document = resource_document("d1", Path::new("/tmp/anhang.txt"));
        let mut messages = Vec::new();
        warn_if_changed(&document, document.mtime + 0.4, &mut messages);
        warn_if_changed(&document, document.mtime - 0.9, &mut messages);
        assert!(messages.is_empty(), "{messages:?}");
    }

    #[test]
    fn a_real_edit_warns_and_names_the_document() {
        let document = resource_document("d1", Path::new("/tmp/anhang.txt"));
        let mut messages = Vec::new();
        warn_if_changed(&document, document.mtime + 5_000.0, &mut messages);
        assert_eq!(messages.len(), 5);
        assert!(messages[1].contains("anhang.txt"), "{}", messages[1]);
    }

    // An original OLDER than the store is still a change — a restored backup.
    #[test]
    fn an_older_original_warns_too() {
        let mut document = resource_document("d1", Path::new("/tmp/anhang.txt"));
        document.mtime = 5_000.0;
        let mut messages = Vec::new();
        warn_if_changed(&document, 0.0, &mut messages);
        assert_eq!(messages.len(), 5);
    }

    // ─── mtime_ms ─────────────────────────────────────────────────

    #[test]
    fn mtime_is_milliseconds_since_the_epoch() {
        let folder = TempDir::new("mtime");
        let file = folder.write("anhang.txt", "hallo");
        let mtime = mtime_ms(&file).unwrap();
        // Milliseconds, not seconds: anything written now is past 2001 in ms and
        // the two units are three orders of magnitude apart.
        assert!(mtime > 1_000_000_000_000.0, "{mtime}");
    }

    #[test]
    fn a_missing_file_has_no_mtime() {
        let folder = TempDir::new("mtime-missing");
        assert!(mtime_ms(&folder.join("weg.txt")).is_err());
    }
}
