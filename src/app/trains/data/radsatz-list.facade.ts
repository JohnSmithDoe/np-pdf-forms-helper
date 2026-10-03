// ─── why ────────────────────────────────────────────────────────
// `ListPageFacade` over the radsaetze. Like the wagen there is no `create`: a
// radsatz is born from an import, where its number was read off a document.
//
// The sender's system id is searchable but not shown: a user holding that
// sender's report has the id in hand, and the row has no room for a second
// identifier nobody else uses.
//
// The row shows where it is NOW, derived from its open einbau — a radsatz is
// under at most one wagen, so "no open einbau" reads as ausgebaut rather than
// as missing data.
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

export interface RadsatzRow extends BaseItem {
  number: string;
  systemId: string;
  fittedTo: string;
  since: string;
  history: {
    wagen: string;
    installed: string;
    removed: string;
    position: string;
  }[];
}

@Injectable({ providedIn: 'root' })
export class RadsatzListFacade implements ListPageFacade {
  readonly #store = inject(TrainsStore);
  readonly #term = signal<string | undefined>(undefined);
  readonly #sort = signal<ItemListSort>({
    sortBy: 'number',
    sortDirection: 'asc',
  });

  readonly sort = this.#sort.asReadonly();
  readonly sortOptions = signal<readonly ItemListSortOption[]>([
    { key: 'number', label: 'Nummer' },
    { key: 'fittedTo', label: 'Wagen' },
  ]).asReadonly();

  readonly #rows = computed<RadsatzRow[] | undefined>(() => {
    const radsaetze = this.#store.radsaetze();
    if (!radsaetze) return undefined;
    const einbauten = this.#store.einbauten() ?? [];
    const wagenById = this.#store.wagenById();
    const wagenLabel = (id: string): string => {
      const wagen = wagenById.get(id);
      return wagen
        ? formatUic(wagen.nummer, this.#store.settings().wagennummer)
        : '';
    };

    return radsaetze.map((radsatz) => {
      const own = einbauten.filter((m) => m.radsatzId === radsatz.id);
      const open = own.find((m) => !m.ausgebautAm);
      return {
        id: radsatz.id,
        name: radsatz.nummer,
        number: radsatz.nummer,
        systemId: radsatz.systemId ?? '',
        fittedTo: open ? wagenLabel(open.wagenId) : '',
        since: open?.eingebautAm ? formatIsoDate(open.eingebautAm) : '',
        history: [...own].sort(byNewest).map((einbau) => ({
          wagen: wagenLabel(einbau.wagenId),
          installed: einbau.eingebautAm
            ? formatIsoDate(einbau.eingebautAm)
            : '',
          removed: einbau.ausgebautAm ? formatIsoDate(einbau.ausgebautAm) : '',
          position: einbau.position ?? '',
        })),
      };
    });
  });

  readonly searchResult = computed(() =>
    searchList(
      this.#rows() ?? [],
      this.#term(),
      (row) => `${row.number} ${row.systemId} ${row.fittedTo}`
    )
  );

  readonly items = computed<BaseItem[] | undefined>(() => {
    const rows = this.#rows();
    if (!rows) return undefined;
    const found = this.searchResult()?.items ?? rows;
    return sortList(found, this.#sort(), (row, key) =>
      key === 'fittedTo' ? row.fittedTo : row.number
    );
  });

  search(term?: string): void {
    this.#term.set(term);
  }

  setSortMode(key: string): void {
    this.#sort.set(toggleSort(this.#sort(), key));
  }
}

function byNewest(left: Einbau, right: Einbau): number {
  const key = (einbau: Einbau): string =>
    einbau.eingebautAm ?? einbau.ausgebautAm ?? '';
  return key(right).localeCompare(key(left));
}
