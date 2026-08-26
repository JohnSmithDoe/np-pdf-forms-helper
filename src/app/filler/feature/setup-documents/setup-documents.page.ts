// ─── why ────────────────────────────────────────────────────────
// Step 2 of the setup wizard's MULTI branch: what the import actually linked,
// one row per document, each one a way into its fields.
//
// Mapping is optional here and that is the whole point of the branch. Somebody
// who pointed at a folder of forty forms is not going to name every field in all
// of them before they are allowed to continue, and a wizard that insisted would
// be worse than the expert page it exists to soften. Weiter is therefore always
// live; the rows are an invitation.
//
// The list is `importedDocuments()` and not every document, so a second folder
// import shows the second folder. That is the slice's whole reason for existing:
// the response carries the full library and says nothing about what is new.
//
// Field counts are shown because they are the one thing that distinguishes a
// document worth opening from one that is already fine — a `resource` has none
// by construction, and a form the auto-mapping found nothing in is the case the
// user has to be told about rather than left to discover.
//
// Rows are `@shared/ui/base-item`'s `ListItemComponent`, the same leaf the trains
// lists use, with `canDelete` off: a setup step offers no swipe-to-delete, and
// the alternative was hand-rolling an `ion-item` that would drift from every
// other row in the app. The whole PAGE is not `@shared/feature/item-lists` — that
// is a self-contained page shell owning its own `.ion-page`, so it cannot nest
// inside the wizard shell, and it brings a searchbar and infinite scroll this
// three-row step has no use for.
// ────────────────────────────────────────────────────────────────

import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { Router } from '@angular/router';
import { IonIcon, IonList, IonNote } from '@ionic/angular/standalone';
import { addIcons } from 'ionicons';
import { chevronForwardOutline, documentTextOutline } from 'ionicons/icons';
import { ListItemComponent } from '../../../@shared/ui/base-item/list-item/list-item.component';
import { BusyOverlayComponent } from '../../../@shared/ui/busy-overlay/busy-overlay.component';
import { FillerFacade } from '../../data';
import type { FillerDocument } from '../../model/filler.types';
import { WizardShellComponent } from '../../ui/wizard-shell/wizard-shell.component';

@Component({
  selector: 'app-page-setup-documents',
  templateUrl: 'setup-documents.page.html',
  styleUrls: ['setup-documents.page.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    BusyOverlayComponent,
    IonIcon,
    IonList,
    IonNote,
    ListItemComponent,
    WizardShellComponent,
  ],
})
export class SetupDocumentsPage {
  protected readonly facade = inject(FillerFacade);
  readonly #router = inject(Router);

  constructor() {
    addIcons({ chevronForwardOutline, documentTextOutline });
  }

  protected onBack(): void {
    void this.#router.navigate(['/documents/setup']);
  }

  protected onNext(): void {
    void this.#router.navigate(['/documents/setup/result']);
  }

  protected onOpen(document: FillerDocument): void {
    void this.#router.navigate(['/documents/setup/documents', document.id]);
  }
}
