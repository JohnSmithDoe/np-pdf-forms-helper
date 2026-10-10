// ─── why ────────────────────────────────────────────────────────
// The `.xlsx` as what it is: a ZIP of XML parts. Every entry the patch does
// not replace is copied with `raw_copy_file` — the COMPRESSED bytes as they
// were, never inflated and deflated again — so styles, theme, shared strings,
// every other sheet, drawings and whatever an Excel version adds next come
// out bit-identical, CRC included. That is the whole point of this writer:
// umya re-serialised parts it did not understand, and stamped every cell
// style `builtinId="0"`, which Excel repairs.
//
// A sheet is found the way Excel finds it: its NAME in `workbook.xml`, the
// `r:id` there, the target in the workbook's rels — never by guessing the
// part name from a position. Names are compared unescaped (`&amp;`).
//
// Edited values invalidate Excel's cached results everywhere a formula reads
// them, so a patched book drops `calcChain.xml` — a stale chain naming a cell
// that is no formula any more is itself a repair — with its relationship and
// content type, and sets `fullCalcOnLoad` on `calcPr`, inserted at its place in
// the schema's order if the book had none.
// ────────────────────────────────────────────────────────────────

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read, Write};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use super::sheet::{attribute, find_element, tag_end};
use crate::error::{AppError, AppResult};

pub const WORKBOOK_RELS_TYPE_CALC_CHAIN: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/calcChain";
const CONTENT_TYPES: &str = "[Content_Types].xml";

pub struct Package<'a> {
    archive: ZipArchive<Cursor<&'a [u8]>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub crc32: u32,
    pub size: u64,
    pub compressed: u64,
}

impl<'a> Package<'a> {
    pub fn open(bytes: &'a [u8]) -> AppResult<Self> {
        ZipArchive::new(Cursor::new(bytes))
            .map(|archive| Self { archive })
            .map_err(|error| unreadable(&error))
    }

    pub fn entries(&mut self) -> AppResult<Vec<Entry>> {
        (0..self.archive.len())
            .map(|index| {
                let file = self
                    .archive
                    .by_index_raw(index)
                    .map_err(|e| unreadable(&e))?;
                Ok(Entry {
                    name: file.name().to_string(),
                    crc32: file.crc32(),
                    size: file.size(),
                    compressed: file.compressed_size(),
                })
            })
            .collect()
    }

    pub fn text(&mut self, name: &str) -> AppResult<String> {
        let mut file = self.archive.by_name(name).map_err(|e| unreadable(&e))?;
        let mut text = String::new();
        file.read_to_string(&mut text).map_err(|e| unreadable(&e))?;
        Ok(text)
    }

    pub fn has(&self, name: &str) -> bool {
        self.archive.index_for_name(name).is_some()
    }

    pub fn rebuild(
        &mut self,
        replaced: &BTreeMap<String, String>,
        dropped: &BTreeSet<String>,
    ) -> AppResult<Vec<u8>> {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for index in 0..self.archive.len() {
            let name = self
                .archive
                .by_index_raw(index)
                .map_err(|e| unreadable(&e))?
                .name()
                .to_string();
            if dropped.contains(&name) {
                continue;
            }
            match replaced.get(&name) {
                Some(text) => {
                    writer
                        .start_file(name.as_str(), options)
                        .map_err(|e| unwritable(&e))?;
                    writer
                        .write_all(text.as_bytes())
                        .map_err(|e| unwritable(&e))?;
                }
                None => {
                    let file = self
                        .archive
                        .by_index_raw(index)
                        .map_err(|e| unreadable(&e))?;
                    writer.raw_copy_file(file).map_err(|e| unwritable(&e))?;
                }
            }
        }
        writer
            .finish()
            .map(Cursor::into_inner)
            .map_err(|e| unwritable(&e))
    }
}

pub fn workbook_part(root_rels: &str) -> Option<String> {
    relationships(root_rels)
        .into_iter()
        .find(|(_, kind, _)| kind.ends_with("/officeDocument"))
        .map(|(_, _, target)| resolve("", &target))
}

pub fn rels_of(part: &str) -> String {
    let (folder, file) = part.rsplit_once('/').unwrap_or(("", part));
    if folder.is_empty() {
        format!("_rels/{file}.rels")
    } else {
        format!("{folder}/_rels/{file}.rels")
    }
}

pub fn sheet_part(workbook: &str, workbook_part: &str, rels: &str, name: &str) -> Option<String> {
    let mut at = 0;
    let id = loop {
        let found = find_element(workbook, "sheet", at)?;
        let end = tag_end(workbook, found)?;
        let tag = &workbook[found..=end];
        if attribute(tag, "name").map(unescape).as_deref() == Some(name) {
            break relationship_id(tag)?.to_string();
        }
        at = end + 1;
    };
    let folder = workbook_part
        .rsplit_once('/')
        .map_or("", |(folder, _)| folder);
    relationships(rels)
        .into_iter()
        .find(|(own, _, _)| *own == id)
        .map(|(_, _, target)| resolve(folder, &target))
}

