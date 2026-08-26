// ─── why ────────────────────────────────────────────────────────
// `ListPageFacade` over the wagen. `create` is absent on purpose: a wagen is
// born from an import, where its number has been read and check-digit tested.
// A hand-typed one would be the phantom entity the whole resolve layer exists to
// prevent.
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
import { formatUic } from '../util/uic.util';
import { TrainsStore } from './trains.store';

export interface WagenRow extends BaseItem {
  nummer: string;
  digits: string;
  owner: string;
  kind: string;
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
    return wagen.map((wagen) => {
      const uic = formatUic(wagen.nummer);
      return {
        id: wagen.id,
        name: uic,
        nummer: uic,
        digits: wagen.nummer,
        owner: wagen.halterId ? (partners.get(wagen.halterId)?.name ?? '') : '',
        kind: wagen.bauart ?? '',
      };
    });
  });

  readonly searchResult = computed(() =>
    searchList(
      this.#rows() ?? [],
      this.#term(),
      // Both forms: people type `3180` and read `31 80 4740 123-4`, and a
      // search that only knows the grouped one misses every plain-digit query.
      (row) => `${row.nummer} ${row.digits} ${row.owner} ${row.kind}`
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
