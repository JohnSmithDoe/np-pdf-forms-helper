// ─── why ────────────────────────────────────────────────────────
// A fake of the RUST backend, installed as `window.__TAURI_INTERNALS__` before
// the bundle boots.
//
// It deliberately stubs the TRANSPORT rather than adding a fake
// `BackendService` to `src/`. Two reasons, and the second is the important one:
//   • the app under test then runs the real `TauriBackendService`, so
//     `describe()`'s positional→named mapping and its `{ messages: [] }` error
//     unwrapping are covered instead of bypassed.
//   • no fake ships in the production bundle. A fake backend behind a runtime
//     check is one failed shell-detection away from silently running the real
//     app on an in-memory store and losing the user's data.
//
// The command set mirrors `src-tauri/src/filler/commands.rs` and
// `src-tauri/src/trains/commands.rs` — same names, same argument keys, same
// presence rules (a list is ABSENT when the command cannot have changed it).
// Keep them in step: a fake that has drifted validates a contract that no longer
// exists.
//
// The trains half fakes the STAGED PREVIEW rather than re-implementing the
// parsers. Reproducing `sanitise` in TypeScript would be a second
// implementation of the one thing whose correctness is a property of bytes, and
// it would agree with itself while disagreeing with Rust. So a seed supplies the
// staged rows and the fake serves them.
//
// The native file picker cannot be driven by any browser, so `seed.picker` is
// what the picker "returns" on the next `add_documents`; `null` means the user
// cancelled. `seed.scan` is the same for the guided import's two pickers and
// for a drop, and `seed.clean` is what `clean_file` answers with.
//
// The cleaning fakes the CLEANING REPORT for the reason it fakes the staged
// preview. `reclean_file` only ECHOES decisions — a correction closes its cell,
// a confirmation sets its card and the counts follow — and never re-parses a
// value: whether `31.13.2025` reads is Rust's question. `write_clean` files a
// `Dokument` for the cleaning and applies none of `clean::ready`'s gate, which
// is proved by `cargo test`.
//
// The import walk is faked the same way. `stage_document` serves
// `seed.document` (else `seed.staging`) — entity groups included, because
// grouping is `entities::group`'s and not something to re-implement here — and
// `commit_document` only records the rows it was sent and marks the document
// imported. An imported document is refused exactly as Rust refuses it, since
// that is a rule the UI is built around, not a property of bytes.
//
// The master workbook is `seed.masterSheets` — its sheet names and header rows,
// which is all `get_master` reads of it — and `seed.masterPicker` the path the
// file picker returns.
//
// The master EXPORT wizard is faked shallow. `open_master_export` offers every
// seeded sheet, suggested ones first — those bound to the document's template —
// and warning on an overview without a template; which sheets Rust would suggest beyond that is
// `cargo test`'s. `preview_master_export` and `write_master_export` serve
// `seed.masterExport[sheet]` — hand-written runs with their structure and cell
// changes — and only ECHO the request's answers back: an answered column leaves
// `open`, nothing is pasted or diffed. The write records `lastExport`.
// Recognising a sheet by its header row is `kinds::recognise`'s too, so the
// defaults are SEEDED (`seed.masterDefaults`): a picked new path and
// `reset_master_bindings` apply them, the same path keeps what is bound.
//
// The master IMPORT is faked at the same depth. `start_master_import` empties
// the facts and records the run over the bindings that have a `kind`;
// `stage_master_sheet` serves `seed.masterStaging` (else `seed.document`, else
// `seed.staging`) stamped with the sheet as its origin, Einbau conflicts and
// all — finding them is `entities::einbau_konflikte`'s; `commit_document` ticks
// the sheet off. Every whole-list answer carries `masterImportRun` while a sheet
// is still open and drops it after the last, as `everything()` does — the
// store reads that absence as "closed". `get_master_sheet` answers `seed.masterSheetViews[sheet]`,
// else the bound sheet's headers with no rows: building the view is Rust's.
//
// The master FILE is `seed.masterFile`, and `seed.masterFilePick` is the
// cleaned version `clean_master_file` answers in place of picker + cleaning —
// what the cleaning changes is `master_file::clean`'s and proved by `cargo
// test`. The pick lands as `pending`; accept puts it first, discard drops it.
// ────────────────────────────────────────────────────────────────

import type { Page } from '@playwright/test';

export interface FakeSheet {
  id: string;
  name: string;
}

export interface FakeMappedField {
  origId: string;
  mappedName: string;
}

export type FakeDocument = {
  id: string;
  name: string;
  filename: string;
  mtime: number;
  mapped?: FakeMappedField[];
} & (
  | { type: 'xlsx'; sheets: FakeSheet[] }
  | { type: 'pdf'; fields: { id: string; path: string }[]; previewfile: string }
  | { type: 'resource' }
);

export interface FakeProfile {
  id: string;
  name: string;
  documentIds: string[];
  fieldIds: string[];
}

