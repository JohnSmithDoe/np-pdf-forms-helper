// ─── why ────────────────────────────────────────────────────────
// A master sheet as a plain table, read-only, laid out like the workbook so
// the two can be compared by eye: the column letter above the sheet's own
// header, columns in sheet order, empty headers kept.
//
// Every cell arrives display-ready from Rust, so nothing here formats or
// decides. A column no entity fills yet is marked in its header, not hidden:
// an empty column has to read as "not in the Schattensystem", never as "the
// customer left it blank".
//
// A plain `<table>` rather than Ionic rows: a sheet is a grid, up to 38 columns
// wide, and an `ion-item` per row would be a list, not a grid.
// ────────────────────────────────────────────────────────────────

import { ChangeDetectionStrategy, Component, input } from '@angular/core';
import { IonNote } from '@ionic/angular/standalone';
import type { SheetColumn, SheetRow } from '../../../model/trains.types';
import { columnLetter } from '../../util/column-letter.util';

@Component({
  selector: 'app-sheet-table',
  templateUrl: 'sheet-table.component.html',
  styleUrls: ['sheet-table.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [IonNote],
})
export class SheetTableComponent {
  readonly columns = input.required<SheetColumn[]>();
  readonly rows = input.required<SheetRow[]>();

  protected readonly letter = columnLetter;
}
