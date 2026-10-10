// ─── why ────────────────────────────────────────────────────────
// Writing INTO a workbook somebody else owns: cell edits applied to the
// original package, and nothing else changed. umya-spreadsheet re-serialises
// every part it read, and its writer is lossy — on the customer's master it
// stamped five custom cell styles `builtinId="0"` (six „Normal“ styles, which
// Excel answers with its repair dialog) and dropped all 105 border colours. A
// library we do not control must not get to rewrite parts we did not mean to
// change; so it reads and computes, and this writes.
//
// What changes, and only that: the edited worksheets (`sheet::patch`), and —
// once anything changed — `calcChain.xml` dropped with its relationship and
// content type, and `fullCalcOnLoad` on `calcPr` (`package`). Every other
// entry is copied as its compressed bytes.
//
// TRUST IS VERIFIED, NOT ASSUMED. The new package is checked before it may
// replace anything: the same entries in the same order bar the dropped chain;
// every entry not meant to change has the same CRC-32 and sizes; every patched
// sheet parses row by row with every style index inside `cellXfs`; and umya,
// reading the written file back, sees exactly the edited values. Only then is
// it renamed over the target — atomically, from a temp file beside it — so a
// failed check leaves the original untouched and says why. `check` is the same
// patch and verification in memory, for a preview to refuse what the write
// would.
// ────────────────────────────────────────────────────────────────

mod package;
mod sheet;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub use sheet::{letters, CellEdit, Content};

use crate::error::{AppError, AppResult};
use package::Package;

#[derive(Debug, Clone, PartialEq)]
pub struct SheetEdits {
    pub sheet: String,
    pub edits: Vec<CellEdit>,
}

pub fn write(path: &Path, sheets: &[SheetEdits]) -> AppResult<()> {
    let original = std::fs::read(path).map_err(|error| AppError::io(path, error))?;
    let (bytes, parts) = patched(&original, sheets)?;
    let temp = path.with_extension("tmp.xlsx");
    std::fs::write(&temp, &bytes).map_err(|error| AppError::io(&temp, error))?;
    let checked = verify(&original, &bytes, &parts).and_then(|()| read_back(&temp, sheets));
    if let Err(error) = checked {
        let _ = std::fs::remove_file(&temp);
        let mut messages = vec![
            "Die geschriebene Datei hat die Prüfung nicht bestanden; die Master-Datei bleibt unverändert."
                .to_string(),
        ];
        messages.extend(error.into_messages());
        return Err(AppError::Report(messages));
    }
    std::fs::rename(&temp, path).map_err(|error| AppError::io(path, error))
}

pub fn check(path: &Path, sheets: &[SheetEdits]) -> AppResult<()> {
    let original = std::fs::read(path).map_err(|error| AppError::io(path, error))?;
    let (bytes, parts) = patched(&original, sheets)?;
    verify(&original, &bytes, &parts)
}

#[derive(Debug, Default)]
struct Parts {
    replaced: BTreeMap<String, String>,
    dropped: BTreeSet<String>,
    sheets: Vec<String>,
}

fn patched(original: &[u8], sheets: &[SheetEdits]) -> AppResult<(Vec<u8>, Parts)> {
    let mut package = Package::open(original)?;
    let workbook = package::workbook_part(&package.text("_rels/.rels")?)
        .ok_or_else(|| incomplete("_rels/.rels"))?;
    let workbook_rels = package::rels_of(&workbook);
    let workbook_xml = package.text(&workbook)?;
    let mut rels = package.text(&workbook_rels)?;

    let mut parts = Parts::default();
    for sheet in sheets {
        let part = package::sheet_part(&workbook_xml, &workbook, &rels, &sheet.sheet).ok_or_else(
            || {
                AppError::Report(vec![format!(
                    "Die Master-Datei hat kein Blatt „{}“ mehr.",
                    sheet.sheet
                )])
            },
        )?;
        if parts.replaced.contains_key(&part) {
            return Err(incomplete(&part));
        }
        let xml = package.text(&part)?;
        parts
            .replaced
            .insert(part.clone(), sheet::patch(&xml, &sheet.edits)?);
        parts.sheets.push(part);
    }
    if sheets.iter().all(|sheet| sheet.edits.is_empty()) {
        return Ok((original.to_vec(), Parts::default()));
    }

    parts
        .replaced
        .insert(workbook.clone(), package::full_calc_on_load(&workbook_xml)?);
    if let Some((id, chain)) = package::calc_chain(&workbook, &rels) {
        rels = package::without_relationship(&rels, &id);
        parts.replaced.insert(workbook_rels, rels);
        if package.has(&chain) {
            parts.dropped.insert(chain.clone());
        }
        let types = package.text(package::content_types())?;
        parts.replaced.insert(
            package::content_types().to_string(),
            package::without_override(&types, &chain),
        );
    }
    let bytes = package.rebuild(&parts.replaced, &parts.dropped)?;
    Ok((bytes, parts))
}

