// ─── why ────────────────────────────────────────────────────────
// Stage three: what came out, one line per row, with the raw text beside it.
//
// A row that needs a decision is UNTICKED until the user makes it. Nothing is
// created silently — that is how a typo'd wagen number becomes a permanent
// phantom every later import then matches against.
//
// A duplicate is shown rather than hidden, and unticked. The user may
// legitimately have two identical repairs, and a row that vanished would look
// like the import lost it.
//
// The status chips filter rather than navigate, because the question at this
// stage is "what still needs me", and the answer is a subset of one list.
//
// `viewRows` builds the whole row — its shown cells and its four entity slots —
// ONCE per staging, because everything in an `@for` body re-runs on every
// change-detection pass and a preview is thousands of rows. What stays a method
// is only what allocates nothing and depends on `choices()`: a checkbox tick
// must not rebuild every row.
//
// The entity slots are a typed constant rather than a string literal in the
// template. The literal cost three `$any` casts and a nested ternary for the
// label, which is a renamed field on `StagedRow` failing silently at runtime
// instead of at build.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
} from '@angular/core';
import {
  IonBadge,
  IonButton,
  IonCheckbox,
  IonChip,
  IonItem,
  IonLabel,
  IonList,
  IonNote,
  IonSelect,
  IonSelectOption,
} from '@ionic/angular/standalone';
import { ImportFacade } from '../../data';
import type {
  MatchCandidate,
  Resolution,
  RowStatus,
  StagedCell,
  StagedRow,
} from '../../model/trains.types';

type EntityKey = 'wagen' | 'werkstatt' | 'halter' | 'eigentuemer' | 'radsatz';

interface EntitySlot {
  key: EntityKey;
  label: string;
  needsChoice: boolean;
  candidates: MatchCandidate[];
  text: string;
}

interface ViewRow {
  row: StagedRow;
  cells: StagedCell[];
  entities: EntitySlot[];
}

const ENTITIES: readonly { key: EntityKey; label: string }[] = [
  { key: 'wagen', label: 'Wagen' },
  { key: 'werkstatt', label: 'Werkstatt' },
  { key: 'halter', label: 'Halter' },
  { key: 'eigentuemer', label: 'Eigentümer' },
  { key: 'radsatz', label: 'Radsatz' },
];

const STATUS_LABELS: Record<RowStatus, string> = {
  ready: 'bereit',
  needsInput: 'prüfen',
  duplicate: 'doppelt',
  rejected: 'abgelehnt',
};

const STATUS_COLORS: Record<RowStatus, string> = {
  ready: 'success',
  needsInput: 'warning',
  duplicate: 'medium',
  rejected: 'danger',
};

@Component({
  selector: 'app-row-preview',
  templateUrl: 'row-preview.component.html',
  styleUrls: ['row-preview.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    IonBadge,
    IonButton,
    IonCheckbox,
    IonChip,
    IonItem,
    IonLabel,
    IonList,
    IonNote,
    IonSelect,
    IonSelectOption,
  ],
})
export class RowPreviewComponent {
  protected readonly facade = inject(ImportFacade);
  protected readonly labels = STATUS_LABELS;
  protected readonly colors = STATUS_COLORS;

  protected readonly viewRows = computed<ViewRow[]>(() =>
    this.facade.rows().map((row) => ({
      row,
      cells: row.cells.filter((cell) => cell.field !== 'ignorieren'),
      entities: ENTITIES.flatMap(({ key, label }) => {
        const resolution = row[key];
        if (!resolution || resolution.state === 'missing') return [];
        return [
          {
            key,
            label,
            needsChoice:
              resolution.state === 'ambiguous' || resolution.state === 'new',
            candidates:
              resolution.state === 'ambiguous' ? resolution.candidates : [],
            text: describe(resolution),
          },
        ];
      }),
    }))
  );

  protected readonly counts = computed(() => {
    const summary = this.facade.summary();
    return [
      { status: 'ready' as RowStatus, count: summary?.ready ?? 0 },
      { status: 'needsInput' as RowStatus, count: summary?.needsInput ?? 0 },
      { status: 'duplicate' as RowStatus, count: summary?.duplicates ?? 0 },
      { status: 'rejected' as RowStatus, count: summary?.rejected ?? 0 },
    ];
  });

  protected included(row: StagedRow): boolean {
    return this.facade.choices()[row.row]?.include ?? false;
  }

  protected canInclude(row: StagedRow): boolean {
    return row.status !== 'rejected';
  }

  protected onInclude(row: StagedRow, include: boolean): void {
    this.facade.setChoice(row.row, { include });
  }

  protected onEntity(row: StagedRow, entity: EntityKey, value: string): void {
    this.facade.setChoice(row.row, {
      [entity]:
        value === 'create'
          ? { action: 'create' }
          : { action: 'use', id: value },
    });
  }

  protected entityValue(row: StagedRow, entity: EntityKey): string {
    const choice = this.facade.choices()[row.row]?.[entity];
    return choice?.action === 'use' ? choice.id : 'create';
  }
}

function describe(resolution: Resolution): string {
  switch (resolution.state) {
    case 'known':
      return resolution.name;
    case 'likely':
      return `${resolution.name} — ${resolution.hint}`;
    case 'new':
      return `neu: ${resolution.proposal}`;
    case 'ambiguous':
      return 'mehrdeutig';
    case 'missing':
      return '';
  }
}
