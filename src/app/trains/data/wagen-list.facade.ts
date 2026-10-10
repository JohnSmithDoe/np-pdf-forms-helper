// ─── why ────────────────────────────────────────────────────────
// `ListPageFacade` over the wagen. `create` is absent on purpose: a wagen is
// born from an import, where its number has been read and check-digit tested.
// A hand-typed one would be the phantom entity the whole resolve layer exists to
// prevent.
//
// `fitted` is what is under the wagen NOW — its open Einbauten, by position.
// A master radsatz sheet without positions adds open Einbauten with none, and
// a wagen whose old set was never booked out carries five to eight; both are
// shown as they are, „ohne Position“ last, because hiding them would make the
// mirror look cleaner than the customer's file. `source` names the sheet and
// row each came from, which is what tells those cases apart.
//
// The Wagen-Zustand rides on the row as `zustand` — where it is, when it last
// reported, what is open — built by `util/wagen-zustand` from the store's one
// `WagenZustand`, so the list and its search read the same text.
//
// Filtering is Excel's, per column (`columns`), and replaces the sort bar. The
// owner column is labelled Halter: it reads `halterId`, and the Halter is not
// the Eigentümer (docs/fachdomaene.md). Each row carries its Farbe —
// `util/farbe` — so the colour filter and sort work on every column.
// ────────────────────────────────────────────────────────────────

import { computed, inject, Injectable, signal } from '@angular/core';
import type {
  BaseItem,
  ColumnChoices,
  ColumnFilter,
  ColumnFilters,
  ItemListSort,
  ListColumn,
} from '../../@shared/model/item-list.types';
import {
  columnChoices,
  filterList,
  withFilter,
} from '../../@shared/util/item-lists/list-filter';
import type { ListPageFacade } from '../../@shared/util/item-lists/list-page.facade';
import {
  searchList,
  sortList,
  toggleSort,
} from '../../@shared/util/item-lists/list.selector';
import type { Einbau } from '../model/trains.types';
import { farbeOf } from '../util/farbe.util';
import { formatIsoDate, formatUic } from '../util/uic.util';
import {
  indexZustand,
  summarise,
  type ZustandSummary,
} from '../util/wagen-zustand.util';
import { TrainsStore } from './trains.store';

export interface WagenRow extends BaseItem {
  nummer: string;
  digits: string;
  owner: string;
  kind: string;
  summary: string;
  fitted: FittedRadsatz[];
  zustand: ZustandSummary;
}

export interface FittedRadsatz {
  radsatz: string;
  position: string;
  since: string;
  source: string;
}

@Injectable({ providedIn: 'root' })
export class WagenListFacade implements ListPageFacade {
  readonly #store = inject(TrainsStore);
  readonly #term = signal<string | undefined>(undefined);
  readonly #sort = signal<ItemListSort>({
    sortBy: 'wagennummer',
    sortDirection: 'asc',
  });

  readonly #filters = signal<ColumnFilters>({});

  readonly sort = this.#sort.asReadonly();
  readonly filters = this.#filters.asReadonly();
  readonly columns = signal<readonly ListColumn[]>([
    { key: 'wagennummer', label: 'Wagennummer' },
    { key: 'bauart', label: 'Bauart' },
    { key: 'halter', label: 'Halter' },
    { key: 'standort', label: 'Standort' },
  ]).asReadonly();

  readonly #rows = computed<WagenRow[] | undefined>(() => {
    const wagen = this.#store.wagen();
    if (!wagen) return undefined;
    const partners = this.#store.partnerById();
    const radsaetze = this.#store.radsatzById();
    const zustand = indexZustand(this.#store.zustand());
    const markierungen = this.#store.markierungen();
    const now = new Date();
    const open = new Map<string, Einbau[]>();
    for (const einbau of this.#store.einbauten() ?? []) {
      if (einbau.ausgebautAm) continue;
      open.set(einbau.wagenId, [...(open.get(einbau.wagenId) ?? []), einbau]);
    }
    return wagen.map((wagen) => {
      const uic = formatUic(wagen.nummer, this.#store.settings().wagennummer);
      const owner = wagen.halterId
        ? (partners.get(wagen.halterId)?.name ?? '')
        : '';
      return {
        id: wagen.id,
        name: uic,
        farbe: farbeOf(markierungen, 'wagen', wagen.nummer).farbe,
        nummer: uic,
        digits: wagen.nummer,
        owner,
        kind: wagen.bauart ?? '',
        summary: [wagen.bauart, owner].filter(Boolean).join(' · '),
        fitted: [...(open.get(wagen.id) ?? [])]
          .sort(byPosition)
          .map((einbau) => ({
            radsatz: radsaetze.get(einbau.radsatzId)?.nummer ?? '',
            position: einbau.position ?? '',
            since: einbau.eingebautAm ? formatIsoDate(einbau.eingebautAm) : '',
            source: `${einbau.source.sheet}, Zeile ${einbau.source.row}`,
          })),
        zustand: summarise(wagen.id, zustand, now),
      };
    });
  });

  readonly searchResult = computed(() =>
    searchList(
      this.#rows() ?? [],
      this.#term(),
      // Both forms: people type `3180` and read `31 80 4740 123-4`, and a
      // search that only knows the grouped one misses every plain-digit query.
      (row) =>
        `${row.nummer} ${row.digits} ${row.owner} ${row.kind} ${row.zustand.standort} ${row.fitted
          .map((entry) => entry.radsatz)
          .join(' ')}`
    )
  );

  readonly #found = computed(
    () => this.searchResult()?.items ?? this.#rows() ?? []
  );

  readonly items = computed<BaseItem[] | undefined>(() => {
    if (!this.#rows()) return undefined;
    return sortList(
      filterList(this.#found(), this.#filters(), valueOf),
      this.#sort(),
      valueOf
    );
  });

  search(term?: string): void {
    this.#term.set(term);
  }

  setSortMode(key: string): void {
    this.#sort.set(toggleSort(this.#sort(), key));
  }

  setSort(sort: ItemListSort): void {
    this.#sort.set(sort);
  }

  columnChoices(key: string): ColumnChoices {
    return columnChoices(this.#found(), this.#filters(), key, valueOf);
  }

  setFilter(key: string, filter: ColumnFilter | undefined): void {
    this.#filters.set(withFilter(this.#filters(), key, filter));
  }
}

function valueOf(row: WagenRow, key: string): string {
  switch (key) {
    case 'bauart':
      return row.kind;
    case 'halter':
      return row.owner;
    case 'standort':
      return row.zustand.standort;
    default:
      return row.nummer;
  }
}

function byPosition(left: Einbau, right: Einbau): number {
  if (!left.position || !right.position) {
    return Number(!left.position) - Number(!right.position);
  }
  return left.position.localeCompare(right.position, 'de', { numeric: true });
}
