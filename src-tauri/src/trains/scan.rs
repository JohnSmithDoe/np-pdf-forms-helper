// ─── why ────────────────────────────────────────────────────────
// What a dropped or picked set of paths contains, file by file, and which saved
// template each one belongs to. It reads and decides; it never stages, so a
// folder of twenty files costs twenty header reads and not twenty previews.
//
// TOP LEVEL ONLY. A folder is listed one level deep, the way the filler's
// folder import does it; a user who files mail into `2026/09/` drops the month,
// not the year.
//
// EVERY SHEET IS ASKED, from its head only (`grid::heads`), because a template
// maps the header row of ONE sheet and a workbook is free to carry it on any of
// them. A sheet with no recognisable
// table is skipped rather than reported — the second sheet of a sender's export
// is often a summary nobody imports.
//
// Each file gets a STATUS the user can act on rather than an error to dismiss:
// `Erkannt` (exactly one template), `Mehrdeutig` (several — the user picks,
// see `recognise`), `Unbekannt` (none — the manual mapper), `NichtUnterstuetzt`
// (not a workbook) and `Unlesbar` (a workbook that would not open, or a folder
// that could not be listed). One bad file never fails the scan.
//
// A file whose BYTES the app already owns is `Vorhanden`, with the date it was
// cleaned and, if so, imported — identity is the content hash, not the name, so
// a renamed copy is found and an edited one is not. The same bytes twice in one
// drop are `Vorhanden` too, pointing at the first: cleaning both would file one
// document twice.
//
// Excel's owner files are skipped silently. Windows writes `~$Name.xlsx` beside
// every workbook somebody has open, and listing it as an unreadable workbook
// would be a false alarm on exactly the folder the user is working in. Hidden
// files go for the same reason.
// ────────────────────────────────────────────────────────────────

use std::path::{Path, PathBuf};

use std::collections::HashMap;

use super::model::{ImportTemplate, ScanFile, ScanMatch, ScanStatus, Vorhanden};
use super::recognise;
use super::sheet::grid;

pub fn scan(
    paths: &[PathBuf],
    templates: &[ImportTemplate],
    owned: &dyn Fn(&str) -> Option<Vorhanden>,
) -> Vec<ScanFile> {
    let mut files = Vec::new();
    let mut unreadable = Vec::new();
    for path in paths {
        if !path.is_dir() {
            files.push(path.clone());
            continue;
        }
        match crate::doc::list_files(path) {
            Ok(listed) => files.extend(listed),
            Err(error) => unreadable.push(failed(path, error)),
        }
    }
    files.retain(|file| !skipped(file));
    let mut seen = std::collections::HashSet::new();
    files.retain(|file| seen.insert(file.clone()));

    let mut first_of: HashMap<String, String> = HashMap::new();
    files
        .iter()
        .map(|path| {
            let mut scanned = file(path, templates);
            if !matches!(
                scanned.status,
                ScanStatus::NichtUnterstuetzt | ScanStatus::Unlesbar
            ) {
                match super::dokument::hash_of(path) {
                    Ok(hash) => mark_owned(&mut scanned, &hash, owned, &mut first_of),
                    Err(error) => return failed(path, error),
                }
            }
            scanned
        })
        .chain(unreadable)
        .collect()
}

fn mark_owned(
    scanned: &mut ScanFile,
    hash: &str,
    owned: &dyn Fn(&str) -> Option<Vorhanden>,
    first_of: &mut HashMap<String, String>,
) {
    if let Some(vorhanden) = owned(hash) {
        scanned.message = Some(match &vorhanden.importiert_am {
            Some(when) => format!(
                "Bereits bereinigt am {} und importiert am {when}.",
                vorhanden.bereinigt_am
            ),
            None => format!("Bereits bereinigt am {}.", vorhanden.bereinigt_am),
        });
        scanned.status = ScanStatus::Vorhanden;
        scanned.vorhanden = Some(vorhanden);
        return;
    }
    if let Some(first) = first_of.get(hash) {
        scanned.status = ScanStatus::Vorhanden;
        scanned.message = Some(format!("Gleicher Inhalt wie „{first}“ in dieser Auswahl."));
        return;
    }
    first_of.insert(hash.to_string(), scanned.name.clone());
}

