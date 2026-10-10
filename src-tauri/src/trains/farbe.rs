// ─── why ────────────────────────────────────────────────────────
// The Farben: the mark a user puts on a Wagen or Radsatz, by hand in the app or
// as a cell fill in the master. See `model::Markierungen` for the two halves
// and why they are keyed by number and not by id.
//
// The master's colour is the fill of the KEY CELL — the Wagennummer on a Wagen
// list, the Radsatznummer on a Radsatz list — because that is the cell Excel's
// „Nach Farbe“ filters on in the customer's own file. A conditional format is
// NOT read: it is a rule over values, and those values are in the
// Schattensystem already. A Radsatz list colours Radsätze only, never the Wagen
// it also names.
//
// A fill is reduced to the six Farben by HUE, so Excel's pale „hellrot“ and a
// saturated red are one Farbe; near-white is no mark at all, an unsaturated fill
// is Grau. A theme colour is resolved through the workbook's theme with its
// tint, or every themed fill would read as black.
//
// The master half is read while a sheet is STAGED, when the workbook is open,
// and merged only when that sheet is COMMITTED (`merge_master`): a walk the user
// cancels leaves no colour behind. The first sheet to colour a key wins, in the
// mirror's own kind order, the way the Wagen list's row wins over a later one.
// ────────────────────────────────────────────────────────────────

use umya_spreadsheet::{PatternValues, Workbook, Worksheet};

use super::db::TrainsDb;
use super::model::{EntityRef, Farbe, Farben, FieldKind, ImportPlan, Markierungen, SheetKind};
use super::resolve::radsatz::match_key;
use super::sanitise::Value;
use super::sheet::grid::Grid;
use super::stage::RowValues;
use crate::error::{AppError, AppResult};

pub fn set(db: &mut TrainsDb, kind: EntityRef, id: &str, farbe: Option<Farbe>) -> AppResult<()> {
    let mut markierungen = db.markierungen().clone();
    let (map, key) = match kind {
        EntityRef::Wagen => {
            let wagen = db.wagen().into_iter().find(|wagen| wagen.id == id);
            let key = wagen.map(|wagen| wagen.nummer).ok_or_else(missing)?;
            (&mut markierungen.hand.wagen, key)
        }
        EntityRef::Radsatz => {
            let radsatz = db.radsaetze().into_iter().find(|radsatz| radsatz.id == id);
            let key = radsatz
                .map(|radsatz| radsatz.match_key)
                .ok_or_else(missing)?;
            (&mut markierungen.hand.radsaetze, key)
        }
        EntityRef::Partner => {
            return Err(AppError::Report(vec![
                "Partner werden nicht farbig markiert.".into(),
            ]))
        }
    };
    match farbe {
        Some(farbe) => {
            map.insert(key, farbe);
        }
        None => {
            map.shift_remove(&key);
        }
    }
    db.save_markierungen(markierungen)
}

pub fn merge_master(db: &mut TrainsDb, farben: &Farben) -> AppResult<()> {
    if farben.is_empty() {
        return Ok(());
    }
    let mut markierungen: Markierungen = db.markierungen().clone();
    for (key, farbe) in &farben.wagen {
        markierungen
            .master
            .wagen
            .entry(key.clone())
            .or_insert(*farbe);
    }
    for (key, farbe) in &farben.radsaetze {
        markierungen
            .master
            .radsaetze
            .entry(key.clone())
            .or_insert(*farbe);
    }
    db.save_markierungen(markierungen)
}

pub struct MasterSheet<'a> {
    pub book: &'a Workbook,
    pub worksheet: &'a Worksheet,
    pub kind: SheetKind,
    pub plan: &'a ImportPlan,
    pub grid: &'a Grid,
    pub values: &'a [RowValues],
}

pub fn read_master(sheet: &MasterSheet) -> Farben {
    let mut farben = Farben::default();
    let field = match sheet.kind {
        SheetKind::Wagenliste => FieldKind::Wagennummer,
        SheetKind::RadsatzEinbau | SheetKind::RadsatzBestand => FieldKind::Radsatznummer,
    };
    let Some(binding) = sheet.plan.binding(field) else {
        return farben;
    };
    for row in sheet.values {
        let Some(farbe) = fill(sheet, binding.index, row.row) else {
            continue;
        };
        match field {
            FieldKind::Wagennummer => {
                if let Some(Value::Uic(uic)) = row.value(FieldKind::Wagennummer) {
                    farben
                        .wagen
                        .entry(uic.as_str().to_string())
                        .or_insert(farbe);
                }
            }
            _ => {
                let key = match_key(sheet.grid.text(binding.index, row.row));
                if !key.is_empty() {
                    farben.radsaetze.entry(key).or_insert(farbe);
                }
            }
        }
    }
    farben
}

fn fill(sheet: &MasterSheet, col: u32, row: u32) -> Option<Farbe> {
    let pattern = sheet
        .worksheet
        .cell((col, row))?
        .style()
        .fill()?
        .pattern_fill()?;
    if *pattern.pattern_type() == PatternValues::None {
        return None;
    }
    let color = pattern.foreground_color()?;
    from_argb(&color.argb_with_theme(sheet.book.theme()))
}

