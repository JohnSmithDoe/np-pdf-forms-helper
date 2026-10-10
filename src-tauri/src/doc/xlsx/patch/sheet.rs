// ─── why ────────────────────────────────────────────────────────
// Cell edits into ONE worksheet's XML, as text, touching nothing else. This is
// the half of the patch writer that decides bytes: every row the edits do not
// reach is copied verbatim, every cell they do not reach inside a touched row
// too, and nothing outside `<sheetData>` changes except the `<dimension>`
// growing to cover appended rows.
//
// No style is ever looked up or translated: an edited cell keeps its own `s`,
// a NEW cell takes the `s` of the same column in row 2 — the paste's own rule
// for new rows — so the workbook's style table can be left exactly as Excel
// wrote it. Text goes in as an inline string, so `sharedStrings.xml` is never
// touched either; Excel folds it into its table on the next save.
//
// It REFUSES rather than guesses. A row or cell without its `r`, a prefixed
// element name, cells out of order, an edit inside an array formula, or a
// shared-formula group only partly edited — the group's text lives in its
// first cell, so editing some members orphans or rewrites the others — is an
// error, and the caller writes nothing. A wrong guess here is a file Excel
// repairs or a cell that silently reads another row.
//
// Text is escaped for XML and for Excel's own `_xHHHH_` scheme: a literal
// `_x0041_` must not come back as `A`, and a control character XML cannot
// carry goes in as its `_xHHHH_` form.
// ────────────────────────────────────────────────────────────────

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, PartialEq)]
pub enum Content {
    Empty,
    Number(f64),
    Text(String),
    Bool(bool),
    Formula(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CellEdit {
    pub col: u32,
    pub row: u32,
    pub content: Content,
}

struct RawCell<'a> {
    col: u32,
    start: usize,
    end: usize,
    style: Option<&'a str>,
    shared: Option<(&'a str, bool)>,
    array: Option<(u32, u32, u32, u32)>,
}

struct RawRow<'a> {
    r: u32,
    start: usize,
    end: usize,
    tag: &'a str,
    cells: Vec<RawCell<'a>>,
}

pub fn patch(xml: &str, edits: &[CellEdit]) -> AppResult<String> {
    if edits.is_empty() {
        return Ok(xml.to_string());
    }
    let (open, open_end, content_start, content_end, closing) = sheet_data(xml)?;
    let rows = rows(xml, content_start, content_end)?;
    let wanted = by_position(edits)?;
    check_groups(&rows, &wanted)?;

    let row_two: HashMap<u32, &str> = rows
        .iter()
        .find(|row| row.r == 2)
        .map(|row| {
            row.cells
                .iter()
                .filter_map(|cell| cell.style.map(|style| (cell.col, style)))
                .collect()
        })
        .unwrap_or_default();

    let edited_rows: BTreeSet<u32> = wanted.keys().map(|(row, _)| *row).collect();
    let mut numbers: BTreeSet<u32> = rows.iter().map(|row| row.r).collect();
    numbers.extend(edited_rows.iter().copied());

    let mut body = String::with_capacity(content_end - content_start + edits.len() * 64);
    for number in numbers {
        let existing = rows.iter().find(|row| row.r == number);
        if !edited_rows.contains(&number) {
            if let Some(row) = existing {
                body.push_str(&xml[row.start..row.end]);
            }
            continue;
        }
        let edits: BTreeMap<u32, &Content> = wanted
            .range((number, 0)..=(number, u32::MAX))
            .map(|((_, col), content)| (*col, *content))
            .collect();
        match existing {
            Some(row) => {
                let tag = without_attribute(
                    row.tag.trim_end_matches("/>").trim_end_matches('>'),
                    "spans",
                );
                body.push_str(&tag);
                body.push('>');
                let mut pending = edits.iter().peekable();
                for cell in &row.cells {
                    while let Some((col, content)) = pending.peek() {
                        if **col >= cell.col {
                            break;
                        }
                        body.push_str(&render(**col, number, row_two.get(col).copied(), content)?);
                        pending.next();
                    }
                    match pending.peek() {
                        Some((col, content)) if **col == cell.col => {
                            body.push_str(&render(cell.col, number, cell.style, content)?);
                            pending.next();
                        }
                        _ => body.push_str(&xml[cell.start..cell.end]),
                    }
                }
                for (col, content) in pending {
                    body.push_str(&render(*col, number, row_two.get(col).copied(), content)?);
                }
                body.push_str("</row>");
            }
            None => {
                body.push_str(&format!("<row r=\"{number}\">"));
                for (col, content) in &edits {
                    body.push_str(&render(*col, number, row_two.get(col).copied(), content)?);
                }
                body.push_str("</row>");
            }
        }
    }

    let mut out = String::with_capacity(xml.len() + body.len());
    if closing {
        out.push_str(&xml[..content_start]);
        out.push_str(&body);
        out.push_str(&xml[content_end..]);
    } else {
        let tag = xml[open..open_end].trim_end_matches('/');
        out.push_str(&xml[..open]);
        out.push_str(tag);
        out.push('>');
        out.push_str(&body);
        out.push_str("</sheetData>");
        out.push_str(&xml[open_end + 1..]);
    }
    grow_dimension(&out, edits)
}

pub fn check(xml: &str, styles: Option<usize>) -> AppResult<()> {
    let (_, _, start, end, _) = sheet_data(xml)?;
    for row in rows(xml, start, end)? {
        for cell in &row.cells {
            let (Some(style), Some(count)) = (cell.style, styles) else {
                continue;
            };
            let index: usize = style
                .parse()
                .map_err(|_| refused("ein Zellformat ohne Nummer"))?;
            if index >= count {
                return Err(refused(&format!(
                    "Zeile {} verweist auf ein Zellformat, das es nicht gibt",
                    row.r
                )));
            }
        }
    }
    Ok(())
}

pub fn cell_xfs_count(styles: &str) -> Option<usize> {
    let open = find_element(styles, "cellXfs", 0)?;
    let open_end = tag_end(styles, open)?;
    if styles[..open_end].ends_with('/') {
        return Some(0);
    }
    let close = open_end + styles[open_end..].find("</cellXfs>")?;
    let mut count = 0;
    let mut at = open_end;
    while let Some(found) = find_element(styles, "xf", at) {
        if found >= close {
            break;
        }
        count += 1;
        at = found + 1;
    }
    Some(count)
}

pub fn needs_excel_escape(text: &str) -> bool {
    escape_text(text) != escape_xml(text)
}

pub fn letters(col: u32) -> String {
    let mut col = col;
    let mut out = Vec::new();
    while col > 0 {
        out.push(b'A' + ((col - 1) % 26) as u8);
        col = (col - 1) / 26;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

pub fn parse_ref(reference: &str) -> Option<(u32, u32)> {
    let reference = reference.trim().replace('$', "");
    let split = reference.find(|ch: char| ch.is_ascii_digit())?;
    let (letters, digits) = reference.split_at(split);
    if letters.is_empty() || !letters.chars().all(|ch| ch.is_ascii_uppercase()) {
        return None;
    }
    let col = letters.bytes().try_fold(0u32, |acc, byte| {
        acc.checked_mul(26)?.checked_add(u32::from(byte - b'A' + 1))
    })?;
    let row = digits.parse().ok()?;
    (col > 0 && row > 0).then_some((col, row))
}

type SheetData = (usize, usize, usize, usize, bool);

fn sheet_data(xml: &str) -> AppResult<SheetData> {
    let open = find_element(xml, "sheetData", 0).ok_or_else(|| refused("kein <sheetData>"))?;
    let open_end = tag_end(xml, open).ok_or_else(|| refused("<sheetData> ohne Ende"))?;
    if xml[..open_end].ends_with('/') {
        return Ok((open, open_end, open_end + 1, open_end + 1, false));
    }
    let close = xml[open_end..]
        .find("</sheetData>")
        .map(|at| open_end + at)
        .ok_or_else(|| refused("</sheetData> fehlt"))?;
    Ok((open, open_end, open_end + 1, close, true))
}

fn refused(detail: &str) -> AppError {
    AppError::Report(vec![
        "Das Blatt kann nicht sicher geschrieben werden; es wurde nichts geändert.".into(),
        detail.to_string(),
    ])
}

fn rows(xml: &str, start: usize, end: usize) -> AppResult<Vec<RawRow<'_>>> {
    let mut rows: Vec<RawRow<'_>> = Vec::new();
    let mut at = skip_space(xml, start);
    while at < end {
        if !is_element(xml, at, "row") {
            return Err(refused("unerwartetes Element in <sheetData>"));
        }
        let tag_close = tag_end(xml, at).ok_or_else(|| refused("<row> ohne Ende"))?;
        let tag = &xml[at..=tag_close];
        let r: u32 = attribute(tag, "r")
            .and_then(|r| r.parse().ok())
            .ok_or_else(|| refused("eine Zeile ohne Nummer"))?;
        if rows.last().is_some_and(|last| last.r >= r) {
            return Err(refused("Zeilen nicht in Reihenfolge"));
        }
        let closed = tag.ends_with("/>");
        let mut cells = Vec::new();
        let mut cursor = tag_close + 1;
        let row_end = if closed {
            cursor
        } else {
            loop {
                cursor = skip_space(xml, cursor);
                if xml[cursor..].starts_with("</row>") {
                    break cursor + "</row>".len();
                }
                if !is_element(xml, cursor, "c") {
                    return Err(refused("unerwartetes Element in einer Zeile"));
                }
                let cell = cell(xml, cursor, r)?;
                if cells
                    .last()
                    .is_some_and(|last: &RawCell<'_>| last.col >= cell.col)
                {
                    return Err(refused("Zellen nicht in Reihenfolge"));
                }
                cursor = cell.end;
                cells.push(cell);
            }
        };
        rows.push(RawRow {
            r,
            start: at,
            end: row_end,
            tag,
            cells,
        });
        at = skip_space(xml, row_end);
    }
    Ok(rows)
}

fn cell(xml: &str, at: usize, row: u32) -> AppResult<RawCell<'_>> {
    let tag_close = tag_end(xml, at).ok_or_else(|| refused("<c> ohne Ende"))?;
    let tag = &xml[at..=tag_close];
    let (col, own_row) = attribute(tag, "r")
        .and_then(parse_ref)
        .ok_or_else(|| refused("eine Zelle ohne Adresse"))?;
    if own_row != row {
        return Err(refused("eine Zelle in der falschen Zeile"));
    }
    let (end, inner) = if tag.ends_with("/>") {
        (tag_close + 1, "")
    } else {
        let close = xml[tag_close..]
            .find("</c>")
            .map(|found| tag_close + found)
            .ok_or_else(|| refused("</c> fehlt"))?;
        (close + "</c>".len(), &xml[tag_close + 1..close])
    };
    let mut shared = None;
    let mut array = None;
    if let Some(found) = find_element(inner, "f", 0) {
        let formula =
            &inner[found..=tag_end(inner, found).ok_or_else(|| refused("<f> ohne Ende"))?];
        match attribute(formula, "t") {
            Some("shared") => {
                let si =
                    attribute(formula, "si").ok_or_else(|| refused("geteilte Formel ohne si"))?;
                shared = Some((si, attribute(formula, "ref").is_some()));
            }
            Some("array") => {
                let range = attribute(formula, "ref").unwrap_or("");
                let (from, to) = range.split_once(':').unwrap_or((range, range));
                let (c1, r1) =
                    parse_ref(from).ok_or_else(|| refused("Matrixformel ohne Bereich"))?;
                let (c2, r2) = parse_ref(to).ok_or_else(|| refused("Matrixformel ohne Bereich"))?;
                array = Some((c1, r1, c2, r2));
            }
            _ => {}
        }
    }
    Ok(RawCell {
        col,
        start: at,
        end,
        style: attribute(tag, "s"),
        shared,
        array,
    })
}

fn by_position(edits: &[CellEdit]) -> AppResult<BTreeMap<(u32, u32), &Content>> {
    let mut wanted = BTreeMap::new();
    for edit in edits {
        if edit.col == 0 || edit.row == 0 {
            return Err(refused("eine Änderung ohne Adresse"));
        }
        if wanted.insert((edit.row, edit.col), &edit.content).is_some() {
            return Err(refused("eine Zelle zweimal geändert"));
        }
    }
    Ok(wanted)
}

fn check_groups(rows: &[RawRow<'_>], wanted: &BTreeMap<(u32, u32), &Content>) -> AppResult<()> {
    let mut groups: HashMap<&str, Vec<(u32, u32)>> = HashMap::new();
    for row in rows {
        for cell in &row.cells {
            if let Some((si, _)) = cell.shared {
                groups.entry(si).or_default().push((row.r, cell.col));
            }
            if let Some((c1, r1, c2, r2)) = cell.array {
                let inside = wanted
                    .keys()
                    .any(|(row, col)| (r1..=r2).contains(row) && (c1..=c2).contains(col));
                if inside {
                    return Err(refused("eine Änderung in einer Matrixformel"));
                }
            }
        }
    }
    for members in groups.values() {
        let touched = members
            .iter()
            .filter(|member| wanted.contains_key(member))
            .count();
        if touched > 0 && touched < members.len() {
            let (row, col) = members[0];
            return Err(refused(&format!(
                "die geteilte Formel ab {}{row} würde nur zum Teil geändert",
                letters(col)
            )));
        }
    }
    Ok(())
}

fn render(col: u32, row: u32, style: Option<&str>, content: &Content) -> AppResult<String> {
    let mut out = format!("<c r=\"{}{row}\"", letters(col));
    if let Some(style) = style {
        out.push_str(&format!(" s=\"{style}\""));
    }
    match content {
        Content::Empty => out.push_str("/>"),
        Content::Number(number) => {
            if !number.is_finite() {
                return Err(refused("eine Zahl, die keine ist"));
            }
            out.push_str(&format!("><v>{number}</v></c>"));
        }
        Content::Bool(value) => out.push_str(&format!(" t=\"b\"><v>{}</v></c>", u8::from(*value))),
        Content::Text(text) => out.push_str(&format!(
            " t=\"inlineStr\"><is><t xml:space=\"preserve\">{}</t></is></c>",
            escape_text(text)
        )),
        Content::Formula(formula) => {
            if formula.trim().is_empty() {
                return Err(refused("eine Formel ohne Text"));
            }
            out.push_str(&format!("><f>{}</f></c>", escape_xml(formula)));
        }
    }
    Ok(out)
}

fn grow_dimension(xml: &str, edits: &[CellEdit]) -> AppResult<String> {
    let Some(at) = find_element(xml, "dimension", 0) else {
        return Ok(xml.to_string());
    };
    let end = tag_end(xml, at).ok_or_else(|| refused("<dimension> ohne Ende"))?;
    let tag = &xml[at..=end];
    let Some(range) = attribute(tag, "ref") else {
        return Ok(xml.to_string());
    };
    let (from, to) = range.split_once(':').unwrap_or((range, range));
    let (Some((c1, r1)), Some((c2, r2))) = (parse_ref(from), parse_ref(to)) else {
        return Ok(xml.to_string());
    };
    let filled = edits.iter().filter(|edit| edit.content != Content::Empty);
    let col = filled
        .clone()
        .map(|edit| edit.col)
        .max()
        .unwrap_or(0)
        .max(c2);
    let row = filled.map(|edit| edit.row).max().unwrap_or(0).max(r2);
    if (col, row) == (c2, r2) {
        return Ok(xml.to_string());
    }
    let grown = format!("{}{r1}:{}{row}", letters(c1), letters(col));
    let new_tag = tag.replacen(&format!("\"{range}\""), &format!("\"{grown}\""), 1);
    Ok(format!("{}{}{}", &xml[..at], new_tag, &xml[end + 1..]))
}

fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        let literal_escape = ch == '_'
            && i + 6 < chars.len()
            && chars[i + 1] == 'x'
            && chars[i + 2..i + 6].iter().all(char::is_ascii_hexdigit)
            && chars[i + 6] == '_';
        if literal_escape {
            out.push_str("_x005F_");
            i += 1;
            continue;
        }
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\t' | '\n' | '\r' => out.push(ch),
            ch if (ch as u32) < 0x20 || ch == '\u{FFFE}' || ch == '\u{FFFF}' => {
                out.push_str(&format!("_x{:04X}_", ch as u32));
            }
            ch => out.push(ch),
        }
        i += 1;
    }
    out
}

