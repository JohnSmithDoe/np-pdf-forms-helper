// ─── why ────────────────────────────────────────────────────────
// The last thing shown before anything is written: which documents, into which
// folder, with which values. The wizard's whole reason for existing is that the
// expert page never had this — there, pressing "Dokumente erstellen" is the
// first and last confirmation.
//
// The folder name shown here is `exportFolder()`, which is built from
// `exportStamp` — and that stamp is only re-read when a run STARTS. So this
// shows the folder of the LAST run until the run button is pressed, which is
// correct: the name it will actually get is minted at that moment, and showing a
// clock ticking in a summary would be a value that changes while it is read.
//
// The suffix input lives here rather than a step of its own, because it names
// the folder this step is about to create — moving it away leaves the user
// naming something they cannot see.
// ────────────────────────────────────────────────────────────────

import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { IonInput, IonNote } from '@ionic/angular/standalone';
import { inputValue } from '../../../@shared/util/input-value.utility';
import { FillerFacade } from '../../data';

@Component({
  selector: 'app-run-summary',
  templateUrl: 'run-summary.component.html',
  styleUrls: ['run-summary.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [IonInput, IonNote],
})
export class RunSummaryComponent {
  readonly #facade = inject(FillerFacade);

  protected readonly exportDocuments = this.#facade.exportDocuments;
  protected readonly exportFields = this.#facade.exportFields;
  protected readonly exportSuffix = this.#facade.exportSuffix;

  protected setExportSuffix(event: Event): void {
    this.#facade.setExportSuffix(inputValue(event));
  }
}
