// ─── why ────────────────────────────────────────────────────────
// The template mapper's own state: which stage the user is on. The staged file
// itself is NOT here — the backend holds it, because re-reading a workbook for
// every nudge of a header row would re-parse a thousand rows for one click.
//
// There are no row choices any more. The mapper's only exit is a template, and
// a template is about columns, not about which rows to take; rows are decided
// on the import walk, per entity, after the file has been cleaned.
// ────────────────────────────────────────────────────────────────

import { patchState, signalStore, withMethods, withState } from '@ngrx/signals';

export type Stage = 'source' | 'mapping';

type ImportState = {
  stage: Stage;
};

const initial: ImportState = {
  stage: 'source',
};

export const ImportStore = signalStore(
  { providedIn: 'root' },
  withState(initial),
  withMethods((store) => ({
    setStage(stage: Stage): void {
      patchState(store, { stage });
    },

    reset(): void {
      patchState(store, initial);
    },
  }))
);
