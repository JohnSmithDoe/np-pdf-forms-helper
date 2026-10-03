// ─── why ────────────────────────────────────────────────────────
// The import walk's two constants, shared by its five pages: the phase label
// that tells the user they are writing to the Schattensystem now, and the step
// count the progress bar is out of. In `model` because every page needs them
// and a feature may not import another feature.
// ────────────────────────────────────────────────────────────────

export const IMPORT_PHASE = 'Import ins Schattensystem';
export const IMPORT_STEPS = 5;
