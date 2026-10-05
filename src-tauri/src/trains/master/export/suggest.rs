// ─── why ────────────────────────────────────────────────────────
// The wizard's first answer: which sheets of the master a filed Dokument should
// go into, and which file to build on. Answered from the stored header SCAN
// (`bindings::sync`, one `stat` when nothing changed), never by opening sheets —
// the real master has 28 of them.
//
// A sheet is SUGGESTED — pre-ticked — only when its binding names the
// document's template (or a user copy of the same shipped one): the sheet the
// template's documents live in, and the only one where new rows are APPENDED
// by default. The client updates ONE sheet per document, so a sheet that merely
// shares a column (`TelematikProjekt` beside `Telematik`) is not pre-ticked any
// more — it is still offered, and ticking it by hand updates its known rows.
// Its KEY is worked out either way: the binding's, else the sheet's Wagen column
// (`kinds::wagen_column` — Radsatz sheets name a Wagen per Radsatz, so not
// theirs), linked to the document's Wagennummer column by an alias when the
// headers differ (`TRANSPORTMITTELNR` ← `Asset`). Nothing is blocked, and the
// preview shows every cell before anything is written.
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
            let reason = remembered.then(|| {
                format!(
                    "Blatt der Vorlage „{}“ · neue Zeilen werden angehängt",
                    dokument.template_name
                )
            });
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