fn skip_space(xml: &str, from: usize) -> usize {
    from + xml[from..].len() - xml[from..].trim_start().len()
}

fn is_element(xml: &str, at: usize, name: &str) -> bool {
    let rest = &xml[at..];
    rest.starts_with('<')
        && rest[1..].starts_with(name)
        && rest[1 + name.len()..]
            .chars()
            .next()
            .is_some_and(|ch| ch == '>' || ch == '/' || ch.is_whitespace())
}

pub(super) fn find_element(xml: &str, name: &str, from: usize) -> Option<usize> {
    let mut at = from;
    while let Some(found) = xml[at..].find('<') {
        let pos = at + found;
        if is_element(xml, pos, name) {
            return Some(pos);
        }
        at = pos + 1;
    }
    None
}

pub(super) fn tag_end(xml: &str, from: usize) -> Option<usize> {
    let mut quote: Option<char> = None;
    for (offset, ch) in xml[from..].char_indices() {
        match (quote, ch) {
            (Some(open), ch) if ch == open => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(ch),
            (None, '>') => return Some(from + offset),
            (None, _) => {}
        }
    }
    None
}

pub(super) fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let mut rest = tag.trim_start_matches('<');
    rest = rest.trim_start_matches(|ch: char| !ch.is_whitespace() && ch != '>' && ch != '/');
    loop {
        rest = rest.trim_start();
        let eq = rest.find('=')?;
        let key = rest[..eq].trim();
        let after = rest[eq + 1..].trim_start();
        let quote = after.chars().next()?;
        if quote != '"' && quote != '\'' {
            return None;
        }
        let close = after[1..].find(quote)? + 1;
        if key == name {
            return Some(&after[1..close]);
        }
        rest = &after[close + 1..];
    }
}

