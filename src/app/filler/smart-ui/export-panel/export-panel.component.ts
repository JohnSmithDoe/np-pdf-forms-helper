// ─── why ────────────────────────────────────────────────────────
// The right-hand half of the screen, as a leaf. Everything it shows is a
// derivation off `FillerFacade`, so the panel cannot go stale.
//
// It does NOT open the report dialog, and that is enforced rather than chosen:
// `smart-ui` is a strict leaf, so one smart component may never compose
// another. A failed command is therefore EMITTED (`failed`) instead of being
// presented here — the page owns every dialog on the screen, which is also the
// only place that can decide whether two commands failing at once should show
// one dialog or two.
//
// `exportDocuments` is the store's list rather than a second filter over
// `documents()`: two definitions of "the export set" and the panel shows a set
// the export does not write. It is also narrower — it does not re-derive when a
// FIELD checkbox changes.
//
// It deliberately does NOT show `exportFolder()` any more. That string is the
// raw `Date.now()` stamp, and the panel showed it as the folder the run would
// write — twice wrong: an epoch number is not a name a user can look for in the
// Explorer, and `createDocuments` re-stamps before it reads it, so the one on
// screen was never the one created. What the user needs is the RULE (a new
// timestamped folder per run, plus the suffix), which is now the suffix input's
// helper text; the actual folder arrives with the report, which has a button
// that opens it.
//
// `summary()` counts the filled values rather than only the documents, because
// an export whose fields are empty writes empty documents and the count is the
// only warning before the run. It reads `exportFields`, so it re-derives per
// keystroke — that is what it is for.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
  output,
} from '@angular/core';
import {
  IonButton,
  IonCard,
  IonCardHeader,
  IonCardTitle,
  IonChip,
  IonIcon,
  IonInput,
  IonItem,
  IonLabel,
  IonListHeader,
  IonNote,
} from '@ionic/angular/standalone';
import { addIcons } from 'ionicons';
import {
  checkboxOutline,
  documentsOutline,
  folderOpenOutline,
} from 'ionicons/icons';
import { BackendError } from '../../../@shared/data/backend/backend.service';
import { inputValue } from '../../../@shared/util/input-value.util';
import { FillerFacade } from '../../data';

@Component({
  selector: 'app-export-panel',
  templateUrl: 'export-panel.component.html',
  styleUrls: ['export-panel.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    IonButton,
    IonCard,
    IonCardHeader,
    IonCardTitle,
    IonChip,
    IonIcon,
    IonInput,
    IonItem,
    IonLabel,
    IonListHeader,
    IonNote,
  ],
})
export class ExportPanelComponent {
  readonly #facade = inject(FillerFacade);

  readonly failed = output<BackendError>();

  constructor() {
    addIcons({ checkboxOutline, documentsOutline, folderOpenOutline });
  }

  protected readonly busy = this.#facade.busy;
  protected readonly hasDocuments = this.#facade.hasDocuments;
  protected readonly exportFields = this.#facade.exportFields;
  protected readonly exportSuffix = this.#facade.exportSuffix;

  protected readonly exportDocuments = this.#facade.exportDocuments;

  protected readonly hasSelection = computed(
    () => this.exportDocuments().length > 0
  );

  protected readonly summary = computed(() => {
    const documents = this.exportDocuments().length;
    const documentPart = `${documents} ${documents === 1 ? 'Vorlage' : 'Vorlagen'} ausgewählt`;

    const fields = this.exportFields();
    if (!fields.length) return documentPart;

    const filled = fields.filter((field) => field.value.trim()).length;
    return `${documentPart} · ${filled} von ${fields.length} Werten gefüllt`;
  });

  protected setExportSuffix(event: Event): void {
    this.#facade.setExportSuffix(inputValue(event));
  }

  protected setFieldValue(mappedName: string, event: Event): void {
    this.#facade.setFieldValue(mappedName, inputValue(event));
  }

  protected createDocuments(): void {
    this.#run(this.#facade.createDocuments());
  }

  protected openOutputFolder(): void {
    this.#run(this.#facade.openOutputFolder());
  }

  #run(command: Promise<void>): void {
    void command.catch((cause: unknown) => {
      if (!(cause instanceof BackendError)) throw cause;
      this.failed.emit(cause);
    });
  }
}
