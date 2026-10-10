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

## Export in die Master-Datei (appended 2026-10-04)

Der Ein-Klick-Refresh („Master aktualisieren“: jedes Blatt aus dem neuesten Dokument seiner Vorlage)
ist ersetzt durch einen **Assistenten je Dokument**, gestartet in der Dokumentliste mit „In Master
übertragen“: Blätter wählen → Spalten abgleichen → Vorschau → Ergebnis. Entschieden mit Martin am
2026-10-04. Code: `trains/master/export/`, Seiten `trains/master/feature/export-*`.

- **Vorschlag aus der Bindung, keine neue Vorlagenart.** `MasterBinding.template_id` war die
  Refresh-Quelle und ist jetzt die gemerkte Zuordnung „Dokumente dieser Vorlage gehen in dieses
  Blatt“. Vorgeschlagen wird ein Blatt, dessen Bindung die Vorlage des Dokuments nennt (oder eine
  Nutzerkopie derselben eingebauten), sonst eines ohne Vorlage, das jede zugeordnete Spalte des
  Dokuments trägt. „Merken“ (Standard an) schreibt Auswahl, Schlüssel, Aliase und Ignorierte auf die
  Bindungen; ein abgewähltes, vorher gemerktes Blatt verliert die Vorlage.
- **Modus bleibt der der Bindung.** *Stand ersetzen* und *Fortlaufend ergänzen* gelten wie zuvor,
  eine Regel für alle Wege.
- **Konflikt heißt Struktur, nie Wert.** Jedes Dokument ist ein Inkrement; ein abweichender Wert ist
  die Aktualisierung selbst und steht in der Vorschau. Gefragt wird nur, wenn eine Spalte des
  Dokuments im Blatt kein Gegenstück hat — beantwortet mit einem Alias auf eine von Hand gepflegte
  Spalte oder „nicht übertragen“ (`MasterBinding.ignored`, neu). Beides wird gemerkt, also fragt das
  nächste Dokument derselben Vorlage erst wieder, wenn sich das Blatt oder die Datei geändert hat.
  Ein gemerkter Alias oder Schlüssel, den das Blatt nicht mehr hat, wird verworfen und GESAGT, nie
  still umgebogen. Weiter bleibt gesperrt, solange eine Spalte offen ist oder ein Blatt gar nicht
  geschrieben werden kann.
- **Vorschau = echter Schreiblauf.** `preview` fügt in ein Buch im Speicher ein und vergleicht das
  Blatt vorher/nachher Zelle für Zelle (`export/diff.rs`), nur in den Spalten, die `paste` schreibt
  (Formelspalten werden neu ausgegeben, ihre zwischengespeicherten Werte sind absichtlich alt).
  `write` ist derselbe Lauf plus Speichern. Eine Vorhersage wäre ein zweites `paste`, das beim ersten
  Unterschied lügt. Dafür kostet jede Antwort im Abgleich einen Lauf über die gewählten Blätter des
  ORIGINALS (nicht der Lesekopie, die nicht jede Zeile hat).
- **Aufbauend auf der letzten Kopie.** Ergebnis ist wie bisher `<Original> <Datum>.xlsx` neben dem
  Original, das nie geschrieben wird. Die Kopie wird als `MasterSettings.last_export` gemerkt und ist
  beim nächsten Export die vorgewählte Ausgangsdatei, solange sie nicht älter als das Original ist —
  so sammeln sich mehrere Dokumente in einer Datei, ohne dass Änderungen verloren gehen, die der Kunde
  inzwischen im Original gespeichert hat. Andere Pfade als Original und letzte Kopie lehnt das Backend
  ab.
- **`paste` bekam eine Lesehälfte** (`structure`: zugeordnet / von Hand / ohne Ziel) und meldet, welche
  Spalten es schreibt; sonst ist das Einfügen unverändert das des Refresh.

- **Kein Blatt ist gesperrt, Vorschläge stehen oben.** Die Erkennung bindet jedes Blatt mit
  Wagen-Schlüsselspalte als Art Wagenliste (Übersicht) — am echten Master 22 von 29, darunter die
  Einfügeziele `ECHO_Eingänge` und `Telematik`. Die erste Fassung sperrte diese Art und damit genau die
  Blätter, die vorgeschlagen gehörten. Jetzt gilt: eine gemerkte Vorlage schlägt vor, egal welche Art;
  eine Übersicht ohne Vorlage wird nie vorgeschlagen und trägt einen Warnhinweis, lässt sich aber
  ankreuzen — die Vorschau zeigt jede Zelle, bevor etwas geschrieben wird.

Bewusst nicht gebaut: mehrere Dokumente in einem Lauf (die Kette über `last_export` deckt das ab);
Konflikte auf Wertebene.

## Die Master-Datei als Datei (appended 2026-10-04)

Die Master-Datei des Kunden kommt als eigene Datei ins Programm, getrennt von allem, was bisher
„Master“ heißt (Bindungen, Spiegel, Export — die laufen aus). Entschieden mit Martin am 2026-10-04.
Code: `trains/master_file/`, Seite `/trains/master-file`, oben angeheftet in der Dokumentliste.

- **Eigener Weg, eigene Seite.** Dateiauswahl → Bereinigung → Übernehmen/Verwerfen. Nicht über den
  Bereinigen-Hub: der bereinigt eine Absenderdatei über eine Vorlage und fragt je Spalte; die Master
  hat keine Vorlage, und ihre Bereinigung fragt nichts.
- **Eigener Datensatz, kein `Dokument`.** Eine Master wird nie per Import-Walk ins Schattensystem
  übernommen und nie in sich selbst exportiert. Als `Dokument` mit Kennzeichen hätte jeder Leser der
  Dokumente einen Wächter gebraucht; ein eigener Typ macht das Vergessen unmöglich.
- **Nur eine, in Fassungen.** Der Kunde erzeugt die Datei immer wieder. Jede Wahl ist eine neue
  Fassung DER Master und ersetzt die vorige als aktuelle (`versions[0]`); ältere bleiben als
  Verlauf. Gleiche Bytes werden abgelehnt (Inhalts-Hash wie beim `Dokument`). Unsere datierten
  Export-Kopien gehören ausdrücklich nicht dazu.
