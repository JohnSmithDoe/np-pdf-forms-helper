// ─── why ────────────────────────────────────────────────────────
// A hand-written mirror of `EAppChannels` in `electron/bridge/shared.model.ts`.
// The Electron main process keeps its own copy, so the channel STRINGS are the
// contract and must stay identical on both sides; the TypeScript shape is free
// to differ, and a frozen const map erases at compile time where an `enum` does
// not — the same reason nothing else in this app declares one.
// ────────────────────────────────────────────────────────────────

export const AppChannel = {
  GET: 'get-templates',
  GET_PROFILES: 'get-profiles',
  REMOVE: 'remove-template',
  OPEN: 'open-file',
  OPEN_OUTPUT: 'open-output',
  SAVE: 'save-templates',
  SAVE_PROFILES: 'save-profiles',
  ADD: 'add-template',
  REMAP: 'remap-template',
  EXPORT: 'export-templates',

  FINISHED_LOAD: 'finished-loading',
  CLIENT_UPDATE: 'client-update',
  CLIENT_ERROR: 'client-error',
} as const;

export type AppChannel = (typeof AppChannel)[keyof typeof AppChannel];
