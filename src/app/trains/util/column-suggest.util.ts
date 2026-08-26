// ─── why ────────────────────────────────────────────────────────
// Auto-mapping: a column header becomes a guess at which field it is.
//
// It is a SUGGESTION and the UI says so on every row it fills in. The user needs
// to know what to CHECK, not what to redo — a silent auto-map is worse than none,
// because it looks like something a human decided.
//
// Aliases are matched longest-first so that a header containing both `nummer`
// and `wagennummer` resolves to the more specific one, and a field already taken
// is not offered twice: two columns mapped to one field would be one of them
// silently winning at stage time.
//
// The catalogue's aliases are constant, so they are normalised ONCE at module
// scope rather than per column: normalising ~45 aliases inside the loop ran six
// regex passes each over every column of every file, all recomputing the same
// strings.
// ────────────────────────────────────────────────────────────────

import { FIELD_CATALOGUE } from '../model/field-catalogue';
import type { ColumnBinding, FieldKind } from '../model/trains.types';

export function normaliseHeader(header: string): string {
  return header
    .toLowerCase()
    .replace(/ä/g, 'ae')
    .replace(/ö/g, 'oe')
    .replace(/ü/g, 'ue')
    .replace(/ß/g, 'ss')
    .replace(/[^a-z0-9]+/g, ' ')
    .trim();
}

const ALIASES: readonly { field: FieldKind; needle: string; length: number }[] =
  FIELD_CATALOGUE.flatMap((entry) =>
    entry.field === 'ignorieren'
      ? []
      : entry.aliases.map((alias) => ({
          field: entry.field,
          needle: normaliseHeader(alias),
          length: alias.length,
        }))
  ).sort((left, right) => right.length - left.length);

export function suggestField(header: string, taken: FieldKind[]): FieldKind {
  const needle = normaliseHeader(header);
  if (!needle) return 'ignorieren';

  return (
    ALIASES.find(
      (alias) => !taken.includes(alias.field) && needle.includes(alias.needle)
    )?.field ?? 'ignorieren'
  );
}

export function suggestBindings(columns: ColumnBinding[]): ColumnBinding[] {
  const taken: FieldKind[] = [];
  return columns.map((column) => {
    const field = suggestField(column.header, taken);
    if (field !== 'ignorieren') taken.push(field);
    return { ...column, field };
  });
}