- **Eingezogen wie ein Dokument** (`dokument::adopt`): das Original wird nach
  `data/trains/masterdatei/<id>/` kopiert, bevor es gelesen wird; die Kundendatei wird nie geschrieben.
- **Bereinigt wird nur, was nichts bedeuten kann.** Nie eine Formel innerhalb der Daten (auch nicht
  die Kinder einer geteilten), nie eine verschobene Zeile. Abgeschnitten werden leere Zellen unterhalb
  der letzten Zeile mit Wert oder Formel — am echten Master drei Blätter mit je rund einer Million
  gestylter Leerzeilen. Ein Schwanz, in dem bis zum Blattende nur noch `0`/`#NV` steht (am echten
  Master heruntergezogene Formeln), behält **3 Zeilen**, der Rest geht — Martins Entscheidung, damit
  das Muster zum Weiterziehen bleibt. Weil das die einzige Stelle ist, an der Formeln entfernt werden,
  wird es getrennt gezählt (`tail_rows_cut`) und in Vorschau, Summen und Dokumentzeile genannt; eine
  geteilte Formel, deren Bereich in den Schnitt reichte, endet an der letzten behaltenen Zeile. Text-Zahlen und Text-Daten werden nur
  umgetypt, wo die Spalte schon überwiegend Zahlen bzw. Daten hält und die Schreibweise eindeutig
  ist; führende Nullen, `1,234`, `01/02/2025` und zweistellige Jahre sind Hinweise. Leerzeichen am
  Rand (auch NBSP) gehen. Die Kopfzeile bleibt, weil Bindungen und `VERGLEICH`-Formeln sie lesen.
- **Ein Blatt ohne Befund bleibt Byte für Byte.** umya verliert in jedem Blatt, das es liest und
  zurückschreibt, Kleinigkeiten (footguns.md). Darum wird zweimal gelesen: ein Probe-Buch bereinigt
  jedes Blatt und wird verworfen, im geschriebenen Buch werden nur die geänderten Blätter
  deserialisiert. Gemessen am echten Master: 18 s, Spitze ~1,7 GB, 12 von 28 Blättern bytegleich,
  19,8 → 10,7 MB. `customXml/` und `calcChain.xml` fallen trotzdem weg (bekannt, Excel rechnet neu).
- **Plan/Apply mit Dateien auf der Platte.** Die bereinigte Fassung liegt als `pending` in
  `masterdatei.json`, bis sie übernommen oder verworfen wird — ein ganzes Buch zwischen zwei Commands
  im Speicher zu halten, wären Gigabyte.

Bewusst offen: Bindungen, Spiegel und Export lesen weiter `MasterSettings.file`, nicht die übernommene
Fassung — sie werden abgelöst, nicht angeschlossen.

## Die Kunden-Master ist die einzige Master (appended 2026-10-04)

Ersetzt den offenen Punkt am Ende von „Die Master-Datei als Datei“ und Teile von „Export in die
Master-Datei“. Entschieden mit Martin am 2026-10-04: „get rid of our own master … client master is the
only thing right now“.

- **Keine zweite Datei.** `/trains/master` wählt keine Datei mehr (`pick_master_file` ist weg).
  `MasterSettings.file` ist abgeleitet: `bindings::follow`, als erstes in jedem `sync`, zeigt es auf
  die bereinigte Kopie der aktuellen Fassung und ist der einzige Schreiber. Bindungen, Spiegel,
  Blattansichten und Aktualisierung lesen damit alle dieselbe Datei. Ohne übernommene Fassung gibt es
  keine Master.
- **Eine neue Fassung behält die Bindungen.** Sie ist dieselbe Arbeitsmappe noch einmal: Bindungen
  bleiben nach Blattname, neue Blätter bekommen die Standardzuordnung, nur Scan und Lesekopie werden
  neu gelesen — beim Übernehmen (`accept_master_file` ruft `sync`) bzw. beim nächsten Leser.
  `save_master` kann `file` nicht setzen.
- **„Master aktualisieren“ schreibt eine neue Fassung, keine datierte Kopie.** Der Assistent pro
  Dokument (Blätter → Spalten → Vorschau mit „Übernehmen“ → Zusammenfassung) baut immer auf der
  aktuellen Fassung auf; das Ergebnis wird über `master_file::updated` als neue `versions[0]`
  gespeichert, unter dem Dateinamen der Kunden-Master, mit `quelle` = Dokumentname. `last_export` und
  die Wahl der Ausgangsdatei sind weg — die Kette mehrerer Dokumente entsteht dadurch, dass jede
  geschriebene Fassung die aktuelle wird. Die Anfrage nennt die Ausgangsdatei weiterhin, damit eine
  zwischen Vorschau und Schreiben übernommene Fassung abgelehnt statt überschrieben wird. Das hebt
  „Unsere datierten Export-Kopien gehören ausdrücklich nicht dazu“ auf: es gibt keine solchen Kopien
  mehr.
- **Die Aktion steht nur da, wo sie geht.** „Master aktualisieren“ erscheint in der Dokumentliste nur,
  solange eine Kunden-Master übernommen ist — nicht mehr hinter dem Schalter `npdh.master`. Der
  Schalter versteckt weiter Kachel und Hinweis des Master-Imports auf dem Dashboard.

## Die Master-Aktualisierung ist immer inkrementell (appended 2026-10-04)

Ersetzt „Two modes, the two shapes a sender file comes in“, „Modus bleibt der der Bindung“ und „Kein Blatt
ist gesperrt, Vorschläge stehen oben“. Entschieden mit Martin am 2026-10-04: „for the master it's always
incremental“; vorausgewählt wird jedes Blatt, „that would be affected by the data“.

- **Kein Ersetzen mehr.** `MasterMode` ist weg, aus Bindung, Assistent und `/trains/master`. Zeilen werden
  über die Schlüsselspalte zugeordnet: eine bekannte Zeile bekommt die Werte der gemeinsamen Spalten,
  Formel- und Handspalten derselben Zeile bleiben, gelöscht wird nie (`paste::incremental`). Ein `mode`
  in einer alten `master.json` wird beim Lesen ignoriert.
- **Wagennummern vergleichen über ihre Ziffern** (`paste::match_key`): die bereinigte Kopie schreibt die
  gewählte Schreibweise (`3385 0659 152-2`), die Master hält meist die Zahl.
