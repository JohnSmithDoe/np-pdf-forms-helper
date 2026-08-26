// ─── why ────────────────────────────────────────────────────────
// The filler's half of the wire: which commands exist, what they are called,
// and what goes in their payloads. It sits in the domain rather than in
// `@shared` because a command list is the most domain-specific thing there is —
// `@shared/data/backend` carries the transport, and knows none of these names.
//
// The `command` strings and payload keys ARE the `#[tauri::command]` names and
// their parameters, so nothing translates on the way out.
//
// Every method answers with `ClientData`, including the two openers that change
// no state: one response shape means the caller never branches on which command
// it sent.
//
// `createDocuments` re-builds its inputs from the two wire fields on purpose:
// the export list carries UI-only companions (`mappedName`, `info`) that must
// not be serialised across the boundary.
//
// The three commands whose report a wizard renders as a PAGE take a `silent`
// flag straight through to the transport. It is a per-CALL argument and not a
// property of the command, because `add_documents` is a toast on the expert page
// and the setup wizard's result step at the same time.
// ────────────────────────────────────────────────────────────────

import { inject, Injectable } from '@angular/core';
import {
  BackendService,
  CallOptions,
} from '../../@shared/data/backend/backend.service';
import { ClientData } from '../../@shared/model/client.types';
import {
  MappedDocument,
  MappedInput,
} from '../../@shared/model/document.types';
import { Profile } from '../../@shared/model/profile.types';
import { DocumentSource } from '../model/filler.types';

export type FillerCommand =
  | { command: 'get_client_data'; payload: Record<string, never> }
  | {
      command: 'add_documents';
      payload: { autoMapFields: boolean; source: DocumentSource };
    }
  | { command: 'remap_document'; payload: { id: string } }
  | { command: 'save_document'; payload: { document: MappedDocument } }
  | { command: 'remove_document'; payload: { id: string } }
  | { command: 'reset_app'; payload: Record<string, never> }
  | { command: 'save_profiles'; payload: { profiles: Profile[] } }
  | {
      command: 'create_documents';
      payload: {
        exportFolder: string;
        documentIds: string[];
        inputs: MappedInput[];
      };
    }
  | { command: 'open_file'; payload: { filename: string } }
  | { command: 'open_output_folder'; payload: { folder: string } };

@Injectable({ providedIn: 'root' })
export class FillerBackend {
  readonly #backend = inject(BackendService);

  #call(command: FillerCommand, options?: CallOptions): Promise<ClientData> {
    return this.#backend.call<ClientData>(command, options);
  }

  getClientData(): Promise<ClientData> {
    return this.#call({ command: 'get_client_data', payload: {} });
  }

  addDocuments(
    autoMapFields: boolean,
    source: DocumentSource,
    options?: CallOptions
  ): Promise<ClientData> {
    return this.#call(
      { command: 'add_documents', payload: { autoMapFields, source } },
      options
    );
  }

  remapDocument(id: string): Promise<ClientData> {
    return this.#call({ command: 'remap_document', payload: { id } });
  }

  saveDocument(document: MappedDocument): Promise<ClientData> {
    return this.#call({ command: 'save_document', payload: { document } });
  }

  removeDocument(id: string): Promise<ClientData> {
    return this.#call({ command: 'remove_document', payload: { id } });
  }

  resetApp(): Promise<ClientData> {
    return this.#call({ command: 'reset_app', payload: {} });
  }

  saveProfiles(profiles: Profile[]): Promise<ClientData> {
    return this.#call({ command: 'save_profiles', payload: { profiles } });
  }

  createDocuments(
    exportFolder: string,
    documentIds: string[],
    inputs: MappedInput[],
    options?: CallOptions
  ): Promise<ClientData> {
    return this.#call(
      {
        command: 'create_documents',
        payload: {
          exportFolder,
          documentIds,
          inputs: inputs.map(({ identifiers, value }) => ({
            identifiers,
            value,
          })),
        },
      },
      options
    );
  }

  openFile(filename: string): Promise<ClientData> {
    return this.#call({ command: 'open_file', payload: { filename } });
  }

  openOutputFolder(folder: string): Promise<ClientData> {
    return this.#call({ command: 'open_output_folder', payload: { folder } });
  }
}
