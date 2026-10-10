// ─── why ────────────────────────────────────────────────────────
// The hand-written mirror of `src-tauri/src/trains/model.rs`, and the third copy
// of the same shape after `e2e/fake-backend.ts`. Nothing generates them — the
// JSON SHAPE is the contract, and only the spelling differs (camelCase here,
// `#[serde(rename_all)]` there). Change one, change the others.
//
// THE NAMES ARE THE TRADE'S — `docs/fachdomaene.md` is the glossary this file
// has to agree with. German where the user has a word, English where only the
// app does (`Resolution`, `ColumnBinding`, `Provenance`, `dedupeKey`), and ASCII
// identifiers throughout: umlauts belong to the labels, not to keys.
//
// `TrainsData` follows `ClientData`'s presence rule: a list that is THERE is the
// whole current one and replaces what is held; absent means the command could not
// have changed it. Instandhaltungen are the one exception and arrive as a PAGE,
// because a full list would make the convention the performance problem.
//
// HALTER AND EIGENTÜMER ARE DIFFERENT PARTIES and a `Wagen` carries both — the
// Halter is the keeper the vehicle is registered to (what the user itself is),
// the Eigentümer usually a leasing company nobody in a workshop names.
//
// A RADSATZ moves between Wagen, so its history is a list of `Einbau`s —
// radsatz, wagen, eingebaut, ausgebaut — and an open one (no `ausgebautAm`) is
// what "currently fitted" means. Work done to it is an `Instandhaltung` with
// `radsatzId` set, not a second type. `Instandhaltung` is the DIN 31051 umbrella;
// Wartung is only one of its four sub-activities and cannot be the name.
//
// `Instandhaltung.datum` is OPTIONAL: the Wagennummer is the only anchor an
// import needs, because plenty of the documents that arrive are about something
// other than a dated repair. It is also the only incoming field that can decide
// anything by itself — it carries a check digit and a `Radsatz.nummer` does not.
//
// `Resolution` is tagged on `state` and `EntityDecision` on `action`, so both
// narrow without a cast; `Reading` and `Confirmation` are tagged on `kind`.
//
// A template is either the user's or BUILT-IN (`builtin`, id `builtin:…`), and
// a built-in is read-only: confirming a reading against one writes a user copy
// whose `origin` names the built-in it now shadows.
//
// The guided import's review is three TIERS, and they are three shapes rather
// than one list with a tag, because each asks something different of the user:
// a `FehlerCell` must be corrected or left empty, a `DeutungCard` must be
// confirmed one way or the other, a `FormatGroup` asks nothing. `CleanDecisions`
// is sent WHOLE on every call — the backend keeps no decisions between calls,
// the same as `restage_import` takes the whole plan.
//
// `TrainsSettings` is the Schattensystem's own configuration and lives in the
// backend, not in localStorage: it changes what Rust writes into cleaned copies
// and exports, so Rust has to hold it. `MasterSettings` likewise: the master
// workbook and which of its sheets each template's documents are exported to. `MasterView` is the
// settings plus what only the workbook can say — its sheets and the headers of
// the bound ones — and a `problem` when the file cannot be read. A binding's
// `kind` says what the sheet IS and makes it part of the master import;
// `importRun` is the backend's record of the last run and is never sent back
// meaningfully — `save_master` keeps the stored one. `MasterSheetView` is one
// bound sheet as the mirror holds it, built whole in Rust: the frontend renders
// its columns and rows and decides nothing about them.
//
// The master EXPORT wizard sends a `MasterExportRequest` WHOLE on every change
// and gets a `MasterExportRun` back — the dry run, or after writing a new
// version of the client master, the same shape plus `target`. `file` is derived
// in Rust: always the client master's current version, never set from here. Per sheet, `open` are the document columns that
// still need an answer (an alias onto one of `targets`, or `ignored`); `pairs`
// is what every fed master column takes, `sources` every document column;
// `conflicts` are remembered answers the sheet no longer fits, already dropped.
//
// A staging says where it came from in `origin`: a file being mapped, a filed
// document, or one sheet of the master. The walk matches its staging by it.
// `einbauten` are the master import's date conflicts — one Radsatz already
// fitted on the same Wagen under another date — answered per key; no answer
// keeps the stored date.
//
// A `Dokument` is a file the app OWNS once its cleaning is filed: original and
// cleaned copy in the data folder, identified by the original's content hash,
// with `importiertAm` set once — and only once — it reaches the Schattensystem.
// The import walk answers ENTITIES, not rows: `EntityGroups` is what the backend
// grouped, `EntityDecisions` one answer per group `key` plus the rows to take.
// ────────────────────────────────────────────────────────────────

