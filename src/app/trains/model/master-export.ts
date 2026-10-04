// ─── why ────────────────────────────────────────────────────────
// The master update wizard's constants, shared by its four pages: the phase
// label that says the customer's workbook is the target, not the
// Schattensystem, and the step count. In `model` because a feature may not
// import another feature.
// ────────────────────────────────────────────────────────────────

import type { MasterMode } from './trains.types';

export const EXPORT_PHASE = 'Master aktualisieren';
export const EXPORT_STEPS = 4;

export const MODE_LABELS: Record<MasterMode, string> = {
  snapshot: 'Stand ersetzen',
  feed: 'Fortlaufend ergänzen',
};
