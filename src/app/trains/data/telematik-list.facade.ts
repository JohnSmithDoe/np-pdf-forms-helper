// ─── why ────────────────────────────────────────────────────────
// `ListPageFacade` over the Telematik list Rust builds (`get_telematik`).
// Unlike the other list facades it composes nothing from the store: the rows,
// their order (longest silent first) and every string are Rust's, so this only
// holds the view and searches it. No sort options, because the order IS the
// view — the silent Wagen are what the list is opened for.
//
// The view is held here and not in the store: it is derived from the
// Wagen-Zustand and a clock, so the page reloads it whenever the store's
// `zustand` changes rather than keeping a copy that would go stale.
// ────────────────────────────────────────────────────────────────

import { computed, inject, Injectable, signal } from '@angular/core';
import type {
  BaseItem,
  ItemListSort,
} from '../../@shared/model/item-list.types';
import type { ListPageFacade } from '../../@shared/util/item-lists/list-page.facade';
import { searchList } from '../../@shared/util/item-lists/list.selector';
import type { TelematikRow, TelematikView } from '../model/trains.types';
import { TrainsFacade } from './trains.facade';

export type TelematikListRow = TelematikRow & BaseItem;

@Injectable({ providedIn: 'root' })
export class TelematikListFacade implements ListPageFacade {
  readonly #trains = inject(TrainsFacade);
  readonly #view = signal<TelematikView | undefined>(undefined);
  readonly #term = signal<string | undefined>(undefined);

  readonly sort = signal<ItemListSort | undefined>(undefined).asReadonly();
  readonly stumm = computed(() => this.#view()?.stumm);
  readonly count = computed(() => this.#view()?.rows.length);

  readonly #rows = computed<TelematikListRow[] | undefined>(() =>
    this.#view()?.rows.map((row) => ({
      ...row,
      id: row.wagenId,
      name: row.title,
    }))
  );

  readonly searchResult = computed(() =>
    searchList(
      this.#rows() ?? [],
      this.#term(),
      (row) => `${row.title} ${row.nummer} ${row.geraet ?? ''} ${row.standort}`
    )
  );

  readonly items = computed<BaseItem[] | undefined>(() => {
    const rows = this.#rows();
    if (!rows) return undefined;
    return this.searchResult()?.items ?? rows;
  });

  async load(): Promise<void> {
    this.#view.set(await this.#trains.telematik());
  }

  searchWagen(wagenId: string): void {
    const row = this.#view()?.rows.find((r) => r.wagenId === wagenId);
    if (row) this.search(row.nummer);
  }

  search(term?: string): void {
    this.#term.set(term);
  }

  setSortMode(): void {
    return;
  }
}
