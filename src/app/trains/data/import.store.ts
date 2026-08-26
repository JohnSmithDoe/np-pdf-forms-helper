// ─── why ────────────────────────────────────────────────────────
// The wizard's own state: which stage the user is on, and the decision they have
// made about each staged row. The staged rows themselves are NOT here — the
// backend holds them, because re-reading a workbook for every nudge of a header
// row would re-parse a thousand rows for one click.
//
// A decision is keyed by row number and defaults are computed rather than
// stored: a `Ready` row needs nothing, a `Duplicate` starts UNTICKED because it
// should not be committed twice, and anything needing input starts unticked
// because nothing may be created without the user saying so.
// ────────────────────────────────────────────────────────────────

import { patchState, signalStore, withMethods, withState } from '@ngrx/signals';
import type { EntityDecision, RowStatus } from '../model/trains.types';

export type Stage = 'source' | 'mapping' | 'review' | 'commit';

export interface RowChoice {
  include: boolean;
  wagen: EntityDecision;
  werkstatt: EntityDecision;
  halter: EntityDecision;
  eigentuemer: EntityDecision;
  radsatz: EntityDecision;
}

type ImportState = {
  stage: Stage;
  choices: Record<number, RowChoice>;
  statusFilter: RowStatus | undefined;
  templateName: string;
};

const initial: ImportState = {
  stage: 'source',
  choices: {},
  statusFilter: undefined,
  templateName: '',
};

export function defaultChoice(status: RowStatus): RowChoice {
  return {
    include: status === 'ready',
    wagen: { action: 'create' },
    werkstatt: { action: 'create' },
    halter: { action: 'create' },
    eigentuemer: { action: 'skip' },
    radsatz: { action: 'create' },
  };
}

export const ImportStore = signalStore(
  { providedIn: 'root' },
  withState(initial),
  withMethods((store) => ({
    setStage(stage: Stage): void {
      patchState(store, { stage });
    },

    setFilter(statusFilter: RowStatus | undefined): void {
      patchState(store, { statusFilter });
    },

    setTemplateName(templateName: string): void {
      patchState(store, { templateName });
    },

    seedChoices(rows: { row: number; status: RowStatus }[]): void {
      patchState(store, {
        choices: Object.fromEntries(
          rows.map((row) => [row.row, defaultChoice(row.status)])
        ),
      });
    },

    setChoice(row: number, patch: Partial<RowChoice>): void {
      const current = store.choices()[row] ?? defaultChoice('ready');
      patchState(store, {
        choices: { ...store.choices(), [row]: { ...current, ...patch } },
      });
    },

    setAllIncluded(rows: number[], include: boolean): void {
      const choices = { ...store.choices() };
      for (const row of rows) {
        choices[row] = { ...(choices[row] ?? defaultChoice('ready')), include };
      }
      patchState(store, { choices });
    },

    reset(): void {
      patchState(store, initial);
    },
  }))
);
