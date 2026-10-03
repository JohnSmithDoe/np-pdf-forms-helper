// ─── why ────────────────────────────────────────────────────────
// The cleaning batch's own state: the scanned files, what the user picked for
// each, how far the walk has got, the cleaning under review, and what became of
// every file — which is what the batch summary renders.
//
// The walk lives HERE and not in the backend because the backend holds exactly
// one cleaning slot. A folder of five files is therefore five passes through
// that slot, and the list that says which file comes next has to outlive every
// one of them.
//
// A pick is keyed by PATH, not by position: a rescan replaces entries, and a
// pick keyed by index would silently move to whatever file now sits there.
//
// The default pick is only ever the UNAMBIGUOUS one. `erkannt` means exactly one
// template matched, so it is preselected; `mehrdeutig` stays empty and the row
// cannot start until the user chooses — preselecting the first of two matches
// would be the auto-resolution the whole import is built to refuse. `unbekannt`
// defaults to creating a template, because that is the only way it can go.
// „Nicht bereinigen“ (`skip`) is a PICK, not an outcome: it answers the row's
// question, and it stays changeable until the walk reaches the row.
//
// `merge` replaces only the files it is given. A file handed to the template
// mapper comes back RESCANNED — now recognised by the template just saved —
// and a full `applyScan` would wipe what the batch already cleaned.
//
// `decisions` are kept whole because the backend keeps none: `reclean_file` and
// `write_clean` both take the full set, and a decision held only in a component
// would be lost on the first reclean that re-renders it.
// ────────────────────────────────────────────────────────────────

import { patchState, signalStore, withMethods, withState } from '@ngrx/signals';
import type { ClientReport } from '../../@shared/model/client.types';
import type {
  CleanDecisions,
  CleanReport,
  ScanFile,
} from '../model/trains.types';

export type FilePick =
  | { kind: 'template'; templateId: string; sheet: string }
  | { kind: 'create' }
  | { kind: 'skip' };

export type FileOutcome = 'bereinigt' | 'verworfen' | 'fehlgeschlagen';

export interface FileResult {
  outcome: FileOutcome;
  report?: ClientReport;
  dokumentId?: string;
}

type IntakeState = {
  scan: ScanFile[] | undefined;
  picks: Record<string, FilePick | undefined>;
  results: Record<string, FileResult | undefined>;
  current: string | undefined;
  handedOver: string | undefined;
  cleaning: CleanReport | undefined;
  decisions: CleanDecisions;
};

const NO_DECISIONS: CleanDecisions = { corrections: [], confirmations: [] };

const initial: IntakeState = {
  scan: undefined,
  picks: {},
  results: {},
  current: undefined,
  handedOver: undefined,
  cleaning: undefined,
  decisions: NO_DECISIONS,
};

export function defaultPick(file: ScanFile): FilePick | undefined {
  if (file.status === 'unbekannt') return { kind: 'create' };
  if (file.status !== 'erkannt' || file.matches.length !== 1) return undefined;
  const match = file.matches[0];
  if (!match) return undefined;
  return { kind: 'template', templateId: match.templateId, sheet: match.sheet };
}

export const IntakeStore = signalStore(
  { providedIn: 'root' },
  withState(initial),
  withMethods((store) => ({
    applyScan(scan: ScanFile[]): void {
      patchState(store, {
        ...initial,
        scan,
        picks: Object.fromEntries(
          scan.map((file) => [file.path, defaultPick(file)])
        ),
      });
    },

    merge(files: ScanFile[]): void {
      const fresh = new Map(files.map((file) => [file.path, file]));
      const picks = { ...store.picks() };
      const results = { ...store.results() };
      for (const file of files) {
        picks[file.path] = defaultPick(file);
        results[file.path] = undefined;
      }
      patchState(store, {
        scan: (store.scan() ?? []).map((file) => fresh.get(file.path) ?? file),
        picks,
        results,
      });
    },

    setPick(path: string, pick: FilePick | undefined): void {
      patchState(store, {
        picks: { ...store.picks(), [path]: pick },
        results: { ...store.results(), [path]: undefined },
      });
    },

    setResult(path: string, result: FileResult): void {
      patchState(store, { results: { ...store.results(), [path]: result } });
    },

    handOver(path: string | undefined): void {
      patchState(store, { handedOver: path });
    },

    begin(path: string, cleaning: CleanReport | undefined): void {
      patchState(store, { current: path, cleaning, decisions: NO_DECISIONS });
    },

    setCleaning(cleaning: CleanReport | undefined): void {
      patchState(store, { cleaning });
    },

    setDecisions(decisions: CleanDecisions): void {
      patchState(store, { decisions });
    },

    endCurrent(): void {
      patchState(store, {
        current: undefined,
        cleaning: undefined,
        decisions: NO_DECISIONS,
      });
    },

    reset(): void {
      patchState(store, initial);
    },
  }))
);
