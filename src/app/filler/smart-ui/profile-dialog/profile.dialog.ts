// ─── why ────────────────────────────────────────────────────────
// Asks for a profile name and nothing else. It takes no props and touches no
// facade — the caller owns what happens with the name, which is what keeps
// `ProfileBar` (a smart leaf that may not import this) out of it.
//
// The name is a signal fed from the input event rather than `ngModel`: the app
// is signals-first and zoneless, and one text field does not earn a forms
// module. Empty is not a name, so `Okay` stays disabled until there is one.
//
// Cancel and confirm both dismiss; only confirm carries data. The page reads
// the DATA, not the role, so a cancel is `undefined` and cannot be mistaken for
// an empty name.
//
// Opens without props: `modalCtrl.create({ component: ProfileDialog })`.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  inject,
  signal,
} from '@angular/core';
import {
  IonButton,
  IonButtons,
  IonContent,
  IonHeader,
  IonInput,
  IonTitle,
  IonToolbar,
  ModalController,
} from '@ionic/angular/standalone';
import { inputValue } from '../../../@shared/util/input-value.util';

@Component({
  selector: 'app-profile-dialog',
  templateUrl: 'profile.dialog.html',
  styleUrls: ['profile.dialog.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    IonButton,
    IonButtons,
    IonContent,
    IonHeader,
    IonInput,
    IonTitle,
    IonToolbar,
  ],
})
export class ProfileDialog {
  readonly #modal = inject(ModalController);

  protected readonly name = signal('');

  protected setName(event: Event): void {
    this.name.set(inputValue(event));
  }

  protected confirm(): void {
    const name = this.name().trim();
    if (name) void this.#modal.dismiss(name, 'confirm');
  }

  protected cancel(): void {
    void this.#modal.dismiss(undefined, 'cancel');
  }
}
