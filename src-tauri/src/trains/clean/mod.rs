// ─── why ────────────────────────────────────────────────────────
// The original, read 1:1 through the same parsers staging uses, and every cell
// that would come out different sorted by WHETHER THE CHANGE COULD ALTER WHAT IT
// MEANS. That sort is the whole feature: a file of four hundred wagen reformats
// four hundred numbers, and the one amount read a thousand times too large must
// not drown in them.
//
//   Fehler   no clean value exists. Blocks until corrected or left empty.
//   Deutung  the value was INTERPRETED and another reading was possible — the
//            decimal style, the order of day and month, a parser's own warning.
//            Asked once per COLUMN, because the question is about the column.
//   Format   a rewrite nothing could be misread through. Counted, not asked.
//
// Whether a column has a question at all is `reading::read_column`'s call, the
// same one staging makes — see there for why a saved reading is doubted and why
// a column whose cells read the same either way asks nothing. This module turns
// that question into a card, and the user's confirmation back into a reading.
//
// A correction REPLACES THE RAW TEXT and is read like any other cell, so a typo
// in the correction is a Fehler again rather than a value let through. An empty
// correction is "leer lassen", recorded as such.
//
// A cell stored as a number is a Format change even when its digits survive:
// the cleaned file writes text, so the type changed, and a protocol that calls
// it unchanged would be the protocol lying.
//
// `confirmed` is the plan plus what the user confirmed. It is what the cleaned
// file is staged with AND what the template learns, and both are right: every
// value the cleaner wrote is canonical — `1234,56`, `31.12.2025` — which reads
// the same under any reading, so the cleaned file raises no question of its own.
//
// `open` takes the template rather than the database, so a caller fetches it and
// lets go of the store before the workbook is read.
// ────────────────────────────────────────────────────────────────

pub mod write;

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use super::model::{
    CardExample, CleanDecisions, CleanReport, CleanSummary, Confirmation, DeutungCard, FehlerCell,
    FieldKind, FormatGroup, FormatSample, ImportPlan, ImportTemplate, Reading, Tier, UicStyle,
};
use super::reading::{read_column, reads_a_date, Confirmed, Interpretation, Question};
use super::recognise;
use super::sanitise::{format, Parsed};
use super::sheet::grid::{self, Grid, RawCell};
use super::stage::parse_cell;
use crate::error::{AppError, AppResult};

const EXAMPLES: usize = 20;
const SAMPLES: usize = 3;

pub struct Change {
    pub row: u32,
    pub column: u32,
    pub header: String,
    pub raw: String,
    pub clean: String,
    pub tier: Tier,
    pub rule: String,
}

pub struct Cleaned {
    pub report: CleanReport,
    pub changes: Vec<Change>,
    pub confirmed: ImportPlan,
}

pub struct HeldClean {
    pub path: PathBuf,
    pub grid: Grid,
    pub plan: ImportPlan,
    pub template_name: String,
    pub uic: UicStyle,
}

pub fn open(
    path: &Path,
    sheet: &str,
    template: &ImportTemplate,
    uic: UicStyle,
) -> AppResult<HeldClean> {
    let source = grid::read(path, Some(sheet))?;
    let (detected, _) = recognise::detected(&source.grid)?;
    let plan = recognise::rebind(template, &detected);
    let missing = plan.missing_required();
    if !missing.is_empty() {
        let labels: Vec<&str> = missing.iter().map(|field| field.label()).collect();
        return Err(AppError::Report(vec![
            format!(
                "Die Vorlage „{}“ findet in dieser Arbeitsmappe keine Spalte für: {}.",
                template.name,
                labels.join(", ")
            ),
            "Bitte eine andere Vorlage wählen oder die Datei von Hand zuordnen.".into(),
        ]));
    }
    Ok(HeldClean {
        path: path.to_path_buf(),
        grid: source.grid,
        plan,
        template_name: template.name.clone(),
        uic,
    })
}

struct Column<'a> {
    interpretation: Interpretation,
    question: Option<Question>,
    hinweise: Vec<CardExample>,
    hinweis_count: u32,
    corrections: &'a HashMap<(u32, u32), &'a str>,
    uic: UicStyle,
}

