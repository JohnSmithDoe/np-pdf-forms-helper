# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

"npDokumentenhilfe" (repo `np-office-document-helper`) — a desktop app that links original PDF/XLSX
documents, maps their form fields / cells to user-chosen field names, and produces filled copies.
Fields sharing a mapped name across documents receive the same value on export.

**The UI, all user-facing strings, and `docs/` are German.** Keep new user-facing text German.

Targets **Windows** (only roughly tested there). **No external binaries** — the app used to shell out
to pdftk and quit on startup without it; that went with Electron.

**One frontend, one backend, no shells side by side:**

1. **Frontend.** Angular 21 (standalone, zoneless, Ionic 8) in the np-commlink layout. The port out
   of the Angular 13 app is complete — `src/_legacy/` is gone. Angular Material is gone too, replaced
   by Ionic to match np-commlink; nothing in `src/` names Material any more.
2. **Backend.** `src-tauri/` (Rust, Tauri 2). Electron is **deleted** — the folder, the builder, the
   renderer's adapter and the channel map. Do not reintroduce a second backend abstraction "in case";
   see [docs/decisions.md](docs/decisions.md).

**No functional gaps left.** PDF, XLSX and resource documents all work end to end;
`src-tauri/src/doc/pdf/` is written against `lopdf` and needs no external binary. What is still open
is coverage, not features — **nothing has ever run on Windows and `tauri:build` has never run at all**.
The Rust half now has unit tests (`pnpm run rust:test`); what they cannot reach is the Tauri surface
itself and the real window. See [docs/state.md](docs/state.md).

The migration plan lives outside the repo, in the session plan file referenced from Claude's project
memory. Renamed from `np-pdf-forms-helper` / "npAusfüllhilfe" on 2026-08-15; the GitHub remote still
carries the old repo name.

