// ─── why ────────────────────────────────────────────────────────
// A file path the backend sent, split for display and for „Ordner öffnen“.
// Both separators: the target is Windows, the dev machine is macOS. A bare name
// has no folder and comes back empty rather than throwing.
// ────────────────────────────────────────────────────────────────

function lastSeparator(path: string): number {
  return Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'));
}

export function folderOf(path: string): string {
  return path.slice(0, Math.max(lastSeparator(path), 0));
}

export function fileOf(path: string): string {
  return path.slice(lastSeparator(path) + 1);
}
