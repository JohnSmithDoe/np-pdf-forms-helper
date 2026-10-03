// ─── why ────────────────────────────────────────────────────────
// The import walk's API: one cleaned document, staged once, then walked BY TYPE
// — Partner, Wagen, Radsätze, Einträge — to a summary that is the plan, and one
// commit that is all-or-nothing. Nothing is written before that commit; the
// steps only collect answers, which is the plan/apply split.
//
// One answer per entity GROUP, not per row: the backend grouped the staging
// (`entities`) and expands the answers back per row (`entities::expand`), so the
// rule for which rows a group covers exists once, in Rust.
//
// The DEFAULT is an answer only where nothing could be misread: a `known`
// group is its match, a `missing` one is skipped. `likely`, `ambiguous` and
// `new` stay undecided, and a step cannot be left while any of its groups is —
// the never-auto-resolve rule, per entity.
//
// A declined Wagen drops its rows, exactly as the commit will: the Wagennummer
// is the anchor, and nothing may hang off a Wagen that is not created. So the
// later steps show those rows as gone rather than offering them — the walk
// must not ask about something the commit is going to skip anyway. A declined
// Partner or Radsatz only leaves a reference empty, so it drops nothing.
//
// Duplicates and rejected rows start excluded; a duplicate may be taken back on
// the Einträge step, a rejected row cannot.
//
// The BULK answers exist because a file of four hundred new Wagen is four
// hundred clicks otherwise. They are still the user's decision, made once for
// a named set — „alle neuen anlegen“, „alle Vorschläge übernehmen“ — and they
// only ever answer groups that are STILL OPEN: a choice already made is never
// overwritten by a bulk click. `ambiguous` has no bulk „take“ — there is no
// one candidate to take — only the bulk „nicht übernehmen“.
//
// `commit` is `silent` and its report PARKED: the result step renders it, and
// a toast saying the same over it would be noise. `next` walks the queue a
// batch summary handed over with „Alle importieren“, one document per walk.
// ────────────────────────────────────────────────────────────────

import { computed, inject, Injectable } from '@angular/core';
import { BackendService } from '../../@shared/data/backend/backend.service';
import type {
  EntityChoice,
  EntityDecision,
  EntityGroup,
  EntityKind,
  StagedRow,
} from '../model/trains.types';
import { ImportWalkStore } from './import-walk.store';
import { TrainsBackend } from './trains.backend';
import { TrainsStore } from './trains.store';

export interface GroupView {
  group: EntityGroup;
  decision: EntityDecision | undefined;
}

export interface EntryView {
  row: StagedRow;
  gone: boolean;
  included: boolean;
  writes: string[];
}

export type BulkAnswer = 'create' | 'suggestion' | 'skip';

export interface BulkCounts {
  create: number;
  suggestion: number;
  skip: number;
}

export interface PlanCount {
  kind: EntityKind;
  label: string;
  linked: number;
  created: number;
  skipped: number;
}

const WORK = new Set(['datum', 'leistung', 'betrag', 'bemerkung']);
const FITTING = new Set(['eingebautAm', 'ausgebautAm']);

const LABELS: Record<EntityKind, string> = {
  partner: 'Partner',
  wagen: 'Wagen',
  radsatz: 'Radsätze',
};

function slotOf(group: EntityGroup): string {
  return `${group.kind}|${group.key}`;
}

function defaultOf(group: EntityGroup): EntityDecision | undefined {
  const resolution = group.resolution;
  if (resolution.state === 'known') return { action: 'use', id: resolution.id };
  if (resolution.state === 'missing') return { action: 'skip' };
  return undefined;
}

function writesOf(row: StagedRow): string[] {
  const filled = row.cells.filter((cell) => cell.ok && cell.parsed !== '');
  const writes: string[] = [];
  if (filled.some((cell) => FITTING.has(cell.field))) {
    writes.push('Ein-/Ausbau');
  }
  if (filled.some((cell) => WORK.has(cell.field))) {
    writes.push('Instandhaltung');
  }
  return writes;
}

@Injectable({ providedIn: 'root' })
export class ImportWalkFacade {
  readonly #backend = inject(TrainsBackend);
  readonly #transport = inject(BackendService);
  readonly #trains = inject(TrainsStore);
  readonly #store = inject(ImportWalkStore);

  readonly busy = this.#transport.busy;
  readonly loaded = this.#trains.loaded;
  readonly report = this.#store.report;
  readonly queue = this.#store.queue;

  readonly staging = computed(() => {
    const staging = this.#trains.staging();
    const id = this.#store.dokumentId();
    return id && staging?.dokumentId === id ? staging : undefined;
  });

  readonly dokument = computed(() => {
    const id = this.#store.dokumentId();
    return id ? this.#trains.dokumentById().get(id) : undefined;
  });

  readonly walking = computed(() => this.staging() !== undefined);

