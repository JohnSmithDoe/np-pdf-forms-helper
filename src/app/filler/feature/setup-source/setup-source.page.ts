// ─── why ────────────────────────────────────────────────────────
// Step 1 of the setup wizard: pick what to link. The three ways in are three
// separate native pickers on the Rust side (`DocumentSource`), so they are three
// buttons here rather than one button and a mode — the OS dialog that opens
// differs, and a user who wanted a folder cannot get there from a file dialog.
//
// It is UNGUARDED, and has to be: it is where every other setup step redirects
// when its own precondition fails, so a guard on it would bounce forever.
//
// Where it goes next depends on how many documents actually arrived, which is
// only knowable after the call — one is taken straight to its fields, several go
// to a list to work through. A cancelled picker adds nothing and must not
// navigate: `importedDocuments` stays empty and the user is left on the step
// they were already on, which is what a cancelled dialog should do.
//
// `autoMapFields` defaults ON here, unlike the expert page's hidden option. In a
// wizard the next step is "check the fields we found", and starting that step
// with an empty list would make the automatic mapping look broken.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  inject,
  signal,
} from '@angular/core';
import { Router } from '@angular/router';
import {
  IonButton,
  IonCheckbox,
  IonIcon,
  IonNote,
} from '@ionic/angular/standalone';
import { addIcons } from 'ionicons';
import {
  documentAttachOutline,
  documentsOutline,
  folderOpenOutline,
} from 'ionicons/icons';
import { ReportPresenterService } from '../../../@shared/feature/report/report-presenter.service';
import { BusyOverlayComponent } from '../../../@shared/ui/busy-overlay/busy-overlay.component';
import { FillerFacade } from '../../data';
import type { DocumentSource } from '../../model/filler.types';
import { WizardShellComponent } from '../../ui/wizard-shell/wizard-shell.component';

@Component({
  selector: 'app-page-setup-source',
  templateUrl: 'setup-source.page.html',
  styleUrls: ['setup-source.page.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    BusyOverlayComponent,
    IonButton,
    IonCheckbox,
    IonIcon,
    IonNote,
    WizardShellComponent,
  ],
})
export class SetupSourcePage {
  protected readonly facade = inject(FillerFacade);
  readonly #reports = inject(ReportPresenterService);
  readonly #router = inject(Router);

  protected readonly autoMapFields = signal(true);

  constructor() {
    addIcons({ documentAttachOutline, documentsOutline, folderOpenOutline });
  }

  protected onBack(): void {
    void this.#router.navigate(['/documents/start']);
  }

  protected async onImport(source: DocumentSource): Promise<void> {
    const linked = await this.#reports.run(() =>
      this.facade.importDocuments(this.autoMapFields(), source)
    );
    if (!linked) return;

    const imported = this.facade.importedDocuments();
    if (imported.length === 0) return;
    await this.#router.navigate(
      imported.length === 1
        ? ['/documents/setup/fields']
        : ['/documents/setup/documents']
    );
  }
}
