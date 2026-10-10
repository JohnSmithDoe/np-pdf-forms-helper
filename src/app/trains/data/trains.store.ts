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
// `masterImportRun` is the one field read on ABSENCE too. Rust sends it with
// every whole-list answer while a run is open and drops it once the last sheet
// is committed, so a list answer without it means "closed", not "unchanged" —
// presence alone would leave the banner standing after the final sheet.
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
  Dokument,
  ImportTemplate,
  Instandhaltung,
  Markierungen,
  Einbau,
  MasterImportRun,
  MasterView,
  Partner,
  StagedImport,
  TrainsCounts,
  TrainsData,
  TrainsSettings,
  Wagen,
  WagenZustand,
  Radsatz,
} from '../model/trains.types';
import type { MasterFile } from '../model/master-file';

type TrainsState = {
  wagen: Wagen[] | undefined;
  partners: Partner[] | undefined;
  templates: ImportTemplate[] | undefined;
  radsaetze: Radsatz[] | undefined;
  einbauten: Einbau[] | undefined;
  zustand: WagenZustand;
  markierungen: Markierungen;
  events: Instandhaltung[] | undefined;
  eventTotal: number;
  dokumente: Dokument[] | undefined;
  archiv: Dokument[] | undefined;
  settings: TrainsSettings;
  master: MasterView | undefined;
  masterImportRun: MasterImportRun | undefined;
  masterFile: MasterFile | undefined;
  counts: TrainsCounts;
  staging: StagedImport | undefined;
};

const initial: TrainsState = {
  wagen: undefined,
  partners: undefined,
  templates: undefined,
  radsaetze: undefined,
  einbauten: undefined,
  zustand: {
    geraete: [],
    meldungen: [],
    schaeden: [],
    auftraege: [],
    pruefungen: [],
  },
  markierungen: {
    hand: { wagen: {}, radsaetze: {} },
    master: { wagen: {}, radsaetze: {} },
  },
  events: undefined,
  eventTotal: 0,
  dokumente: undefined,
  archiv: undefined,
  settings: { wagennummer: 'compact' },
  master: undefined,
  masterImportRun: undefined,
  masterFile: undefined,
  counts: {
    wagen: 0,
    partners: 0,
    events: 0,
    radsaetze: 0,
    dokumente: 0,
    archiviert: 0,
  },
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
    dokumentById: computed(
      () =>
        new Map(
          (store.dokumente() ?? []).map((dokument) => [dokument.id, dokument])
        )
    ),
    wagenById: computed(
      () => new Map((store.wagen() ?? []).map((wagen) => [wagen.id, wagen]))
    ),
  })),
  withMethods((store) => ({
    applyTrainsData(data: TrainsData): void {
      const next: Partial<TrainsState> = {};
      if (data.wagen) {
        next.wagen = data.wagen;
        next.masterImportRun = data.masterImportRun;
      }
      if (data.partners) next.partners = data.partners;
      if (data.templates) next.templates = data.templates;
      if (data.radsaetze) next.radsaetze = data.radsaetze;
      if (data.einbauten) next.einbauten = data.einbauten;
      if (data.zustand) next.zustand = data.zustand;
      if (data.markierungen) next.markierungen = data.markierungen;
      if (data.dokumente) next.dokumente = data.dokumente;
      if (data.archiv) next.archiv = data.archiv;
      if (data.settings) next.settings = data.settings;
      if (data.master) next.master = data.master;
      if (data.masterFile) next.masterFile = data.masterFile;
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
