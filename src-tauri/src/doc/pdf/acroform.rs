// ─── why ────────────────────────────────────────────────────────
// The AcroForm layer, and nothing about npDokumentenhilfe: this module's whole
// vocabulary is `Document` ↔ `(ObjectId, fully-qualified name, value)`. It
// names no wire type, no config path and no German string, which is what makes
// it the half whose correctness is a property of BYTES rather than of the app.
//
// A field's identity is its fully-qualified name: the `/T` values of its
// ancestors joined with '.', which is what the PDF spec calls the FQN.
// ────────────────────────────────────────────────────────────────

use lopdf::{Dictionary, Document, Object, ObjectId, StringFormat};

/// A malformed `/Kids` chain could otherwise recurse until the stack is gone.
const MAX_DEPTH: u8 = 32;

// ─── reading the form ─────────────────────────────────────────────

/// Every terminal field, as `(object id, fully-qualified name)` in document
/// order. The object id is what a fill writes `/V` onto.
pub fn walk_fields(document: &Document) -> Vec<(ObjectId, String)> {
    let mut found = Vec::new();
    for object_id in root_field_ids(document) {
        collect(document, object_id, "", 0, &mut found);
    }
    found
}

fn root_field_ids(document: &Document) -> Vec<ObjectId> {
    let Some(acroform) = acroform(document) else {
        return Vec::new();
    };
    let Ok(fields) = acroform.get(b"Fields").and_then(Object::as_array) else {
        return Vec::new();
    };
    fields
        .iter()
        .filter_map(|field| field.as_reference().ok())
        .collect()
}

// A node is a terminal field when none of its kids carries a `/T` of its own —
// kids without one are widget annotations, which are the field's appearance on
// the page rather than a field in their own right.
fn collect(
    document: &Document,
    object_id: ObjectId,
    prefix: &str,
    depth: u8,
    found: &mut Vec<(ObjectId, String)>,
) {
    if depth > MAX_DEPTH {
        return;
    }
    let Ok(dict) = document.get_dictionary(object_id) else {
        return;
    };
    let path = join(prefix, partial_name(dict).as_deref());
    let kids = named_kids(document, dict);
    if kids.is_empty() {
        if !path.is_empty() {
            found.push((object_id, path));
        }
        return;
    }
    for kid in kids {
        collect(document, kid, &path, depth + 1, found);
    }
}

fn named_kids(document: &Document, dict: &Dictionary) -> Vec<ObjectId> {
    let Ok(kids) = dict.get(b"Kids").and_then(Object::as_array) else {
        return Vec::new();
    };
    kids.iter()
        .filter_map(|kid| kid.as_reference().ok())
        .filter(|kid| document.get_dictionary(*kid).is_ok_and(|kid| kid.has(b"T")))
        .collect()
}

fn partial_name(dict: &Dictionary) -> Option<String> {
    dict.get(b"T")
        .ok()
        .and_then(|name| name.as_str().ok())
        .map(decode_text)
}

fn join(prefix: &str, name: Option<&str>) -> String {
    match name {
        None => prefix.to_string(),
        Some(name) if prefix.is_empty() => name.to_string(),
        Some(name) => format!("{prefix}.{name}"),
    }
}

// A DYNAMIC XFA form is rendered from its XFA stream and the AcroForm layer is
// ignored, so a correct `/V` still comes out blank — and nothing about the
// written file shows it. `/NeedsRendering` is the catalog flag that says which:
// static XFA forms carry `/XFA` too and fill perfectly well, so keying on `/XFA`
// alone would report every form that works.
pub fn is_dynamic_xfa(document: &Document) -> bool {
    document
        .catalog()
        .ok()
        .and_then(|catalog| catalog.get(b"NeedsRendering").ok())
        .and_then(|flag| flag.as_bool().ok())
        .unwrap_or(false)
}

/// A PDF text string is either UTF-16BE behind a byte-order mark or
/// PDFDocEncoded, whose first 256 code points are Latin-1.
fn decode_text(bytes: &[u8]) -> String {
    let Some(utf16) = bytes.strip_prefix(&[0xFE, 0xFF]) else {
        return bytes.iter().map(|&byte| byte as char).collect();
    };
    let units: Vec<u16> = utf16
        .chunks_exact(2)
        .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
        .collect();
    String::from_utf16_lossy(&units)
}

// ─── writing ──────────────────────────────────────────────────────

/// Writes each `(object id, value)` pair as that field's `/V`.
pub fn fill<'a>(document: &mut Document, values: impl IntoIterator<Item = (ObjectId, &'a str)>) {
    for (object_id, value) in values {
        set_value(document, object_id, value);
    }
}

pub fn set_value(document: &mut Document, object_id: ObjectId, value: &str) {
    if let Ok(Object::Dictionary(dict)) = document.get_object_mut(object_id) {
        dict.set("V", text_object(value));
    }
}

