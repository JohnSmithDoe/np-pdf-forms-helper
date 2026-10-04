// ─── why ────────────────────────────────────────────────────────
// The wizard's first answer: which sheets of the master a filed Dokument should
// go into, and which file to build on. Answered from the stored header SCAN
// (`bindings::sync`, one `stat` when nothing changed), never by opening sheets —
// the real master has 28 of them.
//
// A sheet is SUGGESTED for one of two reasons, and says which:
//   • remembered  its binding names the document's template (or a user copy of
//                 the same shipped one). This is the mapping the last export
//                 stored, and what picking the file binds by default
//   • fits        it is bound to no template yet and carries every header the
//                 document's plan maps — `recognise`'s rule, applied backwards
// Nothing is blocked: the user ticks, and the preview shows every cell before
// anything is written. A sheet of the overview kind (`kinds::updates`) is only
// never suggested by `fits` and carries a WARNING while no template is bound to
// it — the recognition binds every sheet with a Wagen key column as that kind,
// pasted exports included, so the kind alone cannot tell the dashboard from a
// paste target; a remembered template can.
//
// Suggested sheets come FIRST, the rest in workbook order: the real master has
// 28 sheets and the two that matter must not be scrolled for.
//
// THE BASE is the last copy an export wrote, while it is at least as new as the
// original: a run of documents then accumulates in one file. Once the customer
// saves the original again, building on the copy would drop their edits, so
// the original is the default again. Both are offered; nothing else is, and
// `run` refuses any other path.
// ────────────────────────────────────────────────────────────────

use std::collections::HashSet;
use std::path::Path;

use super::super::{bindings, kinds};
use crate::error::{AppError, AppResult};
use crate::trains::db::TrainsDb;
use crate::trains::model::{
    Dokument, FieldKind, ImportTemplate, MasterExportBase, MasterExportSheet, MasterExportStart,
    MasterSettings,
};

pub fn start(db: &mut TrainsDb, dokument_id: &str) -> AppResult<MasterExportStart> {
    let dokument = super::dokument(db, dokument_id)?.clone();
    super::original(db.master())?;
    bindings::sync(db, false)?;
    let settings = db.master();
    let scan = settings.scan.as_ref().ok_or_else(|| {
        AppError::Report(vec!["Die Master-Datei wurde noch nicht gelesen.".into()])
    })?;
    let templates = db.templates();
    let related = related(&templates, &dokument.template_id);
    let all = headers(&dokument, false);
    let mapped = headers(&dokument, true);

    let sheets = scan
        .sheets
        .iter()
        .map(|sheet| {
            let binding = settings
                .bindings
                .iter()
                .find(|binding| binding.sheet == sheet.name)
                .cloned()
                .unwrap_or_else(|| bindings::default(sheet, &templates));
            let matched = sheet
                .headers
                .iter()
                .map(|header| header.trim())
                .filter(|header| {
                    !header.is_empty()
                        && (all.iter().any(|source| source == header)
                            || binding.aliases.iter().any(|alias| {
                                alias.master.trim() == *header
                                    && all.iter().any(|source| source == alias.source.trim())
                            }))
                })
                .count() as u32;
            let overview = binding.kind.is_some_and(|kind| !kinds::updates(kind));
            let remembered = related.contains(&binding.template_id);
            let warning = (overview && binding.template_id.is_empty()).then(|| {
                "Als Übersicht zugeordnet: Formeln und Notizen des Kunden können beim \
                 Schreiben ersetzt werden — die Vorschau zeigt jede Zelle."
                    .to_string()
            });
            let fits = binding.template_id.is_empty()
                && !overview
                && !mapped.is_empty()
                && mapped
                    .iter()
                    .all(|header| sheet.headers.iter().any(|own| own.trim() == header));
            let reason = if remembered {
                Some(format!(
                    "zugeordnet zur Vorlage „{}“",
                    dokument.template_name
                ))
            } else if fits {
                Some("alle zugeordneten Spalten des Dokuments vorhanden".into())
            } else {
                None
            };
            MasterExportSheet {
                sheet: sheet.name.clone(),
                kind: binding.kind,
                mode: binding.mode,
                key: binding.key.clone(),
                aliases: binding.aliases.clone(),
                ignored: binding.ignored.clone(),
                matched,
                suggested: reason.is_some(),
                reason,
                warning,
            }
        })
        .collect::<Vec<_>>();
    let mut sheets = sheets;
    sheets.sort_by_key(|sheet| !sheet.suggested);

    let (bases, base) = bases(settings)?;
    Ok(MasterExportStart {
        dokument_id: dokument.id.clone(),
        dokument: dokument.name.clone(),
        template: dokument.template_name.clone(),
        bases,
        base,
        sheets,
    })
}

pub fn related(templates: &[ImportTemplate], template_id: &str) -> HashSet<String> {
    let root = templates
        .iter()
        .find(|template| template.id == template_id)
        .and_then(|template| template.origin.clone())
        .unwrap_or_else(|| template_id.to_string());
    let mut related: HashSet<String> = templates
        .iter()
        .filter(|template| template.origin.as_deref() == Some(root.as_str()))
        .map(|template| template.id.clone())
        .collect();
    related.insert(root);
    related.insert(template_id.to_string());
    related
}

pub fn bases(settings: &MasterSettings) -> AppResult<(Vec<MasterExportBase>, String)> {
    let original = super::original(settings)?;
    let entry = |path: &Path, copy| MasterExportBase {
        path: path.to_string_lossy().into_owned(),
        name: crate::doc::file_name(path),
        copy,
    };
    let mut bases = vec![entry(original, false)];
    let mut base = bases[0].path.clone();
    if let Some(copy) = settings
        .last_export
        .as_deref()
        .map(Path::new)
        .filter(|copy| copy.is_file() && *copy != original)
    {
        bases.push(entry(copy, true));
        let newer = crate::doc::file_mtime_ms(copy)? >= crate::doc::file_mtime_ms(original)?;
        if newer {
            base = bases[1].path.clone();
        }
    }
    Ok((bases, base))
}

fn headers(dokument: &Dokument, mapped_only: bool) -> Vec<String> {
    dokument
        .plan
        .columns
        .iter()
        .filter(|column| !mapped_only || column.field != FieldKind::Ignorieren)
        .map(|column| column.header.trim().to_string())
        .filter(|header| !header.is_empty())
        .collect()
}
