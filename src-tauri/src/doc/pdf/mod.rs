// ─── why ────────────────────────────────────────────────────────
// The PDF service: what npDokumentenhilfe does with a form. The AcroForm layer
// itself — walking the field tree, computing the fully-qualified name, writing
// `/V` — is `acroform`, which knows nothing about this app. Everything here
// names a wire type, a config path or a German string.
//
// The preview is the discovery mechanism: a copy with every field filled with
// its OWN name, so a user can open it and read off what to map.
// ────────────────────────────────────────────────────────────────

mod acroform;

use std::collections::{HashMap, HashSet};
use std::path::Path;

use lopdf::{Document, ObjectId};
use uuid::Uuid;

use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::model::{DocumentKind, MappedDocument, MappedField, MappedInput, PdfField};

pub fn add(
    filename: &Path,
    auto_map_fields: bool,
    config: &AppConfig,
) -> AppResult<(DocumentKind, Option<Vec<MappedField>>)> {
    let (fields, previewfile) = read_fields(filename, &[], config)?;

    let mapped = auto_map_fields.then(|| {
        fields
            .iter()
            .map(|field| MappedField {
                orig_id: field.id.clone(),
                mapped_name: field.path.clone(),
            })
            .collect()
    });

    Ok((
        DocumentKind::Pdf {
            fields,
            previewfile,
        },
        mapped,
    ))
}

// Field identity across a re-link is the PATH: a field whose path is unchanged
// keeps its id, so every `mapped` entry and every profile pointing at it
// survives. A path that is gone takes its mapping with it, and a new path gets a
// new id and no mapping.
pub fn remap(
    document: &mut MappedDocument,
    new_filename: &Path,
    config: &AppConfig,
) -> AppResult<()> {
    let (fields, previewfile) = read_fields(new_filename, pdf_fields(document), config)?;

    if let Some(mapped) = document.mapped.as_mut() {
        let surviving: HashSet<&str> = fields.iter().map(|field| field.id.as_str()).collect();
        mapped.retain(|field| surviving.contains(field.orig_id.as_str()));
    }
    document.kind = DocumentKind::Pdf {
        fields,
        previewfile,
    };
    Ok(())
}

pub fn create(
    source: &Path,
    target: &Path,
    document: &MappedDocument,
    inputs: &[MappedInput],
    messages: &mut Vec<String>,
) -> AppResult<()> {
    let mut pdf = load(source)?;
    warn_if_dynamic_xfa(&pdf, document, messages);

    // Owned keys, so the walk's borrow of `pdf` ends before `/V` is written.
    let by_path: HashMap<String, ObjectId> = acroform::walk_fields(&pdf)
        .into_iter()
        .map(|(object_id, path)| (path, object_id))
        .collect();
    // `mapped[].origId` IS a `PdfField.id`; resolving it by scanning the field
    // list inside the loop would make the export O(fields × mapped).
    let path_of: HashMap<&str, &str> = pdf_fields(document)
        .iter()
        .map(|field| (field.id.as_str(), field.path.as_str()))
        .collect();

    for mapped in document.mapped_fields() {
        // No value means the field was not ticked for this run, which is normal.
        let Some(value) = super::value_for(inputs, &mapped.orig_id) else {
            continue;
        };
        let Some(path) = path_of.get(mapped.orig_id.as_str()) else {
            continue;
        };
        let Some(object_id) = by_path.get(*path) else {
            warn_missing(path, document, messages);
            continue;
        };
        // An empty value leaves the field alone: exporting a blank must not wipe
        // what the template already says. Same rule as `xlsx::create`.
        if value.is_empty() {
            continue;
        }
        acroform::set_value(&mut pdf, *object_id, value);
    }

    acroform::set_need_appearances(&mut pdf);
    pdf.save(target).map_err(|error| {
        AppError::detail(
            format!(
                "Die PDF-Datei {} konnte nicht geschrieben werden.",
                super::file_name(target)
            ),
            error,
        )
    })?;
    Ok(())
}

/// The read half both `add` and `remap` need: parse, walk, write the preview and
/// record the fields. An id survives when its path does — `add` passes no
/// previous fields and so mints every one of them.
fn read_fields(
    filename: &Path,
    previous: &[PdfField],
    config: &AppConfig,
) -> AppResult<(Vec<PdfField>, String)> {
    // `lopdf` parses the whole file into memory and the preview goes to the
    // cache folder, so the original is never the file written.
    let mut pdf = load(filename)?;
    let found = acroform::walk_fields(&pdf);
    if found.is_empty() {
        return Err(no_fields(filename));
    }
    let previewfile = write_preview(&mut pdf, filename, &found, config)?;

    let fields = found
        .into_iter()
        .map(|(_, path)| PdfField {
            id: previous
                .iter()
                .find(|field| field.path == path)
                .map_or_else(|| Uuid::new_v4().to_string(), |field| field.id.clone()),
            path,
        })
        .collect();
    Ok((fields, previewfile))
}

