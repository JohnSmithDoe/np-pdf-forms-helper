# Decisions

Settled — do not re-flag as work. **Append; never re-weave.** Live work is in [state.md](./state.md),
empirical traps in [footguns.md](./footguns.md). No entry cites a commit SHA: the history is rewritten
whenever the migration is squashed, and a claim has to carry its own evidence.

**A decision already argued next to the code it governs is one line here and a link.** The long form
belongs in the file that can be wrong; this list exists so nobody re-opens the question.

## The Rust backend

- **The shell becomes Tauri 2 + Rust and `electron/` is deleted whole**, not ported file by file. The
  Electron process is 2023 Node code carrying an FDF text parser, a shell-out to an external binary and an
  IPC design (`electron/api.ts`: one `invoke` that resolves to nothing and answers on a broadcast channel)
  that only exists because the renderer was written against it. None of that survives contact with
  `Result` and a typed command; there is nothing to port.
- **Learning Rust is the goal, not the means.** Every crate proposed for this backend is judged against
  that first: it has to remove a _problem_, never the exercise. That is the whole reason the dependency
  list below is two entries long, and it is why "there is a crate for that" is not by itself an argument
  here.
- **PDF is `lopdf` 0.44 and nothing above it. AcroForm filling is hand-written** — estimated ~500 lines:
  walk `/AcroForm/Fields`, resolve the field tree, write `/V`, set `/NeedAppearances`. `lopdf` gives the
  object model and the xref/stream handling, which is the part where a mistake produces a file no reader
  opens; the form semantics on top are the part worth writing by hand.
- **The MIT-licensed `acroform` crate (459 lines) is a reference to read, never a dependency and never
  vendored.** It is short enough to read in full, which is exactly what makes taking it as a dependency
  the wrong trade: it would hide the 500 lines this port exists to write. Reading it to check an
  assumption about a field-flag bit is correct; `cargo add acroform` is not.
- **XLSX is `umya-spreadsheet` 3.1.0** — the only crate that credibly round-trips. The requirement is not
  "write a spreadsheet", it is "open a file somebody else authored, change one cell, and give back a file
  in which everything this program did not understand is still intact". Write-only crates cannot do that
  at any level of effort.
- **pdftk is dropped entirely, not made optional.** Keeping a fallback path would keep the external binary
  in the packaging story, and the external binary is precisely what makes the app unrunnable on a fresh
  machine and unpackageable by CI ([state.md](./state.md)).
- **The temp copy went with pdftk, and `TMP_PATH` with it.** `TempCopy` was carried into the Rust port and
  documented there as a read-only-network-share guard, but that was never why it existed: pdftk was a
  _subprocess taking paths on both ends_, so the source copy, the extracted `.fdf` and the output all had
  to be real files on disk (`copyAndValidateOriginalToTemp` → `extractFDFToTemp` → `applyFDF`). `lopdf` and
  `umya` parse into memory and write to the target, and `resource::create` only reads its source, so **no
  service can write the file it was given** — the property the copy bought now holds by construction.
  Deleting it also took `AppConfig` out of the whole export path (`doc::create`, `export::run`), which is
  the tell that it was the only thing there needing config. A `.npconfig` naming `TMP_PATH` still loads;
  the folder is simply never created. Do not reintroduce a copy without a service that writes its source.
- **`electron/bridge/shared.model.ts` is an input to the port, not drift to be fixed.** It and
  `src/app/@shared/model/` describe one JSON shape under two sets of TypeScript names, on purpose — the
  hand-written mirror is what the Rust side gets translated from. The reasoning and the `I`-prefix
  exemption that follows from it are in `eslint.config.js`'s `electron/**` block.

## Electron, removed

Appended 2026-08-16, and the reason it is here rather than woven into the entries above: those were
written while `electron/` still shipped, so several of them read as future tense. This is what actually
happened.

- **`electron/` is gone — the folder, the builder config, `tsconfig.main.json`, and the renderer's half.**
  The trigger was not the Rust port reaching parity: it was that the Electron shell could not START. It
  quit on a missing pdftk, there is no pdftk in the repo or on the maintainer's machine, so the "fallback
  while PDF is unported" was a fallback nobody could run. Keeping a second backend that cannot be
  executed costs a runtime provider swap, a second dependency manifest and an eslint exemption, and buys
  a code path that has never once answered a request. PDF is now explicitly unbuilt rather than
  theoretically available somewhere else — see [state.md](./state.md).
- **`BackendService` is one concrete class, not an abstract base with a subclass and a provider factory.**
  One shell means nothing to abstract over. `provideBackend()`, `TauriBackendService`,
  `ElectronBackendService` and `app-channel.ts` all collapsed into it, and the whole renderer→backend
  boundary is a single file.
- **The wire contract is TWO hand-written copies now, not three.** `electron/bridge/shared.model.ts` was
  the third; `src/app/@shared/model/` and `src-tauri/src/model.rs` remain, and the `I`-prefix eslint
  exemption that existed only for the Electron copy went with it.
- **Retired with it, all of them documented elsewhere as live rules until today:** the "list every Node
  dependency twice" packaging contract and the second `pnpm-lock.yaml` (README ISSUE #2, now moot rather
  than merely explained); `PDFTK_EXE` and `APP_ENCODING`, and with them README ISSUE #1; the
  `.browserslistrc` Electron pin, replaced by the two webviews Tauri actually borrows — WebView2 on
  Windows and **WKWebView on macOS**, which is a Safari engine and therefore a real second target rather
  than a formality.
- **The version bump touches two places now**, not three: root `package.json` and `CHANGELOG.md`.
  `APP_VERSION` is gone from the renderer — the welcome report carries `CARGO_PKG_VERSION` from Rust, so
  the running app states its own version instead of a constant somebody has to remember to edit.

## The PDF filler, written

Appended 2026-08-16. Same reason as the section above: the entries under _The Rust backend_ were written
while `doc/pdf.rs` was a stub, so they read as plans. This is what was built.

