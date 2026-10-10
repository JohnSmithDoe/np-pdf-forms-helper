// ─── why ────────────────────────────────────────────────────────
// The vocabulary the generic list page speaks, cloned from np-commlink where
// thirteen pages share one list shell. Only the presentation half came across:
// its state layer is NgRx over in-memory data, and here the truth lives in Rust.
// The `LIST_FACADE` seam is what makes that substitution free.
//
// `items` is `undefined` while UNKNOWN and `[]` once KNOWN-EMPTY, and the two
// must not be collapsed — `!items()?.length` is true for both, which flashes the
// empty state over data that simply has not arrived yet.
//
// The column filter is Excel's AutoFilter, because that is what the users work
// in: per column a searchable value checklist and „Nach Farbe“, and a
// sort that can put one colour on top. `values` absent means every value — not
// an empty list, which ticks nothing. The colour is the ROW's (`farbe`), not a
// cell's: an entity carries one mark, so every column filters on the same one.
// `''` in `values` is the empty cell, Excel's „(Leer)“.
// ────────────────────────────────────────────────────────────────

import type { Farbe } from './farbe.types';

export interface BaseItem {
  id: string;
  name: string;
  farbe?: Farbe;
}

export type SortDirection = 'asc' | 'desc';

export interface ItemListSort {
  sortBy: string;
  sortDirection: SortDirection;
  farbe?: Farbe;
}

export interface ItemListSortOption {
  key: string;
  label: string;
}

export interface SearchResult<T extends BaseItem> {
  items: T[];
  searchTerm: string;
}

export interface ListColumn {
  key: string;
  label: string;
}

export type FilterOp =
  'gleich' | 'ungleich' | 'beginnt' | 'endet' | 'enthaelt' | 'enthaeltNicht';

export interface ColumnFilter {
  values?: readonly string[];
  farbe?: Farbe | 'keine';
}

export type ColumnFilters = Readonly<Record<string, ColumnFilter>>;

export interface ColumnChoices {
  values: readonly string[];
  farben: readonly Farbe[];
  ohneFarbe: boolean;
}
