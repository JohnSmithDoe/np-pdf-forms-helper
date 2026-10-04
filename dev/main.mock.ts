// ─── why ────────────────────────────────────────────────────────
// The browser entry point: the real app on the e2e fake, for CSS and UX work
// without a desktop shell. `pnpm run start:mock`.
//
// A second ENTRY FILE, not an `isDevMode()` branch in `src/main.ts`: only the
// `mock` build configuration names it, so no production build compiles it. It
// stubs the TRANSPORT, so the real `BackendService` runs underneath.
//
// The import of `src/main` must stay dynamic — a static one is hoisted and
// would bootstrap before the fake transport exists.
//
// WHAT IS ANSWERED HERE rather than in the fake is always the same thing: a
// gesture whose real answer comes from something a browser does not have.
//
// `add_documents` — the native picker. `source: 'file'` invents its next
// document, and the two batch sources are answered here because one line per
// document is the report shape that earns a dialog and the fake's single
// `picker` cannot produce it.
//
// `stage_import` (and `stage_import_path`, the cleaning hub's hand-over of an
// unrecognised file to the template mapper) — the picker again, and one thing
// more: the mapper DISCARDS its staging when a template is saved or the file
// dropped, so a fake with one seeded staging would serve the first file of a
// session and nothing after it. Re-arming on every pick is what makes the
// screen usable by hand.
//
// `stage_document` re-arms `demoDocument()` for the same reason: the commit
// lets go of the staging, and the next document walked would find nothing.
// `stage_master_sheet` re-arms `demoMasterStaging()` per sheet of a run.
//
// `commit_document` — the commit gates live in `trains/commit.rs` and are
// proved by `cargo test`; the fake must not grow a second implementation of
// them. But a commit that visibly changes nothing reads as a broken button, so
// the dev shell invents one Instandhaltung per taken row and lets the fake
// answer with the updated lists and counts.
//
// `pick_import_folder` / `pick_import_files` / `scan_import_paths` — the
// cleaning hub's pickers, answered with a fresh `demoScan()` each time.
// `clean_file` re-arms `demoClean()`, because the walk discards the cleaning
// per file and one seeded copy would serve only the first.
// ────────────────────────────────────────────────────────────────

import {
  install,
  type FakeCleanReport,
  type FakeDocument,
  type FakeInstandhaltung,
  type FakeScanFile,
  type FakeStaging,
} from '../e2e/fake-backend';
import {
  committedEvent,
  DEMO_SEED,
  demoClean,
  demoDocument,
  demoMasterStaging,
  demoScan,
  demoStaging,
  nextPickedDocument,
} from './demo-seed';

interface FakeState {
  documents: FakeDocument[];
  picker: FakeDocument | null;
  staging: FakeStaging | null;
  scan: FakeScanFile[] | null;
  clean: FakeCleanReport | null;
  document: FakeStaging | null;
  masterStaging: FakeStaging | null;
  events: FakeInstandhaltung[];
}

const SCANS = ['pick_import_folder', 'pick_import_files', 'scan_import_paths'];

type Invoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<unknown>;

install(DEMO_SEED);

const state = (window as unknown as { __npFake: FakeState }).__npFake;
const internals = (
  window as unknown as { __TAURI_INTERNALS__: { invoke: Invoke } }
).__TAURI_INTERNALS__;
const forward = internals.invoke;

function addedFolder(): Promise<unknown> {
  const added = [
    nextPickedDocument(),
    nextPickedDocument(),
    nextPickedDocument(),
  ];
  state.documents.push(...added);
  return Promise.resolve({
    documents: [...state.documents],
    message: {
      headline: 'Ordner wurde verknüpft',
      messages: added.map((document) => `${document.name} wurde verknüpft.`),
    },
  });
}

internals.invoke = (command, args = {}) => {
  if (SCANS.includes(command)) {
    state.scan = demoScan();
    return forward(command, args);
  }

  if (command === 'clean_file') {
    state.clean = demoClean();
    return forward(command, args);
  }

  if (command === 'stage_document') {
    state.document = demoDocument();
    return forward(command, args);
  }

  if (command === 'stage_master_sheet') {
    state.masterStaging = demoMasterStaging();
    return forward(command, args);
  }

  if (command === 'stage_import' || command === 'stage_import_path') {
    state.staging = demoStaging();
    return forward(command, args);
  }

  if (command === 'commit_document') {
    const decisions = args['decisions'] as { rows: number[] };
    for (const row of decisions.rows) state.events.push(committedEvent(row));
    return forward(command, args);
  }

  if (command !== 'add_documents') return forward(command, args);

  if (args['source'] === 'file') {
    state.picker = nextPickedDocument();
    return forward(command, args);
  }

  return addedFolder();
};

document.title = `${document.title} — Mock`;

void import('../src/main');
