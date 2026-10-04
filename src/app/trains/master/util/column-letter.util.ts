// ─── why ────────────────────────────────────────────────────────
// Excel's column letter for a 1-based position. The view is compared against
// the open workbook by eye, and Excel names its columns by letter, not by
// header — two of the radsatz exports repeat a header, and several sheets
// have columns with none at all.
// ────────────────────────────────────────────────────────────────

export function columnLetter(index: number): string {
  let letter = '';
  for (let rest = index; rest > 0; rest = Math.floor((rest - 1) / 26)) {
    letter = String.fromCharCode(65 + ((rest - 1) % 26)) + letter;
  }
  return letter;
}