pub fn calc_chain(workbook_part: &str, rels: &str) -> Option<(String, String)> {
    let folder = workbook_part
        .rsplit_once('/')
        .map_or("", |(folder, _)| folder);
    relationships(rels)
        .into_iter()
        .find(|(_, kind, _)| kind == WORKBOOK_RELS_TYPE_CALC_CHAIN)
        .map(|(id, _, target)| (id, resolve(folder, &target)))
}

pub fn without_relationship(rels: &str, id: &str) -> String {
    remove_element(rels, "Relationship", |tag| attribute(tag, "Id") == Some(id))
}

pub fn without_override(types: &str, part: &str) -> String {
    let name = format!("/{part}");
    remove_element(types, "Override", |tag| {
        attribute(tag, "PartName") == Some(name.as_str())
    })
}

pub fn full_calc_on_load(workbook: &str) -> AppResult<String> {
    if let Some(at) = find_element(workbook, "calcPr", 0) {
        let end = tag_end(workbook, at).ok_or_else(|| broken("calcPr"))?;
        let tag = &workbook[at..=end];
        let patched = match attribute(tag, "fullCalcOnLoad") {
            Some(_) => tag
                .replacen("fullCalcOnLoad=\"0\"", "fullCalcOnLoad=\"1\"", 1)
                .replacen("fullCalcOnLoad=\"false\"", "fullCalcOnLoad=\"1\"", 1),
            None => {
                let cut = if tag.ends_with("/>") {
                    tag.len() - 2
                } else {
                    tag.len() - 1
                };
                format!(
                    "{} fullCalcOnLoad=\"1\"{}",
                    tag[..cut].trim_end(),
                    &tag[cut..]
                )
            }
        };
        return Ok(format!(
            "{}{}{}",
            &workbook[..at],
            patched,
            &workbook[end + 1..]
        ));
    }
    let after = [
        "definedNames",
        "externalReferences",
        "functionGroups",
        "sheets",
    ]
    .iter()
    .find_map(|name| element_end(workbook, name))
    .ok_or_else(|| broken("sheets"))?;
    Ok(format!(
        "{}<calcPr fullCalcOnLoad=\"1\"/>{}",
        &workbook[..after],
        &workbook[after..]
    ))
}

pub fn content_types() -> &'static str {
    CONTENT_TYPES
}

fn relationships(rels: &str) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(found) = find_element(rels, "Relationship", at) {
        let Some(end) = tag_end(rels, found) else {
            break;
        };
        let tag = &rels[found..=end];
        if attribute(tag, "TargetMode") != Some("External") {
            if let (Some(id), Some(kind), Some(target)) = (
                attribute(tag, "Id"),
                attribute(tag, "Type"),
                attribute(tag, "Target"),
            ) {
                out.push((id.to_string(), kind.to_string(), unescape(target)));
            }
        }
        at = end + 1;
    }
    out
}

fn relationship_id(tag: &str) -> Option<&str> {
    attribute(tag, "r:id").or_else(|| {
        let at = tag.find(":id=")?;
        let start = tag[..at].rfind(char::is_whitespace)? + 1;
        attribute(tag, &tag[start..at + 3])
    })
}

fn resolve(folder: &str, target: &str) -> String {
    if let Some(absolute) = target.strip_prefix('/') {
        return absolute.to_string();
    }
    let mut parts: Vec<&str> = folder.split('/').filter(|part| !part.is_empty()).collect();
    for part in target.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            part => parts.push(part),
        }
    }
    parts.join("/")
}

fn element_end(xml: &str, name: &str) -> Option<usize> {
    let at = find_element(xml, name, 0)?;
    let end = tag_end(xml, at)?;
    if xml[..end].ends_with('/') {
        return Some(end + 1);
    }
    let close = format!("</{name}>");
    xml[end..]
        .find(&close)
        .map(|found| end + found + close.len())
}

fn remove_element(xml: &str, name: &str, matches: impl Fn(&str) -> bool) -> String {
    let mut at = 0;
    while let Some(found) = find_element(xml, name, at) {
        let Some(end) = tag_end(xml, found) else {
            break;
        };
        let tag = &xml[found..=end];
        if matches(tag) && tag.ends_with("/>") {
            return format!("{}{}", &xml[..found], &xml[end + 1..]);
        }
        at = end + 1;
    }
    xml.to_string()
}

