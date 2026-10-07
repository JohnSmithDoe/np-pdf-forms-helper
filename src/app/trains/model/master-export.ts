// ─── why ────────────────────────────────────────────────────────
// The master update wizard's constants, shared by its four pages: the phase
// label that says the customer's workbook is the target, not the
// Schattensystem, and the step count. In `model` because a feature may not
// import another feature.
// ────────────────────────────────────────────────────────────────

export const EXPORT_PHASE = 'Master aktualisieren';
export const EXPORT_STEPS = 4;

export const APPEND_LABELS = {
  on: 'neue Zeilen werden angehängt',
  off: 'nur vorhandene Zeilen werden aktualisiert',
} as const;

export const REMOVE_LABEL = 'fehlende Zeilen werden geleert';
