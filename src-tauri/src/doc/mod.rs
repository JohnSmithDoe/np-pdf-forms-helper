// ─── why ────────────────────────────────────────────────────────
// The extension dispatch, and nothing else — the shared half lives in
// `shared.rs`.
//
// There is no trait here on purpose. The shared steps — the mtime check, the
// id/name/mtime tail — run here, before the `match`, so the per-format part
// cannot skip them: it is unreachable except through this file.
//
// Every service reads the ORIGINAL and writes the target, with no intermediate
// file in between: `lopdf` and `umya` parse into memory, and `resource::create`
// only ever reads its source. No service can write the file it was handed.
// ────────────────────────────────────────────────────────────────

// One folder per format, so the format being worked on is the folder being
// worked in. `resource` is a single `fs::copy` and stays a file.
pub mod pdf;
pub mod resource;
mod shared;
pub mod xlsx;

use std::path::Path;

use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::model::{DocumentKind, MappedDocument, MappedInput};

pub use shared::{file_name, value_for};
use shared::{free_path, mtime_ms, warn_if_changed};

// Tests that need a document whose stored `mtime` already matches its file must
// compute it the same way the dispatcher does, not re-derive the formula.
#[cfg(test)]
pub use shared::mtime_ms as file_mtime_ms;

/// Which service handles a file. Derived from the extension when a document is
/// added or remapped; from then on the stored `DocumentKind` is the truth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Pdf,
    Xlsx,
    Resource,
}

impl Format {
    // Lowercased: Windows names the same file `form.pdf` or `FORM.PDF`, and a
    // case-sensitive match would link the latter as an unfillable resource.
    fn of_file(path: &Path) -> Self {
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_lowercase);
        match extension.as_deref() {
            Some("pdf") => Self::Pdf,
            Some("xlsx") => Self::Xlsx,
            _ => Self::Resource,
        }
    }

    fn of_document(document: &MappedDocument) -> Self {
        match document.kind {
            DocumentKind::Pdf { .. } => Self::Pdf,
            DocumentKind::Xlsx { .. } => Self::Xlsx,
            DocumentKind::Resource => Self::Resource,
        }
    }
}

/// What an export produced, plus the timestamp the original actually had.
pub struct Export {
    pub basename: String,
    pub mtime: f64,
}

pub fn add(
    filename: &Path,
    auto_map_fields: bool,
    config: &AppConfig,
) -> AppResult<MappedDocument> {
    if !filename.is_file() {
        return Err(AppError::FileUnreadable);
    }
    let (kind, mapped) = match Format::of_file(filename) {
        Format::Pdf => pdf::add(filename, auto_map_fields, config)?,
        Format::Xlsx => (xlsx::add(filename)?, None),
        Format::Resource => (DocumentKind::Resource, None),
    };
    Ok(MappedDocument {
        id: uuid::Uuid::new_v4().to_string(),
        name: file_name(filename),
        filename: filename.to_string_lossy().into_owned(),
        mtime: mtime_ms(filename)?,
        mapped,
        kind,
    })
}

pub fn create(
    document: &MappedDocument,
    output_folder: &Path,
    inputs: &[MappedInput],
    messages: &mut Vec<String>,
) -> AppResult<Export> {
    let original = Path::new(&document.filename);
    let mtime = mtime_ms(original)?;
    warn_if_changed(document, mtime, messages);

    let target = free_path(output_folder, &file_name(original));
    match Format::of_document(document) {
        Format::Pdf => pdf::create(original, &target, document, inputs, messages)?,
        Format::Xlsx => xlsx::create(original, &target, document, inputs)?,
        Format::Resource => resource::create(original, &target)?,
    }
    // The name actually written, which is what the report must show.
    Ok(Export {
        basename: file_name(&target),
        mtime,
    })
}

