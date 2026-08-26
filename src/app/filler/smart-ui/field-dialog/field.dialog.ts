// ─── why ────────────────────────────────────────────────────────
// Maps one more form field / sheet cell of a document to a name. It is
// `smart-ui` and not `ui` because it reads the facade for ONE thing: the names
// already in use anywhere in the app, which is what the suggestion list offers.
// The dialog derives that list itself, so the page passes only the document.
//
// It dismisses with a `MappedField` and performs nothing — writing it back is
// `addField` on the facade, which the page owns because the page is what opened
// the dialog and is the only layer allowed to present the rejection.
//
// `document` is a signal `input()`, which only works because
// `provideIonicAngular` sets `useSetInputAPI: true` — see app.providers.ts.
// `options` therefore has to be a `computed`, not a field initialised once.
//
// Ionic has no autocomplete, so the mapped-name field is an input with its own
// filtered list underneath. That list is the ONLY thing that makes "same name,
// same value" discoverable — without it a user has to remember the exact
// spelling used on another document — so it is a feature, not chrome. It is
// rendered inline rather than in a popover: a popover over a modal is a second
// overlay to focus-manage, and the list has to survive typing in the field it
// floats over.
//
// A pdf field maps once — `origId` IS the field, so a second mapping of it would
// be two names for one box — while a sheet is never used up: for xlsx `origId`
// is the SHEET and every mapped cell repeats it, so uniqueness belongs to the
// address and testing "already used" there would cap a template at one cell per
// sheet. For xlsx the address IS the mapped name.
//
// It dismisses with the `MappedField` to add, or with nothing on cancel:
//
//   modalCtrl.create({ component: FieldDialog, componentProps: { document } })
//
// `onColumnInput` writes the ELEMENT and not just the signal, or the field keeps
// showing the character the sanitiser rejected.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
  input,
  signal,
} from '@angular/core';
import {
  IonButton,
  IonButtons,
  IonContent,
  IonHeader,
  IonInput,
  IonItem,
  IonLabel,
  IonList,
  IonNote,
  IonSelect,
  IonSelectOption,
  IonTitle,
  IonToolbar,
  ModalController,
} from '@ionic/angular/standalone';
import { MappedField } from '../../../@shared/model/document.types';
import { inputValue } from '../../../@shared/util/input-value.util';
import { FillerFacade } from '../../data';
import { FillerDocument } from '../../model/filler.types';

type FieldOption = { id: string; path: string; disabled: boolean };

function toOptions(document: FillerDocument): FieldOption[] {
  if (document.type === 'pdf') {
    const used = new Set(document.mapped.map((field) => field.origId));
    return document.fields.map((field) => ({
      id: field.id,
      path: field.path,
      disabled: used.has(field.id),
    }));
  }
  if (document.type === 'xlsx') {
    return document.sheets.map((sheet) => ({
      id: sheet.id,
      path: sheet.name,
      disabled: false,
    }));
  }
  return [];
}

@Component({
  selector: 'app-field-dialog',
  templateUrl: 'field.dialog.html',
  styleUrls: ['field.dialog.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    IonButton,
    IonButtons,
    IonContent,
    IonHeader,
    IonInput,
    IonItem,
    IonLabel,
    IonList,
    IonNote,
    IonSelect,
    IonSelectOption,
    IonTitle,
    IonToolbar,
  ],
})
export class FieldDialog {
  readonly #facade = inject(FillerFacade);
  readonly #modal = inject(ModalController);

  readonly document = input.required<FillerDocument>();

  protected readonly type = computed(() => this.document().type);
  protected readonly options = computed(() => toOptions(this.document()));

  protected readonly origId = signal('');
  protected readonly mappedName = signal('');
  protected readonly column = signal('');
  protected readonly row = signal('');

  protected readonly suggesting = signal(false);

  protected readonly cellName = computed(() => {
    const option = this.options().find((entry) => entry.id === this.origId());
    return option ? `$${option.path}.${this.column()}${this.row()}` : '';
  });

  protected readonly duplicate = computed(
    () =>
      this.type() === 'xlsx' &&
      this.document().mapped.some(
        (field) => field.mappedName === this.cellName()
      )
  );

  protected readonly valid = computed(() => {
    if (!this.origId() || !this.mappedName().length) return false;
    return this.type() === 'pdf' || !this.duplicate();
  });

  readonly #knownNames = computed(() => [
    ...new Set(
      this.#facade
        .documents()
        .flatMap((document) => document.mapped)
        .map((field) => field.mappedName)
    ),
  ]);

  protected readonly filteredNames = computed(() => {
    const needle = this.mappedName().toLowerCase();
    const names = this.#knownNames();
    if (!needle) return names;
    return names.filter((name) => name.toLowerCase().includes(needle));
  });

  protected onFieldChosen(id: string): void {
    this.origId.set(id);
    if (this.type() !== 'pdf') return;
    const option = this.options().find((entry) => entry.id === id);
    if (option) this.mappedName.set(option.path);
  }

  protected onNameInput(event: Event): void {
    this.mappedName.set(inputValue(event));
    this.suggesting.set(true);
  }

  protected pickName(name: string): void {
    this.mappedName.set(name);
    this.suggesting.set(false);
  }

  protected onColumnInput(event: Event): void {
    const input = event.target as HTMLInputElement;
    const column = input.value.replace(/[^a-zA-Z]/g, '').toUpperCase();
    input.value = column;
    this.column.set(column);
  }

  protected onRowInput(event: Event): void {
    this.row.set(inputValue(event));
  }

  protected onAdd(): void {
    const field: MappedField = {
      origId: this.origId(),
      mappedName: this.type() === 'pdf' ? this.mappedName() : this.cellName(),
    };
    void this.#modal.dismiss(field, 'add');
  }

  protected cancel(): void {
    void this.#modal.dismiss(undefined, 'cancel');
  }
}
