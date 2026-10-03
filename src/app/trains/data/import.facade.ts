// ─── why ────────────────────────────────────────────────────────
// The import wizard's API. It owns the four-stage walk and the decisions, and it
// is the only thing that knows both the staged preview and what the user did
// with it.
//
// Auto-mapping runs ONCE, when a file arrives without a template — re-running it
// on every restage would overwrite the user's own choices with a guess. A
// template hit skips it entirely, which is the whole payoff: the second file
// from a sender costs nothing.
//
// `stagePath` is `pickFile` without the picker: the guided import hands an
// unrecognised file over by path, and it must land on the mapping step exactly
// as a picked file does — same auto-mapping, same template rule.
//
// `decisions` is public because the guided import commits the same preview
// through `IntakeFacade`, silently; two copies of the row→decision mapping would
// drift on the next entity added.
//
// `commit` sends only the rows the user ticked. Everything else is not "skipped"
// as an instruction, it is simply not asked for.
// ────────────────────────────────────────────────────────────────

import { computed, inject, Injectable } from '@angular/core';
import type {
  ColumnBinding,
  CommitDecisions,
  DecimalStyle,
  DateOrder,
  FieldKind,
  ImportPlan,
  RowStatus,
  TrainsData,
} from '../model/trains.types';
import { requiredFields } from '../model/field-catalogue';
import { suggestBindings } from '../util/column-suggest.util';
import { ImportStore, type RowChoice, type Stage } from './import.store';
import { TrainsBackend } from './trains.backend';
import { TrainsStore } from './trains.store';

@Injectable({ providedIn: 'root' })
export class ImportFacade {
  readonly #backend = inject(TrainsBackend);
  readonly #trains = inject(TrainsStore);
  readonly #store = inject(ImportStore);

  readonly staging = this.#trains.staging;
  readonly stage = this.#store.stage;
  readonly choices = this.#store.choices;
  readonly statusFilter = this.#store.statusFilter;
  readonly templateName = this.#store.templateName;

  readonly plan = computed(() => this.staging()?.plan);
  readonly summary = computed(() => this.staging()?.summary);
  readonly sheets = computed(() => this.staging()?.sheets ?? []);
  readonly candidates = computed(() => this.staging()?.candidates ?? []);
  readonly autoMapped = computed(
    () => this.staging()?.plan.templateId !== undefined
  );

  readonly missingRequired = computed(() => {
    const columns = this.plan()?.columns ?? [];
    return requiredFields().filter(
      (field) => !columns.some((column) => column.field === field)
    );
  });

  readonly canReview = computed(
    () => !!this.staging() && this.missingRequired().length === 0
  );

  readonly rows = computed(() => {
    const all = this.staging()?.rows ?? [];
    const filter = this.statusFilter();
    return filter ? all.filter((row) => row.status === filter) : all;
  });

  readonly includedRows = computed(() =>
    (this.staging()?.rows ?? []).filter(
      (row) => this.choices()[row.row]?.include
    )
  );

  async pickFile(): Promise<void> {
    await this.#adopt(await this.#backend.stageImport());
  }

  async stagePath(path: string): Promise<void> {
    await this.#adopt(await this.#backend.stageImportPath(path));
  }

  async #adopt(data: TrainsData): Promise<void> {
    this.#store.reset();
    this.#trains.applyTrainsData(data);
    if (!data.staging) return;

    const plan = data.staging.plan;
    // A template already carries the user's own mapping; guessing over it would
    // replace a decision with an assumption.
    if (!plan.templateId) {
      await this.applyPlan({ ...plan, columns: suggestBindings(plan.columns) });
    } else {
      this.#seed();
    }
    this.#store.setStage('mapping');
  }

  async selectSheet(sheet: string): Promise<void> {
    const data = await this.#backend.restageSheet(sheet);
    this.#trains.applyTrainsData(data);
    const plan = data.staging?.plan;
    if (plan && !plan.templateId) {
      await this.applyPlan({ ...plan, columns: suggestBindings(plan.columns) });
    }
  }

  async applyPlan(plan: ImportPlan): Promise<void> {
    this.#trains.applyTrainsData(await this.#backend.restageImport(plan));
    this.#seed();
  }

  async setField(index: number, field: FieldKind): Promise<void> {
    await this.#patchColumn(index, (column) => ({ ...column, field }));
  }

  async setDecimal(index: number, decimal: DecimalStyle): Promise<void> {
    await this.#patchColumn(index, (column) => ({ ...column, decimal }));
  }

  async setDateOrder(index: number, dateOrder: DateOrder): Promise<void> {
    await this.#patchColumn(index, (column) => ({ ...column, dateOrder }));
  }

  async setHeaderRow(headerRow: number): Promise<void> {
    const plan = this.plan();
    if (!plan) return;
    await this.applyPlan({
      ...plan,
      reader: 'manual',
      layout: { ...plan.layout, headerRow, firstDataRow: headerRow + 1 },
    });
  }

  setStage(stage: Stage): void {
    this.#store.setStage(stage);
  }

  setFilter(status: RowStatus | undefined): void {
    this.#store.setFilter(status);
  }

  setTemplateName(name: string): void {
    this.#store.setTemplateName(name);
  }

  setChoice(row: number, patch: Partial<RowChoice>): void {
    this.#store.setChoice(row, patch);
  }

  setAllIncluded(include: boolean): void {
    this.#store.setAllIncluded(
      this.rows().map((row) => row.row),
      include
    );
  }

  decisions(): CommitDecisions | undefined {
    const staging = this.staging();
    if (!staging) return undefined;
    const choices = this.choices();
    return {
      stagingId: staging.id,
      rows: this.includedRows().map((row) => ({
        row: row.row,
        wagen: choices[row.row]?.wagen ?? { action: 'create' },
        werkstatt: choices[row.row]?.werkstatt ?? { action: 'create' },
        halter: choices[row.row]?.halter ?? { action: 'create' },
        eigentuemer: choices[row.row]?.eigentuemer ?? { action: 'skip' },
        radsatz: choices[row.row]?.radsatz ?? { action: 'create' },
      })),
      saveTemplateAs: this.templateName().trim() || undefined,
    };
  }

  async commit(): Promise<void> {
    const decisions = this.decisions();
    if (!decisions) return;
    this.#trains.applyTrainsData(await this.#backend.commitImport(decisions));
    await this.discard();
  }

  async discard(): Promise<void> {
    this.#trains.applyTrainsData(await this.#backend.discardImport());
    this.#trains.clearStaging();
    this.#store.reset();
  }

  async #patchColumn(
    index: number,
    change: (column: ColumnBinding) => ColumnBinding
  ): Promise<void> {
    const plan = this.plan();
    if (!plan) return;
    await this.applyPlan({
      ...plan,
      columns: plan.columns.map((column) =>
        column.index === index ? change(column) : column
      ),
    });
  }

  #seed(): void {
    this.#store.seedChoices(this.staging()?.rows ?? []);
  }
}
