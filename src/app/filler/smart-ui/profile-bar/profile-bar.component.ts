// ─── why ────────────────────────────────────────────────────────
// The profile toolbar as a smart LEAF. It reads the facade itself, but it must
// never open a dialog: `smart-ui` may not import another smart component, and
// both the name prompt and the delete confirmation ARE components. So the two
// gestures that need one are emitted and `filler.page.ts` opens them; the two
// that need none are executed here.
//
// `selectProfile()` is the ONLY call in this app that re-derives the export
// ticks from a stored profile, and this bar is its only caller. Save, add and
// remove must not refresh the ticks, or renaming a field would throw the user's
// export selection away.
//
// A rejected command is emitted rather than presented: `BackendError` already
// carries the German lines, and the page owns the dialog that shows them.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
  output,
} from '@angular/core';
import {
  IonButton,
  IonIcon,
  IonNote,
  IonSelect,
  IonSelectOption,
} from '@ionic/angular/standalone';
import { addIcons } from 'ionicons';
import { addOutline, saveOutline, trashOutline } from 'ionicons/icons';
import { BackendError } from '../../../@shared/data/backend/backend.service';
import { Profile } from '../../../@shared/model/profile.types';
import { FillerFacade } from '../../data';

@Component({
  selector: 'app-profile-bar',
  templateUrl: 'profile-bar.component.html',
  styleUrls: ['profile-bar.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [IonButton, IonIcon, IonNote, IonSelect, IonSelectOption],
})
export class ProfileBarComponent {
  readonly #facade = inject(FillerFacade);

  constructor() {
    addIcons({ addOutline, saveOutline, trashOutline });
  }

  readonly addProfileRequested = output<void>();
  readonly removeProfileRequested = output<Profile>();
  readonly saveFailed = output<BackendError>();

  protected readonly profiles = this.#facade.profiles;
  protected readonly selectedProfileId = this.#facade.selectedProfileId;
  protected readonly busy = this.#facade.busy;
  protected readonly hasDocuments = this.#facade.hasDocuments;

  protected readonly selectedProfile = computed(() =>
    this.profiles().find((profile) => profile.id === this.selectedProfileId())
  );

  protected selectProfile(id: string): void {
    this.#facade.selectProfile(id);
  }

  protected async saveProfile(): Promise<void> {
    try {
      await this.#facade.saveProfile();
    } catch (cause) {
      if (!(cause instanceof BackendError)) throw cause;
      this.saveFailed.emit(cause);
    }
  }

  protected requestRemove(): void {
    const profile = this.selectedProfile();
    if (profile) this.removeProfileRequested.emit(profile);
  }
}
