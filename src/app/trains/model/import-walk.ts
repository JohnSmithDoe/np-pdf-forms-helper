// ─── why ────────────────────────────────────────────────────────
// The import walk's two constants, shared by its five pages: the phase label
// that tells the user they are writing to the Schattensystem now, and the step
// count the progress bar is out of. In `model` because every page needs them
// and a feature may not import another feature.
//
// A walk is fed by one of two SOURCES: a filed document, or one sheet of the
// customer's master. The steps are the same; where the walk starts, ends and
// what its header names differ, and `WalkSource` is what tells them apart.
// ────────────────────────────────────────────────────────────────

export type WalkSource =
  { kind: 'dokument'; id: string } | { kind: 'master'; sheet: string };

export const IMPORT_PHASE = 'Import ins Schattensystem';
export const IMPORT_STEPS = 5;
