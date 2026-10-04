// ─── why ────────────────────────────────────────────────────────
// One bound master sheet as the MIRROR holds it, built completely here so the
// frontend only renders: the sheet's full header row in sheet order, and one
// display-ready row per entity. Checking the mirror against the customer means
// putting the two side by side, so the view keeps the customer's columns —
// empty headers included — and fills what the app can fill.
//
// The GRAIN follows from what the sheet maps, not from a list per kind: a sheet
// naming a Radsatznummer is one row per Radsatz, one naming only a Wagennummer
// is one row per Wagen, anything else has no rows yet. A later kind therefore
// gets a view the moment its template exists, and a column is `filled` only
// when the grain can actually answer its field — an Instandhaltung column on a
// Radsatz sheet is shown, but as one the app does not fill.
//
// The Radsatz rows are the mirror's CURRENT state, not a copy of the one sheet:
// both Radsatz sheets show every Radsatz with its open Einbau, and `source`
// names the sheet and row the fitting came from. The mirror decided between the
// two sheets during the import; a per-sheet copy would hide that decision.
//
// Formatting is the app's own: the Wagennummer in the Schattensystem's
// spelling (`TrainsSettings`), dates `dd.mm.yyyy`. A missing file or an unbound
// sheet is a `problem`, never an error, like the master view's.
// ────────────────────────────────────────────────────────────────

use std::collections::HashMap;
use std::path::Path;

use super::{book, view};
use crate::error::{AppError, AppResult};
use crate::trains::db::TrainsDb;
use crate::trains::model::{
    Einbau, FieldKind, MasterSheetView, Provenance, Radsatz, SheetColumn, SheetRow, UicStyle, Wagen,
};
use crate::trains::sanitise::format;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Grain {
    Wagen,
    Radsatz,
}

impl Grain {
    fn of(fields: &HashMap<u32, FieldKind>) -> Option<Self> {
        let maps = |wanted| fields.values().any(|field| *field == wanted);
        if maps(FieldKind::Radsatznummer) {
            Some(Self::Radsatz)
        } else if maps(FieldKind::Wagennummer) {
            Some(Self::Wagen)
        } else {
            None
        }
    }

    fn fills(self, field: FieldKind) -> bool {
        let on_wagen = matches!(
            field,
            FieldKind::Wagennummer | FieldKind::Halter | FieldKind::Eigentuemer
        );
        let on_radsatz = matches!(
            field,
            FieldKind::Radsatznummer
                | FieldKind::RadsatzSystemId
                | FieldKind::Wellennummer
                | FieldKind::EingebautAm
                | FieldKind::AusgebautAm
                | FieldKind::Einbauposition
        );
        match self {
            Self::Wagen => on_wagen,
            Self::Radsatz => on_wagen || on_radsatz,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Wagen => "Wagen",
            Self::Radsatz => "Radsätze",
        }
    }
}

struct Entity<'a> {
    wagen: Option<&'a Wagen>,
    radsatz: Option<&'a Radsatz>,
    einbau: Option<&'a Einbau>,
}

pub fn sheet_view(db: &TrainsDb, sheet: &str) -> MasterSheetView {
    let mut out = MasterSheetView {
        sheet: sheet.to_string(),
        ..MasterSheetView::default()
    };
    if let Err(error) = build(db, sheet, &mut out) {
        out.problem = error.into_messages().into_iter().next();
    }
    out
}