export interface FakeStagedCell {
  column: number;
  field: string;
  raw: string;
  parsed: string;
  ok: boolean;
}

/** The five resolutions are the five `trains::model::StagedRow` carries, and
 *  Halter and Eigentümer are separate parties — one `owner` for both was drift
 *  from the wire. */
export interface FakeStagedRow {
  row: number;
  status: 'ready' | 'needsInput' | 'duplicate' | 'rejected';
  cells: FakeStagedCell[];
  wagen: Record<string, unknown>;
  werkstatt: Record<string, unknown>;
  halter: Record<string, unknown>;
  eigentuemer: Record<string, unknown>;
  radsatz: Record<string, unknown>;
  issues: Record<string, unknown>[];
}

export interface FakeStaging {
  id: string;
  file: string;
  sheet: string;
  sheets: string[];
  plan: {
    reader: string;
    layout: { headerRow?: number; firstDataRow: number; lastDataRow?: number };
    columns: {
      header: string;
      index: number;
      field: string;
      decimal?: string;
      dateOrder?: string;
    }[];
    templateId?: string;
    date1904: boolean;
  };
  candidates: {
    reader: string;
    readerLabel: string;
    score: number;
    reason: string;
    hint: { headerRow?: number; firstDataRow: number; lastDataRow?: number };
  }[];
  rows: FakeStagedRow[];
  origin?:
    | { kind: 'datei' }
    | { kind: 'dokument'; id: string }
    | { kind: 'master'; sheet: string };
  entities?: {
    partner: FakeEntityGroup[];
    wagen: FakeEntityGroup[];
    radsaetze: FakeEntityGroup[];
    einbauten?: {
      key: string;
      radsatz: string;
      wagen: string;
      bisher?: string;
      bisherQuelle?: string;
      neu: string;
      rows: number[];
    }[];
  };
  summary: {
    total: number;
    ready: number;
    needsInput: number;
    duplicates: number;
    rejected: number;
    neueWagen: number;
    neuePartner: number;
    neueRadsaetze: number;
  };
}

export interface FakeEntityGroup {
  key: string;
  kind: 'partner' | 'wagen' | 'radsatz';
  rolle?: string;
  spellings: string[];
  resolution: Record<string, unknown>;
  rows: number[];
  changes: {
    row: number;
    column: number;
    header: string;
    raw: string;
    clean: string;
    tier: string;
    rule: string;
  }[];
}

export interface FakeDokument {
  id: string;
  name: string;
  sheet: string;
  templateId: string;
  templateName: string;
  plan: FakeStaging['plan'];
  originalHash: string;
  cleanedHash: string;
  folder: string;
  original: string;
  cleaned: string;
  summary: {
    fehlerOffen: number;
    deutungenOffen: number;
    formatierungen: number;
    korrigiert: number;
  };
  bereinigtAm: string;
  importiertAm?: string;
}

export interface FakeWaggon {
  id: string;
  nummer: string;
  halterId?: string;
  eigentuemerId?: string;
  bauart?: string;
  createdAt: string;
}

/** `matchKey` and the sender-scoped `aliases` are what decide a Radsatznummer;
 *  the fake carries them so the shape cannot drift from `trains/model.rs`. */
export interface FakeRadsatz {
  id: string;
  nummer: string;
  matchKey: string;
  aliases: { matchKey: string; partnerId?: string }[];
  wellennummer?: string;
  systemId?: string;
  bauart?: string;
  createdAt: string;
}

export interface FakePartner {
  id: string;
  rollen: ('halter' | 'eigentuemer' | 'werkstatt')[];
  name: string;
  matchKey: string;
  aliases: string[];
  createdAt: string;
}

export interface FakeProvenance {
  file: string;
  sheet: string;
  row: number;
  importedAt: string;
}

/** An OPEN Einbau — no `ausgebautAm` — is what "currently fitted" means, and a
 *  Radsatz has at most one. */
export interface FakeEinbau {
  id: string;
  radsatzId: string;
  wagenId: string;
  position?: string;
  eingebautAm?: string;
  ausgebautAm?: string;
  source: FakeProvenance;
}

/** `datum` is optional on purpose: the Wagennummer is the only anchor a row
 *  needs. `radsatzId` set means the work was done TO a Radsatz — the same
 *  invoice line, not a second type. */
export interface FakeInstandhaltung {
  id: string;
  wagenId: string;
  werkstattId?: string;
  radsatzId?: string;
  datum?: string;
  leistung: string;
  betragCent?: number;
  bemerkung?: string;
  dedupeKey: string;
  source: FakeProvenance;
}

export interface FakeTemplate {
  id: string;
  name: string;
  plan: FakeStaging['plan'];
  partnerId?: string;
  origin?: string;
  builtin: boolean;
  createdAt: string;
}

