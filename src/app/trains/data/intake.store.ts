// ─── why ────────────────────────────────────────────────────────
// The guided import's own state: the scanned files, what the user picked for
// each, how far the walk has got, the cleaning under review and the two reports
// the result step renders.
//
// The walk lives HERE and not in the backend because the backend holds exactly
// one cleaning and one staging slot. A folder of five files is therefore five
// passes through the same two slots, and the list that says which file comes
// next has to outlive every one of them.
//
// A pick is keyed by PATH, not by position: a rescan replaces the list, and a
// pick keyed by index would silently move to whatever file now sits there.
//
// The default pick is only ever the UNAMBIGUOUS one. `erkannt` means exactly one
// template matched, so it is preselected; `mehrdeutig` stays empty and the row
// cannot start until the user chooses — preselecting the first of two matches
// would be the auto-resolution the whole import is built to refuse. `unbekannt`
// defaults to the manual import, because that is the only way it can go.
// „Nicht importieren“ (`skip`) is a PICK, not an outcome: it answers the row's
// question, so a tie the user does not want resolves by skipping, and it stays
// changeable until the walk reaches the row.
//
// `decisions` are kept whole because the backend keeps none: `reclean_file` and
// `write_clean` both take the full set, and a decision held only in a component
// would be lost on the first reclean that re-renders it.
//
// `cleanedReport` and `commitReport` are two slices for the reason filler keeps
// `setupReport` and `runReport` apart: each is parked by a `silent` call and is
// the PAGE of the result step, and a toast saying the same would cover it.
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
  | { kind: 'manual' }
  | { kind: 'skip' };

export type FileOutcome =
  'importiert' | 'manuell' | 'verworfen' | 'fehlgeschlagen';

type IntakeState = {
  scan: ScanFile[] | undefined;
  picks: Record<string, FilePick | undefined>;
  outcomes: Record<string, FileOutcome | undefined>;
  current: string | undefined;
  cleaning: CleanReport | undefined;
  decisions: CleanDecisions;
  cleanedReport: ClientReport | null;
  cleanedFile: string | undefined;
  commitReport: ClientReport | null;
};

const NO_DECISIONS: CleanDecisions = { corrections: [], confirmations: [] };

const initial: IntakeState = {
  scan: undefined,
  picks: {},
  outcomes: {},
  current: undefined,
  cleaning: undefined,
  decisions: NO_DECISIONS,
  cleanedReport: null,
  cleanedFile: undefined,
  commitReport: null,
};

export function defaultPick(file: ScanFile): FilePick | undefined {
  if (file.status === 'unbekannt') return { kind: 'manual' };
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

    setPick(path: string, pick: FilePick | undefined): void {
      patchState(store, {
        picks: { ...store.picks(), [path]: pick },
        outcomes: { ...store.outcomes(), [path]: undefined },
      });
    },

    setOutcome(path: string, outcome: FileOutcome): void {
      patchState(store, { outcomes: { ...store.outcomes(), [path]: outcome } });
    },

    begin(path: string, cleaning: CleanReport | undefined): void {
      patchState(store, {
        current: path,
        cleaning,
        decisions: NO_DECISIONS,
        cleanedReport: null,
        cleanedFile: undefined,
        commitReport: null,
      });
    },

    setCleaning(cleaning: CleanReport | undefined): void {
      patchState(store, { cleaning });
    },

    setDecisions(decisions: CleanDecisions): void {
      patchState(store, { decisions });
    },

    setCleaned(
      cleanedReport: ClientReport | null,
      cleanedFile: string | undefined
    ): void {
      patchState(store, { cleanedReport, cleanedFile });
    },

    setCommitReport(commitReport: ClientReport | null): void {
      patchState(store, { commitReport });
    },

    endCurrent(): void {
      patchState(store, {
        current: undefined,
        cleaning: undefined,
        decisions: NO_DECISIONS,
        cleanedReport: null,
        cleanedFile: undefined,
        commitReport: null,
      });
    },

    reset(): void {
      patchState(store, initial);
    },
  }))
);
