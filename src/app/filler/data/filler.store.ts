// ─── why ────────────────────────────────────────────────────────
// The filler's single source of truth. Internal to `filler/data` — only
// `FillerFacade` touches it, and the barrel does not export it.
//
// The shape is the point:
//
//   state       documents + profiles + WHICH ids are ticked + what was typed
//   computed    everything the export panel shows
//
// The selection is id lists, never flags living on the documents, so the whole
// export panel is a derivation and cannot go stale. It is also why the ticked
// state survives a document reload: nothing about it is stored on the document
// objects the backend overwrites. The other half of that deal is
// `applyClientData`, which prunes the ticks against the list it is handed — an
// id list the backend has never heard of has to be told when its ids are gone.
// It acts on PRESENCE, not length: a list that is there is the complete current
// one, a list that is absent means the command could not have changed it. A
// response carrying profiles must not re-apply the selected profile onto the
// ticks — the ticks are the user's, so only the user changes them.
//
// `fieldValues` is keyed by MAPPED NAME and not by field id, because one name
// deliberately drives every field that shares it — which is also why nothing in
// a response can clear the typed values and a reset has to say so.
//
// Nothing in here writes a DOCUMENT change ahead of the backend. The three
// field edits build the document they want saved and return it; the response is
// what replaces the list. A command that rejects therefore leaves the screen
// showing what is actually on disk.
//
// Two derivations are split for cost, not for tidiness. `documentViews` hoists
// the id lists into Sets because it feeds the document list, the export panel
// AND the field dialog, and `includes` inside the map made it O(fields ×
// selection). `exportGroups` — the grouping by mapped name that IS the app's
// headline feature — is kept apart from the typed values, which change on every
// character typed into any export input and would otherwise re-run the whole
// grouping pass per keystroke.
//
// `exportStamp` is state and not `Date.now()` inside a computed on purpose. The
// output folder must be a FRESH timestamp per run — the backend refuses to
// export into an existing folder — but a computed that reads the clock is not a
// derivation and would recompute at unpredictable moments. So the clock is read
// once, explicitly, when an export starts. What that number is rendered as is
// `runFolder`'s business, and only its business.
//
// The export suffix is sanitised on the way IN. It is joined onto OUTPUT_PATH by
// `create_documents`, so a separator in it moves the whole run out of the output
// folder — on Windows an absolute one replaces the path outright, because
// `Path::join` drops the prefix. Leading and trailing dots and spaces go too,
// since Windows trims them off a folder name and would create one under a name
// the app never asked for.
//
// Nothing in here renders markup — labels are template structure. A signal of
// HTML strings would be an `[innerHtml]` sink fed with filesystem-derived names.
//
// Four slices exist for the wizards and are meaningless to the expert page:
//
// `loaded` is what the step guards check. It is set by `applyClientData`, so it
// says "the backend has answered once", not "a request is in flight" — a guard
// runs in `checkGuards`, one whole phase before the parent route's resolver, and
// has to be able to tell an empty install from an unanswered one.
//
// `setupReport` and `runReport` are SEPARATE, not one slice with a tag. Both
// wizards end on a step that renders a report out of the store, so a single
// slice would let a finished setup satisfy the export wizard's result guard and
// show "3 Dokument(e) wurden hinzugefügt" as an export result. Two slices make
// that unrepresentable rather than merely unlikely.
//
// `importedIds` is the ids `add_documents` added, worked out by DIFF: the
// response carries the whole document list and marks nothing as new. Widening
// `ClientData` to say so would be a wire change in service of one screen, so the
// facade hands the previous ids in and the diff happens here.
// ────────────────────────────────────────────────────────────────

import { computed } from '@angular/core';
import {
  patchState,
  signalStore,
  withComputed,
  withMethods,
  withState,
} from '@ngrx/signals';
import { ClientData, ClientReport } from '../../@shared/model/client.types';
import { AnyDocument, MappedField } from '../../@shared/model/document.types';
import { Profile } from '../../@shared/model/profile.types';
import {
  ExportField,
  FillerDocument,
  SortDirection,
} from '../model/filler.types';
import { runFolder } from '../util/run-folder.utility';

type FillerState = {
  documents: AnyDocument[];
  profiles: Profile[];
  selectedProfileId: string;
  sortDirection: SortDirection;
  selectedDocumentIds: string[];
  selectedFieldIds: string[];
  fieldValues: Record<string, string>;
  exportSuffix: string;
  exportStamp: number;
  loaded: boolean;
  setupReport: ClientReport | undefined;
  runReport: ClientReport | undefined;
  importedIds: string[];
};

const initialState: FillerState = {
  documents: [],
  profiles: [],
  selectedProfileId: '',
  sortDirection: 'aufsteigend',
  selectedDocumentIds: [],
  selectedFieldIds: [],
  fieldValues: {},
  exportSuffix: '',
  exportStamp: Date.now(),
  loaded: false,
  setupReport: undefined,
  runReport: undefined,
  importedIds: [],
};

