// ─── why ────────────────────────────────────────────────────────
// What one sheet of the master update changes, by ROW — the client reads the
// master by rows, never by cells. On the preview before anything is written
// and on the result after: one component for both, so what the user approved
// and what was written read the same.
//
// Every row arrives whole and display-ready from Rust (`export/diff.rs`), in
// every column of the sheet under its letter and header, so it compares by
// eye with the open workbook; nothing here formats or decides. A changed cell
// is marked, a new row and an emptied row are marked whole. A click on a
// changed row opens its OLD values as a second row directly underneath,
// column for column — never as a list beside the row, which meant scrolling
// back to the start to read what a cell far right used to hold. The row
// labels stick to the left edge for the same reason. Each row is its own
// `<tbody>`, so the zebra stripe covers a row TOGETHER with its opened old
// row: the pair reads as one, and the eye can follow a row across the full
// width. The stripe is the text colour mixed thin into the background, so it
// follows the theme without naming a colour; the sticky label column layers it
// over its opaque background, or the stripe would break at the left edge. Only a changed row
// opens: a new row had nothing before, an emptied one already shows its old
// values. The open rows are this component's own view state, which is why a
// `ui` component holds a signal.
//
// The marks are Ionic colour ROLES (`ion-color-*` classes, read as
// `--ion-color-base` in the stylesheet), as `entity-card` does it: the
// colour comes from the theme, dark mode included, and nothing here names one.
// The one literal is the sticky label column's `#fff`: it must be OPAQUE or
// scrolled cells show through it, and the light theme leaves
// `--ion-background-color` unset — `#fff` is Ionic's own fallback, written
// where `ion-card` writes it, so the column matches the card it sits in.
//
// Rust sends EVERY row, uncapped — the preview is what will be written. Only
// the rendering is paged: `PAGE` rows at first, more as the user scrolls
// (`ion-infinite-scroll`, loading from memory, nothing fetched). No virtual
// scrolling: rows once shown stay in the DOM, so an opened row stays open and
// the browser's own find (Ctrl+F) reaches everything shown. A new dry run
// replaces `rows` and starts the paging over (`linkedSignal`).
// A plain `<table>` for the same reason as `sheet-table`: a sheet is a grid.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  computed,
  input,
  linkedSignal,
  signal,
} from '@angular/core';
import {
  IonInfiniteScroll,
  IonInfiniteScrollContent,
  IonNote,
} from '@ionic/angular/standalone';
import type {
  ChangeColumn,
  RowChange,
  RowChangeStatus,
} from '../../../model/trains.types';
import { columnLetter } from '../../util/column-letter.utility';

const PAGE = 100;

const STATUS: Record<RowChangeStatus, { label: string; color: string }> = {
  geaendert: { label: 'geändert', color: 'warning' },
  neu: { label: 'neu', color: 'success' },
  geleert: { label: 'geleert', color: 'danger' },
};

@Component({
  selector: 'app-row-changes',
  templateUrl: 'row-changes.component.html',
  styleUrls: ['row-changes.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [IonInfiniteScroll, IonInfiniteScrollContent, IonNote],
})
export class RowChangesComponent {
  readonly columns = input.required<ChangeColumn[]>();
  readonly rows = input.required<RowChange[]>();

  protected readonly letter = columnLetter;
  protected readonly status = STATUS;
  protected readonly open = signal<ReadonlySet<number>>(new Set());
  protected readonly shown = linkedSignal(() =>
    Math.min(PAGE, this.rows().length)
  );
  protected readonly visible = computed(() =>
    this.rows().slice(0, this.shown())
  );

  protected onMore(event: Event): void {
    this.shown.update((shown) => shown + PAGE);
    void (event.target as HTMLIonInfiniteScrollElement).complete();
  }

  protected toggle(row: number): void {
    this.open.update((open) => {
      const next = new Set(open);
      if (!next.delete(row)) next.add(row);
      return next;
    });
  }

  protected mark(row: RowChange, changed: boolean): string {
    const gone = row.status === 'geleert' ? ' row-changes__gone' : '';
    if (row.status === 'geaendert' && !changed) return gone;
    return `row-changes__mark ion-color-${STATUS[row.status].color}${gone}`;
  }
}
