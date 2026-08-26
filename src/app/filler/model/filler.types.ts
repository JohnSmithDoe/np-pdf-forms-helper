// ─── why ────────────────────────────────────────────────────────
// The view shapes the filler renders, kept in `model` rather than `data`
// because Sheriff's `type:ui` may reach `type:model` and never `type:data` —
// a dumb row component has to be able to NAME what it is handed.
//
// They are the wire types from `@shared/model` plus what only the screen has:
// which rows are ticked for export, and the value the user typed for a field
// name. The store keeps those two apart from the documents (as id lists and a
// name→value record) and re-joins them here, which is why the export list can
// be a single `computed()`.
//
// `Selectable` distributes over the union rather than collapsing it: written as
// `Omit<AnyDocument, 'mapped'>` the pdf-only and xlsx-only keys would be erased
// on the way in, and `document.type === 'pdf'` would narrow to nothing.
//
// `DocumentSource` mirrors the enum of the same name in
// `src-tauri/src/filler/commands.rs`, and the three spellings ARE the serde
// ones. It replaced a `wholeFolder` boolean: there are three ways in now, and a
// second boolean beside the first would leave two of four combinations meaning
// nothing.
// ────────────────────────────────────────────────────────────────

import {
  AnyDocument,
  MappedDocument,
  MappedField,
  MappedInput,
} from '../../@shared/model/document.types';

export type SortDirection = 'aufsteigend' | 'absteigend';

export type DocumentSource = 'file' | 'files' | 'folder';

export type FillerField = MappedField & { selected: boolean };

type Selectable<D> = D extends MappedDocument
  ? Omit<D, 'mapped'> & { selected: boolean; mapped: FillerField[] }
  : never;

export type FillerDocument = Selectable<AnyDocument>;

export type ExportField = MappedInput & {
  mappedName: string;
  info: string;
};
