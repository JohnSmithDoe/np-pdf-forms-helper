// ─── why ────────────────────────────────────────────────────────
// `ListPageFacade` over the saved mappings. A template has no create button
// because it is born from an import — naming one before a file has been mapped
// would be naming a mapping that does not exist yet.
//
// A BUILT-IN template ships with the app and cannot be removed — the backend
// refuses — so its row says „mitgeliefert“ and offers no delete at all. A user
// copy of one (`origin` set) is the user's own and deletable; removing it brings
// the built-in it shadowed back into matching.
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
import { labelOf } from '../model/field-catalogue';
import { TrainsStore } from './trains.store';

export interface TemplateRow extends BaseItem {
  createdAt: string;
  fields: string;
  builtin: boolean;
}

@Injectable({ providedIn: 'root' })
export class TemplateListFacade implements ListPageFacade {
  readonly #store = inject(TrainsStore);
  readonly #term = signal<string | undefined>(undefined);
  readonly #sort = signal<ItemListSort>({
    sortBy: 'name',
    sortDirection: 'asc',
  });

  readonly sort = this.#sort.asReadonly();
  readonly sortOptions = signal<readonly ItemListSortOption[]>([
    { key: 'name', label: 'Name' },
  ]).asReadonly();

  readonly #rows = computed<TemplateRow[] | undefined>(() => {
    const templates = this.#store.templates();
    if (!templates) return undefined;
    return templates.map((template) => ({
      id: template.id,
      name: template.name,
      createdAt: template.createdAt,
      builtin: template.builtin,
      fields: [
        ...(template.builtin ? ['mitgeliefert'] : []),
        template.plan.columns
          .filter((column) => column.field !== 'ignorieren')
          .map((column) => labelOf(column.field))
          .join(', '),
      ].join(' · '),
    }));
  });

  readonly searchResult = computed(() =>
    searchList(this.#rows() ?? [], this.#term(), (row) => row.name)
  );

  readonly items = computed<BaseItem[] | undefined>(() => {
    const rows = this.#rows();
    if (!rows) return undefined;
    const found = this.searchResult()?.items ?? rows;
    return sortList(found, this.#sort(), (row) => row.name);
  });

  search(term?: string): void {
    this.#term.set(term);
  }

  setSortMode(key: string): void {
    this.#sort.set(toggleSort(this.#sort(), key));
  }
}
