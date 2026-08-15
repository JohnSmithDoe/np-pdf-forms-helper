// ─── why ────────────────────────────────────────────────────────
// A hand-written mirror of `electron/bridge/shared.model.ts`. The Electron main
// process keeps its own copy and the two are never imported across the process
// boundary, so the JSON SHAPE is the contract and must stay identical on both
// sides — only the TypeScript names differ here (no `I`/`T` prefix).
// ────────────────────────────────────────────────────────────────

export type DocumentType = 'pdf' | 'xlsx' | 'resource';

interface DocumentBase<T = DocumentType> {
  id: string;
  name: string;
  filename: string;
  mtime: number;
  type: T;
}

export interface MappedField {
  origId: string;
  mappedName: string;
}

export interface MappedInput {
  identifiers: string[];
  value: string;
}

export interface MappedDocument<T = DocumentType> extends DocumentBase<T> {
  mapped?: MappedField[];
}

export interface XlsxDocument extends MappedDocument<'xlsx'> {
  sheets: { id: string; name: string }[];
}

export interface PdfDocument extends MappedDocument<'pdf'> {
  fields: { id: string; path: string }[];
  previewfile: string;
}
