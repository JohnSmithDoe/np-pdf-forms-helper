// ─── why ────────────────────────────────────────────────────────
// Step 3 of a file's walk: what the commit did and where the cleaned copy went,
// read out of the two parked reports rather than caught off `report$` — the
// same reason filler's result steps exist. The cleaned file's report carries the
// `bereinigt-…` folder, so „Ordner öffnen“ is offered under it by
// `ReportViewComponent` without this page knowing.
//
// The exit is the next file when there is one, because a dropped folder is
// usually several files and the walk is the point; „Fertig“ is the footer's
// second button then, and the only one when the list is done. Both go through
// `finish`, which empties the walk's slots, so the result guard cannot let a
// stale report back in.
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
import { IntakeFacade, type IntakeStep } from '../../data';

@Component({
  selector: 'app-page-guided-result',
  templateUrl: 'guided-result.page.html',
  styleUrls: ['guided-result.page.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    BusyOverlayComponent,
    IonButton,
    ReportViewComponent,
    WizardShellComponent,
  ],
})
export class GuidedResultPage {
  protected readonly facade = inject(IntakeFacade);
  readonly #reports = inject(ReportPresenterService);
  readonly #router = inject(Router);

  protected readonly hasNext = computed(() => this.facade.queue().length > 0);

  protected async onNext(): Promise<void> {
    if (!this.hasNext()) {
      await this.onDone();
      return;
    }
    this.facade.finish();
    let step = 'done' as IntakeStep;
    const ok = await this.#reports.run(async () => {
      step = await this.facade.startNext();
    });
    if (!ok || step === 'done') {
      await this.#router.navigate(['/trains/import']);
    } else if (step === 'clean') {
      await this.#router.navigate(['/trains/import/guided/clean']);
    } else {
      await this.#router.navigate(['/trains/import/manual']);
    }
  }

  protected async onDone(): Promise<void> {
    this.facade.finish();
    await this.#router.navigate(['/trains/import']);
  }

  protected onOpenFolder(folder: string): void {
    void this.#reports.run(() => this.facade.openFolder(folder));
  }
}