/// ASCII goes out as a literal string; anything else as UTF-16BE behind a BOM,
/// because PDFDocEncoding cannot carry it and a literal would arrive mojibake.
fn text_object(value: &str) -> Object {
    if value.is_ascii() {
        return Object::String(value.as_bytes().to_vec(), StringFormat::Literal);
    }
    let mut bytes = vec![0xFE, 0xFF];
    for unit in value.encode_utf16() {
        bytes.extend_from_slice(&unit.to_be_bytes());
    }
    Object::String(bytes, StringFormat::Literal)
}

// Tells the viewer to render the values itself instead of trusting the
// appearance streams, which are still the ones from the empty template.
pub fn set_need_appearances(document: &mut Document) {
    let Some(object_id) = acroform_id(document) else {
        return;
    };
    if let Ok(Object::Dictionary(dict)) = document.get_object_mut(object_id) {
        dict.set("NeedAppearances", Object::Boolean(true));
    }
}

// ─── the AcroForm ─────────────────────────────────────────────────

fn acroform(document: &Document) -> Option<&Dictionary> {
    let object = document.catalog().ok()?.get(b"AcroForm").ok()?;
    match object {
        Object::Reference(object_id) => document.get_dictionary(*object_id).ok(),
        other => other.as_dict().ok(),
    }
}

