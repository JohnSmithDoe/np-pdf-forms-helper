# State — blocked, one-way doors, waiting on a human

**Check before proposing work.** Nothing here is merely undone: each item needs a decision, a person in
front of a screen, or a piece of the migration that does not exist yet. Settled questions are in
[decisions.md](./decisions.md); traps in [footguns.md](./footguns.md).

## Blocked — needs a human in front of a screen

- **Nothing has ever run on Windows, and `tauri:build` has never run at all.** The NSIS installer, the
  WebView2 rendering, and the `./data` layout beside the executable are all unverified — everything so far
  is `tauri:dev` on macOS against WKWebView. The installer would also be **unsigned**, so SmartScreen warns
  on first run; signing needs a certificate configured under `bundle.windows`.
- **`withHashLocation()` is reasoned but untested against a packaged build.** The argument is in
  `src/app/app.providers.ts`: the packaged renderer is served to the webview over a custom protocol, where
  a path-based deep link is resolved by the asset handler rather than by a router-aware server. `ng serve`
  works either way, which is exactly how this breaks only in the build nobody runs during development. Same
  blocker as above — it needs one `pnpm run tauri:build`.

## In flight

- **The backend has unit tests as of 2026-08-16** — `pnpm run rust:test`, a `#[cfg(test)] mod tests` per
  module, driven by a hand-written `TempDir` in `src/testing.rs`. They cover `config`'s precedence ladder,
  `db`'s profile cascade, the AcroForm byte rules, both fill loops, the cell-address parser, and the
  import/export report policy. Three decisions were split out of `#[tauri::command]`s to be reachable at all:
  `AppConfig::resolve_all` (base folder and env lookup passed in), `commands::folder_to_open`, and
  `export::Run::report`.

- **What the unit tests still cannot say is that the app RUNS.** They call the same functions the commands
  call, not the commands; nothing constructs a `WebviewWindow`. The PDF path has been exercised end to end in
  `pnpm run tauri:dev` on macOS against `docs/formular_beispiel.pdf` — link, auto-map, preview, export,
  re-link. The **XLSX and resource services have still never run through the app**, and Playwright drives a
  FAKED transport, so it proves the renderer's half of the contract and none of Rust's.
  `docs/file_example_XLSX_1000.xlsx` is the fixture for that pass.

- **`umya-spreadsheet` itself is verified, by a throwaway `#[cfg(test)]` probe (2026-08-16, macOS).** Read of
  the 1001-row fixture 129 ms, write 88 ms, and a read → write → read round-trip preserved both the used range
  and the cell count. That answers "does the crate work at all", not "does `doc/xlsx` work" — the service still
  has to run through the app. Four measured facts came out of it, and three of them are traps:

  - **`formatted_value()` does NOT apply the cell's number format.** A cell holding `45000` under format code
    `DD.MM.YYYY` answers `"45000"`, not `"15.03.2023"`. So it is _not_ "what the user sees in Excel", and
    anything wanting the rendered date has to render it itself from the serial.
  - **A styled but EMPTY cell inflates the used range.** A sheet with three real cells plus one empty
    formatted cell at (20, 500) reports `highest_column_and_row() == (20, 500)`. Any reader must trim trailing
    empty rows and columns _after_ asking, and cap the scan — unguarded, a file that looks like 40 rows in
    Excel is an allocation of a different order.
  - **The number format is per cell, and real files are inconsistent about it.** In the fixture's `Date`
    column, row 2 carries `MM/DD/YY` while rows 3 and 4 carry `GENERAL` — same column, same kind of value. A
    per-cell "is this a date" test therefore disagrees with itself down a single column; the question is only
    answerable for the column as a whole.
  - **`value()` on a numeric cell always stringifies with a `.` decimal** (`1234.56`), regardless of the
    file's origin or the reader's locale. String-parsing a cell that `value_number()` already answered is
    always a mistake, and under German inference a wrong one.

  **`workbookPr/@date1904` is not exposed.** umya's workbook reader parses `workbookProtection` and nothing
  about the date system, so a Mac-authored 1904 workbook cannot be detected through the crate — every date in
  it is 4 years and a day early, and nothing errors. Reading the flag means opening the xlsx zip directly.

