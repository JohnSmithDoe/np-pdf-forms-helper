// ─── why ────────────────────────────────────────────────────────
// The template mapper's API: a file nobody has a template for is staged, its
// columns mapped, and the mapping saved as a template — after which the file is
// cleaned like every other. It imports nothing; there is no way into the
// Schattensystem that skips the cleaning.
//
// Auto-mapping runs ONCE, when a file arrives without a template — re-running it
// on every restage would overwrite the user's own choices with a guess. A
// template hit skips it entirely: a template already carries the user's own
// mapping, and guessing over it would replace a decision with an assumption.
//
// `stagePath` is `pickFile` without the picker: the cleaning hub hands an
// unrecognised file over by path, and it must land on the mapping step exactly
// as a picked file does — same auto-mapping, same template rule. `handedOver`
// remembers that it came from the hub, so saving can send the user back there.
// ────────────────────────────────────────────────────────────────

import { computed, inject, Injectable, signal } from '@angular/core';
import type {
  ColumnBinding,
  DecimalStyle,
  DateOrder,
  FieldKind,
  ImportPlan,
  TrainsData,
} from '../model/trains.types';
import { requiredFields } from '../model/field-catalogue';
import { suggestBindings } from '../util/column-suggest.util';
import { ImportStore, type Stage } from './import.store';
import { TrainsBackend } from './trains.backend';
import { TrainsStore } from './trains.store';

@Injectable({ providedIn: 'root' })
export class ImportFacade {
  readonly #backend = inject(TrainsBackend);
  readonly #trains = inject(TrainsStore);
  readonly #store = inject(ImportStore);
  readonly #handedOver = signal(false);

  readonly staging = this.#trains.staging;
  readonly stage = this.#store.stage;
  readonly handedOver = this.#handedOver.asReadonly();

  readonly plan = computed(() => this.staging()?.plan);
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

  readonly canSave = computed(
    () => !!this.staging() && this.missingRequired().length === 0
  );

  async pickFile(): Promise<void> {
    this.#handedOver.set(false);
    await this.#adopt(await this.#backend.stageImport());
  }

  async stagePath(path: string): Promise<void> {
    await this.#adopt(await this.#backend.stageImportPath(path));
    this.#handedOver.set(true);
  }

  async #adopt(data: TrainsData): Promise<void> {
    this.#store.reset();
    this.#trains.applyTrainsData(data);
    if (!data.staging) return;

    const plan = data.staging.plan;
    if (!plan.templateId) {
      await this.applyPlan({ ...plan, columns: suggestBindings(plan.columns) });
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

  async setPruefart(art: string): Promise<void> {
    const plan = this.plan();
    if (!plan) return;
    const pruefart = art.trim() || undefined;
    if (pruefart === plan.pruefart) return;
    await this.applyPlan({ ...plan, pruefart });
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

  async saveTemplate(name: string): Promise<void> {
    this.#trains.applyTrainsData(await this.#backend.saveTemplate(name));
    this.#trains.clearStaging();
    this.#store.reset();
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
}