struct Sink {
    changes: Vec<Change>,
    fehler: Vec<FehlerCell>,
    formats: BTreeMap<(u32, &'static str), FormatGroup>,
}

impl HeldClean {
    pub fn run(&self, decisions: &CleanDecisions) -> AppResult<Cleaned> {
        let plan = &self.plan;
        let layout = plan.reader.apply(&self.grid, plan.layout)?;
        let corrections: HashMap<(u32, u32), &str> = decisions
            .corrections
            .iter()
            .map(|c| ((c.row, c.column), c.value.as_str()))
            .collect();
        let mut confirmed = plan.clone();
        let mut cards = Vec::new();
        let mut sink = Sink {
            changes: Vec::new(),
            fehler: Vec::new(),
            formats: BTreeMap::new(),
        };

        for (position, binding) in plan.columns.iter().enumerate() {
            if binding.field == FieldKind::Ignorieren {
                continue;
            }
            let rows = layout.first_data_row..=layout.last_data_row.min(self.grid.rows);
            let cells: Vec<(u32, Option<&RawCell>)> = rows
                .map(|row| (row, self.grid.cell(binding.index, row)))
                .collect();
            let texts: Vec<(u32, &str)> = cells
                .iter()
                .filter_map(|(row, cell)| {
                    let text = match corrections.get(&(*row, binding.index)) {
                        Some(corrected) => *corrected,
                        None => cell.filter(|cell| cell.number.is_none())?.text.as_str(),
                    };
                    (!text.trim().is_empty()).then_some((*row, text))
                })
                .collect();
            let hints: Vec<bool> = cells
                .iter()
                .map(|(_, cell)| cell.is_some_and(|cell| cell.date_format))
                .collect();

            let chosen = confirmed_reading(decisions, binding.index);
            let (interpretation, question) = read_column(binding, &texts, &hints, chosen);
            confirmed.columns[position].decimal = chosen.decimal.or(binding.decimal);
            confirmed.columns[position].date_order = chosen.date_order.or(binding.date_order);

            let mut column = Column {
                interpretation,
                question,
                hinweise: Vec::new(),
                hinweis_count: 0,
                corrections: &corrections,
                uic: self.uic,
            };
            for (row, cell) in &cells {
                clean_cell(&mut column, plan, *cell, *row, &mut sink);
            }

            if let Some(question) = column.question {
                let answered = chosen.decimal.is_some() || chosen.date_order.is_some();
                cards.push(DeutungCard {
                    column: binding.index,
                    header: binding.header.clone(),
                    field: binding.field,
                    count: question.differs.len() as u32,
                    examples: question.differs.into_iter().take(EXAMPLES).collect(),
                    reading: question.reading,
                    reason: question.reason,
                    confirmed: answered,
                });
            }
            if column.hinweis_count > 0 {
                cards.push(DeutungCard {
                    column: binding.index,
                    header: binding.header.clone(),
                    field: binding.field,
                    reading: Reading::Hinweis,
                    reason: format!(
                        "{} Wert(e) wurden mit Hinweis gelesen. Bitte prüfen.",
                        column.hinweis_count
                    ),
                    count: column.hinweis_count,
                    examples: column.hinweise,
                    confirmed: decisions.confirmations.contains(&Confirmation::Hinweis {
                        column: binding.index,
                    }),
                });
            }
        }

        let formats: Vec<FormatGroup> = sink.formats.into_values().collect();
        let summary = CleanSummary {
            fehler_offen: sink.fehler.iter().filter(|cell| cell.open).count() as u32,
            deutungen_offen: cards.iter().filter(|card| !card.confirmed).count() as u32,
            formatierungen: formats.iter().map(|group| group.count).sum(),
            korrigiert: sink
                .changes
                .iter()
                .filter(|change| change.tier == Tier::Fehler)
                .count() as u32,
        };

        Ok(Cleaned {
            report: CleanReport {
                file: self.path.to_string_lossy().into_owned(),
                sheet: self.grid.sheet.clone(),
                template_id: plan.template_id.clone().unwrap_or_default(),
                template_name: self.template_name.clone(),
                plan: plan.clone(),
                fehler: sink.fehler,
                cards,
                formats,
                summary,
            },
            changes: sink.changes,
            confirmed,
        })
    }
}

pub fn ready(report: &CleanReport) -> AppResult<()> {
    let mut open = Vec::new();
    if report.summary.fehler_offen > 0 {
        open.push(format!(
            "{} Fehler sind noch nicht korrigiert.",
            report.summary.fehler_offen
        ));
    }
    if report.summary.deutungen_offen > 0 {
        open.push(format!(
            "{} Deutung(en) sind noch nicht bestätigt.",
            report.summary.deutungen_offen
        ));
    }
    if open.is_empty() {
        return Ok(());
    }
    open.insert(
        0,
        "Die bereinigte Datei kann noch nicht geschrieben werden.".into(),
    );
    Err(AppError::Report(open))
}

fn confirmed_reading(decisions: &CleanDecisions, column: u32) -> Confirmed {
    let mut chosen = Confirmed::default();
    for confirmation in &decisions.confirmations {
        match *confirmation {
            Confirmation::Decimal { column: at, style } if at == column => {
                chosen.decimal = Some(style)
            }
            Confirmation::DateOrder { column: at, order } if at == column => {
                chosen.date_order = Some(order)
            }
            _ => {}
        }
    }
    chosen
}

fn clean_cell(
    column: &mut Column<'_>,
    plan: &ImportPlan,
    original: Option<&RawCell>,
    row: u32,
    sink: &mut Sink,
) {
    let binding = &column.interpretation.binding;
    let raw = original.map_or("", |cell| cell.text.as_str());
    let corrected = column.corrections.get(&(row, binding.index)).copied();
    if raw.trim().is_empty() && corrected.is_none() {
        return;
    }
    let change = |clean: String, tier: Tier, rule: String| Change {
        row,
        column: binding.index,
        header: binding.header.clone(),
        raw: raw.to_string(),
        clean,
        tier,
        rule,
    };
    let fehler = |message: String, correction: Option<&str>, open: bool| FehlerCell {
        row,
        column: binding.index,
        header: binding.header.clone(),
        raw: raw.to_string(),
        message,
        correction: correction.map(str::to_string),
        open,
    };
    let parsed = parse_cell(&column.interpretation, plan, original);

    if let Some(value) = corrected {
        let outcome = if value.trim().is_empty() {
            Ok(String::new())
        } else {
            let cell = RawCell {
                text: value.to_string(),
                number: None,
                date_format: false,
            };
            parse_cell(&column.interpretation, plan, Some(&cell))
                .map(|parsed| format::styled(&parsed.value, column.uic))
        };
        match outcome {
            Ok(clean) => {
                let rule = if clean.is_empty() {
                    "Leer gelassen"
                } else {
                    "Von Hand korrigiert"
                };
                sink.changes.push(change(clean, Tier::Fehler, rule.into()));
                if let Err(message) = parsed {
                    sink.fehler.push(fehler(message, Some(value), false));
                }
            }
            Err(message) => sink.fehler.push(fehler(message, Some(value), true)),
        }
        return;
    }

    let Parsed { value, warning } = match parsed {
        Ok(parsed) => parsed,
        Err(message) => {
            sink.fehler.push(fehler(message, None, true));
            return;
        }
    };
    let clean = format::styled(&value, column.uic);

    if let Some(message) = warning {
        column.hinweis_count += 1;
        if column.hinweise.len() < EXAMPLES {
            column.hinweise.push(CardExample {
                row,
                raw: raw.to_string(),
                chosen: clean.clone(),
                alternative: None,
                message: Some(message.clone()),
            });
        }
        sink.changes.push(change(clean, Tier::Deutung, message));
        return;
    }

    if let Some(example) = column.question.as_ref().and_then(|q| q.example(row)) {
        let rule = format!("Gelesen als {}", example.chosen);
        sink.changes.push(change(clean, Tier::Deutung, rule));
        return;
    }

    let stored_number = original.is_some_and(|cell| cell.number.is_some());
    if !stored_number && clean == raw {
        return;
    }
    let rule = format_rule(binding.field, stored_number);
    let group = sink
        .formats
        .entry((binding.index, rule))
        .or_insert_with(|| FormatGroup {
            column: binding.index,
            header: binding.header.clone(),
            rule: rule.to_string(),
            count: 0,
            samples: Vec::new(),
        });
    group.count += 1;
    if group.samples.len() < SAMPLES {
        group.samples.push(FormatSample {
            row,
            raw: raw.to_string(),
            clean: clean.clone(),
        });
    }
    sink.changes.push(change(clean, Tier::Format, rule.into()));
}

fn format_rule(field: FieldKind, stored_number: bool) -> &'static str {
    match field {
        FieldKind::Wagennummer => "Wagennummer einheitlich geschrieben",
        FieldKind::TelematikZeitpunkt if stored_number => "Excel-Zeitpunkt als Text geschrieben",
        FieldKind::TelematikZeitpunkt => "Zeitpunkt einheitlich geschrieben (TT.MM.JJJJ hh:mm:ss)",
        _ if reads_a_date(field) && stored_number => "Excel-Datum als Text geschrieben",
        _ if reads_a_date(field) => "Datum einheitlich geschrieben (TT.MM.JJJJ)",
        FieldKind::Betrag if stored_number => "Zahl als Betrag geschrieben",
        FieldKind::Betrag => "Betrag einheitlich geschrieben",
        _ if stored_number => "Zahl als Text gespeichert",
        _ => "Leerzeichen bereinigt",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{workbook, TempDir};
    use crate::trains::model::{Correction, DateOrder, DecimalStyle};
    use crate::trains::recognise;
    use crate::trains::sheet::grid;

    fn plan_for(grid: &Grid, fields: &[(&str, FieldKind)]) -> ImportPlan {
        let (mut plan, _) = recognise::detected(grid).unwrap();
        for column in &mut plan.columns {
            if let Some((_, field)) = fields.iter().find(|(header, _)| *header == column.header) {
                column.field = *field;
            }
        }
        plan
    }

    fn cleaned(grid: &Grid, plan: &ImportPlan, decisions: &CleanDecisions) -> Cleaned {
        cleaned_in(grid, plan, decisions, UicStyle::Grouped)
    }

    fn cleaned_in(
        grid: &Grid,
        plan: &ImportPlan,
        decisions: &CleanDecisions,
        uic: UicStyle,
    ) -> Cleaned {
        HeldClean {
            path: "liste.xlsx".into(),
            grid: grid.clone(),
            plan: plan.clone(),
            template_name: "Liste".into(),
            uic,
        }
        .run(decisions)
        .unwrap()
    }

    /// The setting decides what the cleaned copy writes — and so what counts
    /// as a change at all: a compact number in a compact Schattensystem is
    /// already clean, the grouped one is the one rewritten.
    #[test]
    fn the_wagennummer_is_cleaned_into_the_configured_spelling() {
        let grid = Grid::from_text(
            "Tabelle1",
            &[&["Wagennummer"], &["338506591522"], &["33 85 0659 152-2"]],
        );
        let plan = plan_for(&grid, &[("Wagennummer", FieldKind::Wagennummer)]);

        // Row 2 is stored as a NUMBER, so it is a format change under either
        // setting (the copy writes text); row 3 is the one the setting decides.
        let compact = cleaned_in(&grid, &plan, &none(), UicStyle::Compact);
        assert!(compact
            .changes
            .iter()
            .all(|change| change.clean == "338506591522"));
        assert!(compact.changes.iter().any(|change| change.row == 3));

        let grouped = cleaned_in(&grid, &plan, &none(), UicStyle::Grouped);
        assert!(grouped
            .changes
            .iter()
            .all(|change| change.clean == "33 85 0659 152-2"));
        assert!(grouped.changes.iter().all(|change| change.row != 3));
    }

    fn builtin(id: &str) -> ImportTemplate {
        crate::trains::builtin::all()
            .into_iter()
            .find(|template| template.id == id)
            .unwrap()
    }

    fn none() -> CleanDecisions {
        CleanDecisions::default()
    }

    fn betrag(values: &[&str]) -> (Grid, ImportPlan) {
        let mut rows: Vec<&[&str]> = vec![&["Wagen", "Betrag"]];
        let lines: Vec<[&str; 2]> = values
            .iter()
            .map(|value| ["33 85 0659 002-9", *value])
            .collect();
        rows.extend(lines.iter().map(|line| &line[..]));
        let grid = Grid::from_text("Tabelle1", &rows);
        let plan = plan_for(
            &grid,
            &[
                ("Wagen", FieldKind::Wagennummer),
                ("Betrag", FieldKind::Betrag),
            ],
        );
        (grid, plan)
    }

    fn datum(values: &[&str]) -> (Grid, ImportPlan) {
        let mut rows: Vec<&[&str]> = vec![&["Wagen", "Datum"]];
        let lines: Vec<[&str; 2]> = values
            .iter()
            .map(|value| ["33 85 0659 002-9", *value])
            .collect();
        rows.extend(lines.iter().map(|line| &line[..]));
        let grid = Grid::from_text("Tabelle1", &rows);
        let plan = plan_for(
            &grid,
            &[
                ("Wagen", FieldKind::Wagennummer),
                ("Datum", FieldKind::Datum),
            ],
        );
        (grid, plan)
    }

    // The import stages the CLEANED copy, so a time the copy drops is gone for
    // good — the telematics `Timestamp` must come out with its time.
    #[test]
    fn a_telematics_timestamp_keeps_its_time_in_the_cleaned_copy() {
        let grid = Grid::from_text(
            "Tabelle1",
            &[
                &["Asset", "Timestamp"],
                &["33 85 0659 002-9", "2026-10-02 13:37:01"],
            ],
        );
        let plan = plan_for(
            &grid,
            &[
                ("Asset", FieldKind::Wagennummer),
                ("Timestamp", FieldKind::TelematikZeitpunkt),
            ],
        );
        let result = cleaned(&grid, &plan, &none());
        let change = result
            .changes
            .iter()
            .find(|change| change.header == "Timestamp")
            .unwrap();
        assert_eq!(change.clean, "02.10.2026 13:37:01");
        assert!(result.report.cards.is_empty());
        assert!(ready(&result.report).is_ok());
    }

    /// Four hundred reformatted Wagennummern are one line, not four hundred.
    #[test]
    fn a_lossless_rewrite_is_a_format_group_and_asks_nothing() {
        let grid = Grid::from_text(
            "Tabelle1",
            &[
                &["Wagen"],
                &["338506590029"],
                &["3385.0659.002-9"],
                &["33 85 0659 002-9"],
            ],
        );
        let plan = plan_for(&grid, &[("Wagen", FieldKind::Wagennummer)]);
        let result = cleaned(&grid, &plan, &none());

        assert!(result.report.cards.is_empty());
        assert!(result.report.fehler.is_empty());
        assert_eq!(result.report.formats.len(), 1);
        assert_eq!(result.report.formats[0].count, 2);
        assert_eq!(
            result.report.formats[0].samples[0].clean,
            "33 85 0659 002-9"
        );
        // The canonical spelling already in the file is not a change at all.
        assert_eq!(result.changes.len(), 2);
        assert!(ready(&result.report).is_ok());
    }

    /// The cleaned file writes text, so a number Excel stored is a change of type.
    #[test]
    fn a_stored_number_is_a_format_change_even_when_its_value_survives() {
        let (grid, plan) = betrag(&["1234"]);
        let result = cleaned(&grid, &plan, &none());
        assert_eq!(result.report.formats[0].rule, "Zahl als Betrag geschrieben");
        assert_eq!(result.changes[0].clean, "1234,00");
    }

    #[test]
    fn an_unreadable_cell_is_an_open_fehler_and_blocks_the_write() {
        let (grid, plan) = datum(&["31.13.2024"]);
        let result = cleaned(&grid, &plan, &none());
        assert_eq!(result.report.summary.fehler_offen, 1);
        assert!(result.report.fehler[0].open);
        assert!(result.changes.is_empty());
        assert!(ready(&result.report).is_err());
    }

    #[test]
    fn a_correction_is_read_like_any_cell_and_closes_the_fehler() {
        let (grid, plan) = datum(&["31.13.2024"]);
        let fixed = CleanDecisions {
            corrections: vec![Correction {
                row: 2,
                column: 2,
                value: "31.12.2024".into(),
            }],
            confirmations: vec![],
        };
        let result = cleaned(&grid, &plan, &fixed);
        assert_eq!(result.report.summary.fehler_offen, 0);
        assert!(!result.report.fehler[0].open);
        assert_eq!(result.changes[0].clean, "31.12.2024");
        assert_eq!(result.changes[0].tier, Tier::Fehler);
        assert_eq!(result.report.summary.korrigiert, 1);
        assert!(ready(&result.report).is_ok());
    }

    /// A typo in the correction is not let through because a human typed it.
    #[test]
    fn a_correction_that_does_not_read_stays_an_open_fehler() {
        let (grid, plan) = datum(&["31.13.2024"]);
        let still_wrong = CleanDecisions {
            corrections: vec![Correction {
                row: 2,
                column: 2,
                value: "32.12.2024".into(),
            }],
            confirmations: vec![],
        };
        let result = cleaned(&grid, &plan, &still_wrong);
        assert_eq!(result.report.summary.fehler_offen, 1);
        assert_eq!(
            result.report.fehler[0].correction.as_deref(),
            Some("32.12.2024")
        );
    }

    #[test]
    fn an_empty_correction_leaves_the_cell_empty() {
        let (grid, plan) = datum(&["31.13.2024"]);
        let emptied = CleanDecisions {
            corrections: vec![Correction {
                row: 2,
                column: 2,
                value: String::new(),
            }],
            confirmations: vec![],
        };
        let result = cleaned(&grid, &plan, &emptied);
        assert_eq!(result.report.summary.fehler_offen, 0);
        assert_eq!(result.changes[0].clean, "");
        assert_eq!(result.changes[0].rule, "Leer gelassen");
    }

    /// `1.234` is 1234 to a German sender and 1.234 to an English one.
    #[test]
    fn an_undecided_decimal_column_is_one_card_with_the_cells_that_differ() {
        let (grid, plan) = betrag(&["1.234", "1.500", "12"]);
        let result = cleaned(&grid, &plan, &none());

        assert_eq!(result.report.cards.len(), 1);
        let card = &result.report.cards[0];
        assert_eq!(
            card.reading,
            Reading::Decimal {
                chosen: DecimalStyle::German,
                alternative: DecimalStyle::English
            }
        );
        assert_eq!(card.count, 2);
        assert_eq!(card.examples[0].raw, "1.234");
        assert_eq!(card.examples[0].chosen, "1234,00");
        assert_eq!(card.examples[0].alternative.as_deref(), Some("1,23"));
        assert!(!card.confirmed);
        assert!(ready(&result.report).is_err());
    }

    /// No cell reads differently, so there is no choice to offer.
    #[test]
    fn an_undecided_column_that_reads_the_same_either_way_raises_no_card() {
        let (grid, plan) = betrag(&["12", "15"]);
        assert!(cleaned(&grid, &plan, &none()).report.cards.is_empty());
    }

    #[test]
    fn confirming_the_alternative_rereads_the_column_and_is_remembered() {
        let (grid, plan) = betrag(&["1.234"]);
        let english = CleanDecisions {
            corrections: vec![],
            confirmations: vec![Confirmation::Decimal {
                column: 2,
                style: DecimalStyle::English,
            }],
        };
        let result = cleaned(&grid, &plan, &english);
        assert!(result.report.cards[0].confirmed);
        assert_eq!(result.report.summary.deutungen_offen, 0);
        assert_eq!(result.changes[0].clean, "1,23");
        assert_eq!(result.changes[0].tier, Tier::Deutung);
        assert_eq!(
            result.confirmed.columns[1].decimal,
            Some(DecimalStyle::English)
        );
        assert!(ready(&result.report).is_ok());
    }

    /// The reading saved last month answers this month — while the file agrees.
    #[test]
    fn a_saved_reading_answers_an_undecided_column() {
        let (grid, mut plan) = betrag(&["1.234"]);
        plan.columns[1].decimal = Some(DecimalStyle::English);
        let result = cleaned(&grid, &plan, &none());
        assert!(result.report.cards.is_empty());
        assert_eq!(result.changes[0].clean, "1,23");
    }

    /// Never auto-resolve: evidence against the saved reading is a question.
    #[test]
    fn a_saved_reading_the_file_contradicts_is_a_card_again() {
        let (grid, mut plan) = betrag(&["1.234,56", "2.000"]);
        plan.columns[1].decimal = Some(DecimalStyle::English);
        let result = cleaned(&grid, &plan, &none());
        assert_eq!(result.report.cards.len(), 1);
        assert!(result.report.cards[0].reason.contains("Vorlage"));
    }

    #[test]
    fn an_ambiguous_date_order_is_a_card() {
        let (grid, plan) = datum(&["03/04/2024", "2024-05-06"]);
        let result = cleaned(&grid, &plan, &none());
        assert_eq!(result.report.cards.len(), 1);
        let card = &result.report.cards[0];
        assert_eq!(card.count, 1);
        assert_eq!(card.examples[0].chosen, "03.04.2024");
        assert_eq!(card.examples[0].alternative.as_deref(), Some("04.03.2024"));
    }

    /// A 13 in the first place settles the order for the whole column.
    #[test]
    fn a_conclusive_date_column_asks_nothing() {
        let (grid, plan) = datum(&["03/04/2024", "13/04/2024"]);
        let result = cleaned(&grid, &plan, &none());
        assert!(result.report.cards.is_empty());
        assert_eq!(result.changes[0].tier, Tier::Format);
    }

    #[test]
    fn a_parser_warning_is_a_hinweis_card_until_acknowledged() {
        let grid = Grid::from_text("Tabelle1", &[&["Wagen"], &["3385 0659 002"]]);
        let plan = plan_for(&grid, &[("Wagen", FieldKind::Wagennummer)]);
        let result = cleaned(&grid, &plan, &none());
        assert_eq!(result.report.cards.len(), 1);
        assert_eq!(result.report.cards[0].reading, Reading::Hinweis);
        assert!(result.report.cards[0].examples[0].message.is_some());

        let acknowledged = CleanDecisions {
            corrections: vec![],
            confirmations: vec![Confirmation::Hinweis { column: 1 }],
        };
        assert!(ready(&cleaned(&grid, &plan, &acknowledged).report).is_ok());
    }

    #[test]
    fn an_unmapped_column_is_never_touched() {
        let grid = Grid::from_text(
            "Tabelle1",
            &[&["Wagen", "Notiz"], &["33 85 0659 002-9", "  frei   text "]],
        );
        let plan = plan_for(&grid, &[("Wagen", FieldKind::Wagennummer)]);
        assert!(cleaned(&grid, &plan, &none()).changes.is_empty());
    }

    fn template_of(plan: &ImportPlan) -> ImportTemplate {
        ImportTemplate {
            id: "t".into(),
            name: "Liste".into(),
            plan: plan.clone(),
            partner_id: None,
            origin: None,
            builtin: false,
            created_at: String::new(),
        }
    }

    #[test]
    fn the_cleaned_file_keeps_every_sheet_and_carries_its_protocol() {
        let folder = TempDir::new("clean-write");
        let original = workbook(
            &folder,
            "Liste.xlsx",
            &[
                (
                    "Daten",
                    &[
                        &["Wagen", "Betrag", "Notiz"],
                        &["338506590029", "1.234,50", "  bleibt  "],
                        &["33 85 0659 002-9", "#99.5", "x"],
                    ],
                ),
                ("Summen", &[&["Summe"], &["1334"]]),
            ],
        );
        let source = grid::read(&original, Some("Daten")).unwrap();
        let plan = plan_for(
            &source.grid,
            &[
                ("Wagen", FieldKind::Wagennummer),
                ("Betrag", FieldKind::Betrag),
            ],
        );
        let result = cleaned(&source.grid, &plan, &none());
        let out = folder.join("bereinigt");
        let written = write::write(&original, "Daten", &result.changes, &out).unwrap();

        assert_eq!(crate::doc::file_name(&written), "Liste.bereinigt.xlsx");
        let book = umya_spreadsheet::reader::xlsx::read(&written).unwrap();
        let names: Vec<&str> = book
            .sheet_collection_no_check()
            .iter()
            .map(|sheet| sheet.name())
            .collect();
        assert_eq!(names, vec!["Daten", "Summen", write::PROTOCOL]);

        let daten = book.sheet_by_name("Daten").unwrap();
        assert_eq!(daten.value((1, 2)), "33 85 0659 002-9");
        assert_eq!(daten.value((2, 2)), "1234,50");
        assert_eq!(daten.value((2, 3)), "99,50");
        assert_eq!(daten.value((3, 2)), "  bleibt  ");

        let protocol = book.sheet_by_name(write::PROTOCOL).unwrap();
        assert_eq!(protocol.value((1, 1)), "Zeile");
        assert_eq!(protocol.value((4, 2)), "338506590029");
        assert_eq!(protocol.value((6, 2)), "Format");
        assert_eq!(
            protocol.highest_row(),
            result.changes.len() as u32 + 1,
            "one protocol row per change"
        );

        // A second run beside the first keeps both.
        let again = write::write(&original, "Daten", &result.changes, &out).unwrap();
        assert_ne!(again, written);
        assert!(original.is_file(), "the original is never written");
    }

    /// Forcing a template onto a file it does not fit is allowed — until the
    /// one field nothing can be imported without is missing.
    #[test]
    fn a_forced_template_without_a_wagennummer_column_is_refused() {
        let folder = TempDir::new("clean-open");
        let file = workbook(
            &folder,
            "fremd.xlsx",
            &[("Tabelle1", &[&["Kunde", "Ort"], &["a", "b"]])],
        );
        let refused = open(
            &file,
            "Tabelle1",
            &builtin("builtin:telematik"),
            UicStyle::Grouped,
        )
        .err()
        .unwrap();
        assert!(refused.into_messages()[0].contains("Telematikdaten"));
    }

    #[test]
    fn opening_rebinds_the_template_onto_the_files_own_columns() {
        let folder = TempDir::new("clean-open-ok");
        let file = workbook(
            &folder,
            "telematik.xlsx",
            &[(
                "Sheet1",
                &[&["Stadt", "Asset"], &["Sehnde", "3385 0659 002-9"]],
            )],
        );
        let held = open(
            &file,
            "Sheet1",
            &builtin("builtin:telematik"),
            UicStyle::Grouped,
        )
        .unwrap();
        assert_eq!(held.plan.columns[1].field, FieldKind::Wagennummer);
        assert_eq!(held.plan.columns[1].index, 2);
        assert!(held.run(&none()).unwrap().report.cards.is_empty());
    }

    /// Read back with the confirmed plan, the cleaned file asks nothing again —
    /// canonical values read the same under any reading.
    #[test]
    fn the_cleaned_file_reads_back_without_a_single_issue() {
        let folder = TempDir::new("clean-roundtrip");
        let original = workbook(
            &folder,
            "Liste.xlsx",
            &[(
                "Daten",
                &[
                    &["Wagen", "Betrag", "Datum"],
                    &["338506590029", "1.234", "03/04/2024"],
                    &["3385.0659.002-9", "#99.5", "#45580"],
                ],
            )],
        );
        let source = grid::read(&original, None).unwrap();
        let plan = plan_for(
            &source.grid,
            &[
                ("Wagen", FieldKind::Wagennummer),
                ("Betrag", FieldKind::Betrag),
                ("Datum", FieldKind::Datum),
            ],
        );
        let decisions = CleanDecisions {
            corrections: vec![],
            confirmations: vec![
                Confirmation::Decimal {
                    column: 2,
                    style: DecimalStyle::German,
                },
                Confirmation::DateOrder {
                    column: 3,
                    order: DateOrder::MonthFirst,
                },
            ],
        };
        let result = cleaned(&source.grid, &plan, &decisions);
        assert!(ready(&result.report).is_ok());
        let written =
            write::write(&original, "Daten", &result.changes, &folder.join("out")).unwrap();

        let back = grid::read(&written, Some("Daten")).unwrap();
        let (detected, _) = recognise::detected(&back.grid).unwrap();
        let reread = recognise::rebind(&template_of(&result.confirmed), &detected);
        let again = cleaned(&back.grid, &reread, &none());
        assert!(again.report.cards.is_empty(), "{:?}", again.report.cards);
        assert!(again.report.fehler.is_empty());
        assert!(
            again.changes.is_empty(),
            "the cleaned file is already clean"
        );
        assert_eq!(back.grid.text(3, 2), "04.03.2024");
    }
}
