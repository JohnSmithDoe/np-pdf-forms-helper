// ─── why ────────────────────────────────────────────────────────
// The documents the app owns — every file whose cleaning was filed — and the
// one place an import can be started from long after the cleaning. It is the
// load ledger made visible: bereinigt am, importiert am, and both copies.
//
// A plain list rather than the shared list shell: that shell's rows are
// `BaseItem`s that open on a click and carry no actions, and every row here
// has four (its folder, two copies to open, one import to start). A second
// kind of row in the shell would be the shell growing a mode.
//
// An imported document offers no „Importieren“, and that is only the
// explanation: `stage_document` and `commit_document` refuse it themselves.
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
  IonNote,
  IonTitle,
  IonToolbar,
} from '@ionic/angular/standalone';
import { ReportPresenterService } from '../../../@shared/feature/report/report-presenter.service';
import { BusyOverlayComponent } from '../../../@shared/ui/busy-overlay/busy-overlay.component';
import { ImportWalkFacade, TrainsFacade } from '../../data';

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
    IonNote,
    IonTitle,
    IonToolbar,
  ],
})
export class DocumentListPage {
  protected readonly trains = inject(TrainsFacade);
  readonly #walk = inject(ImportWalkFacade);
  readonly #reports = inject(ReportPresenterService);
  readonly #router = inject(Router);

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

  protected onClean(): void {
    void this.#router.navigate(['/trains/clean']);
  }
}