fn verify(original: &[u8], written: &[u8], parts: &Parts) -> AppResult<()> {
    let before = Package::open(original)?.entries()?;
    let mut package = Package::open(written)?;
    let after = package.entries()?;
    let expected: Vec<&str> = before
        .iter()
        .map(|entry| entry.name.as_str())
        .filter(|name| !parts.dropped.contains(*name))
        .collect();
    let names: Vec<&str> = after.iter().map(|entry| entry.name.as_str()).collect();
    if names != expected {
        return Err(failed(
            "die Teile der Datei stimmen nicht mit dem Original überein",
        ));
    }
    for entry in &after {
        match parts.replaced.get(&entry.name) {
            Some(text) => {
                if entry.size != text.len() as u64 {
                    return Err(failed(&format!(
                        "{} wurde nicht vollständig geschrieben",
                        entry.name
                    )));
                }
            }
            None => {
                let old = before.iter().find(|old| old.name == entry.name);
                if old != Some(entry) {
                    return Err(failed(&format!("{} wurde verändert", entry.name)));
                }
            }
        }
    }
    let styles = style_count(&mut package)?;
    for part in &parts.sheets {
        let xml = package.text(part)?;
        sheet::check(&xml, styles)?;
    }
    Ok(())
}

fn style_count(package: &mut Package<'_>) -> AppResult<Option<usize>> {
    if !package.has("xl/styles.xml") {
        return Ok(None);
    }
    let styles = package.text("xl/styles.xml")?;
    Ok(sheet::cell_xfs_count(&styles))
}

fn read_back(path: &Path, sheets: &[SheetEdits]) -> AppResult<()> {
    let headline = "Die geschriebene Datei konnte nicht wieder gelesen werden.".to_string();
    let mut book = AppError::reading(headline.clone(), || {
        umya_spreadsheet::reader::xlsx::lazy_read(path)
    })?;
    for sheet in sheets {
        let index = book
            .sheet_collection_no_check()
            .iter()
            .position(|own| own.name() == sheet.sheet)
            .ok_or_else(|| failed(&format!("das Blatt „{}“ fehlt", sheet.sheet)))?;
        AppError::reading(headline.clone(), || {
            book.read_sheet(index);
            Ok::<_, std::convert::Infallible>(())
        })?;
        let worksheet = book
            .sheet(index)
            .map_err(|error| AppError::detail(headline.clone(), error))?;
        for edit in &sheet.edits {
            if !reads_back(worksheet.cell((edit.col, edit.row)), &edit.content) {
                return Err(failed(&format!(
                    "{}{} liest sich nicht wie geschrieben",
                    letters(edit.col),
                    edit.row
                )));
            }
        }
    }
    Ok(())
}

fn reads_back(cell: Option<&umya_spreadsheet::Cell>, content: &Content) -> bool {
    match (cell, content) {
        (None, Content::Empty) => true,
        (None, _) => false,
        (Some(cell), Content::Empty) => !cell.is_formula() && cell.value().is_empty(),
        (Some(cell), Content::Formula(text)) => cell.formula() == text,
        (Some(cell), Content::Number(number)) => cell
            .value_number()
            .is_some_and(|read| (read - number).abs() <= f64::EPSILON * number.abs().max(1.0)),
        (Some(cell), Content::Bool(value)) => {
            let read = cell.value();
            read.eq_ignore_ascii_case(if *value { "true" } else { "false" })
                || read == if *value { "1" } else { "0" }
        }
        (Some(cell), Content::Text(text)) => {
            sheet::needs_excel_escape(text) || cell.value() == text.as_str()
        }
    }
}

fn failed(detail: &str) -> AppError {
    AppError::Report(vec![format!("Prüfung: {detail}.")])
}

