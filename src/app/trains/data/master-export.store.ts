// ─── why ────────────────────────────────────────────────────────
// The master export wizard's own state: what the backend offered, which sheets
// are ticked, the per-sheet answers (key, aliases, ignored columns), the base
// file, and the two runs — the dry run being looked at and the written one.
//
// Like the import walk, nothing survives leaving the wizard: the answers are
// cheap to give again, and the ones worth keeping are remembered on the master
// bindings by the backend, not here.
//
// `choices` holds an answer for EVERY offered sheet, ticked or not, so unticking
// and re-ticking a sheet does not lose what was answered for it.
// ────────────────────────────────────────────────────────────────

import { patchState, signalStore, withMethods, withState } from '@ngrx/signals';
import type {
  MasterExportChoice,
  MasterExportRun,
  MasterExportStart,
} from '../model/trains.types';

type MasterExportState = {
  start: MasterExportStart | undefined;
  base: string;
  ticked: Record<string, boolean | undefined>;
  choices: Record<string, MasterExportChoice | undefined>;
  remember: boolean;
  preview: MasterExportRun | undefined;
  result: MasterExportRun | undefined;
};

const initial: MasterExportState = {
  start: undefined,
  base: '',
  ticked: {},
  choices: {},
  remember: true,
  preview: undefined,
  result: undefined,
};

export const MasterExportStore = signalStore(
  { providedIn: 'root' },
  withState(initial),
  withMethods((store) => ({
    begin(start: MasterExportStart): void {
      const ticked: Record<string, boolean> = {};
      const choices: Record<string, MasterExportChoice> = {};
      for (const sheet of start.sheets) {
        ticked[sheet.sheet] = sheet.suggested;
        choices[sheet.sheet] = {
          sheet: sheet.sheet,
          key: sheet.key,
          aliases: sheet.aliases,
          ignored: sheet.ignored,
        };
      }
      patchState(store, {
        ...initial,
        start,
        base: start.base,
        ticked,
        choices,
      });
    },

    tick(sheet: string, on: boolean): void {
      patchState(store, {
        ticked: { ...store.ticked(), [sheet]: on },
        preview: undefined,
      });
    },

    setBase(base: string): void {
      patchState(store, { base, preview: undefined });
    },

    setRemember(remember: boolean): void {
      patchState(store, { remember });
    },

    choose(choice: MasterExportChoice): void {
      patchState(store, {
        choices: { ...store.choices(), [choice.sheet]: choice },
      });
    },

    previewed(run: MasterExportRun): void {
      const choices = { ...store.choices() };
      for (const sheet of run.sheets) {
        choices[sheet.sheet] = {
          sheet: sheet.sheet,
          key: sheet.key,
          aliases: sheet.aliases,
          ignored: sheet.ignored,
        };
      }
      patchState(store, { preview: run, choices });
    },

    finish(result: MasterExportRun): void {
      patchState(store, { result });
    },

    reset(): void {
      patchState(store, initial);
    },
  }))
);
