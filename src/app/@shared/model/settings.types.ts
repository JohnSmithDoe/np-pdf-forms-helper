// ─── why ────────────────────────────────────────────────────────
// UI preferences the app remembers between sessions. Deliberately NOT the wire
// contract: nothing here reaches `src-tauri/src/model.rs`, because none of it is
// the user's data — losing it costs one re-toggle, not a document.
//
// `ViewMode` decides what `/documents` redirects to. That redirect resolves
// inside `checkGuards`, before any resolver and before any app initializer would
// help, which is the whole reason this is read SYNCHRONOUSLY out of
// localStorage rather than fetched.
// ────────────────────────────────────────────────────────────────

export type ViewMode = 'expert' | 'wizard';

export interface UiSettings {
  viewMode: ViewMode;
}
