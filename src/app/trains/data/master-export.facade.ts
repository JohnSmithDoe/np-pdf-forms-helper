// ─── why ────────────────────────────────────────────────────────
// The master export wizard's API: one filed Dokument into the sheet of the
// master its template is bound to — Blatt, Vorschau, Ergebnis — written into
// the customer's own file. Rust decides everything about the sheets (which one
// belongs to the template, how columns pair, which cells change); this only
// holds the two toggles and sends the plan WHOLE, like the import walk sends
// its plan.
//
// The column mapping comes from the template and is not asked: the binding's
// key, aliases and ignored columns go out as Rust offered them, so step one
// can only SHOW the key pair and what is wrong. That is why the dry run starts
// with `begin` and not on Weiter — the first step already needs its result —
// and every toggle re-runs it, because append and remove change what the
// paste does. The dry run's effective answers are adopted back (`previewed`):
// a remembered alias the sheet no longer fits comes back dropped.
//
// A document column the sheet has no column for is not transferred and is
// named, never blocking: with no mapping step there is nothing to answer it
// with. Only a sheet that cannot be written at all (`problem`) stops Weiter.
// ────────────────────────────────────────────────────────────────

import { computed, inject, Injectable } from '@angular/core';
import { BackendService } from '../../@shared/data/backend/backend.service';
import type {
  MasterExportChoice,
  MasterExportRequest,
  MasterExportSheet,
  MasterExportSheetRun,
} from '../model/trains.types';
import { MasterExportStore } from './master-export.store';
import { TrainsBackend } from './trains.backend';
import { TrainsStore } from './trains.store';

export interface ExportSheetView {
  sheet: MasterExportSheet;
  ticked: boolean;
  append: boolean;
  remove: boolean;
  run: MasterExportSheetRun | undefined;
  keySource: string | undefined;
}

@Injectable({ providedIn: 'root' })
export class MasterExportFacade {
  readonly #backend = inject(TrainsBackend);
  readonly #transport = inject(BackendService);
  readonly #trains = inject(TrainsStore);
  readonly #store = inject(MasterExportStore);

  readonly busy = this.#transport.busy;
  readonly loaded = this.#trains.loaded;
  readonly start = this.#store.start;
  readonly base = this.#store.base;
  readonly remember = this.#store.remember;
  readonly preview = this.#store.preview;
  readonly result = this.#store.result;

  readonly sheets = computed<ExportSheetView[]>(() => {
    const ticked = this.#store.ticked();
    const choices = this.#store.choices();
    const runs = this.preview()?.sheets ?? [];
    return (this.start()?.sheets ?? []).map((sheet) => {
      const run = runs.find((each) => each.sheet === sheet.sheet);
      return {
        sheet,
        ticked: ticked[sheet.sheet] ?? false,
        append: choices[sheet.sheet]?.append ?? sheet.append,
        remove: choices[sheet.sheet]?.remove ?? sheet.remove,
        run,
        keySource: run?.pairs.find((pair) => pair.master === run.key)?.source,
      };
    });
  });

  readonly tickedCount = computed(
    () => this.sheets().filter((view) => view.ticked).length
  );

  readonly request = computed<MasterExportRequest | undefined>(() => {
    const start = this.start();
    if (!start) return;
    const choices = this.#store.choices();
    return {
      dokumentId: start.dokumentId,
      base: this.base(),
      remember: this.remember(),
      sheets: this.sheets()
        .filter((view) => view.ticked)
        .map(
          (view) =>
            choices[view.sheet.sheet] ?? {
              sheet: view.sheet.sheet,
              aliases: [],
              ignored: [],
              append: view.sheet.append,
              remove: view.sheet.remove,
            }
        ),
    };
  });

  readonly writable = computed(() =>
    (this.preview()?.sheets ?? []).some((sheet) => !sheet.problem)
  );

  async begin(dokumentId: string): Promise<void> {
    const data = await this.#backend.openMasterExport(dokumentId);
    if (!data.masterExportStart) return;
    this.#store.begin(data.masterExportStart);
    if (this.tickedCount() > 0) await this.run();
  }

  async setAppend(sheet: string, append: boolean): Promise<void> {
    this.#store.choose({ ...this.#choice(sheet), append });
    await this.run();
  }

  async setRemove(sheet: string, remove: boolean): Promise<void> {
    this.#store.choose({ ...this.#choice(sheet), remove });
    await this.run();
  }

  setRemember(remember: boolean): void {
    this.#store.setRemember(remember);
  }

  async run(): Promise<void> {
    const request = this.request();
    if (!request) return;
    const data = await this.#backend.previewMasterExport(request);
    if (data.masterExport) this.#store.previewed(data.masterExport);
  }

  async write(): Promise<void> {
    const request = this.request();
    if (!request) return;
    const data = await this.#backend.writeMasterExport(request);
    this.#trains.applyTrainsData(data);
    if (data.masterExport) this.#store.finish(data.masterExport);
  }

  finish(): void {
    this.#store.reset();
  }

  async openFile(path: string): Promise<void> {
    await this.#backend.openFile(path);
  }

  async openFolder(folder: string): Promise<void> {
    await this.#backend.openFolder(folder);
  }

  #choice(sheet: string): MasterExportChoice {
    return (
      this.#store.choices()[sheet] ?? {
        sheet,
        aliases: [],
        ignored: [],
        append: false,
        remove: false,
      }
    );
  }
}