fn unescape(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

fn unreadable(cause: &dyn std::fmt::Display) -> AppError {
    AppError::detail(
        "Die Excel-Datei konnte nicht als Paket gelesen werden.".into(),
        cause,
    )
}

fn unwritable(cause: &dyn std::fmt::Display) -> AppError {
    AppError::detail(
        "Die Excel-Datei konnte nicht neu gepackt werden.".into(),
        cause,
    )
}

fn broken(element: &str) -> AppError {
    AppError::Report(vec![format!(
        "Die Arbeitsmappe ist unvollständig ({element}); es wurde nichts geändert."
    )])
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORKBOOK: &str = concat!(
        "<workbook xmlns:r=\"r\"><sheets>",
        "<sheet name=\"Alle Wagen\" sheetId=\"1\" r:id=\"rId1\"/>",
        "<sheet r:id=\"rId2\" name=\"A &amp; B\" sheetId=\"2\"/>",
        "</sheets><definedNames><definedName name=\"x\">1</definedName></definedNames>",
        "<calcPr calcId=\"191029\"/></workbook>",
    );
    const RELS: &str = concat!(
        "<Relationships>",
        "<Relationship Id=\"rId1\" Type=\"t/worksheet\" Target=\"worksheets/sheet1.xml\"/>",
        "<Relationship Id=\"rId2\" Type=\"t/worksheet\" Target=\"/xl/worksheets/sheet7.xml\"/>",
        "<Relationship Id=\"rId9\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/calcChain\" Target=\"calcChain.xml\"/>",
        "</Relationships>",
    );

    #[test]
    fn a_sheet_is_found_by_name_through_its_relationship() {
        assert_eq!(
            sheet_part(WORKBOOK, "xl/workbook.xml", RELS, "Alle Wagen").as_deref(),
            Some("xl/worksheets/sheet1.xml")
        );
        // Escaped name, attributes in another order, an absolute target.
        assert_eq!(
            sheet_part(WORKBOOK, "xl/workbook.xml", RELS, "A & B").as_deref(),
            Some("xl/worksheets/sheet7.xml")
        );
        assert_eq!(sheet_part(WORKBOOK, "xl/workbook.xml", RELS, "Fehlt"), None);
    }

    #[test]
    fn the_workbook_part_comes_from_the_root_relationships() {
        let root = "<Relationships><Relationship Id=\"rId1\" Type=\"x/officeDocument\" Target=\"xl/workbook.xml\"/></Relationships>";
        assert_eq!(workbook_part(root).as_deref(), Some("xl/workbook.xml"));
        assert_eq!(rels_of("xl/workbook.xml"), "xl/_rels/workbook.xml.rels");
        assert_eq!(resolve("xl", "../docProps/app.xml"), "docProps/app.xml");
    }

    #[test]
    fn the_calc_chain_goes_with_its_relationship_and_content_type() {
        let (id, part) = calc_chain("xl/workbook.xml", RELS).unwrap();
        assert_eq!((id.as_str(), part.as_str()), ("rId9", "xl/calcChain.xml"));
        let rels = without_relationship(RELS, &id);
        assert!(!rels.contains("calcChain") && rels.contains("rId1") && rels.contains("rId2"));
        let types = "<Types><Override PartName=\"/xl/calcChain.xml\" ContentType=\"c\"/><Override PartName=\"/xl/workbook.xml\" ContentType=\"w\"/></Types>";
        assert_eq!(
            without_override(types, &part),
            "<Types><Override PartName=\"/xl/workbook.xml\" ContentType=\"w\"/></Types>"
        );
    }

    #[test]
    fn full_calc_on_load_is_set_or_inserted_in_schema_order() {
        assert!(full_calc_on_load(WORKBOOK)
            .unwrap()
            .contains("<calcPr calcId=\"191029\" fullCalcOnLoad=\"1\"/>"));
        let none = "<workbook><sheets><sheet/></sheets><definedNames/><x/></workbook>";
        assert_eq!(
            full_calc_on_load(none).unwrap(),
            "<workbook><sheets><sheet/></sheets><definedNames/><calcPr fullCalcOnLoad=\"1\"/><x/></workbook>"
        );
        let named = "<workbook><sheets/><definedNames><d/></definedNames><oleSize/></workbook>";
        assert_eq!(
            full_calc_on_load(named).unwrap(),
            "<workbook><sheets/><definedNames><d/></definedNames><calcPr fullCalcOnLoad=\"1\"/><oleSize/></workbook>"
        );
        let bare = "<workbook><sheets><sheet/></sheets></workbook>";
        assert_eq!(
            full_calc_on_load(bare).unwrap(),
            "<workbook><sheets><sheet/></sheets><calcPr fullCalcOnLoad=\"1\"/></workbook>"
        );
        let off = "<workbook><calcPr fullCalcOnLoad=\"0\"/></workbook>";
        assert_eq!(
            full_calc_on_load(off).unwrap(),
            "<workbook><calcPr fullCalcOnLoad=\"1\"/></workbook>"
        );
    }
}
