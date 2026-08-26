// ─── why ────────────────────────────────────────────────────────
// What a caller asks a toast for, and nothing about how it is shown.
//
// `color` is an Ionic colour ROLE, not a value — the same contract the rest of
// the app follows (`color="danger"` on the element, never a hex in a
// stylesheet), so a toast carries its meaning rather than its palette.
// ────────────────────────────────────────────────────────────────

export type ToastColor = 'success' | 'warning' | 'danger';

export interface ToastRequest {
  header: string;
  message?: string;
  color?: ToastColor;
  durationMs?: number;
}
