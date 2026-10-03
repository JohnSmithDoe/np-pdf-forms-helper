// ─── why ────────────────────────────────────────────────────────
// Test-only support, compiled out of every real build by the `#[cfg(test)]` on
// its `mod` declaration in `main.rs`.
//
// `TempDir` is hand-written rather than pulled from a crate: it is a `PathBuf`,
// a unique name and a `Drop`, and the whole point of this backend is to be Rust
// that was written. The name carries the process id and a counter, because
// `cargo test` runs tests in PARALLEL THREADS of one process — a fixed name
// would have two tests sharing a folder.
// ────────────────────────────────────────────────────────────────

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

use crate::config::AppConfig;
use crate::filler::db::Database;
use crate::model::{DocumentKind, MappedDocument, MappedField, MappedInput, PdfField, Profile};
use crate::state::AppState;

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// A folder under the OS temp dir that removes itself when the test ends —
/// including when the test PANICS, since `Drop` still runs while unwinding.
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new(label: &str) -> Self {
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("npdh-test-{}-{label}-{unique}", std::process::id()));
        std::fs::create_dir_all(&path).expect("temp folder");
        Self(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    pub fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    /// A real file on disk, for the paths that stat or copy one.
    pub fn write(&self, name: &str, content: &str) -> PathBuf {
        let path = self.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("parent folder");
        }
        std::fs::write(&path, content).expect("write fixture");
        path
    }

    pub fn config(&self) -> AppConfig {
        let config = AppConfig {
            data_path: self.join("data"),
            cache_path: self.join("data/cache"),
            output_path: self.join("data/out"),
            db_file: self.join("data/data.db"),
            profile_file: self.join("data/profiles.db"),
            master_file: self.join("data/trains/master.xlsx"),
        };
        for folder in [&config.data_path, &config.cache_path, &config.output_path] {
            std::fs::create_dir_all(folder).expect("data folders");
        }
        config
    }

    /// Config plus an empty database — what `import` needs and Tauri does not
    /// reach into. `AppState` is deliberately free of the API surface, which is
    /// what makes this constructible at all.
    pub fn state(&self) -> AppState {
        let config = self.config();
        let db = Database::load(&config).expect("empty database");
        let trains = crate::trains::db::TrainsDb::load(&config).expect("empty trains store");
        AppState {
            config,
            db: Mutex::new(db),
            trains: Mutex::new(trains),
            staging: Mutex::new(None),
            cleaning: Mutex::new(None),
        }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The fixture form: a nested field tree, a UTF-16BE name, a field whose kids
/// are unnamed widgets, and a plain top-level field.
pub fn fixture_pdf() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../docs/formular_beispiel.pdf"
    ))
}

/// A real workbook somebody else authored: 1000 data rows under a header on row
/// one, a blank first cell in that header, and a date column stored as TEXT with
/// the date number format on only some of its rows.
pub fn fixture_xlsx() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../docs/file_example_XLSX_1000.xlsx"
    ))
}

/// A workbook written cell by cell, one `(sheet, rows)` per sheet, every value
/// as text. A cell spelled `#<number>` is stored as an Excel NUMBER instead,
/// because that is how a sender's dates and amounts usually arrive.
pub fn workbook(folder: &TempDir, name: &str, sheets: &[(&str, &[&[&str]])]) -> PathBuf {
    let mut book = umya_spreadsheet::new_file();
    for (position, (sheet, rows)) in sheets.iter().enumerate() {
        if position == 0 {
            if *sheet != "Sheet1" {
                book.set_sheet_name(0, *sheet).unwrap();
            }
        } else {
            book.new_sheet(*sheet).unwrap();
        }
        let worksheet = book.sheet_by_name_mut(sheet).unwrap();
        for (row, cells) in rows.iter().enumerate() {
            for (col, value) in cells.iter().enumerate() {
                let cell = worksheet.cell_mut((col as u32 + 1, row as u32 + 1));
                match value.strip_prefix('#').and_then(|n| n.parse::<f64>().ok()) {
                    Some(number) => {
                        cell.set_value_number(number);
                    }
                    None => {
                        cell.set_value(*value);
                    }
                }
            }
        }
    }
    let path = folder.join(name);
    umya_spreadsheet::writer::xlsx::write(&book, &path).unwrap();
    path
}

// ─── model builders ───────────────────────────────────────────────
// Named arguments would be nicer, but every test here cares about two or three
// fields at most, so the builders take what varies and default the rest.

pub fn resource_document(id: &str, filename: &Path) -> MappedDocument {
    MappedDocument {
        id: id.into(),
        name: filename
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        filename: filename.to_string_lossy().into_owned(),
        mtime: 0.0,
        mapped: None,
        kind: DocumentKind::Resource,
    }
}

pub fn pdf_document(id: &str, filename: &Path, fields: &[(&str, &str)]) -> MappedDocument {
    MappedDocument {
        mapped: Some(
            fields
                .iter()
                .map(|(field_id, mapped_name)| MappedField {
                    orig_id: (*field_id).into(),
                    mapped_name: (*mapped_name).into(),
                })
                .collect(),
        ),
        kind: DocumentKind::Pdf {
            fields: fields
                .iter()
                .map(|(field_id, path)| PdfField {
                    id: (*field_id).into(),
                    path: (*path).into(),
                })
                .collect(),
            previewfile: String::new(),
        },
        ..resource_document(id, filename)
    }
}

pub fn profile(id: &str, document_ids: &[&str], field_ids: &[&str]) -> Profile {
    Profile {
        id: id.into(),
        name: format!("Profil {id}"),
        document_ids: document_ids.iter().map(|id| (*id).into()).collect(),
        field_ids: field_ids.iter().map(|id| (*id).into()).collect(),
    }
}

pub fn input(value: &str, identifiers: &[&str]) -> MappedInput {
    MappedInput {
        identifiers: identifiers.iter().map(|id| (*id).into()).collect(),
        value: value.into(),
    }
}
