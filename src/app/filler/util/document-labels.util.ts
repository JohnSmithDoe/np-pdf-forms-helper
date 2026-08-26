// ─── why ────────────────────────────────────────────────────────
// The two secondary lines a document row shows, in one place: what the app knows
// about a document, and WHAT A MAPPED FIELD IS MAPPED TO.
//
// They live here because two lists render them — the expert page's editable
// `document-list` and the export wizard's read-only `selection-list` — and the
// same row saying "PDF-Formular · 2 von 3 Feldern angehakt" in one mode and
// something else in the other would read as two different apps. `util` is pure,
// so a template calls them through a thin protected method on its component.
//
// Both take the DOCUMENT rather than hanging off `FillerField`: the origin is
// only reachable through the pdf/xlsx branch of the union, and a field carries no
// discriminator of its own.
// ────────────────────────────────────────────────────────────────

import { FillerDocument, FillerField } from '../model/filler.types';

export function documentMeta(document: FillerDocument): string {
  if (document.type === 'resource') return 'Anlage · wird nur kopiert';

  const kind = document.type === 'pdf' ? 'PDF-Formular' : 'Excel-Tabelle';
  const fields = document.mapped;
  if (!fields.length) return `${kind} · noch kein Feld zugeordnet`;

  const selected = fields.filter((field) => field.selected).length;
  return `${kind} · ${selected} von ${fields.length} Feldern angehakt`;
}

export function fieldOrigin(
  document: FillerDocument,
  field: FillerField
): string {
  if (document.type === 'pdf') {
    const source = document.fields.find((entry) => entry.id === field.origId);
    return source ? `Formularfeld ${source.path}` : '';
  }
  if (document.type === 'xlsx') {
    const sheet = document.sheets.find((entry) => entry.id === field.origId);
    return sheet ? `Tabellenblatt ${sheet.name}` : '';
  }
  return '';
}