- **Radsätze are modelled but have never met a real file.** The Einbau history assumes a sender puts
  an install or removal date on the row that moves a Radsatz; if they instead send a separate fitting
  list, or record only a position change, that shape needs looking at.
  Identity is now sender-scoped rather than an exact match (see `resolve/radsatz.rs`), so an
  inconsistent speller produces a QUESTION rather than a silent duplicate — but two open questions
  ride on real data. **What the sender actually is** when a file arrives with no template and no
  resolvable Werkstatt: today that is `None`, and a `None` sender never gets a `Known` off a bare
  number, so such a file asks about every Radsatz on every import. And **whether leading zeros are the
  only formatting variance worth offering** — a sender who writes `RS 4711` one month and `4711` the
  next gets `New`, not a suggestion, because the prefix is not a formatting difference the code may
  assume away. Both need one real workshop file before being guessed at.
- **`Radsatz.wellennummer` is stored and matches nothing.** A column maps onto it and the value
  survives, but no rule reads it until a real sender file proves the format — see `decisions.md`.
- **The trains workflow is complete in code and has never run through the app.** `cargo test` covers
  the parsers, the reader seam, the store, staging, commit and both export halves (359 tests), and
  Playwright covers the screens against a faked transport — but no XLSX has gone in one end of the
  real binary and come out the other. The first `pnpm run tauri:dev` pass should: import a real
  workshop file; check one column of each type against the source in Excel cell by cell; save a
  template and re-import a second file from the same sender; commit; export a document into the
  master and confirm a hand-kept column travelled with its key; and corrupt one cell to confirm the row is reported
  rather than the run aborting.
- **The master write path has run against the real master only from a test.** The former one-click
  refresh pasted `Telematik`, `aktuelleNodepit` and `ECHO_Eingänge` of the real workbook straight from
  the three sender files (1.9 s, only those three sheets changed, every dashboard key still found), and
  Excel opened umya's write cleanly. Its replacement, the EXPORT WIZARD (2026-10-04, `master/export/`,
  `/trains/master/export/*`), runs the same `paste` and is covered by `cargo test` and Playwright
  only: it has never run against the real master. Worth measuring there first: a dry run deserialises
  every ticked sheet of the ORIGINAL on every answer, so a sheet filled to row 1,048,576 makes each
  preview cost what the read copy was built to avoid — and a sheet that size is refused by the grid's
  row limit in the first place. Also unrun: the pages in `tauri:dev`. Every sheet recognised by a Wagen
  key column is bound as the overview kind; the wizard no longer blocks those, it only refuses to
  SUGGEST one without a remembered template and warns on it.
- **The master IMPORT (Phase 1: Wagen, Halter, Radsatz, Einbau) has run against the real master only
  from a temporary test**, 2026-10-04, through `mirror::start` / `stage_sheet` / `commit` with every
  group answered „neu anlegen“: dashboard, fitting list and stock in 3.5 s, peak ~750 MB for the test
  process (sheet view included); 405 Wagen, 1,207 Einbauten with position, 13 Einbau date conflicts,
  533 Radsätze only the stock knows (open, without position), 0 rejected rows. The ~750 MB is worth
  watching on the Citrix desktops. What has not run: the page and the walk in `tauri:dev`, and any
  phase after the first — see the plan and `decisions.md`, „Der Master wird gespiegelt“.
- **The master's read copy has run against the real master only from a temporary test**, 2026-10-04:
  7.9 s and a ~2 GB peak to write it (935 KB), then every one of the 20 bound sheets staged from it in
  14–140 ms and a sheet view in 31 ms; all 28 sheets read cell for cell the same as from the original
  (nodepit and RSanKundSgemeldet were cut at rows 175 and 191). The ~2 GB is the one cost worth watching
  on Citrix — it is paid on the first import or sheet view after the customer saves the file.

## Known limitations — not work, and not fixable here

- **Dynamic XFA forms cannot be filled, by this app or by the one it replaces.** A dynamic XFA document is
  rendered from its XFA stream and its AcroForm layer is ignored, so a correct `/V` write still displays
  blank. `doc/pdf/mod.rs::warn_if_dynamic_xfa` detects the case on the catalog's `/NeedsRendering` flag and puts
  three German lines in the export report; the export still runs, because the file is correct even where the
  viewer will not show it. **Static** XFA forms carry `/XFA` too and fill perfectly well — which is why the
  detection keys on `/NeedsRendering` and not on the presence of `/XFA`. Making dynamic forms work means
  generating appearance streams, a different design and roughly 10–16 hours; nobody has asked for it.