- **Doppelte Schlüssel im Dokument sperren das Blatt** mit einer Meldung, statt die letzte Zeile still
  gewinnen zu lassen (die Werkstattaufträge nennen einen Wagen mehrfach — dort muss im Abgleich eine
  eindeutige Schlüsselspalte gewählt werden). Doppelte Schlüssel im Blatt: die erste Zeile wird
  aktualisiert, das wird gesagt.
- **„Betroffen“ heißt: Schlüssel plus mindestens eine weitere gemeinsame Spalte.** Schlüssel ist der der
  Bindung, sonst die Wagen-Spalte des Blatts (`kinds::wagen_column`; Radsatzblätter nicht, dort steht ein
  Wagen je Radsatz). Heißt sie anders als die Wagennummer-Spalte des Dokuments, verknüpft ein Alias beide
  (`TRANSPORTMITTELNR` ← `Asset`). Dazu jedes Blatt, dessen Bindung die Vorlage des Dokuments nennt.
- **Anhängen nur im Blatt der Vorlage** (`append`, je Blatt umschaltbar). Ein nur betroffenes Blatt
  bekommt seine vorhandenen Zeilen aktualisiert — sonst wüchse eine Projektliste um jeden Wagen eines
  Telematik-Exports. Nur ein Blatt mit `append` fragt im Abgleich nach Spalten ohne Gegenstück; „Merken“
  setzt die Vorlage nur auf Blätter mit `append`.
- **Der gelbe Übersichts-Hinweis ist weg**: er warnte davor, dass Ersetzen Formeln und Notizen löscht, und
  das Ersetzen gibt es nicht mehr.

Nebenbei, auf Martins Wunsch: „Importieren“ und der Status-Chip in der Dokumentliste sind vorerst hinter
dem Schalter `npdh.import` versteckt (`SettingsService.importEnabled`); der Import-Weg selbst ist
unverändert.

## Vorausgewählt wird nur das Blatt der Vorlage (appended 2026-10-05)

Ändert „Die Master-Aktualisierung ist immer inkrementell“ in einem Punkt. Entschieden mit Martin am
2026-10-05, nach Rückmeldung des Kunden: je Dokument wird **ein** Blatt aktualisiert. Ein
Telematik-Export bekam bisher `Telematik` und `TelematikProjekt` vorausgewählt; gewollt ist nur
`Telematik`.

- **Vorausgewählt ist nur noch das Blatt, dessen Bindung die Vorlage des Dokuments nennt**
  (`export::suggest::start`, `suggested = remembered`). „Betroffen“ — Schlüssel plus eine gemeinsame
  Spalte — wählt nichts mehr vor und trägt keinen Grund mehr.
- **Angeboten wird weiter jedes Blatt**, und Schlüssel samt Alias werden für jedes ermittelt: wer ein
  weiteres Blatt von Hand ankreuzt, bekommt dort die vorhandenen Zeilen aktualisiert wie bisher.

## Spalten werden im Abgleich frei zugeordnet (appended 2026-10-05)

Entschieden mit Martin am 2026-10-05. Anlass: `Radsatzmonitoring KNE ….xlsx` gehört in das Blatt
`RSmonitoring`, aber die Datei heißt die Spalte `RadsatzID` und das Blatt `Radsatz ID`. Spalten werden
nur über die gleiche Überschrift gepaart, also fand sich nichts — und gefragt wurde nur auf dem Blatt der
Vorlage, das `RSmonitoring` mangels Bindung nicht war.

- **Schritt 2 („Spalten abgleichen“) zeigt je Blatt jede Zuordnung** Dokument-Spalte → Blatt-Spalte, ob
  über den Namen oder von Hand, und lässt beide Seiten ändern, eine entfernen („nicht übertragen“) oder
  eine aus den freien Spalten hinzufügen. Auf jedem angekreuzten Blatt, nicht nur dem der Vorlage. Der
  Ort ist der Assistent und keine Vorlagenseite: er ist mit der Kürzung der App der einzige erreichbare,
  und gemerkt wird ohnehin an der Bindung des Blatts.
- **Eine Dokument-Spalte landet in genau einer Blatt-Spalte.** Eine Spalte, die von Hand woandershin
  zeigt oder „nicht übertragen“ ist, wird nicht zusätzlich über ihren Namen gepaart
  (`paste::classify`); eine zweite Zuordnung derselben Spalte wird verworfen und gesagt. Mehrfach-
  Zuordnungen (eine Spalte in zwei Blatt-Spalten) sind bewusst nicht vorgesehen.
- **Der Schlüssel ist ein Paar**: Schlüsselspalte des Dokuments → Schlüsselspalte des Blatts, in einem
  Schritt gepaart und zum Schlüssel gemacht. Für `RSmonitoring` ist das `RadsatzID` → `Radsatz ID` —
  ein Radsatzblatt nennt einen Wagen je Radsatz, über die Wagennummer wäre der Schlüssel nie eindeutig.
- **Formelspalten bleiben außen vor**: angeboten werden nur Spalten, die schon gepaart oder von Hand
  gepflegt sind (`targets`).
- **Radsatz-Monitoring bringt seine Master-Zuordnung mit** (`builtin::master_hint`): Schlüssel `RadsatzID`,
  und `RadsatzID` heißt in der Master `Radsatz ID`. Ein Blatt, das unter dieser Schreibweise alle Spalten
  der Vorlage trägt, wird ohne Klick ihr Blatt, mit Schlüssel `Radsatz ID` ← `RadsatzID`
  (`bindings::default`). Kein Blattname im Code — der steht nur in `master.json`. Gegen die echte Master
  geprüft: genau `RSmonitoring`; `nodepit` fehlt das Einbaudatum, `aktuelleNodepit` die Wagennummer.
  Wirkt auch auf bestehende Installationen: jede Bindung, die niemand angefasst hat (`auto`), wird bei
  jedem `sync` aus der gespeicherten Kopfzeile neu abgeleitet — eine bessere Vorgabe in einem Update
  erreicht sie also. Eine von Hand geänderte Bindung (`auto: false`, Masterseite oder „Merken“) bleibt,
  wie sie ist.

## Fehlende Zeilen leeren, je Blatt umschaltbar (appended 2026-10-07)

