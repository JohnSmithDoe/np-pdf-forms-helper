// ─── why ────────────────────────────────────────────────────────
// The last screen before anything is written, and it is the PLAN: per type what
// will be linked, created or left out, and how many rows go in. „Importieren“
// is the apply — one transaction, all of it or nothing — and the document is
// marked imported in that same write, which is what makes it final.
//
// The counts are the walk's answers, not a prediction of the commit's tally:
// the commit can still decline a row (a gate re-checked server-side, an entity
// deleted in the meantime) and says so in its report on the next step.
// ────────────────────────────────────────────────────────────────

import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { Router } from '@angular/router';
import { IonItem, IonLabel, IonList, IonNote } from '@ionic/angular/standalone';
import { ReportPresenterService } from '../../../@shared/feature/report/report-presenter.service';
import { BusyOverlayComponent } from '../../../@shared/ui/busy-overlay/busy-overlay.component';
import { WizardShellComponent } from '../../../@shared/ui/wizard-shell/wizard-shell.component';
import { ImportWalkFacade } from '../../data';
import { IMPORT_PHASE, IMPORT_STEPS } from '../../model/import-walk';

@Component({
  selector: 'app-page-import-summary',
  templateUrl: 'import-summary.page.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    BusyOverlayComponent,
    IonItem,
    IonLabel,
    IonList,
    IonNote,
    WizardShellComponent,
  ],
})
export class ImportSummaryPage {
  protected readonly facade = inject(ImportWalkFacade);
  readonly #reports = inject(ReportPresenterService);
  readonly #router = inject(Router);

  protected readonly phase = IMPORT_PHASE;
  protected readonly steps = IMPORT_STEPS;

  protected async onBack(): Promise<void> {
    await this.#router.navigate(['/trains/import/entries']);
  }

  protected async onCommit(): Promise<void> {
    const ok = await this.#reports.run(() => this.facade.commit());
    if (ok) await this.#router.navigate(['/trains/import/result']);
  }
}
