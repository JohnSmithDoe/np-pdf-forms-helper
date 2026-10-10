// ─── why ────────────────────────────────────────────────────────
// The cleaning batch's API: scan → pick → clean → review → file → next file →
// summary. It is a CLOSED cycle and imports nothing: a filed file is a
// `Dokument`, and taking it into the Schattensystem is `ImportWalkFacade`'s job,
// started from the summary or the document list whenever the user likes.
//
// `startNext` RETURNS where to go instead of navigating: navigation is a page
// decision, and a facade that routed would be a data layer that knows URLs.
// A file that needs a template is STAGED by path (`ImportFacade.stagePath`) and
// remembered as `handedOver`; when the hub is shown again, `resume` rescans
// exactly that file, which the template just saved now recognises.
//
// A failed `clean_file` marks the file `fehlgeschlagen` and rethrows. Without
// the mark the next „Fortsetzen“ would retry the same file forever; picking a
// different template for it clears the mark.
//
// `write_clean` is `silent` and its report PARKED on the file's result: the
// batch summary renders every file's report at once, and a toast per file in
// between would interrupt a walk the user is in the middle of. The new
// document's id is found by its OWNED original — the cleaning report names that
// path, and the backend files the record under it.
// ────────────────────────────────────────────────────────────────

import { computed, inject, Injectable } from '@angular/core';
import {
  BackendError,
  BackendService,
} from '../../@shared/data/backend/backend.service';
import type {
  CleanDecisions,
  Confirmation,
  Dokument,
  ScanFile,
  ScanStatus,
  TrainsData,
} from '../model/trains.types';
import { ImportFacade } from './import.facade';
import {
  IntakeStore,
  type FileOutcome,
  type FilePick,
  type FileResult,
} from './intake.store';
import { TrainsBackend } from './trains.backend';
import { TrainsStore } from './trains.store';

export type IntakeStep = 'clean' | 'template' | 'summary';

export interface PickOption {
  value: string;
  label: string;
}

export interface IntakeRow {
  file: ScanFile;
  selectable: boolean;
  options: PickOption[];
  value: string | undefined;
  result: FileResult | undefined;
  skipped: boolean;
}

export interface SummaryRow {
  file: ScanFile;
  outcome: FileOutcome | 'vorhanden' | 'uebersprungen' | 'offen';
  result: FileResult | undefined;
  dokument: Dokument | undefined;
}

const CREATE = 'create';
const SKIP = 'skip';
const UNSELECTABLE: ReadonlySet<ScanStatus> = new Set([
  'nichtUnterstuetzt',
  'unlesbar',
  'vorhanden',
]);

function optionsOf(file: ScanFile): PickOption[] {
  const named = file.sheets.length > 1;
  return [
    ...file.matches.map((match, index) => ({
      value: `match:${index}`,
      label: named
        ? `${match.templateName} · Blatt „${match.sheet}“`
        : match.templateName,
    })),
    { value: CREATE, label: 'Vorlage anlegen' },
    { value: SKIP, label: 'Nicht bereinigen' },
  ];
}

function valueOf(
  file: ScanFile,
  pick: FilePick | undefined
): string | undefined {
  if (!pick) return undefined;
  if (pick.kind === 'create') return CREATE;
  if (pick.kind === 'skip') return SKIP;
  const index = file.matches.findIndex(
    (match) =>
      match.templateId === pick.templateId && match.sheet === pick.sheet
  );
  return index === -1 ? undefined : `match:${index}`;
}

@Injectable({ providedIn: 'root' })
export class IntakeFacade {
  readonly #backend = inject(TrainsBackend);
  readonly #transport = inject(BackendService);
  readonly #trains = inject(TrainsStore);
  readonly #import = inject(ImportFacade);
  readonly #store = inject(IntakeStore);

  readonly busy = this.#transport.busy;
  readonly fileDrops$ = this.#transport.fileDrops$;

  readonly loaded = this.#trains.loaded;
  readonly scan = this.#store.scan;
  readonly cleaning = this.#store.cleaning;
  readonly decisions = this.#store.decisions;

  readonly rows = computed<IntakeRow[]>(() => {
    const picks = this.#store.picks();
    const results = this.#store.results();
    return (this.scan() ?? []).map((file) => ({
      file,
      selectable: !UNSELECTABLE.has(file.status),
      options: optionsOf(file),
      value: valueOf(file, picks[file.path]),
      result: results[file.path],
      skipped: picks[file.path]?.kind === 'skip',
    }));
  });

