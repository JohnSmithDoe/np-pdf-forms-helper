// ─── why ────────────────────────────────────────────────────────
// `ListPageFacade` over the maintenance events, and the ONLY list that pages.
// The others hold their whole set; events do not fit in a message, so
// `query_events` serves a page and `loadMore` asks for the next one.
//
// That is also the one thing np-commlink's pattern did not cover — every list
// there is a local array — so `loadMore` is new code hung on the cloned shell
// rather than something copied.
//
// Searching and sorting run over what has been LOADED, not over the store. That
// is honest for a page-at-a-time list and the reason the header shows the total:
// the user can see there is more behind what they are filtering.
//
// The row carries its RENDERED date rather than the template calling a method
// per row: an `@for` body re-runs every change-detection pass, and this list
// grows to hundreds of rows by scrolling. An undated event is not older than
// everything, it is unplaced — so it says so rather than showing a blank where a
// date belongs.
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
import { formatCents, formatIsoDate, formatUic } from '../util/uic.utility';
import { TrainsFacade } from './trains.facade';
import { TrainsStore } from './trains.store';

export interface EventRow extends BaseItem {
  date: string;
  displayDate: string;
  wagen: string;
  werkstatt: string;
  amount: string;
  kind: string;
}

@Injectable({ providedIn: 'root' })
export class EventListFacade implements ListPageFacade {
  readonly #store = inject(TrainsStore);
  readonly #trains = inject(TrainsFacade);
  readonly #term = signal<string | undefined>(undefined);
  readonly #sort = signal<ItemListSort>({
    sortBy: 'datum',
    sortDirection: 'desc',
  });

  readonly activeSort = this.#sort.asReadonly();
  readonly total = this.#store.eventTotal;
  readonly sortOptions = signal<readonly ItemListSortOption[]>([
    { key: 'datum', label: 'Datum' },
    { key: 'betrag', label: 'Betrag' },
  ]).asReadonly();

  readonly #rows = computed<EventRow[] | undefined>(() => {
    const events = this.#store.events();
    if (!events) return;
    const wagenById = this.#store.wagenById();
    const partners = this.#store.partnerById();
    return events.map((event) => {
      const wagen = wagenById.get(event.wagenId);
      const date = event.datum ?? '';
      return {
        id: event.id,
        name: event.leistung,
        kind: event.leistung,
        date,
        displayDate: date ? formatIsoDate(date) : 'ohne Datum',
        wagen: wagen
          ? formatUic(wagen.nummer, this.#store.settings().wagennummer)
          : '',
        werkstatt: event.werkstattId
          ? (partners.get(event.werkstattId)?.name ?? '')
          : '',
        amount: formatCents(event.betragCent),
      };
    });
  });

  readonly searchResult = computed(() =>
    searchList(
      this.#rows() ?? [],
      this.#term(),
      (row) => `${row.wagen} ${row.werkstatt} ${row.kind}`
    )
  );

  readonly items = computed<BaseItem[] | undefined>(() => {
    const rows = this.#rows();
    if (!rows) return;
    const found = this.searchResult()?.items ?? rows;
    return sortList(found, this.#sort(), (row, key) =>
      key === 'betrag' ? row.amount.replace(',', '.') : row.date
    );
  });

  search(term?: string): void {
    this.#term.set(term);
  }

  setSortMode(key: string): void {
    this.#sort.set(toggleSort(this.#sort(), key));
  }

  loadMore(): void {
    const loaded = this.#store.events()?.length ?? 0;
    if (loaded >= this.#store.eventTotal()) return;
    void this.#trains.loadEvents(loaded);
  }
}