export interface FakeScanFile {
  path: string;
  name: string;
  status:
    | 'erkannt'
    | 'mehrdeutig'
    | 'unbekannt'
    | 'nichtUnterstuetzt'
    | 'unlesbar'
    | 'vorhanden';
  matches: { templateId: string; templateName: string; sheet: string }[];
  sheets: string[];
  message?: string;
  vorhanden?: {
    dokumentId: string;
    bereinigtAm: string;
    importiertAm?: string;
  };
}

export type FakeReading =
  | { kind: 'decimal'; chosen: string; alternative: string }
  | { kind: 'dateOrder'; chosen: string; alternative: string }
  | { kind: 'hinweis' };

export interface FakeDeutungCard {
  column: number;
  header: string;
  field: string;
  reading: FakeReading;
  reason: string;
  count: number;
  examples: {
    row: number;
    raw: string;
    chosen: string;
    alternative?: string;
    message?: string;
  }[];
  confirmed: boolean;
}

export interface FakeFehlerCell {
  row: number;
  column: number;
  header: string;
  raw: string;
  message: string;
  correction?: string;
  open: boolean;
}

export interface FakeCleanReport {
  file: string;
  sheet: string;
  templateId: string;
  templateName: string;
  plan: FakeStaging['plan'];
  fehler: FakeFehlerCell[];
  cards: FakeDeutungCard[];
  formats: {
    column: number;
    header: string;
    rule: string;
    count: number;
    samples: { row: number; raw: string; clean: string }[];
  }[];
  summary: {
    fehlerOffen: number;
    deutungenOffen: number;
    formatierungen: number;
    korrigiert: number;
  };
}

export interface FakeCleanDecisions {
  corrections: { row: number; column: number; value: string }[];
  confirmations: (
    | { kind: 'decimal'; column: number; style: string }
    | { kind: 'dateOrder'; column: number; order: string }
    | { kind: 'hinweis'; column: number }
  )[];
}

export interface FakeMasterSheet {
  name: string;
  headers: string[];
}

export interface FakeMasterSettings {
  file?: string;
  bindings: {
    sheet: string;
    templateId: string;
    kind?: 'wagenliste' | 'radsatzEinbau' | 'radsatzBestand';
    mode: 'snapshot' | 'feed';
    key?: string;
    aliases: { master: string; source: string }[];
    ignored?: string[];
    auto?: boolean;
  }[];
  importRun?: { startedAt: string; sheets: string[]; done: string[] };
  lastExport?: string;
}

export interface FakeMasterFileVersion {
  id: string;
  name: string;
  folder: string;
  original: string;
  cleaned: string;
  originalHash: string;
  cleanedHash: string;
  bereinigtAm: string;
  uebernommenAm?: string;
  report: {
    sheets: {
      sheet: string;
      rowsCut: number;
      tailRowsCut: number;
      formulaTail?: number;
      trimmed: number;
      numbers: number;
      dates: number;
      examples: {
        row: number;
        column: number;
        header: string;
        raw: string;
        clean: string;
        rule: 'trimmed' | 'number' | 'date';
      }[];
      notes: {
        row: number;
        column: number;
        header: string;
        raw: string;
        reason: string;
      }[];
      noteCount: number;
    }[];
    totals: {
      rowsCut: number;
      tailRowsCut: number;
      trimmed: number;
      numbers: number;
      dates: number;
      notes: number;
    };
  };
}

export interface FakeMasterFile {
  versions: FakeMasterFileVersion[];
  pending?: FakeMasterFileVersion;
}

export interface FakeExportSheetRun {
  matched: string[];
  targets: string[];
  open: string[];
  conflicts?: string[];
  problem?: string;
  line: string;
  notes?: string[];
  changed: number;
  changes: {
    cell: string;
    row: number;
    column: string;
    key: string;
    before: string;
    after: string;
  }[];
}

interface FakeExportRequest {
  dokumentId: string;
  base: string;
  sheets: {
    sheet: string;
    key?: string;
    aliases: { master: string; source: string }[];
    ignored: string[];
  }[];
  remember: boolean;
}

export interface FakeMasterSheetView {
  sheet: string;
  kind?: string;
  rowLabel: string;
  columns: { index: number; header: string; filled: boolean }[];
  rows: { key: string; cells: string[]; source?: string }[];
  problem?: string;
}