pub fn remap(
    document: &mut MappedDocument,
    new_filename: &Path,
    config: &AppConfig,
) -> AppResult<()> {
    if !new_filename.is_file() {
        return Err(AppError::FileUnreadable);
    }
    if Format::of_file(new_filename) != Format::of_document(document) {
        return Err(AppError::DocumentTypeChanged);
    }
    // Format-specific first, so a failure leaves the stored document untouched
    // rather than pointing at a file whose fields were never re-read.
    match Format::of_document(document) {
        Format::Pdf => pdf::remap(document, new_filename, config)?,
        Format::Xlsx => xlsx::remap()?,
        Format::Resource => {}
    }
    document.filename = new_filename.to_string_lossy().into_owned();
    document.name = file_name(new_filename);
    document.mtime = mtime_ms(new_filename)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{fixture_pdf, resource_document, TempDir};

    // ─── the dispatch ─────────────────────────────────────────────

    #[test]
    fn the_extension_picks_the_service() {
        assert_eq!(Format::of_file(Path::new("a.pdf")), Format::Pdf);
        assert_eq!(Format::of_file(Path::new("a.xlsx")), Format::Xlsx);
        assert_eq!(Format::of_file(Path::new("a.docx")), Format::Resource);
        assert_eq!(Format::of_file(Path::new("liesmich")), Format::Resource);
    }

    // Windows names the same file `form.pdf` or `FORM.PDF`; a case-sensitive
    // match would link the latter as an unfillable resource.
    #[test]
    fn the_extension_is_matched_case_insensitively() {
        assert_eq!(Format::of_file(Path::new("A.PDF")), Format::Pdf);
        assert_eq!(Format::of_file(Path::new("A.Xlsx")), Format::Xlsx);
    }

    // `.xls` is not `.xlsx` — an old-format workbook has no `umya` reader and
    // linking it as a resource is the honest outcome.
    #[test]
    fn the_legacy_excel_extension_is_a_resource() {
        assert_eq!(Format::of_file(Path::new("alt.xls")), Format::Resource);
    }

    #[test]
    fn a_stored_document_dispatches_on_its_kind_not_its_name() {
        // Filename says PDF, `kind` says resource — the stored kind wins, which
        // is what keeps a linked document on one service for its whole life.
        let document = resource_document("d1", Path::new("/tmp/formular.pdf"));
        assert_eq!(Format::of_document(&document), Format::Resource);
    }

    // ─── add ──────────────────────────────────────────────────────

    #[test]
    fn adding_something_that_is_not_a_file_is_unreadable() {
        let folder = TempDir::new("add-missing");
        assert!(matches!(
            add(&folder.join("weg.pdf"), false, &folder.config()),
            Err(AppError::FileUnreadable)
        ));
        // A FOLDER too: the picker cannot hand one over, but `import::folder`
        // walks a directory tree.
        assert!(matches!(
            add(folder.path(), false, &folder.config()),
            Err(AppError::FileUnreadable)
        ));
    }

    #[test]
    fn adding_a_resource_mints_an_id_and_takes_the_file_name() {
        let folder = TempDir::new("add-resource");
        let file = folder.write("anhang.txt", "hallo");
        let document = add(&file, true, &folder.config()).unwrap();

        assert_eq!(document.name, "anhang.txt");
        assert_eq!(document.filename, file.to_string_lossy());
        assert!(matches!(document.kind, DocumentKind::Resource));
        // Auto-mapping is a PDF affair — a resource has no fields to map.
        assert!(document.mapped.is_none());
        assert!(!document.id.is_empty());
        assert!(document.mtime > 0.0);
    }

    #[test]
    fn two_adds_of_one_file_mint_two_ids() {
        let folder = TempDir::new("add-twice");
        let file = folder.write("anhang.txt", "hallo");
        let first = add(&file, false, &folder.config()).unwrap();
        let second = add(&file, false, &folder.config()).unwrap();
        assert_ne!(first.id, second.id);
    }

    #[test]
    fn adding_a_pdf_reads_its_fields_and_can_auto_map_them() {
        let folder = TempDir::new("add-pdf");
        let document = add(&fixture_pdf(), true, &folder.config()).unwrap();

        let DocumentKind::Pdf { fields, .. } = &document.kind else {
            panic!("expected a pdf document");
        };
        // Auto-mapping maps each field onto its own path.
        let mapped = document.mapped.as_ref().unwrap();
        assert_eq!(mapped.len(), fields.len());
        for (field, mapped) in fields.iter().zip(mapped) {
            assert_eq!(field.id, mapped.orig_id);
            assert_eq!(field.path, mapped.mapped_name);
        }
    }

    #[test]
    fn adding_a_pdf_without_auto_mapping_leaves_it_unmapped() {
        let folder = TempDir::new("add-pdf-unmapped");
        let document = add(&fixture_pdf(), false, &folder.config()).unwrap();
        assert!(document.mapped.is_none());
        assert!(document.mapped_fields().is_empty());
    }

    // ─── remap ────────────────────────────────────────────────────

    #[test]
    fn remapping_to_another_type_is_refused() {
        let folder = TempDir::new("remap-type");
        let mut document = add(&fixture_pdf(), false, &folder.config()).unwrap();
        let other = folder.write("anhang.txt", "hallo");

        assert!(matches!(
            remap(&mut document, &other, &folder.config()),
            Err(AppError::DocumentTypeChanged)
        ));
        // Refused means UNTOUCHED, not half-applied.
        assert_eq!(document.filename, fixture_pdf().to_string_lossy());
    }

    #[test]
    fn remapping_to_something_that_is_not_a_file_is_unreadable() {
        let folder = TempDir::new("remap-missing");
        let mut document = resource_document("d1", &folder.write("anhang.txt", "hallo"));
        assert!(matches!(
            remap(&mut document, &folder.join("weg.txt"), &folder.config()),
            Err(AppError::FileUnreadable)
        ));
    }

    #[test]
    fn remapping_a_resource_moves_its_name_filename_and_mtime() {
        let folder = TempDir::new("remap-resource");
        let mut document = resource_document("d1", &folder.write("alt.txt", "hallo"));
        let new_file = folder.write("neu.txt", "hallo");

        remap(&mut document, &new_file, &folder.config()).unwrap();
        assert_eq!(document.name, "neu.txt");
        assert_eq!(document.filename, new_file.to_string_lossy());
        assert!(document.mtime > 0.0);
        // The id is the document's identity and survives a re-link — profiles
        // point at it.
        assert_eq!(document.id, "d1");
    }
}
