// ─── why ────────────────────────────────────────────────────────
// Two JSON files, rewritten in full on every mutation. No real database — the
// working set is a few dozen documents, and a file the user can open in Notepad
// is part of this app's support story.
//
// Profiles reference documents and mapped fields BY ID, so every document
// mutation has to cascade into them or a profile keeps pointing at a field that
// does not exist. That cascade lives in `prune_profiles` and nowhere else.
//
// Writes are atomic (temp file + rename), so a crash or a full disk cannot
// leave a half-written `data.db` behind, and a failed write is an error the
// user sees rather than a silent loss under a UI reporting success.
// ────────────────────────────────────────────────────────────────

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::model::{MappedDocument, Profile};

const VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
struct DocumentStore {
    version: u32,
    documents: IndexMap<String, MappedDocument>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProfileStore {
    version: u32,
    profiles: IndexMap<String, Profile>,
}

impl Default for DocumentStore {
    fn default() -> Self {
        Self {
            version: VERSION,
            documents: IndexMap::new(),
        }
    }
}

impl Default for ProfileStore {
    fn default() -> Self {
        Self {
            version: VERSION,
            profiles: IndexMap::new(),
        }
    }
}

pub struct Database {
    db_file: PathBuf,
    profile_file: PathBuf,
    documents: DocumentStore,
    profiles: ProfileStore,
}

impl Database {
    pub fn load(config: &AppConfig) -> AppResult<Self> {
        Ok(Self {
            documents: read_or_default(&config.db_file)?,
            profiles: read_or_default(&config.profile_file)?,
            db_file: config.db_file.clone(),
            profile_file: config.profile_file.clone(),
        })
    }

    pub fn documents(&self) -> Vec<MappedDocument> {
        self.documents.documents.values().cloned().collect()
    }

    pub fn profiles(&self) -> Vec<Profile> {
        self.profiles.profiles.values().cloned().collect()
    }

    /// Document order, not the order the ids arrived in — the list the user sees
    /// is the order the export report should read in.
    pub fn documents_by_ids(&self, ids: &[String]) -> Vec<MappedDocument> {
        self.documents
            .documents
            .values()
            .filter(|document| ids.contains(&document.id))
            .cloned()
            .collect()
    }

    pub fn contains_id(&self, id: &str) -> bool {
        self.documents.documents.contains_key(id)
    }

    pub fn contains_filename(&self, filename: &Path) -> bool {
        self.documents
            .documents
            .values()
            .any(|document| Path::new(&document.filename) == filename)
    }

    pub fn get(&self, id: &str) -> Option<&MappedDocument> {
        self.documents.documents.get(id)
    }

    pub fn add_document(&mut self, document: MappedDocument) -> AppResult<()> {
        if self.contains_id(&document.id) {
            return Ok(());
        }
        self.documents
            .documents
            .insert(document.id.clone(), document);
        self.write_documents()
    }

    pub fn remove_document(&mut self, id: &str) -> AppResult<()> {
        let Some(removed) = self.documents.documents.shift_remove(id) else {
            return Err(AppError::DocumentMissing);
        };
        self.write_documents()?;
        for profile in self.profiles.profiles.values_mut() {
            profile.document_ids.retain(|held| held != &removed.id);
        }
        self.prune_profile_fields()
    }

    /// `force` exists for the remap path, where the field list can be identical
    /// while the file behind it changed — the caller knows that, this cannot.
    pub fn update_document(&mut self, document: MappedDocument, force: bool) -> AppResult<()> {
        // Saving a document that is not there is an error, not a silent `Ok`:
        // otherwise the command reports success for a write that never happened.
        let changed = {
            let Some(previous) = self.documents.documents.get(&document.id) else {
                return Err(AppError::DocumentMissing);
            };
            // Compares the field IDS, not how many there are: removing one field
            // and adding another in the same save keeps the count identical
            // while every profile pointing at the old id is now dangling.
            field_ids(previous) != field_ids(&document)
        };
        self.documents
            .documents
            .insert(document.id.clone(), document);
        self.write_documents()?;
        if force || changed {
            self.prune_profile_fields()?;
        }
        Ok(())
    }

    pub fn update_profiles(&mut self, profiles: Vec<Profile>) -> AppResult<()> {
        self.profiles.profiles = profiles
            .into_iter()
            .map(|profile| (profile.id.clone(), profile))
            .collect();
        self.write_profiles()
    }

