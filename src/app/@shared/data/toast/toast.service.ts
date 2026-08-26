// ─── why ────────────────────────────────────────────────────────
// The app's transient messages, in one place. A success report is not worth a
// modal the user has to dismiss before working on — it is an acknowledgement,
// so it announces itself and leaves.
//
// ONE toast at a time, by construction: a new one dismisses the incumbent
// first. Commands here are triggered in bursts (tick a field, save, tick the
// next), and Ionic stacks toasts rather than replacing them — three
// acknowledgements piled up cover the panel the user is working in.
//
// A `duration` alone would be an accessibility trap — an auto-dismissing
// message cannot be re-read — so every toast also carries a cancel button.
// `role: 'cancel'` and not the label: relabelling the button must not change
// what dismissing means.
//
// `ion-toast` is `role="status"` + `aria-live="polite"`, so its text is
// announced but a button inside it is not. Nothing here may therefore be the
// ONLY way to reach an action — a toast informs, and the app offers no action
// through one. A report that does offer one (the export folder) stays a dialog.
//
// The durations: 4s is Material's own snackbar guidance and the app renders
// `mode: 'md'`; a failure is read rather than noticed and cannot be acted on
// while it is up, so it stays long enough to finish reading twice.
// ────────────────────────────────────────────────────────────────

import { inject, Injectable } from '@angular/core';
import { ToastController } from '@ionic/angular/standalone';
import type { ToastRequest } from '../../model/toast.types';

const DURATION_MS = 4000;
const DURATION_DANGER_MS = 8000;

@Injectable({ providedIn: 'root' })
export class ToastService {
  readonly #toasts = inject(ToastController);

  #current?: HTMLIonToastElement;

  async show({
    header,
    message,
    color = 'success',
    durationMs,
  }: ToastRequest): Promise<void> {
    await this.#dismissCurrent();

    const toast = await this.#toasts.create({
      header,
      message,
      color,
      position: 'bottom',
      duration:
        durationMs ?? (color === 'danger' ? DURATION_DANGER_MS : DURATION_MS),
      buttons: [{ text: 'Schließen', role: 'cancel' }],
    });

    this.#current = toast;
    void toast.onDidDismiss().then(() => {
      if (this.#current === toast) this.#current = undefined;
    });
    await toast.present();
  }

  async #dismissCurrent(): Promise<void> {
    const incumbent = this.#current;
    this.#current = undefined;
    await incumbent?.dismiss(undefined, 'cancel');
  }
}
