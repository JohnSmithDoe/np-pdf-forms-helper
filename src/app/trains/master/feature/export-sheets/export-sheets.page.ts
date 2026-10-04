// ─── why ────────────────────────────────────────────────────────
// Step one of the master update: which sheets the document goes into. The base
// is always the client master's current version, so it is named, not offered. The ticks start on Rust's suggestion — the sheets the
// template was exported to before, or that carry every mapped column — and say
// why, so a pre-ticked sheet is never a mystery.
//
// Every sheet can be ticked; Rust sorts the suggested ones first. A sheet bound
// as an overview without a template carries a warning instead of being
// disabled — the recognition calls pasted exports overviews too, and the
// preview shows every cell before anything is written.
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
} from '@ionic/angular/standalone';
import { ReportPresenterService } from '../../../../@shared/feature/report/report-presenter.service';
import { BusyOverlayComponent } from '../../../../@shared/ui/busy-overlay/busy-overlay.component';
import { WizardShellComponent } from '../../../../@shared/ui/wizard-shell/wizard-shell.component';
import { MasterExportFacade } from '../../../data';
import {
  EXPORT_PHASE,
  EXPORT_STEPS,
  MODE_LABELS,
} from '../../../model/master-export';

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
    WizardShellComponent,
  ],
})
export class ExportSheetsPage {
  protected readonly facade = inject(MasterExportFacade);
  readonly #reports = inject(ReportPresenterService);
  readonly #router = inject(Router);

  protected readonly phase = EXPORT_PHASE;
  protected readonly steps = EXPORT_STEPS;
  protected readonly modes = MODE_LABELS;

  protected onTick(sheet: string, event: Event): void {
    this.facade.tick(
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