- **The app has no live users, so the port is greenfield.** Stated by Martin on 2026-08-16: the shipped
  v1.1.9 is not in use and starting over is fine. Everything that followed from "somebody else's `data.db`
  is out there" is therefore void — **there is no legacy field-key fallback, no `fqn.replace('.', "")`
  match and no per-document self-heal.** The stored `path` is the real dotted FQN and is matched by
  equality. The old Electron code joined ancestor `/T` values with no separator, which is what made this a
  one-way door; with nothing to stay compatible with, it is not one.
- **The pdftk reference capture never happened and no longer matters.** It was the oracle a hand-written
  filler would have been eyeballed against. With no deployed data and no pdftk on any machine here, there
  is nothing to be bug-compatible with. Verification is `docs/formular_beispiel.pdf` instead — a generated
  AcroForm fixture covering the four shapes the walk has to handle: a nested field tree (dotted FQN), a
  name needing UTF-16BE, a field whose `/Kids` are unnamed widgets (terminal despite having kids), and a
  plain top-level field.
- **`lopdf` is 0.36, not the 0.44 named above.** `Cargo.toml` pins `rust-version = "1.82"` and 0.44
  requires 1.88, so `cargo add` resolves down. Bumping the MSRV is a CI question, not a local one — the
  Homebrew toolchain here is far newer.
- **Field identity is the fully-qualified name, everywhere.** `add` mints a UUID per path; `create` looks
  the stored path up in a fresh walk of the source file; `remap` keeps the id of every path that survived
  and drops the rest, so mappings and profiles follow a re-link. A path that has vanished produces four
  German lines in the export report and the run continues — **loud, never silent**, which is the whole
  reason the missing fallback is safe.
- **Dynamic XFA is detected and warned about, not supported** — see [state.md](./state.md). Note the
  evidence that settled it: the shipped app filled these forms through pdftk `fill_form`, which writes the
  AcroForm layer and nothing else, so the reference documents cannot have been dynamic.
- **The `#[cfg(test)]` exception granted below was not taken.** It stands unused and still granted;
  verification so far is the fixture plus a live `tauri:dev` pass.

## Verification

- **No test suite, and the exception is bounded.** Verification is manual spot-checking; CI runs lint,
  format, Sheriff, both type-checks and a production build, and `.github/workflows/ci.yml` says in its own
  comments that adding a test runner is not wanted. The one exception granted: `#[cfg(test)]` unit tests
  inside the future Rust `doc/pdf.rs`. An AcroForm writer is the only part of this app whose correctness
  is a property of _bytes_ rather than of what a person sees on screen, and it is being written by someone
  learning the language — those two facts are the whole argument, and neither generalises to the renderer.

## Builds

- **One native runner per target, never cross-compilation** — `windows-latest` for the NSIS installer,
  `macos-latest` for the `.dmg`. The full argument, and why no packaging job exists yet, is the RELEASE
  SEAM comment at the bottom of `.github/workflows/ci.yml`.

## The Angular app

- **Angular 21, not 22.** `@ngrx/signals@21.1.1` declares `@angular/core: ^21.0.0` (checked in the
  installed `node_modules/@ngrx/signals/package.json`), and Angular's intra-family peers are exact, so one
  held-back member pins the whole set. Revisit when `@ngrx/signals@22` is stable — and then as its own
  commit, because a framework major on top of other changes makes a red gate unattributable.
- **One domain (`filler`) plus `@shared`.** Documents, field mapping, profiles and export are a single
  workflow over a single data model, so a second sealed domain would earn nothing and would put the
  `@shared` bus in front of every interaction between two halves of one screen. `sheriff.config.ts` names
  no domain — its matcher and dep rules are generic, so a second domain costs a folder and no config
  change. The full argument is that file's docblock.
- **Electron 18 → 43 was forced, not chosen.** `mat.theme()` emits colour through CSS `light-dark()`,
  which needs Chromium 123 and which no build step can polyfill; Electron 18 is Chromium 100. The floor,
  the CVE argument and the browserslist syntax trap are in `.browserslistrc`'s banner.
- **`--np-*` aliases FROM `--mat-sys-*`, never the reverse**, now that a generated M3 palette is the
  single colour source. See `src/theme/_tokens.scss` and `src/theme/_material.scss`.
- **An acknowledgement is a toast, not a modal, and the report decides which** — `needsDialog` in
  `filler.page.ts` keys on the report's own shape (`messageFolder`, or more than one line), never on
  the command that produced it. Every backend command may attach a `ClientReport`, so a rule keyed on
  the sender would have to be re-decided for the next command; keyed on the shape it holds for one
  that does not exist yet. The Material port had shipped all of them as `ReportDialog`, which put a
  modal with a Schließen button in front of the user on startup and after every single save. The
  dialog stays for what earns it — the export run's "Ordner öffnen", a folder import's per-file lines,
  a multi-line failure. `ToastService` is modelled on np-commlink's `notifications-toast.effects.ts`
  minus the parts this app has no use for: no i18n keys (the strings are German literals), no
  dispatchable action (nothing offers one through a toast), and one toast at a time instead of a
  `group` map.

## The two README ISSUES, closed

- **ISSUE #1 — encoding detection — is closed by the port, not by a charset detector.** The app assumes
  win1252 on Windows and utf8 elsewhere (`electron/np-assistant.ts`) because pdftk hands back FDF as a
  _text file_ that has to be read again through `iconv-lite`. `lopdf` reads the PDF's own objects, where a
  string's encoding is declared by the document. The question disappears with FDF. **Do not add
  `jschardet` or any equivalent.**
- **ISSUE #2 — "dependencies must be listed twice" — was never a defect, it is the packaging contract.**
  `electron/package.json` is electron-builder's `directories.app` manifest and is installed separately;
  CLAUDE.md's _Conventions_ section owns the rule. It also dies with `electron/`: a Tauri build has one
  manifest and no second `node_modules`.

## Two modes: the expert page keeps its place, the wizards get their own

Appended 2026-08-16. The all-in-one page asks the user to understand documents, field mapping, profiles
and export before anything happens — correct for whoever set the documents up, wrong for whoever only
has to fill them in. So it stays, as one of two modes, and does not grow a beginner path inside itself.

