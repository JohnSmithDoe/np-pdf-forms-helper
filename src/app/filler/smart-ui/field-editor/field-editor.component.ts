// ─── why ────────────────────────────────────────────────────────
// One document's mapped fields, renameable and removable. The setup wizard's
// mapping step, and the detail page behind its imported list.
//
// It takes the document as an INPUT, which is what separates it from
// `DocumentListComponent`: the list reads every document off the facade and owns
// the sort, the add buttons and the export ticks, none of which belong on a step
// that is about one file. Giving the list an "only this one" input would have
// put a wizard's concern inside the expert page's component and left both
// harder to read.
//
// Renames go through the facade because that is a command; adding and removing
// a field are EMITTED, because both need an overlay — a modal to name the new
// field, an alert to confirm the removal — and `smart-ui` is a strict leaf that
// may not compose either. The step page owns them, exactly as `FillerPage` does.
//
// A rename fires on BLUR and only when the text actually changed, the same rule
// the expert list uses: on every keystroke it would be one save per character.
//
// `type: 'resource'` has no fields by construction — it is copied, not filled —
// so it gets a sentence rather than an empty list that looks like a failure.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  inject,
  input,
  output,
} from '@angular/core';
import {
  IonButton,
  IonIcon,
  IonInput,
  IonNote,
} from '@ionic/angular/standalone';
import { addIcons } from 'ionicons';
import { addOutline, trashOutline } from 'ionicons/icons';
import { BackendError } from '../../../@shared/data/backend/backend.service';
import { inputValue } from '../../../@shared/util/input-value.util';
import { FillerFacade } from '../../data';
import type { FillerDocument, FillerField } from '../../model/filler.types';

@Component({
  selector: 'app-field-editor',
  templateUrl: 'field-editor.component.html',
  styleUrls: ['field-editor.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [IonButton, IonIcon, IonInput, IonNote],
})
export class FieldEditorComponent {
  readonly #facade = inject(FillerFacade);

  readonly document = input.required<FillerDocument>();

  readonly addFieldRequested = output<FillerDocument>();
  readonly removeFieldRequested = output<FillerField>();
  readonly failed = output<BackendError>();

  constructor() {
    addIcons({ addOutline, trashOutline });
  }

  protected onNameBlur(field: FillerField, event: Event): void {
    const mappedName = inputValue(event);
    if (mappedName === field.mappedName) return;
    void this.#run(this.#facade.renameField(field.origId, mappedName));
  }

  async #run(command: Promise<void>): Promise<void> {
    try {
      await command;
    } catch (error) {
      if (!(error instanceof BackendError)) throw error;
      this.failed.emit(error);
    }
  }
}
