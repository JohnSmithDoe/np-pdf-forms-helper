// ─── why ────────────────────────────────────────────────────────
// The trains wire contract, hand-written here and hand-mirrored in
// `src/app/trains/model/` and `e2e/fake-backend.ts`. Nothing generates them —
// the JSON SHAPE is the contract. Change one, change the others.
//
// Same rule as `crate::model`: anything not modelled here is DROPPED on the next
// write, which is how UI-only keys stay out of the store for free. The layout
// vocabulary is the one exception and lives in `sheet/layout.rs`, because that
// is where it is produced and a second copy would be the thing that drifts.
//
// `DecimalStyle` and `DateOrder` are properties of the SENDER'S FILE, never of
// the machine reading it: nothing in trains may consult a system locale, or one
// file parses differently on two desks.
//
// THE NAMES HERE ARE THE TRADE'S, not translations of English ones. The users
// are Halter of freight wagons, and `docs/fachdomaene.md` is the glossary this
// file has to agree with — change a name here, change it there. German where the
// user has a word (Wagen, Radsatz, Einbau, Instandhaltung, Halter, Werkstatt),
// English where only the app does (`Provenance`, `Resolution`, `dedupe_key`).
// Identifiers stay ASCII; umlauts belong to the labels.
//
// `Halter`, `Eigentuemer` and `Werkstatt` are ONE `Partner` with a `rollen` list.
// A workshop that also keeps wagons is real, and three structs would duplicate
// the alias machinery to say it. `aliases` is the learning loop: every raw source
// spelling the user has ever confirmed for that partner, so "Fa. Müller GmbH &
// Co. KG" is asked about once and resolves silently from then on.
//
// HALTER AND EIGENTÜMER ARE DIFFERENT PARTIES and a `Wagen` carries both. The
// Halter is the keeper the vehicle is registered to — what the user itself is, and
// what a sender's "Halter" column means. The Eigentümer is usually a leasing
// company nobody in a workshop ever names. One `owner_id` for both was not a
// translation but a mistake.
//
// A RADSATZ is an asset in its own right that moves between Wagen, so its history
// is a list of `Einbau`s — radsatz, wagen, eingebaut, ausgebaut — rather than a
// field on either. An open Einbau (no `ausgebaut_am`) is what "currently fitted"
// means, and a Radsatz has at most one.
//
// Work done to a Radsatz is an `Instandhaltung` with `radsatz_id` set, not a
// second type: it is the same invoice line, and splitting it would mean two
// places to look for "what was done". `Instandhaltung` is the DIN 31051 umbrella
// — Wartung is only one of its four sub-activities, so it cannot be the name.
//
// `Wagen.nummer` is the canonical twelve digits and the ONLY thing matched on;
// it carries a check digit, which is why it may decide without asking. A
// `Radsatz.nummer` carries none and is not unique across senders, so it may not —
// see `resolve/`. `Instandhaltung.datum` is ISO `YYYY-MM-DD` because it sorts
// lexicographically, and `betrag_cent` is cents because money is never an f64.
// `dedupe_key` is what stops a re-imported file doubling every row, and
// `Provenance` is four fields that answer "where did this number come from".
//
// `TrainsData` follows `ClientData`'s presence rule — a list that is THERE is the
// whole current one, and absent means the command could not have changed it —
// with one deliberate exception. Instandhaltungen are served as an
// `InstandhaltungPage` from `query_events`, not as a whole-list `Option`: the
// convention removes a guess and is affordable only while the list fits in a
// message, and a full list would make the convention the performance problem.
// `counts` carries the total.
// ────────────────────────────────────────────────────────────────

use serde::{Deserialize, Serialize};

