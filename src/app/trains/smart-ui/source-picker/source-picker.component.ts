// ─── why ────────────────────────────────────────────────────────
// Stage one: which file, which sheet, and where its header is.
//
// The detected header row is shown WITH ITS CONFIDENCE and its runner-up,
// because detection is a proposal and never a silent decision. A user who can
// see "Zeile 4, Sicherheit 78 %" can tell at a glance whether to look closer;
// one who is shown nothing has to check every time.
//
// Changing the header row goes through the backend rather than being applied
// locally: the file is already held there, so a restage is a reparse of memory
// and the preview underneath stays the single source of what was read.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  inject,
  output,
} from '@angular/core';
import {
  IonButton,
  IonChip,
  IonIcon,
  IonInput,
  IonItem,
  IonLabel,
  IonList,
  IonNote,
  IonSelect,
  IonSelectOption,
} from '@ionic/angular/standalone';
import { addIcons } from 'ionicons';
import { cloudUploadOutline, documentOutline } from 'ionicons/icons';
import { BackendError } from '../../../@shared/data/backend/backend.service';
import { inputValue } from '../../../@shared/util/input-value.util';
import { ImportFacade } from '../../data';

@Component({
  selector: 'app-source-picker',
  templateUrl: 'source-picker.component.html',
  styleUrls: ['source-picker.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    IonButton,
    IonChip,
    IonIcon,
    IonInput,
    IonItem,
    IonLabel,
    IonList,
    IonNote,
    IonSelect,
    IonSelectOption,
  ],
})
export class SourcePickerComponent {
  protected readonly facade = inject(ImportFacade);
  readonly failed = output<BackendError>();

  constructor() {
    addIcons({ cloudUploadOutline, documentOutline });
  }

  protected async pick(): Promise<void> {
    await this.#run(() => this.facade.pickFile());
  }

  protected async onSheet(event: Event): Promise<void> {
    const sheet = (event as CustomEvent<{ value: string }>).detail.value;
    await this.#run(() => this.facade.selectSheet(sheet));
  }

  protected async onHeaderRow(event: Event): Promise<void> {
    const value = Number(inputValue(event));
    if (!Number.isFinite(value) || value < 1) return;
    await this.#run(() => this.facade.setHeaderRow(value));
  }

  async #run(action: () => Promise<void>): Promise<void> {
    try {
      await action();
    } catch (error) {
      if (error instanceof BackendError) this.failed.emit(error);
      else throw error;
    }
  }
}
