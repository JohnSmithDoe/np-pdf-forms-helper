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
// ────────────────────────────────────────────────────────────────

import { computed, inject, Injectable, signal } from '@angular/core';
import type {
  BaseItem,
  ItemListSort,
  ItemListSortOption,
} from '../../@shared/model/item-list.types';
import type { ListPageFacade } from '../../@shared/util/item-lists/list-page.facade';
import {
  searchList,
  sortList,
  toggleSort,
} from '../../@shared/util/item-lists/list.selector';
import type { Einbau } from '../model/trains.types';
import { formatIsoDate, formatUic } from '../util/uic.util';
import { TrainsStore } from './trains.store';

export interface WagenRow extends BaseItem {
  nummer: string;
  digits: string;
  owner: string;
  kind: string;
  fitted: FittedRadsatz[];
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

  readonly sort = this.#sort.asReadonly();
  readonly sortOptions = signal<readonly ItemListSortOption[]>([
    { key: 'wagennummer', label: 'Nummer' },
    { key: 'halter', label: 'Eigentümer' },
  ]).asReadonly();

  readonly #rows = computed<WagenRow[] | undefined>(() => {
    const wagen = this.#store.wagen();
    if (!wagen) return undefined;
    const partners = this.#store.partnerById();
    const radsaetze = this.#store.radsatzById();
    const open = new Map<string, Einbau[]>();
    for (const einbau of this.#store.einbauten() ?? []) {
      if (einbau.ausgebautAm) continue;
      open.set(einbau.wagenId, [...(open.get(einbau.wagenId) ?? []), einbau]);
    }
    return wagen.map((wagen) => {
      const uic = formatUic(wagen.nummer, this.#store.settings().wagennummer);
      return {
        id: wagen.id,
        name: uic,
        nummer: uic,
        digits: wagen.nummer,
        owner: wagen.halterId ? (partners.get(wagen.halterId)?.name ?? '') : '',
        kind: wagen.bauart ?? '',
        fitted: [...(open.get(wagen.id) ?? [])]
          .sort(byPosition)
          .map((einbau) => ({
            radsatz: radsaetze.get(einbau.radsatzId)?.nummer ?? '',
            position: einbau.position ?? '',
            since: einbau.eingebautAm ? formatIsoDate(einbau.eingebautAm) : '',
            source: `${einbau.source.sheet}, Zeile ${einbau.source.row}`,
          })),
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
        `${row.nummer} ${row.digits} ${row.owner} ${row.kind} ${row.fitted
          .map((entry) => entry.radsatz)
          .join(' ')}`
    )
  );

  readonly items = computed<BaseItem[] | undefined>(() => {
    const rows = this.#rows();
    if (!rows) return undefined;
    const found = this.searchResult()?.items ?? rows;
    return sortList(found, this.#sort(), (row, key) =>
      key === 'halter' ? row.owner : row.nummer
    );
  });

  search(term?: string): void {
    this.#term.set(term);
  }

  setSortMode(key: string): void {
    this.#sort.set(toggleSort(this.#sort(), key));
  }
}

function byPosition(left: Einbau, right: Einbau): number {
  if (!left.position || !right.position) {
    return Number(!left.position) - Number(!right.position);
  }
  return left.position.localeCompare(right.position, 'de', { numeric: true });
}
