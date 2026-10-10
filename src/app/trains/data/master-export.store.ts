// ─── why ────────────────────────────────────────────────────────
// The master export wizard's own state: what the backend offered, which sheet
// is ticked (the template's, fixed at `begin`), the per-sheet plan (key,
// aliases, ignored columns — the template's, never edited here), the base
// file (the current master version, sent back so a newer one is refused), and the two runs — the dry run being looked at and the written one.
//
// Like the import walk, nothing survives leaving the wizard: the answers are
// cheap to give again, and the ones worth keeping are remembered on the master
// bindings by the backend, not here.
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
          append: sheet.append,
          remove: sheet.remove,
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
          append: sheet.append,
          remove: sheet.remove,
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
