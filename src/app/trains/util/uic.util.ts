// ─── why ────────────────────────────────────────────────────────
// The display grouping for a wagen number, mirrored from
// `trains::sanitise::format::uic_display`. The store holds the bare twelve
// digits and nothing else; this is derived every time it is shown.
//
// Anything that is not twelve digits comes back untouched rather than sliced —
// it runs inside labels and error text, where throwing is the worst outcome.
// ────────────────────────────────────────────────────────────────

export function formatUic(digits: string): string {
  if (!/^\d{12}$/.test(digits)) return digits;
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