use super::sheet::layout::{Candidate, LayoutHint};
use super::sheet::readers::ReaderKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DecimalStyle {
    German,
    English,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DateOrder {
    DayFirst,
    MonthFirst,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Provenance {
    pub file: String,
    pub sheet: String,
    pub row: u32,
    pub imported_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Wagen {
    pub id: String,
    pub nummer: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub halter_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eigentuemer_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bauart: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bemerkung: Option<String>,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Provenance>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PartnerRolle {
    Halter,
    Eigentuemer,
    Werkstatt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Partner {
    pub id: String,
    pub rollen: Vec<PartnerRolle>,
    pub name: String,
    pub match_key: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bemerkung: Option<String>,
    pub created_at: String,
}

impl Partner {
    pub fn has_rolle(&self, rolle: PartnerRolle) -> bool {
        self.rollen.contains(&rolle)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instandhaltung {
    pub id: String,
    pub wagen_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub werkstatt_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radsatz_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub datum: Option<String>,
    pub leistung: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub betrag_cent: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bemerkung: Option<String>,
    pub dedupe_key: String,
    pub source: Provenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RadsatzAlias {
    pub match_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partner_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Radsatz {
    pub id: String,
    pub nummer: String,
    pub match_key: String,
    #[serde(default)]
    pub aliases: Vec<RadsatzAlias>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wellennummer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bauart: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bemerkung: Option<String>,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Provenance>,
}

impl Radsatz {
    /// Tier one, and the whole point of the scoped alias: a spelling the user
    /// confirmed FOR THIS SENDER decides. The same spelling from anyone else
    /// does not.
    pub fn known_to(&self, key: &str, sender: Option<&str>) -> bool {
        self.aliases
            .iter()
            .any(|alias| alias.match_key == key && alias.partner_id.as_deref() == sender)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Einbau {
    pub id: String,
    pub radsatz_id: String,
    pub wagen_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eingebaut_am: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ausgebaut_am: Option<String>,
    pub source: Provenance,
}

impl Einbau {
    pub fn is_open(&self) -> bool {
        self.ausgebaut_am.is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FieldKind {
    Wagennummer,
    Datum,
    Werkstatt,
    Halter,
    Eigentuemer,
    Leistung,
    Betrag,
    Bemerkung,
    Radsatznummer,
    Wellennummer,
    RadsatzSystemId,
    Einbauposition,
    EingebautAm,
    AusgebautAm,
    Ignorieren,
}

impl FieldKind {
    pub fn label(self) -> &'static str {
        match self {
            FieldKind::Wagennummer => "Wagennummer",
            FieldKind::Datum => "Datum",
            FieldKind::Werkstatt => "Werkstatt",
            FieldKind::Halter => "Halter",
            FieldKind::Eigentuemer => "Eigentümer",
            FieldKind::Leistung => "Leistung",
            FieldKind::Betrag => "Betrag",
            FieldKind::Bemerkung => "Bemerkung",
            FieldKind::Radsatznummer => "Radsatznummer",
            FieldKind::Wellennummer => "Radsatzwellennummer",
            FieldKind::RadsatzSystemId => "Radsatz-ID",
            FieldKind::Einbauposition => "Einbauposition",
            FieldKind::EingebautAm => "Eingebaut am",
            FieldKind::AusgebautAm => "Ausgebaut am",
            FieldKind::Ignorieren => "Nicht importieren",
        }
    }

    pub fn required(self) -> bool {
        matches!(self, FieldKind::Wagennummer)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnBinding {
    pub header: String,
    pub index: u32,
    pub field: FieldKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decimal: Option<DecimalStyle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_order: Option<DateOrder>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPlan {
    pub reader: ReaderKind,
    pub layout: LayoutHint,
    pub columns: Vec<ColumnBinding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_id: Option<String>,
    #[serde(default)]
    pub date1904: bool,
}

impl ImportPlan {
    pub fn binding(&self, field: FieldKind) -> Option<&ColumnBinding> {
        self.columns.iter().find(|column| column.field == field)
    }

    pub fn missing_required(&self) -> Vec<FieldKind> {
        [FieldKind::Wagennummer]
            .into_iter()
            .filter(|field| self.binding(*field).is_none())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportTemplate {
    pub id: String,
    pub name: String,
    pub fingerprint: String,
    pub plan: ImportPlan,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partner_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    Warnung,
    Fehler,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CellIssue {
    pub row: u32,
    pub column: String,
    pub raw: String,
    pub message: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchCandidate {
    pub id: String,
    pub name: String,
    pub score: u8,
    pub why: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum Resolution {
    Known {
        id: String,
        name: String,
    },
    Likely {
        id: String,
        name: String,
        hint: String,
    },
    Ambiguous {
        candidates: Vec<MatchCandidate>,
    },
    New {
        proposal: String,
    },
    Missing,
}

impl Resolution {
    pub fn needs_input(&self) -> bool {
        matches!(self, Resolution::Ambiguous { .. } | Resolution::New { .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RowStatus {
    Ready,
    NeedsInput,
    Duplicate,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StagedCell {
    pub column: u32,
    pub field: FieldKind,
    pub raw: String,
    pub parsed: String,
    pub ok: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StagedRow {
    pub row: u32,
    pub status: RowStatus,
    pub cells: Vec<StagedCell>,
    pub wagen: Resolution,
    pub werkstatt: Resolution,
    pub halter: Resolution,
    pub eigentuemer: Resolution,
    pub radsatz: Resolution,
    pub issues: Vec<CellIssue>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StagedSummary {
    pub total: u32,
    pub ready: u32,
    pub needs_input: u32,
    pub duplicates: u32,
    pub rejected: u32,
    pub neue_wagen: u32,
    pub neue_partner: u32,
    pub neue_radsaetze: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StagedImport {
    pub id: String,
    pub file: String,
    pub sheet: String,
    pub sheets: Vec<String>,
    pub plan: ImportPlan,
    pub candidates: Vec<Candidate>,
    pub rows: Vec<StagedRow>,
    pub summary: StagedSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum EntityDecision {
    Use { id: String },
    Create,
    Skip,
}

fn skip_decision() -> EntityDecision {
    EntityDecision::Skip
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RowDecision {
    pub row: u32,
    pub wagen: EntityDecision,
    pub werkstatt: EntityDecision,
    #[serde(default = "skip_decision")]
    pub halter: EntityDecision,
    #[serde(default = "skip_decision")]
    pub eigentuemer: EntityDecision,
    #[serde(default = "skip_decision")]
    pub radsatz: EntityDecision,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitDecisions {
    pub staging_id: String,
    pub rows: Vec<RowDecision>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub save_template_as: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainsCounts {
    pub wagen: u32,
    pub partner: u32,
    pub instandhaltungen: u32,
    pub radsaetze: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstandhaltungPage {
    pub rows: Vec<Instandhaltung>,
    pub total: u32,
    pub offset: u32,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainsData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wagen: Option<Vec<Wagen>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partner: Option<Vec<Partner>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radsaetze: Option<Vec<Radsatz>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub einbauten: Option<Vec<Einbau>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub templates: Option<Vec<ImportTemplate>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counts: Option<TrainsCounts>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub staging: Option<StagedImport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instandhaltung_page: Option<InstandhaltungPage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<crate::model::ClientReport>,
}

impl TrainsData {
    pub fn nothing() -> Self {
        Self::default()
    }

    pub fn wagen(mut self, wagen: Vec<Wagen>) -> Self {
        self.wagen = Some(wagen);
        self
    }

    pub fn partner(mut self, partner: Vec<Partner>) -> Self {
        self.partner = Some(partner);
        self
    }

    pub fn radsaetze(mut self, radsaetze: Vec<Radsatz>) -> Self {
        self.radsaetze = Some(radsaetze);
        self
    }

    pub fn einbauten(mut self, einbauten: Vec<Einbau>) -> Self {
        self.einbauten = Some(einbauten);
        self
    }

    pub fn templates(mut self, templates: Vec<ImportTemplate>) -> Self {
        self.templates = Some(templates);
        self
    }

    pub fn counts(mut self, counts: TrainsCounts) -> Self {
        self.counts = Some(counts);
        self
    }

    pub fn staging(mut self, staging: StagedImport) -> Self {
        self.staging = Some(staging);
        self
    }

    pub fn instandhaltungen(mut self, page: InstandhaltungPage) -> Self {
        self.instandhaltung_page = Some(page);
        self
    }

    pub fn report(mut self, report: crate::model::ClientReport) -> Self {
        self.message = Some(report);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn an_absent_list_is_absent_from_the_json_rather_than_null() {
        let value = serde_json::to_value(TrainsData::nothing()).unwrap();
        assert_eq!(value, json!({}));
    }

    /// Presence is the signal: an EMPTY list still has to travel, because
    /// "there are none now" and "this command changed nothing" are different.
    #[test]
    fn an_empty_list_is_present_in_the_json() {
        let value = serde_json::to_value(TrainsData::nothing().wagen(Vec::new())).unwrap();
        assert_eq!(value, json!({ "wagen": [] }));
    }

    #[test]
    fn a_resolution_is_tagged_by_its_state() {
        let value = serde_json::to_value(Resolution::Known {
            id: "w1".into(),
            name: "31 80 4740 123-4".into(),
        })
        .unwrap();
        assert_eq!(value["state"], "known");
        assert_eq!(
            serde_json::to_value(Resolution::Missing).unwrap()["state"],
            "missing"
        );
    }

    #[test]
    fn a_decision_is_read_from_its_action() {
        let decision: EntityDecision =
            serde_json::from_value(json!({ "action": "use", "id": "p1" })).unwrap();
        assert_eq!(decision, EntityDecision::Use { id: "p1".into() });
        let decision: EntityDecision =
            serde_json::from_value(json!({ "action": "create" })).unwrap();
        assert_eq!(decision, EntityDecision::Create);
    }

    #[test]
    fn only_ambiguous_and_new_need_the_user() {
        assert!(!Resolution::Missing.needs_input());
        assert!(Resolution::New {
            proposal: "Müller".into()
        }
        .needs_input());
        assert!(Resolution::Ambiguous {
            candidates: Vec::new()
        }
        .needs_input());
    }

    #[test]
    fn a_plan_names_which_required_fields_are_still_unmapped() {
        let plan = ImportPlan {
            reader: ReaderKind::HeaderRow,
            layout: LayoutHint {
                header_row: Some(1),
                first_data_row: 2,
                last_data_row: None,
            },
            columns: vec![ColumnBinding {
                header: "Wagennummer".into(),
                index: 1,
                field: FieldKind::Wagennummer,
                decimal: None,
                date_order: None,
            }],
            template_id: None,
            date1904: false,
        };
        assert!(plan.missing_required().is_empty());
        assert!(plan.binding(FieldKind::Wagennummer).is_some());
    }

    #[test]
    fn every_field_is_labelled_in_german() {
        for field in [
            FieldKind::Wagennummer,
            FieldKind::Datum,
            FieldKind::Werkstatt,
            FieldKind::Halter,
            FieldKind::Leistung,
            FieldKind::Betrag,
            FieldKind::Bemerkung,
            FieldKind::Ignorieren,
        ] {
            assert!(!field.label().is_empty());
        }
        // The wagen number is the only anchor an import needs. A date is not
        // required — plenty of the documents that arrive are about something
        // other than a dated repair.
        assert!(FieldKind::Wagennummer.required());
        assert!(!FieldKind::Datum.required());
        assert!(!FieldKind::Bemerkung.required());
    }
}
