// ─── why ────────────────────────────────────────────────────────
// The cells one sheet of the master export changes — on the preview before
// anything is written, and on the result after. One component for both, so
// what the user approved and what was written read the same.
//
// Rust sends a CAPPED list and the full count: a re-sent snapshot can touch
// thousands of cells, and the table is for reading, not for a grid of them all.
// The rest is said in a line, never dropped silently.
//
// A plain `<table>` for the same reason as `sheet-table`: this is a grid of
// cell, key, column, before and after.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  computed,
  input,
} from '@angular/core';
import { IonNote } from '@ionic/angular/standalone';
import type { CellChange } from '../../../model/trains.types';

@Component({
  selector: 'app-cell-changes',
  templateUrl: 'cell-changes.component.html',
  styleUrls: ['cell-changes.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [IonNote],
})
export class CellChangesComponent {
  readonly changes = input.required<CellChange[]>();
  readonly changed = input.required<number>();

  protected readonly more = computed(
    () => this.changed() - this.changes().length
  );
}
