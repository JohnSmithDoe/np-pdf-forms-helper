// ─── why ────────────────────────────────────────────────────────
// Step one of the master update: which sheets the document goes into. The base
// is always the client master's current version, so it is named, not offered.
// The ticks start on Rust's suggestion — every sheet the document's data would
// change: the template's own sheet, and any sheet with a key that shares a
// column with the document — and say why, so a pre-ticked sheet is never a
// mystery.
//
// Every sheet can be ticked; Rust sorts the suggested ones first. A ticked sheet
// carries „Neue Zeilen anhängen“: the update is incremental either way, the
// toggle only decides whether a key the sheet lacks becomes a row. Rust turns it
// on for the template's own sheet only, so a project list does not grow every
// Wagen of a telematics export.
//
// The label sits INSIDE `ion-checkbox`, so the whole row toggles it; a separate
// `ion-label` beside a slotted checkbox leaves only the box itself clickable.
// ────────────────────────────────────────────────────────────────

import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { Router } from '@angular/router';
import {
  IonCheckbox,
  IonChip,
  IonItem,
  IonLabel,
  IonList,
  IonListHeader,
  IonNote,
  IonToggle,
} from '@ionic/angular/standalone';
import { ReportPresenterService } from '../../../../@shared/feature/report/report-presenter.service';
import { BusyOverlayComponent } from '../../../../@shared/ui/busy-overlay/busy-overlay.component';
import { WizardShellComponent } from '../../../../@shared/ui/wizard-shell/wizard-shell.component';
import { MasterExportFacade } from '../../../data';
import { EXPORT_PHASE, EXPORT_STEPS } from '../../../model/master-export';

@Component({
  selector: 'app-page-export-sheets',
  templateUrl: 'export-sheets.page.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    BusyOverlayComponent,
    IonCheckbox,
    IonChip,
    IonItem,
    IonLabel,
    IonList,
    IonListHeader,
    IonNote,
    IonToggle,
    WizardShellComponent,
  ],
})
export class ExportSheetsPage {
  protected readonly facade = inject(MasterExportFacade);
  readonly #reports = inject(ReportPresenterService);
  readonly #router = inject(Router);

  protected readonly phase = EXPORT_PHASE;
  protected readonly steps = EXPORT_STEPS;

  protected onTick(sheet: string, event: Event): void {
    this.facade.tick(
      sheet,
      (event as CustomEvent<{ checked: boolean }>).detail.checked
    );
  }

  protected onAppend(sheet: string, event: Event): void {
    this.facade.setAppend(
      sheet,
      (event as CustomEvent<{ checked: boolean }>).detail.checked
    );
  }

  protected async onBack(): Promise<void> {
    this.facade.finish();
    await this.#router.navigate(['/trains/documents']);
  }

  protected async onNext(): Promise<void> {
    const ok = await this.#reports.run(() => this.facade.run());
    if (ok) await this.#router.navigate(['/trains/master/export/structure']);
  }
}