fn incomplete(part: &str) -> AppError {
    AppError::Report(vec![format!(
        "Die Excel-Datei ist unvollständig ({part}); es wurde nichts geändert."
    )])
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;
    use crate::testing::TempDir;

    // The exact shape that broke: custom cell styles WITHOUT builtinId, border
    // colours, a calc chain, a shared formula group, a second sheet.
    const STYLES: &str = concat!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>",
        "<styleSheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">",
        "<fonts count=\"1\"><font><sz val=\"11\"/><name val=\"Calibri\"/></font></fonts>",
        "<fills count=\"2\"><fill><patternFill patternType=\"none\"/></fill><fill><patternFill patternType=\"gray125\"/></fill></fills>",
        "<borders count=\"2\"><border><left/><right/><top/><bottom/><diagonal/></border>",
        "<border><left style=\"thin\"><color rgb=\"FFFF0000\"/></left><right/><top/><bottom style=\"thin\"><color theme=\"4\"/></bottom><diagonal/></border></borders>",
        "<cellStyleXfs count=\"2\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\"/><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\"/></cellStyleXfs>",
        "<cellXfs count=\"3\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\"/>",
        "<xf numFmtId=\"14\" fontId=\"0\" fillId=\"0\" borderId=\"1\" xfId=\"0\" applyNumberFormat=\"1\" applyBorder=\"1\"/>",
        "<xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"1\" xfId=\"1\"/></cellXfs>",
        "<cellStyles count=\"2\"><cellStyle name=\"Standard\" xfId=\"0\" builtinId=\"0\"/><cellStyle name=\"Standard 2\" xfId=\"1\"/></cellStyles>",
        "</styleSheet>",
    );
    const SHEET1: &str = concat!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>",
        "<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\">",
        "<dimension ref=\"A1:C3\"/><sheetData>",
        "<row r=\"1\"><c r=\"A1\" t=\"s\"><v>0</v></c><c r=\"B1\" t=\"s\"><v>1</v></c><c r=\"C1\" t=\"s\"><v>2</v></c></row>",
        "<row r=\"2\"><c r=\"A2\" s=\"2\"><v>338506591522</v></c><c r=\"B2\" s=\"2\" t=\"s\"><v>3</v></c><c r=\"C2\" s=\"1\"><f t=\"shared\" ref=\"C2:C3\" si=\"0\">A2+1</f><v>338506591523</v></c></row>",
        "<row r=\"3\"><c r=\"A3\" s=\"2\"><v>338506590011</v></c><c r=\"B3\" s=\"2\" t=\"s\"><v>3</v></c><c r=\"C3\" s=\"1\"><f t=\"shared\" si=\"0\"/><v>338506590012</v></c></row>",
        "</sheetData></worksheet>",
    );
    const SHEET2: &str = concat!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>",
        "<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><sheetData>",
        "<row r=\"1\"><c r=\"A1\"><f>VLOOKUP(1,Telematik!A:C,2,0)</f><v>0</v></c></row>",
        "</sheetData></worksheet>",
    );

    fn package() -> Vec<u8> {
        let parts: [(&str, &str); 10] = [
            ("[Content_Types].xml", concat!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>",
                "<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">",
                "<Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>",
                "<Default Extension=\"xml\" ContentType=\"application/xml\"/>",
                "<Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/>",
                "<Override PartName=\"/xl/worksheets/sheet1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>",
                "<Override PartName=\"/xl/worksheets/sheet2.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>",
                "<Override PartName=\"/xl/theme/theme1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.theme+xml\"/>",
                "<Override PartName=\"/xl/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml\"/>",
                "<Override PartName=\"/xl/sharedStrings.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml\"/>",
                "<Override PartName=\"/xl/calcChain.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.calcChain+xml\"/>",
                "</Types>",
            )),
            ("_rels/.rels", concat!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>",
                "<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">",
                "<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"xl/workbook.xml\"/>",
                "</Relationships>",
            )),
            ("xl/workbook.xml", concat!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>",
                "<workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\">",
                "<sheets><sheet name=\"Telematik\" sheetId=\"1\" r:id=\"rId1\"/><sheet name=\"Überblick\" sheetId=\"2\" r:id=\"rId2\"/></sheets>",
                "<calcPr calcId=\"191029\"/></workbook>",
            )),
            ("xl/_rels/workbook.xml.rels", concat!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>",
                "<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">",
                "<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet1.xml\"/>",
                "<Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet2.xml\"/>",
                "<Relationship Id=\"rId3\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme\" Target=\"theme/theme1.xml\"/>",
                "<Relationship Id=\"rId4\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/>",
                "<Relationship Id=\"rId5\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings\" Target=\"sharedStrings.xml\"/>",
                "<Relationship Id=\"rId6\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/calcChain\" Target=\"calcChain.xml\"/>",
                "</Relationships>",
            )),
            ("xl/worksheets/sheet1.xml", SHEET1),
            ("xl/worksheets/sheet2.xml", SHEET2),
            ("xl/theme/theme1.xml", "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><a:theme xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" name=\"Office\"><a:themeElements/></a:theme>"),
            ("xl/styles.xml", STYLES),
            ("xl/sharedStrings.xml", concat!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>",
                "<sst xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" count=\"5\" uniqueCount=\"4\">",
                "<si><t>Asset</t></si><si><t>Stadt</t></si><si><t>Plus</t></si><si><t>Altstadt</t></si></sst>",
            )),
            ("xl/calcChain.xml", "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><calcChain xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><c r=\"C2\" i=\"1\"/><c r=\"C3\"/><c r=\"A1\" i=\"2\"/></calcChain>"),
        ];
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (name, text) in parts {
            writer.start_file(name, options).unwrap();
            writer.write_all(text.as_bytes()).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    fn file(folder: &TempDir) -> std::path::PathBuf {
        let path = folder.path().join("Master.xlsx");
        std::fs::write(&path, package()).unwrap();
        path
    }

    fn entries(path: &Path) -> Vec<package::Entry> {
        let bytes = std::fs::read(path).unwrap();
        Package::open(&bytes).unwrap().entries().unwrap()
    }

    fn part(path: &Path, name: &str) -> String {
        let bytes = std::fs::read(path).unwrap();
        Package::open(&bytes).unwrap().text(name).unwrap()
    }

    fn telematik(edits: Vec<CellEdit>) -> Vec<SheetEdits> {
        vec![SheetEdits {
            sheet: "Telematik".into(),
            edits,
        }]
    }

    #[test]
    fn only_the_edited_sheet_changes_and_every_other_part_is_bit_identical() {
        let folder = TempDir::new("patch-identical");
        let path = file(&folder);
        let before = entries(&path);

        write(
            &path,
            &telematik(vec![
                CellEdit {
                    col: 2,
                    row: 2,
                    content: Content::Text("Neuhof".into()),
                },
                CellEdit {
                    col: 1,
                    row: 4,
                    content: Content::Number(338_506_599_999.0),
                },
                CellEdit {
                    col: 2,
                    row: 4,
                    content: Content::Text("Fulda".into()),
                },
                CellEdit {
                    col: 3,
                    row: 4,
                    content: Content::Formula("A4+1".into()),
                },
            ]),
        )
        .unwrap();

        let after = entries(&path);
        let changed = [
            "xl/worksheets/sheet1.xml",
            "xl/workbook.xml",
            "xl/_rels/workbook.xml.rels",
            "[Content_Types].xml",
        ];
        for entry in &after {
            if changed.contains(&entry.name.as_str()) {
                continue;
            }
            // Styles, theme, shared strings and the other sheet: same CRC, same sizes.
            let old = before.iter().find(|old| old.name == entry.name).unwrap();
            assert_eq!(old, entry, "{}", entry.name);
        }
        // The regression itself: the style table is exactly what Excel wrote.
        assert_eq!(part(&path, "xl/styles.xml"), STYLES);
        assert!(!part(&path, "xl/styles.xml").contains("Standard 2\" xfId=\"1\" builtinId"));
    }

    #[test]
    fn the_calc_chain_goes_and_excel_is_told_to_recalculate() {
        let folder = TempDir::new("patch-calc");
        let path = file(&folder);
        write(
            &path,
            &telematik(vec![CellEdit {
                col: 2,
                row: 2,
                content: Content::Text("Neuhof".into()),
            }]),
        )
        .unwrap();

        let names: Vec<String> = entries(&path).into_iter().map(|entry| entry.name).collect();
        assert!(!names.contains(&"xl/calcChain.xml".to_string()));
        assert!(!part(&path, "xl/_rels/workbook.xml.rels").contains("calcChain"));
        assert!(!part(&path, "[Content_Types].xml").contains("calcChain"));
        assert!(part(&path, "xl/workbook.xml")
            .contains("<calcPr calcId=\"191029\" fullCalcOnLoad=\"1\"/>"));
    }

    #[test]
    fn the_written_values_read_back_through_umya() {
        let folder = TempDir::new("patch-readback");
        let path = file(&folder);
        write(
            &path,
            &telematik(vec![
                CellEdit {
                    col: 2,
                    row: 2,
                    content: Content::Text("Neuhof".into()),
                },
                CellEdit {
                    col: 1,
                    row: 4,
                    content: Content::Number(338_506_599_999.0),
                },
            ]),
        )
        .unwrap();
        let mut book = umya_spreadsheet::reader::xlsx::lazy_read(&path).unwrap();
        book.read_sheet(0);
        let sheet = book.sheet(0).unwrap();
        assert_eq!(sheet.cell((2u32, 2u32)).unwrap().value(), "Neuhof");
        assert_eq!(
            sheet.cell((1u32, 4u32)).unwrap().value_number(),
            Some(338_506_599_999.0)
        );
        // A cell nobody edited still reads its shared string.
        assert_eq!(sheet.cell((2u32, 3u32)).unwrap().value(), "Altstadt");
    }

    #[test]
    fn a_refused_patch_leaves_the_file_and_folder_untouched() {
        let folder = TempDir::new("patch-refused");
        let path = file(&folder);
        let original = std::fs::read(&path).unwrap();
        // Emptying the first cell of the shared group alone would orphan C3.
        let error = write(
            &path,
            &telematik(vec![CellEdit {
                col: 3,
                row: 2,
                content: Content::Empty,
            }]),
        )
        .unwrap_err()
        .into_messages();
        assert!(
            error.iter().any(|line| line.contains("geteilte Formel")),
            "{error:?}"
        );
        assert_eq!(std::fs::read(&path).unwrap(), original);
        assert_eq!(
            std::fs::read_dir(folder.path()).unwrap().count(),
            1,
            "no temp file left"
        );
    }

    #[test]
    fn an_unknown_sheet_is_said_and_nothing_written() {
        let folder = TempDir::new("patch-unknown");
        let path = file(&folder);
        let original = std::fs::read(&path).unwrap();
        let sheets = vec![SheetEdits {
            sheet: "Fehlt".into(),
            edits: vec![CellEdit {
                col: 1,
                row: 1,
                content: Content::Empty,
            }],
        }];
        let error = write(&path, &sheets).unwrap_err().into_messages();
        assert!(error[0].contains("kein Blatt „Fehlt“"), "{error:?}");
        assert_eq!(std::fs::read(&path).unwrap(), original);
    }

    #[test]
    fn no_edits_leave_the_file_byte_for_byte() {
        let folder = TempDir::new("patch-none");
        let path = file(&folder);
        let original = std::fs::read(&path).unwrap();
        write(&path, &telematik(Vec::new())).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), original);
    }

    // The checks themselves: a package whose untouched part changed, or whose
    // sheet names a style that does not exist, must not pass.
    #[test]
    fn verification_catches_a_changed_part_and_a_missing_style() {
        let original = package();
        let parts = Parts {
            sheets: vec!["xl/worksheets/sheet1.xml".into()],
            ..Parts::default()
        };
        assert!(verify(&original, &original, &parts).is_ok());

        let mut tampered = Package::open(&original).unwrap();
        let mut replaced = BTreeMap::new();
        replaced.insert(
            "xl/styles.xml".to_string(),
            STYLES.replace("Standard 2\"", "Standard 2\" builtinId=\"0\""),
        );
        let bytes = tampered.rebuild(&replaced, &BTreeSet::new()).unwrap();
        let error = verify(&original, &bytes, &parts)
            .unwrap_err()
            .into_messages();
        assert!(
            error[0].contains("xl/styles.xml wurde verändert"),
            "{error:?}"
        );

        let mut bad = Package::open(&original).unwrap();
        let mut sheet = BTreeMap::new();
        sheet.insert(
            "xl/worksheets/sheet1.xml".to_string(),
            SHEET1.replace("s=\"2\"", "s=\"9\""),
        );
        let bytes = bad.rebuild(&sheet, &BTreeSet::new()).unwrap();
        let parts = Parts {
            replaced: sheet,
            sheets: parts.sheets,
            ..Parts::default()
        };
        assert!(verify(&original, &bytes, &parts).is_err());
    }
}
