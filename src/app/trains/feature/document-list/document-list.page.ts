// ─── why ────────────────────────────────────────────────────────
// The documents the app owns — every file whose cleaning was filed — and the
// one place an import can be started from long after the cleaning. It is the
// load ledger made visible: bereinigt am, importiert am, and both copies.
//
// A plain list rather than the shared list shell: that shell's rows are
// `BaseItem`s that open on a click and carry no actions, and every row here
// has five (its folder, two copies to open, an import and a master export to start). A second
// kind of row in the shell would be the shell growing a mode.
//
// „Master aktualisieren“ is hidden for now behind `fullEnabled` (`npdh.full`):
// the master is not written from the app until that switch is on. It opens the
// master update wizard for one document:
// sheets, columns, a cell-by-cell preview to approve, then the summary. It is
// offered for every document, imported or not — the master and the
// Schattensystem are separate targets — and only while a client master has been
// taken over, because the result is that master's next version.
//
// The customer's MASTER FILE is pinned on top, apart from the documents: there
// is only ever one, it is never imported by the walk nor exported into itself,
// so its row offers its copies and the way to its versions and nothing else.
//
// „Importieren“ writes into the Schattensystem and nowhere else — the master
// is only ever written by „Master aktualisieren“. An imported document offers
// no „Importieren“, and that is only the explanation: `stage_document` and
// `commit_document` refuse it themselves.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
} from '@angular/core';
import { Router } from '@angular/router';
import {
  IonBackButton,
  IonButton,
  IonButtons,
  IonChip,
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
import { SettingsService } from '../../../@shared/data/settings/settings.service';
import { ReportPresenterService } from '../../../@shared/feature/report/report-presenter.service';
import { BusyOverlayComponent } from '../../../@shared/ui/busy-overlay/busy-overlay.component';
import {
  ImportWalkFacade,
  MasterExportFacade,
  MasterFileFacade,
  TrainsFacade,
} from '../../data';

@Component({
  selector: 'app-page-document-list',
  templateUrl: 'document-list.page.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { class: 'ion-page' },
  imports: [
    BusyOverlayComponent,
    IonBackButton,
    IonButton,
    IonButtons,
    IonChip,
    IonContent,
    IonHeader,
    IonItem,
    IonLabel,
    IonList,
    IonListHeader,
    IonNote,
    IonTitle,
    IonToolbar,
  ],
})
export class DocumentListPage {
  protected readonly trains = inject(TrainsFacade);
  protected readonly masterFile = inject(MasterFileFacade);
  readonly #walk = inject(ImportWalkFacade);
  readonly #export = inject(MasterExportFacade);
  readonly #reports = inject(ReportPresenterService);
  readonly #router = inject(Router);
  protected readonly fullEnabled = inject(SettingsService).fullEnabled;

  protected readonly dokumente = computed(() =>
    [...(this.trains.dokumente() ?? [])].sort(
      (a, b) =>
        b.bereinigtAm.localeCompare(a.bereinigtAm) ||
        a.name.localeCompare(b.name)
    )
  );

  protected onOpen(path: string): void {
    void this.#reports.run(() => this.trains.openFile(path));
  }

  protected onOpenFolder(folder: string): void {
    void this.#reports.run(() => this.trains.openFolder(folder));
  }

  protected async onImport(id: string): Promise<void> {
    const ok = await this.#reports.run(() => this.#walk.start(id));
    if (ok) await this.#router.navigate(['/trains/import/partners']);
  }

  protected async onExport(id: string): Promise<void> {
    const ok = await this.#reports.run(() => this.#export.begin(id));
    if (ok) await this.#router.navigate(['/trains/master/export/sheets']);
  }

  protected onMasterFile(): void {
    void this.#router.navigate(['/trains/master-file']);
  }

  protected onClean(): void {
    void this.#router.navigate(['/trains/clean']);
  }
}