import type { ClientReport } from '../../@shared/model/client.types';
import type { Farbe } from '../../@shared/model/farbe.types';
import type { MasterFile } from './master-file';

export type DecimalStyle = 'german' | 'english';
export type DateOrder = 'dayFirst' | 'monthFirst';
export type UicStyle = 'compact' | 'grouped';

export interface TrainsSettings {
  wagennummer: UicStyle;
}

export interface MasterAlias {
  master: string;
  source: string;
}

export type SheetKind = 'wagenliste' | 'radsatzEinbau' | 'radsatzBestand';

export interface MasterBinding {
  sheet: string;
  templateId: string;
  kind?: SheetKind;
  key?: string;
  aliases: MasterAlias[];
  ignored?: string[];
  removeFor?: string[];
  auto: boolean;
}

export interface MasterScan {
  modified: number;
  sheets: MasterSheet[];
}

export interface MasterImportRun {
  startedAt: string;
  sheets: string[];
  done: string[];
}

export interface MasterSettings {
  file?: string;
  bindings: MasterBinding[];
  importRun?: MasterImportRun;
  scan?: MasterScan;
}

export interface MasterSheet {
  name: string;
  headers: string[];
}

export interface MasterView {
  settings: MasterSettings;
  sheets: string[];
  headers: MasterSheet[];
  problem?: string;
}

export interface SheetColumn {
  index: number;
  header: string;
  filled: boolean;
}

export interface SheetRow {
  key: string;
  cells: string[];
  source?: string;
}

export interface MasterSheetView {
  sheet: string;
  kind?: SheetKind;
  rowLabel: string;
  columns: SheetColumn[];
  rows: SheetRow[];
  problem?: string;
}

export interface MasterExportChoice {
  sheet: string;
  key?: string;
  aliases: MasterAlias[];
  ignored: string[];
  append: boolean;
  remove: boolean;
}

export interface MasterExportRequest {
  dokumentId: string;
  base: string;
  sheets: MasterExportChoice[];
  remember: boolean;
}

export interface MasterExportBase {
  path: string;
  name: string;
}

export interface MasterExportSheet {
  sheet: string;
  kind?: SheetKind;
  key?: string;
  aliases: MasterAlias[];
  ignored: string[];
  append: boolean;
  remove: boolean;
  matched: number;
  suggested: boolean;
  reason?: string;
}

export interface MasterExportStart {
  dokumentId: string;
  dokument: string;
  template: string;
  bases: MasterExportBase[];
  base: string;
  sheets: MasterExportSheet[];
}

export interface ChangeColumn {
  index: number;
  header: string;
}

export type RowChangeStatus = 'geaendert' | 'neu' | 'geleert';

export interface RowCell {
  text: string;
  before: string;
  changed: boolean;
}

export interface RowChange {
  row: number;
  key: string;
  status: RowChangeStatus;
  cells: RowCell[];
}

export interface MasterExportSheetRun {
  sheet: string;
  append: boolean;
  remove: boolean;
  key?: string;
  aliases: MasterAlias[];
  ignored: string[];
  matched: string[];
  pairs: MasterAlias[];
  sources: string[];
  targets: string[];
  open: string[];
  conflicts: string[];
  problem?: string;
  line: string;
  notes: string[];
  changed: number;
  columns: ChangeColumn[];
  rows: RowChange[];
  removed: string[];
}

export interface MasterExportRun {
  dokumentId: string;
  base: string;
  sheets: MasterExportSheetRun[];
  target?: string;
  folder?: string;
  sicherung?: string;
}
export type ReaderKind = 'headerRow' | 'manual';
export type PartnerRolle = 'halter' | 'eigentuemer' | 'werkstatt';

