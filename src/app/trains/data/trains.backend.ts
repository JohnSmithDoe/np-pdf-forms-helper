// ─── why ────────────────────────────────────────────────────────
// The trains half of the wire: which commands exist, what they are called, and
// what goes in their payloads. It sits in the domain rather than in `@shared`
// because a command list is the most domain-specific thing there is — the
// transport there knows none of these names.
//
// The `command` strings and payload keys ARE the `#[tauri::command]` names and
// their parameters, so nothing translates on the way out.
// ────────────────────────────────────────────────────────────────

import { inject, Injectable } from '@angular/core';
import { BackendService } from '../../@shared/data/backend/backend.service';
import type {
  CommitDecisions,
  ImportPlan,
  Partner,
  TrainsData,
  Wagen,
  Radsatz,
} from '../model/trains.types';

export type TrainsCommand =
  | { command: 'get_trains_data'; payload: Record<string, never> }
  | {
      command: 'query_events';
      payload: { wagenId?: string; offset: number; limit: number };
    }
  | { command: 'stage_import'; payload: Record<string, never> }
  | { command: 'restage_import'; payload: { plan: ImportPlan } }
  | { command: 'restage_sheet'; payload: { sheet: string } }
  | { command: 'discard_import'; payload: Record<string, never> }
  | { command: 'commit_import'; payload: { decisions: CommitDecisions } }
  | { command: 'save_waggon'; payload: { wagen: Wagen } }
  | { command: 'remove_waggon'; payload: { id: string } }
  | { command: 'save_wheelset'; payload: { radsatz: Radsatz } }
  | { command: 'remove_wheelset'; payload: { id: string } }
  | { command: 'save_partner'; payload: { partner: Partner } }
  | { command: 'remove_partner'; payload: { id: string } }
  | { command: 'remove_template'; payload: { id: string } }
  | { command: 'reset_trains'; payload: Record<string, never> }
  | { command: 'create_trains_export'; payload: Record<string, never> }
  | { command: 'open_output_folder'; payload: { folder: string } };

@Injectable({ providedIn: 'root' })
export class TrainsBackend {
  readonly #backend = inject(BackendService);

  #call(command: TrainsCommand): Promise<TrainsData> {
    return this.#backend.call<TrainsData>(command);
  }

  load(): Promise<TrainsData> {
    return this.#call({ command: 'get_trains_data', payload: {} });
  }

  queryEvents(
    wagenId: string | undefined,
    offset: number,
    limit: number
  ): Promise<TrainsData> {
    return this.#call({
      command: 'query_events',
      payload: { wagenId, offset, limit },
    });
  }

  stageImport(): Promise<TrainsData> {
    return this.#call({ command: 'stage_import', payload: {} });
  }

  restageImport(plan: ImportPlan): Promise<TrainsData> {
    return this.#call({ command: 'restage_import', payload: { plan } });
  }

  restageSheet(sheet: string): Promise<TrainsData> {
    return this.#call({ command: 'restage_sheet', payload: { sheet } });
  }

  discardImport(): Promise<TrainsData> {
    return this.#call({ command: 'discard_import', payload: {} });
  }

  commitImport(decisions: CommitDecisions): Promise<TrainsData> {
    return this.#call({ command: 'commit_import', payload: { decisions } });
  }

  saveWagen(wagen: Wagen): Promise<TrainsData> {
    return this.#call({ command: 'save_waggon', payload: { wagen } });
  }

  removeWagen(id: string): Promise<TrainsData> {
    return this.#call({ command: 'remove_waggon', payload: { id } });
  }

  saveRadsatz(radsatz: Radsatz): Promise<TrainsData> {
    return this.#call({ command: 'save_wheelset', payload: { radsatz } });
  }

  removeRadsatz(id: string): Promise<TrainsData> {
    return this.#call({ command: 'remove_wheelset', payload: { id } });
  }

  savePartner(partner: Partner): Promise<TrainsData> {
    return this.#call({ command: 'save_partner', payload: { partner } });
  }

  removePartner(id: string): Promise<TrainsData> {
    return this.#call({ command: 'remove_partner', payload: { id } });
  }

  removeTemplate(id: string): Promise<TrainsData> {
    return this.#call({ command: 'remove_template', payload: { id } });
  }

  reset(): Promise<TrainsData> {
    return this.#call({ command: 'reset_trains', payload: {} });
  }

  createExport(): Promise<TrainsData> {
    return this.#call({ command: 'create_trains_export', payload: {} });
  }

  // The opener is the filler's command and is deliberately reused: opening a
  // folder is not a trains concept, and a second command would be the same
  // `opener` plugin call under a different name.
  openFolder(folder: string): Promise<TrainsData> {
    return this.#call({ command: 'open_output_folder', payload: { folder } });
  }
}
