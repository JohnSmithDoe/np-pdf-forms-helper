// ─── why ────────────────────────────────────────────────────────
// The filler's whole API. Components read signals off it and call commands on
// it; nothing outside `filler/data` sees the store, the channels, or
// `@ngrx/signals` at all — which is what lets the state library be swapped, or
// the store split, without touching a template.
//
// Every command follows one shape: send the request, feed the answer back
// through `applyClientData`. There is no broadcast to subscribe to and no
// `error$` — a failed command REJECTS with a `BackendError` whose `messages`
// are already the German lines for the dialog, so the caller that triggered it
// is the one that hears about it.
//
// Nothing is written into the store ahead of the response. The backend owns the
// documents and echoes the whole list back, so an optimistic write would be
// overwritten on success and stranded on failure — a removed row still gone, a
// renamed field still renamed, with only a dialog saying otherwise. The store's
// `documentWith…` builders exist for exactly that: they produce the payload
// without touching state.
//
// It injects the transport as well as the domain backend, for the two signals
// that belong to no domain: `busy` counts every command in flight and `report$`
// carries whatever any of them attached. Reading them off `FillerBackend` would
// mean forwarding app-wide state through a domain that does not own it.
//
// `resetApp` clears the typed values by hand, and only AFTER the reset happened:
// they are keyed by something the backend never sees, so no response can empty
// them. `openOutputFolder` takes the run subfolder from a `ClientReport`, not
// the output root — passing '' opens the parent and loses the run the report is
// about.
//
// Two commands take `silent`, and it is the CALLER's choice rather than the
// command's: the same import is a toast on the expert page and the setup
// wizard's whole result step. When silenced the report is parked in the matching
// store slice instead, which is where the result step reads it from.
//
// `importDocuments` reads the document ids BEFORE the call so the store can diff
// them afterwards. The response carries the whole list and marks nothing as new,
// and the two halves have to happen in that order — which is why `applyImport`
// takes them together rather than leaving a caller to remember.
//
// That diff is the ONLY reason it is not the same method as `addDocuments`. Both
// take a `DocumentSource`; the loud one just applies the response, the wizard's
// one silences the report and works out what arrived. Two names for two
// behaviours, not one per source — a `addDocumentFolder` wrapper would be a
// third name for a literal.
// ────────────────────────────────────────────────────────────────

import { inject, Injectable } from '@angular/core';
import { BackendService } from '../../@shared/data/backend/backend.service';
import { MappedField } from '../../@shared/model/document.types';
import { DocumentSource } from '../model/filler.types';
import { FillerBackend } from './filler.backend';
import { FillerStore } from './filler.store';

@Injectable({ providedIn: 'root' })
export class FillerFacade {
  readonly #backend = inject(FillerBackend);
  readonly #transport = inject(BackendService);
  readonly #store = inject(FillerStore);

  readonly busy = this.#transport.busy;
  readonly report$ = this.#transport.report$.asObservable();

  readonly documents = this.#store.documentViews;
  readonly hasDocuments = this.#store.hasDocuments;
  readonly profiles = this.#store.profiles;
  readonly selectedProfileId = this.#store.selectedProfileId;
  readonly sortDirection = this.#store.sortDirection;

  readonly exportSuffix = this.#store.exportSuffix;
  readonly exportFolder = this.#store.exportFolder;
  readonly exportFields = this.#store.exportFields;
  readonly exportDocuments = this.#store.exportDocuments;
  readonly hasExportFields = this.#store.hasExportFields;

  readonly loaded = this.#store.loaded;
  readonly setupReport = this.#store.setupReport;
  readonly runReport = this.#store.runReport;
  readonly importedDocuments = this.#store.importedDocuments;

  async load(): Promise<void> {
    this.#store.applyClientData(await this.#backend.getClientData());
  }

  async addDocuments(
    autoMapFields: boolean,
    source: DocumentSource
  ): Promise<void> {
    this.#store.applyClientData(
      await this.#backend.addDocuments(autoMapFields, source)
    );
  }

  async importDocuments(
    autoMapFields: boolean,
    source: DocumentSource
  ): Promise<void> {
    const previousIds = this.#store.documents().map((document) => document.id);
    const data = await this.#backend.addDocuments(autoMapFields, source, {
      silent: true,
    });
    this.#store.applyImport(data, previousIds);
  }

  async remapDocument(id: string): Promise<void> {
    this.#store.applyClientData(await this.#backend.remapDocument(id));
  }

  async removeDocument(id: string): Promise<void> {
    this.#store.applyClientData(await this.#backend.removeDocument(id));
  }

  async addField(documentId: string, field: MappedField): Promise<void> {
    const document = this.#store.documentWithField(documentId, field);
    if (!document) return;
    this.#store.applyClientData(await this.#backend.saveDocument(document));
  }

  async renameField(origId: string, mappedName: string): Promise<void> {
    const document = this.#store.documentWithRenamedField(origId, mappedName);
    if (!document) return;
    this.#store.applyClientData(await this.#backend.saveDocument(document));
  }

  async removeField(origId: string): Promise<void> {
    const document = this.#store.documentWithoutField(origId);
    if (!document) return;
    this.#store.applyClientData(await this.#backend.saveDocument(document));
  }

  async resetApp(): Promise<void> {
    this.#store.applyClientData(await this.#backend.resetApp());
    this.#store.clearFieldValues();
  }

  toggleSort(): void {
    this.#store.toggleSort();
  }

  setDocumentSelected(id: string, selected: boolean): void {
    this.#store.setDocumentSelected(id, selected);
  }

  setFieldSelected(origId: string, selected: boolean): void {
    this.#store.setFieldSelected(origId, selected);
  }

  setFieldValue(mappedName: string, value: string): void {
    this.#store.setFieldValue(mappedName, value);
  }

  setExportSuffix(suffix: string): void {
    this.#store.setExportSuffix(suffix);
  }

  async createDocuments({ silent = false } = {}): Promise<void> {
    this.#store.stampExport();
    const documentIds = this.#store
      .exportDocuments()
      .map((document) => document.id);
    const data = await this.#backend.createDocuments(
      this.#store.exportFolder(),
      documentIds,
      this.#store.exportFields(),
      { silent }
    );
    this.#store.applyClientData(data);
    if (silent) this.#store.setRunReport(data.message ?? null);
  }

  startOver(): void {
    this.#store.clearSelection();
  }

  async openFile(filename: string): Promise<void> {
    await this.#backend.openFile(filename);
  }

  async openOutputFolder(folder = ''): Promise<void> {
    await this.#backend.openOutputFolder(folder);
  }

  selectProfile(id: string): void {
    this.#store.selectProfile(id);
  }

  async addProfile(name: string): Promise<void> {
    this.#store.applyClientData(
      await this.#backend.saveProfiles(this.#store.addProfile(name))
    );
  }

  async saveProfile(): Promise<void> {
    this.#store.applyClientData(
      await this.#backend.saveProfiles(this.#store.saveProfile())
    );
  }

  async removeProfile(): Promise<void> {
    this.#store.applyClientData(
      await this.#backend.saveProfiles(this.#store.removeProfile())
    );
  }
}