export type FieldKind =
  | 'wagennummer'
  | 'datum'
  | 'werkstatt'
  | 'halter'
  | 'eigentuemer'
  | 'leistung'
  | 'betrag'
  | 'bemerkung'
  | 'radsatznummer'
  | 'wellennummer'
  | 'radsatzSystemId'
  | 'einbauposition'
  | 'eingebautAm'
  | 'ausgebautAm'
  | 'telematikGeraet'
  | 'telematikAngebautAm'
  | 'telematikZeitpunkt'
  | 'telematikStadt'
  | 'telematikLand'
  | 'telematikStandort'
  | 'telematikLaufleistung'
  | 'telematikEnergie'
  | 'telematikBewegung'
  | 'schadenGemeldetAm'
  | 'schadenGemeldetVon'
  | 'schadcode'
  | 'schadenNotiz'
  | 'ausgesetzt'
  | 'beladen'
  | 'schadenAusfuehrender'
  | 'schadenGeplantAm'
  | 'schadenAktion'
  | 'schadenErledigtAm'
  | 'bestellnummer'
  | 'auftragStatus'
  | 'auftragErfasstAm'
  | 'auftragEingangAm'
  | 'auftragAusgangAm'
  | 'auftragVersendetAm'
  | 'auftragBemerkung'
  | 'pruefart'
  | 'pruefungFaelligAm'
  | 'pruefungGeplantAm'
  | 'pruefungDurchgefuehrtAm'
  | 'pruefungStatus'
  | 'ignorieren';

export interface Provenance {
  file: string;
  sheet: string;
  row: number;
  importedAt: string;
}

export interface Wagen {
  id: string;
  nummer: string;
  halterId?: string;
  eigentuemerId?: string;
  bauart?: string;
  bemerkung?: string;
  createdAt: string;
  source?: Provenance;
}

export interface Partner {
  id: string;
  rollen: PartnerRolle[];
  name: string;
  matchKey: string;
  aliases: string[];
  bemerkung?: string;
  createdAt: string;
}

export interface RadsatzAlias {
  matchKey: string;
  partnerId?: string;
}

export interface Radsatz {
  id: string;
  nummer: string;
  matchKey: string;
  aliases: RadsatzAlias[];
  wellennummer?: string;
  systemId?: string;
  bauart?: string;
  bemerkung?: string;
  createdAt: string;
  source?: Provenance;
}

export interface Einbau {
  id: string;
  radsatzId: string;
  wagenId: string;
  position?: string;
  eingebautAm?: string;
  ausgebautAm?: string;
  source: Provenance;
}

export interface TelematikGeraet {
  id: string;
  kennung: string;
  wagenId: string;
  angebautAm?: string;
  source: Provenance;
}

export interface TelematikMeldung {
  id: string;
  wagenId: string;
  geraetId?: string;
  zeitpunkt: string;
  stadt?: string;
  land?: string;
  standort?: string;
  laufleistungKm?: number;
  energieProzent?: number;
  bewegung?: string;
  source: Provenance;
}

export interface Schadensmeldung {
  id: string;
  wagenId: string;
  gemeldetAm?: string;
  gemeldetVon?: string;
  schadcode?: string;
  notiz?: string;
  ausgesetzt?: boolean;
  beladen?: boolean;
  ausfuehrender?: string;
  geplantAm?: string;
  aktion?: string;
  erledigtAm?: string;
  source: Provenance;
}

export interface Werkstattauftrag {
  id: string;
  wagenId: string;
  bestellnummer: string;
  werkstattId?: string;
  status?: string;
  erfasstAm?: string;
  eingangAm?: string;
  ausgangAm?: string;
  versendetAm?: string;
  bemerkung?: string;
  source: Provenance;
}

export interface Pruefung {
  id: string;
  wagenId: string;
  art?: string;
  faelligAm?: string;
  geplantAm?: string;
  durchgefuehrtAm?: string;
  status?: string;
  bestellnummer?: string;
  source: Provenance;
}

export type EntityRef = 'wagen' | 'radsatz' | 'partner';

export type DetailLinkKind = EntityRef | 'telematik';

export interface DetailLink {
  kind: DetailLinkKind;
  id: string;
}

export type DetailTone = 'danger' | 'warning' | 'medium';

export interface DetailField {
  label: string;
  value: string;
  link?: DetailLink;
}

export interface DetailRow {
  title: string;
  lines: string[];
  link?: DetailLink;
  tone?: DetailTone;
}

export interface DetailSection {
  title: string;
  rows: DetailRow[];
  empty: string;
}

export interface EntityDetail {
  kind: EntityRef;
  id: string;
  title: string;
  subtitle?: string;
  fields: DetailField[];
  sections: DetailSection[];
}

