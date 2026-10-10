// ─── why ────────────────────────────────────────────────────────
// The seam every list page hangs on. A page depends on this INTERFACE and not on
// whatever holds the data, which is what let np-commlink's NgRx-backed pattern
// come across to a signalStore over a Rust backend without the shell changing.
//
// Members are optional where their ABSENCE is the declaration: a facade with no
// `sortOptions` renders no sort bar, and one with no `create` renders no add
// button. That is why `create` must not live on a base class — presence is read
// as `!!facade.create`, and an inherited no-op would make every list claim it.
//
// `columns` is the same kind of declaration: a facade that names columns gets
// Excel's filter dialog per column INSTEAD of the sort bar — the dialog sorts
// too, and two controls for one sort would disagree about which was pressed
// last. Its four members come together or not at all.
// ────────────────────────────────────────────────────────────────

import { InjectionToken, Signal } from '@angular/core';
import type {
  BaseItem,
  ColumnChoices,
  ColumnFilter,
  ColumnFilters,
  ItemListSort,
  ListColumn,
  ItemListSortOption,
  SearchResult,
} from '../../model/item-list.types';

export interface ListPageFacade {
  readonly items: Signal<BaseItem[] | undefined>;
  readonly searchResult: Signal<SearchResult<BaseItem> | undefined>;
  readonly activeSort: Signal<ItemListSort | undefined>;
  readonly sortOptions?: Signal<readonly ItemListSortOption[]>;
  readonly total?: Signal<number>;
  readonly columns?: Signal<readonly ListColumn[]>;
  readonly filters?: Signal<ColumnFilters>;

  search(term?: string): void;
  setSortMode(key: string): void;
  create?(): void;
  loadMore?(): void;
  columnChoices?(key: string): ColumnChoices;
  setFilter?(key: string, filter: ColumnFilter | undefined): void;
  setSort?(sort: ItemListSort): void;
}

export const LIST_FACADE = new InjectionToken<ListPageFacade>('LIST_FACADE');
