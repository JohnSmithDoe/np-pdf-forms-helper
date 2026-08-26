// ─── why ────────────────────────────────────────────────────────
// The wizard, and the only place in the domain that opens an overlay — Sheriff
// makes `smart-ui` a strict leaf, so each stage component emits "the user asked
// for X" and this file decides what that means. Every modal flow in trains is
// readable here, exactly as `filler.page.ts` does it.
//
// ONE routed page with a stage signal rather than a route per step. The run's
// authority lives in the backend and stepping back must not discard it; a route
// per step would make the browser's back button a data-loss control.
//
// NOTHING here sets `:host { display: block }`. `.ion-page` is `display: flex`
// with a column direction, and a component style overriding that stops
// `ion-content` flexing — the content then takes the full page height, and the
// footer is pushed below the fold where it cannot be clicked.
//
// Reports go through `ReportPresenterService`, which owns the shared rule — a
// folder to open or more than one line is a dialog, anything else a toast. It
// RETURNS the folder rather than opening it, because `@shared` may not reach a
// domain's facade; the export report carries a `messageFolder`, so it gets the
// dialog with "Ordner öffnen" without this file knowing that.
//
// `AlertController` stays for the template prompt alone: `OverlayService.confirm`
// answers a yes/no, and this alert has an input to read. That prompt is offered
// rather than demanded — an empty name means "do not save one", which is the
// right default for a file that will never arrive again.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  computed,
  DestroyRef,
  inject,
} from '@angular/core';
import { takeUntilDestroyed } from '@angular/core/rxjs-interop';
import {
  AlertController,
  IonBackButton,
  IonButton,
  IonButtons,
  IonContent,
  IonFooter,
  IonHeader,
  IonIcon,
  IonSegment,
  IonSegmentButton,
  IonTitle,
  IonToolbar,
} from '@ionic/angular/standalone';
import { addIcons } from 'ionicons';
import { cloudUploadOutline } from 'ionicons/icons';
import type { BackendError } from '../../../@shared/data/backend/backend.service';
import { ReportPresenterService } from '../../../@shared/feature/report/report-presenter.service';
import type { ClientReport } from '../../../@shared/model/client.types';
import { BusyOverlayComponent } from '../../../@shared/ui/busy-overlay/busy-overlay.component';
import { ImportFacade, TrainsFacade, type Stage } from '../../data';
import { ColumnMapperComponent } from '../../smart-ui/column-mapper/column-mapper.component';
import { RowPreviewComponent } from '../../smart-ui/row-preview/row-preview.component';
import { SourcePickerComponent } from '../../smart-ui/source-picker/source-picker.component';

@Component({
  selector: 'app-page-import',
  templateUrl: 'import.page.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { class: 'ion-page' },
  imports: [
    BusyOverlayComponent,
    ColumnMapperComponent,
    IonBackButton,
    IonButton,
    IonButtons,
    IonContent,
    IonFooter,
    IonHeader,
    IonIcon,
    IonSegment,
    IonSegmentButton,
    IonTitle,
    IonToolbar,
    RowPreviewComponent,
    SourcePickerComponent,
  ],
})
export class ImportPage {
  protected readonly facade = inject(ImportFacade);
  protected readonly trains = inject(TrainsFacade);
  readonly #reports = inject(ReportPresenterService);
  readonly #alerts = inject(AlertController);
  readonly #destroyRef = inject(DestroyRef);

  protected readonly stage = this.facade.stage;
  protected readonly includedCount = computed(
    () => this.facade.includedRows().length
  );
  protected readonly canReview = this.facade.canReview;

  constructor() {
    addIcons({ cloudUploadOutline });
    this.trains.report$
      .pipe(takeUntilDestroyed(this.#destroyRef))
      .subscribe((report) => void this.#showReport(report));
  }

  protected onStage(stage: Stage): void {
    if (stage === 'review' && !this.canReview()) return;
    this.facade.setStage(stage);
  }

  protected async onCommit(): Promise<void> {
    const name = await this.#askTemplateName();
    if (name === undefined) return;
    this.facade.setTemplateName(name);
    await this.#run(() => this.facade.commit());
  }

  protected async onDiscard(): Promise<void> {
    await this.#run(() => this.facade.discard());
  }

  protected async onExport(): Promise<void> {
    await this.#run(() => this.trains.createExport());
  }

  protected presentError(error: BackendError): void {
    void this.#reports.showError(error);
  }

  async #askTemplateName(): Promise<string | undefined> {
    const alert = await this.#alerts.create({
      header: 'Zuordnung als Vorlage speichern?',
      message:
        'Die nächste Datei desselben Absenders wird dann automatisch zugeordnet.',
      inputs: [
        { name: 'name', placeholder: 'z. B. Werkstatt Müller — Monatsliste' },
      ],
      buttons: [
        { text: 'Abbrechen', role: 'cancel' },
        { text: 'Ohne Vorlage', role: 'plain' },
        { text: 'Übernehmen', role: 'confirm' },
      ],
    });
    await alert.present();
    const { role, data } = await alert.onWillDismiss<{
      values: { name: string };
    }>();
    if (role === 'cancel' || role === 'backdrop') return undefined;
    return role === 'confirm' ? (data?.values.name ?? '') : '';
  }

  async #showReport(report: ClientReport): Promise<void> {
    const folder = await this.#reports.show(report);
    if (folder) await this.#run(() => this.trains.openFolder(folder));
  }

  async #run(action: () => Promise<void>): Promise<void> {
    await this.#reports.run(action);
  }
}
