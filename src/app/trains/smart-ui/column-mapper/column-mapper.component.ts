// ─── why ────────────────────────────────────────────────────────
// Stage two, and the product. Mapping columns is technically trivial; making it
// fast, visible and repeatable is the whole feature.
//
// Three things carry that, and none is decoration:
//
// SAMPLE VALUES, verbatim, on every row. Nobody can map a column called `Bem.`
// without seeing what is in it, and nobody should trust an interpretation
// without seeing what it did to a real value. The samples come from the staged
// rows, so they are what the backend actually read.
//
// THE INTERPRETATION, not a parser choice. The field decides the parser; what
// the user overrides is how the backend READ the column — German or English
// decimals, day or month first. Because that inference is column-level, one flip
// re-reads every cell in the column, which is impossible if the guess is
// per-cell.
//
// AUTO-MAPPED ROWS SAY SO. A silent guess looks like something a human decided.
// The user needs to know what to check, not what to redo.
//
// Samples are taken from the COLUMN and not from the field, so an unmapped
// column still shows what is in it — which is the whole way a user works out
// what it is. Matching by field would also merge two columns mapped to the same
// one.
//
// Every change goes to the backend and comes back re-staged, so what is on
// screen is always what was actually parsed rather than a local prediction of
// it. A half-mapped plan stages fine; only review and commit need the required
// fields.
//
// „Prüfart der Vorlage“ appears only while a Prüfung column is mapped and no
// Prüfart column is: a P8 list has no Art column because the whole file is P8,
// and the template has to say so once.
//
// The samples are collected in ONE pass over the staged rows, not one pass per
// column. This recomputes on every single field, decimal and date-order pick —
// each of which re-stages — and a per-column scan walked every cell of every row
// once per column only to throw all but three values away.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
  output,
} from '@angular/core';
import {
  IonBadge,
  IonCard,
  IonCardContent,
  IonCardHeader,
  IonCardTitle,
  IonInput,
  IonItem,
  IonLabel,
  IonList,
  IonNote,
  IonSegment,
  IonSegmentButton,
  IonSelect,
  IonSelectOption,
} from '@ionic/angular/standalone';
import { BackendError } from '../../../@shared/data/backend/backend.service';
import { ImportFacade } from '../../data';
import {
  FIELD_CATALOGUE,
  GROUP_LABELS,
  labelOf,
} from '../../model/field-catalogue';
import type {
  DateOrder,
  DecimalStyle,
  FieldKind,
} from '../../model/trains.types';

interface MapperRow {
  index: number;
  header: string;
  field: FieldKind;
  samples: string[];
  parsed: string[];
  decimal?: DecimalStyle;
  dateOrder?: DateOrder;
  warning?: string;
}

const SAMPLE_COUNT = 3;

@Component({
  selector: 'app-column-mapper',
  templateUrl: 'column-mapper.component.html',
  styleUrls: ['column-mapper.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    IonBadge,
    IonCard,
    IonCardContent,
    IonCardHeader,
    IonCardTitle,
    IonInput,
    IonItem,
    IonLabel,
    IonList,
    IonNote,
    IonSegment,
    IonSegmentButton,
    IonSelect,
    IonSelectOption,
  ],
})
export class ColumnMapperComponent {
  protected readonly facade = inject(ImportFacade);
  readonly failed = output<BackendError>();

  protected readonly groups = GROUP_LABELS;
  protected readonly catalogue = FIELD_CATALOGUE;

  protected readonly asksPruefart = computed(() => {
    const columns = this.facade.staging()?.plan.columns ?? [];
    const fields = new Set(columns.map((column) => column.field));
    return (
      !fields.has('pruefart') &&
      [...fields].some(
        (field) =>
          field !== 'pruefart' &&
          FIELD_CATALOGUE.find((entry) => entry.field === field)?.group ===
            'pruefung'
      )
    );
  });

  protected readonly pruefart = computed(
    () => this.facade.staging()?.plan.pruefart ?? ''
  );

  protected readonly missing = computed(() =>
    this.facade.missingRequired().map((field) => labelOf(field))
  );

  protected readonly rows = computed<MapperRow[]>(() => {
    const staging = this.facade.staging();
    if (!staging) return [];

    const samples = new Map<
      FieldKind,
      { raw: Set<string>; parsed: Set<string> }
    >();
    const warnings = new Map<string, string>();
    for (const row of staging.rows) {
      for (const cell of row.cells) {
        if (!cell.raw.trim()) continue;
        let seen = samples.get(cell.field);
        if (!seen) {
          seen = { raw: new Set(), parsed: new Set() };
          samples.set(cell.field, seen);
        }
        if (seen.raw.size < SAMPLE_COUNT) seen.raw.add(cell.raw);
        if (seen.parsed.size < SAMPLE_COUNT) seen.parsed.add(cell.parsed);
      }
      for (const issue of row.issues) {
        if (issue.severity === 'warnung' && !warnings.has(issue.column))
          warnings.set(issue.column, issue.message);
      }
    }

    return staging.plan.columns.map((column) => {
      const seen = samples.get(column.field);
      return {
        index: column.index,
        header: column.header,
        field: column.field,
        decimal: column.decimal,
        dateOrder: column.dateOrder,
        samples: [...(seen?.raw ?? [])],
        parsed: [...(seen?.parsed ?? [])],
        warning: warnings.get(column.header),
      };
    });
  });

  protected async onField(index: number, event: Event): Promise<void> {
    const field = (event as CustomEvent<{ value: FieldKind }>).detail.value;
    await this.#run(() => this.facade.setField(index, field));
  }

  protected async onPruefart(event: Event): Promise<void> {
    const value = (event as CustomEvent<{ value?: string | null }>).detail
      .value;
    await this.#run(() => this.facade.setPruefart(value ?? ''));
  }

  protected async onDecimal(index: number, event: Event): Promise<void> {
    const value = (event as CustomEvent<{ value: DecimalStyle }>).detail.value;
    await this.#run(() => this.facade.setDecimal(index, value));
  }

  protected async onDateOrder(index: number, event: Event): Promise<void> {
    const value = (event as CustomEvent<{ value: DateOrder }>).detail.value;
    await this.#run(() => this.facade.setDateOrder(index, value));
  }

  async #run(action: () => Promise<void>): Promise<void> {
    try {
      await action();
    } catch (error) {
      if (error instanceof BackendError) this.failed.emit(error);
      else throw error;
    }
  }
}
