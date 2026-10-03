// ─── why ────────────────────────────────────────────────────────
// The Schattensystem's own settings. A page of its own rather than a card on
// the dashboard, so the next setting has somewhere to go.
//
// The Wagennummer spelling is the first: some users read `33 85 0659 152-2`,
// some `338506591522`, and one setting decides it EVERYWHERE the Schattensystem
// shows or writes a number — the lists, the import's cards, the cleaned copies
// and both exports. Compact is the default. It is stored in the backend, beside
// the data, because Rust writes the files and every desk on that data folder
// should see the same spelling.
//
// What it does NOT do is rewrite what already exists: a cleaned copy filed
// earlier keeps the spelling it was written in, and both spellings read back to
// the same twelve digits, so nothing that matches on them is affected.
// ────────────────────────────────────────────────────────────────

import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import {
  IonBackButton,
  IonButtons,
  IonContent,
  IonHeader,
  IonItem,
  IonLabel,
  IonList,
  IonListHeader,
  IonNote,
  IonRadio,
  IonRadioGroup,
  IonTitle,
  IonToolbar,
} from '@ionic/angular/standalone';
import { ReportPresenterService } from '../../../@shared/feature/report/report-presenter.service';
import { BusyOverlayComponent } from '../../../@shared/ui/busy-overlay/busy-overlay.component';
import { TrainsFacade } from '../../data';
import type { UicStyle } from '../../model/trains.types';

interface StyleOption {
  value: UicStyle;
  example: string;
  label: string;
}

const STYLES: readonly StyleOption[] = [
  { value: 'compact', example: '338506591522', label: 'Ziffern am Stück' },
  {
    value: 'grouped',
    example: '33 85 0659 152-2',
    label: 'Gruppiert wie am Wagen angeschrieben',
  },
];

@Component({
  selector: 'app-page-trains-settings',
  templateUrl: 'settings.page.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { class: 'ion-page' },
  imports: [
    BusyOverlayComponent,
    IonBackButton,
    IonButtons,
    IonContent,
    IonHeader,
    IonItem,
    IonLabel,
    IonList,
    IonListHeader,
    IonNote,
    IonRadio,
    IonRadioGroup,
    IonTitle,
    IonToolbar,
  ],
})
export class TrainsSettingsPage {
  protected readonly facade = inject(TrainsFacade);
  readonly #reports = inject(ReportPresenterService);

  protected readonly styles = STYLES;

  protected onStyle(event: Event): void {
    const value = (event as CustomEvent<{ value?: UicStyle }>).detail.value;
    if (!value || value === this.facade.settings().wagennummer) return;
    void this.#reports.run(() =>
      this.facade.saveSettings({
        ...this.facade.settings(),
        wagennummer: value,
      })
    );
  }
}