Settled, each argued in full in the file named — one line here so nobody re-opens it:

- **One store, two presentations.** `FillerStore` is not copied per mode; ticking in the wizard shows
  ticked in expert, deliberately — see `filler.store.ts`.
- **Both wizards live in `filler/`; `info` is the third domain.** Sealing a `setup` domain would cut it
  off from the store it exists to drive — see `sheriff.config.ts`'s docblock.
- **Steps are child routes, not a `step` signal** — back and Alt+Left work for free, illegal states
  become reachable, and the guards are the price. See `filler.routes.ts`.
- **Guards never load; they are pure predicates plus a redirect**, and each wizard's first step is
  unguarded — see `wizard.guards.ts`.
- **The parent's resolver cannot be got in front of a child's guard.** `checkGuards` → `resolveData` →
  `loadComponents` are whole-tree phases — see `client-data.resolver.ts`.
- **Route paths are English, titles are German.** `FillerStore`'s `SortDirection`
  (`'aufsteigend' | 'absteigend'`) is the one place it is broken.
- **`setupReport` and `runReport` are separate slices**, so a finished setup cannot satisfy the export
  wizard's result guard — see `filler.store.ts`.
- **A report the page IS must not also be a toast** — `silent` on `BackendService.call`, and one
  `@shared/ui/report-view` behind the dialog and both result steps. See `backend.service.ts`.
- **"Which documents did this import add?" is a diff in the facade, not a wire change** — see
  `filler.facade.ts`.

Three alternatives were weighed and rejected, which is the part that lives only here:

- **`ionic-storage` and a `settings.db` both lost to localStorage** for the view mode. The
  `/documents` redirect resolves during `checkGuards`, so the read must be SYNCHRONOUS: ionic-storage
  is IndexedDB-backed and async, and a Tauri-backed `settings.db` would cost a Rust module plus an IPC
  round trip blocking the first navigation — both to hold one enum whose loss costs one re-toggle. The
  rule that follows: UI preferences in localStorage, the user's data in the backend.
- **`report$` was NOT made a `ReplaySubject(1)`.** That was the cheap way to keep the welcome toast
  alive once `load()` moved into a resolver. It was retired instead: the version moved to `/about`,
  where it sits with the facts a corporate IT approval actually asks about unknown software — no
  network calls, no external binaries, no admin rights, all data in a folder beside the executable.
  A replaying transport would also change every other subscriber's semantics to save one toast.
- **A second `FillerStore` per mode was rejected** before it was written: it would be a second thing to
  keep in step with the backend's echo on every command.

## The trains workflow (appended 2026-08-16)

A second workflow beside `filler`: read the spreadsheets that arrive by email, map their columns onto
waggons, partners and maintenance events, and write sanitised data back out. Files only — no network,
no ERP connection.

**`src-tauri/src` splits shared from per-workflow.** `config`, `error`, `state`, `model` and all of
`doc/` are shared; `filler/` and `trains/` are one folder per workflow and never reach into each
other. `doc/` is deliberately shared rather than filler's, and that constrains what could move: it
speaks `model`'s document vocabulary, so `MappedDocument`, `DocumentKind`, `PdfField` and `Sheet` stay
in the shared `model.rs`. Moving them into `filler/` would have made the shared document layer point
back at a workflow. `Profile` stays there too though it is filler's alone, because `ClientData` names
both; and `state.rs` names `filler::db::Database`, which is the composition root's handle.

**The sheet-reading seam is `Grid → Layout`, dispatched by an enum and not a trait.** `doc/mod.rs`
reaches the same answer for a different reason, so this is reconciled with it rather than copied.
Reasons specific here: detection folds over `ALL`, so an exhaustive match is what catches a reader
missing from the override list; the choice is persisted in a template and comes back months later,
where an enum fails loudly on a removed variant instead of needing an invented "unknown reader" path;
and a reader is a pure function with no state for `&self` to be. `Manual` is a real variant, so
overriding detection and detecting travel the same code path.

**Detection is a proposal, never a silent decision.** `choose` refuses on a weak best or a close
second, the candidates always travel to the UI, and the tests assert WHICH row wins and never the
score — the weights are judgement tuned against real files, and a test pinning `score == 78` gets
re-pinned on every tuning pass.

**`1.234` is inferred per COLUMN, never per cell.** There is no local information that decides German
thousands from English decimal, and a per-cell rule flips independently per row — so one column
silently mixes both readings and a total is wrong on some rows and right on others. The column has
evidence the cell does not. Where nothing is conclusive the default is flagged on every cell, which is
what lets the preview offer one control that re-reads the whole column. The same argument, unchanged,
applies to `03/04/2025`.

**A wrong UIC check digit is a warning, never a rejection.** Fleets carry typo'd numbers in
circulation, on paper and in the customer's own ERP, and a tool that refuses them gets worked around.
The real typo detector is a bad check digit AND no matching known waggon; neither alone is evidence.
(The number in the original request, `31 80 4740 123-4`, fails its own check digit — the algorithm
says `-3`.)

**The dedupe key is built from the file's own words**, the canonical waggon number and the workshop's
normalised name, and never from a resolved entity id. An id only exists once the partner does, so a
key using one hashes the same row differently before and after its first import — and re-sending a
file would double every event in it.

**Nothing is ever created implicitly, and `commit` re-checks every gate.** The frontend's ticks are an
input and never the authority: a stale preview must not be able to mint four hundred partners, and a
typo'd waggon number created silently becomes a phantom every later import matches against.

**Confirming a partner learns the raw spelling as an alias.** Without that loop the user answers the
same question about the same workshop every month and stops using the import.

**Export is two asymmetric halves.** The ERP workbook is ours: generated from scratch, disposable.
The master is the user's — and how it is written was replaced once a real master arrived, see "The
master is refreshed sheet by sheet, into a copy" below. XLSX and not CSV, because CSV reopens the encoding question this project already closed — Excel renders
BOM-less UTF-8 as mojibake.

