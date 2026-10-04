// ─── why ────────────────────────────────────────────────────────
// The customer's master workbook as a file: pick it, read what the cleaning
// changed, take it over. Its own page and its own walk on purpose — the
// Bereinigen hub cleans a sender's file through a template and asks per
// column; the master has no template, and its cleaning asks nothing because it
// only does what cannot change a meaning (`master_file::clean` in Rust).
//
// Plan/apply: „Master-Datei wählen“ copies and cleans and shows the result as
// PENDING; nothing is the current master until „Übernehmen“. A new pick
// replaces an untaken one. Only one master exists, so taking over makes the
// pick the latest version and the previous ones history.
//
// No import and no export here: the master is never walked into the
// Schattensystem as a Dokument, nor pasted into itself.
// ────────────────────────────────────────────────────────────────

import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
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
  IonItem,
  IonLabel,
  IonList,
  IonListHeader,
  IonNote,
  IonTitle,
  IonToolbar,
} from '@ionic/angular/standalone';
import { ReportPresenterService } from '../../../@shared/feature/report/report-presenter.service';
import { BusyOverlayComponent } from '../../../@shared/ui/busy-overlay/busy-overlay.component';
import { MasterFileFacade, TrainsFacade } from '../../data';
import { MasterFileReportComponent } from '../../ui/master-file-report/master-file-report.component';

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
    IonItem,
    IonLabel,
    IonList,
    IonListHeader,
    IonNote,
    IonTitle,
    IonToolbar,
    MasterFileReportComponent,
  ],
})
export class MasterFilePage {
  protected readonly facade = inject(MasterFileFacade);
  protected readonly trains = inject(TrainsFacade);
  readonly #reports = inject(ReportPresenterService);

  constructor() {
    void this.#reports.run(() => this.facade.load());
  }

  protected onPick(): void {
    void this.#reports.run(() => this.facade.clean());
  }

  protected onAccept(): void {
    void this.#reports.run(() => this.facade.accept());
  }

  protected onDiscard(): void {
    void this.#reports.run(() => this.facade.discard());
  }

  protected onOpen(path: string): void {
    void this.#reports.run(() => this.trains.openFile(path));
  }

  protected onOpenFolder(folder: string): void {
    void this.#reports.run(() => this.trains.openFolder(folder));
  }
}
