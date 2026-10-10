// ─── why ────────────────────────────────────────────────────────
// Filtering and sorting a list, as pure functions, so every facade does it the
// same way and none of them injects anything to do it.
//
// The comparator sniffs the first non-empty value rather than being told a type:
// a column of amounts sorts numerically, a column of ISO dates sorts as text
// (which is correct for ISO, and the reason the wire carries dates as strings),
// and everything else compares with `localeCompare` so `Ä` lands beside `A`
// instead of after `Z`.
//
// A sort with a `farbe` is Excel's „Nach Farbe sortieren“: rows of that colour
// first, and the column's order inside both halves.
// ────────────────────────────────────────────────────────────────

import type {
  BaseItem,
  ItemListSort,
  SearchResult,
} from '../../model/item-list.types';

export function searchList<T extends BaseItem>(
  items: T[],
  term: string | undefined,
  haystack: (item: T) => string
): SearchResult<T> | undefined {
  const needle = term?.trim().toLowerCase();
  if (!needle) return undefined;
  return {
    searchTerm: term ?? '',
    items: items.filter((item) =>
      haystack(item).toLowerCase().includes(needle)
    ),
  };
}

export function sortList<T extends BaseItem>(
  items: T[],
  sort: ItemListSort | undefined,
  valueOf: (item: T, key: string) => string
): T[] {
  if (!sort) return items;
  const direction = sort.sortDirection === 'desc' ? -1 : 1;
  const top = (item: T): number =>
    sort.farbe && item.farbe === sort.farbe ? 0 : 1;
  return [...items].sort(
    (left, right) =>
      top(left) - top(right) ||
      direction *
        compareValues(valueOf(left, sort.sortBy), valueOf(right, sort.sortBy))
  );
}

export function toggleSort(
  sort: ItemListSort | undefined,
  key: string
): ItemListSort {
  if (sort?.sortBy !== key) return { sortBy: key, sortDirection: 'asc' };
  return {
    sortBy: key,
    sortDirection: sort.sortDirection === 'asc' ? 'desc' : 'asc',
  };
}

export function compareValues(left: string, right: string): number {
  if (!left) return right ? 1 : 0;
  if (!right) return -1;
  const numbers = Number(left.replace(',', '.'));
  const others = Number(right.replace(',', '.'));
  if (!Number.isNaN(numbers) && !Number.isNaN(others)) return numbers - others;
  return left.localeCompare(right, 'de');
}
