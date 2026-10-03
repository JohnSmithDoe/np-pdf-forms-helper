// ─── why ────────────────────────────────────────────────────────
// The cells that could not be read at all. Each one blocks the file until the
// user either types what was meant or says it stays empty — there is no third
// way, because a guess here is exactly the silent auto-resolution the import is
// built to refuse.
//
// The input starts with the ORIGINAL text rather than empty: most Fehler are
// one wrong character (`31.13.2025`, a `O` for a `0`), and retyping the whole
// value to fix one is how a second typo gets in.
//
// A correction is sent as typed and the BACKEND decides whether it now reads —
// the parser lives in Rust and a TypeScript pre-check would be a second one that
// drifts. One that still fails comes back open with the correction echoed, which
// is what the "Ihre Eingabe" line is for.
//
// The input's text is read off the element on click rather than mirrored into a
// signal per cell: nothing renders from it until it is sent, and a signal per
// row would be state with no reader.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  input,
  output,
} from '@angular/core';
import {
  IonButton,
  IonIcon,
  IonInput,
  IonItem,
  IonItemGroup,
  IonLabel,
  IonNote,
} from '@ionic/angular/standalone';
import { addIcons } from 'ionicons';
import { alertCircleOutline, checkmarkCircleOutline } from 'ionicons/icons';
import type { FehlerCell } from '../../model/trains.types';

export interface CellCorrection {
  row: number;
  column: number;
  value: string;
}

@Component({
  selector: 'app-fehler-list',
  templateUrl: 'fehler-list.component.html',
  styleUrls: ['fehler-list.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    IonButton,
    IonIcon,
    IonInput,
    IonItem,
    IonItemGroup,
    IonLabel,
    IonNote,
  ],
})
export class FehlerListComponent {
  readonly fehler = input.required<FehlerCell[]>();

  readonly correct = output<CellCorrection>();

  constructor() {
    addIcons({ alertCircleOutline, checkmarkCircleOutline });
  }

  protected apply(
    cell: FehlerCell,
    value: string | number | null | undefined
  ): void {
    this.correct.emit({
      row: cell.row,
      column: cell.column,
      value: value === null || value === undefined ? '' : String(value),
    });
  }

  protected leaveEmpty(cell: FehlerCell): void {
    this.correct.emit({ row: cell.row, column: cell.column, value: '' });
  }
}