**Store migration is deferred.** The trains stores write `version: 1` and do not read it, exactly as
`filler::db` does. There is no deployed store and no shipped shape, so a ladder now would be a guess
about a format nobody has written yet. The trigger that turns it on: the first change to a persisted
trains shape *after* a build ships.

**No new crates.** `chrono`/`time`, `regex`, `rust_decimal`, `strsim`, `calamine`, `csv`, `rusqlite`
and `fnv` were each judged against the standing bar — a crate must remove a problem, never the
exercise — and each declined with a named condition that would flip it: `chrono` the day timezones or
business-day maths appear, `rusqlite` above ~500k events or a measured >1 s commit.

**The e2e stays shallow.** It proves the app reaches the right screens and renders what the backend
sent, and does not drive Ionic's popovers, alerts or shadow roots. Anything whose correctness is a
property of bytes is proved by `cargo test`; a TypeScript copy of `sanitise` in the fake would agree
with itself and drift from Rust.

## The trains domain language (appended 2026-08-16)

**The trains module speaks trade German, and [fachdomaene.md](./fachdomaene.md) is the key.** The
users are Halter of freight wagons and every sender file, support call
and feature request arrives in that language. An invented English model made each one cost a
translation step with no written key, and one that could come out differently twice. The glossary in
that document maps Fachbegriff → Bezeichner and is maintained with the code.

**Domain nouns are German; app machinery stays English.** The test is whether the user has a word
for it. Wagen, Radsatz, Einbau, Instandhaltung, Halter, Werkstatt — yes. `Resolution`,
`ColumnBinding`, `LayoutHint`, `Provenance`, `dedupeKey`, `fingerprint`, staging — no, and
translating those buys nothing while costing readability. Identifiers and JSON keys are **ASCII**,
which is nearly free because the trade terms are umlaut-free by luck (`Wagen` not `Güterwagen`,
`Halter` not `Eigentümer`); where unavoidable, transliterate `ae/oe/ue/ss`. Umlauts belong to
user-facing labels only.

**Halter and Eigentümer are two parties, so a wagon carries both.** The Halter is the NVR-registered
keeper marked by the VKM on the sole bar — that is what the user *is*, and what a sender's "Halter"
column means. The Eigentümer is usually a leasing SPV nobody in a workshop names. The old single
`owner` was not a translation but a mistake, and `PartnerRole` therefore has three variants, not two.
For the same reason the event entity is `Instandhaltung` and not `Wartung`: under DIN 31051 / EN 13306
Wartung is one of four sub-activities (Wartung, Inspektion, Instandsetzung, Verbesserung), and an
imported row can be any of them.

**Only two fields in an incoming file are trustworthy.** The Wagennummer, because its twelve digits
carry a check digit; and the Radsatznummer, but *only after the user has confirmed it* — everything
else is evidence. This is the standing assumption behind the whole import: only the Wagennummer is
required, `sanitise` reports rather than asserts, decimal style and date order are decided over the
whole column, and unknown means a question rather than an insert.

**A Radsatznummer is not a key, so it is resolved like a partner name and not like a Wagennummer.**
It has no check digit, no European format, and is assigned by the Halter or the Werkstatt — two
workshops can legitimately use one string for two different wheelsets, and one wheelset arrives under
several spellings. The alias table is therefore **scoped by sender**: an alias means "this sender
calls this Radsatz X". An alias hit for the current sender is `Known`; a bare number hit from a
different or unknown sender is `Ambiguous`, never a silent merge. `by_radsatznummer` consequently
maps one key to *several* ids.

**The sender is not taken from `Provenance`.** It was the obvious place and is deliberately not used:
provenance records the one import that created a record, while the real relation is many senders per
Radsatz and it grows as merges are confirmed. `Radsatz.source` is `Option` besides, so a hand-created
Radsatz would have no sender and no way to gain one. The sender is an input to the staging run —
`ImportTemplate.partnerId`, falling back to the row's resolved Werkstatt.

**The Radsatzwellennummer is modelled but does not resolve.** The EN 13261 stamp on the axle is the
only near-global identifier a wheelset has, so the field and its `FieldKind` exist and a column can be
mapped onto them. It decides nothing until a real sender file proves the format — an untested
matching rule on the one identifier users would trust most is the wrong thing to ship.

**Routes and store filenames went German too, beyond the agreed boundary.** The rename was scoped to
the model vocabulary, with route paths, `.db` filenames and `#[tauri::command]` names explicitly out
of it. The commands held — `save_waggon` still takes a `Wagen`, which is the one visible seam — but
the paths (`/trains/wagen`, `/radsaetze`, `/werkstaetten`, `/halter`, `/eigentuemer`,
`/instandhaltungen`) and three of the six stores moved with the identifiers, and were then finished
rather than half-reverted (`wagen.db`, `partner.db`, `instandhaltungen.db`, `radsaetze.db`,
`einbauten.db`, `templates.db`). Nothing is deployed and there is no store to migrate, so the cost
was zero; reverting to English paths is still one commit if a bookmarked URL ever matters.

## Radsatz identity (appended 2026-08-16)

**The alias table is scoped by sender, because a Radsatznummer is not a key.** It has no check digit,
no European format, and is assigned by the Halter or the Werkstatt, so two workshops can legitimately
use one string for two different radsaetze and one radsatz arrives under several spellings. An alias
therefore means "THIS SENDER calls this radsatz X", not "this radsatz is also spelled X". An alias hit
for the current sender is `Known`; the same number from a sender who never confirmed it is
`Ambiguous`, never a silent merge. `by_radsatznummer` consequently maps one key to SEVERAL ids, and
choosing "neu" against a colliding number leaves two radsaetze sharing a `match_key`, each recognised
only by its own sender.

**The sender is not taken from `Provenance`.** It was the obvious place and is deliberately unused:
provenance records only whoever created the record, while the relation is many senders per radsatz and
grows as merges are confirmed — and `Radsatz.source` is optional, so a hand-made radsatz would have no
sender and no way to gain one. It is an input to the run instead: the import template's partner, else
the row's CONFIRMED Werkstatt. A `Likely` is not confirmed and does not scope an alias.