**Users get the PORTABLE exe, one copy per user on their home drive** — they work in Citrix Windows
11 desktops, where a pooled desktop wipes `%LOCALAPPDATA%` (the NSIS install's home) at logoff.
`docs/installations-anleitung.md` is written for that, and never one shared copy: `data.db` is
rewritten whole, so the last save wins silently.

## Commands

| Command                                          | Purpose                                                              |
| ------------------------------------------------ | -------------------------------------------------------------------- |
| `pnpm start`                                     | Angular dev server only (`ng serve`), no desktop shell               |
| `pnpm run build`                                 | Angular production build → `dist/renderer`                           |
| `pnpm run lint`                                  | eslint + stylelint                                                   |
| `pnpm run verify`                                | **Sheriff** — module boundaries (`sheriff verify src/main.ts`)       |
| `pnpm run format` / `format:check`               | prettier over `src/**` and `e2e/**`                                  |
| `pnpm run tauri:dev`                             | Dev: `ng serve` + the Rust window (`tauri dev`)                      |
| `pnpm run tauri:build`                           | Angular prod build + Tauri bundle (NSIS on Windows, `.dmg` on macOS) |
| `pnpm run rust:check` / `rust:fmt` / `rust:lint` | `cargo check` / `fmt` / `clippy` on `src-tauri/`                     |
| `pnpm run rust:test`                             | **`cargo test`** over `src-tauri/` — the Rust unit tests             |
| `pnpm run e2e` / `e2e:ui`                        | **Playwright** over `e2e/`, against a faked Tauri transport          |
| `pnpm run e2e:release`                           | Production build, then `e2e/release/` under the shipped CSP          |

### The two test layers

They are split at the **transport**, and each covers what the other cannot reach.

**`cargo test` — everything below the transport.** Rust's runner is built into the toolchain; there
is no framework to choose. Tests live in a `#[cfg(test)] mod tests` at the foot of the module they
cover, so they compile out of every real build and can reach that module's **private** items — which
is why nothing had to be made `pub` to be tested. `src/testing.rs` (itself `#[cfg(test)]`) holds the
shared support: a hand-written `TempDir` that cleans up in `Drop`, and the model builders. **No
`tempfile` crate** — it is a `PathBuf`, a unique name and a `Drop`, and writing it is the point.

Two things shape what the tests may do, both load-bearing:

- **`cargo test` runs tests in parallel threads of ONE process.** Nothing may set an env var or
  `current_dir` — that is exactly why `AppConfig::resolve_all` takes its base folder and an env
  lookup as arguments. `TempDir` names itself from the process id plus an atomic counter for the
  same reason.
- **A `State<'_, AppState>` and a `WebviewWindow` cannot be built outside a running app.** So a
  decision worth testing does not stay inside a `#[tauri::command]`: it is split into a free
  function the command then calls (`folder_to_open`, `Run::report`, and trains' `adopt_and_clean`
  / `file_cleaned` / `stage_owned` / `commit_owned`, over `&AppState` — which is what lets one test
  walk a file from adoption to an imported, final document). `AppState` is already free of
  Tauri, which is what lets `import`/`export`/`db` be driven directly.

`docs/formular_beispiel.pdf` is the one real fixture. The AcroForm tests otherwise assemble `lopdf`
object graphs by hand, because byte-level rules need byte-level inputs; the fixture test exists so
the hand-built graphs cannot all be wrong in the same way.

**`e2e/` — the renderer, at the transport.** Playwright drives the Angular app in Chrome against a
fake `window.__TAURI_INTERNALS__` (`e2e/fake-backend.ts`). There are still no unit tests and no test
runner inside `src/`.

Two constraints fixed that shape:

- **Playwright cannot drive the real app.** Tauri speaks WebDriver, and driving it directly is
  Windows/Linux only because macOS ships no WKWebView driver. The dev machine is macOS.
- **The fake stubs the transport, not `BackendService`.** So the app under test runs the real
  service — its command construction and its `{ messages: [] }` error unwrapping are covered rather
  than bypassed — and, more importantly, **no fake ships in the production bundle**. Do not
  "simplify" this into a `FakeBackendService` under `src/`.

`e2e/fake-backend.ts` mirrors `src-tauri/src/filler/commands.rs`, `src-tauri/src/trains/commands.rs` and `src-tauri/src/trains/master_file/commands.rs`:
same command names, same argument keys, same presence rules. **Change a command, change the fake.**
Native file pickers cannot be driven by any browser, so `seed.picker` and `seed.staging` stand in for
what the picker returns.

**Keep the e2e shallow.** It proves the app reaches the right screens and renders what the backend
sent. It does not drive `ion-select` popovers, alert inputs, or anything inside an Ionic shadow root —
those assert Ionic's internals, break on its upgrades, and say nothing about this app. Anything whose
correctness is a property of bytes — parsing, resolution, the commit gates — is proved by `cargo test`
instead, and the fake never re-implements it: a TypeScript copy of `sanitise` would agree with itself
and drift from Rust.

**`e2e/release/` is the one spec that opens the artefact that SHIPS** — `dist/renderer` from a
production build, served by `serve.ts` with the CSP read from `tauri.conf.json` and rewritten the way
Tauri rewrites it (a nonce per `<style>`). Everything else runs against `ng serve`, which has neither
that CSP nor critical-CSS inlining, and v2.0.1 shipped unstyled for exactly that reason — see
`docs/footguns.md`. Its own config (`playwright.release.config.ts`, port **4401**, never a reused
server), ignored by the main one; CI runs it after its production build.

The `test.fail()` block at the bottom of `filler.spec.ts` pins known-open defects: the suite stays
green while they are open, and Playwright reports "expected to fail but passed" the moment one is
fixed — that is the signal to delete the marker, not a broken test.

`wizard.spec.ts` is deliberately two tests, not a mirror of `filler.spec.ts`: the steps compose
components that suite already covers, so what is worth proving is only what the wizard adds — the
guards let a legitimately-reached step through, a cold deep link bounces to step one, and the run's
report renders as the PAGE with `ion-modal` at zero. Two traps it ran into, both worth knowing:
**Ionic keeps the outgoing page in the DOM during a route transition**, so every locator is scoped to
its own `app-page-…` element or `Weiter` matches twice; and `filler.spec.ts` now navigates to
`/#/documents/expert` directly, because `/documents` redirects by view mode and the default is the wizard (and `/` now opens the Schattensystem).

`intake.spec.ts` covers both walks the same shallow way, seeded through `seed.scan`, `seed.clean`,
`seed.dokumente` and `seed.document`; the fake's `reclean_file` only echoes decisions back and never
re-parses, its `write_clean` applies no gate, and its `stage_document` serves hand-written entity
groups rather than grouping — all three are `cargo test`'s. The import walk is proved end to end with
`known` groups only, so the defaults answer everything and no radio button has to be driven. **Playwright reuses
whatever already listens on its port**, so a different app there fails nearly every spec with
nothing pointing at the cause. The dev server runs on **4400**, not Angular's default 4200, for
exactly that reason — set in `angular.json` (`serve.options.port`), `tauri.conf.json` (`devUrl`) and
`playwright.config.ts`; change one, change all three.

`lefthook` runs prettier/eslint/stylelint pre-commit and lint+sheriff+typecheck+`cargo test`
pre-push; `e2e` is not in either hook, because it needs a dev server. `cargo test` builds the crate
with `cfg(test)` on and so type-checks `src-tauri/` on the way through — `rust:check` in the same
hook would be redundant.

## Architecture

Two source roots, two builds:

| Folder       | Process             | Built by                                       |
| ------------ | ------------------- | ---------------------------------------------- |
| `src/`       | Angular 21 renderer | `@angular/build:application` → `dist/renderer` |
| `src-tauri/` | Rust backend        | `cargo` / `tauri build`                        |

### The Angular app

**Three domains — `filler`, `trains`, `info` — plus `@shared`**, on the np-commlink layer axis. They
are sealed from each other and none may import another; anything two need moves to `@shared`.
`src/app/<domain>/<type>` and `domain:*` are generic, so a domain costs a folder and not a config
change; `sheriff.config.ts` names `trains` only for its `master` submodule (below).

```
src/app/@shared/{model,ui,util,data,smart-ui,feature}
src/app/filler/{routes,feature,smart-ui,ui,data,util,model}
src/app/info/{routes,feature,data,model}
src/app/trains/{routes,feature,smart-ui,data,util,model}
src/app/trains/master/{feature,ui,util}
```

**`trains/master` is a submodule, not a fourth domain**: it renders trains' data, and a sealed domain
could not reach it. Sheriff patterns form a tree, so `sheriff.config.ts` names `trains/<type>` beside
`trains/master/<type>`; without it every other trains module falls back to the shell. It holds the
**master export wizard** (`/trains/master/export/{sheets,preview,result}`, started per
document from the document list, state in `data/master-export.*`) and the
**master sheet views** (`/trains/master/sheets/:sheet`): one read-only view per bound sheet, the
Schattensystem's entities laid out in that sheet's columns, to be compared by eye against the
workbook. **Backend for frontend:** `get_master_sheet` returns the whole view — columns (full header
row, `filled` per column), rows as display strings — built in Rust from the same rebind the master
import runs. Angular decides nothing about rows or formatting; a column with `filled: false` is shown
empty on purpose.

**The Schattensystem is becoming the information hub for every imported document.** The entity lists
(Wagen, Radsätze, Partner) are CARDS (`@shared/ui/base-item/entity-card`, the list shell's
`layout="cards"`) carrying the most important facts; a click opens the entity's detail page
(`/trains/wagen/:id`, `/trains/radsaetze/:id`, `/trains/partner/:id`), built whole in Rust
(`trains/detail/`), where every row naming another entity links to it and where removing an entity
now lives. The Wagen card shows each Wagen's Zustand — where it is and how long it has been silent, open Schadensmeldungen and
Aufträge, the next Prüfung — from typed records (`WagenZustand`), not by looking documents up. The
customer's dashboard sheet „Alle Wagen Überblick“ is the model for what it has to show.
**Each task gets its own way in.** Looking for a silent device starts from the devices, so
**Telematik** is a dashboard tile and a list of its own (`/trains/telematik`) rather than a filter on
the Wagen list: one card per Wagen with a device or a reading, the longest silent first, built whole
in Rust (`get_telematik`, `trains/telematik.rs`). The tile's „N stumm“ comes from the same view, so the
two cannot disagree.

`trains` reads spreadsheets somebody else authored, maps their columns onto Wagen, Partner, Radsätze
and Instandhaltungen, and writes sanitised data back out. The mapping is trivial; the mapping **UX**
is the feature, which is why `smart-ui/column-mapper` shows sample values verbatim beside what the
backend made of them — and why they come from the COLUMN rather than the field, so an unmapped column
still shows what is in it.

**The app is an MVP for now: clean the sender's sheets, then write them INTO the customer's own master
file, after a backup** — see [docs/decisions.md](docs/decisions.md), „MVP: In die Kunden-Master schreiben“.
**The menu carries one entry per JOB** — **Dokumente** (`/trains`, `feature/start`: Bereinigen,
Dokumente, Master-Datei; the app opens here), **Schattensystem** (`/trains/erp`) and Info,
then the filler as „Formulare ausfüllen“ under a „Werkzeuge“ heading. `/trains/erp` is a
DASHBOARD (`feature/dashboard`) and not a redirect to the import: it is the domain's only way in, so
what it lands on has to reach everything else. Its tiles carry counts, which is what makes it more
than a second menu — `counts` from the backend for Wagen, Radsätze and Instandhaltungen (the events
list is paged, so its loaded length would lie), and the three partner roles derived from the loaded
list, because nothing counts them server-side. The spokes therefore take `backHref="/trains/erp"` on the
list shell, which replaces the burger with a back button: a screen reached from a hub needs the way
up, not the menu that no longer links to it. **The app is scaled back for now:** without the
`npdh.full` switch (`SettingsService.fullEnabled`) the dashboard hides only Vorlagen and Einstellungen,
and „Export erstellen“ is hidden too. The document list's „In Master übertragen“ needs no switch, only a
chosen master file (`MasterFile.pfad`). The entity tiles and „Master-Import“ (`/trains/master`, which
EMPTIES the current data, Wagen-Zustand included, and rebuilds it from the master's sheets) are always
there.

**Getting a file in is TWO walks, Bereinigen and Import, and the URLs say which.** See
[docs/decisions.md](docs/decisions.md), "Bereinigen und Import getrennt".

- **`/trains/clean` is the cleaning hub.** A folder or a set of files is dropped (Tauri's native
  drag-drop — `BackendService.fileDrops$`, the only other place `@tauri-apps/api` is touched) or
  picked (click = folder dialog, a separate button = file dialog, because Windows cannot offer both
  in one), scanned top-level by `scan_import_paths` / `pick_import_*`, and listed file → template,
  adjustable. A file whose bytes the app already owns is `vorhanden`. Each recognised file is
  reviewed at `clean/file` and FILED — original and cleaned copy copied into the app as a `Dokument`,
  the template's readings learned — and the batch ends at `clean/summary`. An unknown file goes to
  `clean/template`, the mapper, whose only exit is a saved template; the hub then rescans that file.
  Nothing on this side writes an entity, and the header chip says „Bereinigen“.
- **`/trains/documents` is the ledger** and the import's way in, beside the batch summary.
  „Archivieren“ HIDES a Dokument (`archiviertAm`), never deletes it: Rust splits `dokumente` from
  `archiv`, the bytes stay `vorhanden`, and `clear_mirror` keeps the flag, and a successful „In Master übertragen“ archives its
  document itself (`master::export::write`, after the file is written, never as an error) — see
  decisions.md, „Dokumente archivieren“. „Importieren“ — there and on the batch summary — writes into the Schattensystem and nothing else.
- **`/trains/import/*` walks ONE document by type** — `partners` → `wagons` → `wheelsets` →
  `entries` → `summary` → `result`, the chip saying „Import ins Schattensystem“. One decision per
  entity group (`entities::group` in Rust), a declined Wagen dropping its rows, nothing written until
  the summary's „Importieren“, one transaction, and the document final after it.

`IntakeStore` holds the cleaning batch and `ImportWalkStore` the import walk; both are forgotten on
leaving. The backend holds one cleaning and one staging, so files go one at a time. The guards are
`intake.guards.ts`, pure predicates as in filler. The review step is the
point of the feature and is built from `ui` components that only emit (`clean-summary`,
`fehler-list`, `deutung-card`, `format-list`) — the page sends every command, the decisions object is
held in the store and sent WHOLE each time, like `restage_import`'s plan. `WizardShellComponent`
lives in `@shared/ui` because both domains use it.

**Only `Wagennummer` is required.** A date is not: plenty of the arriving documents are about
something other than a dated repair, and the Wagennummer is the only anchor that ties a row to
anything. Required-ness is checked at COMMIT, never at staging — the mapping screen re-stages on
every pick, so refusing a half-mapped plan there makes the first pick fail and nothing mappable at
all.

**A Radsatz is an asset that moves, not a field on a Wagen.** Its history is a list of `Einbau`s
(radsatz, wagen, eingebautAm, ausgebautAm, position); an OPEN one — no `ausgebautAm` — is what
"currently fitted" means, and a Radsatz has at most one, so fitting it elsewhere closes the previous
one. Work done to it is an `Instandhaltung` with `radsatzId` set rather than a second type: it is the
same invoice line. A row that merely NAMES a Radsatz records no fitting — only an install or removal
date is a movement.

**A Radsatznummer is not a key, so it never decides alone.** No check digit, no European format, and
assigned by the Halter or the Werkstatt, so two workshops legitimately use one string for two
different Radsätze. `Radsatz.aliases` is therefore **scoped by sender** — an alias means "this sender
calls it this". An alias hit for the current sender is `Known`; the same number from a different or
unknown sender is `Ambiguous`, never a silent merge, which is why `by_radsatznummer` maps one key to
**several** ids. The sender is an input to the run (the template's partner, else the row's *confirmed*
Werkstatt), deliberately **not** `Provenance`. There is no fuzzy tier here unlike `partner`: a
differing digit is a different Radsatz, and only leading zeros — a formatting difference Excel makes
by itself — are offered, as `Likely`.

**The domain is written down in German in [docs/fachdomaene.md](docs/fachdomaene.md)** — the users
are Halter of freight wagons, and that file is the key between a Fachbegriff and a code identifier.
**Every firm named anywhere in this repo is invented**: the target user is `Wagenmut AG`, its
workshops are `Schienenbein Waggonwerk GmbH`, `Dreh & Gestell Technik` and `Rundlauf
Radsatztechnik`. No real company name belongs in the code, the docs or the fixtures.

**Read it before naming anything in `trains`.** Two things in it govern the whole module. First,
**only two fields in an incoming file are trustworthy**: the Wagennummer, because twelve digits carry
a check digit, and the Radsatznummer *only after the user has confirmed it*. Everything else — partner names, dates, amounts, positions — is evidence, which is
why `sanitise` reports rather than asserts and why unknown means a question rather than an insert.
Second, **Halter and Eigentümer are different parties** (the Halter is the NVR-registered keeper,
which is what the target user *is*; the Eigentümer is usually a leasing SPV), and **Instandhaltung is
the umbrella** of which Wartung is only one of four sub-activities under DIN 31051.

`info` is `/about`: the version and the resolved data folders, plus the four things a corporate IT
approval asks about unknown software (no network, no external binaries, no admin rights, local data).
It is a domain of its own because `@shared` is never routed to; it is two files and no state.

#### The two modes under `/documents`

`filler` is mounted at `documents` and carries the expert page **and** both wizards — one store, two
presentations. Paths are English, titles German.

```
/documents            → redirects by the stored view mode
  start                 the wizard fork: einrichten │ ausfüllen
  expert                the all-in-one FillerPage
  setup/…               source → fields │ documents → documents/:id → result
  wizard/…              selection → values → generate → result
```

**Each wizard does ONE of the two jobs, and the split is what they are for.** Linking a file or a
folder, automatic field mapping, remapping, renaming, removing and "Alles zurücksetzen" are setup,
so they exist on the expert page and in the setup wizard and **nowhere in the export wizard** — its
step 1 renders `smart-ui/selection-list`, the ticking-only sibling of `document-list`, and not that
component behind a flag. A `[selectionOnly]` input would be one component with both modes inside it,
which is the state the two modes replace. With nothing linked, that step cannot be completed at all,
so its empty state sends the user to the setup wizard rather than opening a picker. The two rows'
secondary lines come from `util/document-labels`, so the same document cannot read differently in the
two modes.

Four things make the routing work, all argued in the files named:

- **`clientDataResolver` on the pathless parent** does the one load — three pages can now be the
  first one reached. It swallows a failure into a toast rather than rejecting, because a rejected
  resolver cancels the navigation and leaves an empty window.
- **Guards never load.** `wizard.guards.ts` holds pure predicates over the store (`loaded` plus their
  own condition) that redirect to their wizard's first step. They run in `checkGuards`, one whole
  phase *before* `resolveData` — the parent's resolver cannot be got in front of a child's guard, and
  a cold deep link into a later step has no valid selection anyway. **The first step of each wizard
  is unguarded**, or a redirect would bounce forever.
- **The mode redirect is synchronous.** A `RedirectFunction` runs in an injection context and reads
  `SettingsService` out of localStorage during `checkGuards`. Nothing fetched could answer in that
  phase — see _Settings_ below.
- **A report the page IS must not also be a toast.** The import and the run are sent `silent`, so
  their report is parked in `setupReport` / `runReport` instead of going out on `report$`. Two
  slices, not one with a tag: a finished setup must not satisfy the export wizard's result guard.
  Both result steps and `ReportDialog` render one `@shared/ui/report-view`.

### Settings

**UI preferences live in localStorage (`SettingsService`); the user's data lives in the backend.**
The exception that proves the rule is the Schattensystem's own settings (`TrainsSettings`,
`data/trains/einstellungen.json`, page `/trains/settings`): the Wagennummer spelling changes what
Rust WRITES into cleaned copies and exports, so the backend has to hold it.
The master workbook's bindings (`MasterSettings`, `data/trains/master.json`, page `/trains/master`)
are the same kind of thing for the same reason.
There is no `settings.db` and no `ionic-storage`. The reason is the mode redirect above: it resolves
before any resolver or initializer could have answered, so the read has to be **synchronous**, and
losing a view-mode preference costs one re-toggle. `sortDirection` and the `autoMapFields` default
belong here too when they are made to persist.

**The list pattern is cloned from np-commlink**, presentation half only — `@shared/feature/item-lists`
plus `@shared/ui/base-item`, hanging on the `LIST_FACADE` token. Its state layer did not come across:
np-commlink is NgRx over in-memory data and here the truth lives in Rust, and depending on an
interface rather than a store is exactly what made that substitution free. Five routes share one
shell. A facade that names `columns` gets Excel's AutoFilter dialog per column
(`@shared/ui/base-item/column-filter`, pure filtering in `util/item-lists/list-filter.ts`) instead
of the sort bar — Wagen and Radsätze so far. The one thing the clone does **not** cover is paging — every np-commlink list is a local
array, so the events list adds `ion-infinite-scroll` on top.

**A page that is `.ion-page` must never be given `display: block`.** `.ion-page` is a flex column, and
overriding it stops `ion-content` flexing: the content takes the full page height and anything below
it — a footer, most visibly — is pushed off the bottom of the viewport where it cannot be clicked.
That is also why the list shell carries `class="ion-page"` on `<app-list-page>` rather than on the
routed component: the class belongs on the element that actually holds the header/content pair.

Sheriff enforces the ladder: `ui` may never reach `data` (so anything injecting a service is
`smart-ui`, not `ui`), `smart-ui` is a strict leaf (never composes another smart component), `util`
is pure — an eslint rule bans `@Injectable` there — and `model` is a leaf.

Conventions, all enforced or copied from np-commlink: standalone only (no NgModule), `OnPush`
everywhere, **zoneless** (no `zone.js`), signals-first (`input()`/`output()`/`model()`, never the
decorators), `inject()` with `#private` fields rather than constructor DI, built-in `@if`/`@for`,
`app` selector prefix, class suffixes limited to `Page | Dialog | Component`.
**No `I`/`T` prefix on types** — enforced by `@typescript-eslint/naming-convention`; the regex
deliberately lets acronyms through (`IPCChannel`).

### The wire contract, deliberately written twice

`src/app/@shared/model/` and `src-tauri/src/model.rs` describe the **same JSON shape** in two
languages. That is not drift to be fixed: nothing generates them and the JSON shape _is_ the
contract. **Change one, change the other.**

`ClientData.documents` / `.profiles` are **optional**, and the optionality carries meaning: absent
means "this command cannot have changed the list", present — even empty — means "this is the whole
list now". `FillerStore` therefore acts on **presence**, and a present list replaces what is held.

Documents are a **discriminated union** on `type` (`AnyDocument` = pdf | xlsx | resource), matching
serde's tagged `DocumentKind`. Narrow on `document.type`; do not cast.

### The backend seam

`BackendService` (`src/app/@shared/data/backend/backend.service.ts`) is the whole boundary — one
concrete class, `providedIn: 'root'`, and the only file in `src/` that imports `@tauri-apps/api`.
There is no abstract base, no subclass and no provider factory: one shell means nothing to abstract
over.

A request is a `BackendCommand` — a discriminated union of `{ command, payload }` whose command
strings and payload keys **are** the `#[tauri::command]` names and parameters, so nothing translates
on the way out. Tauri maps camelCase payload keys onto snake_case Rust arguments itself.

Errors arrive as the serialised `AppError`, a plain `{ messages: string[] }` object rather than an
`Error`, and become a `BackendError` carrying those German lines. Nothing in `data/` opens a dialog —
smart components emit and the page presents. Success **reports** ride back on `report$`, the one
broadcast left, because any command may attach one and only the page shows them.

### Reports: a toast unless the user is needed

`ToastService` (`src/app/@shared/data/toast/toast.service.ts`) is the app's transient-message layer —
`ToastController`, `ToastRequest` from `@shared/model/toast.types.ts`, and **one toast at a time** by
construction: a new one dismisses the incumbent, because commands come in bursts and Ionic stacks
toasts rather than replacing them.

`needsDialog` in `filler.page.ts` is the whole routing rule, and it reads the report rather than its
sender: **a `messageFolder` or more than one message line means a dialog; anything else is a toast.**
A folder is an action to press and several lines are a list to read — an export run and a folder
import both produce one line per document. The welcome/version report and every "erfolgreich
gespeichert" are acknowledgements, and an acknowledgement that has to be clicked away is a modal the
app interrupts itself with after every save. Errors take the same rule in `color="danger"`, with a
longer duration.

`ReportDialog` therefore survives for the cases that earn it. Do not give a toast the only path to an
action: `ion-toast` is `role="status"` + `aria-live="polite"`, so its text is announced and a button
inside it is not.

### The Rust backend (`src-tauri/`)

Every module is one concern, and no file is over ~290 lines. The split is by **what a reader has to
know**, not by size: a module that names a German string, a wire type or a config path is app-level;
`doc/pdf/acroform.rs` is the one that is not. Under `doc/` there is **one folder per format**, so the
format being worked on is the folder being worked in.

**The top level splits shared from per-workflow, mirroring the Angular side.** `config`, `error`,
`state`, `model` and the whole of `doc/` are shared; `filler/` and `trains/` are one folder per
workflow, sealed from each other and reaching only into the shared half. `main.rs` is the
composition root and the only file that names both — and its `generate_handler!` is **the** index of
what commands exist, which is what lets each workflow keep its own `commands` module.

`doc/` is deliberately shared rather than filler's: it speaks `model`'s document vocabulary, and
`trains` reaches it for the path and mtime helpers in `doc/shared.rs`. That is also the constraint on
moving anything else — `MappedDocument`, `DocumentKind`, `PdfField` and `Sheet` must stay in the
shared `model.rs`, or `doc/` would point back into `filler/`. `Profile` sits there too, which is the
one thing in `model.rs` that is filler's alone; splitting it out costs a file and buys little while
`ClientData` still has to name both.

**There is no temp copy, and no `TMP_PATH`.** Every service reads the original. The copy existed
because pdftk was a _subprocess taking paths on both ends_ — the source copy, the extracted `.fdf`
and the output all had to be real files. `lopdf` and `umya` parse into memory and write to the
target, and `resource::create` only reads its source, so no service can write the file it was given:
the guarantee the copy used to buy now holds by construction. Do not reintroduce it — see
[docs/decisions.md](docs/decisions.md).

| Module                | Notes                                                                                                                                                                                                                                                                                                                                                         |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `config.rs`           | `.npconfig` → `APP_*` → `./data`, resolved against the **working** directory. `DB_FILE`/`PROFILE_FILE` no longer share `APP_CONFIG` (that was the documented bug); `PDFTK_EXE`/`ENCODING` are gone. Unknown `.npconfig` keys are ignored, so old configs still load                                                                                           |
| `model.rs`            | see _The wire contract_. Anything not modelled here is **dropped** on the next write — which is how the UI-only `export`/`disabled` keys stay out for free                                                                                                                                                                                                    |
| `about.rs`            | `app_info` — the version and the RESOLVED data/output/cache paths, for `/about`. Belongs to no workflow, so it sits at the top level. It replaced the welcome report, which rode `get_client_data` onto `report$` and would now be emitted before any page is subscribed. `info` is a free fn over `AppConfig`, so it is testable without a running app        |
| `filler/db.rs`        | `IndexMap`, not a `BTreeMap` — insertion order _is_ the UI's document order and a sorted map would silently reshuffle every existing install. Writes are atomic (temp + rename) and a failed write is now an error the user sees                                                                                                                              |
| `error.rs`            | `thiserror` enum, serialised as `{ messages: [...] }`. Messages are user-facing **German** and shown verbatim. `AppError::detail` is the "German sentence + the cause underneath" shape every file I/O failure uses; `AppError::reading` wraps every umya/lopdf read so a parser panic becomes that shape too                                                                                                                                           |
| `state.rs`            | `AppState`, the composition root's shared handle. Outside any workflow so `filler/import` and `filler/export` reach the database without depending on the API surface — which is also why it is the one shared module allowed to name a workflow's store                                                                                                     |
| `picker.rs`           | the native pickers (`file`/`files`/`folder`), shared by both workflows because they are sealed from each other. Every picker blocks — see the next row |
| `filler/commands.rs`  | the API surface and nothing else: `#[tauri::command]` per operation, the native pickers (via `picker`), `open`. The two picker commands are `#[tauri::command(async)]` on a **sync** fn — a plain command runs on the main thread, where a blocking native dialog deadlocks the loop it needs, and a real `async fn` cannot hold the `Mutex<Database>` guard across an await. `add_documents` takes a `DocumentSource` (`file`/`files`/`folder`), declared here rather than in `model.rs` because it is a command argument and nothing stores it |
| `filler/import.rs`    | linking one file, a hand-picked batch or a whole folder, **no Tauri** — a per-file failure becomes a report line, and only "nothing linked at all" is an error. `folder` is `many` over a sorted listing, so multi-select and folder cannot disagree about which failures are fatal                                                                            |
| `filler/export.rs`    | one run of documents, **no Tauri** — `Run { messages, failed, refreshed }`; one document failing is a report line, only an unusable output folder stops the run                                                                                                                                                                                               |
| `doc/mod.rs`          | the extension dispatch only. No trait: the shared steps run here, before the `match`, so the per-format half cannot skip them — and a subclass could forget `super`                                                                                                                                                                                           |
| `doc/shared.rs`       | what the services share — `value_for`, `file_name`, `mtime_ms`, `free_path`, `warn_if_changed`                                                                                                                                                                                                                                                                |
| `doc/pdf/mod.rs`      | the PDF _service_: `add`/`create`/`remap` over `acroform`, the preview, and the German warnings. `read_fields` is the shared read half — `add` passes no previous fields and so mints every id                                                                                                                                                                |
| `doc/pdf/acroform.rs` | the AcroForm layer, **`lopdf` and nothing from this app** — the field walk, the fully-qualified name, `/V`, `NeedAppearances`, XFA detection. The half whose correctness is a property of bytes                                                                                                                                                               |
| `doc/xlsx/mod.rs`     | `umya-spreadsheet`. `remap` refuses — the sheet ids `mapped[].origId` points at cannot be rebuilt from a new file                                                                                                                                                                                                                                             |
| `doc/xlsx/address.rs` | the `$Tabelle1.A1` format, parsed in one place. Validates the cell half rather than handing it on: `umya` unwraps a half-parsed address and **panics**, and a panic is not an `AppError`, so the German dialog is lost                                                                                                                                        |
| `doc/resource.rs`     | plain copy. One `fs::copy`, so it stays a file rather than a folder                                                                                                                                                                                                                                                                                           |
| `trains/model.rs`     | the trains wire contract. `DecimalStyle`/`DateOrder` are properties of the SENDER'S FILE — nothing in `trains` may consult a system locale, or one file parses differently on two desks                                                                                                                                                                       |
| `trains/sanitise/`    | raw cell text → a typed value or a German line. `Err` is a Fehler, `Ok` with a `warning` is a Warnung, so severity is the result's SHAPE and not a field to keep in step. The one part of trains with unit tests, because being wrong here is invisible: `1.234` read as `1.234` instead of `1234` looks equally plausible in a preview                       |
| `trains/sanitise/column.rs` | the decimal style and date order, inferred over the WHOLE column. A per-cell guess flips independently per row and silently mixes both readings; the column has evidence the cell does not. Where nothing is conclusive the default is flagged, which is what lets the preview offer one control that re-reads the column                               |
| `trains/db.rs`        | eight JSON stores under `data/trains/` (`wagen`, `partner`, `instandhaltungen`, `radsaetze`, `einbauten`, `templates`, `dokumente`, and `zustand.db` — the whole `WagenZustand` as ONE object), split so saving a partner does not rewrite the Instandhaltungen. `transaction` is the API, not a convention: an import is a handful of writes, not one per row, and a failed flush rolls memory back. Five indexes — Wagennummer, match key incl. aliases, dedupe key, Radsatznummer, original content hash. `reset` also removes the owned files. `clear_mirror` is the master import's narrow wipe — facts go, Partner, templates and Dokumente stay (Dokumente reopened) — and a write that CLEARS a store must `reindex` after it, or `event_exists` keeps stale keys |
| `trains/commit.rs`    | the only module that writes ENTITIES. Re-checks EVERY gate server-side: the frontend's ticks are an input, never the authority. Confirming a partner learns the raw spelling as an alias, which is what makes the second file from a sender free. A staging from a document marks it imported in the same transaction and is refused if it already was. A `StagingOrigin::Master` staging commits without a document; on that path a Radsatz already open on the SAME Wagen under another date is corrected in place (`einbau_uebernehmen`) or left — never closed |
| `trains/zustand.rs`   | the Wagen-Zustand half of a committed row — Telematik-Gerät and the LATEST Meldung per Wagen (time kept, an older one skipped and counted), Schadensmeldung, Werkstattauftrag (needs a Bestellnummer AND an order column), Prüfung (Art from a column, else `ImportPlan.pruefart`). Found by key and updated, an empty cell clears nothing; a Meldung is replaced whole. No walk step: it hangs off the row's Wagen. See decisions.md, „Der Wagen-Zustand ist typisiert“ |
| `trains/detail/`      | one Wagen, Radsatz or Partner as its detail page shows it (`get_entity_detail`, backend for frontend): header fields and sections of rows, every string formatted here, a row's `link` naming the entity it opens — so the hub is navigable Wagen → Radsatz → its other Wagen → Werkstatt. One generic shape, one page (`feature/entity-detail`) for all three kinds. A Partner's lists are capped at 50 rows; an overdue, undone Prüfung is `danger` against today |
| `trains/farbe.rs`     | the Farben — a mark, not a fact: `markierungen.json` holds a `hand` half (set on the detail page, `set_farbe`, kept by `clear_mirror`) and a `master` half (the master's KEY CELL fill, read at `stage_sheet`, merged at commit), keyed by Wagennummer digits and Radsatz match key because the mirror re-mints ids. A fill becomes one of six Farben by hue (`from_argb`), each an Ionic colour role. Hand wins. See decisions.md, „Farben und der Spaltenfilter wie in Excel“ |
| `trains/telematik.rs` | the Telematik list (`get_telematik`, backend for frontend): one row per Wagen with a device or a reading, longest silent first, a device that never reported first of all. Silence is counted in CALENDAR days against `today`, passed in so the view is pure; red above `STUMM_AB_TAGEN` (7), the same number as the Wagen card's in `util/wagen-zustand.utility.ts` — change one, change both |
| `trains/export/`     | „Export erstellen“: `Master-Übersicht.xlsx`, built fresh from the Schattensystem — never the customer's master. An „Übersicht“ of FORMULAS (`MINIFS`/`MAXIFS`/`COUNTIFS` over whole columns, conditional colours against `TODAY()`, `STUMM_AB_TAGEN` shared with `telematik.rs`) over one data sheet per entity (`sheets` declares the columns, `rows` fills them). Formula letters are resolved from the sheets' headers (`layout::letter_of`), never typed. Cells are TYPED (`layout::Cell`): dates as serials, a compact Wagennummer as a number. The hand columns are written empty. `fill` (text cells) stays for `clean::write`. See decisions.md, „Export erstellen schreibt eine Master-Übersicht“ |
| `trains/dokument.rs`  | the app OWNS what it cleans: `adopt` copies the original into `dokumente/<id>/` before it is read, the cleaned copy and a `protokoll.json` sidecar go beside it, `importable` refuses an imported document or a cleaned copy edited since. Identity is `hash::bytes` of the original |
| `trains/entities.rs`  | a staging grouped per entity for the import walk — Partner by role + match key, Wagen by canonical number, Radsatz by number + the sender STAGING used (deliberately not the walk's Partner answer) — with each group's protocol lines; `expand` turns one answer per group back into the per-row decisions `commit` runs on. Missing answer = skip. `einbau_konflikte` finds the master's disputed Einbau dates against the STORE (keyed by Radsatz id); `expand` hands each answer to its rows |
| `trains/template.rs`  | the two template writes outside an import: `learned` (what a FILED cleaning teaches, returned so it lands in the document's transaction) and `save` (the mapper's only exit; a name is required) |
| `trains/recognise.rs` | which template a header row belongs to: every MAPPED template header present, extra columns ignored. Several matches are a question, never a pick. `rebind` carries bindings over BY HEADER, because an index is positional. Replaced the exact `fingerprint` hash, which failed the first time an export grew a column |
| `trains/builtin.rs`   | the shipped templates, in Rust so a typo is a compile error. `master_hint` is what one knows about its master sheet — key column and the master's respellings (`RadsatzID` → `Radsatz ID`) — applied by `bindings::default`, never a sheet name. Read-only; confirming a reading against one writes a user copy (`origin`) that shadows it — `TrainsDb::templates` merges them at read time and never stores them. `shaped` builds a template for the master's sheet kinds too, which are deliberately NOT in `all()` — offered to recognition they would make ordinary files ambiguous |
| `trains/scan.rs`      | a dropped/picked set of paths → one `ScanFile` per file with a status. Top level only, every sheet asked, Excel's `~$` owner files skipped. Reads headers, never stages. Bytes already owned — or repeated in the same drop — are `Vorhanden` |
| `trains/reading.rs`   | how ONE column is read — decimal style, date order — decided once, for `stage` and `clean` alike, so the preview and the cleaned copy cannot read a file two ways. A saved reading the file conclusively contradicts is a question again; a question exists only if some cell actually reads differently under the alternative, and staging warns on exactly those rows |
| `trains/clean/`       | the original read 1:1 through staging's own `parse_cell` and `reading`, every change sorted into Fehler / Deutung / Format; `write.rs` writes the copy (full read, text cells via `export::fill`, `doc::write_book`) with its `Änderungsprotokoll` into the document's folder. The confirmed plan is stored on the `Dokument` and the copy is staged with it at import — canonical values read the same under any reading, so it asks nothing again |
| `trains/master/` | the customer's master workbook, both directions; its own module because it is the CUSTOMER'S file, not an export. The workbook is ALWAYS the client master's current version (`trains/master_file/`): `bindings::follow`, run first in every `sync`, points `MasterSettings.file` at `versions[0].cleaned` and is that field's only writer — nothing here picks a file. `export` („Master aktualisieren“, offered on a document only while a client master exists) writes ONE filed `Dokument` into the sheets the user ticks, INTO the customer's own file (`master_file::write_in_place`, after a backup; a file whose mtime moved since the wizard's scan is refused in preview and write); columns by header, keys typed as numbers because every VLOOKUP is keyed on them, formula columns re-emitted per row (`paste`, `source`). The write is ALWAYS INCREMENTAL (`paste::incremental`): rows matched by key — Wagennummern by their digits — a known row has its shared columns overwritten, an unknown key is appended only on a sheet with `append` (by default the one whose binding names the document's template), and a sheet row whose key the document lacks is EMPTIED in place — never deleted, so no reference anywhere shifts — only with `remove`: offered on every ticked sheet, off unless remembered for the sheet and template (`MasterBinding.remove_for`), its keys listed in the preview, refused when it would empty every keyed row; ONLY that sheet is pre-ticked — the client updates one sheet per document — and step 1 SHOWS only that sheet, no selection; Rust still sends every other sheet unticked, its Wagen column linked to the document's by an alias, so offering them again is a template change. Its preview IS the write, into an in-memory book, diffed before/after and shown by ROW (`export/diff.rs` → `ui/row-changes`, preview and result alike): each changed row whole in every sheet column, changed cells marked, `neu` / `geleert` rows marked whole, a click on a changed row laying its OLD row underneath column for column (`RowCell.before`; labels sticky) — never a before/after list beside it, which meant scrolling back; every row sent, 100 rendered then more on scroll; an unwritten column shows its value from BEFORE, because formula columns are re-emitted with stale caches. The column mapping is the TEMPLATE'S — the binding's key, aliases and `ignored`, sent as Rust offered them — and the wizard edits none of it: step 1 runs the dry run at `begin`, states the key pair read-only and lists the run's `problem` (Weiter dead), `conflicts` and `open` (a document column without a master column, not transferred, never blocking). Rust still accepts hand pairs (`pairs` / `sources`, `paste::classify`); only the step that set them is gone. See decisions.md, „Master aktualisieren: ein Blatt, Zuordnung aus der Vorlage“ and „Export in die Master-Datei“. `import_all` („Alles importieren“, `import_master_all`) is that same run in one call: every group answered as „alle neuen anlegen“ would — Known used, New created, Likely/Ambiguous left unlinked and NAMED in the report, a disputed Einbau date kept — and a report one line per sheet afterwards. `mirror` imports the other way: `start` empties the facts and records `import_run`, `stage_sheet` reads one bound sheet (`grid::from_master`) with its kind's template rebound onto row 1 and stages it with `StagingOrigin::Master` — no `Dokument` is filed, or the export would offer the master to be pasted into itself. `kinds/` is one module per sheet KIND (template, written back or not); the customer's sheet names live only in `master.json`. `sheet_view` builds a bound sheet whole for display (backend-for-frontend), its column→field map the same `rebind` the import runs. `book`: `lazy_read` + `sheet_mut(index)` only — `sheet_collection_mut()` deserialises every sheet (footguns.md). `prepare`: every READER (staging, sheet view) opens a trimmed „Lesekopie“ under `data/trains/master/`, written by `bindings::sync` once per file version (mtime + copy present) — umya parses whole sheets and five are filled to row 1,048,576; the export still reads the current version itself. See decisions.md, „Der Master wird gespiegelt“ and „Die Kunden-Master ist die einzige Master“ |
| `trains/master_file/` | the customer's master workbook as a FILE, and the ONLY master. **MVP: `pfad` is the customer's own file, picked where it lies (`pick_master_target`, nothing copied or cleaned), preferred by `bindings::follow` over any version; `write_in_place` writes into it — open-for-write first (Excel's lock), then a copy to `Sicherungen/<Name> <stamp>.xlsx` beside it, then `write_book`; no backup, no write.** The older path below is unreachable from the UI and kept until the MVP is confirmed: picked on `/trains/master-file`, copied in (`dokument::adopt`) under `data/trains/masterdatei/<id>/`, cleaned, held as `pending` until „Übernehmen“ (which reads its headers once, `bindings::sync`). One master, many versions — `MasterFile.versions[0]` is the latest, the store is `masterdatei.json`; a version with `quelle` was written by the master update (`updated`), not cleaned, so its original and cleaned copy are one file and its report is empty. `clean.rs` never touches a formula inside the data and never moves a row: it cuts empty cells below the last value-or-formula row, and a tail of nothing but `0`/`#NV` (a formula pulled to row 1,048,576) keeps 3 rows and loses the rest — counted apart as `tailRowsCut` and named in the report, shared-formula ranges ended at the last kept row; trims text and retypes text numbers/dates only where the column already holds them and the spelling reads one way. A sheet with nothing to clean is written back byte for byte (probe book first). Own `commands.rs` |
| `trains/sheet/grid.rs` | the only trains file that knows umya on the read side. Coordinates are `(col, row)` numbers, never strings, so `address.rs`'s panic cannot recur. Bounds come from cells that hold something — `highest_column_and_row()` counts styled blanks; only the chosen sheet is parsed (`lazy_read`). `from_master` is the one reader that trims: the master's zero tail is cut and cached formula errors (`#N/A`) read as empty; the strict path still names the row instead |

The whole backend is ported. `cargo check`, `cargo clippy --all-targets` and `cargo fmt --check` are
**warning-free**, and every module above carries its own `#[cfg(test)] mod tests`.

Three rules in `doc/pdf/acroform.rs` that a reader will otherwise re-derive:

- **A node is a terminal field when none of its `/Kids` carries its own `/T`.** Kids without one are
  widget annotations — the field's appearance on the page, not fields. Recursing into them invents
  fields that do not exist.
- **A PDF text string is UTF-16BE behind a BOM or PDFDocEncoded**, and values go back out the same
  way. Without that a German field name or a typed `ä` round-trips as mojibake; pdftk needed
  `iconv-lite` and an `ENCODING` config key for the same reason, and both are gone.
- **Dynamic XFA is detected, not supported.** `acroform::is_dynamic_xfa` keys on the catalog's
  `/NeedsRendering`, **not** on the presence of `/XFA` — static XFA forms carry `/XFA` too and fill
  perfectly well, so the looser check would warn on every form that works. It decides what is _true_;
  `pdf::warn_if_dynamic_xfa` owns what the user is _told_.

`docs/formular_beispiel.pdf` is the fixture: a generated AcroForm covering a nested field tree, a
UTF-16BE name, a field whose kids are unnamed widgets, and a plain top-level field.

`capabilities/default.json` is a deny-by-default allowlist: the window may only reach the _plugin_
commands named there. App commands (`commands.rs`) are not gated by it.

`tauri dev` runs the binary with `src-tauri/` as its working directory, so the app's `./data` folder
lands at `src-tauri/data` in development and beside the executable in production.

### Persistence

Two JSON files (`data.db`, `profiles.db`), rewritten in full on every mutation, atomically (temp +
rename). `version` is 1 and there is no migration yet — write one and bump together if the shape
changes. Profiles reference documents and mapped fields **by id**, so every document mutation
cascades through `prune_profile_fields`; this has been the source of several past bugfix releases, so
touch document add/remove/remap and re-check profiles.

The **view mode** is not in either file — it is a UI preference in localStorage. See _Settings_.

**Cite functions, not line numbers.** A reformat invalidates every `file:line` reference written
before it. Names survive one.

### Styling

**There is no theme.** `src/theme/` is gone and so is the `--np-*` token group: no palette, no type
scale, no spacing tokens. Colour and typography are **stock Ionic**, and `src/global.scss` is the
entire style layer — Ionic's own sheets and nothing else. Dark mode is
`palettes/dark.system.css`, so it follows `prefers-color-scheme` rather than an attribute.

The rule that keeps it that way: **a component never names a colour or a font size.** An accent is
asked for on the element, the Ionic way — `color="danger"` on a destructive button, `color="success"`
on the add buttons, `color="medium"` on a row's secondary icons, `color="warning"` on an empty state's
icon, `<ion-note>` for muted text. Component stylesheets are layout only (flex, gap, width) in plain
units. If a stylesheet under `src/app/` grows a `color:` or a `font-size:`, that is the regression.

That rule is why **structure is chosen as components, not written as CSS**: the expert page's two panes
are `ion-card`s with `ion-card-title` headings, `ion-item` rows and `ion-list-header` section labels,
because each of those brings its own surface, border and text colour. Which is also the trap —
see _Two traps worth keeping_ below and `docs/footguns.md`.

This replaced a hand-written two-tier setup that mirrored the Angular 13 indigo-pink. Do not
reintroduce it: the `#3f51b5` / `#ff4081` heritage is deliberately abandoned.

Three traps worth keeping:

- **A wrong `ion-icon` name renders nothing and raises no error.** Every icon is registered by
  importing the symbol from `ionicons/icons` and passing it to `addIcons` in the component that
  renders it, so a typo is a TypeScript error rather than an invisible button.
- **`--color` on `ion-card` does not reach `ion-card-title` / `-subtitle`** — which is one more
  reason a notice's accent is `color="warning"` on its icon rather than a hand-set background: Ionic's
  own colour role carries the contrast with it.
- **`ion-card` MUTES everything it contains** (`--ion-color-step-550`), a typed input value included,
  and an `fill="outline"` input inside an `ion-item` has its label clipped. Both are why the panes are
  built out of Ionic's own row and heading components with block padding around each input, and neither
  can be found by reading this repo — they are in `docs/footguns.md`.

Two Ionic behaviours that are load-bearing in components, both written down where they bite:

- **`ion-accordion` does not defer its `slot="content"`.** Material's `<ng-template
matExpansionPanelContent>` did. `document-list` keeps an `expanded` signal and gates the rows on it,
  or every field row of every document is built up front.
- **`ionChange` bubbles.** Every checkbox and input inside an accordion raises one that reaches the
  `ion-accordion-group` listener, so `onExpandedChange` checks `event.target === event.currentTarget`
  first. Unguarded, ticking a document closed the panel.

Icons are **ionicons**, tree-shaken through `addIcons`; there is no icon font.

## Conventions

**Comments live in the file header, and nowhere else.** One `─── why ───` block at the top of the
file carrying what a reader cannot get from the code — the trap, the constraint, the decision and
what it rules out. Below that block a source file has **no comments at all**: no section banners, no
JSDoc on exported symbols, no `///` in Rust, no explanation on the line above. If a piece of
rationale is load-bearing it moves up into the header; if it only restates the code it goes.

**Tests are the only exception**, and deliberately so: `e2e/*.spec.ts` and Rust `#[cfg(test)] mod`
bodies may comment inline, because a test's reason for existing is not visible in its assertions.
`e2e/fake-backend.ts` counts as test code.

This is a rule about WHERE, not about how much: a long header is fine, a two-word comment on line 40
is not. What the header cannot hold belongs in this file or in `docs/decisions.md`.

**`eslint-plugin-unicorn` runs the same rules as np-commlink and np-debt-growth** — `all` on
TypeScript, `recommended` on JS, their overrides verbatim — pinned to an EXACT version because `all`
grows with every release. Hence `*.utility.ts` rather than `*.util.ts`, and `activeSort` on the list
facades (the rule takes `this.sort()` for `Array#sort()`). The one difference from the siblings:
`no-manually-wrapped-comments` is off, because they write `/* */` headers and this repo keeps `//`.
Never an `eslint-disable` in source.

**One dependency list.** The root `package.json` is the whole Node story; a Rust dependency goes in
`src-tauri/Cargo.toml`. There is no second manifest and no second lockfile — that was Electron's
packaging contract and it went with it. `pnpm-workspace.yaml` exists only to hold `allowBuilds`,
pnpm's deny-by-default allowlist for dependency lifecycle scripts — pnpm **errors** rather than warns
on an unlisted one, so a new dependency with a postinstall breaks every `pnpm run` until it is added.

**`.browserslistrc` targets two engines, not one.** Tauri does not bundle a browser, it borrows the
OS one: WebView2 (Chromium) on the Windows target, **WKWebView (Safari) on the macOS dev machine**.
Safari is the binding constraint, and the reason the file cannot name a single Chrome version.

**Version bumps** touch four places: root `package.json`, `src-tauri/tauri.conf.json` (it names the
installer), `src-tauri/Cargo.toml` (the app reports `CARGO_PKG_VERSION`) and `CHANGELOG.md`. CI's
`package` job refuses a `v*` tag that disagrees with any of the three. Release commits follow
`release(vX.Y.Z): …`, and pushing the tag `vX.Y.Z` builds the Windows installer into a draft
GitHub release.

**Errors are user-facing German strings.** An `AppError` is surfaced verbatim in a dialog; multiple
lines are separate entries in its `messages` vector.