export interface TelematikRow {
  wagenId: string;
  title: string;
  nummer: string;
  geraet?: string;
  standort: string;
  funk: string;
  stumm: boolean;
  tage?: number;
  lines: string[];
}

export interface Farben {
  wagen: Record<string, Farbe>;
  radsaetze: Record<string, Farbe>;
}

export interface Markierungen {
  hand: Farben;
  master: Farben;
}

export interface TelematikView {
  rows: TelematikRow[];
  stumm: number;
}

export interface WagenZustand {
  geraete: TelematikGeraet[];
  meldungen: TelematikMeldung[];
  schaeden: Schadensmeldung[];
  auftraege: Werkstattauftrag[];
  pruefungen: Pruefung[];
}

export interface Instandhaltung {
  id: string;
  wagenId: string;
  werkstattId?: string;
  radsatzId?: string;
  datum?: string;
  leistung: string;
  betragCent?: number;
  bemerkung?: string;
  dedupeKey: string;
  source: Provenance;
}

export interface LayoutHint {
  headerRow?: number;
  firstDataRow: number;
  lastDataRow?: number;
}

export interface LayoutCandidate {
  reader: ReaderKind;
  readerLabel: string;
  score: number;
  reason: string;
  hint: LayoutHint;
}

export interface ColumnBinding {
  header: string;
  index: number;
  field: FieldKind;
  decimal?: DecimalStyle;
  dateOrder?: DateOrder;
}

export interface ImportPlan {
  reader: ReaderKind;
  layout: LayoutHint;
  columns: ColumnBinding[];
  templateId?: string;
  date1904: boolean;
  pruefart?: string;
}

export interface ImportTemplate {
  id: string;
  name: string;
  plan: ImportPlan;
  partnerId?: string;
  origin?: string;
  builtin: boolean;
  createdAt: string;
}

export type Severity = 'warnung' | 'fehler';

export interface CellIssue {
  row: number;
  column: string;
  raw: string;
  message: string;
  severity: Severity;
}

export interface MatchCandidate {
  id: string;
  name: string;
  score: number;
  why: string;
}

export type Resolution =
  | { state: 'known'; id: string; name: string }
  | { state: 'likely'; id: string; name: string; hint: string }
  | { state: 'ambiguous'; candidates: MatchCandidate[] }
  | { state: 'new'; proposal: string }
  | { state: 'missing' };

export type RowStatus = 'ready' | 'needsInput' | 'duplicate' | 'rejected';

export interface StagedCell {
  column: number;
  field: FieldKind;
  raw: string;
  parsed: string;
  ok: boolean;
}

export interface StagedRow {
  row: number;
  status: RowStatus;
  cells: StagedCell[];
  wagen: Resolution;
  werkstatt: Resolution;
  halter: Resolution;
  eigentuemer: Resolution;
  radsatz: Resolution;
  issues: CellIssue[];
  sender?: string;
}

export interface StagedSummary {
  total: number;
  ready: number;
  needsInput: number;
  duplicates: number;
  rejected: number;
  neueWagen: number;
  neuePartner: number;
  neueRadsaetze: number;
}

export interface StagedImport {
  id: string;
  file: string;
  sheet: string;
  sheets: string[];
  plan: ImportPlan;
  candidates: LayoutCandidate[];
  rows: StagedRow[];
  summary: StagedSummary;
  origin: StagingOrigin;
  entities?: EntityGroups;
}

export type StagingOrigin =
  | { kind: 'datei' }
  | { kind: 'dokument'; id: string }
  | { kind: 'master'; sheet: string };

export interface EinbauKonflikt {
  key: string;
  radsatz: string;
  wagen: string;
  bisher?: string;
  bisherQuelle?: string;
  neu: string;
  rows: number[];
}

export interface EinbauChoice {
  key: string;
  uebernehmen: boolean;
}

export type EntityDecision =
  { action: 'use'; id: string } | { action: 'create' } | { action: 'skip' };

export type EntityKind = 'partner' | 'wagen' | 'radsatz';

export interface ProtocolLine {
  row: number;
  column: number;
  header: string;
  raw: string;
  clean: string;
  tier: Tier;
  rule: string;
}

export interface EntityGroup {
  key: string;
  kind: EntityKind;
  rolle?: PartnerRolle;
  spellings: string[];
  resolution: Resolution;
  rows: number[];
  changes: ProtocolLine[];
}

