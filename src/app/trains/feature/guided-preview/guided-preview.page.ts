// ─── why ────────────────────────────────────────────────────────
// Step 2 of a file's walk: the CLEANED copy, staged, shown in the same row
// preview the manual import uses. From here on the guided import is an ordinary
// import — the rows resolve against the store the same way and commit through
// the same command — which is the point of cleaning into a file first: the
// import never sees anything the user did not review.
//
// The commit is `silent` and asks no template name. The template was chosen in
// the hub, and the readings confirmed on step 1 are taught to it by the backend
// on commit; a prompt here would ask for something already decided.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
} from '@angular/core';
import { Router } from '@angular/router';
import { IonNote } from '@ionic/angular/standalone';
import { OverlayService } from '../../../@shared/data/overlays/overlay.service';
import { ReportPresenterService } from '../../../@shared/feature/report/report-presenter.service';
import { BusyOverlayComponent } from '../../../@shared/ui/busy-overlay/busy-overlay.component';
import { WizardShellComponent } from '../../../@shared/ui/wizard-shell/wizard-shell.component';
import { ImportFacade, IntakeFacade } from '../../data';
import { RowPreviewComponent } from '../../smart-ui/row-preview/row-preview.component';

@Component({
  selector: 'app-page-guided-preview',
  templateUrl: 'guided-preview.page.html',
  styleUrls: ['guided-preview.page.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    BusyOverlayComponent,
    IonNote,
    RowPreviewComponent,
    WizardShellComponent,
  ],
})
export class GuidedPreviewPage {
  protected readonly facade = inject(IntakeFacade);
  readonly #import = inject(ImportFacade);
  readonly #reports = inject(ReportPresenterService);
  readonly #overlays = inject(OverlayService);
  readonly #router = inject(Router);

  protected readonly includedCount = computed(
    () => this.#import.includedRows().length
  );

  protected async onCommit(): Promise<void> {
    const ok = await this.#reports.run(() => this.facade.commit());
    if (ok) await this.#router.navigate(['/trains/import/guided/result']);
  }

  protected async onDiscard(): Promise<void> {
    const confirmed = await this.#overlays.confirm(
      'Import dieser Datei verwerfen? Die bereinigte Datei bleibt erhalten.'
    );
    if (!confirmed) return;
    const ok = await this.#reports.run(() => this.facade.discardPreview());
    if (ok) await this.#router.navigate(['/trains/import']);
  }
}
