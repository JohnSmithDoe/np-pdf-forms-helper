// ─── why ────────────────────────────────────────────────────────
// The left pane: the document list, as a smart leaf.
//
// It owns every command that needs no confirmation and no second dialog:
// adding, remapping, opening, sorting, ticking, renaming. What it CANNOT own is
// anything that first has to ask the user — Sheriff makes `smart-ui` a strict
// leaf, so this component may not open the field dialog or a confirm alert.
// Those four gestures leave as outputs and the page answers them.
//
// A rejected command is emitted on `failed` rather than thrown: the page is the
// only layer allowed to open the message dialog, and a floating promise would
// otherwise turn a German backend message into an unhandled rejection.
//
// The two options are pure screen state and stay here — nothing else in the app
// has an opinion about whether the options bar is folded out.
//
// The pane is an `ion-card` and the row is an `ion-item` with a two-line
// `ion-label`, so every surface, border and type step on this screen is one
// Ionic component's own. `meta` and `origin` are what those two lines say:
// a document row that only shows a file name hides what the app knows about it
// (kind, how many fields are ticked), and a mapped field that only shows its own
// name hides WHAT IT IS MAPPED TO — the one thing this screen exists to set.
// Both are `util/document-labels`, because the export wizard's `selection-list`
// renders the same two lines and a row that read differently in the two modes
// would look like two different apps; the methods here are the template's way in
// to a pure function.
//
// The field-name input carries an `aria-label` instead of a visible `label`.
// The label used to repeat the value verbatim above the box it sits in, three
// times over with the placeholder; the origin note is the useful thing to put
// there.
//
// That name is the ORIGIN and never the mapped name, and the reason is a trap:
// Ionic copies `aria-*` onto the native input ONCE, in `componentWillLoad`, so
// a name derived from a value the user edits goes stale the moment they edit it
// — the box would still answer to the name it had before the rename. The origin
// is fixed for the life of the row, so there is nothing to keep in step. It is
// also the better name: six boxes called "Feldname" are one label read six
// times, while "Feldname für Formularfeld Mieter.Vorname" says which mapping
// the box belongs to.
//
// `expanded` is not decoration. `ion-accordion` renders whatever sits in its
// `slot="content"` IMMEDIATELY; it does not defer. With auto-mapping on that is
// a text field per form field of every PDF, built on load, for documents nobody
// opened. So the content is gated on this signal, fed by the accordion group.
//
// `onExpandedChange` checks `event.target === event.currentTarget` first, and
// that check is load-bearing: `ionChange` is a bubbling, composed event, so
// every checkbox and input inside a panel raises one that reaches the group
// listener — and a checkbox's `detail.value` is its own value, not a document
// id. Unguarded, ticking a document closed the panel the user had open.
//
// `#run` presents only a `BackendError`: `BackendService.call()` routes every
// rejection through its normaliser, so anything else is a programming bug and
// belongs in the global error listener, not in a German dialog.
//
// In the template the header `ion-item` toggles the accordion on click, so the
// document tick stops propagation. That is safe: Ionic's checkbox toggles from a
// listener on the same element, and `stopPropagation` does not cancel listeners
// on the element it is called from — only on ancestors, which is the accordion.
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
  IonInput,
  IonItem,
  IonLabel,
  IonNote,
  IonToggle,
} from '@ionic/angular/standalone';
import { addIcons } from 'ionicons';
import {
  addOutline,
  documentAttachOutline,
  eyeOutline,
  folderOpenOutline,
  openOutline,
  optionsOutline,
  shieldOutline,
  swapVerticalOutline,
  syncOutline,
  trashOutline,
} from 'ionicons/icons';
import { BackendError } from '../../../@shared/data/backend/backend.service';
import { inputValue } from '../../../@shared/util/input-value.util';
import { FillerFacade } from '../../data';
import { FillerDocument, FillerField } from '../../model/filler.types';
import { documentMeta, fieldOrigin } from '../../util/document-labels.util';

