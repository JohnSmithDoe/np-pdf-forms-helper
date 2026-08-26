// ─── why ────────────────────────────────────────────────────────
// The seam every list page hangs on. A page depends on this INTERFACE and not on
// whatever holds the data, which is what let np-commlink's NgRx-backed pattern
// come across to a signalStore over a Rust backend without the shell changing.
//
// Members are optional where their ABSENCE is the declaration: a facade with no
// `sortOptions` renders no sort bar, and one with no `create` renders no add
// button. That is why `create` must not live on a base class — presence is read
// as `!!facade.create`, and an inherited no-op would make every list claim it.
// ────────────────────────────────────────────────────────────────

import { InjectionToken, Signal } from '@angular/core';
import type {
  BaseItem,
  ItemListSort,
  ItemListSortOption,
  SearchResult,
} from '../../model/item-list.types';

export interface ListPageFacade {
  readonly items: Signal<BaseItem[] | undefined>;
  readonly searchResult: Signal<SearchResult<BaseItem> | undefined>;
  readonly sort: Signal<ItemListSort | undefined>;
  readonly sortOptions?: Signal<readonly ItemListSortOption[]>;
  readonly total?: Signal<number>;

  search(term?: string): void;
  setSortMode(key: string): void;
  create?(): void;
  loadMore?(): void;
}

export const LIST_FACADE = new InjectionToken<ListPageFacade>('LIST_FACADE');
