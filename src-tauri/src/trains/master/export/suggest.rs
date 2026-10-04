// ─── why ────────────────────────────────────────────────────────
// The wizard's first answer: which sheets of the master a filed Dokument should
// go into, and which file to build on. Answered from the stored header SCAN
// (`bindings::sync`, one `stat` when nothing changed), never by opening sheets —
// the real master has 28 of them.
//
// A sheet is SUGGESTED — pre-ticked — when the document's data would change it,
// and says why:
//   • remembered  its binding names the document's template (or a user copy of
//                 the same shipped one): the sheet the template's documents
//                 live in. Only here are new rows APPENDED by default
//   • affected    it has a key and shares at least one more column with the
//                 document, so the incremental update writes into it. Only its
//                 existing rows are updated: a project list must not grow every
//                 Wagen of a telematics export
// The KEY is the binding's, else the sheet's Wagen column (`kinds::wagen_column`
// — Radsatz sheets name a Wagen per Radsatz, so not theirs), linked to the
// document's Wagennummer column by an alias when the headers differ
// (`TRANSPORTMITTELNR` ← `Asset`). Nothing is blocked: every sheet can be
// ticked, and the preview shows every cell before anything is written.
//
// Suggested sheets come FIRST, the rest in workbook order: the real master has
// 28 sheets and the two that matter must not be scrolled for.
//
// THE BASE is the current version of the client master and nothing else: a run
// of documents accumulates because each written version becomes the current
// one. `bindings::sync` has pointed the settings at it before anything is read.
// ────────────────────────────────────────────────────────────────

use std::collections::HashSet;

use super::super::{bindings, kinds};
use crate::error::{AppError, AppResult};
use crate::trains::db::TrainsDb;
use crate::trains::model::{
    Dokument, FieldKind, ImportTemplate, MasterAlias, MasterExportBase, MasterExportSheet,
    MasterExportStart, MasterSettings, SheetKind,
};

pub fn start(db: &mut TrainsDb, dokument_id: &str) -> AppResult<MasterExportStart> {
    let dokument = super::dokument(db, dokument_id)?.clone();
    bindings::sync(db, false)?;
    super::original(db.master())?;
    let settings = db.master();
    let scan = settings.scan.as_ref().ok_or_else(|| {
        AppError::Report(vec!["Die Master-Datei wurde noch nicht gelesen.".into()])
    })?;
    let templates = db.templates();
    let related = related(&templates, &dokument.template_id);
    let all = headers(&dokument);
    let wagen = dokument
        .plan
        .columns
        .iter()
        .find(|column| column.field == FieldKind::Wagennummer)
        .map(|column| column.header.trim().to_string());

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
            let has = |name: &str| sheet.headers.iter().any(|own| own.trim() == name.trim());
            let radsatz = matches!(
                binding.kind,
                Some(SheetKind::RadsatzEinbau | SheetKind::RadsatzBestand)
            );
            let key = binding.key.clone().filter(|key| has(key)).or_else(|| {
                (!radsatz)
                    .then(|| kinds::wagen_column(&sheet.headers))
                    .flatten()
            });
            let mut aliases = binding.aliases.clone();
            if let (Some(key), Some(source)) = (&key, &wagen) {
                let linked = all.iter().any(|header| header == key.trim())
                    || aliases
                        .iter()
                        .any(|alias| alias.master.trim() == key.trim());
                if !linked {
                    aliases.push(MasterAlias {
                        master: key.clone(),
                        source: source.clone(),
                    });
                }
            }
            let shared: Vec<&str> = sheet
                .headers
                .iter()
                .map(|header| header.trim())
                .filter(|header| {
                    !header.is_empty()
                        && Some(*header) != key.as_deref().map(str::trim)
                        && (all.iter().any(|source| source == header)
                            || aliases.iter().any(|alias| {
                                alias.master.trim() == *header
                                    && all.iter().any(|source| source == alias.source.trim())
                            }))
                })
                .collect();
            let remembered = related.contains(&binding.template_id);
            let affected = key.is_some() && !shared.is_empty();
            let reason = if remembered {
                Some(format!(
                    "Blatt der Vorlage „{}“ · neue Zeilen werden angehängt",
                    dokument.template_name
                ))
            } else if affected {
                let named: Vec<String> = shared
                    .iter()
                    .take(3)
                    .map(|header| format!("„{header}“"))
                    .collect();
                Some(format!(
                    "betroffen über {}{} · Schlüssel „{}“ · nur vorhandene Zeilen",
                    named.join(", "),
                    if shared.len() > 3 { " …" } else { "" },
                    key.as_deref().unwrap_or_default()
                ))
            } else {
                None
            };
            MasterExportSheet {
                sheet: sheet.name.clone(),
                kind: binding.kind,
                key,
                aliases,
                ignored: binding.ignored.clone(),
                append: remembered,
                matched: shared.len() as u32,
                suggested: reason.is_some(),
                reason,
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
    let base = MasterExportBase {
        path: original.to_string_lossy().into_owned(),
        name: crate::doc::file_name(original),
    };
    let path = base.path.clone();
    Ok((vec![base], path))
}

fn headers(dokument: &Dokument) -> Vec<String> {
    dokument
        .plan
        .columns
        .iter()
        .map(|column| column.header.trim().to_string())
        .filter(|header| !header.is_empty())
        .collect()
}