    pub fn reset(&mut self) -> AppResult<()> {
        self.documents.documents.clear();
        self.profiles.profiles.clear();
        self.write_documents()?;
        self.write_profiles()
    }

    // A profile may only hold field ids that some document still maps. Called
    // after every document change, because that is the only thing that can
    // invalidate one.
    fn prune_profile_fields(&mut self) -> AppResult<()> {
        let known: HashSet<String> = self
            .documents
            .documents
            .values()
            .flat_map(|document| document.mapped_fields())
            .map(|field| field.orig_id.clone())
            .collect();

        for profile in self.profiles.profiles.values_mut() {
            profile.field_ids.retain(|id| known.contains(id));
        }
        self.write_profiles()
    }

    fn write_documents(&self) -> AppResult<()> {
        write_atomically(&self.db_file, &self.documents)
    }

    fn write_profiles(&self) -> AppResult<()> {
        write_atomically(&self.profile_file, &self.profiles)
    }
}

fn field_ids(document: &MappedDocument) -> HashSet<&str> {
    document
        .mapped_fields()
        .iter()
        .map(|field| field.orig_id.as_str())
        .collect()
}

// A missing store is a first run, not a failure. A CORRUPT one is a failure —
// silently starting from an empty database would present the user with an app
// that has forgotten every document they ever linked.
fn read_or_default<T: Default + for<'de> Deserialize<'de>>(path: &Path) -> AppResult<T> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(T::default()),
        Err(error) => return Err(AppError::io(path, error)),
    };
    serde_json::from_str(&content).map_err(|error| AppError::json(path, error))
}

