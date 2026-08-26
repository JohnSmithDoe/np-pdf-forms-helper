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
// `stage_import` — the picker again, and one thing more: the trains import
// DISCARDS its staging when it is committed or cancelled, so a fake with one
// seeded staging would serve the first file of a session and nothing after it.
// Re-arming on every pick is what makes the screen usable by hand.
//
// `commit_import` — the commit gates live in `trains/commit.rs` and are proved by
// `cargo test`; the fake must not grow a second implementation of them. But a
// commit that visibly changes nothing reads as a broken button, so the dev shell
// invents one Instandhaltung per taken row and lets the fake answer with the
// updated lists and counts.
// ────────────────────────────────────────────────────────────────

import {
  install,
  type FakeDocument,
  type FakeInstandhaltung,
  type FakeStaging,
} from '../e2e/fake-backend';
import {
  committedEvent,
  DEMO_SEED,
  demoStaging,
  nextPickedDocument,
} from './demo-seed';

interface FakeState {
  documents: FakeDocument[];
  picker: FakeDocument | null;
  staging: FakeStaging | null;
  events: FakeInstandhaltung[];
}

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
  if (command === 'stage_import') {
    state.staging = demoStaging();
    return forward(command, args);
  }

  if (command === 'commit_import') {
    const decisions = args['decisions'] as { rows: { row: number }[] };
    for (const row of decisions.rows)
      state.events.push(committedEvent(row.row));
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
