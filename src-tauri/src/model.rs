// ─── why ────────────────────────────────────────────────────────
// The second hand-written copy of the wire contract, alongside
// `src/app/@shared/model/`. Nothing generates these — the JSON SHAPE is the
// contract, so field names and optionality must match byte for byte.
// Change one, change the other.
//
// These types are also the ON-DISK format: `data.db` and `profiles.db` hold
// exactly this, including files written by older installs. Anything not
// modelled here is DROPPED on the next write, which is how UI-only keys stay
// out of the store for free.
// ────────────────────────────────────────────────────────────────

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MappedField {
    pub orig_id: String,
    pub mapped_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfField {
    pub id: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sheet {
    pub id: String,
    pub name: String,
}

/// The per-type tail of a document. `type` is the discriminator on the wire, so
/// an unknown value fails to deserialise instead of quietly becoming a resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum DocumentKind {
    Pdf {
        fields: Vec<PdfField>,
        previewfile: String,
    },
    Xlsx {
        sheets: Vec<Sheet>,
    },
    Resource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MappedDocument {
    pub id: String,
    pub name: String,
    pub filename: String,
    pub mtime: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mapped: Option<Vec<MappedField>>,
    #[serde(flatten)]
    pub kind: DocumentKind,
}

impl MappedDocument {
    pub fn mapped_fields(&self) -> &[MappedField] {
        self.mapped.as_deref().unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub document_ids: Vec<String>,
    pub field_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MappedInput {
    pub identifiers: Vec<String>,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientReport {
    pub headline: String,
    pub messages: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_folder: Option<String>,
}

impl ClientReport {
    pub fn headline(headline: impl Into<String>) -> Self {
        Self {
            headline: headline.into(),
            messages: Vec::new(),
            message_folder: None,
        }
    }
}

/// Every command answers with this.
///
/// `documents` / `profiles` are ABSENT when the command cannot have changed them
/// and present when it can. A single `[]` cannot say which — "unchanged" after
/// an export and "there really are none now" after a reset look identical, and
/// the renderer has to guess. `Option` says it outright.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documents: Option<Vec<MappedDocument>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profiles: Option<Vec<Profile>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<ClientReport>,
}

impl ClientData {
    pub fn nothing() -> Self {
        Self {
            documents: None,
            profiles: None,
            message: None,
        }
    }

    pub fn documents(mut self, documents: Vec<MappedDocument>) -> Self {
        self.documents = Some(documents);
        self
    }

    pub fn profiles(mut self, profiles: Vec<Profile>) -> Self {
        self.profiles = Some(profiles);
        self
    }

    pub fn report(mut self, report: ClientReport) -> Self {
        self.message = Some(report);
        self
    }
}

// ─── the wire contract, pinned ────────────────────────────────────
// These tests assert JSON, not Rust: the shape IS the contract with
// `src/app/@shared/model/` and with `data.db` on every existing install. A
// rename that compiles is exactly the change they exist to catch.
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn pdf_document() -> MappedDocument {
        MappedDocument {
            id: "d1".into(),
            name: "formular.pdf".into(),
            filename: "/tmp/formular.pdf".into(),
            mtime: 1_700_000_000_000.0,
            mapped: Some(vec![MappedField {
                orig_id: "f1".into(),
                mapped_name: "Vorname".into(),
            }]),
            kind: DocumentKind::Pdf {
                fields: vec![PdfField {
                    id: "f1".into(),
                    path: "Formular.Vorname".into(),
                }],
                previewfile: "/tmp/cache/formular-preview-data.pdf".into(),
            },
        }
    }

    // ─── ClientData optionality ───────────────────────────────────

    // ABSENT means "this command cannot have changed the list"; present — even
    // empty — means "this is the whole list now". A single `[]` cannot say
    // which, and the store acts on presence.
    #[test]
    fn absent_lists_are_omitted_entirely() {
        assert_eq!(
            serde_json::to_value(ClientData::nothing()).unwrap(),
            json!({})
        );
    }

    #[test]
    fn an_empty_list_is_present_and_distinct_from_absent() {
        let data = serde_json::to_value(ClientData::nothing().documents(Vec::new())).unwrap();
        assert_eq!(data, json!({ "documents": [] }));
        assert!(data.get("profiles").is_none());
        assert!(data.get("message").is_none());
    }

    #[test]
    fn a_report_rides_under_message_not_report() {
        let data =
            serde_json::to_value(ClientData::nothing().report(ClientReport::headline("Fertig")))
                .unwrap();
        assert_eq!(
            data,
            json!({ "message": { "headline": "Fertig", "messages": [] } })
        );
    }

    #[test]
    fn a_message_folder_is_camel_cased_and_omitted_when_absent() {
        let report = ClientReport {
            headline: "Fertig".into(),
            messages: vec!["eins".into()],
            message_folder: Some("/tmp/out/run".into()),
        };
        assert_eq!(
            serde_json::to_value(&report).unwrap(),
            json!({
                "headline": "Fertig",
                "messages": ["eins"],
                "messageFolder": "/tmp/out/run",
            })
        );
    }

    // ─── the document union ───────────────────────────────────────

    #[test]
    fn a_pdf_document_serialises_with_a_flattened_tag_and_camel_case_keys() {
        assert_eq!(
            serde_json::to_value(pdf_document()).unwrap(),
            json!({
                "id": "d1",
                "name": "formular.pdf",
                "filename": "/tmp/formular.pdf",
                "mtime": 1_700_000_000_000.0,
                "mapped": [{ "origId": "f1", "mappedName": "Vorname" }],
                "type": "pdf",
                "fields": [{ "id": "f1", "path": "Formular.Vorname" }],
                "previewfile": "/tmp/cache/formular-preview-data.pdf",
            })
        );
    }

    #[test]
    fn a_resource_document_carries_the_tag_and_nothing_else() {
        let document = MappedDocument {
            mapped: None,
            kind: DocumentKind::Resource,
            ..pdf_document()
        };
        let value = serde_json::to_value(document).unwrap();
        assert_eq!(value["type"], "resource");
        // `mapped` is skipped when absent rather than written as null.
        assert!(value.get("mapped").is_none());
        assert!(value.get("fields").is_none());
    }

    #[test]
    fn an_xlsx_document_carries_its_sheets() {
        let document = MappedDocument {
            kind: DocumentKind::Xlsx {
                sheets: vec![Sheet {
                    id: "s1".into(),
                    name: "Tabelle1".into(),
                }],
            },
            ..pdf_document()
        };
        let value = serde_json::to_value(document).unwrap();
        assert_eq!(value["type"], "xlsx");
        assert_eq!(value["sheets"], json!([{ "id": "s1", "name": "Tabelle1" }]));
    }

    // An unknown discriminator FAILS rather than quietly becoming a resource —
    // a resource export copies the file instead of filling it, silently.
    #[test]
    fn an_unknown_type_fails_to_deserialise() {
        let json = json!({
            "id": "d1", "name": "a", "filename": "/tmp/a", "mtime": 0.0,
            "type": "docx",
        });
        assert!(serde_json::from_value::<MappedDocument>(json).is_err());
    }

    #[test]
    fn a_missing_type_fails_to_deserialise() {
        let json = json!({ "id": "d1", "name": "a", "filename": "/tmp/a", "mtime": 0.0 });
        assert!(serde_json::from_value::<MappedDocument>(json).is_err());
    }

    // Anything not modelled here is DROPPED on the next write, which is how the
    // UI-only `export` / `disabled` keys stay out of the store for free.
    #[test]
    fn unmodelled_keys_are_dropped_on_the_next_write() {
        let stored = json!({
            "id": "d1", "name": "a", "filename": "/tmp/a", "mtime": 0.0,
            "type": "resource",
            "export": true,
            "disabled": false,
        });
        let document: MappedDocument = serde_json::from_value(stored).unwrap();
        let written = serde_json::to_value(document).unwrap();
        assert!(written.get("export").is_none());
        assert!(written.get("disabled").is_none());
    }

    #[test]
    fn a_document_written_by_an_older_install_still_loads() {
        // No `mapped` key at all — every document starts that way.
        let stored = json!({
            "id": "d1", "name": "a", "filename": "/tmp/a", "mtime": 0.0,
            "type": "resource",
        });
        let document: MappedDocument = serde_json::from_value(stored).unwrap();
        assert!(document.mapped.is_none());
        assert!(document.mapped_fields().is_empty());
    }

    #[test]
    fn a_document_round_trips_unchanged() {
        let once = serde_json::to_value(pdf_document()).unwrap();
        let twice: Value =
            serde_json::to_value(serde_json::from_value::<MappedDocument>(once.clone()).unwrap())
                .unwrap();
        assert_eq!(once, twice);
    }

    // ─── profiles and inputs ──────────────────────────────────────

    #[test]
    fn a_profile_uses_camel_cased_id_lists() {
        let profile = Profile {
            id: "p1".into(),
            name: "Standard".into(),
            document_ids: vec!["d1".into()],
            field_ids: vec!["f1".into()],
        };
        assert_eq!(
            serde_json::to_value(profile).unwrap(),
            json!({
                "id": "p1",
                "name": "Standard",
                "documentIds": ["d1"],
                "fieldIds": ["f1"],
            })
        );
    }

    // Flat — one entry per VALUE, carrying every field id that shares it. That
    // is how "same mapped name, same value" is expressed on the wire.
    #[test]
    fn an_input_deserialises_from_identifiers_and_a_value() {
        let input: MappedInput =
            serde_json::from_value(json!({ "identifiers": ["f1", "f2"], "value": "Berlin" }))
                .unwrap();
        assert_eq!(input.identifiers, ["f1", "f2"]);
        assert_eq!(input.value, "Berlin");
    }
}