// Rename is atomic within a filesystem: readers see either the old file or the
// new one, never a truncated one.
fn write_atomically<T: Serialize>(path: &Path, value: &T) -> AppResult<()> {
    let json = serde_json::to_vec(value).map_err(|error| AppError::json(path, error))?;
    let temp = path.with_extension("tmp");
    std::fs::write(&temp, &json).map_err(|error| AppError::io(&temp, error))?;
    std::fs::rename(&temp, path).map_err(|error| AppError::io(path, error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{pdf_document, profile, resource_document, TempDir};

    fn document(id: &str, fields: &[(&str, &str)]) -> MappedDocument {
        pdf_document(id, Path::new("/tmp/formular.pdf"), fields)
    }

    fn ids(documents: &[MappedDocument]) -> Vec<&str> {
        documents
            .iter()
            .map(|document| document.id.as_str())
            .collect()
    }

    /// A database seeded with documents and profiles, and the folder holding it.
    fn seeded(
        documents: Vec<MappedDocument>,
        profiles: Vec<Profile>,
        label: &str,
    ) -> (TempDir, Database) {
        let folder = TempDir::new(label);
        let mut db = Database::load(&folder.config()).unwrap();
        for document in documents {
            db.add_document(document).unwrap();
        }
        db.update_profiles(profiles).unwrap();
        (folder, db)
    }

    // ─── loading ──────────────────────────────────────────────────

    // A missing store is a first run, not a failure.
    #[test]
    fn a_missing_store_starts_empty() {
        let folder = TempDir::new("db-first-run");
        let db = Database::load(&folder.config()).unwrap();
        assert!(db.documents().is_empty());
        assert!(db.profiles().is_empty());
    }

    // A CORRUPT one is a failure: silently starting empty would present the user
    // with an app that has forgotten every document they ever linked.
    #[test]
    fn a_corrupt_store_is_an_error_not_an_empty_start() {
        let folder = TempDir::new("db-corrupt");
        let config = folder.config();
        std::fs::write(&config.db_file, "{kaputt").unwrap();
        assert!(matches!(
            Database::load(&config),
            Err(AppError::Json { .. })
        ));
    }

    #[test]
    fn a_corrupt_profile_store_is_an_error_too() {
        let folder = TempDir::new("db-corrupt-profiles");
        let config = folder.config();
        std::fs::write(&config.profile_file, "[]").unwrap();
        assert!(Database::load(&config).is_err());
    }

    // ─── order ────────────────────────────────────────────────────

    // Insertion order IS the order the UI lists documents in. A sorted map would
    // silently reshuffle every existing install by UUID on first write.
    #[test]
    fn insertion_order_survives_a_reload() {
        let folder = TempDir::new("db-order");
        let config = folder.config();
        let mut db = Database::load(&config).unwrap();
        for id in ["zebra", "alpha", "mitte"] {
            db.add_document(document(id, &[])).unwrap();
        }
        assert_eq!(ids(&db.documents()), ["zebra", "alpha", "mitte"]);

        let reloaded = Database::load(&config).unwrap();
        assert_eq!(ids(&reloaded.documents()), ["zebra", "alpha", "mitte"]);
    }

    #[test]
    fn updating_a_document_keeps_its_place_in_the_list() {
        let (_folder, mut db) = seeded(
            vec![document("a", &[]), document("b", &[]), document("c", &[])],
            Vec::new(),
            "db-order-update",
        );
        let mut updated = document("b", &[]);
        updated.name = "neu.pdf".into();
        db.update_document(updated, false).unwrap();
        assert_eq!(ids(&db.documents()), ["a", "b", "c"]);
    }

    // Document order, not the order the ids arrived in — the export report
    // should read in the order the user sees.
    #[test]
    fn documents_by_ids_answers_in_document_order() {
        let (_folder, db) = seeded(
            vec![document("a", &[]), document("b", &[]), document("c", &[])],
            Vec::new(),
            "db-by-ids",
        );
        let selected = db.documents_by_ids(&["c".into(), "a".into()]);
        assert_eq!(ids(&selected), ["a", "c"]);
    }

    #[test]
    fn documents_by_ids_ignores_ids_that_are_gone() {
        let (_folder, db) = seeded(vec![document("a", &[])], Vec::new(), "db-by-ids-missing");
        assert_eq!(
            ids(&db.documents_by_ids(&["weg".into()])),
            Vec::<&str>::new()
        );
    }

    // ─── adding and removing ──────────────────────────────────────

    #[test]
    fn adding_the_same_id_twice_is_a_no_op_not_a_replacement() {
        let (_folder, mut db) = seeded(vec![document("a", &[])], Vec::new(), "db-add-twice");
        let mut second = document("a", &[]);
        second.name = "anders.pdf".into();
        db.add_document(second).unwrap();

        assert_eq!(db.documents().len(), 1);
        assert_eq!(db.get("a").unwrap().name, "formular.pdf");
    }

    #[test]
    fn removing_an_unknown_document_is_an_error() {
        let (_folder, mut db) = seeded(Vec::new(), Vec::new(), "db-remove-missing");
        assert!(matches!(
            db.remove_document("weg"),
            Err(AppError::DocumentMissing)
        ));
    }

    // Saving a document that is not there is an error, not a silent `Ok`:
    // otherwise the command reports success for a write that never happened.
    #[test]
    fn updating_an_unknown_document_is_an_error() {
        let (_folder, mut db) = seeded(Vec::new(), Vec::new(), "db-update-missing");
        assert!(matches!(
            db.update_document(document("weg", &[]), false),
            Err(AppError::DocumentMissing)
        ));
    }

    #[test]
    fn contains_filename_compares_paths_not_names() {
        let folder = TempDir::new("db-filename");
        let mut db = Database::load(&folder.config()).unwrap();
        db.add_document(resource_document("a", &folder.join("anhang.txt")))
            .unwrap();

        assert!(db.contains_filename(&folder.join("anhang.txt")));
        assert!(!db.contains_filename(Path::new("/woanders/anhang.txt")));
    }

    // ─── the profile cascade ──────────────────────────────────────

    #[test]
    fn removing_a_document_drops_it_from_every_profile() {
        let (_folder, mut db) = seeded(
            vec![
                document("a", &[("f1", "Vorname")]),
                document("b", &[("f2", "Ort")]),
            ],
            vec![profile("p1", &["a", "b"], &["f1", "f2"])],
            "db-remove-cascade",
        );
        db.remove_document("a").unwrap();

        let profiles = db.profiles();
        assert_eq!(profiles[0].document_ids, ["b"]);
        // And its FIELDS with it — a profile holding `f1` would point at a field
        // no document maps any more.
        assert_eq!(profiles[0].field_ids, ["f2"]);
    }

    // The trap the id comparison exists for: removing one field and adding
    // another in the same save keeps the COUNT identical while every profile
    // pointing at the old id is now dangling.
    #[test]
    fn swapping_a_field_for_another_prunes_the_profile() {
        let (_folder, mut db) = seeded(
            vec![document("a", &[("f1", "Vorname")])],
            vec![profile("p1", &["a"], &["f1"])],
            "db-swap-field",
        );
        db.update_document(document("a", &[("f2", "Ort")]), false)
            .unwrap();
        assert!(db.profiles()[0].field_ids.is_empty());
    }

    #[test]
    fn a_save_that_touches_no_field_leaves_the_profile_alone() {
        let (_folder, mut db) = seeded(
            vec![document("a", &[("f1", "Vorname")])],
            vec![profile("p1", &["a"], &["f1"])],
            "db-save-unchanged",
        );
        let mut renamed = document("a", &[("f1", "Vorname")]);
        renamed.name = "neu.pdf".into();
        db.update_document(renamed, false).unwrap();
        assert_eq!(db.profiles()[0].field_ids, ["f1"]);
    }

    // `force` is the remap path: the field list can be identical while every id
    // behind it was regenerated, which no comparison here can see.
    #[test]
    fn force_prunes_even_when_the_field_ids_look_unchanged() {
        let (_folder, mut db) = seeded(
            vec![document("a", &[("f1", "Vorname")])],
            vec![profile("p1", &["a"], &["f1", "weg"])],
            "db-force",
        );
        db.update_document(document("a", &[("f1", "Vorname")]), true)
            .unwrap();
        // `f1` still exists; the stale `weg` is gone.
        assert_eq!(db.profiles()[0].field_ids, ["f1"]);
    }

    // A field id is only pruned when NO document maps it — two documents can
    // share one through the same mapped name.
    #[test]
    fn a_field_still_mapped_by_another_document_survives() {
        let (_folder, mut db) = seeded(
            vec![
                document("a", &[("shared", "Vorname")]),
                document("b", &[("shared", "Vorname")]),
            ],
            vec![profile("p1", &["a", "b"], &["shared"])],
            "db-shared-field",
        );
        db.remove_document("a").unwrap();
        assert_eq!(db.profiles()[0].field_ids, ["shared"]);
    }

    #[test]
    fn saving_profiles_replaces_the_whole_set() {
        let (_folder, mut db) = seeded(
            Vec::new(),
            vec![profile("p1", &[], &[]), profile("p2", &[], &[])],
            "db-profiles-replace",
        );
        db.update_profiles(vec![profile("p3", &[], &[])]).unwrap();
        assert_eq!(db.profiles().len(), 1);
        assert_eq!(db.profiles()[0].id, "p3");
    }

    #[test]
    fn reset_clears_both_stores_on_disk() {
        let folder = TempDir::new("db-reset");
        let config = folder.config();
        let mut db = Database::load(&config).unwrap();
        db.add_document(document("a", &[("f1", "Vorname")]))
            .unwrap();
        db.update_profiles(vec![profile("p1", &["a"], &["f1"])])
            .unwrap();

        db.reset().unwrap();
        let reloaded = Database::load(&config).unwrap();
        assert!(reloaded.documents().is_empty());
        assert!(reloaded.profiles().is_empty());
    }

    // ─── writing ──────────────────────────────────────────────────

    // Rename is atomic within a filesystem: readers see either the old file or
    // the new one, never a truncated one — and never the temp file afterwards.
    #[test]
    fn a_write_leaves_no_temp_file_behind() {
        let folder = TempDir::new("db-atomic");
        let config = folder.config();
        let mut db = Database::load(&config).unwrap();
        db.add_document(document("a", &[])).unwrap();

        assert!(config.db_file.exists());
        assert!(!config.db_file.with_extension("tmp").exists());
    }

    // A failed write is an error the USER sees rather than a silent loss under
    // a UI reporting success.
    #[test]
    fn an_unwritable_store_is_an_error() {
        let folder = TempDir::new("db-unwritable");
        let mut config = folder.config();
        config.db_file = folder.join("kein/ordner/data.db");
        let mut db = Database::load(&config).unwrap();
        assert!(db.add_document(document("a", &[])).is_err());
    }

    #[test]
    fn the_stored_version_is_written_and_read_back() {
        let folder = TempDir::new("db-version");
        let config = folder.config();
        Database::load(&config)
            .unwrap()
            .add_document(document("a", &[]))
            .unwrap();

        let stored: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&config.db_file).unwrap()).unwrap();
        assert_eq!(stored["version"], VERSION);
        // Keyed by id, not an array — the shape a migration would have to bump.
        assert!(stored["documents"]["a"].is_object());
    }
}