**There is no fuzzy tier for radsaetze, unlike partners.** Two radsaetze differing by a digit ARE two
radsaetze, and suggesting otherwise puts the wrong history under a wagen. The single exception is
leading zeros — a formatting difference, not a digit, and one Excel introduces by itself — which is a
`Likely` and never a match. `match_key` deliberately does NOT strip them: tier one must never surprise.
That narrowness is also why `similarity`/`distance` stayed inside `resolve::partner` instead of being
extracted into a shared module: nothing else needs them.

**`Radsatz.nummer` keeps the sender's spelling; only `match_key` is normalised.** The list shows what
the file said, and matching still ignores dashes and spaces.

**The Radsatzwellennummer is stored and never matched on.** The EN 13261 stamp on the axle is the only
near-global identifier a wheelset has, so the field and its `FieldKind` exist and a column maps onto
them — but it fills a blank only, and no rule leans on it until a real sender file proves the format.
Shipping an untested rule on the one identifier users would trust most is the wrong trade.

## Workshop orders are a feed, not an entity (appended 2026-10-03)

**The first real order export is read as a source of Instandhaltungen, not as orders.** It is the
Halter's ERP list of workshop orders: one row per order, an order number, the wagen, the Werkstatt,
an order date, a planned and an actual intake, a workshop exit, and a status that moves
`erfasst → zugestellt → ausgeführt`. It is a SNAPSHOT that is sent again as statuses move, whereas
everything `trains` stores is an EVENT that does not change once it happened.

**So only finished orders are imported, dated by the workshop exit.** Map the exit date onto
`Datum`, the Werkstatt column onto `Werkstatt`, the note onto `Bemerkung`, and untick the rows with no
exit date. That needs no model change and dedupes correctly on re-import: an exit date does not move
once set, so the dedupe key (wagen, Datum, Werkstatt, Leistung, Betrag) stays stable. The order date
was the obvious alternative and is wrong: it would record work that has not happened, and every
later status change would be skipped as a duplicate.

**The cost is bad, and accepted only as a stopgap.** Every import of this file means unticking the
unfinished orders by hand — 33 of 42 on the first file, every time it is sent again — and a row
missed is an Instandhaltung that did not happen, committed without complaint. It cannot be fixed
by a default: rows without a date are legitimate in other files (see `fachdomaene.md` §8), so
"no date → unticked" is wrong globally. The fix is per TEMPLATE — a stored row filter such as
"only rows where <column> is filled" on `ImportPlan`, applied at staging — and it is the first thing
to build if this file stays in use, or if a second sender file shows the same snapshot shape.

**Deferred: an `Auftrag` entity keyed on the order number.** The order number is the column this
reading throws away, and probably the most valuable one: in the same Halter's own tracking workbook
an invoice line sits next to the order number it settles, which makes it the join from an order to
its later invoice. Modelling it means a mutable record (status, intake, exit) that a re-import
UPDATES rather than appends, and that an Instandhaltung or an invoice line points at. Not built until
a real invoice file shows that the join actually holds: designing the link from one side of it is
guessing.

## Snapshots of fitted radsaetze (appended 2026-10-03)

**The first real wheelset-monitoring export broke three assumptions at once.** It lists every
radsatz CURRENTLY FITTED, one row each, four per wagen, with an install date and no Datum,
Leistung or Betrag — and it is regenerated and sent again. All three fixes are pinned by tests.

- **The dedupe key now carries the radsatz number and both fitting dates.** It was wagen, Datum,
  Werkstatt, Leistung, Betrag — so four fittings on one wagen hashed alike and three of every four
  staged as duplicates. Changing the key's composition would normally orphan every stored key;
  there is no deployed store yet, so nothing needed migrating.
- **Only work is an event.** An Instandhaltung is written when the row has a Datum, Leistung,
  Betrag or Bemerkung. Every row used to write one, so a fitting-only file left an empty event per
  wagen. This is the converse of the existing rule that naming a radsatz records no fitting.
- **A fitting already stored is not recorded again.** Same radsatz, same wagen, same install
  date is the same Einbau: identical, it is left alone; with a removal date it lacked, it is
  closed in place. Re-sending used to close the open fitting on its own install date and open a
  copy — a zero-length Einbau per radsatz per re-send.

**The sender's own radsatz id is a new field, `RadsatzSystemId` → `Radsatz.systemId`.** The export
carries one per row, unique — a better identifier than the Radsatznummer, but only inside that one
sender's system. It is stored and fills a blank exactly like the Wellennummer, and decides nothing:
matching on it would need it scoped by sender the way aliases are, and one file from one sender is
not the evidence to design that on. It is searchable in the Radsätze list, not shown. Named
`systemId` because `radsatzId` is already the foreign key on `Einbau` and `Instandhaltung`.

## Typed import: recognise, clean to a file, then import (appended 2026-10-03)

**While the big workbook is parked, the small sender files get a guided path.** A folder is dropped
(or picked) on the import page, each file is matched to a template, and each matched file is
CLEANED into a copy before anything is imported. The rule over all of it, Martin's: **never
auto-resolve when another reading is possible — the user has to know.**

- **A file's "type" is its import template**, not a new concept. Three ship with the program
  (`trains/builtin.rs`: Werkstattaufträge, Radsatz-Monitoring, Telematikdaten), written in Rust so a
  typo is a compile error. They name the file's SHAPE, never a firm, and carry no partner — safe,
  because an alias learned without a sender matches without one (`Radsatz::known_to`).
- **Recognition is "every MAPPED header of the template is present"** (`trains/recognise.rs`). It
  replaced an exact hash of sheet name plus all headers in order, which failed the first time a
  portal export grew a column. Several matches are a question in the scan list, never a pick by
  "most columns". Bindings are carried over BY HEADER (`rebind`), because a binding's index is
  positional and a column inserted in front would shift every one of them.
