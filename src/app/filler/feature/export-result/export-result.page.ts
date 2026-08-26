// ─── why ────────────────────────────────────────────────────────
// Step 4: what the run produced, and the two ways on. The report is read out of
// `runReport`, which the run filled because it was called `silent` — this page
// IS the dialog that would otherwise have opened over it.
//
// The two exits are the two different intentions, and they are deliberately not
// one button with a checkbox:
//
//   Erneut erzeugen   keeps the selection AND the typed values, and steps back
//                     to the values so one of them can be changed. The common
//                     case is the same set of forms for the next person, where
//                     most of what was typed still applies.
//   Neu beginnen      clears selection, values, suffix and profile. A fresh
//                     start that left the old values in the boxes would be a
//                     trap the first time somebody trusted it.
//
// "Neu beginnen" asks first. It throws away typed work that nothing else can
// recover — the values were never persisted, by design, because they are keyed
// by mapped name and belong to a run rather than to the documents.
//
// `openOutputFolder` is handed the run's OWN folder off the report, never ''.
// The empty string opens the output root, which is the parent of every run ever
// made, and loses the one this page is about.
// ────────────────────────────────────────────────────────────────

import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { Router } from '@angular/router';
import { IonButton } from '@ionic/angular/standalone';
import { OverlayService } from '../../../@shared/data/overlays/overlay.service';
import { ReportPresenterService } from '../../../@shared/feature/report/report-presenter.service';
import { BusyOverlayComponent } from '../../../@shared/ui/busy-overlay/busy-overlay.component';
import { ReportViewComponent } from '../../../@shared/ui/report-view/report-view.component';
import { FillerFacade } from '../../data';
import { WizardShellComponent } from '../../ui/wizard-shell/wizard-shell.component';

@Component({
  selector: 'app-page-export-result',
  templateUrl: 'export-result.page.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    BusyOverlayComponent,
    IonButton,
    ReportViewComponent,
    WizardShellComponent,
  ],
})
export class ExportResultPage {
  protected readonly facade = inject(FillerFacade);
  protected readonly report = this.facade.runReport;

  readonly #reports = inject(ReportPresenterService);
  readonly #overlays = inject(OverlayService);
  readonly #router = inject(Router);

  protected onAgain(): void {
    void this.#router.navigate(['/documents/wizard/values']);
  }

  protected async onFresh(): Promise<void> {
    const question =
      'Auswahl und alle eingegebenen Werte werden verworfen. Wirklich neu beginnen?';
    if (!(await this.#overlays.confirm(question))) return;
    this.facade.startOver();
    await this.#router.navigate(['/documents/wizard/selection']);
  }

  protected onOpenFolder(folder: string): void {
    void this.#reports.run(() => this.facade.openOutputFolder(folder));
  }
}
