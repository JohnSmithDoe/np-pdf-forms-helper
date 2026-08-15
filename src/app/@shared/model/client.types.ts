// ─── why ────────────────────────────────────────────────────────
// A hand-written mirror of `electron/bridge/shared.model.ts`. The Electron main
// process keeps its own copy and the two are never imported across the process
// boundary, so the JSON SHAPE is the contract and must stay identical on both
// sides — only the TypeScript names differ here (no `I`/`T` prefix).
// ────────────────────────────────────────────────────────────────

import { MappedDocument } from './document.types';
import { Profile } from './profile.types';

export interface ClientReport {
  headline: string;
  messages: string[];
  messageFolder?: string;
}

export interface ClientData {
  documents: MappedDocument[];
  profiles: Profile[];
  message: ClientReport;
}