- **Built-ins are read-only; editing makes a user copy.** Confirmed readings go onto a copy whose
  `origin` names the built-in, and the copy shadows it from then on. A program update can improve
  the shipped template without overwriting what the user made of it; a reset brings it back.
- **The original is sanitised 1:1 into a copy, and the COPY is imported.** Every sheet survives, only
  changed cells are replaced, and an `Änderungsprotokoll` sheet lists every change. It is the
  landing-zone pattern: raw → cleaned file → load, so an import gone wrong can be traced to the
  cleaning or to the load, because the step between is a file. Copies go to
  `<output>/bereinigt-YYYY-MM-DD/`, never over the original.
- **Changes are sorted by whether they could alter meaning.** Fehler (no clean value — blocks),
  Deutung (another reading was possible — one card per COLUMN, confirmed explicitly), Format
  (lossless — counted). The screen shows decisions; the protocol sheet carries the evidence.
- **A card is raised only when a cell actually reads differently under the alternative.** An
  undecided column whose values come out the same either way offers no choice, and a card for it
  would train users to click cards away.
- **A confirmed reading is saved on the template**, so the sender's next file asks nothing — but
  conclusive evidence in a new file against the saved reading makes it a card again.
- **Found on the way:** `infer_date_order` counted an ISO date's leading year as a day over twelve,
  so a column holding one `2025-05-06` was silently settled as day-first. Year-first values are no
  longer evidence.

**One column reader for both paths** (`trains/reading.rs`). The cleaner first grew its own rules —
doubt a saved reading the file contradicts, ask only when a cell reads differently, take evidence
from text cells only — while staging kept the old ones, so "never auto-resolve" held on the guided
path and not on the manual one. Both now call `read_column`. Staging warns on the rows that read
differently instead of on every row of an undecided column, and the cleaned file needs no pinned
plan: its canonical values read the same under any reading.

**Found on the way: the partner list and two counts never reached the real app.** Rust sent
`partner` and counts `partner`/`instandhaltungen`; the frontend reads `partners` and
`partners`/`events`. The e2e fake speaks the frontend's names and hid it. Fixed with serde renames
on the Rust side and pinned by a serialisation test.