export interface FakeSeed {
  documents?: FakeDocument[];
  profiles?: FakeProfile[];
  wagen?: FakeWaggon[];
  partners?: FakePartner[];
  radsaetze?: FakeRadsatz[];
  einbauten?: FakeEinbau[];
  events?: FakeInstandhaltung[];
  templates?: FakeTemplate[];
  /** What `stage_import` and `stage_import_path` hand back. `null` = the picker was cancelled. */
  staging?: FakeStaging | null;
  /** What the guided pickers and a drop scan. `null` = the picker was cancelled. */
  scan?: FakeScanFile[] | null;
  /** What `clean_file` answers with, whatever file and template it is sent. */
  clean?: FakeCleanReport | null;
  /** The Schattensystem settings; compact Wagennummern when absent. */
  settings?: { wagennummer: string };
  /** The documents the app already owns. */
  dokumente?: FakeDokument[];
  /** What `stage_document` stages, whatever id it is sent; falls back to `staging`. */
  document?: FakeStaging | null;
  /** What the native picker hands back on the next add. `null` = cancelled. */
  picker?: FakeDocument | null;
  /** The master workbook settings; none chosen when absent. */
  master?: FakeMasterSettings;
  /** The master workbook's sheets with their header rows. */
  masterSheets?: FakeMasterSheet[];
  /** What the master file picker hands back. `null` = cancelled. */
  masterPicker?: string | null;
  /** The bindings recognition would produce — `pick_master_file` on a new path and `reset_master_bindings` apply them. */
  masterDefaults?: FakeMasterSettings['bindings'];
  /** What `stage_master_sheet` stages, whatever sheet; falls back to `document`, then `staging`. */
  masterStaging?: FakeStaging | null;
  /** What `get_master_sheet` answers per sheet name. */
  masterSheetViews?: Record<string, FakeMasterSheetView>;
  /** What the master export's dry run and write answer per sheet name. */
  masterExport?: Record<string, FakeExportSheetRun>;
  /** The master file's versions and its untaken pick. */
  masterFile?: FakeMasterFile;
  /** What `clean_master_file` answers in place of picker and cleaning; `null` = cancelled. */
  masterFilePick?: FakeMasterFileVersion | null;
  /** Command name → the German lines it should reject with. */
  failures?: Record<string, string[]>;
}

export interface RecordedCall {
  command: string;
  args: Record<string, unknown>;
}

/** Must run before `page.goto` — the bundle reads the globals at bootstrap. */
export async function installFakeBackend(
  page: Page,
  seed: FakeSeed = {}
): Promise<void> {
  await page.addInitScript(install, seed);
}

/** Every invoke the app made, in order — for asserting what was SENT. */
export function recordedCalls(page: Page): Promise<RecordedCall[]> {
  return page.evaluate(
    () =>
      (window as unknown as { __npFake: { calls: RecordedCall[] } }).__npFake
        .calls
  );
}

/**
 * Installs the fake on the CURRENT page — what `installFakeBackend` sends into
 * the browser, callable directly.
 *
 * `dev/main.mock.ts` calls it that way to serve the real app against this same
 * fake (`pnpm run start:mock`), so the command set stays in one file.
 *
 * Serialised into the page by `addInitScript`, so it must be self-contained: no
 * imports, no closure over anything in this module.
 */
