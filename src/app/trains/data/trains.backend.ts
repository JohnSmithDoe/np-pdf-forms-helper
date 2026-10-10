// ─── why ────────────────────────────────────────────────────────
// The trains half of the wire: which commands exist, what they are called, and
// what goes in their payloads. It sits in the domain rather than in `@shared`
// because a command list is the most domain-specific thing there is — the
// transport there knows none of these names.
//
// The `command` strings and payload keys ARE the `#[tauri::command]` names and
// their parameters, so nothing translates on the way out.
//
// `silent` is taken by the commands whose report the caller presents itself —
// `write_clean` (the cleaning's batch summary), `commit_document` (the
// import's result), `create_trains_export` (the dashboard) and
// `import_master_all` (the master page) — the same
// opt-in per call that `FillerBackend` uses.
// ────────────────────────────────────────────────────────────────

import { inject, Injectable } from '@angular/core';
import {
  BackendService,
  type CallOptions,
} from '../../@shared/data/backend/backend.service';
import type {
  CleanDecisions,
  EntityDecisions,
  EntityRef,
  ImportPlan,
  MasterExportRequest,
  MasterSettings,
  Partner,
  TrainsData,
  TrainsSettings,
  Wagen,
  Radsatz,
} from '../model/trains.types';
import type { Farbe } from '../../@shared/model/farbe.types';

export type TrainsCommand =
  | { command: 'get_trains_data'; payload: Record<string, never> }
  | {
      command: 'query_events';
      payload: { wagenId?: string; offset: number; limit: number };
    }
  | { command: 'stage_import'; payload: Record<string, never> }
  | { command: 'stage_import_path'; payload: { path: string } }
  | { command: 'restage_import'; payload: { plan: ImportPlan } }
  | { command: 'restage_sheet'; payload: { sheet: string } }
  | { command: 'discard_import'; payload: Record<string, never> }
  | { command: 'save_template'; payload: { name: string } }
  | { command: 'stage_document'; payload: { id: string } }
  | { command: 'commit_document'; payload: { decisions: EntityDecisions } }
  | { command: 'save_waggon'; payload: { wagen: Wagen } }
  | { command: 'remove_waggon'; payload: { id: string } }
  | { command: 'save_wheelset'; payload: { radsatz: Radsatz } }
  | { command: 'remove_wheelset'; payload: { id: string } }
  | { command: 'save_partner'; payload: { partner: Partner } }
  | { command: 'remove_partner'; payload: { id: string } }
  | { command: 'remove_template'; payload: { id: string } }
  | { command: 'save_trains_settings'; payload: { settings: TrainsSettings } }
  | { command: 'reset_trains'; payload: Record<string, never> }
  | { command: 'create_trains_export'; payload: Record<string, never> }
  | { command: 'get_master'; payload: Record<string, never> }
  | { command: 'reset_master_bindings'; payload: Record<string, never> }
  | { command: 'save_master'; payload: { settings: MasterSettings } }
  | { command: 'open_master_export'; payload: { id: string } }
  | {
      command: 'preview_master_export';
      payload: { request: MasterExportRequest };
    }
  | {
      command: 'write_master_export';
      payload: { request: MasterExportRequest };
    }
  | { command: 'get_master_sheet'; payload: { sheet: string } }
  | { command: 'get_entity_detail'; payload: { kind: EntityRef; id: string } }
  | { command: 'get_telematik'; payload: Record<string, never> }
  | {
      command: 'set_farbe';
      payload: { kind: EntityRef; id: string; farbe: Farbe | null };
    }
  | { command: 'start_master_import'; payload: Record<string, never> }
  | { command: 'import_master_all'; payload: Record<string, never> }
  | { command: 'stage_master_sheet'; payload: { sheet: string } }
  | { command: 'get_master_file'; payload: Record<string, never> }
  | { command: 'clean_master_file'; payload: Record<string, never> }
  | { command: 'pick_master_target'; payload: Record<string, never> }
  | { command: 'accept_master_file'; payload: Record<string, never> }
  | { command: 'discard_master_file'; payload: Record<string, never> }
  | { command: 'open_output_folder'; payload: { folder: string } }
  | { command: 'open_file'; payload: { filename: string } }
  | { command: 'pick_import_folder'; payload: Record<string, never> }
  | { command: 'pick_import_files'; payload: Record<string, never> }
  | { command: 'scan_import_paths'; payload: { paths: string[] } }
  | {
      command: 'clean_file';
      payload: { path: string; sheet: string; templateId: string };
    }
  | { command: 'reclean_file'; payload: { decisions: CleanDecisions } }
  | { command: 'write_clean'; payload: { decisions: CleanDecisions } }
  | { command: 'discard_clean'; payload: Record<string, never> };

@Injectable({ providedIn: 'root' })
export class TrainsBackend {
  readonly #backend = inject(BackendService);

  getEntityDetail(kind: EntityRef, id: string): Promise<TrainsData> {
    return this.#call({ command: 'get_entity_detail', payload: { kind, id } });
  }

