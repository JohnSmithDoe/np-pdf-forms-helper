// ─── why ────────────────────────────────────────────────────────
// Each `Farbe` is one Ionic colour role: `color="…"` on an element, or the
// `ion-color-…` class where a stripe needs `--ion-color-base`. Orange and yellow
// share `warning` because Ionic has one role for both — Rust's `farbe::from_argb`
// folds Excel's orange into Gelb for the same reason.
// ────────────────────────────────────────────────────────────────

import type { Farbe } from '../model/farbe.types';

export const FARBEN: readonly Farbe[] = [
  'rot',
  'gelb',
  'gruen',
  'blau',
  'lila',
  'grau',
];

export const FARBE_LABEL: Record<Farbe, string> = {
  rot: 'Rot',
  gelb: 'Gelb',
  gruen: 'Grün',
  blau: 'Blau',
  lila: 'Lila',
  grau: 'Grau',
};

export const FARBE_COLOR: Record<Farbe, string> = {
  rot: 'danger',
  gelb: 'warning',
  gruen: 'success',
  blau: 'primary',
  lila: 'tertiary',
  grau: 'medium',
};
