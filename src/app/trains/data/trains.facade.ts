// ─── why ────────────────────────────────────────────────────────
// The trains domain's whole public API. Components read signals off it and call
// commands on it; nothing outside `trains/data` sees the store, the backend or
// `@ngrx/signals`.
//
// Every command has one shape: send the request, feed the answer through
// `applyTrainsData`. Nothing is written ahead of the response — the backend owns
// the entities and echoes the list back, so an optimistic write would be
// overwritten on success and stranded on failure.
//
// `masterSheet` is the one exception: it RETURNS the view and stores nothing.
// A sheet view belongs to the one page that shows it, and up to 1,700 rows of
// display strings would otherwise outlive it in the store.
// ────────────────────────────────────────────────────────────────

import { inject, Injectable } from '@angular/core';
import { BackendService } from '../../@shared/data/backend/backend.service';
import type { ClientReport } from '../../@shared/model/client.types';
import type {
  MasterSettings,
  MasterSheetView,
  Partner,
  TrainsSettings,
  Wagen,
} from '../model/trains.types';
import { TrainsBackend } from './trains.backend';
import { TrainsStore } from './trains.store';

const PAGE = 100;

@Injectable({ providedIn: 'root' })
export class TrainsFacade {
  readonly #backend = inject(TrainsBackend);
  readonly #transport = inject(BackendService);
  readonly #store = inject(TrainsStore);

  readonly busy = this.#transport.busy;
  readonly report$ = this.#transport.report$.asObservable();

  readonly loaded = this.#store.loaded;
  readonly wagen = this.#store.wagen;
  readonly radsaetze = this.#store.radsaetze;
  readonly einbauten = this.#store.einbauten;
  readonly partners = this.#store.partners;
  readonly templates = this.#store.templates;
  readonly dokumente = this.#store.dokumente;
  readonly settings = this.#store.settings;
  readonly master = this.#store.master;
  readonly masterImportRun = this.#store.masterImportRun;
  readonly events = this.#store.events;
  readonly eventTotal = this.#store.eventTotal;
  readonly counts = this.#store.counts;
  readonly partnerById = this.#store.partnerById;
  readonly wagenById = this.#store.wagenById;

  async load(): Promise<void> {
    this.#store.applyTrainsData(await this.#backend.load());
  }

  async loadEvents(offset = 0): Promise<void> {
    this.#store.applyTrainsData(
      await this.#backend.queryEvents(undefined, offset, PAGE)
    );
  }

  async saveWagen(wagen: Wagen): Promise<void> {
    this.#store.applyTrainsData(await this.#backend.saveWagen(wagen));
  }

  async removeWagen(id: string): Promise<void> {
    this.#store.applyTrainsData(await this.#backend.removeWagen(id));
    this.#store.clearEvents();
    await this.loadEvents();
  }

  async removeRadsatz(id: string): Promise<void> {
    this.#store.applyTrainsData(await this.#backend.removeRadsatz(id));
  }

  async savePartner(partner: Partner): Promise<void> {
    this.#store.applyTrainsData(await this.#backend.savePartner(partner));
  }

  async removePartner(id: string): Promise<void> {
    this.#store.applyTrainsData(await this.#backend.removePartner(id));
  }

  async saveSettings(settings: TrainsSettings): Promise<void> {
    this.#store.applyTrainsData(await this.#backend.saveSettings(settings));
  }

  async openFile(path: string): Promise<void> {
    await this.#backend.openFile(path);
  }

  async removeTemplate(id: string): Promise<void> {
    this.#store.applyTrainsData(await this.#backend.removeTemplate(id));
  }

  async reset(): Promise<void> {
    this.#store.applyTrainsData(await this.#backend.reset());
    this.#store.clearEvents();
    this.#store.clearStaging();
  }

  async openFolder(folder: string): Promise<void> {
    await this.#backend.openFolder(folder);
  }

  async loadMaster(): Promise<void> {
    this.#store.applyTrainsData(await this.#backend.loadMaster());
  }

  async resetMasterBindings(): Promise<void> {
    this.#store.applyTrainsData(await this.#backend.resetMasterBindings());
  }

  async saveMaster(settings: MasterSettings): Promise<void> {
    this.#store.applyTrainsData(await this.#backend.saveMaster(settings));
  }

  async startMasterImport(): Promise<string[]> {
    const data = await this.#backend.startMasterImport();
    this.#store.applyTrainsData(data);
    return data.master?.settings.importRun?.sheets ?? [];
  }

  async masterSheet(sheet: string): Promise<MasterSheetView | undefined> {
    return (await this.#backend.getMasterSheet(sheet)).masterSheet;
  }

  async createExport(): Promise<ClientReport | undefined> {
    const data = await this.#backend.createExport({ silent: true });
    this.#store.applyTrainsData(data);
    return data.message;
  }
}
