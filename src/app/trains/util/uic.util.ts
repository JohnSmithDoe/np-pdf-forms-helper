// ─── why ────────────────────────────────────────────────────────
// How a wagen number is shown, mirrored from `trains::sanitise::format::uic_in`.
// The store holds the bare twelve digits and nothing else; the Schattensystem
// setting decides whether they are shown as they are (`compact`, the default)
// or grouped the way they are painted on the wagon.
//
// Anything that is not twelve digits comes back untouched rather than sliced —
// it runs inside labels and error text, where throwing is the worst outcome.
// ────────────────────────────────────────────────────────────────

import type { UicStyle } from '../model/trains.types';

export function formatUic(digits: string, style: UicStyle): string {
  if (style === 'compact' || !/^\d{12}$/.test(digits)) return digits;
  return `${digits.slice(0, 2)} ${digits.slice(2, 4)} ${digits.slice(4, 8)} ${digits.slice(8, 11)}-${digits.slice(11)}`;
}

export function formatCents(cents: number | undefined): string {
  if (cents === undefined) return '';
  const sign = cents < 0 ? '-' : '';
  const absolute = Math.abs(cents);
  return `${sign}${Math.trunc(absolute / 100)},${String(absolute % 100).padStart(2, '0')}`;
}

export function formatIsoDate(iso: string): string {
  const [year, month, day] = iso.split('-');
  return year && month && day ? `${day}.${month}.${year}` : iso;
}