pub fn from_argb(hex: &str) -> Option<Farbe> {
    let hex = hex.trim().trim_start_matches('#');
    let rgb = match hex.len() {
        8 if &hex[..2] == "00" => return None,
        8 => &hex[2..],
        6 => hex,
        _ => return None,
    };
    let channel = |at: usize| {
        u8::from_str_radix(&rgb[at..at + 2], 16)
            .ok()
            .map(|value| f64::from(value) / 255.0)
    };
    let (red, green, blue) = (channel(0)?, channel(2)?, channel(4)?);
    let max = red.max(green).max(blue);
    let min = red.min(green).min(blue);
    let lightness = (max + min) / 2.0;
    if lightness > 0.95 {
        return None;
    }
    let delta = max - min;
    let saturation = if delta == 0.0 {
        0.0
    } else {
        delta / (1.0 - (2.0 * lightness - 1.0).abs())
    };
    if saturation < 0.2 {
        return Some(Farbe::Grau);
    }
    let hue = if max == red {
        60.0 * ((green - blue) / delta).rem_euclid(6.0)
    } else if max == green {
        60.0 * ((blue - red) / delta + 2.0)
    } else {
        60.0 * ((red - green) / delta + 4.0)
    };
    Some(match hue {
        hue if !(20.0..330.0).contains(&hue) => Farbe::Rot,
        hue if hue < 70.0 => Farbe::Gelb,
        hue if hue < 170.0 => Farbe::Gruen,
        hue if hue < 255.0 => Farbe::Blau,
        _ => Farbe::Lila,
    })
}

fn missing() -> AppError {
    AppError::Report(vec![
        "Der Eintrag ist nicht mehr vorhanden.".into(),
        "Die Liste wird neu geladen.".into(),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TempDir;

    #[test]
    fn excel_fills_map_to_one_farbe_by_hue() {
        // Excel's own "Gut/Schlecht/Neutral" fills and the saturated defaults.
        assert_eq!(from_argb("FFFFC7CE"), Some(Farbe::Rot));
        assert_eq!(from_argb("FFFF0000"), Some(Farbe::Rot));
        assert_eq!(from_argb("FFFFC000"), Some(Farbe::Gelb));
        assert_eq!(from_argb("FFFFEB9C"), Some(Farbe::Gelb));
        assert_eq!(from_argb("FFC6EFCE"), Some(Farbe::Gruen));
        assert_eq!(from_argb("FF00B050"), Some(Farbe::Gruen));
        assert_eq!(from_argb("FF0070C0"), Some(Farbe::Blau));
        assert_eq!(from_argb("FF7030A0"), Some(Farbe::Lila));
        assert_eq!(from_argb("FFD9D9D9"), Some(Farbe::Grau));
        // A theme colour resolves to six digits, without alpha.
        assert_eq!(from_argb("5B9BD5"), Some(Farbe::Blau));
    }

    #[test]
    fn white_transparent_and_garbage_are_no_mark() {
        assert_eq!(from_argb("FFFFFFFF"), None);
        assert_eq!(from_argb("00FF0000"), None);
        assert_eq!(from_argb(""), None);
        assert_eq!(from_argb("FFZZ0000"), None);
    }

    #[test]
    fn a_master_colour_never_overwrites_an_earlier_sheet() {
        let dir = TempDir::new("farbe-merge");
        let mut db = TrainsDb::load(&dir.config()).unwrap();
        let mut first = Farben::default();
        first.wagen.insert("318047401234".into(), Farbe::Rot);
        merge_master(&mut db, &first).unwrap();

        let mut second = Farben::default();
        second.wagen.insert("318047401234".into(), Farbe::Blau);
        second.wagen.insert("318047405678".into(), Farbe::Gelb);
        merge_master(&mut db, &second).unwrap();

        let master = &db.markierungen().master.wagen;
        assert_eq!(master.get("318047401234"), Some(&Farbe::Rot));
        assert_eq!(master.get("318047405678"), Some(&Farbe::Gelb));
    }

    #[test]
    fn the_mirror_wipe_keeps_hand_colours_and_drops_the_masters() {
        let dir = TempDir::new("farbe-wipe");
        let mut db = TrainsDb::load(&dir.config()).unwrap();
        let mut hand = Markierungen::default();
        hand.hand.wagen.insert("318047401234".into(), Farbe::Gruen);
        hand.master.wagen.insert("318047401234".into(), Farbe::Rot);
        db.save_markierungen(hand).unwrap();

        db.clear_mirror().unwrap();

        let markierungen = db.markierungen();
        assert_eq!(
            markierungen.hand.wagen.get("318047401234"),
            Some(&Farbe::Gruen)
        );
        assert!(markierungen.master.is_empty());

        db.reset().unwrap();
        assert!(db.markierungen().hand.is_empty());
    }
}