@Component({
  selector: 'app-document-list',
  templateUrl: 'document-list.component.html',
  styleUrls: ['document-list.component.scss'],
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
    IonInput,
    IonItem,
    IonLabel,
    IonNote,
    IonToggle,
  ],
})
export class DocumentListComponent {
  readonly #facade = inject(FillerFacade);

  readonly addFieldRequested = output<FillerDocument>();
  readonly removeDocumentRequested = output<FillerDocument>();
  readonly removeFieldRequested = output<FillerField>();
  readonly resetRequested = output<void>();
  readonly failed = output<BackendError>();

  protected readonly documents = this.#facade.documents;
  protected readonly hasDocuments = this.#facade.hasDocuments;
  protected readonly sortDirection = this.#facade.sortDirection;

  protected readonly optionsVisible = signal(false);
  protected readonly autoMapFields = signal(true);

  protected readonly expanded = signal<string[]>([]);

  protected readonly selectionSummary = computed(() => {
    const documents = this.documents();
    const selected = documents.filter((document) => document.selected).length;
    return `${selected} von ${documents.length} für den Export ausgewählt`;
  });

  constructor() {
    addIcons({
      addOutline,
      documentAttachOutline,
      eyeOutline,
      folderOpenOutline,
      openOutline,
      optionsOutline,
      shieldOutline,
      swapVerticalOutline,
      syncOutline,
      trashOutline,
    });
  }

  protected isExpanded(id: string): boolean {
    return this.expanded().includes(id);
  }

  protected meta(document: FillerDocument): string {
    return documentMeta(document);
  }

  protected fieldLabel(document: FillerDocument, field: FillerField): string {
    const source = this.origin(document, field);
    return source ? `Feldname für ${source}` : 'Feldname';
  }

  protected origin(document: FillerDocument, field: FillerField): string {
    return fieldOrigin(document, field);
  }

  protected onExpandedChange(event: Event): void {
    if (event.target !== event.currentTarget) return;

    const { value } = (event as CustomEvent<{ value?: string | string[] }>)
      .detail;
    if (!value) this.expanded.set([]);
    else this.expanded.set(Array.isArray(value) ? value : [value]);
  }

  protected onAddDocument(): void {
    void this.#run(this.#facade.addDocuments(this.autoMapFields(), 'file'));
  }

  protected onAddDocumentFolder(): void {
    void this.#run(this.#facade.addDocuments(this.autoMapFields(), 'folder'));
  }

  protected onRemapDocument(document: FillerDocument, event: Event): void {
    event.stopPropagation();
    void this.#run(this.#facade.remapDocument(document.id));
  }

  protected onOpenDocument(document: FillerDocument, event: Event): void {
    event.stopPropagation();
    void this.#run(this.#facade.openFile(document.filename));
  }

  protected onOpenPreview(document: FillerDocument, event: Event): void {
    event.stopPropagation();
    if (document.type !== 'pdf' || !document.previewfile) return;
    void this.#run(this.#facade.openFile(document.previewfile));
  }

  protected onToggleSort(): void {
    this.#facade.toggleSort();
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

  protected onNameBlur(field: FillerField, event: Event): void {
    const mappedName = inputValue(event);
    if (mappedName === field.mappedName) return;
    void this.#run(this.#facade.renameField(field.origId, mappedName));
  }

  protected onAddField(document: FillerDocument, event: Event): void {
    event.stopPropagation();
    this.addFieldRequested.emit(document);
  }

  protected onRemoveDocument(document: FillerDocument, event: Event): void {
    event.stopPropagation();
    this.removeDocumentRequested.emit(document);
  }

  async #run(command: Promise<void>): Promise<void> {
    try {
      await command;
    } catch (cause) {
      if (!(cause instanceof BackendError)) throw cause;
      this.failed.emit(cause);
    }
  }
}
