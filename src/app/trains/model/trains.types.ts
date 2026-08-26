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
// narrow without a cast.
// ────────────────────────────────────────────────────────────────

import type { ClientReport } from '../../@shared/model/client.types';

export type DecimalStyle = 'german' | 'english';
export type DateOrder = 'dayFirst' | 'monthFirst';
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
  | 'einbauposition'
  | 'eingebautAm'
  | 'ausgebautAm'
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
}

export interface ImportTemplate {
  id: string;
  name: string;
  fingerprint: string;
  plan: ImportPlan;
  partnerId?: string;
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
}

export type EntityDecision =
  { action: 'use'; id: string } | { action: 'create' } | { action: 'skip' };

export interface RowDecision {
  row: number;
  wagen: EntityDecision;
  werkstatt: EntityDecision;
  halter: EntityDecision;
  eigentuemer: EntityDecision;
  radsatz: EntityDecision;
}

export interface CommitDecisions {
  stagingId: string;
  rows: RowDecision[];
  saveTemplateAs?: string;
}

export interface TrainsCounts {
  wagen: number;
  partners: number;
  events: number;
  radsaetze: number;
}

export interface InstandhaltungPage {
  rows: Instandhaltung[];
  total: number;
  offset: number;
}

export interface TrainsData {
  wagen?: Wagen[];
  partners?: Partner[];
  radsaetze?: Radsatz[];
  einbauten?: Einbau[];
  templates?: ImportTemplate[];
  counts?: TrainsCounts;
  staging?: StagedImport;
  instandhaltungPage?: InstandhaltungPage;
  message?: ClientReport;
}
