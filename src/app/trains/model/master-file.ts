// ─── why ────────────────────────────────────────────────────────
// The mirror of the master FILE types in `src-tauri/src/trains/model.rs` —
// the customer's workbook copied in and cleaned, in versions. Kept apart from
// `trains.types.ts` because it shares nothing with the master bindings, mirror
// or export there. Change one, change the other, and `e2e/fake-backend.ts`.
//
// `tailRowsCut` is counted apart from `rowsCut` because those rows held
// formulas (or `0`/`#NV`) — the one place the cleaning removes a formula, so
// the report names it on its own.
//
// `versions` is newest first: `versions[0]` IS the current master. `pending`
// is a cleaned version the user has not taken over yet.
// ────────────────────────────────────────────────────────────────

export interface MasterFile {
  versions: MasterFileVersion[];
  pending?: MasterFileVersion;
}

export interface MasterFileVersion {
  id: string;
  name: string;
  folder: string;
  original: string;
  cleaned: string;
  originalHash: string;
  cleanedHash: string;
  bereinigtAm: string;
  uebernommenAm?: string;
  report: MasterFileReport;
}

export interface MasterFileReport {
  sheets: MasterFileSheet[];
  totals: MasterFileTotals;
}

export interface MasterFileTotals {
  rowsCut: number;
  tailRowsCut: number;
  trimmed: number;
  numbers: number;
  dates: number;
  notes: number;
}

export interface MasterFileSheet {
  sheet: string;
  rowsCut: number;
  tailRowsCut: number;
  formulaTail?: number;
  trimmed: number;
  numbers: number;
  dates: number;
  examples: MasterFileChange[];
  notes: MasterFileNote[];
  noteCount: number;
}

export type MasterFileRule = 'trimmed' | 'number' | 'date';

export interface MasterFileChange {
  row: number;
  column: number;
  header: string;
  raw: string;
  clean: string;
  rule: MasterFileRule;
}

export interface MasterFileNote {
  row: number;
  column: number;
  header: string;
  raw: string;
  reason: string;
}

export const RULE_LABELS: Record<MasterFileRule, string> = {
  trimmed: 'Leerzeichen entfernt',
  number: 'Zahl statt Text',
  date: 'Datum statt Text',
};
