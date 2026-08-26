// ─── why ────────────────────────────────────────────────────────
// A hand-written mirror of `src-tauri/src/model.rs`. Nothing generates either
// side and neither is imported across the process boundary, so the JSON SHAPE
// is the contract and must stay identical on both — only the spelling differs
// (camelCase here, `#[serde(rename_all)]` there). Change one, change the other.
//
// `AnyDocument` is the union as it actually arrives: `type` discriminates, so
// `previewfile` and `fields` are reachable only after narrowing on it. Narrow,
// never cast — `MappedDocument` alone is the erased base, and using it for a
// wire value loses what the JSON already carries. `DocumentKind` in `model.rs`
// is the same union, tagged by serde on the same key.
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

export type ResourceDocument = MappedDocument<'resource'>;

export type AnyDocument = PdfDocument | XlsxDocument | ResourceDocument;