Ändert „Die Master-Aktualisierung ist immer inkrementell“ in einem Punkt: „gelöscht wird nie“ gilt
nur noch als Vorgabe. Entschieden mit Martin am 2026-10-07: drei Schritte, unabhängig voneinander —
vorhandene Zeilen aktualisieren (immer), neue anhängen (`append`), fehlende leeren (`remove`).

- **Geleert, nicht gelöscht.** Jede Zelle der Zeile verliert Wert und Formel, ihr Format bleibt; die
  Zeile selbst bleibt stehen. Keine Zeile rückt nach, also verschiebt sich KEIN Bezug — weder im Blatt
  noch aus anderen Blättern, die ein Löschen nicht hätte nachziehen können, weil nur das eine Blatt
  gelesen wird. Ein Bezug auf die geleerte Zeile liest leer, und das ist sie jetzt. Ein erster Entwurf
  löschte Zeilen und nahm verschobene Bezüge in Kauf; das war nicht akzeptabel. Der Preis ist eine
  Lücke im Blatt; angehängt wird weiter unter der letzten Zeile.
- **Lücken werden nie aufgefüllt** (Martin, 2026-10-07). Ein Bezug auf eine geleerte Zeile liest leer
  — richtig. Füllte ein neuer Wagen die Lücke, zeigte derselbe Bezug still die Werte eines ANDEREN
  Wagens.
- **Angeboten auf JEDEM angekreuzten Blatt**, nicht nur dem der Vorlage. Ob ein Dokument für ein
  Blatt vollständig ist, weiß nur der Benutzer; eine Einschränkung im Code wäre geraten.
- **Gemerkt je Blatt UND Vorlage** (`MasterBinding.remove_for`, Vorlagen-IDs; jede Kopie derselben
  mitgelieferten Vorlage zählt), über „Merken“ in der Vorschau. Für dieselbe Art Dokument ist die
  Antwort immer dieselbe. Sonst aus: anders als ein falsches `append` leert ein falsches `remove`.
- **Geleert wird jede Kopie eines doppelten Schlüssels**, **eine Zeile ohne Schlüssel nie** — Summen,
  Notizen, der Formel-Schwanz haben keinen.
- **Stünde kein einziger Schlüssel des Dokuments im Blatt, wird das Blatt abgelehnt** statt geleert:
  das ist fast immer eine falsche Schlüsselspalte, nicht eine gewollte Leerung.
- **Die Vorschau nennt jede geleerte Zeile mit ihrem Schlüssel**, statt sie Zelle für Zelle als
  Änderung zu listen (`diff::changes`).
- **Eine geteilte Formel, deren erste Zelle geleert wird, wird vorher zu einfachen Formeln** — nur
  ihren Text trägt nur die erste Zelle, der Rest der Gruppe stünde sonst ohne da.

## Ein älteres Dokument wird gewarnt, nicht abgewiesen (appended 2026-10-07)

Entschieden mit Martin am 2026-10-07. **Entschieden, noch nicht gebaut** (→ state.md).

Anlass: Die Master-Aktualisierung vom 05.10. schrieb einen Telematik-Export vom 02.10. über einen
Master, der schon den Stand vom 05.10. hatte. Danach war „Timestamp“ in 360 von 403 Zeilen älter, in
keiner neuer, und der Standort fiel mit zurück. Gesagt wurde nichts. `paste::incremental` überschreibt
mit Absicht (jedes Dokument ist ein Inkrement) und vergleicht nicht, welche Seite aktueller ist.

- **Die Vorschau warnt je geschriebener Spalte**, wenn mehr Daten zurückgingen als vorrückten, und nennt
  beide Zahlen, etwa: „„Timestamp“: 360 Daten würden älter, 0 neuer. Ist das Dokument älter als der Stand
  der Master-Datei?“ Abgewiesen wird nichts, je Zeile entschieden wird nichts. Ob ein älteres Dokument
  gewollt ist (eine Korrektur, ein nachgereichter Stand), weiß nur der Benutzer.
- **Kein „neuer gewinnt“ je Zeile.** Das wäre eine Vorrangregel im Einfügen, und solche Regeln bleiben aus
  dem Master-Weg heraus (der Benutzer entscheidet, siehe „Der Master wird gespiegelt“). Welche Spalte den
  Stand trägt, ist auch nicht vorher bekannt. Die Warnung braucht keine Bindung, sie gilt für jede
  Datumsspalte.
- **Geplante Form:** direkt neben `diff::changes`, auf denselben Rastern vorher und nachher und denselben
  Spalten. So beurteilt sie genau das, was geschrieben würde, und hat nichts vorherzusagen. Ein Datum ist
  eine Seriennummer mit Datumsformat oder ein Text, den `date::parse_text` ohne Warnung liest.
  Zweistellige Jahre zählen nicht, damit Schadcodes wie `3.3.4` keine Daten werden. Eine Zahl ohne
  Datumsformat ist kein Datum. Verglichen wird nach Kalendertag. Ein eigenes Feld `warnings` an
  `MasterExportSheetRun` (Vertrag doppelt: `model.rs` und `trains.types.ts`, dazu der Fake). In der
  Vorschau wird es dargestellt wie die Konflikte in Schritt 2.

## Der Wagen-Zustand ist typisiert (appended 2026-10-07)

Entschieden mit Martin am 2026-10-07. Das Schattensystem wird zum **Informations-Hub** für alle
importierten Dokumente. Was das Blatt „Alle Wagen Überblick“ des Kunden heute über etwa 17 SVERWEISE,
Handspalten und zwei Handfarben zeigt, wird nicht nachgebaut, indem man Dokumente zur Laufzeit
nachschlägt. Stattdessen werden **typisierte Entitäten** gebaut, jede aus einer eigenen Absenderdatei
importiert. Für Dateien, die es noch nicht gibt, wird angenommen, dass es sie geben wird. Verworfen
wurde die Alternative „Spalte = Vorlage + Quellspalte, gelesen aus dem neuesten Dokument“: Die App
wüsste dann einen Wert, aber nicht, was er bedeutet, und Fristen, Status und Funkstille könnte sie
nicht selbst beurteilen.

