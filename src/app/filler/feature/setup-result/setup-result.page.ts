// ─── why ────────────────────────────────────────────────────────
// Step 3 of the setup wizard: what the import actually did, read out of
// `setupReport` rather than caught off `report$`.
//
// That is why the import is called with `silent`. The report used to be a toast
// or a `ReportDialog`, and both would land ON TOP of this page saying the same
// thing — a folder import produces one line per file, which is exactly the shape
// that earns a dialog.
//
// The body is `ReportViewComponent`, the same component `ReportDialog` renders,
// so the two cannot drift into two ways of showing one report.
//
// The two exits are the two things somebody actually does next: link more
// documents, or go and fill some in. There is no "Fertig" — finishing is
// arriving somewhere, not dismissing something.
// ────────────────────────────────────────────────────────────────

import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { Router } from '@angular/router';
import { IonButton } from '@ionic/angular/standalone';
import { ReportPresenterService } from '../../../@shared/feature/report/report-presenter.service';
import { BusyOverlayComponent } from '../../../@shared/ui/busy-overlay/busy-overlay.component';
import { ReportViewComponent } from '../../../@shared/ui/report-view/report-view.component';
import { FillerFacade } from '../../data';
import { WizardShellComponent } from '../../ui/wizard-shell/wizard-shell.component';

@Component({
  selector: 'app-page-setup-result',
  templateUrl: 'setup-result.page.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    BusyOverlayComponent,
    IonButton,
    ReportViewComponent,
    WizardShellComponent,
  ],
})
export class SetupResultPage {
  protected readonly facade = inject(FillerFacade);
  readonly #reports = inject(ReportPresenterService);
  readonly #router = inject(Router);

  protected onMore(): void {
    void this.#router.navigate(['/documents/setup']);
  }

  protected onFill(): void {
    void this.#router.navigate(['/documents/wizard/selection']);
  }

  protected onOpenFolder(folder: string): void {
    void this.#reports.run(() => this.facade.openOutputFolder(folder));
  }
}
