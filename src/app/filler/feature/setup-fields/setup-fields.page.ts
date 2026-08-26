// ─── why ────────────────────────────────────────────────────────
// Step 2 of the setup wizard, and the detail page behind step 2's list. ONE
// component on two routes — np-commlink's `data: { … }` pattern, the same one
// `trains.routes.ts` uses for owners and workshops — because the two differ only
// in which document is shown and where Zurück goes, and a second component would
// be this file with two lines changed.
//
// Which document: the `:id` param when the route has one, otherwise the single
// document the import produced. That fallback is what makes the one-file branch
// need no list — `importedDocuments()` has exactly one entry, and the guard has
// already established it is not empty.
//
// It owns the modal and the alert because `FieldEditorComponent` may not: a
// `smart-ui` leaf cannot compose another, so it emits "the user asked to add a
// field" and the composition point decides that means a dialog. Identical
// arrangement to `FillerPage`, for the identical reason.
//
// A field can be added, renamed and removed here, but a document cannot be
// removed — that is not an unfinished thought. This step is "check what we
// found in this file"; throwing the file away again belongs on the expert page,
// where the whole library is in front of the user rather than one row of it.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
} from '@angular/core';
import { ActivatedRoute, Router } from '@angular/router';
import { toSignal } from '@angular/core/rxjs-interop';
import { BackendError } from '../../../@shared/data/backend/backend.service';
import { OverlayService } from '../../../@shared/data/overlays/overlay.service';
import { ReportPresenterService } from '../../../@shared/feature/report/report-presenter.service';
import type { MappedField } from '../../../@shared/model/document.types';
import { BusyOverlayComponent } from '../../../@shared/ui/busy-overlay/busy-overlay.component';
import { FillerFacade } from '../../data';
import type { FillerDocument, FillerField } from '../../model/filler.types';
import { FieldDialog } from '../../smart-ui/field-dialog/field.dialog';
import { FieldEditorComponent } from '../../smart-ui/field-editor/field-editor.component';
import { WizardShellComponent } from '../../ui/wizard-shell/wizard-shell.component';

@Component({
  selector: 'app-page-setup-fields',
  templateUrl: 'setup-fields.page.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [BusyOverlayComponent, FieldEditorComponent, WizardShellComponent],
})
export class SetupFieldsPage {
  protected readonly facade = inject(FillerFacade);
  readonly #reports = inject(ReportPresenterService);
  readonly #overlays = inject(OverlayService);
  readonly #router = inject(Router);
  readonly #route = inject(ActivatedRoute);

  readonly #params = toSignal(this.#route.paramMap);

  protected readonly documentId = computed(
    () => this.#params()?.get('id') ?? null
  );

  protected readonly document = computed<FillerDocument | undefined>(() => {
    const imported = this.facade.importedDocuments();
    const id = this.documentId();
    return id ? imported.find((entry) => entry.id === id) : imported[0];
  });

  protected onBack(): void {
    void this.#router.navigate([
      this.documentId() ? '/documents/setup/documents' : '/documents/setup',
    ]);
  }

  protected onNext(): void {
    void this.#router.navigate([
      this.documentId()
        ? '/documents/setup/documents'
        : '/documents/setup/result',
    ]);
  }

  protected async onAddField(document: FillerDocument): Promise<void> {
    const field = await this.#overlays.openModal<MappedField>(FieldDialog, {
      document,
    });
    if (!field) return;

    await this.#reports.run(async () => {
      await this.facade.addField(document.id, field);
      this.facade.setFieldSelected(field.origId, true);
    });
  }

  protected async onRemoveField(field: FillerField): Promise<void> {
    const question = `Feld ${field.mappedName} wirklich entfernen?`;
    if (!(await this.#overlays.confirm(question))) return;
    await this.#reports.run(() => this.facade.removeField(field.origId));
  }

  protected presentError(error: BackendError): void {
    void this.#reports.showError(error);
  }
}
