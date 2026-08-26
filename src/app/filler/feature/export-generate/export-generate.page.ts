// ─── why ────────────────────────────────────────────────────────
// Step 3 of the export wizard: the last look before anything is written, and
// the button that writes it.
//
// The run is `silent`, so its report lands in `runReport` instead of on
// `report$`. Left loud it would raise a `ReportDialog` — an export produces one
// line per document, which is exactly the shape `needsDialog` sends to a modal —
// on top of the result page that is about to show the same lines.
//
// Navigation happens only after the run RESOLVES, and only then, because the
// result step's guard reads `runReport`: navigating first would arrive before
// the slice is filled and bounce straight back to step one.
//
// A failure does NOT navigate. `export::run` fails only when the output folder
// is unusable — a per-document failure is a report line, not an error — so there
// is no partial run to show, and the user is left on the step whose button they
// pressed with a dialog saying why.
// ────────────────────────────────────────────────────────────────

import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { Router } from '@angular/router';
import { ReportPresenterService } from '../../../@shared/feature/report/report-presenter.service';
import { BusyOverlayComponent } from '../../../@shared/ui/busy-overlay/busy-overlay.component';
import { FillerFacade } from '../../data';
import { RunSummaryComponent } from '../../smart-ui/run-summary/run-summary.component';
import { WizardShellComponent } from '../../ui/wizard-shell/wizard-shell.component';

@Component({
  selector: 'app-page-export-generate',
  templateUrl: 'export-generate.page.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [BusyOverlayComponent, RunSummaryComponent, WizardShellComponent],
})
export class ExportGeneratePage {
  protected readonly facade = inject(FillerFacade);
  readonly #reports = inject(ReportPresenterService);
  readonly #router = inject(Router);

  protected onBack(): void {
    void this.#router.navigate(['/documents/wizard/values']);
  }

  protected async onGenerate(): Promise<void> {
    const ran = await this.#reports.run(() =>
      this.facade.createDocuments({ silent: true })
    );
    if (!ran) return;
    await this.#router.navigate(['/documents/wizard/result']);
  }
}