  setFarbe(
    kind: EntityRef,
    id: string,
    farbe: Farbe | undefined
  ): Promise<TrainsData> {
    return this.#call({
      command: 'set_farbe',
      payload: { kind, id, farbe: farbe ?? null },
    });
  }

  getTelematik(): Promise<TrainsData> {
    return this.#call({ command: 'get_telematik', payload: {} });
  }

  getMasterSheet(sheet: string): Promise<TrainsData> {
    return this.#call({ command: 'get_master_sheet', payload: { sheet } });
  }

  #call(command: TrainsCommand, options?: CallOptions): Promise<TrainsData> {
    return this.#backend.call<TrainsData>(command, options);
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

  stageImportPath(path: string): Promise<TrainsData> {
    return this.#call({ command: 'stage_import_path', payload: { path } });
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

  saveTemplate(name: string): Promise<TrainsData> {
    return this.#call({ command: 'save_template', payload: { name } });
  }

  stageDocument(id: string): Promise<TrainsData> {
    return this.#call({ command: 'stage_document', payload: { id } });
  }

  commitDocument(
    decisions: EntityDecisions,
    options?: CallOptions
  ): Promise<TrainsData> {
    return this.#call(
      { command: 'commit_document', payload: { decisions } },
      options
    );
  }

  pickImportFolder(): Promise<TrainsData> {
    return this.#call({ command: 'pick_import_folder', payload: {} });
  }

  pickImportFiles(): Promise<TrainsData> {
    return this.#call({ command: 'pick_import_files', payload: {} });
  }

  scanImportPaths(paths: string[]): Promise<TrainsData> {
    return this.#call({ command: 'scan_import_paths', payload: { paths } });
  }

  cleanFile(
    path: string,
    sheet: string,
    templateId: string
  ): Promise<TrainsData> {
    return this.#call({
      command: 'clean_file',
      payload: { path, sheet, templateId },
    });
  }

  recleanFile(decisions: CleanDecisions): Promise<TrainsData> {
    return this.#call({ command: 'reclean_file', payload: { decisions } });
  }

  writeClean(
    decisions: CleanDecisions,
    options?: CallOptions
  ): Promise<TrainsData> {
    return this.#call(
      { command: 'write_clean', payload: { decisions } },
      options
    );
  }

  discardClean(): Promise<TrainsData> {
    return this.#call({ command: 'discard_clean', payload: {} });
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

  saveSettings(settings: TrainsSettings): Promise<TrainsData> {
    return this.#call({
      command: 'save_trains_settings',
      payload: { settings },
    });
  }

  removeTemplate(id: string): Promise<TrainsData> {
    return this.#call({ command: 'remove_template', payload: { id } });
  }

  reset(): Promise<TrainsData> {
    return this.#call({ command: 'reset_trains', payload: {} });
  }

  createExport(options?: CallOptions): Promise<TrainsData> {
    return this.#call(
      { command: 'create_trains_export', payload: {} },
      options
    );
  }

  loadMaster(): Promise<TrainsData> {
    return this.#call({ command: 'get_master', payload: {} });
  }

  resetMasterBindings(): Promise<TrainsData> {
    return this.#call({ command: 'reset_master_bindings', payload: {} });
  }

  saveMaster(settings: MasterSettings): Promise<TrainsData> {
    return this.#call({ command: 'save_master', payload: { settings } });
  }

  openMasterExport(id: string): Promise<TrainsData> {
    return this.#call({ command: 'open_master_export', payload: { id } });
  }

  previewMasterExport(request: MasterExportRequest): Promise<TrainsData> {
    return this.#call({
      command: 'preview_master_export',
      payload: { request },
    });
  }

  writeMasterExport(request: MasterExportRequest): Promise<TrainsData> {
    return this.#call({
      command: 'write_master_export',
      payload: { request },
    });
  }

  startMasterImport(): Promise<TrainsData> {
    return this.#call({ command: 'start_master_import', payload: {} });
  }

  importMasterAll(options?: CallOptions): Promise<TrainsData> {
    return this.#call({ command: 'import_master_all', payload: {} }, options);
  }

  stageMasterSheet(sheet: string): Promise<TrainsData> {
    return this.#call({ command: 'stage_master_sheet', payload: { sheet } });
  }

  loadMasterFile(): Promise<TrainsData> {
    return this.#call({ command: 'get_master_file', payload: {} });
  }

  cleanMasterFile(): Promise<TrainsData> {
    return this.#call({ command: 'clean_master_file', payload: {} });
  }

  pickMasterTarget(): Promise<TrainsData> {
    return this.#call({ command: 'pick_master_target', payload: {} });
  }

  acceptMasterFile(): Promise<TrainsData> {
    return this.#call({ command: 'accept_master_file', payload: {} });
  }

  discardMasterFile(): Promise<TrainsData> {
    return this.#call({ command: 'discard_master_file', payload: {} });
  }

  // The opener is the filler's command and is deliberately reused: opening a
  // folder is not a trains concept, and a second command would be the same
  // `opener` plugin call under a different name. `open_file` likewise.
  openFolder(folder: string): Promise<TrainsData> {
    return this.#call({ command: 'open_output_folder', payload: { folder } });
  }

  openFile(filename: string): Promise<TrainsData> {
    return this.#call({ command: 'open_file', payload: { filename } });
  }
}
