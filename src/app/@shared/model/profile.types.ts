// ─── why ────────────────────────────────────────────────────────
// A hand-written mirror of `src-tauri/src/model.rs`. Nothing generates either
// side and neither is imported across the process boundary, so the JSON SHAPE
// is the contract and must stay identical on both — only the spelling differs
// (camelCase here, `#[serde(rename_all)]` there). Change one, change the other.
// ────────────────────────────────────────────────────────────────

export interface Profile {
  id: string;
  name: string;
  documentIds: string[];
  fieldIds: string[];
}
