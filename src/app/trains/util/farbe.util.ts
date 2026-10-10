// ─── why ────────────────────────────────────────────────────────
// Which Farbe an entity shows: the hand mark wins, the master's is the fallback,
// so removing a hand mark brings the master's back rather than none. Keyed as
// Rust keys them — the Wagennummer's digits and the Radsatz match key — see
// `Markierungen` in `src-tauri/src/trains/model.rs`.
// ────────────────────────────────────────────────────────────────

import type { Farbe } from '../../@shared/model/farbe.types';
import type { Markierungen } from '../model/trains.types';

export type FarbArt = 'wagen' | 'radsaetze';

export interface FarbStand {
  farbe?: Farbe;
  hand?: Farbe;
  master?: Farbe;
}

export function farbeOf(
  markierungen: Markierungen,
  art: FarbArt,
  key: string
): FarbStand {
  const hand = markierungen.hand[art][key];
  const master = markierungen.master[art][key];
  return { farbe: hand ?? master, hand, master };
}
