// ─── why ────────────────────────────────────────────────────────
// Excel's AutoFilter dialog for one column, because the users live in Excel and
// already know where everything in it is: Sortieren (auf/ab, Nach Farbe), then
// Filter (Nach Farbe, a searchable value checklist), and at
// the foot „Automatisch anwenden“ with „Filter anwenden“ / „Filter entfernen“.
// It only emits; the facade holds the state, so closing and reopening shows what
// is applied, not what was half-edited.
//
// The colours are `ion-chip`s and not an `ion-select`: a chip carries its colour
// role, so the choice is SEEN like Excel's swatches, and a select option cannot be
// coloured at all.
//
// The draft is `linkedSignal` over the applied filter — with „Automatisch
// anwenden“ off, edits stay local until „Filter anwenden“.
//
// The SEARCH is the filter, as in Excel: every change of the search text starts
// the selection over — exactly what it finds is ticked, and an emptied search
// ticks everything again — so an earlier search never leaves values unticked
// behind it. Unticking narrows further, and the ticked values are what is
// applied — there is no separate value box. The condition („Enthält“, „Beginnt mit“ …) steers
// how the search matches; it is not stored, the ticked values say it all.
// A search that finds nothing changes nothing, or one typo would empty the
// list. „(Alles auswählen)“ acts on what the search shows. A full tick goes out
// as no `values` at all, or a value arriving later would be filtered away
// unasked.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  computed,
  input,
  linkedSignal,
  output,
  signal,
} from '@angular/core';
import {
  IonButton,
  IonButtons,
  IonCheckbox,
  IonChip,
  IonContent,
  IonFooter,
  IonHeader,
  IonIcon,
  IonItem,
  IonLabel,
  IonList,
  IonListHeader,
  IonSearchbar,
  IonSelect,
  IonSelectOption,
  IonTitle,
  IonToolbar,
} from '@ionic/angular/standalone';
import { addIcons } from 'ionicons';
import { arrowDownOutline, arrowUpOutline } from 'ionicons/icons';
import type { Farbe } from '../../../model/farbe.types';
import type {
  ColumnChoices,
  ColumnFilter,
  FilterOp,
  ItemListSort,
  SortDirection,
} from '../../../model/item-list.types';
import { FARBE_COLOR, FARBE_LABEL } from '../../../util/farbe.util';
import { matches } from '../../../util/item-lists/list-filter';
import { inputValue } from '../../../util/input-value.util';

const OPS: readonly { op: FilterOp; label: string }[] = [
  { op: 'gleich', label: 'Ist gleich' },
  { op: 'ungleich', label: 'Ist nicht gleich' },
  { op: 'beginnt', label: 'Beginnt mit' },
  { op: 'endet', label: 'Endet mit' },
  { op: 'enthaelt', label: 'Enthält' },
  { op: 'enthaeltNicht', label: 'Enthält nicht' },
];

@Component({
  selector: 'app-column-filter',
  templateUrl: 'column-filter.component.html',
  styleUrls: ['column-filter.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    IonButton,
    IonButtons,
    IonCheckbox,
    IonChip,
    IonContent,
    IonFooter,
    IonHeader,
    IonIcon,
    IonItem,
    IonLabel,
    IonList,
    IonListHeader,
    IonSearchbar,
    IonSelect,
    IonSelectOption,
    IonTitle,
    IonToolbar,
  ],
})
export class ColumnFilterComponent {
  readonly column = input.required<string>();
  readonly label = input.required<string>();
  readonly choices = input.required<ColumnChoices>();
  readonly filter = input<ColumnFilter>();
  readonly activeSort = input<ItemListSort>();

  readonly sortChange = output<ItemListSort>();
  readonly filterChange = output<ColumnFilter | undefined>();
  readonly closed = output<void>();

  protected readonly ops = OPS;
  protected readonly farbeLabel = FARBE_LABEL;
  protected readonly farbeColor = FARBE_COLOR;

  protected readonly auto = signal(true);
  protected readonly term = signal('');
  protected readonly values = linkedSignal(() => this.filter()?.values);
  protected readonly farbe = linkedSignal(() => this.filter()?.farbe);
  protected readonly op = signal<FilterOp>('enthaelt');

  protected readonly sortedHere = computed(() =>
    this.activeSort()?.sortBy === this.column() ? this.activeSort() : undefined
  );
  protected readonly searching = computed(() => !!this.term().trim());
  protected readonly visible = computed(() =>
    this.choices().values.filter((value) =>
      matches(value, this.op(), this.term())
    )
  );
  protected readonly allTicked = computed(() =>
    this.visible().every((value) => this.isTicked(value))
  );
  protected readonly someTicked = computed(
    () =>
      !this.allTicked() && this.visible().some((value) => this.isTicked(value))
  );

  constructor() {
    addIcons({ arrowDownOutline, arrowUpOutline });
  }

  protected isTicked(value: string): boolean {
    return this.values()?.includes(value) ?? true;
  }

  protected onSort(direction: SortDirection): void {
    this.sortChange.emit({
      sortBy: this.column(),
      sortDirection: direction,
      farbe: this.sortedHere()?.farbe,
    });
  }

  protected onSortFarbe(farbe: Farbe | undefined): void {
    this.sortChange.emit({
      sortBy: this.column(),
      sortDirection: this.sortedHere()?.sortDirection ?? 'asc',
      farbe,
    });
  }

  protected onFarbe(farbe: Farbe | 'keine' | undefined): void {
    this.farbe.set(farbe);
    this.#changed();
  }

  protected onOp(event: Event): void {
    this.op.set((event as CustomEvent<{ value: FilterOp }>).detail.value);
    this.#takeSearch();
  }

  protected onSearch(event: Event): void {
    this.term.set(inputValue(event));
    this.#takeSearch();
  }

  protected onToggle(value: string): void {
    const ticked = new Set(this.values() ?? this.choices().values);
    if (ticked.has(value)) ticked.delete(value);
    else ticked.add(value);
    this.#setValues(ticked);
  }

  protected onToggleAll(): void {
    const ticked = new Set(this.values() ?? this.choices().values);
    const tick = !this.allTicked();
    for (const value of this.visible()) {
      if (tick) ticked.add(value);
      else ticked.delete(value);
    }
    this.#setValues(ticked);
  }

  protected onAuto(event: Event): void {
    this.auto.set((event as CustomEvent<{ checked: boolean }>).detail.checked);
  }

  protected onApply(): void {
    this.filterChange.emit(this.#draft());
  }

  protected onClear(): void {
    this.values.set(undefined);
    this.farbe.set(undefined);
    this.term.set('');
    this.filterChange.emit(undefined);
  }

  #takeSearch(): void {
    if (!this.searching()) {
      this.values.set(undefined);
      this.#changed();
      return;
    }
    const found = this.visible();
    if (found.length > 0) this.#setValues(new Set(found));
  }

  #setValues(ticked: Set<string>): void {
    const all = this.choices().values;
    const everything =
      ticked.size >= all.length && all.every((value) => ticked.has(value));
    this.values.set(everything ? undefined : [...ticked]);
    this.#changed();
  }

  #changed(): void {
    if (this.auto()) this.filterChange.emit(this.#draft());
  }

  #draft(): ColumnFilter {
    return { values: this.values(), farbe: this.farbe() };
  }
}
