// ─── why ────────────────────────────────────────────────────────
// The guided import's API: scan → pick → clean → review → write → preview →
// commit → next file. It is the only thing that knows the walk, the cleaning
// under review and the preview that follows it, which is why it reaches three
// stores — `IntakeStore` for the walk, `TrainsStore` for the staging every
// import shares, `ImportStore` for the row choices the preview edits.
//
// `startNext` RETURNS where to go instead of navigating: navigation is a page
// decision, and a facade that routed would be a data layer that knows URLs.
// A file the user wants imported by hand is STAGED by path
// (`ImportFacade.stagePath`) and marked done as it is handed over, so the manual
// page opens on its mapping step. That page knows nothing about the walk and
// could not report back — the walk resumes from the hub with the file after it.
//
// A failed `clean_file` or `stage_import_path` marks the file `fehlgeschlagen` and rethrows. Without
// the mark the next „Fortsetzen“ would retry the same file forever; picking a
// different template for it clears the mark.
//
// `write_clean` and `commit_import` are `silent` and their reports PARKED: the
// result step renders both, and a toast or dialog on top would say the same
// thing over the page that already says it. The cleaned file's report is the
// one with the folder, so it is the one that offers "Ordner öffnen".
//
// After `write_clean` the staging is a PREVIEW like any other, so the row
// choices are seeded exactly as the manual import seeds them — and the commit
// goes through `ImportFacade.decisions`, the one place rows become decisions.
// ────────────────────────────────────────────────────────────────

import { computed, inject, Injectable } from '@angular/core';
import {
  BackendError,
  BackendService,
} from '../../@shared/data/backend/backend.service';
import type {
  CleanDecisions,
  Confirmation,
  ScanFile,
  ScanStatus,
  TrainsData,
} from '../model/trains.types';
import { ImportFacade } from './import.facade';
import { ImportStore } from './import.store';
import { IntakeStore, type FileOutcome, type FilePick } from './intake.store';
import { TrainsBackend } from './trains.backend';
import { TrainsStore } from './trains.store';

export type IntakeStep = 'clean' | 'manual' | 'done';

export interface PickOption {
  value: string;
  label: string;
}

export interface IntakeRow {
  file: ScanFile;
  selectable: boolean;
  options: PickOption[];
  value: string | undefined;
  outcome: FileOutcome | undefined;
  skipped: boolean;
}

const MANUAL = 'manual';
const SKIP = 'skip';
const SKIPPED: readonly ScanStatus[] = ['nichtUnterstuetzt', 'unlesbar'];

function optionsOf(file: ScanFile): PickOption[] {
  const named = file.sheets.length > 1;
  return [
    ...file.matches.map((match, index) => ({
      value: `match:${index}`,
      label: named
        ? `${match.templateName} · Blatt „${match.sheet}“`
        : match.templateName,
    })),
    { value: MANUAL, label: 'Von Hand zuordnen' },
    { value: SKIP, label: 'Nicht importieren' },
  ];
}

function valueOf(
  file: ScanFile,
  pick: FilePick | undefined
): string | undefined {
  if (!pick) return undefined;
  if (pick.kind === 'manual') return MANUAL;
  if (pick.kind === 'skip') return SKIP;
  const index = file.matches.findIndex(
    (match) =>
      match.templateId === pick.templateId && match.sheet === pick.sheet
  );
  return index >= 0 ? `match:${index}` : undefined;
}

@Injectable({ providedIn: 'root' })
export class IntakeFacade {
  readonly #backend = inject(TrainsBackend);
  readonly #transport = inject(BackendService);
  readonly #trains = inject(TrainsStore);
  readonly #imports = inject(ImportStore);
  readonly #import = inject(ImportFacade);
  readonly #store = inject(IntakeStore);

  readonly busy = this.#transport.busy;
  readonly fileDrops$ = this.#transport.fileDrops$;

  readonly loaded = this.#trains.loaded;
  readonly scan = this.#store.scan;
  readonly cleaning = this.#store.cleaning;
  readonly decisions = this.#store.decisions;
  readonly cleanedReport = this.#store.cleanedReport;
  readonly cleanedFile = this.#store.cleanedFile;
  readonly commitReport = this.#store.commitReport;
  readonly staging = this.#trains.staging;

  readonly rows = computed<IntakeRow[]>(() => {
    const picks = this.#store.picks();
    const outcomes = this.#store.outcomes();
    return (this.scan() ?? []).map((file) => ({
      file,
      selectable: !SKIPPED.includes(file.status),
      options: optionsOf(file),
      value: valueOf(file, picks[file.path]),
      outcome: outcomes[file.path],
      skipped: picks[file.path]?.kind === 'skip',
    }));
  });

  readonly queue = computed(() =>
    this.rows().filter(
      (row) =>
        row.selectable &&
        !row.skipped &&
        (row.outcome === undefined || row.outcome === 'fehlgeschlagen')
    )
  );

  readonly unchosen = computed(
    () => this.queue().filter((row) => row.value === undefined).length
  );

