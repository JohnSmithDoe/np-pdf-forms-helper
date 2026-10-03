// ─── why ────────────────────────────────────────────────────────
// The import walk's own state: which document is being walked, which are
// queued behind it, one decision per entity GROUP, the rows the user dropped or
// took back, and the commit's report.
//
// Nothing here survives leaving the walk — a deliberate fresh start. The
// decisions are cheap to give again and a half-answered walk restored days
// later would be answering a store that has moved on since.
//
// A decision is keyed by kind AND group key: the key spaces are separate on the
// backend, and one map for all three must not let a Wagen key answer a Partner.
// What is not in `decisions` is not undecided by accident — the facade computes
// the default (a `known` group is its match, everything else asks).
// ────────────────────────────────────────────────────────────────

import { patchState, signalStore, withMethods, withState } from '@ngrx/signals';
import type { ClientReport } from '../../@shared/model/client.types';
import type { EntityDecision } from '../model/trains.types';

type ImportWalkState = {
  dokumentId: string | undefined;
  queue: string[];
  decisions: Record<string, EntityDecision | undefined>;
  included: Record<number, boolean | undefined>;
  report: ClientReport | null;
};

const initial: ImportWalkState = {
  dokumentId: undefined,
  queue: [],
  decisions: {},
  included: {},
  report: null,
};

export const ImportWalkStore = signalStore(
  { providedIn: 'root' },
  withState(initial),
  withMethods((store) => ({
    begin(dokumentId: string, queue: string[]): void {
      patchState(store, { ...initial, dokumentId, queue });
    },

    decide(slot: string, decision: EntityDecision): void {
      patchState(store, {
        decisions: { ...store.decisions(), [slot]: decision },
      });
    },

    decideMany(decisions: Record<string, EntityDecision>): void {
      patchState(store, {
        decisions: { ...store.decisions(), ...decisions },
      });
    },

    include(row: number, include: boolean): void {
      patchState(store, { included: { ...store.included(), [row]: include } });
    },

    includeMany(rows: number[], include: boolean): void {
      const included = { ...store.included() };
      for (const row of rows) included[row] = include;
      patchState(store, { included });
    },

    finish(report: ClientReport): void {
      patchState(store, { report });
    },

    reset(): void {
      patchState(store, initial);
    },
  }))
);