  readonly partner = computed(() =>
    this.#views(this.staging()?.entities?.partner)
  );
  readonly wagen = computed(() => this.#views(this.staging()?.entities?.wagen));
  readonly radsaetze = computed(() =>
    this.#views(this.staging()?.entities?.radsaetze)
  );

  readonly goneRows = computed(() => {
    const gone = new Set<number>();
    const anchored = new Set<number>();
    for (const view of this.wagen()) {
      for (const row of view.group.rows) {
        anchored.add(row);
        if (view.decision?.action === 'skip') gone.add(row);
      }
    }
    for (const row of this.staging()?.rows ?? []) {
      if (!anchored.has(row.row)) gone.add(row.row);
    }
    return gone;
  });

  readonly entries = computed<EntryView[]>(() => {
    const gone = this.goneRows();
    const included = this.#store.included();
    return (this.staging()?.rows ?? []).map((row) => ({
      row,
      gone: gone.has(row.row),
      included:
        !gone.has(row.row) &&
        row.status !== 'rejected' &&
        (included[row.row] ?? row.status !== 'duplicate'),
      writes: writesOf(row),
    }));
  });

  readonly takenRows = computed(() =>
    this.entries()
      .filter((entry) => entry.included)
      .map((entry) => entry.row.row)
  );

  readonly plan = computed<PlanCount[]>(() =>
    (
      [
        ['partner', this.partner()],
        ['wagen', this.wagen()],
        ['radsatz', this.radsaetze()],
      ] as const
    ).map(([kind, views]) => {
      const live = views.filter((view) =>
        view.group.rows.some((row) => !this.goneRows().has(row))
      );
      return {
        kind,
        label: LABELS[kind],
        linked: live.filter((view) => view.decision?.action === 'use').length,
        created: live.filter((view) => view.decision?.action === 'create')
          .length,
        skipped: live.filter(
          (view) => !view.decision || view.decision.action === 'skip'
        ).length,
      };
    })
  );

  readonly undecided = computed(() => {
    const open = (views: GroupView[]): number =>
      views.filter(
        (view) =>
          !view.decision &&
          view.group.rows.some((row) => !this.goneRows().has(row))
      ).length;
    return {
      partner: open(this.partner()),
      wagen: this.wagen().filter((view) => !view.decision).length,
      radsatz: open(this.radsaetze()),
    };
  });

  readonly complete = computed(() => {
    const open = this.undecided();
    return open.partner + open.wagen + open.radsatz === 0;
  });

  bulkCounts(kind: EntityKind): BulkCounts {
    const open = this.#open(kind);
    return {
      create: open.filter((view) => this.#bulk(view, 'create')).length,
      suggestion: open.filter((view) => this.#bulk(view, 'suggestion')).length,
      skip: open.length,
    };
  }

  answerOpen(kind: EntityKind, answer: BulkAnswer): void {
    const decisions: Record<string, EntityDecision> = {};
    for (const view of this.#open(kind)) {
      const decision = this.#bulk(view, answer);
      if (decision) decisions[slotOf(view.group)] = decision;
    }
    this.#store.decideMany(decisions);
  }

  includeAll(include: boolean): void {
    this.#store.includeMany(
      this.entries()
        .filter((entry) => !entry.gone && entry.row.status !== 'rejected')
        .map((entry) => entry.row.row),
      include
    );
  }

  async start(dokumentId: string, queue: string[] = []): Promise<void> {
    const data = await this.#backend.stageDocument(dokumentId);
    this.#trains.applyTrainsData(data);
    this.#store.begin(dokumentId, queue);
  }

  async startQueue(ids: string[]): Promise<void> {
    const [first, ...rest] = ids;
    if (first) await this.start(first, rest);
  }

  async next(): Promise<boolean> {
    const queue = this.queue();
    this.#store.reset();
    this.#trains.clearStaging();
    if (queue.length === 0) return false;
    await this.startQueue(queue);
    return true;
  }

  decide(group: EntityGroup, decision: EntityDecision): void {
    this.#store.decide(slotOf(group), decision);
  }

  include(row: number, include: boolean): void {
    this.#store.include(row, include);
  }

  async commit(): Promise<void> {
    const staging = this.staging();
    if (!staging) return;
    const choices = (views: GroupView[]): EntityChoice[] =>
      views.flatMap((view) =>
        view.decision ? [{ key: view.group.key, decision: view.decision }] : []
      );
    const data = await this.#backend.commitDocument(
      {
        stagingId: staging.id,
        partner: choices(this.partner()),
        wagen: choices(this.wagen()),
        radsaetze: choices(this.radsaetze()),
        rows: this.takenRows(),
      },
      { silent: true }
    );
    this.#trains.applyTrainsData(data);
    this.#trains.clearStaging();
    this.#store.finish(
      data.message ?? { headline: 'Import wurde übernommen', messages: [] }
    );
  }

  async cancel(): Promise<void> {
    if (this.staging()) {
      this.#trains.applyTrainsData(await this.#backend.discardImport());
      this.#trains.clearStaging();
    }
    this.#store.reset();
  }

  finish(): void {
    this.#store.reset();
  }

  async openFile(path: string): Promise<void> {
    await this.#backend.openFile(path);
  }

  #open(kind: EntityKind): GroupView[] {
    const views =
      kind === 'partner'
        ? this.partner()
        : kind === 'wagen'
          ? this.wagen()
          : this.radsaetze();
    const gone = this.goneRows();
    return views.filter(
      (view) => !view.decision && view.group.rows.some((row) => !gone.has(row))
    );
  }

  #bulk(view: GroupView, answer: BulkAnswer): EntityDecision | undefined {
    const resolution = view.group.resolution;
    if (answer === 'skip') return { action: 'skip' };
    if (answer === 'create') {
      return resolution.state === 'new' ? { action: 'create' } : undefined;
    }
    return resolution.state === 'likely'
      ? { action: 'use', id: resolution.id }
      : undefined;
  }

  #views(groups: EntityGroup[] | undefined): GroupView[] {
    const decisions = this.#store.decisions();
    return (groups ?? []).map((group) => ({
      group,
      decision: decisions[slotOf(group)] ?? defaultOf(group),
    }));
  }
}
