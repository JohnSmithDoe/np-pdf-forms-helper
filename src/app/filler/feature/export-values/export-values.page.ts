// ─── why ────────────────────────────────────────────────────────
// Step 2 of the export wizard: one input per mapped name, and nothing else on
// the screen. That is the whole reason the wizard exists — on the expert page
// these inputs share a pane with the selection they derive from, so typing a
// value and changing what it applies to look like the same activity.
//
// Guarded on `hasExportFields`, so this page cannot be reached with nothing to
// type into. An empty value is legal and stays legal: a field left blank writes
// an empty string, which is what somebody printing a form to fill in by hand
// wants. So Weiter is never disabled here.
// ────────────────────────────────────────────────────────────────

import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { Router } from '@angular/router';
import { BusyOverlayComponent } from '../../../@shared/ui/busy-overlay/busy-overlay.component';
import { FillerFacade } from '../../data';
import { ValueFormComponent } from '../../smart-ui/value-form/value-form.component';
import { WizardShellComponent } from '../../ui/wizard-shell/wizard-shell.component';

@Component({
  selector: 'app-page-export-values',
  templateUrl: 'export-values.page.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [BusyOverlayComponent, ValueFormComponent, WizardShellComponent],
})
export class ExportValuesPage {
  protected readonly facade = inject(FillerFacade);
  readonly #router = inject(Router);

  protected onBack(): void {
    void this.#router.navigate(['/documents/wizard/selection']);
  }

  protected onNext(): void {
    void this.#router.navigate(['/documents/wizard/generate']);
  }
}
