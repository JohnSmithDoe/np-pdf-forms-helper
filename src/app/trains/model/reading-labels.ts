// ─── why ────────────────────────────────────────────────────────
// The human names of the two readings a column can be in doubt about, beside
// the wire values they label — the same split as `field-catalogue.ts`. Each
// label carries an EXAMPLE in its own notation, because "deutsch" says nothing
// to somebody looking at `1.234` and wondering which way it was meant; the
// example is what they compare the file against.
// ────────────────────────────────────────────────────────────────

import type { DateOrder, DecimalStyle } from './trains.types';

export const DECIMAL_LABELS: Record<DecimalStyle, string> = {
  german: 'Komma als Dezimalzeichen (1.234,56)',
  english: 'Punkt als Dezimalzeichen (1,234.56)',
};

export const DATE_ORDER_LABELS: Record<DateOrder, string> = {
  dayFirst: 'Tag zuerst (31/12/2025)',
  monthFirst: 'Monat zuerst (12/31/2025)',
};

export const DECIMAL_SHORT: Record<DecimalStyle, string> = {
  german: 'Komma',
  english: 'Punkt',
};

export const DATE_ORDER_SHORT: Record<DateOrder, string> = {
  dayFirst: 'Tag zuerst',
  monthFirst: 'Monat zuerst',
};
