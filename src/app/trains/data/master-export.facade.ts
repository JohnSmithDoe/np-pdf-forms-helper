// ─── why ────────────────────────────────────────────────────────
// The master export wizard's API: one filed Dokument into the sheets of the
// master the user ticks — Blätter, Abgleich, Vorschau, Ergebnis — written as a
// new version of the client master. Rust decides everything about the sheets (which to suggest,
// what a column conflict is, which cells change); this only holds the answers
// and sends them WHOLE, like the import walk sends its plan.
//
// Every answer re-runs the DRY RUN, because an alias or a key changes what the
// paste does, and a preview of the previous answer would show the wrong cells.
// The dry run's effective answers are adopted back (`previewed`): a remembered
// alias the sheet no longer fits comes back dropped, and the selects must show
// what will actually be written.
//
// The update is incremental; `append` per sheet says whether a key the sheet
// lacks becomes a new row — on, by Rust's default, only for the sheet the
// document's template belongs to. `remove` says whether a sheet row whose key
// the document lacks is emptied in place — offered on every ticked sheet, because only
// the user knows which documents are complete, and on where Rust remembered it
// for this template. On such a sheet a document column with
// nowhere to go blocks Weiter until answered — an alias onto a hand-kept
// column, or „nicht übertragen“; a sheet that is only updated asks nothing. That is the conflict rule of
// this wizard: a value difference is the update, a structure difference is a
// question (docs/decisions.md, „Export in die Master-Datei“).
//
// Beyond the questions, every pair of a sheet can be set by hand (`pair`):
// document column → master column, one place per document column, stored as
// an alias. Removing a pair (`unpair`) means „nicht übertragen“, so a column of
// the same name in the sheet does not quietly take it back. The key is set the
// same way, as a pair (`setIdentifier`): the document's identifier column and
// the sheet's, paired and made the key in one dry run — `RadsatzID` →
// `Radsatz ID` on RSmonitoring, where no column of the same name exists.
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
}

export interface ColumnAnswer {
  column: string;
  target: string | undefined;
  ignored: boolean;
  options: string[];
}

export interface PairView {
  source: string;
  master: string;
  sources: string[];
  masters: string[];
}

export interface StructureView {
  run: MasterExportSheetRun;
  keySource: string | undefined;
  keyMasters: string[];
  answers: ColumnAnswer[];
  pairs: PairView[];
  unpaired: string[];
  untransferred: string[];
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
    return (this.start()?.sheets ?? []).map((sheet) => ({
      sheet,
      ticked: ticked[sheet.sheet] ?? false,
      append: choices[sheet.sheet]?.append ?? sheet.append,
      remove: choices[sheet.sheet]?.remove ?? sheet.remove,
    }));
  });

  readonly tickedCount = computed(
    () => this.sheets().filter((view) => view.ticked).length
  );

  readonly request = computed<MasterExportRequest | undefined>(() => {
    const start = this.start();
    if (!start) return undefined;
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

  readonly structure = computed<StructureView[]>(() =>
    (this.preview()?.sheets ?? []).map((run) => {
      const paired = new Set(run.pairs.map((pair) => pair.source));
      const unpaired = run.sources.filter((source) => !paired.has(source));
      return {
        run,
        keySource: run.pairs.find((pair) => pair.master === run.key)?.source,
        keyMasters: [
          ...new Set([...run.pairs.map((pair) => pair.master), ...run.targets]),
        ],
        answers: this.#answers(run),
        pairs: run.pairs.map((pair) => ({
          ...pair,
          sources: [pair.source, ...unpaired],
          masters: [pair.master, ...run.targets],
        })),
        unpaired,
        untransferred: unpaired.filter((source) => !run.open.includes(source)),
      };
    })
  );

  readonly unanswered = computed(() =>
    (this.preview()?.sheets ?? []).reduce(
      (sum, sheet) => sum + sheet.open.length + (sheet.problem ? 1 : 0),
      0
    )
  );

  readonly writable = computed(
    () =>
      this.unanswered() === 0 &&
      (this.preview()?.sheets ?? []).some((sheet) => !sheet.problem)
  );

  async begin(dokumentId: string): Promise<void> {
    const data = await this.#backend.openMasterExport(dokumentId);
    if (data.masterExportStart) this.#store.begin(data.masterExportStart);
  }

  tick(sheet: string, on: boolean): void {
    this.#store.tick(sheet, on);
  }

  setAppend(sheet: string, append: boolean): void {
    this.#store.choose({ ...this.#choice(sheet), append });
  }

  setRemove(sheet: string, remove: boolean): void {
    this.#store.choose({ ...this.#choice(sheet), remove });
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

  async answer(
    sheet: string,
    column: string,
    target: string | undefined
  ): Promise<void> {
    if (target) await this.pair(sheet, column, target);
    else await this.unpair(sheet, column);
  }

  async pair(sheet: string, source: string, master: string): Promise<void> {
    const choice = this.#choice(sheet);
    this.#store.choose({
      ...choice,
      aliases: [
        ...choice.aliases.filter(
          (alias) => alias.source !== source && alias.master !== master
        ),
        { master, source },
      ],
      ignored: choice.ignored.filter((ignored) => ignored !== source),
    });
    await this.run();
  }

  async setIdentifier(
    sheet: string,
    source: string,
    master: string
  ): Promise<void> {
    const choice = this.#choice(sheet);
    this.#store.choose({
      ...choice,
      key: master,
      aliases: [
        ...choice.aliases.filter(
          (alias) => alias.source !== source && alias.master !== master
        ),
        { master, source },
      ],
      ignored: choice.ignored.filter((ignored) => ignored !== source),
    });
    await this.run();
  }

  async unpair(sheet: string, source: string): Promise<void> {
    const choice = this.#choice(sheet);
    this.#store.choose({
      ...choice,
      aliases: choice.aliases.filter((alias) => alias.source !== source),
      ignored: [
        ...choice.ignored.filter((ignored) => ignored !== source),
        source,
      ],
    });
    await this.run();
  }

  async setKey(sheet: string, key: string | undefined): Promise<void> {
    this.#store.choose({ ...this.#choice(sheet), key });
    await this.run();
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

  #answers(run: MasterExportSheetRun): ColumnAnswer[] {
    const columns = [
      ...run.open,
      ...run.aliases.map((alias) => alias.source),
      ...run.ignored,
    ];
    return [...new Set(columns)].map((column) => {
      const target = run.aliases.find(
        (alias) => alias.source === column
      )?.master;
      return {
        column,
        target,
        ignored: run.ignored.includes(column),
        options: [...new Set([...(target ? [target] : []), ...run.targets])],
      };
    });
  }
}
