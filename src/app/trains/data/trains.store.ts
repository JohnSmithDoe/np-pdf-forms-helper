// ─── why ────────────────────────────────────────────────────────
// The master entities and the in-flight import, in one signalStore.
//
// `applyTrainsData` acts on PRESENCE, not on length: a list that is there is the
// complete current one and replaces what is held, and one that is absent means
// the command could not have changed it. A single `[]` cannot say which — "there
// really are none now" after a reset and "unchanged" after an export look
// identical — so `Option` on the wire is what removes the guess.
//
// Events are the exception and arrive as a PAGE, so they APPEND rather than
// replace when the offset is past the start. Replacing on every page would make
// infinite scroll show only the last page it fetched.
//
// One `patchState` per response, not one per present list: each call notifies
// every downstream computed, so six of them make the four list facades recompute
// six times for one command.
//
// `loaded` reads `wagen` because that list is the one every command that could
// have loaded anything sends back. It is what lets the route resolver do the
// entry load once instead of five page constructors doing it each.
// ────────────────────────────────────────────────────────────────

import { computed } from '@angular/core';
import {
  patchState,
  signalStore,
  withComputed,
  withMethods,
  withState,
} from '@ngrx/signals';
import type {
  ImportTemplate,
  Instandhaltung,
  Einbau,
  Partner,
  StagedImport,
  TrainsCounts,
  TrainsData,
  Wagen,
  Radsatz,
} from '../model/trains.types';

type TrainsState = {
  wagen: Wagen[] | undefined;
  partners: Partner[] | undefined;
  templates: ImportTemplate[] | undefined;
  radsaetze: Radsatz[] | undefined;
  einbauten: Einbau[] | undefined;
  events: Instandhaltung[] | undefined;
  eventTotal: number;
  counts: TrainsCounts;
  staging: StagedImport | undefined;
};

const initial: TrainsState = {
  wagen: undefined,
  partners: undefined,
  templates: undefined,
  radsaetze: undefined,
  einbauten: undefined,
  events: undefined,
  eventTotal: 0,
  counts: { wagen: 0, partners: 0, events: 0, radsaetze: 0 },
  staging: undefined,
};

export const TrainsStore = signalStore(
  { providedIn: 'root' },
  withState(initial),
  withComputed((store) => ({
    loaded: computed(() => store.wagen() !== undefined),
    partnerById: computed(
      () =>
        new Map(
          (store.partners() ?? []).map((partner) => [partner.id, partner])
        )
    ),
    radsatzById: computed(
      () =>
        new Map(
          (store.radsaetze() ?? []).map((radsatz) => [radsatz.id, radsatz])
        )
    ),
    wagenById: computed(
      () => new Map((store.wagen() ?? []).map((wagen) => [wagen.id, wagen]))
    ),
  })),
  withMethods((store) => ({
    applyTrainsData(data: TrainsData): void {
      const next: Partial<TrainsState> = {};
      if (data.wagen) next.wagen = data.wagen;
      if (data.partners) next.partners = data.partners;
      if (data.templates) next.templates = data.templates;
      if (data.radsaetze) next.radsaetze = data.radsaetze;
      if (data.einbauten) next.einbauten = data.einbauten;
      if (data.counts) next.counts = data.counts;
      if (data.staging) next.staging = data.staging;
      if (data.instandhaltungPage) {
        const page = data.instandhaltungPage;
        next.events =
          page.offset > 0
            ? [...(store.events() ?? []), ...page.rows]
            : page.rows;
        next.eventTotal = page.total;
      }
      patchState(store, next);
    },

    clearStaging(): void {
      patchState(store, { staging: undefined });
    },

    clearEvents(): void {
      patchState(store, { events: undefined, eventTotal: 0 });
    },
  }))
);