// Named `<basename>-preview-data.pdf` in the cache folder — an existing preview
// for the same file is simply overwritten.
fn write_preview(
    document: &mut Document,
    filename: &Path,
    found: &[(ObjectId, String)],
    config: &AppConfig,
) -> AppResult<String> {
    acroform::fill(
        document,
        found.iter().map(|(id, path)| (*id, path.as_str())),
    );
    acroform::set_need_appearances(document);

    let stem = filename.file_stem().unwrap_or_default().to_string_lossy();
    let target = config.cache_path.join(format!("{stem}-preview-data.pdf"));
    document.save(&target).map_err(|error| {
        AppError::detail(
            format!(
                "Die Vorschau {} konnte nicht geschrieben werden.",
                super::file_name(&target)
            ),
            error,
        )
    })?;
    Ok(target.to_string_lossy().into_owned())
}

/// The stored field list. Narrowing on the union, never a cast — a document of
/// another kind simply has none.
fn pdf_fields(document: &MappedDocument) -> &[PdfField] {
    match &document.kind {
        DocumentKind::Pdf { fields, .. } => fields,
        _ => &[],
    }
}

fn load(filename: &Path) -> AppResult<Document> {
    AppError::reading(
        format!(
            "Die PDF-Datei {} konnte nicht gelesen werden.",
            super::file_name(filename)
        ),
        || Document::load(filename),
    )
}

// ─── what the user is told ────────────────────────────────────────

fn no_fields(filename: &Path) -> AppError {
    AppError::Report(vec![
        format!(
            "{} enthält keine Formularfelder.",
            super::file_name(filename)
        ),
        "Bitte prüfe ob es sich um ein ausfüllbares PDF-Formular handelt.".into(),
    ])
}

// A path that is no longer in the file means the template was edited. The export
// keeps going and says so, because one changed field must not lose the run.
fn warn_missing(path: &str, document: &MappedDocument, messages: &mut Vec<String>) {
    messages.push(format!(
        "ACHTUNG: Das Feld {path} konnte nicht gefunden werden."
    ));
    messages.push(format!(
        "Bitte prüfe das Dokument {} auf Änderungen.",
        document.name
    ));
    messages.push(
        "Entferne das Dokument gegebenenfalls aus der Software und füge es erneut hinzu.".into(),
    );
    messages.push("Falls dies nicht hilft wende dich an deine IT.".into());
}

