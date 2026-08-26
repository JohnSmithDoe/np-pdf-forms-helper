// ─── why ────────────────────────────────────────────────────────
// A hand-written mirror of `AppInfo` in `src-tauri/src/about.rs`. Same rule as
// `@shared/model`: nothing generates either side, the JSON shape IS the
// contract, and only the spelling differs. Change one, change the other.
// ────────────────────────────────────────────────────────────────

export interface AppInfo {
  name: string;
  version: string;
  dataPath: string;
  outputPath: string;
  cachePath: string;
}
