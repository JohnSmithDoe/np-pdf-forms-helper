// ─── why ────────────────────────────────────────────────────────
// One rule, in one place, because every domain has to route reports the same
// way — and because it reads the REPORT rather than its sender, a command
// written later gets the right treatment without being listed anywhere.
//
// A folder to open is an ACTION and more than one line is a list — an export
// run and a folder import both produce one line per document, and neither
// survives a toast that takes itself away again. Everything else is an
// acknowledgement, and an acknowledgement that has to be clicked away is a
// modal the app interrupts itself with after every save.
// ────────────────────────────────────────────────────────────────

import type { ClientReport } from '../model/client.types';

export function needsDialog(report: ClientReport): boolean {
  return Boolean(report.messageFolder) || report.messages.length > 1;
}