export interface EntityGroups {
  partner: EntityGroup[];
  wagen: EntityGroup[];
  radsaetze: EntityGroup[];
  einbauten: EinbauKonflikt[];
}

export interface EntityChoice {
  key: string;
  decision: EntityDecision;
}

export interface EntityDecisions {
  stagingId: string;
  partner: EntityChoice[];
  wagen: EntityChoice[];
  radsaetze: EntityChoice[];
  einbauten: EinbauChoice[];
  rows: number[];
}

export interface TrainsCounts {
  wagen: number;
  partners: number;
  events: number;
  radsaetze: number;
  dokumente: number;
  archiviert: number;
}

export interface InstandhaltungPage {
  rows: Instandhaltung[];
  total: number;
  offset: number;
}

export type ScanStatus =
  | 'erkannt'
  | 'mehrdeutig'
  | 'unbekannt'
  | 'nichtUnterstuetzt'
  | 'unlesbar'
  | 'vorhanden';

export interface Vorhanden {
  dokumentId: string;
  bereinigtAm: string;
  importiertAm?: string;
  archiviertAm?: string;
}

export interface ScanMatch {
  templateId: string;
  templateName: string;
  sheet: string;
}

export interface ScanFile {
  path: string;
  name: string;
  status: ScanStatus;
  matches: ScanMatch[];
  sheets: string[];
  message?: string;
  vorhanden?: Vorhanden;
}

export type Reading =
  | { kind: 'decimal'; chosen: DecimalStyle; alternative: DecimalStyle }
  | { kind: 'dateOrder'; chosen: DateOrder; alternative: DateOrder }
  | { kind: 'hinweis' };

export interface CardExample {
  row: number;
  raw: string;
  chosen: string;
  alternative?: string;
  message?: string;
}

export interface DeutungCard {
  column: number;
  header: string;
  field: FieldKind;
  reading: Reading;
  reason: string;
  count: number;
  examples: CardExample[];
  confirmed: boolean;
}

export interface FehlerCell {
  row: number;
  column: number;
  header: string;
  raw: string;
  message: string;
  correction?: string;
  open: boolean;
}

export interface FormatSample {
  row: number;
  raw: string;
  clean: string;
}

export interface FormatGroup {
  column: number;
  header: string;
  rule: string;
  count: number;
  samples: FormatSample[];
}

export type Tier = 'fehler' | 'deutung' | 'format';

export interface CleanSummary {
  fehlerOffen: number;
  deutungenOffen: number;
  formatierungen: number;
  korrigiert: number;
}

export interface CleanReport {
  file: string;
  sheet: string;
  templateId: string;
  templateName: string;
  plan: ImportPlan;
  fehler: FehlerCell[];
  cards: DeutungCard[];
  formats: FormatGroup[];
  summary: CleanSummary;
}

export interface Correction {
  row: number;
  column: number;
  value: string;
}

export type Confirmation =
  | { kind: 'decimal'; column: number; style: DecimalStyle }
  | { kind: 'dateOrder'; column: number; order: DateOrder }
  | { kind: 'hinweis'; column: number };

export interface CleanDecisions {
  corrections: Correction[];
  confirmations: Confirmation[];
}

export interface Dokument {
  id: string;
  name: string;
  sheet: string;
  templateId: string;
  templateName: string;
  plan: ImportPlan;
  originalHash: string;
  cleanedHash: string;
  folder: string;
  original: string;
  cleaned: string;
  summary: CleanSummary;
  bereinigtAm: string;
  importiertAm?: string;
  archiviertAm?: string;
}

export interface TrainsData {
  wagen?: Wagen[];
  partners?: Partner[];
  radsaetze?: Radsatz[];
  einbauten?: Einbau[];
  zustand?: WagenZustand;
  templates?: ImportTemplate[];
  counts?: TrainsCounts;
  staging?: StagedImport;
  instandhaltungPage?: InstandhaltungPage;
  scan?: ScanFile[];
  cleaning?: CleanReport;
  dokumente?: Dokument[];
  archiv?: Dokument[];
  settings?: TrainsSettings;
  master?: MasterView;
  masterSheet?: MasterSheetView;
  entityDetail?: EntityDetail;
  telematik?: TelematikView;
  masterImportRun?: MasterImportRun;
  masterExportStart?: MasterExportStart;
  masterExport?: MasterExportRun;
  masterFile?: MasterFile;
  markierungen?: Markierungen;
  message?: ClientReport;
}
