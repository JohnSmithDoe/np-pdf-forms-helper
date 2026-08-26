// ─── why ────────────────────────────────────────────────────────
// `ListPageFacade` over the partners, filtered by the role the route names. Two
// routes share it, which is why the role is settable rather than baked in.
//
// The alias count is shown because it is the thing that makes imports quiet: a
// partner with five aliases is one the user has stopped being asked about.
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
import { rollenLabel } from '../model/field-catalogue';
import type { PartnerRolle } from '../model/trains.types';
import { TrainsStore } from './trains.store';

export interface PartnerRow extends BaseItem {
  aliasCount: number;
  rollen: string;
}

@Injectable({ providedIn: 'root' })
export class PartnerListFacade implements ListPageFacade {
  readonly #store = inject(TrainsStore);
  readonly #term = signal<string | undefined>(undefined);
  readonly #sort = signal<ItemListSort>({
    sortBy: 'name',
    sortDirection: 'asc',
  });
  readonly #role = signal<PartnerRolle>('werkstatt');

  readonly role = this.#role.asReadonly();
  readonly sort = this.#sort.asReadonly();
  readonly sortOptions = signal<readonly ItemListSortOption[]>([
    { key: 'name', label: 'Name' },
  ]).asReadonly();

  readonly #rows = computed<PartnerRow[] | undefined>(() => {
    const partners = this.#store.partners();
    if (!partners) return undefined;
    const role = this.#role();
    return partners
      .filter((partner) => partner.rollen.includes(role))
      .map((partner) => ({
        id: partner.id,
        name: partner.name,
        aliasCount: partner.aliases.length,
        rollen: rollenLabel(partner.rollen),
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

  setRole(role: PartnerRolle): void {
    this.#role.set(role);
  }

  search(term?: string): void {
    this.#term.set(term);
  }

  setSortMode(key: string): void {
    this.#sort.set(toggleSort(this.#sort(), key));
  }
}
