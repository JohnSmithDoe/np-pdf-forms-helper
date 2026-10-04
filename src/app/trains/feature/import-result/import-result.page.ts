// ─── why ────────────────────────────────────────────────────────
// What the commit did, read out of the parked report rather than caught off
// `report$` — the same reason filler's result steps exist. The exit is the next
// queued document when „Alle importieren“ started the walk, because the queue
// is the point; „Fertig“ leads to the document list, where the user sees the
// document now marked imported. A master import queues its sheets the same way
// and ends on the master page, where the run shows as complete or not.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
} from '@angular/core';
import { Router } from '@angular/router';
import { IonButton } from '@ionic/angular/standalone';
import { ReportPresenterService } from '../../../@shared/feature/report/report-presenter.service';
import { BusyOverlayComponent } from '../../../@shared/ui/busy-overlay/busy-overlay.component';
import { ReportViewComponent } from '../../../@shared/ui/report-view/report-view.component';
import { WizardShellComponent } from '../../../@shared/ui/wizard-shell/wizard-shell.component';
import { ImportWalkFacade } from '../../data';
import { IMPORT_PHASE, IMPORT_STEPS } from '../../model/import-walk';

@Component({
  selector: 'app-page-import-result',
  templateUrl: 'import-result.page.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    BusyOverlayComponent,
    IonButton,
    ReportViewComponent,
    WizardShellComponent,
  ],
})
export class ImportResultPage {
  protected readonly facade = inject(ImportWalkFacade);
  readonly #reports = inject(ReportPresenterService);
  readonly #router = inject(Router);

  protected readonly phase = IMPORT_PHASE;
  protected readonly steps = IMPORT_STEPS;
  protected readonly hasNext = computed(() => this.facade.queue().length > 0);
  protected readonly home = computed(() =>
    this.facade.source()?.kind === 'master'
      ? '/trains/master'
      : '/trains/documents'
  );

  protected async onNext(): Promise<void> {
    if (!this.hasNext()) {
      await this.onDone();
      return;
    }
    const home = this.home();
    let started = false;
    const ok = await this.#reports.run(async () => {
      started = await this.facade.next();
    });
    await this.#router.navigate([
      ok && started ? '/trains/import/partners' : home,
    ]);
  }

  protected async onDone(): Promise<void> {
    const home = this.home();
    this.facade.finish();
    await this.#router.navigate([home]);
  }
}