fn build(db: &TrainsDb, sheet: &str, out: &mut MasterSheetView) -> AppResult<()> {
    let settings = db.master();
    let path =
        settings.file.as_deref().map(Path::new).ok_or_else(|| {
            AppError::Report(vec!["Es ist noch keine Master-Datei gewählt.".into()])
        })?;
    let binding = settings
        .bindings
        .iter()
        .find(|binding| binding.sheet == sheet)
        .ok_or_else(|| {
            AppError::Report(vec![format!(
                "Das Blatt „{sheet}“ ist in den Master-Einstellungen nicht zugeordnet."
            )])
        })?;
    out.kind = binding.kind;

    let mut workbook = book::open(path)?;
    let index = book::index(&book::names(&workbook), sheet)?;
    book::deserialise(&mut workbook, index, path)?;
    let header = view::header_row(&workbook.sheet_collection_no_check()[index]);
    let fields = view::fields(&header, view::template_of(binding, db).as_ref());
    let grain = Grain::of(&fields);

    let named: HashMap<u32, &str> = header
        .iter()
        .map(|(col, text)| (*col, text.as_str()))
        .collect();
    let last = header.last().map_or(0, |(col, _)| *col);
    let filled: Vec<Option<FieldKind>> = (1..=last)
        .map(|index| {
            fields
                .get(&index)
                .copied()
                .filter(|field| grain.is_some_and(|grain| grain.fills(*field)))
        })
        .collect();
    out.columns = (1..=last)
        .map(|index| SheetColumn {
            index,
            header: named.get(&index).copied().unwrap_or_default().to_string(),
            filled: filled[index as usize - 1].is_some(),
        })
        .collect();

    let Some(grain) = grain else {
        return Ok(());
    };
    out.row_label = grain.label().to_string();
    let style = db.settings().wagennummer;
    let mut entities = entities(db, grain);
    entities.sort_by(|left, right| order(left).cmp(&order(right)));
    out.rows = entities
        .iter()
        .map(|entity| row(db, entity, &filled, style))
        .collect();
    Ok(())
}

fn entities(db: &TrainsDb, grain: Grain) -> Vec<Entity<'_>> {
    match grain {
        Grain::Wagen => db
            .wagen_refs()
            .map(|wagen| Entity {
                wagen: Some(wagen),
                radsatz: None,
                einbau: None,
            })
            .collect(),
        Grain::Radsatz => db
            .radsatz_refs()
            .map(|radsatz| {
                let einbau = db.open_einbau(&radsatz.id);
                Entity {
                    wagen: einbau.and_then(|einbau| db.wagen_by_id(&einbau.wagen_id)),
                    radsatz: Some(radsatz),
                    einbau,
                }
            })
            .collect(),
    }
}

fn order<'a>(entity: &Entity<'a>) -> (&'a str, Option<&'a str>, &'a str) {
    (
        entity.wagen.map_or("", |wagen| wagen.nummer.as_str()),
        entity.einbau.and_then(|einbau| einbau.position.as_deref()),
        entity.radsatz.map_or("", |radsatz| radsatz.nummer.as_str()),
    )
}

fn row(
    db: &TrainsDb,
    entity: &Entity<'_>,
    filled: &[Option<FieldKind>],
    style: UicStyle,
) -> SheetRow {
    let wagen = entity
        .wagen
        .map(|wagen| format::uic_in(&wagen.nummer, style))
        .unwrap_or_default();
    let key = match entity.radsatz {
        Some(radsatz) if wagen.is_empty() => radsatz.nummer.clone(),
        Some(radsatz) => format!("{wagen} · {}", radsatz.nummer),
        None => wagen.clone(),
    };
    let source = entity
        .einbau
        .map(|einbau| &einbau.source)
        .or_else(|| entity.radsatz.and_then(|radsatz| radsatz.source.as_ref()))
        .or_else(|| entity.wagen.and_then(|wagen| wagen.source.as_ref()))
        .map(|source: &Provenance| format!("{}, Zeile {}", source.sheet, source.row));
    SheetRow {
        key,
        cells: filled
            .iter()
            .map(|field| {
                field
                    .map(|field| cell(db, entity, field, style))
                    .unwrap_or_default()
            })
            .collect(),
        source,
    }
}