fn without_attribute(tag: &str, name: &str) -> String {
    let Some(value) = attribute(tag, name) else {
        return tag.to_string();
    };
    for quote in ['"', '\''] {
        let whole = format!(" {name}={quote}{value}{quote}");
        if tag.contains(&whole) {
            return tag.replacen(&whole, "", 1);
        }
    }
    tag.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sheet(data: &str) -> String {
        format!(
            "<?xml version=\"1.0\"?><worksheet xmlns=\"x\"><dimension ref=\"A1:C3\"/><sheetData>{data}</sheetData><mergeCells count=\"0\"/></worksheet>"
        )
    }

    fn edit(col: u32, row: u32, content: Content) -> CellEdit {
        CellEdit { col, row, content }
    }

    const BASE: &str = concat!(
        "<row r=\"1\" spans=\"1:3\"><c r=\"A1\" s=\"1\" t=\"s\"><v>0</v></c><c r=\"B1\" s=\"1\" t=\"s\"><v>1</v></c></row>",
        "<row r=\"2\" spans=\"1:3\"><c r=\"A2\" s=\"5\"><v>10</v></c><c r=\"B2\" s=\"6\" t=\"s\"><v>2</v></c><c r=\"C2\" s=\"7\"><f>A2*2</f><v>20</v></c></row>",
        "<row r=\"3\" spans=\"1:3\"><c r=\"A3\" s=\"5\"><v>11</v></c><c r=\"C3\" s=\"7\"><f>A3*2</f><v>22</v></c></row>",
    );

    #[test]
    fn nothing_to_do_is_the_same_bytes() {
        let xml = sheet(BASE);
        assert_eq!(patch(&xml, &[]).unwrap(), xml);
    }

    #[test]
    fn an_edited_cell_keeps_its_style_and_untouched_rows_keep_their_bytes() {
        let xml = sheet(BASE);
        let out = patch(&xml, &[edit(2, 2, Content::Text("Neuhof & Co".into()))]).unwrap();
        // Rows 1 and 3 are copied byte for byte, spans and all.
        assert!(out.contains(&BASE[..BASE.find("<row r=\"2\"").unwrap()]));
        assert!(out.contains("<row r=\"3\" spans=\"1:3\">"));
        // The edited cell keeps s="6" and goes in as an inline string.
        assert!(out.contains(
            "<c r=\"B2\" s=\"6\" t=\"inlineStr\"><is><t xml:space=\"preserve\">Neuhof &amp; Co</t></is></c>"
        ));
        // Its neighbours in the same row are untouched, the formula included.
        assert!(out.contains("<c r=\"A2\" s=\"5\"><v>10</v></c>"));
        assert!(out.contains("<c r=\"C2\" s=\"7\"><f>A2*2</f><v>20</v></c>"));
        // Everything outside <sheetData> is unchanged.
        assert!(out.ends_with("</sheetData><mergeCells count=\"0\"/></worksheet>"));
        assert!(out.contains("<dimension ref=\"A1:C3\"/>"));
    }

    #[test]
    fn a_new_row_takes_row_two_styles_and_grows_the_dimension() {
        let xml = sheet(BASE);
        let out = patch(
            &xml,
            &[
                edit(1, 5, Content::Number(12.5)),
                edit(3, 5, Content::Formula("A5*2".into())),
                edit(2, 5, Content::Bool(true)),
            ],
        )
        .unwrap();
        assert!(out.contains(
            "<row r=\"5\"><c r=\"A5\" s=\"5\"><v>12.5</v></c><c r=\"B5\" s=\"6\" t=\"b\"><v>1</v></c><c r=\"C5\" s=\"7\"><f>A5*2</f></c></row></sheetData>"
        ));
        assert!(out.contains("<dimension ref=\"A1:C5\"/>"));
    }

    #[test]
    fn a_cell_missing_from_a_row_is_inserted_in_column_order() {
        let xml = sheet(BASE);
        let out = patch(&xml, &[edit(2, 3, Content::Number(7.0))]).unwrap();
        assert!(out.contains(
            "<row r=\"3\"><c r=\"A3\" s=\"5\"><v>11</v></c><c r=\"B3\" s=\"6\"><v>7</v></c><c r=\"C3\" s=\"7\"><f>A3*2</f><v>22</v></c></row>"
        ));
    }

    #[test]
    fn an_emptied_cell_keeps_its_style_and_loses_value_and_formula() {
        let xml = sheet(BASE);
        let out = patch(
            &xml,
            &[edit(3, 3, Content::Empty), edit(1, 3, Content::Empty)],
        )
        .unwrap();
        assert!(out.contains("<row r=\"3\"><c r=\"A3\" s=\"5\"/><c r=\"C3\" s=\"7\"/></row>"));
    }

    #[test]
    fn a_self_closing_row_and_an_empty_sheet_data_take_cells() {
        let xml = sheet("<row r=\"2\" spans=\"1:1\" ht=\"15\"/>");
        let out = patch(&xml, &[edit(1, 2, Content::Number(1.0))]).unwrap();
        assert!(out.contains("<row r=\"2\" ht=\"15\"><c r=\"A2\"><v>1</v></c></row>"));

        let empty = "<worksheet><sheetData/><x/></worksheet>";
        let out = patch(empty, &[edit(1, 1, Content::Number(1.0))]).unwrap();
        assert_eq!(out, "<worksheet><sheetData><row r=\"1\"><c r=\"A1\"><v>1</v></c></row></sheetData><x/></worksheet>");
    }

    #[test]
    fn a_shared_formula_group_is_edited_whole_or_not_at_all() {
        let xml = sheet(concat!(
            "<row r=\"2\"><c r=\"A2\"><f t=\"shared\" ref=\"A2:A4\" si=\"0\">B2</f><v>1</v></c></row>",
            "<row r=\"3\"><c r=\"A3\"><f t=\"shared\" si=\"0\"/><v>1</v></c></row>",
            "<row r=\"4\"><c r=\"A4\"><f t=\"shared\" si=\"0\"/><v>1</v></c></row>",
        ));
        // Emptying the first cell alone would orphan the other two.
        let error = patch(&xml, &[edit(1, 2, Content::Empty)]).unwrap_err();
        assert!(error.into_messages()[1].contains("geteilte Formel ab A2"));
        // Made plain as a whole — the paste's own way — it goes through.
        let out = patch(
            &xml,
            &[
                edit(1, 2, Content::Empty),
                edit(1, 3, Content::Formula("B3".into())),
                edit(1, 4, Content::Formula("B4".into())),
            ],
        )
        .unwrap();
        assert!(!out.contains("t=\"shared\""));
        assert!(out.contains("<c r=\"A3\"><f>B3</f></c>"));
    }

    #[test]
    fn an_edit_inside_an_array_formula_is_refused() {
        let xml =
            sheet("<row r=\"2\"><c r=\"A2\"><f t=\"array\" ref=\"A2:B3\">X</f><v>1</v></c></row>");
        assert!(patch(&xml, &[edit(2, 3, Content::Number(1.0))]).is_err());
        assert!(patch(&xml, &[edit(3, 3, Content::Number(1.0))]).is_ok());
    }

    #[test]
    fn what_cannot_be_read_with_certainty_is_refused() {
        // No row number, a cell in the wrong row, a prefixed sheetData.
        assert!(patch(
            &sheet("<row><c r=\"A1\"/></row>"),
            &[edit(1, 1, Content::Empty)]
        )
        .is_err());
        assert!(patch(
            &sheet("<row r=\"1\"><c r=\"A2\"/></row>"),
            &[edit(1, 1, Content::Empty)]
        )
        .is_err());
        assert!(patch(
            "<x:worksheet><x:sheetData/></x:worksheet>",
            &[edit(1, 1, Content::Empty)]
        )
        .is_err());
        // One cell twice, a NaN, a formula without text.
        let xml = sheet(BASE);
        assert!(patch(
            &xml,
            &[edit(1, 9, Content::Empty), edit(1, 9, Content::Empty)]
        )
        .is_err());
        assert!(patch(&xml, &[edit(1, 9, Content::Number(f64::NAN))]).is_err());
        assert!(patch(&xml, &[edit(1, 9, Content::Formula(" ".into()))]).is_err());
    }

    #[test]
    fn text_is_escaped_for_xml_and_for_excel() {
        assert_eq!(escape_text("a<b>&c"), "a&lt;b&gt;&amp;c");
        // A literal Excel escape survives as text, not as the character it names.
        assert_eq!(escape_text("_x0041_"), "_x005F_x0041_");
        assert_eq!(escape_text("x\u{1}y"), "x_x0001_y");
        assert_eq!(escape_text("a\tb\nc"), "a\tb\nc");
        assert_eq!(escape_text("_x12_"), "_x12_");
    }

    #[test]
    fn addresses_read_and_spell_as_excel_does() {
        assert_eq!(parse_ref("AB12"), Some((28, 12)));
        assert_eq!(parse_ref("$C$3"), Some((3, 3)));
        assert_eq!(parse_ref("12"), None);
        assert_eq!(parse_ref("a1"), None);
        assert_eq!(letters(1), "A");
        assert_eq!(letters(27), "AA");
        assert_eq!(letters(703), "AAA");
    }

    #[test]
    fn attributes_are_read_in_either_quote_and_in_any_order() {
        assert_eq!(attribute("<c s='3' r=\"B2\">", "r"), Some("B2"));
        assert_eq!(attribute("<c s='3' r=\"B2\">", "s"), Some("3"));
        assert_eq!(attribute("<c r=\"B2\"/>", "s"), None);
        assert_eq!(tag_end("<c a=\"x>y\">", 0), Some(10));
    }
}