- **Fünf Entitäten, KISS:**
  - `TelematikGeraet` (Schlüssel `kennung`, die Pointer-ID des Absenders) und `TelematikMeldung`, von
    der **nur die neueste je Wagen** bleibt.
  - `Schadensmeldung` (Schlüssel Wagen + gemeldet am + Schadcode).
  - `Werkstattauftrag` (Wagen + Bestellnummer).
  - `Pruefung` (Wagen + Art + fällig am). P8 ist die jährliche Inspektion, Revision G4.x die
    Hauptuntersuchung, und beide sind dieselbe Entität.
- **Kein neuer Schritt im Import.** Alles hängt am Wagen, den der Schritt „Wagen“ schon entschieden
  hat. Ein abgelehnter Wagen lässt seine Zeilen fallen wie bisher. Jedes Attribut hat eine eigene
  Spaltenart. Datum, Leistung, Betrag und Bemerkung werden absichtlich nicht wiederverwendet, sonst
  würde eine Telematik- oder Auftragszeile zur Instandhaltung.
- **Fortschreiben statt anhängen:** Ein gespeicherter Datensatz wird über seinen Schlüssel gefunden
  und überschrieben, eine leere Zelle löscht dabei nichts. Die Telematik-Meldung ist die Ausnahme:
  Sie ist eine Messung, und eine neuere ersetzt die alte ganz. Eine Position vom 05.10. neben einem
  km-Stand vom 02.10. wäre eine Messung, die es nie gab.
- **Die Telematik-Meldung behält ihre Uhrzeit.** Das ist die einzige Ausnahme von „Daten sind
  Kalendertage“, denn zwei Exporte desselben Tages lassen sich nur über die Uhrzeit ordnen
  (`sanitise::date::Zeitpunkt`). Die bereinigte Kopie schreibt `TT.MM.JJJJ hh:mm:ss`, sonst ginge die
  Zeit dort verloren.
- **Eine ältere Meldung wird übersprungen und im Bericht gezählt.** Damit kann der Rücksprung vom
  05.10. (Export vom 02.10. über den Stand vom 05.10., siehe „Ein älteres Dokument wird gewarnt“) im
  Schattensystem nicht mehr vorkommen. Für den Master-Paste gilt die dortige Entscheidung weiter.
- **Die Prüfart kommt aus der Spalte oder fest von der Vorlage** (`ImportPlan.pruefart`). Die
  P8-Liste hat keine Art-Spalte, weil die ganze Datei P8 ist. Die Zuordnung fragt nach der Art nur,
  wenn eine Prüfungsspalte zugeordnet ist und keine Art-Spalte.
- **Ein Werkstattauftrag braucht die Bestellnummer und noch eine Auftragsspalte.** Die P8-Liste
  nennt die Bestellnummer einer Prüfung, und das allein darf keinen leeren Auftrag erzeugen.
- **Die Auftragsliste ist keine Quelle für Instandhaltungen mehr.** `builtin:werkstattauftraege`
  ordnet `werk_ausg_ist` jetzt dem Werkstattausgang zu und nicht mehr dem Datum. Damit fällt die
  Notlösung aus „Workshop orders are a feed“ weg, bei der unfertige Aufträge von Hand abgehakt
  werden mussten. `vers_datum` bleibt ohne Zuordnung: Es ist der Versand des WAGENS, nicht des
  Auftrags.
- **Ein Speicher, nicht fünf:** `zustand.db` hält einen `WagenZustand`. Jede Liste hat ein paar
  hundert Zeilen je Flotte, und die Aufteilung, die die große Instandhaltungsdatei schützt, würde
  hier nur fünfmal so viel Verdrahtung kosten.
- **Berechnet, nicht gespeichert:** „Tage seit letztem Funk“ ergibt sich aus dem Zeitpunkt der
  Meldung (`util/wagen-zustand`). Über 7 Tagen ist es rot wie im Dashboard. „Offen“ heißt bei einer
  Schadensmeldung „kein erledigt am“ (die orange Zeile) und bei einem Auftrag „kein Ausgang“.
- **Noch offen:**
  - Keine Quelle füllt „Auftrag versendet am“ (die lila Zelle), deshalb zeigt die Liste dafür noch
    kein Abzeichen.
  - Der Master-Spiegel liest diese Entitäten noch nicht aus dem Master selbst.
  - Die Blattansicht des Masters zeigt ihre Spalten leer.

## Import nur ins Schattensystem, Master-Import leert (appended 2026-10-08)

Entschieden mit Martin am 2026-10-08.

- **„Importieren“ ist wieder da**, unter Dokumente und in der Zusammenfassung des Bereinigens, ohne
  Schalter (`npdh.import` entfällt). Ein Import schreibt **nur ins Schattensystem**.
- **„Master aktualisieren“ ist vorerst verborgen**, hinter `npdh.full` wie „Export erstellen“. Die App
  schreibt die Master-Datei also gerade nicht. Der Assistent bleibt erhalten und getestet.
- **Die Daten der Master-Blätter stellt weiter die App bereit**: der Master-Import und die
  Blattansichten. Die Kachel heißt „Master-Import“, getrennt von „Master-Datei“, wo nur die Datei
  übernommen wird. Sie ist ohne Schalter da (`npdh.master` entfällt).
- **Ein Master-Import leert die aktuellen Daten.** Wagen, Radsätze, Einbauten, Instandhaltungen und
  der Wagen-Zustand werden neu aus dem Master aufgebaut. Partner, Vorlagen und Dokumente bleiben,
  und die Dokumente lassen sich danach erneut importieren (`clear_mirror`). Die Bestätigung nennt
  den Wagen-Zustand ausdrücklich, weil der Master ihn heute noch nicht zurückbringt.
- **Die Entitäten-Kacheln sind immer sichtbar**: Wagen, Radsätze, Instandhaltungen und die drei
  Partnerrollen. Ohne `npdh.full` fehlen nur Vorlagen und Einstellungen.

## Entitäten als Karten, Detailseite aus Rust (appended 2026-10-08)

Entschieden mit Martin am 2026-10-08. Neben der Originalansicht der Master-Blätter (Tabelle) sind
die Entitätenlisten der zweite Blick auf die Daten.

- **Wagen, Radsätze und Partner werden als Karten gezeigt**, je Karte nur das Wichtigste:
  - Wagen: Bauart, Halter, Standort und Funkstille, offene Schäden und Aufträge, nächste Prüfung,
    Zahl der eingebauten Radsätze.
  - Radsatz: wo und seit wann er eingebaut ist.
  - Partner: Rollen und Schreibweisen.

  Die Instandhaltungen bleiben eine Liste: Sie sind seitenweise geladen und zahlreich.
