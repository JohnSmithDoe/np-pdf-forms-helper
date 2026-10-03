// ─── why ────────────────────────────────────────────────────────
// The menu's vocabulary. It lives in `model` rather than next to the menu
// component because the shell owns the entry LIST and the component only
// renders it — `type:shell` may reach `type:model`, not `type:ui`'s internals.
//
// A `NavGroup` is a block of entries with an optional heading; the menu leads
// with the group that has none.
//
// `icon` is an ionicons name, registered by importing the symbol in the
// component that renders it: a wrong name renders NOTHING and raises no error.
// ────────────────────────────────────────────────────────────────

export interface NavItem {
  route: string;
  label: string;
  icon: string;
}

export interface NavGroup {
  header?: string;
  items: NavItem[];
}
