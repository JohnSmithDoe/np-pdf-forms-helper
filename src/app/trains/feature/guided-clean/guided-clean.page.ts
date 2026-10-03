// ─── why ────────────────────────────────────────────────────────
// Step 1 of a file's walk, and the tricky part of the app: what cleaning the
// ORIGINAL would change, before anything is written. Three tiers, three
// sections, in the order they need the user:
//   • Fehler — unreadable cells. Correct or leave empty; each one blocks.
//   • Deutungen — columns that read two ways, or cells read with a caveat.
//     Confirm each; each one blocks. Never resolved without a click.
//   • Formatierungen — lossless rewrites, for information only.
// Weiter stays dead until the first two are zero, and the summary bar on top
// says why. The backend re-checks the same gate in `write_clean`; the disabled
// button is the explanation, not the authority.
//
// Every decision is a round trip (`reclean_file` with the WHOLE decision set):
// a correction is re-parsed in Rust and a confirmed reading re-reads the column,
// and either can move the counts. Doing that locally would mean a second parser.
//
// The pieces are `ui` components that emit, and this page is what turns an
// emission into a command — `smart-ui` is a strict leaf and four of them could
// not be composed into one review.
//
// „Verwerfen“ asks first: by this point the user may have corrected twenty
// cells, and the backend keeps none of it once the cleaning is discarded.
// ────────────────────────────────────────────────────────────────

import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { Router } from '@angular/router';
import {
  IonIcon,
  IonItem,
  IonLabel,
  IonList,
  IonNote,
} from '@ionic/angular/standalone';
import { addIcons } from 'ionicons';
import { documentOutline } from 'ionicons/icons';
import { OverlayService } from '../../../@shared/data/overlays/overlay.service';
import { ReportPresenterService } from '../../../@shared/feature/report/report-presenter.service';
import { BusyOverlayComponent } from '../../../@shared/ui/busy-overlay/busy-overlay.component';
import { WizardShellComponent } from '../../../@shared/ui/wizard-shell/wizard-shell.component';
import { IntakeFacade } from '../../data';
import type { Confirmation } from '../../model/trains.types';
import { CleanSummaryComponent } from '../../ui/clean-summary/clean-summary.component';
import { DeutungCardComponent } from '../../ui/deutung-card/deutung-card.component';
import {
  FehlerListComponent,
  type CellCorrection,
} from '../../ui/fehler-list/fehler-list.component';
import { FormatListComponent } from '../../ui/format-list/format-list.component';

@Component({
  selector: 'app-page-guided-clean',
  templateUrl: 'guided-clean.page.html',
  styleUrls: ['guided-clean.page.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    BusyOverlayComponent,
    CleanSummaryComponent,
    DeutungCardComponent,
    FehlerListComponent,
    FormatListComponent,
    IonIcon,
    IonItem,
    IonLabel,
    IonList,
    IonNote,
    WizardShellComponent,
  ],
})
export class GuidedCleanPage {
  protected readonly facade = inject(IntakeFacade);
  readonly #reports = inject(ReportPresenterService);
  readonly #overlays = inject(OverlayService);
  readonly #router = inject(Router);

  constructor() {
    addIcons({ documentOutline });
  }

  protected async onCorrect({
    row,
    column,
    value,
  }: CellCorrection): Promise<void> {
    await this.#reports.run(() => this.facade.correct(row, column, value));
  }

  protected async onConfirm(confirmation: Confirmation): Promise<void> {
    await this.#reports.run(() => this.facade.confirm(confirmation));
  }

  protected async onWrite(): Promise<void> {
    const ok = await this.#reports.run(() => this.facade.writeClean());
    if (ok) await this.#router.navigate(['/trains/import/guided/preview']);
  }

  protected async onDiscard(): Promise<void> {
    const confirmed = await this.#overlays.confirm(
      'Bereinigung dieser Datei verwerfen? Ihre Korrekturen gehen verloren.'
    );
    if (!confirmed) return;
    const ok = await this.#reports.run(() => this.facade.discardClean());
    if (ok) await this.#router.navigate(['/trains/import']);
  }
}
