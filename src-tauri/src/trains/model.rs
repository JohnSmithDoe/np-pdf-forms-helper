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
// THE WAGEN-ZUSTAND — what a Wagen's dashboard row shows beyond its fittings —
// is five record types hanging off a Wagen, all imported like everything else
// and none asked about in the walk: a Telematik device (`kennung` = the
// sender's pointer id) and the LATEST reading per Wagen, its `zeitpunkt` ISO
// with the time; Schadensmeldungen („offen“ = no `erledigt_am`, the orange row
// of the customer's dashboard); Werkstattaufträge keyed by `bestellnummer`
// („noch nicht versendet“ = no `versendet_am`, the purple cell); and Prüfungen
// — P8, the yearly inspection, and Revision G4.x — with an `art` from a column
// or the template's fixed `ImportPlan.pruefart`. They travel together as one
// `WagenZustand`, the shape of their one store. See decisions.md, „Der
// Wagen-Zustand ist typisiert“.
//
// An `EntityDetail` is one Wagen, Radsatz or Partner as its detail page shows
// it — backend for frontend: header fields and sections of rows, every string
// already formatted, a row's `link` naming the entity it opens. Angular renders
// it and decides nothing; `detail/` builds it. A `TelematikView` is the
// Telematik list the same way, built by `telematik`.
//
// `TrainsData` follows `ClientData`'s presence rule — a list that is THERE is the
// whole current one, and absent means the command could not have changed it —
// with one deliberate exception. Instandhaltungen are served as an
// `InstandhaltungPage` from `query_events`, not as a whole-list `Option`: the
// convention removes a guess and is affordable only while the list fits in a
// message, and a full list would make the convention the performance problem.
// `counts` carries the total.
//
// A `Dokument` is a file the app OWNS: the original and its cleaned copy are
// copied into `data/trains/dokumente/<id>/`, and the record is the load ledger —
// what came in (`original_hash`, the identity), what it was cleaned with
// (`plan`, so a later import does not depend on how the template has moved on),
// and whether it has been imported (`importiert_am`, final once set).
// `cleaned_hash` is what lets an import notice the copy was edited in Excel.
//
// `EntityDecisions` is the import walk's answer: ONE decision per entity group,
// not per row, plus the rows to take. `entities::expand` turns it into the
// per-row `CommitDecisions` the commit has always run on, which is why that one
// is no longer on the wire.
//
// `TrainsSettings` is the Schattensystem's own configuration, stored beside the
// data rather than in the browser's localStorage: it changes what the backend
// WRITES (cleaned copies, exports), so the backend has to know it, and it holds
// for every desk that shares the data folder. `MasterSettings` is the same
// kind of thing: which workbook is the customer's master and which of its sheets
// a template's documents are exported into — names from the customer's file, so user data and never
// `builtin.rs`. `MasterView` adds what only the workbook can say, its sheets and
// the header row of each bound one, so the page can offer them without a second
// read per select.
//
// A staging names its ORIGIN — a file being mapped, a filed document, or one
// sheet of the master — instead of an optional document id: the commit's gate
// ("only a cleaned document is imported") has to tell the master sheet, which
// is deliberately never filed, apart from a loose file. `SheetKind` is what a
// master sheet IS; the customer's sheet names stay in `MasterBinding.sheet`.
// `EinbauKonflikt` is the master import's one non-entity question — a Radsatz
// already fitted on the same Wagen under another date — answered per Radsatz
// by an `EinbauChoice`.
//
// The master EXPORT is a wizard over one filed Dokument: `MasterExportStart`
// offers the sheets and the base file, a `MasterExportRequest` is the user's
// whole answer — sheets, keys, aliases, `append`, `remove` — sent again on every change like
// `restage_import`'s plan, and a `MasterExportRun` is the dry run or the
// written copy, with each sheet's structure and its changed cells.
// The update is ALWAYS incremental: rows are matched by `key`, a known key is
// updated, an unknown key is appended only where the sheet's `append` is on,
// and a sheet row whose key the document lacks is emptied — never moved —
// only where its `remove` is on. `remove` is remembered per sheet AND template
// (`MasterBinding.remove_for`, template ids): only the user knows which
// documents are complete for which sheet. There is no replace mode any more — a
// `mode` left in an old `master.json` is ignored on read and dropped on the next
// write.
// It writes into the customer's own file (`MasterFile.pfad`) after a backup,
// named in `sicherung`. `MasterSettings.file` is derived — that file, else the
// current version's cleaned copy (`bindings::follow`) — and the backend owns
// it, like `import_run`.
//
// The MASTER FILE is the customer's workbook itself and the only master: copied
// in and cleaned of what nothing reads — never a formula, never a moved row — or
// written by an export (`quelle` set). Everything above reads its current
// version. `MasterFile.versions` is newest first, so the current one is
// `versions[0]`; `pending` is a cleaned version not yet taken over.
//
// `master_import_run` rides on every whole-list answer, and only while a master
// import is OPEN: the dashboard warns about a partial mirror without opening the
// workbook, which `MasterView` would.
//
// Three fields go out RENAMED, because the frontend reads those keys: the
// partner list as `partners`, and the counts as `partners` and `events`. They had
// drifted apart unnoticed — the e2e fake speaks the frontend's names — so a test
// now pins the serialised keys.
//
// A `Farbe` is a MARK, not a fact — the Handfarbe a user puts on a row in Excel.
// Six of them, each one Ionic colour role, so no component ever names a hex
// value. `Markierungen` holds two halves keyed by what survives the master
// import's wipe — the Wagennummer's digits and the Radsatz match key, never an
// id: `hand` is set in the app and wins, `master` is read off the master's key
// cell and rebuilt with the mirror.
// ────────────────────────────────────────────────────────────────

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::sheet::layout::{Candidate, LayoutHint};
use super::sheet::readers::ReaderKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DecimalStyle {
    German,
    English,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UicStyle {
    #[default]
    Compact,
    Grouped,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TrainsSettings {
    pub wagennummer: UicStyle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MasterAlias {
    pub master: String,
    pub source: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SheetKind {
    Wagenliste,
    RadsatzEinbau,
    RadsatzBestand,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MasterBinding {
    pub sheet: String,
    #[serde(default)]
    pub template_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<SheetKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(default)]
    pub aliases: Vec<MasterAlias>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ignored: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove_for: Vec<String>,
    #[serde(default)]
    pub auto: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MasterScan {
    pub modified: u64,
    pub sheets: Vec<MasterSheet>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MasterImportRun {
    pub started_at: String,
    pub sheets: Vec<String>,
    pub done: Vec<String>,
}

impl MasterImportRun {
    pub fn is_open(&self) -> bool {
        self.sheets.iter().any(|sheet| !self.done.contains(sheet))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MasterSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    pub bindings: Vec<MasterBinding>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub import_run: Option<MasterImportRun>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scan: Option<MasterScan>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MasterSheet {
    pub name: String,
    pub headers: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MasterSheetView {
    pub sheet: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<SheetKind>,
    pub row_label: String,
    pub columns: Vec<SheetColumn>,
    pub rows: Vec<SheetRow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetColumn {
    pub index: u32,
    pub header: String,
    pub filled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetRow {
    pub key: String,
    pub cells: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MasterView {
    pub settings: MasterSettings,
    pub sheets: Vec<String>,
    pub headers: Vec<MasterSheet>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MasterExportChoice {
    pub sheet: String,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub aliases: Vec<MasterAlias>,
    #[serde(default)]
    pub ignored: Vec<String>,
    #[serde(default)]
    pub append: bool,
    #[serde(default)]
    pub remove: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MasterExportRequest {
    pub dokument_id: String,
    pub base: String,
    pub sheets: Vec<MasterExportChoice>,
    #[serde(default)]
    pub remember: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MasterExportBase {
    pub path: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MasterExportSheet {
    pub sheet: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<SheetKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    pub aliases: Vec<MasterAlias>,
    pub ignored: Vec<String>,
    pub append: bool,
    pub remove: bool,
    pub matched: u32,
    pub suggested: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MasterExportStart {
    pub dokument_id: String,
    pub dokument: String,
    pub template: String,
    pub bases: Vec<MasterExportBase>,
    pub base: String,
    pub sheets: Vec<MasterExportSheet>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CellChange {
    pub cell: String,
    pub row: u32,
    pub column: String,
    pub key: String,
    pub before: String,
    pub after: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MasterExportSheetRun {
    pub sheet: String,
    pub append: bool,
    pub remove: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    pub aliases: Vec<MasterAlias>,
    pub ignored: Vec<String>,
    pub matched: Vec<String>,
    pub pairs: Vec<MasterAlias>,
    pub sources: Vec<String>,
    pub targets: Vec<String>,
    pub open: Vec<String>,
    pub conflicts: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
    pub line: String,
    pub notes: Vec<String>,
    pub changed: u32,
    pub changes: Vec<CellChange>,
    pub removed: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MasterExportRun {
    pub dokument_id: String,
    pub base: String,
    pub sheets: Vec<MasterExportSheetRun>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sicherung: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelematikGeraet {
    pub id: String,
    pub kennung: String,
    pub wagen_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub angebaut_am: Option<String>,
    pub source: Provenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelematikMeldung {
    pub id: String,
    pub wagen_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geraet_id: Option<String>,
    pub zeitpunkt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stadt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub land: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standort: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub laufleistung_km: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub energie_prozent: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bewegung: Option<String>,
    pub source: Provenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Schadensmeldung {
    pub id: String,
    pub wagen_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gemeldet_am: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gemeldet_von: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schadcode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notiz: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ausgesetzt: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub beladen: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ausfuehrender: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geplant_am: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aktion: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub erledigt_am: Option<String>,
    pub source: Provenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Werkstattauftrag {
    pub id: String,
    pub wagen_id: String,
    pub bestellnummer: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub werkstatt_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub erfasst_am: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eingang_am: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ausgang_am: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub versendet_am: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bemerkung: Option<String>,
    pub source: Provenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pruefung {
    pub id: String,
    pub wagen_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub art: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faellig_am: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geplant_am: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub durchgefuehrt_am: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bestellnummer: Option<String>,
    pub source: Provenance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EntityRef {
    Wagen,
    Radsatz,
    Partner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Farbe {
    Rot,
    Gelb,
    Gruen,
    Blau,
    Lila,
    Grau,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Farben {
    #[serde(default)]
    pub wagen: IndexMap<String, Farbe>,
    #[serde(default)]
    pub radsaetze: IndexMap<String, Farbe>,
}

impl Farben {
    pub fn is_empty(&self) -> bool {
        self.wagen.is_empty() && self.radsaetze.is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Markierungen {
    #[serde(default)]
    pub hand: Farben,
    #[serde(default)]
    pub master: Farben,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LinkKind {
    Wagen,
    Radsatz,
    Partner,
    Telematik,
}

impl From<EntityRef> for LinkKind {
    fn from(kind: EntityRef) -> Self {
        match kind {
            EntityRef::Wagen => LinkKind::Wagen,
            EntityRef::Radsatz => LinkKind::Radsatz,
            EntityRef::Partner => LinkKind::Partner,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetailLink {
    pub kind: LinkKind,
    pub id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DetailTone {
    Danger,
    Warning,
    Medium,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetailField {
    pub label: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<DetailLink>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetailRow {
    pub title: String,
    pub lines: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<DetailLink>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tone: Option<DetailTone>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetailSection {
    pub title: String,
    pub rows: Vec<DetailRow>,
    pub empty: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityDetail {
    pub kind: EntityRef,
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    pub fields: Vec<DetailField>,
    pub sections: Vec<DetailSection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TelematikRow {
    pub wagen_id: String,
    pub title: String,
    pub nummer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geraet: Option<String>,
    pub standort: String,
    pub funk: String,
    pub stumm: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tage: Option<i64>,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TelematikView {
    pub rows: Vec<TelematikRow>,
    pub stumm: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WagenZustand {
    #[serde(default)]
    pub geraete: Vec<TelematikGeraet>,
    #[serde(default)]
    pub meldungen: Vec<TelematikMeldung>,
    #[serde(default)]
    pub schaeden: Vec<Schadensmeldung>,
    #[serde(default)]
    pub auftraege: Vec<Werkstattauftrag>,
    #[serde(default)]
    pub pruefungen: Vec<Pruefung>,
}

impl WagenZustand {
    pub fn without_wagen(&mut self, wagen_id: &str) {
        self.geraete.retain(|item| item.wagen_id != wagen_id);
        self.meldungen.retain(|item| item.wagen_id != wagen_id);
        self.schaeden.retain(|item| item.wagen_id != wagen_id);
        self.auftraege.retain(|item| item.wagen_id != wagen_id);
        self.pruefungen.retain(|item| item.wagen_id != wagen_id);
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
    TelematikGeraet,
    TelematikAngebautAm,
    TelematikZeitpunkt,
    TelematikStadt,
    TelematikLand,
    TelematikStandort,
    TelematikLaufleistung,
    TelematikEnergie,
    TelematikBewegung,
    SchadenGemeldetAm,
    SchadenGemeldetVon,
    Schadcode,
    SchadenNotiz,
    Ausgesetzt,
    Beladen,
    SchadenAusfuehrender,
    SchadenGeplantAm,
    SchadenAktion,
    SchadenErledigtAm,
    Bestellnummer,
    AuftragStatus,
    AuftragErfasstAm,
    AuftragEingangAm,
    AuftragAusgangAm,
    AuftragVersendetAm,
    AuftragBemerkung,
    Pruefart,
    PruefungFaelligAm,
    PruefungGeplantAm,
    PruefungDurchgefuehrtAm,
    PruefungStatus,
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
            FieldKind::TelematikGeraet => "Telematik-Gerät",
            FieldKind::TelematikAngebautAm => "Telematik angebaut am",
            FieldKind::TelematikZeitpunkt => "Telematik-Zeitpunkt",
            FieldKind::TelematikStadt => "Stadt",
            FieldKind::TelematikLand => "Land",
            FieldKind::TelematikStandort => "Standort",
            FieldKind::TelematikLaufleistung => "Laufleistung (km)",
            FieldKind::TelematikEnergie => "Energie-Reserve (%)",
            FieldKind::TelematikBewegung => "Bewegung",
            FieldKind::SchadenGemeldetAm => "Schaden gemeldet am",
            FieldKind::SchadenGemeldetVon => "Schaden gemeldet von",
            FieldKind::Schadcode => "Schadcode",
            FieldKind::SchadenNotiz => "Schadensnotiz",
            FieldKind::Ausgesetzt => "Ausgesetzt",
            FieldKind::Beladen => "Beladen",
            FieldKind::SchadenAusfuehrender => "Ausführende Werkstatt/EVU",
            FieldKind::SchadenGeplantAm => "Schaden geplant am",
            FieldKind::SchadenAktion => "Notwendige Aktion",
            FieldKind::SchadenErledigtAm => "Schaden erledigt am",
            FieldKind::Bestellnummer => "Bestellnummer",
            FieldKind::AuftragStatus => "Auftragsstatus",
            FieldKind::AuftragErfasstAm => "Auftrag erfasst am",
            FieldKind::AuftragEingangAm => "Werkstatteingang",
            FieldKind::AuftragAusgangAm => "Werkstattausgang",
            FieldKind::AuftragVersendetAm => "Auftrag versendet am",
            FieldKind::AuftragBemerkung => "Auftragsbemerkung",
            FieldKind::Pruefart => "Prüfart",
            FieldKind::PruefungFaelligAm => "Prüfung fällig am",
            FieldKind::PruefungGeplantAm => "Prüfung geplant am",
            FieldKind::PruefungDurchgefuehrtAm => "Prüfung durchgeführt am",
            FieldKind::PruefungStatus => "Prüfungsstatus",
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pruefart: Option<String>,
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
    pub plan: ImportPlan,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partner_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    #[serde(default)]
    pub builtin: bool,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender: Option<String>,
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
    pub origin: StagingOrigin,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entities: Option<EntityGroups>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum StagingOrigin {
    #[default]
    Datei,
    Dokument {
        id: String,
    },
    Master {
        sheet: String,
    },
}

impl StagingOrigin {
    pub fn dokument_id(&self) -> Option<&str> {
        match self {
            Self::Dokument { id } => Some(id),
            _ => None,
        }
    }

    pub fn is_master(&self) -> bool {
        matches!(self, Self::Master { .. })
    }
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
    #[serde(default)]
    pub einbau_uebernehmen: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitDecisions {
    pub staging_id: String,
    pub rows: Vec<RowDecision>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityChoice {
    pub key: String,
    pub decision: EntityDecision,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityDecisions {
    pub staging_id: String,
    #[serde(default)]
    pub partner: Vec<EntityChoice>,
    #[serde(default)]
    pub wagen: Vec<EntityChoice>,
    #[serde(default)]
    pub radsaetze: Vec<EntityChoice>,
    #[serde(default)]
    pub einbauten: Vec<EinbauChoice>,
    pub rows: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EinbauChoice {
    pub key: String,
    pub uebernehmen: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EinbauKonflikt {
    pub key: String,
    pub radsatz: String,
    pub wagen: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bisher: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bisher_quelle: Option<String>,
    pub neu: String,
    pub rows: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EntityKind {
    Partner,
    Wagen,
    Radsatz,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityGroup {
    pub key: String,
    pub kind: EntityKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rolle: Option<PartnerRolle>,
    pub spellings: Vec<String>,
    pub resolution: Resolution,
    pub rows: Vec<u32>,
    pub changes: Vec<ProtocolLine>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityGroups {
    pub partner: Vec<EntityGroup>,
    pub wagen: Vec<EntityGroup>,
    pub radsaetze: Vec<EntityGroup>,
    pub einbauten: Vec<EinbauKonflikt>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolLine {
    pub row: u32,
    pub column: u32,
    pub header: String,
    pub raw: String,
    pub clean: String,
    pub tier: Tier,
    pub rule: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dokument {
    pub id: String,
    pub name: String,
    pub sheet: String,
    pub template_id: String,
    pub template_name: String,
    pub plan: ImportPlan,
    pub original_hash: String,
    pub cleaned_hash: String,
    #[serde(default)]
    pub folder: String,
    pub original: String,
    pub cleaned: String,
    pub summary: CleanSummary,
    pub bereinigt_am: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub importiert_am: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Vorhanden {
    pub dokument_id: String,
    pub bereinigt_am: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub importiert_am: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ScanStatus {
    Erkannt,
    Mehrdeutig,
    Unbekannt,
    NichtUnterstuetzt,
    Unlesbar,
    Vorhanden,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanMatch {
    pub template_id: String,
    pub template_name: String,
    pub sheet: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanFile {
    pub path: String,
    pub name: String,
    pub status: ScanStatus,
    pub matches: Vec<ScanMatch>,
    pub sheets: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vorhanden: Option<Vorhanden>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Tier {
    Fehler,
    Deutung,
    Format,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Reading {
    Decimal {
        chosen: DecimalStyle,
        alternative: DecimalStyle,
    },
    DateOrder {
        chosen: DateOrder,
        alternative: DateOrder,
    },
    Hinweis,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardExample {
    pub row: u32,
    pub raw: String,
    pub chosen: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alternative: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeutungCard {
    pub column: u32,
    pub header: String,
    pub field: FieldKind,
    pub reading: Reading,
    pub reason: String,
    pub count: u32,
    pub examples: Vec<CardExample>,
    pub confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FehlerCell {
    pub row: u32,
    pub column: u32,
    pub header: String,
    pub raw: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correction: Option<String>,
    pub open: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormatSample {
    pub row: u32,
    pub raw: String,
    pub clean: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormatGroup {
    pub column: u32,
    pub header: String,
    pub rule: String,
    pub count: u32,
    pub samples: Vec<FormatSample>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanSummary {
    pub fehler_offen: u32,
    pub deutungen_offen: u32,
    pub formatierungen: u32,
    pub korrigiert: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanReport {
    pub file: String,
    pub sheet: String,
    pub template_id: String,
    pub template_name: String,
    pub plan: ImportPlan,
    pub fehler: Vec<FehlerCell>,
    pub cards: Vec<DeutungCard>,
    pub formats: Vec<FormatGroup>,
    pub summary: CleanSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Correction {
    pub row: u32,
    pub column: u32,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Confirmation {
    Decimal { column: u32, style: DecimalStyle },
    DateOrder { column: u32, order: DateOrder },
    Hinweis { column: u32 },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanDecisions {
    #[serde(default)]
    pub corrections: Vec<Correction>,
    #[serde(default)]
    pub confirmations: Vec<Confirmation>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainsCounts {
    pub wagen: u32,
    #[serde(rename = "partners")]
    pub partner: u32,
    #[serde(rename = "events")]
    pub instandhaltungen: u32,
    pub radsaetze: u32,
    pub dokumente: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstandhaltungPage {
    pub rows: Vec<Instandhaltung>,
    pub total: u32,
    pub offset: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MasterFile {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pfad: Option<String>,
    pub versions: Vec<MasterFileVersion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending: Option<MasterFileVersion>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MasterFileVersion {
    pub id: String,
    pub name: String,
    pub folder: String,
    pub original: String,
    pub cleaned: String,
    pub original_hash: String,
    pub cleaned_hash: String,
    pub bereinigt_am: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uebernommen_am: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quelle: Option<String>,
    pub report: MasterFileReport,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MasterFileReport {
    pub sheets: Vec<MasterFileSheet>,
    pub totals: MasterFileTotals,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MasterFileTotals {
    pub rows_cut: u32,
    #[serde(default)]
    pub tail_rows_cut: u32,
    pub trimmed: u32,
    pub numbers: u32,
    pub dates: u32,
    pub notes: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MasterFileSheet {
    pub sheet: String,
    pub rows_cut: u32,
    #[serde(default)]
    pub tail_rows_cut: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formula_tail: Option<u32>,
    pub trimmed: u32,
    pub numbers: u32,
    pub dates: u32,
    pub examples: Vec<MasterFileChange>,
    pub notes: Vec<MasterFileNote>,
    pub note_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MasterFileRule {
    Trimmed,
    Number,
    Date,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MasterFileChange {
    pub row: u32,
    pub column: u32,
    pub header: String,
    pub raw: String,
    pub clean: String,
    pub rule: MasterFileRule,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MasterFileNote {
    pub row: u32,
    pub column: u32,
    pub header: String,
    pub raw: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainsData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wagen: Option<Vec<Wagen>>,
    #[serde(rename = "partners", skip_serializing_if = "Option::is_none")]
    pub partner: Option<Vec<Partner>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radsaetze: Option<Vec<Radsatz>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub einbauten: Option<Vec<Einbau>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub templates: Option<Vec<ImportTemplate>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zustand: Option<WagenZustand>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counts: Option<TrainsCounts>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub staging: Option<StagedImport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instandhaltung_page: Option<InstandhaltungPage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scan: Option<Vec<ScanFile>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cleaning: Option<CleanReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dokumente: Option<Vec<Dokument>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<TrainsSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub master: Option<MasterView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub master_sheet: Option<MasterSheetView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_detail: Option<EntityDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telematik: Option<TelematikView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub master_import_run: Option<MasterImportRun>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub master_export_start: Option<MasterExportStart>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub master_export: Option<MasterExportRun>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub master_file: Option<MasterFile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markierungen: Option<Markierungen>,
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

    pub fn zustand(mut self, zustand: WagenZustand) -> Self {
        self.zustand = Some(zustand);
        self
    }

    pub fn markierungen(mut self, markierungen: Markierungen) -> Self {
        self.markierungen = Some(markierungen);
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

    pub fn cleaning(mut self, cleaning: CleanReport) -> Self {
        self.cleaning = Some(cleaning);
        self
    }

    pub fn settings(mut self, settings: TrainsSettings) -> Self {
        self.settings = Some(settings);
        self
    }

    pub fn dokumente(mut self, dokumente: Vec<Dokument>) -> Self {
        self.dokumente = Some(dokumente);
        self
    }

    pub fn scan(mut self, scan: Vec<ScanFile>) -> Self {
        self.scan = Some(scan);
        self
    }

    pub fn master(mut self, master: MasterView) -> Self {
        self.master = Some(master);
        self
    }

    pub fn master_import_run(mut self, run: Option<MasterImportRun>) -> Self {
        self.master_import_run = run.filter(MasterImportRun::is_open);
        self
    }

    pub fn telematik(mut self, view: TelematikView) -> Self {
        self.telematik = Some(view);
        self
    }

    pub fn entity_detail(mut self, detail: EntityDetail) -> Self {
        self.entity_detail = Some(detail);
        self
    }

    pub fn master_sheet(mut self, sheet: MasterSheetView) -> Self {
        self.master_sheet = Some(sheet);
        self
    }

    pub fn master_export_start(mut self, start: MasterExportStart) -> Self {
        self.master_export_start = Some(start);
        self
    }

    pub fn master_export(mut self, run: MasterExportRun) -> Self {
        self.master_export = Some(run);
        self
    }

    pub fn master_file(mut self, file: MasterFile) -> Self {
        self.master_file = Some(file);
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

    /// The keys `trains.types.ts` and the e2e fake read. These had drifted —
    /// `partner` went out where `partners` was read — and the fake, speaking the
    /// frontend's names, hid it: partner lists never reached the real app.
    #[test]
    fn trains_data_goes_out_under_the_keys_the_frontend_reads() {
        let data = TrainsData::nothing()
            .partner(vec![])
            .counts(TrainsCounts::default());
        let value = serde_json::to_value(&data).unwrap();
        assert!(value.get("partners").is_some(), "{value}");
        assert_eq!(
            value["counts"],
            json!({ "wagen": 0, "partners": 0, "events": 0, "radsaetze": 0, "dokumente": 0 })
        );
    }

    /// The new wire, pinned the same way: these are the keys `trains.types.ts`
    /// and the e2e fake speak.
    #[test]
    fn the_document_wire_goes_out_and_comes_in_under_its_camel_case_keys() {
        assert_eq!(
            serde_json::to_value(ScanStatus::Vorhanden).unwrap(),
            "vorhanden"
        );
        assert_eq!(
            serde_json::to_value(EntityKind::Radsatz).unwrap(),
            "radsatz"
        );

        let dokument = Dokument {
            id: "d1".into(),
            name: "monat.xlsx".into(),
            sheet: "Tabelle1".into(),
            template_id: "t1".into(),
            template_name: "Monatsliste".into(),
            plan: crate::trains::builtin::all()[0].plan.clone(),
            original_hash: "a".into(),
            cleaned_hash: "b".into(),
            folder: "f".into(),
            original: "o".into(),
            cleaned: "c".into(),
            summary: CleanSummary::default(),
            bereinigt_am: "2026-10-03".into(),
            importiert_am: None,
        };
        let value = serde_json::to_value(&dokument).unwrap();
        for key in ["templateId", "originalHash", "cleanedHash", "bereinigtAm"] {
            assert!(value.get(key).is_some(), "{key} in {value}");
        }
        assert!(value.get("importiertAm").is_none());

        let decisions: EntityDecisions = serde_json::from_value(json!({
            "stagingId": "s1",
            "partner": [{ "key": "werkstatt:ULA", "decision": { "action": "create" } }],
            "rows": [2]
        }))
        .unwrap();
        assert_eq!(decisions.partner[0].decision, EntityDecision::Create);
        assert!(decisions.wagen.is_empty());
    }

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
            pruefart: None,
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
