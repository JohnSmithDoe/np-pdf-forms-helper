// ─── why ────────────────────────────────────────────────────────
// The two overlays every composition point needs: ask a question, or open a
// dialog and wait for what it returns. Four pages want both now, and four copies
// of "create, present, await onWillDismiss, read the role" is four chances to
// get the role wrong.
//
// The answer to a confirmation is the dismiss ROLE, never the button's label:
// `'confirm'` is the contract, so relabelling a button cannot silently invert
// the test. That is the single most important line in this file.
//
// The question is the alert's HEADER and there is no message. An `ion-alert`
// renders its header as the accessible name of the dialog, so a question put in
// `message` instead is announced after the dialog is already focused — and the
// two buttons are all the body there is.
//
// It lives in `data` rather than `feature` because it composes nothing from the
// app — it takes the component to open as an ARGUMENT and imports no `smart-ui`,
// which is exactly what keeps it under `type:data`'s ceiling. Contrast
// `ReportPresenterService`, which names `ReportDialog` and therefore cannot.
// ────────────────────────────────────────────────────────────────

import { inject, Injectable } from '@angular/core';
import {
  AlertController,
  ModalController,
  type ModalOptions,
} from '@ionic/angular/standalone';

@Injectable({ providedIn: 'root' })
export class OverlayService {
  readonly #modals = inject(ModalController);
  readonly #alerts = inject(AlertController);

  async openModal<T>(
    component: ModalOptions['component'],
    componentProps?: ModalOptions['componentProps']
  ): Promise<T | undefined> {
    const modal = await this.#modals.create({ component, componentProps });
    await modal.present();
    const { data } = await modal.onWillDismiss<T>();
    return data;
  }

  async confirm(question: string): Promise<boolean> {
    const alert = await this.#alerts.create({
      header: question,
      buttons: [
        { text: 'Abbrechen', role: 'cancel' },
        { text: 'Bestätigen', role: 'confirm' },
      ],
    });
    await alert.present();
    const { role } = await alert.onWillDismiss();
    return role === 'confirm';
  }
}