export function install(seed: FakeSeed): void {
  interface State {
    documents: FakeDocument[];
    profiles: FakeProfile[];
    picker: FakeDocument | null;
    wagen: FakeWaggon[];
    partners: FakePartner[];
    radsaetze: FakeRadsatz[];
    einbauten: FakeEinbau[];
    events: FakeInstandhaltung[];
    templates: FakeTemplate[];
    staging: FakeStaging | null;
    scan: FakeScanFile[] | null;
    clean: FakeCleanReport | null;
    cleaning: FakeCleanReport | null;
    dokumente: FakeDokument[];
    document: FakeStaging | null;
    settings: { wagennummer: string };
    master: FakeMasterSettings;
    masterSheets: FakeMasterSheet[];
    masterPicker: string | null;
    masterDefaults: FakeMasterSettings['bindings'];
    masterStaging: FakeStaging | null;
    masterSheetViews: Record<string, FakeMasterSheetView>;
    masterExport: Record<string, FakeExportSheetRun>;
    masterFile: FakeMasterFile;
    masterFilePick: FakeMasterFileVersion | null;
    committed: number[];
    failures: Record<string, string[]>;
    calls: RecordedCall[];
  }

  const state: State = {
    documents: seed.documents ?? [],
    profiles: seed.profiles ?? [],
    picker: seed.picker ?? null,
    wagen: seed.wagen ?? [],
    partners: seed.partners ?? [],
    radsaetze: seed.radsaetze ?? [],
    einbauten: seed.einbauten ?? [],
    events: seed.events ?? [],
    templates: seed.templates ?? [],
    staging: seed.staging ?? null,
    scan: seed.scan ?? null,
    clean: seed.clean ?? null,
    cleaning: null,
    dokumente: seed.dokumente ?? [],
    document: seed.document ?? null,
    settings: seed.settings ?? { wagennummer: 'compact' },
    master: seed.master ?? { bindings: [] },
    masterSheets: seed.masterSheets ?? [],
    masterPicker: seed.masterPicker ?? null,
    masterDefaults: seed.masterDefaults ?? [],
    masterStaging: seed.masterStaging ?? null,
    masterSheetViews: seed.masterSheetViews ?? {},
    masterExport: seed.masterExport ?? {},
    masterFile: seed.masterFile ?? { versions: [] },
    masterFilePick: seed.masterFilePick ?? null,
    committed: [],
    failures: seed.failures ?? {},
    calls: [],
  };

  const copy = <T>(value: T): T => JSON.parse(JSON.stringify(value)) as T;

  const report = (
    headline: string,
    messages: string[] = [],
    folder?: string
  ) => ({
    headline,
    messages,
    ...(folder ? { messageFolder: folder } : {}),
  });

  const lists = () => ({
    documents: copy(state.documents),
    profiles: copy(state.profiles),
  });

  const counts = () => ({
    wagen: state.wagen.length,
    partners: state.partners.length,
    events: state.events.length,
    radsaetze: state.radsaetze.length,
    dokumente: state.dokumente.length,
  });

  const trainsLists = () => ({
    wagen: copy(state.wagen),
    partners: copy(state.partners),
    radsaetze: copy(state.radsaetze),
    einbauten: copy(state.einbauten),
    templates: copy(state.templates),
    dokumente: copy(state.dokumente),
    settings: copy(state.settings),
    counts: counts(),
    masterImportRun: openRun(),
    masterFile: copy(state.masterFile),
  });

  const openRun = () => {
    const run = state.master.importRun;
    return run && run.sheets.some((sheet) => !run.done.includes(sheet))
      ? copy(run)
      : undefined;
  };

  const masterView = () => {
    const bound = new Set(state.master.bindings.map((entry) => entry.sheet));
    const open = !!state.master.file;
    return {
      master: {
        settings: copy(state.master),
        sheets: open ? state.masterSheets.map((sheet) => sheet.name) : [],
        headers: open
          ? copy(state.masterSheets.filter((sheet) => bound.has(sheet.name)))
          : [],
      },
    };
  };

  const baseName = (path: string) => path.split(/[\\/]/).pop() ?? path;

  const exportRun = (request: FakeExportRequest) => ({
    dokumentId: request.dokumentId,
    base: request.base,
    sheets: request.sheets.map((choice) => {
      const seeded = state.masterExport[choice.sheet] ?? {
        matched: [],
        targets: [],
        open: [],
        line: `„${choice.sheet}“: übernommen.`,
        changed: 0,
        changes: [],
      };
      const answered = new Set([
        ...choice.aliases.map((alias) => alias.source),
        ...choice.ignored,
      ]);
      const mode =
        state.master.bindings.find((entry) => entry.sheet === choice.sheet)
          ?.mode ?? 'snapshot';
      return {
        ...copy(seeded),
        sheet: choice.sheet,
        mode,
        key: choice.key,
        aliases: copy(choice.aliases),
        ignored: copy(choice.ignored),
        open: seeded.open.filter((column) => !answered.has(column)),
        conflicts: copy(seeded.conflicts ?? []),
        notes: copy(seeded.notes ?? []),
      };
    }),
  });

  const scanned = () => (state.scan ? { scan: copy(state.scan) } : {});

  const recleaned = (decisions: FakeCleanDecisions): FakeCleanReport | null => {
    if (!state.cleaning) return null;
    const report = copy(state.cleaning);
    for (const cell of report.fehler) {
      const fix = decisions.corrections.find(
        (entry) => entry.row === cell.row && entry.column === cell.column
      );
      if (fix) {
        cell.correction = fix.value;
        cell.open = false;
      }
    }
    for (const card of report.cards) {
      const confirmation = decisions.confirmations.find(
        (entry) => entry.column === card.column
      );
      if (!confirmation) continue;
      card.confirmed = true;
      if (confirmation.kind === 'decimal' && card.reading.kind === 'decimal') {
        if (confirmation.style !== card.reading.chosen) {
          card.reading = {
            kind: 'decimal',
            chosen: card.reading.alternative,
            alternative: card.reading.chosen,
          };
        }
      }
      if (
        confirmation.kind === 'dateOrder' &&
        card.reading.kind === 'dateOrder'
      ) {
        if (confirmation.order !== card.reading.chosen) {
          card.reading = {
            kind: 'dateOrder',
            chosen: card.reading.alternative,
            alternative: card.reading.chosen,
          };
        }
      }
    }
    report.summary = {
      ...report.summary,
      fehlerOffen: report.fehler.filter((cell) => cell.open).length,
      deutungenOffen: report.cards.filter((card) => !card.confirmed).length,
      korrigiert: report.fehler.filter((cell) => !cell.open).length,
    };
    return report;
  };

  const commands: Record<string, (args: Record<string, never>) => unknown> = {
    // No report: the welcome text was retired when the load moved into a route
    // resolver, which runs before any page subscribes to `report$`.
    get_client_data: () => lists(),

    app_info: () => ({
      name: 'npDokumentenhilfe',
      version: '1.1.9',
      dataPath: '/fake/data',
      outputPath: '/fake/data/output',
      cachePath: '/fake/data/cache',
    }),

    add_documents: () => {
      if (!state.picker) return { documents: copy(state.documents) };
      state.documents.push(copy(state.picker));
      state.picker = null;
      return {
        documents: copy(state.documents),
        message: report('Dokument wurde erfolgreich hinzugefügt'),
      };
    },

    remap_document: () => lists(),

    save_document: (args) => {
      const document = args['document'] as unknown as FakeDocument;
      const index = state.documents.findIndex(
        (entry) => entry.id === document.id
      );
      if (index >= 0) state.documents[index] = copy(document);
      return {
        ...lists(),
        message: report('Dokument wurde erfolgreich gespeichert'),
      };
    },

    remove_document: (args) => {
      const id = args['id'] as unknown as string;
      state.documents = state.documents.filter((entry) => entry.id !== id);
      return {
        ...lists(),
        message: report('Dokument wurde erfolgreich entfernt'),
      };
    },

    reset_app: () => {
      state.documents = [];
      state.profiles = [];
      return {
        ...lists(),
        message: report('System wurde erfolgreich zurückgesetzt'),
      };
    },

    save_profiles: (args) => {
      state.profiles = copy(args['profiles'] as unknown as FakeProfile[]);
      return {
        profiles: copy(state.profiles),
        message: report('Profile wurden erfolgreich aktualisiert'),
      };
    },

    // Answers with `documents` because the real command refreshes stored mtimes.
    create_documents: (args) => ({
      documents: copy(state.documents),
      message: report(
        'Dokumente wurden erfolgreich erstellt',
        ['Alle Dokumente wurden erfolgreich erstellt.'],
        `data/out/${String(args['exportFolder'])}`
      ),
    }),

    open_file: () => ({}),
    open_output_folder: () => ({}),

    // ─── trains ───────────────────────────────────────────────

    get_trains_data: () => trainsLists(),

    query_events: (args) => ({
      instandhaltungPage: {
        rows: copy(state.events),
        total: state.events.length,
        offset: Number(args['offset'] ?? 0),
      },
    }),

    stage_import: () => (state.staging ? { staging: copy(state.staging) } : {}),

    // The real command reads the file at `path` instead of asking the picker;
    // the fake serves the same seeded staging.
    stage_import_path: () =>
      state.staging ? { staging: copy(state.staging), counts: counts() } : {},

    // The plan comes back applied, so the app sees what it asked for — the real
    // command re-parses the held grid and answers with the result.
    restage_import: (args) => {
      if (!state.staging) return {};
      const plan = args['plan'] as unknown as FakeStaging['plan'];
      state.staging = { ...state.staging, plan: copy(plan) };
      return { staging: copy(state.staging) };
    },

    restage_sheet: (args) => {
      if (!state.staging) return {};
      state.staging = { ...state.staging, sheet: String(args['sheet']) };
      return { staging: copy(state.staging) };
    },

    discard_import: () => {
      state.staging = null;
      return { counts: counts() };
    },

    save_template: (args) => {
      const name = String(args['name'] ?? '').trim();
      if (!name || !state.staging) {
        return Promise.reject({
          messages: ['Die Vorlage braucht einen Namen.'],
        });
      }
      state.templates.push({
        id: `t${state.templates.length + 1}`,
        name,
        plan: copy(state.staging.plan),
        builtin: false,
        createdAt: '2026-10-03',
      });
      state.staging = null;
      return {
        templates: copy(state.templates),
        message: report(`Vorlage „${name}“ wurde gespeichert`),
      };
    },

    stage_document: (args) => {
      const id = String(args['id']);
      const dokument = state.dokumente.find((entry) => entry.id === id);
      if (!dokument) {
        return Promise.reject({
          messages: ['Das Dokument gibt es nicht mehr.'],
        });
      }
      if (dokument.importiertAm) {
        return Promise.reject({
          messages: [
            `„${dokument.name}“ wurde bereits am ${dokument.importiertAm} importiert.`,
          ],
        });
      }
      const staged = state.document ?? state.staging;
      if (!staged) return {};
      state.staging = { ...copy(staged), origin: { kind: 'dokument', id } };
      return { staging: copy(state.staging), counts: counts() };
    },

    commit_document: (args) => {
      const decisions = args['decisions'] as unknown as { rows: number[] };
      const origin = state.staging?.origin;
      const id = origin?.kind === 'dokument' ? origin.id : undefined;
      if (origin?.kind === 'master' && state.master.importRun) {
        const run = state.master.importRun;
        if (!run.done.includes(origin.sheet)) run.done.push(origin.sheet);
      }
      state.committed = [...decisions.rows];
      state.dokumente = state.dokumente.map((entry) =>
        entry.id === id ? { ...entry, importiertAm: '2026-10-03' } : entry
      );
      state.staging = null;
      return {
        ...trainsLists(),
        message: report('Import wurde erfolgreich übernommen', [
          `${decisions.rows.length} Zeile(n) übernommen.`,
        ]),
      };
    },

    save_waggon: () => ({ ...trainsLists() }),

    remove_waggon: (args) => {
      const id = String(args['id']);
      state.wagen = state.wagen.filter((entry) => entry.id !== id);
      return {
        ...trainsLists(),
        message: report('Wagen wurde entfernt'),
      };
    },

    save_wheelset: () => ({ ...trainsLists() }),

    remove_wheelset: (args) => {
      const id = String(args['id']);
      state.radsaetze = state.radsaetze.filter((entry) => entry.id !== id);
      state.einbauten = state.einbauten.filter(
        (entry) => entry.radsatzId !== id
      );
      return { ...trainsLists(), message: report('Radsatz wurde entfernt') };
    },

    save_partner: () => ({ ...trainsLists() }),

    remove_partner: (args) => {
      const id = String(args['id']);
      state.partners = state.partners.filter((entry) => entry.id !== id);
      return { ...trainsLists(), message: report('Partner wurde entfernt') };
    },

    save_trains_settings: (args) => {
      state.settings = copy(
        args['settings'] as unknown as { wagennummer: string }
      );
      return {
        settings: copy(state.settings),
        message: report('Einstellungen wurden gespeichert'),
      };
    },

    remove_template: (args) => {
      const id = String(args['id']);
      state.templates = state.templates.filter((entry) => entry.id !== id);
      return {
        templates: copy(state.templates),
        message: report('Vorlage wurde entfernt'),
      };
    },

    reset_trains: () => {
      state.wagen = [];
      state.partners = [];
      state.radsaetze = [];
      state.einbauten = [];
      state.events = [];
      state.templates = [];
      state.dokumente = [];
      state.staging = null;
      state.cleaning = null;
      return {
        ...trainsLists(),
        message: report('Zugdaten wurden zurückgesetzt'),
      };
    },

    pick_import_folder: () => scanned(),
    pick_import_files: () => scanned(),
    scan_import_paths: () => scanned(),

    clean_file: () => {
      if (!state.clean) return {};
      state.cleaning = copy(state.clean);
      return { cleaning: copy(state.cleaning) };
    },

    reclean_file: (args) => {
      const report = recleaned(
        args['decisions'] as unknown as FakeCleanDecisions
      );
      return report ? { cleaning: report } : {};
    },

    write_clean: () => {
      const cleaning = state.cleaning;
      if (!cleaning) return {};
      const id = `d${state.dokumente.length + 1}`;
      const name = cleaning.file.split(/[\\/]/).pop() ?? cleaning.file;
      state.dokumente.push({
        id,
        name,
        sheet: cleaning.sheet,
        templateId: cleaning.templateId,
        templateName: cleaning.templateName,
        plan: copy(cleaning.plan),
        originalHash: id,
        cleanedHash: id,
        folder: `data/trains/dokumente/${id}`,
        original: cleaning.file,
        cleaned: `data/trains/dokumente/${id}/bereinigt.xlsx`,
        summary: copy(cleaning.summary),
        bereinigtAm: '2026-10-03',
      });
      state.cleaning = null;
      return {
        dokumente: copy(state.dokumente),
        templates: copy(state.templates),
        counts: counts(),
        message: report(`„${name}“ wurde bereinigt`, [
          '3 Änderung(en) im Blatt „Änderungsprotokoll“ der bereinigten Datei protokolliert.',
        ]),
      };
    },

    discard_clean: () => {
      state.cleaning = null;
      return {};
    },

    get_master: () => masterView(),

    pick_master_file: () => {
      if (!state.masterPicker) return {};
      const fresh = state.master.file !== state.masterPicker;
      state.master = {
        ...state.master,
        file: state.masterPicker,
        bindings: fresh ? copy(state.masterDefaults) : state.master.bindings,
      };
      return masterView();
    },

    reset_master_bindings: () => {
      state.master = { ...state.master, bindings: copy(state.masterDefaults) };
      return masterView();
    },

    save_master: (args) => {
      const run = state.master.importRun;
      state.master = {
        ...copy(args['settings'] as unknown as FakeMasterSettings),
        importRun: run,
      };
      return masterView();
    },

    get_master_sheet: (args) => {
      const sheet = String(args['sheet']);
      const seeded = state.masterSheetViews[sheet];
      if (seeded) return { masterSheet: copy(seeded) };
      const headers =
        state.masterSheets.find((entry) => entry.name === sheet)?.headers ?? [];
      const bound = state.master.bindings.find(
        (entry) => entry.sheet === sheet
      );
      return {
        masterSheet: {
          sheet,
          kind: bound?.kind,
          rowLabel: '',
          columns: headers.map((header, index) => ({
            index: index + 1,
            header,
            filled: false,
          })),
          rows: [],
          problem: bound
            ? undefined
            : `Das Blatt „${sheet}“ ist in den Master-Einstellungen nicht zugeordnet.`,
        },
      };
    },

    start_master_import: () => {
      const sheets = state.master.bindings
        .filter((entry) => entry.kind)
        .map((entry) => entry.sheet);
      if (!state.master.file || sheets.length === 0) {
        return Promise.reject({
          messages: [
            'Es ist noch keinem Blatt der Master-Datei eine Art zugeordnet.',
          ],
        });
      }
      state.wagen = [];
      state.radsaetze = [];
      state.einbauten = [];
      state.events = [];
      state.staging = null;
      state.dokumente = state.dokumente.map((entry) => ({
        ...entry,
        importiertAm: undefined,
      }));
      state.master = {
        ...state.master,
        importRun: { startedAt: '2026-10-04', sheets, done: [] },
      };
      return { ...trainsLists(), ...masterView() };
    },

    stage_master_sheet: (args) => {
      const sheet = String(args['sheet']);
      const staged = state.masterStaging ?? state.document ?? state.staging;
      if (!staged) return {};
      state.staging = { ...copy(staged), origin: { kind: 'master', sheet } };
      return { staging: copy(state.staging), counts: counts() };
    },

    open_master_export: (args) => {
      const dokument = state.dokumente.find(
        (entry) => entry.id === String(args['id'])
      );
      if (!dokument) {
        return Promise.reject({
          messages: ['Das Dokument gibt es nicht mehr.'],
        });
      }
      const file = state.master.file;
      if (!file) {
        return Promise.reject({
          messages: ['Es ist noch keine Master-Datei gewählt.'],
        });
      }
      const bases = [{ path: file, name: baseName(file), copy: false }];
      const last = state.master.lastExport;
      if (last) bases.push({ path: last, name: baseName(last), copy: true });
      return {
        masterExportStart: {
          dokumentId: dokument.id,
          dokument: dokument.name,
          template: dokument.templateName,
          bases,
          base: last ?? file,
          sheets: state.masterSheets
            .map((sheet) => {
              const bound = state.master.bindings.find(
                (entry) => entry.sheet === sheet.name
              );
              const overview = bound?.kind === 'wagenliste';
              const remembered = bound?.templateId === dokument.templateId;
              return {
                sheet: sheet.name,
                kind: bound?.kind,
                mode: bound?.mode ?? 'snapshot',
                key: bound?.key,
                aliases: copy(bound?.aliases ?? []),
                ignored: copy(bound?.ignored ?? []),
                matched: state.masterExport[sheet.name]?.matched.length ?? 0,
                suggested: remembered,
                reason: remembered
                  ? `zugeordnet zur Vorlage „${dokument.templateName}“`
                  : undefined,
                warning:
                  overview && !bound?.templateId
                    ? 'Als Übersicht zugeordnet: Formeln und Notizen des Kunden können beim Schreiben ersetzt werden.'
                    : undefined,
              };
            })
            .sort((a, b) => Number(b.suggested) - Number(a.suggested)),
        },
      };
    },

    preview_master_export: (args) => ({
      masterExport: exportRun(args['request'] as unknown as FakeExportRequest),
    }),

    write_master_export: (args) => {
      const run = exportRun(args['request'] as unknown as FakeExportRequest);
      const file = state.master.file ?? '';
      const folder = file.replace(/[\\/][^\\/]*$/, '');
      const target = `${folder}/Master 2026-10-04.xlsx`;
      state.master = { ...state.master, lastExport: target };
      return {
        masterExport: { ...run, target, folder },
        ...masterView(),
      };
    },

    get_master_file: () => ({ masterFile: copy(state.masterFile) }),

    clean_master_file: () => {
      if (!state.masterFilePick) return {};
      state.masterFile = {
        ...state.masterFile,
        pending: copy(state.masterFilePick),
      };
      return { masterFile: copy(state.masterFile) };
    },

    accept_master_file: () => {
      const pending = state.masterFile.pending;
      if (!pending) {
        return Promise.reject({
          messages: [
            'Es liegt keine bereinigte Master-Datei zur Übernahme vor.',
          ],
        });
      }
      state.masterFile = {
        versions: [
          { ...pending, uebernommenAm: '2026-10-04' },
          ...state.masterFile.versions,
        ],
      };
      return { masterFile: copy(state.masterFile) };
    },

    discard_master_file: () => {
      state.masterFile = { versions: state.masterFile.versions };
      return { masterFile: copy(state.masterFile) };
    },

    create_trains_export: () => ({
      message: report(
        'Export wurde erfolgreich erstellt',
        ['Datei wurde erstellt: erp-import.xlsx'],
        'data/out/trains-2026-08-16'
      ),
    }),
  };

  (globalThis as unknown as { isTauri: boolean }).isTauri = true;
  (window as unknown as { __npFake: State }).__npFake = state;
  (
    window as unknown as {
      __TAURI_INTERNALS__: {
        invoke: (
          command: string,
          args?: Record<string, unknown>
        ) => Promise<unknown>;
      };
    }
  ).__TAURI_INTERNALS__ = {
    invoke: (command, args = {}) => {
      state.calls.push({ command, args: copy(args) });
      const failure = state.failures[command];
      // Rejects with the SERIALISED `AppError`, a plain object — not an Error.
      if (failure) return Promise.reject({ messages: failure });
      const handler = commands[command];
      if (!handler) {
        return Promise.reject({ messages: [`Unbekannter Befehl: ${command}`] });
      }
      return Promise.resolve(handler(args as Record<string, never>));
    },
  };
}
