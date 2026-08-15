// ─── why ────────────────────────────────────────────────────────
// A hand-written mirror of `electron/bridge/shared.model.ts`. The Electron main
// process keeps its own copy and the two are never imported across the process
// boundary, so the JSON SHAPE is the contract and must stay identical on both
// sides — only the TypeScript names differ here (no `I`/`T` prefix).
// ────────────────────────────────────────────────────────────────

export const APP_VERSION = 'v1.1.9';

export interface AppConfig {
  PDFTK_EXE?: string;
  ENCODING?: string;
  DATA_PATH?: string;
  TMP_PATH?: string;
  CACHE_PATH?: string;
  OUTPUT_PATH?: string;
  DB_FILE?: string;
  PROFILE_FILE?: string;
}
