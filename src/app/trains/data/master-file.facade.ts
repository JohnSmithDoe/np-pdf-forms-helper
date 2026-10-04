// ─── why ────────────────────────────────────────────────────────
// The customer's master FILE: pick it, read the cleaning, take it over or drop
// it. Its own facade because it shares no command and no decision with the
// master bindings, mirror or export behind `TrainsFacade`; the state still
// rides `TrainsStore`, because `get_trains_data` carries it to the document
// list too.
//
// A cancelled picker answers without `masterFile`, so the store keeps what it
// held and the page stays where it was.
// ────────────────────────────────────────────────────────────────

import { computed, inject, Injectable } from '@angular/core';
import { TrainsBackend } from './trains.backend';
import { TrainsStore } from './trains.store';

@Injectable({ providedIn: 'root' })
export class MasterFileFacade {
  readonly #backend = inject(TrainsBackend);
  readonly #store = inject(TrainsStore);

  readonly masterFile = this.#store.masterFile;
  readonly current = computed(() => this.#store.masterFile()?.versions[0]);
  readonly older = computed(
    () => this.#store.masterFile()?.versions.slice(1) ?? []
  );
  readonly pending = computed(() => this.#store.masterFile()?.pending);

  async load(): Promise<void> {
    this.#store.applyTrainsData(await this.#backend.loadMasterFile());
  }

  async clean(): Promise<void> {
    this.#store.applyTrainsData(await this.#backend.cleanMasterFile());
  }

  async accept(): Promise<void> {
    this.#store.applyTrainsData(await this.#backend.acceptMasterFile());
  }

  async discard(): Promise<void> {
    this.#store.applyTrainsData(await this.#backend.discardMasterFile());
  }
}
