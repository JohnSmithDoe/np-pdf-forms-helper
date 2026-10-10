// ─── why ────────────────────────────────────────────────────────
// The app OWNS what it cleans. The original is copied into
// `dokumente/<id>/` BEFORE it is read, and cleaning reads that copy — so what
// was cleaned is byte for byte what is stored, and a sender's file changing on
// the share between the scan and the write cannot reach the record. The
// cleaned copy, and the protocol of what changed, are written beside it.
//
// It is the landing-zone pattern with a LEDGER: raw → cleaned file → load, and
// the `Dokument` record says which step a file has reached. Identity is the
// original's content hash (`hash::bytes`), so the same bytes dropped twice are
// one document, whatever they are called and wherever they came from.
//
// The adopted folder exists before the record does. An abandoned cleaning
// therefore has to `discard` it, and a failed record write does the same —
// otherwise the folder outlives the transaction that was meant to own it.
//
// The protocol is a JSON sidecar rather than part of `dokumente.db`: a file of
// four hundred wagen carries four hundred lines, and the store is rewritten on
// every import. Reading the `Änderungsprotokoll` sheet back was the other
// option — it would make our own output a format we have to parse.
//
// `cleaned_hash` is checked again at import. A cleaned copy the user opened
// and saved in Excel is no longer what the protocol describes, and importing
// it would load values nobody reviewed.
// ────────────────────────────────────────────────────────────────

use std::path::{Path, PathBuf};

use uuid::Uuid;

use super::clean::{Change, HeldClean};
use super::model::{CleanSummary, Dokument, ImportPlan, ProtocolLine};
use crate::error::{AppError, AppResult};

pub const PROTOCOL_FILE: &str = "protokoll.json";

pub struct Adopted {
    pub id: String,
    pub folder: PathBuf,
    pub name: String,
    pub path: PathBuf,
    pub hash: String,
}

pub struct Cleaning {
    pub adopted: Adopted,
    pub held: HeldClean,
}

pub struct Filed<'a> {
    pub sheet: &'a str,
    pub template_id: &'a str,
    pub template_name: &'a str,
    pub plan: ImportPlan,
    pub cleaned: &'a Path,
    pub summary: CleanSummary,
    pub stamp: &'a str,
}

pub fn hash_of(path: &Path) -> AppResult<String> {
    let bytes = std::fs::read(path).map_err(|error| AppError::io(path, error))?;
    Ok(super::hash::bytes(&bytes))
}

pub fn adopt(original: &Path, root: &Path) -> AppResult<Adopted> {
    let bytes = std::fs::read(original).map_err(|error| AppError::io(original, error))?;
    let id = Uuid::new_v4().to_string();
    let folder = root.join(&id);
    std::fs::create_dir_all(&folder).map_err(|error| AppError::io(&folder, error))?;
    let name = crate::doc::file_name(original);
    let path = folder.join(&name);
    if let Err(error) = std::fs::write(&path, &bytes) {
        discard(&folder);
        return Err(AppError::io(&path, error));
    }
    Ok(Adopted {
        id,
        folder,
        name,
        path,
        hash: super::hash::bytes(&bytes),
    })
}

pub fn discard(folder: &Path) {
    let _ = std::fs::remove_dir_all(folder);
}

pub fn write_protocol(folder: &Path, changes: &[Change]) -> AppResult<()> {
    let lines: Vec<ProtocolLine> = changes.iter().map(line).collect();
    let path = folder.join(PROTOCOL_FILE);
    let json = serde_json::to_vec(&lines).map_err(|error| AppError::json(&path, error))?;
    std::fs::write(&path, json).map_err(|error| AppError::io(&path, error))
}

pub fn read_protocol(folder: &Path) -> AppResult<Vec<ProtocolLine>> {
    let path = folder.join(PROTOCOL_FILE);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(AppError::io(&path, error)),
    };
    serde_json::from_str(&text).map_err(|error| AppError::json(&path, error))
}

pub fn record(adopted: &Adopted, filed: Filed<'_>) -> AppResult<Dokument> {
    Ok(Dokument {
        id: adopted.id.clone(),
        name: adopted.name.clone(),
        sheet: filed.sheet.to_string(),
        template_id: filed.template_id.to_string(),
        template_name: filed.template_name.to_string(),
        plan: filed.plan,
        original_hash: adopted.hash.clone(),
        cleaned_hash: hash_of(filed.cleaned)?,
        folder: adopted.folder.to_string_lossy().into_owned(),
        original: adopted.path.to_string_lossy().into_owned(),
        cleaned: filed.cleaned.to_string_lossy().into_owned(),
        summary: filed.summary,
        bereinigt_am: filed.stamp.to_string(),
        importiert_am: None,
        archiviert_am: None,
    })
}