- **Ein Klick öffnet die Detailseite** mit allem, was zur Entität gespeichert ist. Jede Zeile, die
  eine andere Entität nennt, führt zu deren Seite.
- **Gebaut wird die Detailseite in Rust** (`trains/detail/`, `get_entity_detail`), Backend for
  Frontend. Es gibt eine allgemeine Form (Felder, Abschnitte, Zeilen mit Link und Ton) und eine Seite
  für alle drei Arten. Angular entscheidet nichts über Inhalt oder Schreibweise. Die Karten bauen ihre
  Zeilen weiter aus den Listen im Store; das ist die schon bestehende Verknüpfung, nur leicht
  erweitert.
- **„Entfernen“ wandert auf die Detailseite.** Eine Karte ist selbst die Schaltfläche, und eine
  zweite Aktion darin wäre ein verschachteltes Bedienelement.
- **Lange Listen eines Partners werden auf 50 Zeilen gekürzt**, mit einer Zeile, die den Rest nennt.
  Eingebaute Radsätze stehen nach Position, die ohne Position zuletzt.

## Telematik als eigener Einstieg (appended 2026-10-08)

Entschieden mit Martin am 2026-10-08: Verschiedene Aufgaben brauchen verschiedene Zugänge zu den
Daten. Wer nach der Telematik sieht, fängt bei der Telematik an und nicht bei den Wagen.

- **Eine eigene Kachel „Telematik“ und eine eigene Liste** (`/trains/telematik`). Kein Filter auf der
  Wagenliste: Die Wagenliste ist der Zugang über den Wagen, die Telematik-Liste der über die Geräte.
- **Eine Karte je Wagen mit Gerät oder Meldung**, die am längsten stummen zuerst. Ein Gerät, das sich
  nie gemeldet hat, gilt als stumm und steht ganz oben. Ein Klick öffnet die Detailseite des Wagens.
- **Gebaut in Rust** (`trains/telematik.rs`, `get_telematik`), Backend for Frontend. Die Kachel zählt
  aus derselben Antwort, damit Kachel und Liste nie verschieden zählen.
- **Gezählt wird in Kalendertagen**, so wie das Kundenblatt Daten abzieht; rot ab mehr als 7 Tagen.
  Die Wagenkarte rechnet ihre Funkstille noch selbst, in TypeScript, mit derselben Schwelle. Kurz nach
  Mitternacht kann sie deshalb um einen Tag von der Liste abweichen. Ändert sich die Schwelle,
  müssen beide Stellen geändert werden.

## Master-Import ohne Rückfragen (appended 2026-10-08)

Entschieden mit Martin am 2026-10-08. Ergänzt „Blatt für Blatt, der Nutzer entscheidet Konflikte“,
ersetzt es nicht: Der blattweise Weg bleibt.

- **„Alles importieren“ liest die ganze Master-Datei in einem Lauf** (`master/import_all.rs`,
  `import_master_all`) und zeigt danach einen Bericht, eine Zeile je Blatt. Gefragt wird nur einmal,
  vorher, weil der Lauf die Fakten leert.
- **Es ist derselbe Lauf wie der blattweise**: `mirror::start`, je Blatt `stage_sheet` und `commit`
  mit allen Prüfungen, `done`. Ein Blatt, das scheitert, steht im Bericht und bleibt im Lauf offen.
- **Geantwortet wird wie mit „alle neuen anlegen“**: Bekanntes wird zugeordnet, Neues angelegt. Was nur
  ähnlich oder mehrdeutig ist, wird NICHT zugeordnet, sondern im Bericht genannt. Eine
  Radsatznummer entscheidet nie allein, auch nicht ohne Rückfrage. Ein strittiges Einbaudatum
  behält das gespeicherte; Dubletten und abgelehnte Zeilen bleiben draußen.

## Farben und der Spaltenfilter wie in Excel (appended 2026-10-10)

Entschieden mit Martin am 2026-10-10. Die Nutzer arbeiten in Excel und kennen dessen AutoFilter. Wagen-
und Radsatzliste bekommen deshalb je Spalte denselben Dialog: Sortieren (auf/ab, nach Farbe), Filter
nach Farbe, eine Textbedingung und eine durchsuchbare Werteliste mit „(Alles auswählen)“ sowie
„Automatisch anwenden“. Andere Listen bekommen ihn erst, wenn sie ihn brauchen. Eine Fassade, die
`columns` nennt, bekommt den Dialog statt der Sortierleiste.

- **Das ist eine Ausnahme von „Der Wagen-Zustand ist typisiert“.** Dort wurden die Handfarben bewusst
  nicht nachgebaut. Eine Farbe ist aber eine **Markierung**, keine Tatsache: Sie hält fest, was noch
  kein Feld hat („nachfragen“, „Montag prüfen“). Was eine Farbe heute im Kundenblatt bedeutet und was
  das Programm selbst beurteilen kann (Funkstille, offene Schäden, Fristen), wird weiter typisiert.
- **Sechs Farben, jede eine Ionic-Farbrolle** (Rot, Gelb, Grün, Blau, Lila, Grau). Ein beliebiger
  RGB-Wert würde die Regel „eine Komponente nennt nie eine Farbe“ brechen und im Dunkelmodus
  unlesbar werden. Orange fällt mit Gelb zusammen, weil Ionic für beides nur `warning` hat.
- **Beide Quellen, die Hand geht vor.** `markierungen.json` hat zwei Hälften. `hand` wird in der App
  gesetzt, auf der Detailseite. `master` ist die Füllfarbe der Schlüsselzelle in der Master-Datei:
  die Wagennummer auf einer Wagenliste, die Radsatznummer auf einer Radsatzliste. Das ist die Zelle,
  auf die Excels „Nach Farbe“ filtert. Zuerst gilt die Handfarbe, sonst die aus dem Master. „Keine“
  löscht also nur die Handfarbe.