function sortByName(
  documents: AnyDocument[],
  direction: SortDirection
): AnyDocument[] {
  return documents.toSorted((a, b) =>
    direction === 'aufsteigend'
      ? a.name.localeCompare(b.name)
      : b.name.localeCompare(a.name)
  );
}

function withoutDuplicates(ids: string[]): string[] {
  return [...new Set(ids)];
}

const UNSAFE_IN_FOLDER_NAME = /[<>:"/\\|?*\u0000-\u001F]/g;

function safeSuffix(suffix: string): string {
  return suffix
    .replaceAll(UNSAFE_IN_FOLDER_NAME, '')
    .replaceAll(/^[\s.]+|[\s.]+$/g, '');
}

export const FillerStore = signalStore(
  { providedIn: 'root' },
  withState(initialState),

  withComputed((store) => {
    const documentViews = computed<FillerDocument[]>(() => {
      const selectedDocumentIds = new Set(store.selectedDocumentIds());
      const selectedFieldIds = new Set(store.selectedFieldIds());
      return store.documents().map((document) => ({
        ...document,
        selected: selectedDocumentIds.has(document.id),
        mapped: (document.mapped ?? []).map((field) => ({
          ...field,
          selected: selectedFieldIds.has(field.origId),
        })),
      }));
    });

    const exportDocuments = computed(() => {
      const selectedDocumentIds = new Set(store.selectedDocumentIds());
      return store
        .documents()
        .filter((document) => selectedDocumentIds.has(document.id));
    });

    const exportFolder = computed(() =>
      runFolder(store.exportStamp(), store.exportSuffix())
    );

    const exportGroups = computed(() => {
      const documents = exportDocuments();
      const selectedFieldIds = new Set(store.selectedFieldIds());

      const byName = new Map<
        string,
        { identifiers: string[]; documentIds: Set<string> }
      >();
      for (const document of documents) {
        for (const field of document.mapped ?? []) {
          if (!selectedFieldIds.has(field.origId)) continue;
          let group = byName.get(field.mappedName);
          if (!group) {
            group = { identifiers: [], documentIds: new Set() };
            byName.set(field.mappedName, group);
          }
          group.identifiers.push(field.origId);
          group.documentIds.add(document.id);
        }
      }

      return [...byName].map(([mappedName, group]) => ({
        mappedName,
        identifiers: group.identifiers,
        info: documents
          .filter((document) => group.documentIds.has(document.id))
          .map((document) => document.name)
          .join(', '),
      }));
    });

    const exportFields = computed<ExportField[]>(() => {
      const values = store.fieldValues();
      return exportGroups().map((group) => ({
        ...group,
        value: values[group.mappedName] ?? '',
      }));
    });

    const importedDocuments = computed<FillerDocument[]>(() => {
      const importedIds = new Set(store.importedIds());
      return documentViews().filter((document) => importedIds.has(document.id));
    });

    return {
      documentViews,
      exportDocuments,
      exportFolder,
      exportFields,
      importedDocuments,
      hasDocuments: computed(() => store.documents().length > 0),
      hasExportFields: computed(() => exportFields().length > 0),
    };
  }),

  withMethods((store) => {
    const applyProfile = (id: string): void => {
      const profile = store.profiles().find((entry) => entry.id === id);
      patchState(store, {
        selectedProfileId: profile ? id : '',
        selectedDocumentIds: profile?.documentIds ?? [],
        selectedFieldIds: profile?.fieldIds ?? [],
      });
    };

    const writeSelectionToProfile = (): Profile[] => {
      const selectedProfileId = store.selectedProfileId();
      const documents = store.documents();
      const selectedDocumentIds = store.selectedDocumentIds();
      const selectedFieldIds = store.selectedFieldIds();
      const profiles = store.profiles().map((profile) =>
        profile.id === selectedProfileId
          ? {
              ...profile,
              documentIds: documents
                .filter((document) => selectedDocumentIds.includes(document.id))
                .map((document) => document.id),
              fieldIds: documents
                .flatMap((document) => document.mapped ?? [])
                .filter((field) => selectedFieldIds.includes(field.origId))
                .map((field) => field.origId),
            }
          : profile
      );
      patchState(store, { profiles });
      return profiles;
    };

    const documentOfField = (origId: string): AnyDocument | undefined =>
      store
        .documents()
        .find((document) =>
          (document.mapped ?? []).some((field) => field.origId === origId)
        );

    const applyClientData = (data: ClientData): void => {
      const documents = data.documents;
      if (documents) {
        const documentIds = new Set(documents.map((entry) => entry.id));
        const fieldIds = new Set(
          documents
            .flatMap((entry) => entry.mapped ?? [])
            .map((field) => field.origId)
        );
        patchState(store, (state) => ({
          documents: sortByName(documents, state.sortDirection),
          selectedDocumentIds: state.selectedDocumentIds.filter((id) =>
            documentIds.has(id)
          ),
          selectedFieldIds: state.selectedFieldIds.filter((id) =>
            fieldIds.has(id)
          ),
        }));
      }
      const profiles = data.profiles;
      if (profiles) {
        const selectedProfileId = store.selectedProfileId();
        const stillExists = profiles.some(
          (profile) => profile.id === selectedProfileId
        );
        patchState(store, {
          profiles,
          selectedProfileId: stillExists ? selectedProfileId : '',
        });
      }
      patchState(store, { loaded: true });
    };

    return {
      applyClientData,

      applyImport(data: ClientData, previousIds: string[]): void {
        const before = new Set(previousIds);
        applyClientData(data);
        patchState(store, (state) => ({
          importedIds: state.documents
            .map((document) => document.id)
            .filter((id) => !before.has(id)),
          setupReport: data.message ?? undefined,
        }));
      },

      setRunReport(report: ClientReport | undefined): void {
        patchState(store, { runReport: report });
      },

      clearSelection(): void {
        patchState(store, {
          selectedDocumentIds: [],
          selectedFieldIds: [],
          fieldValues: {},
          exportSuffix: '',
          selectedProfileId: '',
          runReport: undefined,
        });
      },

      clearFieldValues(): void {
        patchState(store, { fieldValues: {} });
      },

      documentWithField(
        documentId: string,
        field: MappedField
      ): AnyDocument | undefined {
        const document = store
          .documents()
          .find((entry) => entry.id === documentId);
        if (!document) return undefined;
        return { ...document, mapped: [...(document.mapped ?? []), field] };
      },

      documentWithRenamedField(
        origId: string,
        mappedName: string
      ): AnyDocument | undefined {
        const document = documentOfField(origId);
        if (!document) return undefined;
        return {
          ...document,
          mapped: (document.mapped ?? []).map((field) =>
            field.origId === origId ? { ...field, mappedName } : field
          ),
        };
      },

      documentWithoutField(origId: string): AnyDocument | undefined {
        const document = documentOfField(origId);
        if (!document) return undefined;
        return {
          ...document,
          mapped: (document.mapped ?? []).filter(
            (field) => field.origId !== origId
          ),
        };
      },

      toggleSort(): void {
        patchState(store, (state) => {
          const sortDirection: SortDirection =
            state.sortDirection === 'aufsteigend'
              ? 'absteigend'
              : 'aufsteigend';
          return {
            sortDirection,
            documents: sortByName(state.documents, sortDirection),
          };
        });
      },

      setDocumentSelected(id: string, selected: boolean): void {
        const document = store.documents().find((entry) => entry.id === id);
        const fieldIds = (document?.mapped ?? []).map((field) => field.origId);
        patchState(store, (state) => ({
          selectedDocumentIds: selected
            ? withoutDuplicates([...state.selectedDocumentIds, id])
            : state.selectedDocumentIds.filter((entry) => entry !== id),
          selectedFieldIds: selected
            ? withoutDuplicates([...state.selectedFieldIds, ...fieldIds])
            : state.selectedFieldIds.filter(
                (entry) => !fieldIds.includes(entry)
              ),
        }));
      },

      setFieldSelected(origId: string, selected: boolean): void {
        patchState(store, (state) => ({
          selectedFieldIds: selected
            ? withoutDuplicates([...state.selectedFieldIds, origId])
            : state.selectedFieldIds.filter((entry) => entry !== origId),
        }));
      },

      setFieldValue(mappedName: string, value: string): void {
        patchState(store, (state) => ({
          fieldValues: { ...state.fieldValues, [mappedName]: value },
        }));
      },

      setExportSuffix(exportSuffix: string): void {
        patchState(store, { exportSuffix: safeSuffix(exportSuffix) });
      },

      stampExport(): void {
        patchState(store, { exportStamp: Date.now() });
      },

      selectProfile: applyProfile,

      saveProfile: writeSelectionToProfile,

      addProfile(name: string): Profile[] {
        const id = `${Date.now()}`;
        patchState(store, (state) => ({
          profiles: [
            ...state.profiles,
            { id, name, documentIds: [], fieldIds: [] },
          ],
          selectedProfileId: id,
        }));
        return writeSelectionToProfile();
      },

      removeProfile(): Profile[] {
        const selectedProfileId = store.selectedProfileId();
        const profiles = store
          .profiles()
          .filter((profile) => profile.id !== selectedProfileId);
        patchState(store, { profiles, selectedProfileId: '' });
        return profiles;
      },
    };
  })
);