pub fn importable(dokument: &Dokument) -> AppResult<PathBuf> {
    if let Some(when) = &dokument.importiert_am {
        return Err(AppError::Report(vec![
            format!("„{}“ wurde bereits am {when} importiert.", dokument.name),
            "Ein Dokument wird nur einmal ins Schattensystem übernommen.".into(),
        ]));
    }
    let cleaned = PathBuf::from(&dokument.cleaned);
    if hash_of(&cleaned)? != dokument.cleaned_hash {
        return Err(AppError::Report(vec![
            format!(
                "Die bereinigte Datei von „{}“ wurde seit dem Bereinigen verändert.",
                dokument.name
            ),
            "Importiert wird nur, was geprüft wurde. Bitte das Original neu bereinigen.".into(),
        ]));
    }
    Ok(cleaned)
}

fn line(change: &Change) -> ProtocolLine {
    ProtocolLine {
        row: change.row,
        column: change.column,
        header: change.header.clone(),
        raw: change.raw.clone(),
        clean: change.clean.clone(),
        tier: change.tier,
        rule: change.rule.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TempDir;
    use crate::trains::model::Tier;

    #[test]
    fn adopting_copies_the_bytes_and_hashes_the_copy() {
        let folder = TempDir::new("dok-adopt");
        let original = folder.write("eingang/monat.xlsx", "inhalt");
        let adopted = adopt(&original, &folder.join("dokumente")).unwrap();

        assert_eq!(adopted.name, "monat.xlsx");
        assert_eq!(std::fs::read_to_string(&adopted.path).unwrap(), "inhalt");
        assert_eq!(adopted.hash, crate::trains::hash::bytes(b"inhalt"));
        assert!(adopted.path.starts_with(folder.join("dokumente")));
        // The sender's file is only ever read.
        assert_eq!(std::fs::read_to_string(&original).unwrap(), "inhalt");
    }

    #[test]
    fn discarding_removes_the_adopted_folder() {
        let folder = TempDir::new("dok-discard");
        let original = folder.write("monat.xlsx", "inhalt");
        let adopted = adopt(&original, &folder.join("dokumente")).unwrap();
        discard(&adopted.folder);
        assert!(!adopted.folder.exists());
    }

    #[test]
    fn the_protocol_round_trips_and_a_missing_one_is_empty() {
        let folder = TempDir::new("dok-protocol");
        assert!(read_protocol(folder.path()).unwrap().is_empty());

        let change = Change {
            row: 2,
            column: 1,
            header: "Wagennummer".into(),
            raw: "31 80 4740 123-4".into(),
            clean: "318047401234".into(),
            tier: Tier::Format,
            rule: "Wagennummer vereinheitlicht".into(),
        };
        write_protocol(folder.path(), &[change]).unwrap();
        let lines = read_protocol(folder.path()).unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].clean, "318047401234");
        assert_eq!(lines[0].tier, Tier::Format);
    }

    fn dokument(cleaned: &Path) -> Dokument {
        Dokument {
            id: "d1".into(),
            name: "monat.xlsx".into(),
            sheet: "Tabelle1".into(),
            template_id: "t1".into(),
            template_name: "Monatsliste".into(),
            plan: crate::trains::builtin::all()[0].plan.clone(),
            original_hash: "o".into(),
            cleaned_hash: hash_of(cleaned).unwrap(),
            folder: String::new(),
            original: String::new(),
            cleaned: cleaned.to_string_lossy().into_owned(),
            summary: CleanSummary::default(),
            bereinigt_am: "2026-10-03".into(),
            importiert_am: None,
            archiviert_am: None,
        }
    }

    #[test]
    fn an_untouched_cleaned_copy_is_importable() {
        let folder = TempDir::new("dok-importable");
        let cleaned = folder.write("monat.bereinigt.xlsx", "bereinigt");
        assert_eq!(importable(&dokument(&cleaned)).unwrap(), cleaned);
    }

    /// Saved in Excel after cleaning: what would be imported is no longer
    /// what the protocol describes.
    #[test]
    fn an_edited_cleaned_copy_is_refused() {
        let folder = TempDir::new("dok-edited");
        let cleaned = folder.write("monat.bereinigt.xlsx", "bereinigt");
        let record = dokument(&cleaned);
        std::fs::write(&cleaned, "von Hand geändert").unwrap();
        let error = importable(&record).unwrap_err();
        assert!(error.into_messages()[0].contains("verändert"));
    }

    #[test]
    fn an_imported_document_is_final() {
        let folder = TempDir::new("dok-final");
        let cleaned = folder.write("monat.bereinigt.xlsx", "bereinigt");
        let mut record = dokument(&cleaned);
        record.importiert_am = Some("2026-10-03".into());
        let error = importable(&record).unwrap_err();
        assert!(error.into_messages()[0].contains("bereits"));
    }
}