// The values still go into the AcroForm layer, which a dynamic viewer ignores —
// so the export succeeds and the document can still come out blank. Only the
// warning belongs here; `acroform::is_dynamic_xfa` decides what is true.
fn warn_if_dynamic_xfa(pdf: &Document, document: &MappedDocument, messages: &mut Vec<String>) {
    if !acroform::is_dynamic_xfa(pdf) {
        return;
    }
    messages.push(format!(
        "ACHTUNG: {} ist ein dynamisches XFA-Formular und wird nicht unterstützt.",
        document.name
    ));
    messages.push(
        "Die Werte wurden in die AcroForm-Ebene geschrieben, die ein dynamischer Viewer \
         ignoriert. Das Dokument kann leer erscheinen."
            .into(),
    );
    messages.push("Bitte das Ergebnis vor dem Weitergeben prüfen.".into());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{fixture_pdf, input, TempDir};
    use lopdf::{dictionary, Object, StringFormat};
    use std::path::PathBuf;

    /// A minimal AcroForm on disk, one flat field per name. Built rather than
    /// checked in, because `remap` needs a SECOND file whose field paths only
    /// partly overlap the first.
    fn form_file(folder: &TempDir, name: &str, fields: &[&str]) -> PathBuf {
        let mut document = Document::with_version("1.5");
        let field_ids: Vec<Object> = fields
            .iter()
            .map(|field| {
                document
                    .add_object(dictionary! {
                        "T" => Object::String(field.as_bytes().to_vec(), StringFormat::Literal),
                    })
                    .into()
            })
            .collect();
        let acroform = document.add_object(dictionary! { "Fields" => field_ids });
        let catalog = document.add_object(dictionary! {
            "Type" => "Catalog",
            "AcroForm" => acroform,
        });
        document.trailer.set("Root", catalog);

        let path = folder.join(name);
        document.save(&path).unwrap();
        path
    }

    /// The raw `/V` bytes of a field in a written file. Raw, because how a
    /// value is ENCODED is part of what the fill has to get right.
    fn raw_at(path: &Path, field: &str) -> Option<Vec<u8>> {
        let document = Document::load(path).unwrap();
        acroform::walk_fields(&document)
            .into_iter()
            .find(|(_, name)| name == field)
            .and_then(|(object_id, _)| {
                match document.get_dictionary(object_id).unwrap().get(b"V") {
                    Ok(Object::String(bytes, _)) => Some(bytes.clone()),
                    _ => None,
                }
            })
    }

    fn value_at(path: &Path, field: &str) -> Option<String> {
        raw_at(path, field).map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
    }

    fn paths(document: &MappedDocument) -> Vec<&str> {
        pdf_fields(document)
            .iter()
            .map(|field| field.path.as_str())
            .collect()
    }

    // ─── add ──────────────────────────────────────────────────────

    #[test]
    fn adding_reads_every_field_path() {
        let folder = TempDir::new("pdf-add");
        let (kind, mapped) = add(&fixture_pdf(), false, &folder.config()).unwrap();

        let DocumentKind::Pdf { fields, .. } = kind else {
            panic!("expected a pdf kind");
        };
        assert_eq!(
            fields
                .iter()
                .map(|field| field.path.as_str())
                .collect::<Vec<_>>(),
            [
                "Formular1[0].#subform[0].Feld[0]",
                "Formular1[0].#subform[0].Feld[1]",
                "Straße",
                "Unterschrift",
                "Bemerkung",
            ]
        );
        assert!(mapped.is_none());
    }

    #[test]
    fn every_field_gets_its_own_id() {
        let folder = TempDir::new("pdf-add-ids");
        let (kind, _) = add(&fixture_pdf(), false, &folder.config()).unwrap();
        let DocumentKind::Pdf { fields, .. } = kind else {
            panic!("expected a pdf kind");
        };
        let unique: HashSet<&str> = fields.iter().map(|field| field.id.as_str()).collect();
        assert_eq!(unique.len(), fields.len());
    }

    // The preview is the discovery mechanism: a copy with every field filled
    // with its OWN name, so a user can open it and read off what to map.
    #[test]
    fn the_preview_is_written_to_the_cache_with_the_names_as_values() {
        let folder = TempDir::new("pdf-preview");
        let config = folder.config();
        let (kind, _) = add(&fixture_pdf(), false, &config).unwrap();
        let DocumentKind::Pdf { previewfile, .. } = kind else {
            panic!("expected a pdf kind");
        };

        let preview = Path::new(&previewfile);
        assert!(preview.is_file());
        assert_eq!(preview.parent().unwrap(), config.cache_path);
        assert_eq!(
            preview.file_name().unwrap(),
            "formular_beispiel-preview-data.pdf"
        );
        assert_eq!(value_at(preview, "Bemerkung").as_deref(), Some("Bemerkung"));
    }

    // The original is never the file written — `lopdf` parses into memory.
    #[test]
    fn adding_leaves_the_original_untouched() {
        let folder = TempDir::new("pdf-untouched");
        let before = std::fs::read(fixture_pdf()).unwrap();
        add(&fixture_pdf(), true, &folder.config()).unwrap();
        assert_eq!(std::fs::read(fixture_pdf()).unwrap(), before);
    }

    #[test]
    fn a_pdf_without_form_fields_is_refused_with_a_reason() {
        let folder = TempDir::new("pdf-no-fields");
        let empty = form_file(&folder, "leer.pdf", &[]);

        let error = add(&empty, false, &folder.config()).unwrap_err();
        let messages = error.into_messages();
        assert!(messages[0].contains("leer.pdf"), "{messages:?}");
        assert!(messages[0].contains("keine Formularfelder"), "{messages:?}");
    }

    #[test]
    fn a_file_that_is_not_a_pdf_is_refused_with_its_name() {
        let folder = TempDir::new("pdf-not-a-pdf");
        let fake = folder.write("kaputt.pdf", "kein PDF");
        let messages = add(&fake, false, &folder.config())
            .unwrap_err()
            .into_messages();
        assert!(messages[0].contains("kaputt.pdf"), "{messages:?}");
    }

    // ─── create ───────────────────────────────────────────────────

    #[test]
    fn a_mapped_field_is_filled_from_its_input() {
        let folder = TempDir::new("pdf-create");
        let document = crate::testing::pdf_document(
            "d1",
            &fixture_pdf(),
            &[("f1", "Bemerkung"), ("f2", "Straße")],
        );
        let target = folder.join("gefüllt.pdf");
        let inputs = [input("Hallo", &["f1"]), input("Hauptstraße 1", &["f2"])];

        let mut messages = Vec::new();
        create(&fixture_pdf(), &target, &document, &inputs, &mut messages).unwrap();

        // ASCII goes out as plain bytes, a German value as UTF-16BE behind a
        // BOM — otherwise the „ß“ arrives mojibake.
        assert_eq!(value_at(&target, "Bemerkung").as_deref(), Some("Hallo"));
        assert_eq!(&raw_at(&target, "Straße").unwrap()[..2], &[0xFE, 0xFF]);

        // A clean fill says nothing: every message this path can add is a
        // warning about the template.
        assert!(messages.is_empty(), "{messages:?}");
    }

    // No value means the field was not ticked for this run, which is normal.
    #[test]
    fn a_field_with_no_input_is_left_alone() {
        let folder = TempDir::new("pdf-untouched-field");
        let document = crate::testing::pdf_document("d1", &fixture_pdf(), &[("f1", "Bemerkung")]);
        let target = folder.join("gefüllt.pdf");

        let mut messages = Vec::new();
        create(&fixture_pdf(), &target, &document, &[], &mut messages).unwrap();
        assert_eq!(value_at(&target, "Bemerkung"), None);
    }

    // An empty value leaves the field alone: exporting a blank must not wipe
    // what the template already says.
    #[test]
    fn an_empty_value_does_not_wipe_the_template() {
        let folder = TempDir::new("pdf-empty-value");
        let template = form_file(&folder, "vorlage.pdf", &["Ort"]);
        {
            let mut pdf = Document::load(&template).unwrap();
            let (object_id, _) = acroform::walk_fields(&pdf)[0];
            acroform::set_value(&mut pdf, object_id, "Vorgabe");
            pdf.save(&template).unwrap();
        }

        let document = crate::testing::pdf_document("d1", &template, &[("f1", "Ort")]);
        let target = folder.join("gefüllt.pdf");
        let mut messages = Vec::new();
        create(
            &template,
            &target,
            &document,
            &[input("", &["f1"])],
            &mut messages,
        )
        .unwrap();

        assert_eq!(value_at(&target, "Ort").as_deref(), Some("Vorgabe"));
    }

    // A path that is no longer in the file means the template was edited. The
    // export keeps going and says so, because one changed field must not lose
    // the run.
    #[test]
    fn a_field_that_left_the_template_warns_without_failing() {
        let folder = TempDir::new("pdf-missing-field");
        let template = form_file(&folder, "vorlage.pdf", &["Ort"]);
        // The stored document still knows a field the file no longer has.
        let document =
            crate::testing::pdf_document("d1", &template, &[("f1", "Ort"), ("f2", "Weg")]);
        let target = folder.join("gefüllt.pdf");

        let mut messages = Vec::new();
        create(
            &template,
            &target,
            &document,
            &[input("Berlin", &["f1"]), input("egal", &["f2"])],
            &mut messages,
        )
        .unwrap();

        assert_eq!(value_at(&target, "Ort").as_deref(), Some("Berlin"));
        assert!(
            messages[0].contains("Das Feld Weg konnte nicht gefunden werden"),
            "{messages:?}"
        );
        assert!(messages[1].contains(&document.name), "{messages:?}");
    }

    // Tells the viewer to render the values itself instead of trusting the
    // appearance streams, which are still the ones from the empty template.
    #[test]
    fn the_written_file_asks_the_viewer_to_render_appearances() {
        let folder = TempDir::new("pdf-appearances");
        let document = crate::testing::pdf_document("d1", &fixture_pdf(), &[]);
        let target = folder.join("gefüllt.pdf");

        create(&fixture_pdf(), &target, &document, &[], &mut Vec::new()).unwrap();

        let reloaded = Document::load(&target).unwrap();
        let acroform_id = reloaded
            .catalog()
            .unwrap()
            .get(b"AcroForm")
            .unwrap()
            .as_reference()
            .unwrap();
        assert!(reloaded
            .get_dictionary(acroform_id)
            .unwrap()
            .get(b"NeedAppearances")
            .unwrap()
            .as_bool()
            .unwrap());
    }

    // The values still go into the AcroForm layer, which a dynamic viewer
    // ignores — so the export succeeds and the document can still come out
    // blank. The warning is the only thing that says so.
    #[test]
    fn a_dynamic_xfa_form_warns_but_still_exports() {
        let folder = TempDir::new("pdf-xfa");
        let mut template = Document::load(fixture_pdf()).unwrap();
        template
            .catalog_mut()
            .unwrap()
            .set("NeedsRendering", Object::Boolean(true));
        let source = folder.join("dynamisch.pdf");
        template.save(&source).unwrap();

        let document = crate::testing::pdf_document("d1", &source, &[]);
        let mut messages = Vec::new();
        create(
            &source,
            &folder.join("aus.pdf"),
            &document,
            &[],
            &mut messages,
        )
        .unwrap();

        assert!(
            messages[0].contains("dynamisches XFA-Formular"),
            "{messages:?}"
        );
        assert!(messages[0].contains(&document.name), "{messages:?}");
    }

    #[test]
    fn an_unwritable_target_names_the_file_it_could_not_write() {
        let folder = TempDir::new("pdf-unwritable");
        let document = crate::testing::pdf_document("d1", &fixture_pdf(), &[]);
        let target = folder.join("kein/ordner/aus.pdf");

        let error = create(&fixture_pdf(), &target, &document, &[], &mut Vec::new()).unwrap_err();
        assert!(error.into_messages()[0].contains("aus.pdf"));
    }

    // ─── remap ────────────────────────────────────────────────────

    // Field identity across a re-link is the PATH: a field whose path is
    // unchanged keeps its id, so every `mapped` entry and every profile
    // pointing at it survives.
    #[test]
    fn a_surviving_path_keeps_its_id_and_its_mapping() {
        let folder = TempDir::new("pdf-remap-survive");
        let config = folder.config();
        let old_file = form_file(&folder, "alt.pdf", &["Ort", "Weg"]);
        let new_file = form_file(&folder, "neu.pdf", &["Ort", "Neu"]);

        let (kind, mapped) = add(&old_file, true, &config).unwrap();
        let mut document = MappedDocument {
            kind,
            mapped,
            ..crate::testing::resource_document("d1", &old_file)
        };
        let before: HashMap<&str, &str> = pdf_fields(&document)
            .iter()
            .map(|field| (field.path.as_str(), field.id.as_str()))
            .collect();
        let ort_id = before["Ort"].to_string();
        let weg_id = before["Weg"].to_string();

        remap(&mut document, &new_file, &config).unwrap();

        assert_eq!(paths(&document), ["Ort", "Neu"]);
        let after: HashMap<&str, &str> = pdf_fields(&document)
            .iter()
            .map(|field| (field.path.as_str(), field.id.as_str()))
            .collect();
        // Same path, same id — the mapping and every profile survive.
        assert_eq!(after["Ort"], ort_id);
        // A new path gets a NEW id and no mapping.
        assert_ne!(after["Neu"], weg_id);

        let mapped = document.mapped.as_ref().unwrap();
        assert_eq!(mapped.len(), 1);
        assert_eq!(mapped[0].orig_id, ort_id);
    }

    #[test]
    fn remapping_a_document_with_no_mapping_leaves_it_unmapped() {
        let folder = TempDir::new("pdf-remap-unmapped");
        let config = folder.config();
        let old_file = form_file(&folder, "alt.pdf", &["Ort"]);
        let new_file = form_file(&folder, "neu.pdf", &["Ort"]);

        let (kind, _) = add(&old_file, false, &config).unwrap();
        let mut document = MappedDocument {
            kind,
            mapped: None,
            ..crate::testing::resource_document("d1", &old_file)
        };
        remap(&mut document, &new_file, &config).unwrap();
        assert!(document.mapped.is_none());
    }

    // Format-specific work runs FIRST, so a failure leaves the stored document
    // untouched rather than pointing at a file whose fields were never re-read.
    #[test]
    fn remapping_to_a_fieldless_pdf_fails_and_changes_nothing() {
        let folder = TempDir::new("pdf-remap-fail");
        let config = folder.config();
        let old_file = form_file(&folder, "alt.pdf", &["Ort"]);
        let empty = form_file(&folder, "leer.pdf", &[]);

        let (kind, mapped) = add(&old_file, true, &config).unwrap();
        let mut document = MappedDocument {
            kind,
            mapped,
            ..crate::testing::resource_document("d1", &old_file)
        };

        assert!(remap(&mut document, &empty, &config).is_err());
        assert_eq!(paths(&document), ["Ort"]);
        assert_eq!(document.mapped.as_ref().unwrap().len(), 1);
    }
}
