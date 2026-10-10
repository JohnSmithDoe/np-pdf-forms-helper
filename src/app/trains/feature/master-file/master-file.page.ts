// ─── why ────────────────────────────────────────────────────────
// The customer's master workbook, chosen WHERE IT LIES: the MVP writes into
// that very file (`master_file::write_in_place`), so nothing is copied in and
// nothing is cleaned — the customer keeps working in their own file, and every
// write leaves a backup in `Sicherungen/` beside it first.
//
// The older walk — copy in, clean, take over as a version — still exists in the
// backend and the facade for the ERP, but has no button here: two ways to pick
// „the master“ on one page would leave the user guessing which one is written.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
} from '@angular/core';
import {
  IonBackButton,
  IonButton,
  IonButtons,
  IonCard,
  IonCardContent,
  IonCardHeader,
  IonCardSubtitle,
  IonCardTitle,
  IonContent,
  IonHeader,
  IonNote,
  IonTitle,
  IonToolbar,
} from '@ionic/angular/standalone';
import { ReportPresenterService } from '../../../@shared/feature/report/report-presenter.service';
import { BusyOverlayComponent } from '../../../@shared/ui/busy-overlay/busy-overlay.component';
import { MasterFileFacade, TrainsFacade } from '../../data';
import { fileOf, folderOf } from '../../util/path.util';

@Component({
  selector: 'app-page-master-file',
  templateUrl: 'master-file.page.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { class: 'ion-page' },
  imports: [
    BusyOverlayComponent,
    IonBackButton,
    IonButton,
    IonButtons,
    IonCard,
    IonCardContent,
    IonCardHeader,
    IonCardSubtitle,
    IonCardTitle,
    IonContent,
    IonHeader,
    IonNote,
    IonTitle,
    IonToolbar,
  ],
})
export class MasterFilePage {
  protected readonly facade = inject(MasterFileFacade);
  protected readonly trains = inject(TrainsFacade);
  readonly #reports = inject(ReportPresenterService);

  protected readonly name = computed(() => fileOf(this.facade.target() ?? ''));
  protected readonly folder = computed(() =>
    folderOf(this.facade.target() ?? '')
  );

  constructor() {
    void this.#reports.run(() => this.facade.load());
  }

  protected onPick(): void {
    void this.#reports.run(() => this.facade.pickTarget());
  }

  protected onOpen(path: string): void {
    void this.#reports.run(() => this.trains.openFile(path));
  }

  protected onOpenFolder(folder: string): void {
    void this.#reports.run(() => this.trains.openFolder(folder));
  }
}