  readonly queue = computed(() =>
    this.rows().filter(
      (row) =>
        row.selectable &&
        !row.skipped &&
        (row.result === undefined || row.result.outcome === 'fehlgeschlagen')
    )
  );

  readonly unchosen = computed(
    () => this.queue().filter((row) => row.value === undefined).length
  );

  readonly started = computed(() =>
    this.rows().some((row) => row.result !== undefined)
  );

  readonly canStart = computed(() => {
    const queue = this.queue();
    return queue.length > 0 && this.unchosen() === 0;
  });

  readonly currentFile = computed(() => {
    const path = this.#store.current();
    return this.scan()?.find((file) => file.path === path);
  });

  readonly progress = computed(() => {
    const walked = this.rows().filter((row) => row.selectable && !row.skipped);
    const done = walked.filter(
      (row) => row.result && row.result.outcome !== 'fehlgeschlagen'
    ).length;
    return {
      step: Math.min(done + 1, Math.max(walked.length, 1)),
      count: Math.max(walked.length, 1),
    };
  });

  readonly canWrite = computed(() => {
    const summary = this.cleaning()?.summary;
    return (
      !!summary && summary.fehlerOffen === 0 && summary.deutungenOffen === 0
    );
  });

  readonly summary = computed<SummaryRow[]>(() => {
    const byId = this.#trains.dokumentById();
    return this.rows().map((row) => {
      const id = row.result?.dokumentId ?? row.file.vorhanden?.dokumentId;
      return {
        file: row.file,
        outcome:
          row.result?.outcome ??
          (row.file.status === 'vorhanden'
            ? 'vorhanden'
            : row.skipped || !row.selectable
              ? 'uebersprungen'
              : 'offen'),
        result: row.result,
        dokument: id ? byId.get(id) : undefined,
      };
    });
  });

  readonly importable = computed(() =>
    this.summary()
      .filter((row) => row.outcome === 'bereinigt')
      .map((row) => row.dokument)
      .filter((dokument): dokument is Dokument => !!dokument)
      .filter((dokument) => !dokument.importiertAm)
  );

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

  async resume(): Promise<void> {
    const path = this.#store.handedOver();
    if (!path) return;
    this.#store.handOver(undefined);
    const data = await this.#backend.scanImportPaths([path]);
    if (data.scan) this.#store.merge(data.scan);
  }

  choose(path: string, value: string | undefined): void {
    const file = this.scan()?.find((entry) => entry.path === path);
    if (!file) return;
    if (value === CREATE || value === SKIP) {
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
    if (!row) return 'summary';
    const pick = this.#store.picks()[row.file.path];
    if (!pick || pick.kind === 'skip') return 'summary';

    try {
      if (pick.kind === 'create') {
        await this.#import.stagePath(row.file.path);
        this.#store.handOver(row.file.path);
        return 'template';
      }
      const data = await this.#backend.cleanFile(
        row.file.path,
        pick.sheet,
        pick.templateId
      );
      this.#store.begin(row.file.path, data.cleaning);
      return 'clean';
    } catch (error) {
      if (error instanceof BackendError) {
        this.#store.setResult(row.file.path, { outcome: 'fehlgeschlagen' });
      }
      throw error;
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
    const owned = this.cleaning()?.file;
    const data = await this.#backend.writeClean(this.decisions(), {
      silent: true,
    });
    this.#trains.applyTrainsData(data);
    const dokument = data.dokumente?.find((entry) => entry.original === owned);
    this.#endAs({
      outcome: 'bereinigt',
      report: data.message,
      dokumentId: dokument?.id,
    });
  }

  async discardClean(): Promise<void> {
    await this.#backend.discardClean();
    this.#endAs({ outcome: 'verworfen' });
  }

  async openOriginal(): Promise<void> {
    const file = this.cleaning()?.file;
    if (file) await this.#backend.openFile(file);
  }

  async openFile(path: string): Promise<void> {
    await this.#backend.openFile(path);
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

  #endAs(result: FileResult): void {
    const path = this.#store.current();
    if (path) this.#store.setResult(path, result);
    this.#store.endCurrent();
  }
}
