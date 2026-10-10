// ─── why ────────────────────────────────────────────────────────
// The name of the folder one export run writes into, and the only place that
// decides it.
//
// It is a LOCAL, sortable timestamp — `2026-08-22_13-37-48` — because the user
// meets it in the Explorer, next to their own clock. What it replaced was the
// raw `Date.now()` number, which is unreadable and unsortable by eye; a UTC one
// would be readable and wrong by an hour or two on every German desk. That is
// also why this is not the backend's job, the way `trains-<iso>` is: Rust has no
// time zone without a crate, and the renderer has the user's for free.
//
// The date leads so a year sorts before its months, `:` and `/` are out because
// Windows rejects both in a folder name, and the SECONDS are the collision
// guard: `create_output_folder` refuses to write into a folder that already
// exists, so the resolution of this name is what decides how soon a second run
// may follow. Minutes would refuse two runs made in the same minute.
//
// The suffix is appended raw — `FillerStore.setExportSuffix` has already taken
// the path separators out of it, which is the half that must not be skipped.
// ────────────────────────────────────────────────────────────────

function pad(value: number): string {
  return `${value}`.padStart(2, '0');
}

export function runFolder(stamp: number, suffix: string): string {
  const at = new Date(stamp);
  const date = `${at.getFullYear()}-${pad(at.getMonth() + 1)}-${pad(at.getDate())}`;
  const time = `${pad(at.getHours())}-${pad(at.getMinutes())}-${pad(at.getSeconds())}`;
  const name = `${date}_${time}`;
  return suffix ? `${name}-${suffix}` : name;
}
