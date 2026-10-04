// ─── why ────────────────────────────────────────────────────────
// Step three, the approval: what the update WILL write, per sheet — the paste's own line, its
// notes, and every cell that changes. It is a real write into an in-memory
// copy of the workbook, diffed before and after, so this is the result and not
// a prediction of it. „Übernehmen“ runs the same code once more and saves it
// as the master's next version.
//
// „Zuordnung merken“ stores the ticks and the column answers on the master
// bindings, so the next document of this template opens with the same sheets
// and nothing to answer. On by default: remembering is what makes the second
// export one click.
// ────────────────────────────────────────────────────────────────

import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { Router } from '@angular/router';
import {
  IonCard,
  IonCardContent,
  IonCardHeader,
  IonCardSubtitle,
  IonCardTitle,
  IonCheckbox,
  IonItem,
  IonList,
  IonNote,
} from '@ionic/angular/standalone';
import { ReportPresenterService } from '../../../../@shared/feature/report/report-presenter.service';
import { BusyOverlayComponent } from '../../../../@shared/ui/busy-overlay/busy-overlay.component';
import { WizardShellComponent } from '../../../../@shared/ui/wizard-shell/wizard-shell.component';
import { MasterExportFacade } from '../../../data';
import { EXPORT_PHASE, EXPORT_STEPS } from '../../../model/master-export';
import { CellChangesComponent } from '../../ui/cell-changes/cell-changes.component';

@Component({
  selector: 'app-page-export-preview',
  templateUrl: 'export-preview.page.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    BusyOverlayComponent,
    CellChangesComponent,
    IonCard,
    IonCardContent,
    IonCardHeader,
    IonCardSubtitle,
    IonCardTitle,
    IonCheckbox,
    IonItem,
    IonList,
    IonNote,
    WizardShellComponent,
  ],
})
export class ExportPreviewPage {
  protected readonly facade = inject(MasterExportFacade);
  readonly #reports = inject(ReportPresenterService);
  readonly #router = inject(Router);

  protected readonly phase = EXPORT_PHASE;
  protected readonly steps = EXPORT_STEPS;

  protected onRemember(event: Event): void {
    this.facade.setRemember(
      (event as CustomEvent<{ checked: boolean }>).detail.checked
    );
  }

  protected async onBack(): Promise<void> {
    await this.#router.navigate(['/trains/master/export/structure']);
  }

  protected async onWrite(): Promise<void> {
    const ok = await this.#reports.run(() => this.facade.write());
    if (ok) await this.#router.navigate(['/trains/master/export/result']);
  }
}
