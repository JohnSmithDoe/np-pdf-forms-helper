// ─── why ────────────────────────────────────────────────────────
// Excel's AutoFilter as pure functions over rows, so a facade holds the filter
// state as a signal and decides nothing itself.
//
// A column's CHOICES come from the rows every OTHER column's filter lets
// through, which is what Excel does: filter the Bauart, open the Eigentümer,
// and only the owners of that Bauart are offered. Its own filter is left out,
// or unticking a value would remove it from the list it has to be ticked in
// again.
//
// `matches` is the dialog's SEARCH, steered by the condition („Beginnt mit“ …),
// and is not part of a stored filter: what the search finds is ticked, and the
// ticked values are the filter. It compares case-insensitively, like Excel.
// ────────────────────────────────────────────────────────────────

import type {
  BaseItem,
  ColumnChoices,
  ColumnFilter,
  ColumnFilters,
  FilterOp,
} from '../../model/item-list.types';
import { FARBEN } from '../farbe.util';
import { compareValues } from './list.selector';

export type ValueOf<T> = (item: T, key: string) => string;

export function filterList<T extends BaseItem>(
  items: readonly T[],
  filters: ColumnFilters,
  valueOf: ValueOf<T>,
  except?: string
): T[] {
  const active = Object.entries(filters).filter(
    ([key, filter]) => key !== except && isFiltered(filter)
  );
  if (!active.length) return [...items];
  return items.filter((item) =>
    active.every(([key, filter]) => passes(item, valueOf(item, key), filter))
  );
}

export function columnChoices<T extends BaseItem>(
  items: readonly T[],
  filters: ColumnFilters,
  key: string,
  valueOf: ValueOf<T>
): ColumnChoices {
  const rows = filterList(items, filters, valueOf, key);
  const values = [...new Set(rows.map((row) => valueOf(row, key).trim()))];
  const farben = new Set(rows.map((row) => row.farbe));
  return {
    values: values.sort(compareValues),
    farben: FARBEN.filter((farbe) => farben.has(farbe)),
    ohneFarbe: farben.has(undefined),
  };
}

export function withFilter(
  filters: ColumnFilters,
  key: string,
  filter: ColumnFilter | undefined
): ColumnFilters {
  const { [key]: _dropped, ...rest } = filters;
  return filter && isFiltered(filter) ? { ...rest, [key]: filter } : rest;
}

export function isFiltered(filter: ColumnFilter | undefined): boolean {
  return (
    !!filter && (filter.values !== undefined || filter.farbe !== undefined)
  );
}

function passes<T extends BaseItem>(
  item: T,
  raw: string,
  filter: ColumnFilter
): boolean {
  const value = raw.trim();
  if (filter.values && !filter.values.includes(value)) return false;
  if (filter.farbe === 'keine' && item.farbe) return false;
  return !(
    filter.farbe &&
    filter.farbe !== 'keine' &&
    item.farbe !== filter.farbe
  );
}

export function matches(value: string, op: FilterOp, term: string): boolean {
  const text = term.trim().toLowerCase();
  if (!text) return true;
  const haystack = value.toLowerCase();
  switch (op) {
    case 'gleich':
      return haystack === text;
    case 'ungleich':
      return haystack !== text;
    case 'beginnt':
      return haystack.startsWith(text);
    case 'endet':
      return haystack.endsWith(text);
    case 'enthaelt':
      return haystack.includes(text);
    case 'enthaeltNicht':
      return !haystack.includes(text);
  }
}
