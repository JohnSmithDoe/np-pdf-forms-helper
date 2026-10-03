// ─── why ────────────────────────────────────────────────────────
// Creating a template from a file nobody has one for, and the only place in
// trains that opens an overlay of its own — Sheriff makes `smart-ui` a strict
// leaf, so each stage component emits "the user asked for X" and this file
// decides what that means.
//
// It IMPORTS NOTHING. An unknown file used to be mapped and committed straight
// into the Schattensystem here, the one path past the cleaning. Now the mapping
// is saved as a template, the user goes back to the cleaning hub, and the file
// — rescanned there — is recognised and cleaned like every other. That is why
// the name is required: a template with no name is a mapping nobody can pick.
//
// ONE routed page with a stage signal rather than a route per step. The staged
// file lives in the backend and stepping back must not discard it; a route per
// step would make the browser's back button a data-loss control.
//
// NOTHING here sets `:host { display: block }`. `.ion-page` is `display: flex`
// with a column direction, and a component style overriding that stops
// `ion-content` flexing — the content then takes the full page height, and the
// footer is pushed below the fold where it cannot be clicked.
//
// `AlertController` stays for the name prompt alone: `OverlayService.confirm`
// answers a yes/no, and this alert has an input to read.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  inject,
} from '@angular/core';
import { takeUntilDestroyed } from '@angular/core/rxjs-interop';
import { Router } from '@angular/router';
import {
  AlertController,
  IonBackButton,
  IonButton,
  IonButtons,
  IonChip,
  IonContent,
  IonFooter,
  IonHeader,
  IonNote,
  IonSegment,
  IonSegmentButton,
  IonTitle,
  IonToolbar,
} from '@ionic/angular/standalone';
import type { BackendError } from '../../../@shared/data/backend/backend.service';
import { ReportPresenterService } from '../../../@shared/feature/report/report-presenter.service';
import type { ClientReport } from '../../../@shared/model/client.types';
import { BusyOverlayComponent } from '../../../@shared/ui/busy-overlay/busy-overlay.component';
import { ImportFacade, TrainsFacade, type Stage } from '../../data';
import { ColumnMapperComponent } from '../../smart-ui/column-mapper/column-mapper.component';
import { SourcePickerComponent } from '../../smart-ui/source-picker/source-picker.component';

@Component({
  selector: 'app-page-template-mapper',
  templateUrl: 'template-mapper.page.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { class: 'ion-page' },
  imports: [
    BusyOverlayComponent,
    ColumnMapperComponent,
    IonBackButton,
    IonButton,
    IonButtons,
    IonChip,
    IonContent,
    IonFooter,
    IonHeader,
    IonNote,
    IonSegment,
    IonSegmentButton,
    IonTitle,
    IonToolbar,
    SourcePickerComponent,
  ],
})
export class TemplateMapperPage {
  protected readonly facade = inject(ImportFacade);
  protected readonly trains = inject(TrainsFacade);
  readonly #reports = inject(ReportPresenterService);
  readonly #alerts = inject(AlertController);
  readonly #router = inject(Router);

  protected readonly stage = this.facade.stage;

  constructor() {
    this.trains.report$
      .pipe(takeUntilDestroyed(inject(DestroyRef)))
      .subscribe((report) => void this.#showReport(report));
  }

  protected onStage(stage: Stage): void {
    if (stage === 'mapping' && !this.facade.staging()) return;
    this.facade.setStage(stage);
  }

  protected async onSave(): Promise<void> {
    const name = await this.#askName();
    if (name === undefined) return;
    const ok = await this.#reports.run(() => this.facade.saveTemplate(name));
    if (ok) await this.#router.navigate(['/trains/clean']);
  }

  protected async onDiscard(): Promise<void> {
    const ok = await this.#reports.run(() => this.facade.discard());
    if (ok && this.facade.handedOver()) {
      await this.#router.navigate(['/trains/clean']);
    }
  }

  protected presentError(error: BackendError): void {
    void this.#reports.showError(error);
  }

  async #askName(): Promise<string | undefined> {
    const alert = await this.#alerts.create({
      header: 'Als Vorlage speichern',
      message:
        'Danach wird die Datei — und jede weitere desselben Aufbaus — mit dieser Vorlage bereinigt.',
      inputs: [
        { name: 'name', placeholder: 'z. B. Werkstatt Müller — Monatsliste' },
      ],
      buttons: [
        { text: 'Abbrechen', role: 'cancel' },
        { text: 'Speichern', role: 'confirm' },
      ],
    });
    await alert.present();
    const { role, data } = await alert.onWillDismiss<{
      values: { name: string };
    }>();
    return role === 'confirm' ? (data?.values.name ?? '') : undefined;
  }

  async #showReport(report: ClientReport): Promise<void> {
    await this.#reports.show(report);
  }
}
