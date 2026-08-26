// ─── why ────────────────────────────────────────────────────────
// A hand-written mirror of `src-tauri/src/model.rs`. Nothing generates either
// side and neither is imported across the process boundary, so the JSON SHAPE
// is the contract and must stay identical on both — only the spelling differs
// (camelCase here, `#[serde(rename_all)]` there). Change one, change the other.
//
// `ClientData.documents` / `.profiles` are ABSENT when the command that answered
// cannot have changed them, and present — possibly empty — when it can. Rust
// says this natively with an `Option`, so `FillerStore` acts on presence: a list
// that is there is the complete current one and replaces what is held.
// ────────────────────────────────────────────────────────────────

import { AnyDocument } from './document.types';
import { Profile } from './profile.types';

export interface ClientReport {
  headline: string;
  messages: string[];
  messageFolder?: string;
}

export interface ClientData {
  documents?: AnyDocument[];
  profiles?: Profile[];
  message?: ClientReport;
}
