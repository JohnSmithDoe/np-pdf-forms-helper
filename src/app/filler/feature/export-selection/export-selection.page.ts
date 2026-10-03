// ─── why ────────────────────────────────────────────────────────
// Step 1 of the export wizard: which documents and which of their fields, or a
// saved profile that answers both at once.
//
// It composes `SelectionListComponent` and NOT the expert page's
// `DocumentListComponent`, which is the whole reason the wizard exists. That one
// carries setup — adding a file or a folder, the options fold-out with automatic
// field mapping, remapping, renaming, removing, "Alles zurücksetzen" — and a
// wizard for filling documents that also links and unlinks them is the expert
// page again, one screen at a time. Linking happens in the setup wizard; this
// step only ticks. So the ONLY commands left here are the profile ones, and
// every gesture the old page answered with a confirm alert went with the list.
//
// `SelectionListComponent` still writes the same ticks off the same facade, so
// there is no second definition of "the export set" — which is the bug
// `exportDocuments` exists to prevent.
//
// The list's empty state has nowhere to go on its own: with no document linked
// this step cannot be completed and the picker belongs to setup, so the button
// leaves for that wizard and this page is what routes.
//
// UNGUARDED, because it is where every other export step redirects.
//
// Weiter is dead until at least one FIELD is ticked, not one document. A
// document with nothing ticked contributes no inputs, so the value step would be
// blank and the run would copy the original unchanged — which is a legal thing
// to want and precisely not what a wizard should let happen silently.
// ────────────────────────────────────────────────────────────────

import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { Router } from '@angular/router';
import { BackendError } from '../../../@shared/data/backend/backend.service';
import { OverlayService } from '../../../@shared/data/overlays/overlay.service';
import { ReportPresenterService } from '../../../@shared/feature/report/report-presenter.service';
import type { Profile } from '../../../@shared/model/profile.types';
import { BusyOverlayComponent } from '../../../@shared/ui/busy-overlay/busy-overlay.component';
import { FillerFacade } from '../../data';
import { ProfileBarComponent } from '../../smart-ui/profile-bar/profile-bar.component';
import { ProfileDialog } from '../../smart-ui/profile-dialog/profile.dialog';
import { SelectionListComponent } from '../../smart-ui/selection-list/selection-list.component';
import { WizardShellComponent } from '../../../@shared/ui/wizard-shell/wizard-shell.component';

@Component({
  selector: 'app-page-export-selection',
  templateUrl: 'export-selection.page.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    BusyOverlayComponent,
    ProfileBarComponent,
    SelectionListComponent,
    WizardShellComponent,
  ],
})
export class ExportSelectionPage {
  protected readonly facade = inject(FillerFacade);
  readonly #reports = inject(ReportPresenterService);
  readonly #overlays = inject(OverlayService);
  readonly #router = inject(Router);

  protected onBack(): void {
    void this.#router.navigate(['/documents/start']);
  }

  protected onNext(): void {
    void this.#router.navigate(['/documents/wizard/values']);
  }

  protected onSetup(): void {
    void this.#router.navigate(['/documents/setup']);
  }

  protected async onAddProfile(): Promise<void> {
    const name = await this.#overlays.openModal<string>(ProfileDialog);
    if (!name) return;
    await this.#reports.run(() => this.facade.addProfile(name));
  }

  protected async onRemoveProfile(profile: Profile): Promise<void> {
    const question = `Export Profil ${profile.name} wirklich entfernen?`;
    if (!(await this.#overlays.confirm(question))) return;
    await this.#reports.run(() => this.facade.removeProfile());
  }

  protected presentError(error: BackendError): void {
    void this.#reports.showError(error);
  }
}
