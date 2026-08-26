// ─── why ────────────────────────────────────────────────────────
// Where wizard mode lands: the fork between "I have documents to set up" and "I
// have documents to fill in". Two jobs that used to be two halves of one screen,
// which is what made that screen hard to face.
//
// It carries the mode toggle, and so does the expert page — the switch has to be
// reachable from wherever you are, or the mode you are not in is the mode you
// cannot leave. Switching WRITES the preference and then navigates, so the next
// start lands where you left off.
//
// The fill card is offered even with nothing set up, rather than hidden. Its
// first step is unguarded and shows the empty state that explains what to do,
// which teaches more than a card that is not there.
// ────────────────────────────────────────────────────────────────

import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { Router } from '@angular/router';
import {
  IonButton,
  IonButtons,
  IonCard,
  IonCardContent,
  IonCardHeader,
  IonCardSubtitle,
  IonCardTitle,
  IonContent,
  IonHeader,
  IonIcon,
  IonMenuButton,
  IonNote,
  IonTitle,
  IonToolbar,
} from '@ionic/angular/standalone';
import { addIcons } from 'ionicons';
import { buildOutline, createOutline, optionsOutline } from 'ionicons/icons';
import { SettingsService } from '../../../@shared/data/settings/settings.service';
import { APP_WORDMARK } from '../../../@shared/model/app.consts';
import { FillerFacade } from '../../data';

@Component({
  selector: 'app-page-wizard-start',
  templateUrl: 'wizard-start.page.html',
  styleUrls: ['wizard-start.page.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { class: 'ion-page' },
  imports: [
    IonButton,
    IonButtons,
    IonCard,
    IonCardContent,
    IonCardHeader,
    IonCardSubtitle,
    IonCardTitle,
    IonContent,
    IonHeader,
    IonIcon,
    IonMenuButton,
    IonNote,
    IonTitle,
    IonToolbar,
  ],
})
export class WizardStartPage {
  protected readonly facade = inject(FillerFacade);
  readonly #settings = inject(SettingsService);
  readonly #router = inject(Router);

  protected readonly wordmark = APP_WORDMARK;

  constructor() {
    addIcons({ buildOutline, createOutline, optionsOutline });
  }

  protected onSetup(): void {
    void this.#router.navigate(['/documents/setup']);
  }

  protected onFill(): void {
    void this.#router.navigate(['/documents/wizard']);
  }

  protected onExpertMode(): void {
    this.#settings.setViewMode('expert');
    void this.#router.navigate(['/documents/expert']);
  }
}