Deliberately not done here: telematics entities (the telematics template maps only the Wagennummer
until telematics has a model), the per-template row filter the order feed needs ("Workshop orders
are a feed…" above), and subfolders — a scan reads one level deep.

## Bereinigen und Import getrennt (appended 2026-10-03)

**Getting a file in is two walks, and the first is complete without the second.** The guided import
used to go file by file — clean, write the copy, stage it, preview rows, commit, next. Martin wanted
the jobs apart: cleaning a whole batch is a closed cycle that ends in a batch summary and may be the
only thing a user does; importing into the Schattensystem is a separate, later act. The two are told
apart by a header label (`WizardShellComponent.phase`), not by a theme — a forced dark mode for one
half was considered and dropped, because it would have meant the app overriding the OS palette.

- **The app owns the files.** `clean_file` copies the original into `data/trains/dokumente/<id>/`
  *before* reading it, and `write_clean` writes the cleaned copy and a `protokoll.json` beside it,
  then files a `Dokument` record. It is the landing-zone pattern with a ledger: the record says which
  step a file has reached (`bereinigt_am`, `importiert_am`). Reading the owned copy means what was
  cleaned is byte for byte what is stored. An abandoned cleaning removes its folder.
- **Identity is the content hash** of the original (`hash::bytes`, the same FNV-1a). The same bytes
  dropped again — renamed, in another folder, or twice in one drop — are `vorhanden` in the scan and
  refused by `clean_file`. A re-export differing in one cell is a new document; the row-level
  `dedupe_key` catches its repeated rows on import.
- **A template learns when a cleaning is FILED**, in the same transaction as the document
  (`template::learned`). It used to learn at import, so a user who only cleaned answered the same
  Deutung for every file; learning on each confirmed card would have taught readings from files the
  user then abandoned.
- **The mapper's only exit is a template.** An unknown file used to be mapped and committed directly
  — the one path into the Schattensystem that skipped the cleaning. Now it is mapped, saved as a
  named template (`save_template`), rescanned and cleaned like every other file. `commit_import` is
  gone; `commit_document` is the only way in, and it refuses a staging that is not a document.
- **The import walks ONE document by entity type**, in the commit's dependency order: Partner →
  Wagen → Radsätze → Einträge → summary. One decision per entity group, not per row
  (`entities::group`, keyed by each type's identity rule), expanded back to the per-row decisions
  `commit` has always checked (`entities::expand`), so the gates did not move. Missing answer = skip.
  `likely`/`ambiguous`/`new` groups start undecided and block their step. Several documents are not
  merged into one walk — being asked about another file's entities would be confusing — and the
  alias learned on commit makes the next file's question go away anyway.
- **Plan/apply.** The steps only collect answers; the summary is the plan; „Importieren“ is one
  transaction that also marks the document imported. Imported is final, enforced in `commit` and
  `stage_document`, not by the hidden button. Answers live in the frontend only — leaving the walk
  forgets them, deliberately: a half-answered walk resumed days later answers a store that has moved.
- **A cleaned copy edited since cleaning is refused** at import (`cleaned_hash`): it would load
  values nobody reviewed.
- **Radsatz resolution stays decoupled from the Partner step.** A Radsatznummer is scoped by sender,
  and the sender is the template partner or the row's *confirmed* Werkstatt. Re-resolving the
  Radsätze with the walk's Partner answers would have made step three depend on step two. Instead
  the groups are keyed by the sender STAGING used: a Werkstatt first confirmed in this walk makes its
  Radsätze a question, never a silent match, and the commit still learns the alias against the
  committed Werkstatt, so the sender's next file resolves cleanly.

Deliberately not done here: a per-Wagen overview across types, deleting documents, and subfolders.
The import walk is MVP quality for review; nothing of it has run in `tauri:dev` yet.

## Wagennummer spelling is a Schattensystem setting (appended 2026-10-03)

**One setting, `TrainsSettings.wagennummer`, decides how every Wagennummer is shown and written** —
lists, import cards, the cleaned copies and both exports: `compact` (`338506591522`, the default)
or `grouped` (`33 85 0659 152-2`). Users differ; Martin's side reads the grouped form, others the
compact one. The store keeps the bare twelve digits as before, and both spellings parse back to
them, so nothing that matches on a number is affected by switching.

- **In the backend, not localStorage** (`data/trains/einstellungen.json`, read with defaults so an
  old data folder loads): Rust writes the cleaned copies and exports, and every desk on one data
  folder should spell numbers alike. `reset_trains` keeps it — it is configuration, not data.
- **Exports follow it too.** Checked with Martin: the receiving ERP import must accept the chosen
  spelling. If it turns out to need one fixed form, the export gets its own setting.
- **Read where the database already is** (`db.settings()` in `stage`, `resolve`, `export`), and
  passed into `clean::open` because a cleaning holds no store. `format::uic_display` stays grouped
  for the parsers' own messages, which run before any setting is in reach.
- **Existing cleaned copies are not rewritten.** The setting also changes what counts as a Format
  change: in a compact Schattensystem a compact number is already clean.

## The master is refreshed sheet by sheet, into a copy (appended 2026-10-03)

The first real master (`data/`, never committed) is not a list the app could own rows in. It is a
**hub**: one dashboard sheet of ~9,000 `VLOOKUP`s keyed on the Wagennummer, over about fifteen sheets
that people fill by **pasting portal exports** into them — the telematics portal, the wheelset
monitoring, the workshop orders, the revision plan. The three sender files analysed so far are exactly
three of those exports. So the app feeds the master the way a person does: it refreshes the paste
targets and leaves the dashboard, the curated sheets and everything else alone.
The old `Wartungen` upsert keyed on our own `Id` was written before any real master existed and is gone.

- **One binding per sheet, sheet ← template** (`data/trains/master.json`, `MasterSettings`). The source
  is the template's LATEST filed `Dokument`, read from its cleaned copy — every mapped value there is
  canonical, so one fixed reading types it exactly. Sheet names are the customer's, so the bindings are
  user data and never `builtin.rs`.
- **The original is never written.** The result is `<Name> <Datum>.xlsx` beside it. That replaces the
  `.bak`: the user compares and switches, and the one file whose loss ends the project cannot be lost
  here. With nothing written there is also no reason left for `MASTER_FILE` in `.npconfig`; the file
  is picked in the app, which a Citrix desk can do and `.npconfig` editing it cannot.
- **Columns by header, positions untouched.** The dashboard's lookups are positional (`A:K,10` is
  "Stadt" only because Stadt is the tenth column), so values go into the master's columns found by
  header text, and an alias maps a renamed one (`Empfangsdatum` ← `empf_datum`).
- **Two modes, the two shapes a sender file comes in.** *Stand ersetzen* (snapshot: the telematics
  list, the wheelset monitoring) clears rows 2… and writes the file; hand-kept columns travel by key if
  one is named. *Fortlaufend ergänzen* (feed: the order list) upserts by key and never deletes — the
  same split as "Workshop orders are a feed" and "Snapshots of fitted radsaetze" above.
- **Types follow the master, because the keys do.** The exports deliver `"3385 0659 152-2"`,
  `"180028676"`, `"6715.0"` as text; the pasted sheets hold numbers, and a text key misses a number in
  every VLOOKUP. Wagennummer, dates and amounts are typed from the field; any other plain decimal goes
  in as a number where most of the master column already holds numbers — never the other way round.
- **Formula columns are re-emitted as plain per-row formulas** (umya's `set_coordinate` shifts them).
  Excel's shared-formula groups do not survive clearing a snapshot's stale rows. Cached results are
  left stale on purpose: the written book carries an older `calcId`, and Excel recalculates on open.
- **umya `lazy_read` is the writer, measured, not assumed** — see `footguns.md`, "Writing a workbook
  back". On the real 20 MB master the three-sheet refresh takes 1.9 s, and only the three bound sheets
  change. The fallback, a part-level zip patch with the `zip` crate, stays unbuilt unless Excel ever
  refuses a copy.

Deliberately not done here: importing the master's own sheets into the Schattensystem. They are listed
as candidates in `fachdomaene.md` §10. **Superseded 2026-10-04** by „Der Master wird gespiegelt“ below;
the module moved from `trains/export/master/` to `trains/master/` and `export/` keeps only the ERP file.

## Der Master wird gespiegelt (appended 2026-10-04)

Die Gegenrichtung zum Refresh: die Master-Datei des Kunden wird ins Schattensystem gelesen. Entschieden
in einem Drill mit Martin am 2026-10-04; die Analyse aller 28 Blätter liegt lokal in
`data/Übersicht KundS.struktur.md` (echte Namen, nie im Repo).

- **Wiederkehrend, und ein SPIEGEL, kein Merge.** Bis zur Umstellung pflegt der Kunde in Excel; die App
  hat keine Bearbeitung. Jeder Lauf leert die FAKTEN (Wagen, Radsatz, Einbau, Instandhaltung) und baut
  sie aus der Kundendatei neu auf. Ein Merge müsste zwischen zwei Werten des Kunden selbst entscheiden;
  ein Spiegel nie. Damit ist auch die Echo-Schleife weg: liest der nächste Import eine vom Refresh
  geschriebene Fassung, wird der Spiegel ersetzt, nicht um eigene Werte „ergänzt“. Das Schattensystem
  als führendes System ist v3. Hauptziel bleibt der Refresh: den Master aktuell halten.
- **Identität bleibt, Fakten gehen** (`TrainsDb::clear_mirror`). Partner mit gelernten Aliasen, Vorlagen
  mit gelernten Lesarten und die abgelegten Dokumente (Quelle des Refresh) überleben; Dokumente werden
  wieder importierbar. Kein separates `PartnerAlias`: ein Alias ist ein Zeiger und überlebt nur mit
  seinem Ziel. Radsätze werden bewusst NICHT behalten — innerhalb des Masters ist RadsatzID ↔
  Radsatznummer 1:1 und es gibt einen Absender, also entsteht keine Mehrdeutigkeit.
- **Kein Dokument für Master-Blätter.** Der Refresh nimmt je Vorlage das neueste Dokument; ein abgelegtes
  Master-Blatt würde zur Quelle, die der nächste Refresh in den Master zurückschreibt. Die Staging trägt
  stattdessen `StagingOrigin::Master { sheet }` — das öffnet `commit` für sie und hält das Gate „nur
  bereinigte Dokumente“ für jede andere Datei geschlossen.
- **Ein Modul je Blatt-ART, nie je Blatt** (`trains/master/kinds/`). Die Blattnamen des Kunden enthalten
  Firmen- und Personennamen und stehen nur in `master.json`, gebunden an eine Art. Die Vorlagen der
  Arten sind NICHT in `builtin::all()`: der Bestand ohne Position würde jede Datei mit Position
  ebenfalls treffen und sie in Bereinigen `Mehrdeutig` machen.
- **Blatt für Blatt, der Nutzer entscheidet Konflikte, die App wählt keine Quelle.** AllERADSätzE und
  AL-ECHO-Üsicht sind derselbe Portalbericht zu zwei Zeitpunkten, keine SVERWEISE. Gefragt wird nur
  (a): derselbe Radsatz am SELBEN Wagen mit anderem Einbaudatum — ein Einbau mit strittigem Datum,
  keine Bewegung. „Übernehmen“ korrigiert das Datum an Ort und Stelle, ohne Antwort bleibt das
  gespeicherte. Nie wird dabei ein Einbau geschlossen: das hätte Phantom-Ausbauten erzeugt. Ein Radsatz,
  den nur der Bestand kennt, kommt als zusätzlicher offener Einbau „ohne Position“ — sichtbar zur
  Sichtprüfung, nicht weggeraten.
- **Keine Achszahl**, weder modelliert noch abgeleitet: aus offenen Einbauten abgeleitet machte sie die
  57 Wagen mit nie ausgebuchten Altsätzen zu 5- bis 8-Achsern.
- **`ausbau_am`/`aus_wagen` der Radsatz-Exporte gehören zum VORIGEN Einbau** und bleiben unverknüpft.
- **Lesen im Master-Pfad** (`grid::from_master`, `stage` mit `master`): der bis Zeile 1.048.576 gefüllte
  Null-Schwanz wird abgeschnitten statt abgelehnt; zwischengespeicherte Formelfehler (`#N/A`) sind leer;
  `0` in einer DATUMS-Spalte ist leer (in einer Betragsspalte bleibt `0` ein Betrag); ein Fehler in
  IRGENDEINER zugeordneten Zelle verwirft die Zeile und listet sie — eine still verkürzte Zeile ginge
  bei der Sichtprüfung als korrekt durch. Der strenge Bereinigen-Pfad bleibt unverändert.
- **Ein abgebrochener Lauf ist sichtbar unvollständig** (`MasterSettings.import_run`, vom Backend
  verwaltet; `save_master` übernimmt ihn nie von der Seite).
- **Fälligkeiten werden gezeigt, nicht berechnet.** Zyklusregeln je Fristart zu besitzen ist v3.
- **Bestellnummer + Wagennummer verknüpfen ungefragt**, die Bestellnummer allein nie (Sammelbestellung).
- **Nichts wird ausgelassen.** Der Kunde disponiert Wagen und prüft Rechnungen selbst; jedes Blatt, das er
  pflegt, dient einem dieser Jobs. Die Phasen folgen seinen drei Jobs — wo ist der Wagen / stimmt die
  Rechnung / wer muss wann in die Werkstatt — nicht den Entitäten.
- **Blattansicht als Backend-for-Frontend** (`master::sheet_view`, Command `get_master_sheet`): Rust baut
  jede Ansicht vollständig — Spalten in Blattreihenfolge inkl. leerer Köpfe, je Spalte ob die App sie
  füllt, fertig formatierte Zeilen. Die Spalte→Feld-Zuordnung ist derselbe `rebind` wie beim Import,
  also kann die Ansicht ein Blatt nicht anders lesen als der Import, der sie füllt.
- **Gelesen wird eine Lesekopie, nicht die Kundendatei** (`trains/master/prepare.rs`). umya liest ein
  Blatt nur ganz, und fünf Blätter sind bis Zeile 1.048.576 gefüllt. Der eine Lesedurchgang, der die
  Kopfzeilen holt (`bindings::sync`), schneidet deshalb jedes Blatt hinter der letzten echten Zeile ab
  und schreibt das Ergebnis nach `data/trains/master/<Name> Lesekopie.xlsx` — einmal je Dateistand,
  danach lesen Import und Blattansicht nur noch die Kopie. Bewusst umya behalten statt eines eigenen
  Streaming-XML-Lesers: keine zweite Leseschicht, die vom Rest der Pipeline abweichen könnte. Die Kopie
  liegt nie neben der Kundendatei (eine zweite Mappe in seinem Ordner würde geöffnet und bearbeitet)
  und heißt nicht „bereinigt“ (das ist der Bereinigen-Gang). Der Refresh liest weiter das ORIGINAL: er
  schreibt eine datierte Kopie mit jeder Zeile des Kunden. Gemessen 2026-10-04: Kopie in 7,9 s, Spitze
  ~2 GB, 935 KB groß; danach jedes Blatt in 14–140 ms gestaget, alle 28 Blätter zellgleich zum Original.

Gemessen am echten Master (lokal, 2026-10-04): 405 Wagen, 1.207 Einbauten mit Position, 13
Datumskonflikte, 533 zusätzliche Radsätze ohne Position, 0 verworfene Zeilen; drei Blätter in 3,5 s,
Spitze ~750 MB.

Deliberately not done here (Phasen 2–5 im Plan): Wagenmeldung inkl. der Handfarben des Dashboards,
Telematik-Gerät, Werkstattauftrag, Rechnungsaufteilung (Matrix-Leser), Leistungskatalog, Frist,
Werkstattbedarf, Standorte, Bauteile, Radsatz-Messwerte, Wagen-Stammdaten.
