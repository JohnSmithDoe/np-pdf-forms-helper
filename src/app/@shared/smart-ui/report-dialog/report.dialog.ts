// ─── why ────────────────────────────────────────────────────────
// The backend's `ClientReport` on screen — the welcome text, an export
// summary, and (with `headline` set by the caller) a failed command. The prop
// IS `ClientReport`, so a report needs no translation on the way in:
//
//   modalCtrl.create({ component: ReportDialog, componentProps: { report } })
//   modalCtrl.create({ component: ReportDialog, componentProps: {
//     report: { headline: 'Es ist ein Problem aufgetreten',
//               messages: error.messages },
//   }})
//
// `report` is a signal `input()`, which only works because
// `provideIonicAngular` sets `useSetInputAPI: true` — without it Ionic ASSIGNS
// `componentProps` onto the instance, and an assignment to an `InputSignal`
// property does nothing at all. The dialog would open blank. See app.providers.
//
// `messages` are rendered as TEXT and nothing here interpolates markup — they
// carry filesystem-derived names, so `[innerHtml]` would be an HTML sink.
//
// "Ordner öffnen" DISMISSES with the role `'folder'` rather than emitting: a
// controller-created modal exposes no instance to subscribe to, and by the time
// the button has done its job the folder is open in front of the user. The
// BUTTON itself lives in `ReportViewComponent`, which is the body this dialog
// and both wizards' result steps share — only the meaning of pressing it
// differs, and that is what this file supplies.
//
// The template marks the headline as a heading explicitly: `ion-title` renders a
// plain div, and a dialog whose title is not a heading gives a screen reader
// nothing to land on.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  inject,
  input,
} from '@angular/core';
import {
  IonButton,
  IonButtons,
  IonContent,
  IonFooter,
  IonHeader,
  IonTitle,
  IonToolbar,
  ModalController,
} from '@ionic/angular/standalone';
import { ClientReport } from '../../model/client.types';
import { ReportViewComponent } from '../../ui/report-view/report-view.component';

@Component({
  selector: 'app-report-dialog',
  templateUrl: 'report.dialog.html',
  styleUrls: ['report.dialog.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    IonButton,
    IonButtons,
    IonContent,
    IonFooter,
    IonHeader,
    IonTitle,
    IonToolbar,
    ReportViewComponent,
  ],
})
export class ReportDialog {
  readonly #modal = inject(ModalController);

  readonly report = input.required<ClientReport>();

  protected openFolder(folder: string): void {
    void this.#modal.dismiss(folder, 'folder');
  }

  protected close(): void {
    void this.#modal.dismiss(undefined, 'close');
  }
}