fn cell(db: &TrainsDb, entity: &Entity<'_>, field: FieldKind, style: UicStyle) -> String {
    let partner = |id: Option<&String>| {
        id.and_then(|id| db.partner_by_id(id))
            .map(|partner| partner.name.clone())
            .unwrap_or_default()
    };
    let text = |value: Option<&String>| value.cloned().unwrap_or_default();
    let date = |value: Option<&String>| value.map(|iso| format::iso_date(iso)).unwrap_or_default();
    match field {
        FieldKind::Wagennummer => entity
            .wagen
            .map(|wagen| format::uic_in(&wagen.nummer, style))
            .unwrap_or_default(),
        FieldKind::Halter => partner(entity.wagen.and_then(|wagen| wagen.halter_id.as_ref())),
        FieldKind::Eigentuemer => {
            partner(entity.wagen.and_then(|wagen| wagen.eigentuemer_id.as_ref()))
        }
        FieldKind::Radsatznummer => text(entity.radsatz.map(|radsatz| &radsatz.nummer)),
        FieldKind::RadsatzSystemId => text(
            entity
                .radsatz
                .and_then(|radsatz| radsatz.system_id.as_ref()),
        ),
        FieldKind::Wellennummer => text(
            entity
                .radsatz
                .and_then(|radsatz| radsatz.wellennummer.as_ref()),
        ),
        FieldKind::EingebautAm => date(
            entity
                .einbau
                .and_then(|einbau| einbau.eingebaut_am.as_ref()),
        ),
        FieldKind::AusgebautAm => date(
            entity
                .einbau
                .and_then(|einbau| einbau.ausgebaut_am.as_ref()),
        ),
        FieldKind::Einbauposition => {
            text(entity.einbau.and_then(|einbau| einbau.position.as_ref()))
        }
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TempDir;
    use crate::trains::master::mirror::tests::{bound, walk, walk_first};

    fn imported(folder: &TempDir) -> TrainsDb {
        let mut db = bound(folder);
        walk_first(&mut db);
        walk(&mut db, "Einbauliste", false);
        walk(&mut db, "Bestand", false);
        db
    }

    #[test]
    fn a_radsatz_sheet_keeps_every_column_and_fills_only_the_first_repeated_one() {
        let folder = TempDir::new("sheetview-columns");
        let db = imported(&folder);
        let view = sheet_view(&db, "Einbauliste");
        assert_eq!(view.problem, None);
        assert_eq!(view.row_label, "Radsätze");

        let shape: Vec<(&str, bool)> = view
            .columns
            .iter()
            .map(|column| (column.header.as_str(), column.filled))
            .collect();
        assert_eq!(
            shape,
            [
                ("an_wagen", true),
                ("radsatzid", true),
                ("radsatz", true),
                ("einbau_am", true),
                ("einbau_am", false),
                ("an_wagen", false),
                ("pos", true),
                ("halter", true),
            ]
        );
    }

    #[test]
    fn radsatz_rows_are_the_mirror_sorted_by_wagen_and_position_with_their_source() {
        let folder = TempDir::new("sheetview-rows");
        let db = imported(&folder);
        let view = sheet_view(&db, "Bestand");

        let keys: Vec<&str> = view.rows.iter().map(|row| row.key.as_str()).collect();
        assert_eq!(
            keys,
            [
                "218124712173 · RS1",
                "218124712173 · RS2",
                "218124712181 · RS3"
            ]
        );
        let first = &view.rows[0];
        assert_eq!(first.cells.len(), view.columns.len());
        assert_eq!(first.source.as_deref(), Some("Einbauliste, Zeile 2"));
        assert_eq!(view.rows[2].source.as_deref(), Some("Bestand, Zeile 4"));
        assert!(
            first.cells.contains(&"Wagenmut AG".to_string()),
            "{:?}",
            first.cells
        );
        assert!(
            first
                .cells
                .iter()
                .any(|cell| cell.len() == 10 && cell.contains('.')),
            "the date is dd.mm.yyyy: {:?}",
            first.cells
        );
    }

    #[test]
    fn the_dashboard_is_one_row_per_wagen_with_its_notes_column_unfilled() {
        let folder = TempDir::new("sheetview-fleet");
        let db = imported(&folder);
        let view = sheet_view(&db, "Übersicht");
        assert_eq!(view.row_label, "Wagen");
        assert_eq!(view.rows.len(), 2);
        assert_eq!(view.columns[1].header, "Bemerkungen");
        assert!(!view.columns[1].filled);
        assert_eq!(view.rows[0].cells, ["218124712173", ""]);
    }

    #[test]
    fn an_unbound_sheet_is_a_problem_not_an_error() {
        let folder = TempDir::new("sheetview-unbound");
        let db = imported(&folder);
        let view = sheet_view(&db, "Kontakte");
        assert!(view.problem.unwrap().contains("nicht zugeordnet"));
        assert!(view.rows.is_empty());
    }
}
