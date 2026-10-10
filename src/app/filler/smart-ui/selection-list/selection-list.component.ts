// ─── why ────────────────────────────────────────────────────────
// The export wizard's document list: WHICH templates and fields this run fills,
// and nothing else. It is the read-only half of `document-list`, deliberately a
// second component rather than that one behind a `[selectionOnly]` input.
//
// The split is the point of having two modes at all. The expert page does setup
// and export on one screen; the wizards each do one thing. So a list that offers
// "Datei hinzufügen", "Ordner hinzufügen", the options fold-out with automatic
// field mapping, remapping, renaming, removing and "Alles zurücksetzen" belongs
// to setup and cannot appear in the wizard that only generates — a flag hiding
// six controls would be one component with two modes inside it, which is the
// state the two modes exist to replace.
//
// It emits no command that can fail, so there is no `failed` output and no
// `#run`: every gesture here writes ticks into the store, which cannot reject.
// The one thing it cannot do itself is leave for the setup wizard — a `smart-ui`
// leaf does not navigate — so the empty state emits and the page routes.
//
// There is no "alles auswählen": a saved profile IS the way to tick many
// documents at once, and the profile bar sits directly above this card.
//
// Two behaviours copied over from `document-list` because they are properties of
// `ion-accordion` rather than of either list: content is gated on `expanded`
// (the accordion renders its `slot="content"` immediately, so every field row of
// every document would be built on load), and `onExpandedChange` checks
// `event.target === event.currentTarget` first, because `ionChange` bubbles —
// unguarded, ticking a field closes the panel it sits in.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
  output,
  signal,
} from '@angular/core';
import {
  IonAccordion,
  IonAccordionGroup,
  IonButton,
  IonCard,
  IonCardHeader,
  IonCardTitle,
  IonCheckbox,
  IonIcon,
  IonItem,
  IonLabel,
  IonNote,
} from '@ionic/angular/standalone';
import { addIcons } from 'ionicons';
import { buildOutline, documentAttachOutline } from 'ionicons/icons';
import { FillerFacade } from '../../data';
import { FillerDocument, FillerField } from '../../model/filler.types';
import { documentMeta, fieldOrigin } from '../../util/document-labels.utility';

@Component({
  selector: 'app-selection-list',
  templateUrl: 'selection-list.component.html',
  styleUrls: ['selection-list.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    IonAccordion,
    IonAccordionGroup,
    IonButton,
    IonCard,
    IonCardHeader,
    IonCardTitle,
    IonCheckbox,
    IonIcon,
    IonItem,
    IonLabel,
    IonNote,
  ],
})
export class SelectionListComponent {
  readonly #facade = inject(FillerFacade);

  readonly setupRequested = output<void>();

  protected readonly documents = this.#facade.documents;
  protected readonly hasDocuments = this.#facade.hasDocuments;

  protected readonly expanded = signal<string[]>([]);

  protected readonly selectionSummary = computed(() => {
    const documents = this.documents();
    const selected = documents.filter((document) => document.selected).length;
    return `${selected} von ${documents.length} für den Export ausgewählt`;
  });

  constructor() {
    addIcons({ buildOutline, documentAttachOutline });
  }

  protected isExpanded(id: string): boolean {
    return this.expanded().includes(id);
  }

  protected meta(document: FillerDocument): string {
    return documentMeta(document);
  }

  protected origin(document: FillerDocument, field: FillerField): string {
    return fieldOrigin(document, field);
  }

  protected onExpandedChange(event: Event): void {
    if (event.target !== event.currentTarget) return;

    const { value } = (event as CustomEvent<{ value?: string | string[] }>)
      .detail;
    if (value) {
      this.expanded.set(Array.isArray(value) ? value : [value]);
    } else {
      this.expanded.set([]);
    }
  }

  protected onDocumentSelected(
    document: FillerDocument,
    selected: boolean
  ): void {
    this.#facade.setDocumentSelected(document.id, selected);
  }

  protected onFieldSelected(field: FillerField, selected: boolean): void {
    this.#facade.setFieldSelected(field.origId, selected);
  }
}