  readonly started = computed(() =>
    this.rows().some((row) => row.outcome !== undefined)
  );

  readonly canStart = computed(() => {
    const queue = this.queue();
    return queue.length > 0 && this.unchosen() === 0;
  });

  readonly currentFile = computed(() => {
    const path = this.#store.current();
    return this.scan()?.find((file) => file.path === path);
  });

  readonly inGuidedPreview = computed(
    () => this.#store.current() !== undefined && this.staging() !== undefined
  );

  readonly canWrite = computed(() => {
    const summary = this.cleaning()?.summary;
    return (
      !!summary && summary.fehlerOffen === 0 && summary.deutungenOffen === 0
    );
  });

  async pickFolder(): Promise<void> {
    this.#applyScan(await this.#backend.pickImportFolder());
  }

  async pickFiles(): Promise<void> {
    this.#applyScan(await this.#backend.pickImportFiles());
  }

  async scanPaths(paths: string[]): Promise<void> {
    if (paths.length === 0) return;
    this.#applyScan(await this.#backend.scanImportPaths(paths));
  }

  choose(path: string, value: string | undefined): void {
    const file = this.scan()?.find((entry) => entry.path === path);
    if (!file) return;
    if (value === MANUAL || value === SKIP) {
      this.#store.setPick(path, { kind: value });
      return;
    }
    const match = file.matches[Number(value?.replace('match:', ''))];
    this.#store.setPick(
      path,
      match
        ? { kind: 'template', templateId: match.templateId, sheet: match.sheet }
        : undefined
    );
  }

  clear(): void {
    this.#store.reset();
  }

  async startNext(): Promise<IntakeStep> {
    const row = this.queue()[0];
    if (!row) return 'done';
    const pick = this.#store.picks()[row.file.path];
    if (!pick || pick.kind === 'skip') return 'done';

    try {
      if (pick.kind === 'manual') {
        await this.#import.stagePath(row.file.path);
        this.#store.setOutcome(row.file.path, 'manuell');
        this.#store.endCurrent();
        return 'manual';
      }
      const data = await this.#backend.cleanFile(
        row.file.path,
        pick.sheet,
        pick.templateId
      );
      this.#store.begin(row.file.path, data.cleaning);
      return 'clean';
    } catch (cause) {
      if (cause instanceof BackendError) {
        this.#store.setOutcome(row.file.path, 'fehlgeschlagen');
      }
      throw cause;
    }
  }

  async correct(row: number, column: number, value: string): Promise<void> {
    const decisions = this.decisions();
    await this.#reclean({
      ...decisions,
      corrections: [
        ...decisions.corrections.filter(
          (entry) => entry.row !== row || entry.column !== column
        ),
        { row, column, value },
      ],
    });
  }

  async confirm(confirmation: Confirmation): Promise<void> {
    const decisions = this.decisions();
    await this.#reclean({
      ...decisions,
      confirmations: [
        ...decisions.confirmations.filter(
          (entry) => entry.column !== confirmation.column
        ),
        confirmation,
      ],
    });
  }

  async writeClean(): Promise<void> {
    const data = await this.#backend.writeClean(this.decisions(), {
      silent: true,
    });
    this.#trains.applyTrainsData(data);
    this.#imports.reset();
    this.#imports.seedChoices(data.staging?.rows ?? []);
    this.#store.setCleaning(undefined);
    this.#store.setCleaned(data.message ?? null, data.cleanedFile);
  }

  async discardClean(): Promise<void> {
    await this.#backend.discardClean();
    this.#endAs('verworfen');
  }

  async commit(): Promise<void> {
    const decisions = this.#import.decisions();
    if (!decisions) return;
    const data = await this.#backend.commitImport(decisions, { silent: true });
    this.#trains.applyTrainsData(data);
    await this.#import.discard();
    const path = this.#store.current();
    if (path) this.#store.setOutcome(path, 'importiert');
    this.#store.setCommitReport(
      data.message ?? { headline: 'Import wurde übernommen', messages: [] }
    );
  }

  async discardPreview(): Promise<void> {
    await this.#import.discard();
    this.#endAs('verworfen');
  }

  finish(): void {
    this.#store.endCurrent();
  }

  async openOriginal(): Promise<void> {
    const file = this.cleaning()?.file;
    if (file) await this.#backend.openFile(file);
  }

  async openCleaned(): Promise<void> {
    const file = this.cleanedFile();
    if (file) await this.#backend.openFile(file);
  }

  async openFolder(folder: string): Promise<void> {
    await this.#backend.openFolder(folder);
  }

  async #reclean(decisions: CleanDecisions): Promise<void> {
    const data = await this.#backend.recleanFile(decisions);
    this.#store.setDecisions(decisions);
    if (data.cleaning) this.#store.setCleaning(data.cleaning);
  }

  #applyScan(data: TrainsData): void {
    if (data.scan) this.#store.applyScan(data.scan);
  }

  #endAs(outcome: FileOutcome): void {
    const path = this.#store.current();
    if (path) this.#store.setOutcome(path, outcome);
    this.#store.endCurrent();
  }
}