- **Geschlüsselt nach Wagennummer und Radsatz-`match_key`, nicht nach id**, denn der Master-Import
  vergibt die ids neu. `clear_mirror` leert nur die Master-Hälfte, `reset` beide. Bewusst in Kauf
  genommen: Haben zwei Radsätze verschiedener Absender dieselbe Nummer, tragen beide dieselbe
  Markierung. Zusammengeführt wird dabei nichts; eine Markierung ist eine Notiz fürs Auge.
- **Die Master-Farbe wird beim Stagen gelesen und erst beim Commit gespeichert**
  (`farbe::merge_master`). Ein abgebrochener Gang hinterlässt keine Farbe. Das erste Blatt, das einen
  Schlüssel färbt, gewinnt. Bedingte Formatierung wird nicht gelesen: Sie ist eine Regel über Werte,
  und die Werte stehen schon im Schattensystem.
- **Gefiltert wird im Frontend**, mit reinen Funktionen (`@shared/util/item-lists/list-filter.ts`).
  Die Listen liegen ohnehin ganz im Store. Die Auswahl einer Spalte kommt, wie in Excel, aus den
  Zeilen, die die Filter der ANDEREN Spalten durchlassen.

## „Export erstellen“ schreibt eine Master-Übersicht (appended 2026-10-10)

Entschieden mit Martin am 2026-10-10. Ersetzt die ERP-Arbeitsmappe, die kein ERP je gelesen hat.

- **Eine neue Datei, `Master-Übersicht.xlsx`, jedes Mal frisch aus dem Schattensystem.** Die Master
  des Kunden wird dabei nicht angefasst; sie bleibt die Datei, die „Master aktualisieren“ schreibt.
- **Ein Datenblatt je Entität**: Wagen, Radsätze, Einbauten, Instandhaltungen, Telematik, Schäden,
  Aufträge, Prüfungen. Die Wagennummer steht immer in Spalte A. Die Blätter des Kunden (eingefügte
  Portal-Exporte mit ihren Eigenheiten) werden nicht nachgebaut, denn neue Exporte kommen über die App.
- **Die „Übersicht“ rechnet mit Formeln** (`MINIFS`/`MAXIFS`/`ZÄHLENWENNS` über ganze Spalten der
  Datenblätter), mit bedingter Formatierung gegen `HEUTE()`: Funkstille über 7 Tage, überfällige
  P8/Revision rot, offene Schäden und Aufträge orange. Wie beim Dashboard des Kunden bewegt eine Änderung in
  einem Datenblatt die Übersicht mit. Werte werden nicht zusätzlich geschrieben, denn Excel rechnet die
  Mappe beim Öffnen neu. Fristen werden gezeigt, wie sie in den Daten stehen, nie berechnet.
- **Typisierte Zellen**: Datum als Seriennummer, Wagennummer als Zahl in der kompakten Schreibweise
  (wie in den Portal-Exporten, sonst trifft kein SVERWEIS) und als Text in der gruppierten. Radsatz-,
  Bestellnummer und Schadcode sind Text.
- **Die Handspalten sind leer** (Bemerkungen/notwendige Aktion, AUSGESETZT, beladen, Auftrag noch zu
  versenden, Notiz) und haben eine eigene Kopffarbe. Kein Export übernimmt sie aus einem früheren.
  Das steht in der Legende in Zeile 2.

## MVP: In die Kunden-Master schreiben, mit Sicherung (appended 2026-10-10)

Entschieden mit Martin am 2026-10-10: „we need to go back to a MVP … only load and sanitize the data
sheets and then export them to the client's original master file“. Hebt „die Kundendatei wird nie
geschrieben“ (aus „Die Master-Datei als Datei“) und „schreibt eine neue Fassung“ (aus „Die
Kunden-Master ist die einzige Master“) auf.

- **Geschrieben wird in die Datei des Kunden, dort wo sie liegt.** „Master-Datei wählen“ merkt nur
  den Pfad (`MasterFile.pfad`, `pick_master_target`). Nichts wird kopiert, nichts bereinigt.
  `bindings::follow` nimmt `pfad` vor jeder Fassung. Die Lesekopie (`prepare`) kürzt die Formelschwänze
  weiter für die Leser; die Kundendatei selbst bleibt bis auf die angekreuzten Blätter unberührt.
- **Keine Schreibung ohne Sicherung** (`master_file::write_in_place`). Zuerst wird die Datei zum
  Schreiben geöffnet. Unter Windows schlägt das fehl, solange Excel sie offen hat, und dann wird
  nichts kopiert. Danach kommt eine Kopie nach `Sicherungen/<Name> <JJJJ-MM-TT hhmmss>.xlsx` neben der
  Datei (UTC, ohne Doppelpunkt). Erst dann wird über `write_book` ersetzt (Temp-Datei und
  Umbenennen). Scheitert die Kopie, wird nichts geschrieben. Ein zweiter Lauf in derselben Sekunde
  bekommt einen freien Namen (`free_path`).
- **Neben der Kundendatei, nicht in `data/`.** Dort sucht der Kunde nach der Sicherung. Und `data/`
  liegt nicht unbedingt auf demselben Laufwerk wie die Master.
- **Eine inzwischen geänderte Datei wird abgelehnt.** Vorschau und Schreiben vergleichen die mtime
  der Datei mit der des gespeicherten Scans. Ein Stand, den der Kunde in Excel gespeichert hat,
  während der Assistent offen war, wird also nie ungesehen überschrieben. Nach dem eigenen Schreiben
  liest `sync` neu ein, damit das nächste Dokument nicht abgewiesen wird. Das ersetzt die Prüfung „eine
  zwischen Vorschau und Schreiben übernommene Fassung“.
- **Was umya beim Schreiben verliert** (`customXml`, `calcChain`, siehe footguns.md), trifft jetzt die
  echte Datei. Erträglich wird das durch die Sicherung.
- **Das Menü trennt die beiden Aufgaben.** „Dokumente“ (`/trains`, `feature/start`, Startseite der App)
  enthält Bereinigen, Dokumente und Master-Datei. „Schattensystem“ (`/trains/erp`, das bisherige Dashboard)
  enthält das Schattensystem. Die Dokumente-Kachel steht in beiden, weil der Import dort beginnt.
  Nichts wurde gelöscht: Fassungen, Master-Bereinigung und `MasterFileReport` bleiben im Backend und
  in der Fassade, sind aber über keinen Knopf mehr erreichbar. Sie fallen weg, sobald das MVP beim
  Kunden bestätigt ist.