fn failed(path: &Path, error: crate::error::AppError) -> ScanFile {
    ScanFile {
        path: path.to_string_lossy().into_owned(),
        name: crate::doc::file_name(path),
        status: ScanStatus::Unlesbar,
        matches: Vec::new(),
        sheets: Vec::new(),
        message: Some(error.into_messages().join(" ")),
        vorhanden: None,
    }
}

fn skipped(path: &Path) -> bool {
    let name = crate::doc::file_name(path);
    name.starts_with("~$") || name.starts_with('.')
}

fn is_workbook(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("xlsx"))
}

fn file(path: &Path, templates: &[ImportTemplate]) -> ScanFile {
    let mut scanned = ScanFile {
        path: path.to_string_lossy().into_owned(),
        name: crate::doc::file_name(path),
        status: ScanStatus::Unbekannt,
        matches: Vec::new(),
        sheets: Vec::new(),
        message: None,
        vorhanden: None,
    };
    if !is_workbook(path) {
        scanned.status = ScanStatus::NichtUnterstuetzt;
        scanned.message = Some("Nur Excel-Dateien (.xlsx) können importiert werden.".into());
        return scanned;
    }

    let grids = match grid::heads(path) {
        Ok(grids) => grids,
        Err(error) => return failed(path, error),
    };
    scanned.sheets = grids.iter().map(|sheet| sheet.sheet.clone()).collect();

    for sheet in &grids {
        let Ok((detected, _)) = recognise::detected(sheet) else {
            continue;
        };
        for template in recognise::matching(templates, &detected.columns) {
            scanned.matches.push(ScanMatch {
                template_id: template.id.clone(),
                template_name: template.name.clone(),
                sheet: sheet.sheet.clone(),
            });
        }
    }

    scanned.status = match scanned.matches.len() {
        0 => ScanStatus::Unbekannt,
        1 => ScanStatus::Erkannt,
        _ => ScanStatus::Mehrdeutig,
    };
    scanned
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{workbook, TempDir, TELEMATIK, TELEMATIK_ROW};
    use crate::trains::builtin;

    fn names(files: &[ScanFile]) -> Vec<&str> {
        files.iter().map(|file| file.name.as_str()).collect()
    }

    #[test]
    fn a_folder_is_read_one_level_deep_and_in_order() {
        let folder = TempDir::new("scan-folder");
        workbook(
            &folder,
            "b.xlsx",
            &[("Tabelle1", &[TELEMATIK, TELEMATIK_ROW])],
        );
        workbook(
            &folder,
            "a.xlsx",
            &[("Tabelle1", &[TELEMATIK, TELEMATIK_ROW])],
        );
        std::fs::create_dir(folder.join("2026")).unwrap();
        workbook(
            &folder,
            "2026/c.xlsx",
            &[("Tabelle1", &[TELEMATIK, TELEMATIK_ROW])],
        );

        let scanned = scan(&[folder.path().to_path_buf()], &builtin::all(), &|_| None);
        assert_eq!(names(&scanned), vec!["a.xlsx", "b.xlsx"]);
    }

    /// Windows' owner file for an open workbook is not a second workbook.
    #[test]
    fn excel_owner_files_and_hidden_files_are_skipped() {
        let folder = TempDir::new("scan-owner");
        folder.write("~$Liste.xlsx", "owner");
        folder.write(".DS_Store", "");
        workbook(
            &folder,
            "Liste.xlsx",
            &[("Tabelle1", &[TELEMATIK, TELEMATIK_ROW])],
        );

        let scanned = scan(&[folder.path().to_path_buf()], &builtin::all(), &|_| None);
        assert_eq!(names(&scanned), vec!["Liste.xlsx"]);
    }

    /// Dropping a folder and a file inside it lists that file once.
    #[test]
    fn a_file_named_twice_is_listed_once() {
        let folder = TempDir::new("scan-twice");
        let file = workbook(
            &folder,
            "a.xlsx",
            &[("Tabelle1", &[TELEMATIK, TELEMATIK_ROW])],
        );
        let scanned = scan(
            &[folder.path().to_path_buf(), file],
            &builtin::all(),
            &|_| None,
        );
        assert_eq!(scanned.len(), 1);
    }

    #[test]
    fn each_file_gets_a_status_and_one_bad_file_fails_nothing() {
        let folder = TempDir::new("scan-status");
        workbook(
            &folder,
            "1-telematik.xlsx",
            &[("Sheet1", &[TELEMATIK, TELEMATIK_ROW])],
        );
        workbook(
            &folder,
            "2-fremd.xlsx",
            &[("Tabelle1", &[&["Kunde", "Ort"], &["a", "b"]])],
        );
        folder.write("3-notiz.pdf", "%PDF");
        folder.write("4-kaputt.xlsx", "not a zip");

        let scanned = scan(&[folder.path().to_path_buf()], &builtin::all(), &|_| None);
        let statuses: Vec<ScanStatus> = scanned.iter().map(|file| file.status).collect();
        assert_eq!(
            statuses,
            vec![
                ScanStatus::Erkannt,
                ScanStatus::Unbekannt,
                ScanStatus::NichtUnterstuetzt,
                ScanStatus::Unlesbar,
            ]
        );
        assert_eq!(scanned[0].matches[0].template_id, "builtin:telematik");
        assert!(scanned[3].message.is_some());
    }

    /// The template's sheet need not be the first one.
    #[test]
    fn a_template_on_the_second_sheet_is_found_with_its_sheet() {
        let folder = TempDir::new("scan-sheet");
        let file = workbook(
            &folder,
            "monitoring.xlsx",
            &[
                ("Übersicht", &[&["Wagentyp", "Achsen"], &["a", "4"]]),
                ("Telematik", &[TELEMATIK, TELEMATIK_ROW]),
            ],
        );
        let scanned = scan(&[file], &builtin::all(), &|_| None);
        assert_eq!(scanned[0].status, ScanStatus::Erkannt);
        assert_eq!(scanned[0].matches[0].sheet, "Telematik");
        assert_eq!(scanned[0].sheets, vec!["Übersicht", "Telematik"]);
    }

    #[test]
    fn two_fitting_templates_make_the_file_ambiguous() {
        let folder = TempDir::new("scan-ambiguous");
        let file = workbook(
            &folder,
            "beides.xlsx",
            &[(
                "Tabelle1",
                &[
                    &[
                        TELEMATIK,
                        &[
                            "Wagennr.",
                            "RadsatzID",
                            "Radsatznummer",
                            "Einbaudatum NACH letzter IS2/3",
                        ],
                    ]
                    .concat(),
                    &[TELEMATIK_ROW, &["y", "z", "w", "v"]].concat(),
                ],
            )],
        );
        let scanned = scan(&[file], &builtin::all(), &|_| None);
        assert_eq!(scanned[0].status, ScanStatus::Mehrdeutig);
        assert_eq!(scanned[0].matches.len(), 2);
    }

    #[test]
    fn a_file_the_app_already_owns_is_vorhanden_with_its_dates() {
        let folder = TempDir::new("scan-owned");
        let file = workbook(
            &folder,
            "a.xlsx",
            &[("Tabelle1", &[TELEMATIK, TELEMATIK_ROW])],
        );
        let hash = crate::trains::dokument::hash_of(&file).unwrap();
        let owned = |seen: &str| {
            (seen == hash).then(|| Vorhanden {
                dokument_id: "d1".into(),
                bereinigt_am: "2026-10-01".into(),
                importiert_am: Some("2026-10-02".into()),
            })
        };

        let scanned = scan(&[file], &builtin::all(), &owned);
        assert_eq!(scanned[0].status, ScanStatus::Vorhanden);
        assert_eq!(scanned[0].vorhanden.as_ref().unwrap().dokument_id, "d1");
        assert!(scanned[0]
            .message
            .as_deref()
            .unwrap()
            .contains("importiert"));
    }

    /// Identity is the bytes: a renamed copy in the same drop is one document.
    #[test]
    fn the_same_bytes_twice_in_one_drop_are_cleaned_once() {
        let folder = TempDir::new("scan-copy");
        let file = workbook(
            &folder,
            "a.xlsx",
            &[("Tabelle1", &[TELEMATIK, TELEMATIK_ROW])],
        );
        std::fs::copy(&file, folder.join("a - Kopie.xlsx")).unwrap();

        let scanned = scan(&[folder.path().to_path_buf()], &builtin::all(), &|_| None);
        let statuses: Vec<ScanStatus> = scanned.iter().map(|file| file.status).collect();
        assert_eq!(names(&scanned), vec!["a - Kopie.xlsx", "a.xlsx"]);
        assert_eq!(statuses, vec![ScanStatus::Erkannt, ScanStatus::Vorhanden]);
        assert!(scanned[1]
            .message
            .as_deref()
            .unwrap()
            .contains("a - Kopie.xlsx"));
    }
}
