// ─── why ────────────────────────────────────────────────────────
// A hand-written mirror of `electron/bridge/shared.model.ts`. The Electron main
// process keeps its own copy and the two are never imported across the process
// boundary, so the JSON SHAPE is the contract and must stay identical on both
// sides — only the TypeScript names differ here (no `I`/`T` prefix).
// ────────────────────────────────────────────────────────────────

export interface Profile {
  id: string;
  name: string;
  documentIds: string[];
  fieldIds: string[];
}