- **„In Master übertragen“ ist ohne Schalter da**, sobald eine Master-Datei gewählt ist. Es löst
  „Master aktualisieren“ hinter `npdh.full` ab.

## Master aktualisieren: ein Blatt, Zuordnung aus der Vorlage (appended 2026-10-10)

Entschieden mit Martin am 2026-10-10. Ersetzt für die Oberfläche „Angeboten wird weiter jedes Blatt“
(„Export in die Master-Datei“) und Schritt 2 aus „Spalten werden im Abgleich frei zugeordnet“.

- **Ein Dokument aktualisiert genau ein Blatt**: das, dessen Bindung die Vorlage des Dokuments nennt.
  Schritt 1 zeigt nur dieses Blatt, ohne Auswahl. Hat die Vorlage kein Blatt, sagt der Schritt, wo es
  zugeordnet wird („Master-Import“), und Weiter bleibt gesperrt.
- **Die Spaltenzuordnung kommt aus der Vorlage** — Schlüssel, Aliase und Ignorierte der Bindung, wie
  das Backend sie anbietet. Der Schritt „Spalten abgleichen“ ist entfallen; Schritt 1 nennt nur das
  Schlüsselpaar (nur lesend) und was der Probelauf beanstandet. Der Assistent hat drei Schritte:
  Blatt → Vorschau → Ergebnis.
- **Eine Dokument-Spalte ohne Gegenstück hält nicht mehr an.** Ohne Abgleich gibt es nichts, womit sie
  beantwortet werden könnte; sie wird als „nicht übertragen“ genannt. Gesperrt ist Weiter nur noch,
  wenn das Blatt gar nicht geschrieben werden kann.
- **Das Backend ist unverändert**: es schickt weiter jedes Blatt und nimmt Aliase und Schlüssel im
  Auftrag an. Blätter oder Zuordnung wieder anzubieten ist eine Änderung der Oberfläche.

## Vorschau zeilenweise (appended 2026-10-10)

Entschieden mit Martin am 2026-10-10. Der Kunde liest die Master nach Zeilen — die Zeile eines Wagens —,
nie nach Zellen; die Zellliste (Zelle | Schlüssel | Spalte | Vorher | Nachher) ist ersetzt.

- **Eine Zeile je geänderter Blattzeile, in allen Spalten des Blatts**, unter Spaltenbuchstabe und
  Überschrift, damit sie sich mit der offenen Datei in Excel vergleichen lässt. Geänderte Zellen sind
  markiert, eine neue Zeile ganz grün, eine geleerte ganz rot und durchgestrichen. Ein Klick auf eine
  geänderte Zeile klappt darunter die Zeile von VORHER auf, Spalte für Spalte — der alte Wert steht
  genau unter dem neuen. Eine Liste „Spalte: vorher → nachher“ neben der Zeile hieß: nach rechts
  scrollen, um die Zelle zu finden, und zurück an den Anfang, um zu lesen, was sie vorher war. Die
  Zeilenbeschriftung bleibt dafür links stehen (sticky). Eine neue Zeile hatte kein Vorher, eine
  geleerte zeigt es schon — beide klappen nicht auf. Jede Zelle bringt ihren Vorher-Wert dafür aus
  Rust mit (`RowCell.before`).
- **Geleerte Zeilen stehen in derselben Tabelle**, mit dem, was sie heute enthalten; die Zeile mit den
  Schlüsseln darüber bleibt als Warnung vor dem Schreiben.
- **Eine nicht geschriebene Spalte zeigt den Wert von VORHER** — das, was Excel heute zeigt. Formelspalten
  werden mit veraltetem Cache neu ausgegeben; der Wert von nachher wäre dort falsch.
- **Rust baut die Zeilen** (`export/diff.rs`, Backend for Frontend) und schickt ALLE, ohne Kappung: die
  Vorschau ist das, was geschrieben wird, und eine Zeile, die sich nicht öffnen lässt, wäre ungesehen
  freigegeben. Angular zeigt nur an und klappt auf; gerendert werden 100 Zeilen, weitere beim Scrollen
  (`ion-infinite-scroll` aus dem Speicher, kein virtuelles Scrollen — gezeigte Zeilen bleiben im DOM). Vorschau und Ergebnis nutzen dieselbe Komponente
  (`trains/master/ui/row-changes`).

## Dokumente archivieren (appended 2026-10-10)

Entschieden mit Martin am 2026-10-10. Die Dokumentliste (`/trains/documents`) wächst mit jeder
bereinigten Datei; was erledigt ist, soll aus dem Blick, ohne verloren zu gehen.

- **Archivieren blendet aus, es löscht nichts.** `Dokument.archiviertAm` ist ein Datum am Datensatz;
  Ordner `dokumente/<id>/`, Original, bereinigte Kopie und Import-Stand bleiben, wie sie sind. Ein
  Datensatz ohne das Feld (alles vor heute) gilt als nicht archiviert.
- **Rust entscheidet, was die Liste enthält.** `dokumente` und `counts.dokumente` lassen das Archiv
  weg, `archiv` und `counts.archiviert` enthalten nur es. Angular filtert nichts.
- **„Archivierte zeigen“ holt das Archiv** (`get_dokument_archiv`); jede archivierte Zeile bietet
  „Wiederherstellen“ und die Dateien, aber weder Import noch Master-Übertragung — dafür wird sie erst
  zurückgeholt.
- **„Alle archivieren“ nimmt alles, was die Liste zeigt**, importiert oder nicht, **ohne Rückfrage**:
  es ist mit „Wiederherstellen“ umkehrbar, und eine Rückfrage vor etwas Umkehrbarem ist ein Klick
  ohne Wert.
- **Ein archiviertes Dokument bleibt „vorhanden“.** Seine Bytes gehören der App weiter; dieselbe Datei
  noch einmal zu bereinigen würde ein Dokument doppelt ablegen. Die Zeile im Bereinigen sagt
  „Archiviert am …“, sonst sucht der Nutzer es in einer Liste, die es nicht mehr zeigt.
- **Der Master-Import lässt das Archiv stehen.** `clear_mirror` öffnet jedes Dokument wieder für den
  Import, das Archiv ist ein eigenes Feld und bleibt.