// Mutating it needs the id, and an AcroForm written inline in the catalog has
// none — rare enough that the caller simply skips the write.
fn acroform_id(document: &Document) -> Option<ObjectId> {
    document
        .catalog()
        .ok()?
        .get(b"AcroForm")
        .ok()?
        .as_reference()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{dictionary, Dictionary, Object};

    // ─── building forms by hand ───────────────────────────────────
    // Byte-level rules need byte-level inputs, so these tests assemble the
    // object graph directly rather than reaching for a file. Only the fixture
    // test below reads one.

    /// A document whose catalog points at an AcroForm holding `fields`.
    fn form(builder: impl FnOnce(&mut Document) -> Vec<Object>) -> Document {
        let mut document = Document::with_version("1.5");
        let fields = builder(&mut document);
        let acroform = document.add_object(dictionary! { "Fields" => fields });
        let catalog = document.add_object(dictionary! {
            "Type" => "Catalog",
            "AcroForm" => acroform,
        });
        document.trailer.set("Root", catalog);
        document
    }

    fn text(value: &str) -> Object {
        Object::String(value.as_bytes().to_vec(), StringFormat::Literal)
    }

    fn utf16(value: &str) -> Object {
        let mut bytes = vec![0xFE, 0xFF];
        for unit in value.encode_utf16() {
            bytes.extend_from_slice(&unit.to_be_bytes());
        }
        Object::String(bytes, StringFormat::Literal)
    }

    fn names(document: &Document) -> Vec<String> {
        walk_fields(document)
            .into_iter()
            .map(|(_, name)| name)
            .collect()
    }

    fn value_of(document: &Document, object_id: ObjectId) -> Vec<u8> {
        match document.get_dictionary(object_id).unwrap().get(b"V") {
            Ok(Object::String(bytes, _)) => bytes.clone(),
            other => panic!("expected a string /V, got {other:?}"),
        }
    }

    // ─── walking the field tree ───────────────────────────────────

    #[test]
    fn a_flat_form_yields_its_field_names() {
        let document = form(|document| {
            vec![
                document
                    .add_object(dictionary! { "T" => text("Vorname") })
                    .into(),
                document
                    .add_object(dictionary! { "T" => text("Nachname") })
                    .into(),
            ]
        });
        assert_eq!(names(&document), ["Vorname", "Nachname"]);
    }

    #[test]
    fn a_nested_tree_joins_the_names_with_dots() {
        let document = form(|document| {
            let leaf = document.add_object(dictionary! { "T" => text("Feld") });
            let branch = document.add_object(dictionary! {
                "T" => text("subform"),
                "Kids" => vec![Object::Reference(leaf)],
            });
            vec![document
                .add_object(dictionary! {
                    "T" => text("Formular"),
                    "Kids" => vec![Object::Reference(branch)],
                })
                .into()]
        });
        assert_eq!(names(&document), ["Formular.subform.Feld"]);
    }

    // THE load-bearing rule: kids without a `/T` are widget annotations — the
    // field's appearance on the page. Recursing into them invents fields that do
    // not exist, and the parent then never appears at all.
    #[test]
    fn kids_without_a_name_are_widgets_so_the_parent_is_the_field() {
        let document = form(|document| {
            let widget =
                document.add_object(dictionary! { "Subtype" => "Widget", "Rect" => vec![] });
            let other =
                document.add_object(dictionary! { "Subtype" => "Widget", "Rect" => vec![] });
            vec![document
                .add_object(dictionary! {
                    "T" => text("Unterschrift"),
                    "Kids" => vec![Object::Reference(widget), Object::Reference(other)],
                })
                .into()]
        });
        assert_eq!(names(&document), ["Unterschrift"]);
    }

    #[test]
    fn a_named_kid_beside_a_widget_still_recurses() {
        let document = form(|document| {
            let widget = document.add_object(dictionary! { "Subtype" => "Widget" });
            let named = document.add_object(dictionary! { "T" => text("Feld") });
            vec![document
                .add_object(dictionary! {
                    "T" => text("Gruppe"),
                    "Kids" => vec![Object::Reference(named), Object::Reference(widget)],
                })
                .into()]
        });
        assert_eq!(names(&document), ["Gruppe.Feld"]);
    }

    // A terminal node with no `/T` anywhere above it has no identity to fill by,
    // so it is not a field.
    #[test]
    fn an_unnamed_terminal_is_dropped() {
        let document = form(|document| {
            vec![document
                .add_object(dictionary! { "Subtype" => "Widget" })
                .into()]
        });
        assert!(walk_fields(&document).is_empty());
    }

    #[test]
    fn a_form_without_an_acroform_has_no_fields() {
        let mut document = Document::with_version("1.5");
        let catalog = document.add_object(dictionary! { "Type" => "Catalog" });
        document.trailer.set("Root", catalog);
        assert!(walk_fields(&document).is_empty());
    }

    // An AcroForm written inline in the catalog rather than as a reference —
    // rare, but reading it must still work.
    #[test]
    fn an_inline_acroform_is_read_too() {
        let mut document = Document::with_version("1.5");
        let field = document.add_object(dictionary! { "T" => text("Feld") });
        let catalog = document.add_object(dictionary! {
            "Type" => "Catalog",
            "AcroForm" => Object::Dictionary(dictionary! {
                "Fields" => vec![Object::Reference(field)],
            }),
        });
        document.trailer.set("Root", catalog);
        assert_eq!(names(&document), ["Feld"]);
    }

    // A malformed `/Kids` chain could otherwise recurse until the stack is gone.
    #[test]
    fn a_kids_cycle_terminates() {
        let mut document = Document::with_version("1.5");
        let object_id = document.add_object(dictionary! { "T" => text("Schleife") });
        let dict = document.get_dictionary_mut(object_id).unwrap();
        dict.set("Kids", vec![Object::Reference(object_id)]);

        let acroform = document.add_object(dictionary! {
            "Fields" => vec![Object::Reference(object_id)],
        });
        let catalog = document.add_object(dictionary! {
            "Type" => "Catalog",
            "AcroForm" => acroform,
        });
        document.trailer.set("Root", catalog);

        // Bounded, not empty: the walk gives up at MAX_DEPTH rather than hanging.
        assert!(walk_fields(&document).is_empty());
    }

    // ─── text encoding ────────────────────────────────────────────

    // A PDF text string is UTF-16BE behind a BOM or PDFDocEncoded. Without this
    // a German field name round-trips as mojibake.
    #[test]
    fn a_utf16be_name_decodes_to_the_german_text() {
        let document = form(|document| {
            vec![document
                .add_object(dictionary! { "T" => utf16("Straße") })
                .into()]
        });
        assert_eq!(names(&document), ["Straße"]);
    }

    #[test]
    fn a_pdfdocencoded_name_decodes_as_latin1() {
        let document = form(|document| {
            vec![document
                .add_object(dictionary! { "T" => Object::String(vec![b'G', 0xFC, b'l'], StringFormat::Literal) })
                .into()]
        });
        assert_eq!(names(&document), ["Gül"]);
    }

    #[test]
    fn an_ascii_value_goes_out_as_plain_bytes() {
        let mut document = form(|document| {
            vec![document
                .add_object(dictionary! { "T" => text("Ort") })
                .into()]
        });
        let (object_id, _) = walk_fields(&document)[0];
        set_value(&mut document, object_id, "Berlin");
        assert_eq!(value_of(&document, object_id), b"Berlin");
    }

    // PDFDocEncoding cannot carry it, so a literal would arrive mojibake.
    #[test]
    fn a_non_ascii_value_goes_out_as_utf16be_behind_a_bom() {
        let mut document = form(|document| {
            vec![document
                .add_object(dictionary! { "T" => text("Ort") })
                .into()]
        });
        let (object_id, _) = walk_fields(&document)[0];
        set_value(&mut document, object_id, "Köln");

        let written = value_of(&document, object_id);
        assert_eq!(&written[..2], &[0xFE, 0xFF]);
        assert_eq!(decode_text(&written), "Köln");
    }

    // The property that matters, stated once: whatever this layer reads, it can
    // write back unchanged.
    #[test]
    fn every_value_round_trips_through_the_encoding() {
        for value in ["", "Berlin", "Köln", "Straße 1", "日本", "a\nb"] {
            let Object::String(bytes, _) = text_object(value) else {
                panic!("expected a string object");
            };
            assert_eq!(decode_text(&bytes), value, "round trip of {value:?}");
        }
    }

    #[test]
    fn fill_writes_every_pair_and_leaves_the_rest_alone() {
        let mut document = form(|document| {
            vec![
                document.add_object(dictionary! { "T" => text("a") }).into(),
                document.add_object(dictionary! { "T" => text("b") }).into(),
            ]
        });
        let fields = walk_fields(&document);
        fill(&mut document, [(fields[0].0, "eins")]);

        assert_eq!(value_of(&document, fields[0].0), b"eins");
        assert!(!document.get_dictionary(fields[1].0).unwrap().has(b"V"));
    }

    #[test]
    fn writing_to_an_unknown_object_is_a_no_op() {
        let mut document = form(|_| Vec::new());
        set_value(&mut document, (999, 0), "Berlin");
    }

    // ─── NeedAppearances ──────────────────────────────────────────

    #[test]
    fn need_appearances_is_set_on_the_acroform() {
        let mut document =
            form(|document| vec![document.add_object(dictionary! { "T" => text("a") }).into()]);
        set_need_appearances(&mut document);

        let acroform_id = document
            .catalog()
            .unwrap()
            .get(b"AcroForm")
            .unwrap()
            .as_reference()
            .unwrap();
        assert!(document
            .get_dictionary(acroform_id)
            .unwrap()
            .get(b"NeedAppearances")
            .unwrap()
            .as_bool()
            .unwrap());
    }

    // An inline AcroForm has no object id to mutate; the write is skipped rather
    // than panicking.
    #[test]
    fn an_inline_acroform_skips_the_write() {
        let mut document = Document::with_version("1.5");
        let catalog = document.add_object(dictionary! {
            "Type" => "Catalog",
            "AcroForm" => Object::Dictionary(Dictionary::new()),
        });
        document.trailer.set("Root", catalog);
        set_need_appearances(&mut document);
    }

    // ─── dynamic XFA ──────────────────────────────────────────────

    // Static XFA forms carry `/XFA` too and fill perfectly well, so keying on
    // its presence would warn on every form that works.
    #[test]
    fn an_xfa_stream_alone_is_not_dynamic() {
        let mut document = Document::with_version("1.5");
        let catalog = document.add_object(dictionary! {
            "Type" => "Catalog",
            "AcroForm" => Object::Dictionary(dictionary! { "XFA" => vec![] }),
        });
        document.trailer.set("Root", catalog);
        assert!(!is_dynamic_xfa(&document));
    }

    #[test]
    fn needs_rendering_is_what_makes_it_dynamic() {
        for (flag, expected) in [(true, true), (false, false)] {
            let mut document = Document::with_version("1.5");
            let catalog = document.add_object(dictionary! {
                "Type" => "Catalog",
                "NeedsRendering" => Object::Boolean(flag),
            });
            document.trailer.set("Root", catalog);
            assert_eq!(is_dynamic_xfa(&document), expected);
        }
    }

    #[test]
    fn a_document_without_a_catalog_is_not_dynamic() {
        assert!(!is_dynamic_xfa(&Document::with_version("1.5")));
    }

    // ─── the fixture ──────────────────────────────────────────────

    // One test against a real file, so the hand-built graphs above cannot all be
    // wrong in the same way. The fixture covers a nested tree, a UTF-16BE name,
    // a field whose kids are unnamed widgets, and a plain top-level field.
    #[test]
    fn the_fixture_form_reads_as_documented() {
        let document = Document::load(crate::testing::fixture_pdf()).unwrap();
        assert_eq!(
            names(&document),
            [
                "Formular1[0].#subform[0].Feld[0]",
                "Formular1[0].#subform[0].Feld[1]",
                "Straße",
                "Unterschrift",
                "Bemerkung",
            ]
        );
        assert!(!is_dynamic_xfa(&document));
    }

    #[test]
    fn a_filled_fixture_survives_a_save_and_reload() {
        let folder = crate::testing::TempDir::new("acroform-save");
        let mut document = Document::load(crate::testing::fixture_pdf()).unwrap();
        let fields = walk_fields(&document);

        fill(&mut document, [(fields[2].0, "Hauptstraße 1")]);
        set_need_appearances(&mut document);
        let target = folder.join("gefüllt.pdf");
        document.save(&target).unwrap();

        let reloaded = Document::load(&target).unwrap();
        let (object_id, name) = walk_fields(&reloaded)[2].clone();
        assert_eq!(name, "Straße");
        assert_eq!(
            decode_text(&value_of(&reloaded, object_id)),
            "Hauptstraße 1"
        );
    }
}
