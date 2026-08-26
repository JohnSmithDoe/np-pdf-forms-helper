// ─── why ────────────────────────────────────────────────────────
// The vocabulary the generic list page speaks, cloned from np-commlink where
// thirteen pages share one list shell. Only the presentation half came across:
// its state layer is NgRx over in-memory data, and here the truth lives in Rust.
// The `LIST_FACADE` seam is what makes that substitution free.
//
// `items` is `undefined` while UNKNOWN and `[]` once KNOWN-EMPTY, and the two
// must not be collapsed — `!items()?.length` is true for both, which flashes the
// empty state over data that simply has not arrived yet.
// ────────────────────────────────────────────────────────────────

export interface BaseItem {
  id: string;
  name: string;
}

export type SortDirection = 'asc' | 'desc';

export interface ItemListSort {
  sortBy: string;
  sortDirection: SortDirection;
}

export interface ItemListSortOption {
  key: string;
  label: string;
}

export interface SearchResult<T extends BaseItem> {
  items: T[];
  searchTerm: string;
}
