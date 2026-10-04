# Footguns

Empirical failures that do **not** reproduce from a read of the source. Something derivable by reading the
code does not belong here — that is the entry criterion, and it is what keeps the file short.

Several of these also carry a banner in the file they bit. **The banner is the long argument; this list is
the index you need _before_ you open that file** — a trap you only learn about once you are already
editing the file it lives in has taught you nothing.

Settled questions are in [decisions.md](./decisions.md); blocked work in [state.md](./state.md).

## Sass and the Material theme

- **`mat.theme()`'s typography keys are `plain-family` and `brand-family`.** The short spelling
  `plain`/`brand` **compiles clean** — the mixin reads the map with no fallback and no diagnostic — and
  emits every `--mat-sys-*-font` **empty**. There is no error, no warning, and the page still renders,
  in the browser's default font. The only way it was found is grepping the built CSS for `-font:`. See
  `src/theme/_material.scss`.
- **`mat.theme-overrides()` silently drops a token it does not know** (`@if map.has-key`), rather than
  erroring. A token renamed upstream therefore reads as a component with a _bad value_, not as a mistake
  in the override map, and it will send you looking at the value first. Check the spelling against
  `md-sys-color` / `md-sys-shape` before assuming the value is wrong.
- **`ng generate @angular/material:m3-theme --directory=src/theme` writes
  `src/theme_theme-colors.scss`.** It concatenates the directory and the filename with no separator, so
  the "directory" is a prefix. The schematic's own prompt asks for a path _with a trailing slash_
  (`'src/app/styles/'`, in `schema.json`), which is the tell — and the mis-placed file is a sibling of
  `src/theme/`, so it does not look obviously wrong in a file tree.
- **The CLI wants kebab-case flags, not the camelCase names in the schema.** `schema.json` declares
  `primaryColor`, `tertiaryColor`, `includeHighContrast`; the command line takes `--primary-color`,
  `--tertiary-color`, `--include-high-contrast`. Passing the schema spelling does not error — the option
  is simply not applied, and the schematic falls back to prompting or to its default.

## Ionic components

Measured 2026-08-22 in Chrome against `@ionic/angular@8`, while reworking the expert page.

- **`ion-card` sets a MUTED text colour on everything it contains** — `color: var(--ion-color-step-550)`,
  measured as `rgb(115,115,115)` in the light palette. Plain markup inside a card therefore renders grey,
  including **the value the user typed into an `ion-input`**, which then looks like a placeholder or a
  disabled field. Nothing in this repo's stylesheets is doing it and no rule of ours can be searched for
  to find it. The fix is compositional, not a `color:` (which the styling rules forbid): put text in the
  components that carry the full-contrast colour with them — `ion-card-title`, `ion-item`,
  `ion-list-header`. An `ion-input` in an `ion-item` measures black; the same input directly in the card
  does not.
- **`ion-item` CLIPS an `fill="outline"` input's floating label.** The label is drawn ON the box's top
  border, the item's content box ends there, and the top half of the label is simply cut off — Ionic's own
  docs say not to put a filled input in an item. It is still the only way to get the full-contrast colour
  above, so the input needs block padding of its own inside the item (`.export-panel__field`,
  `.document-list__field-main`). Without it the label is unreadable and looks like a font-loading bug.
- **Ionic copies `aria-*` onto its inner native element ONCE**, in `componentWillLoad`. An
  `[attr.aria-label]` bound to a value the user edits is therefore correct until the first edit and stale
  afterwards: the box keeps answering to the name it had before. It is invisible on screen — only a
  screen reader or a Playwright `getByLabel` sees it, and a test written against that name passes while
  the app is wrong (that is how it was found: a pinned `test.fail()` in `filler.spec.ts` started
  reporting "expected to fail but passed"). Derive such a name from something that does not change —
  `document-list.component.ts` uses the field's ORIGIN — or use a real visible `label`, which is rendered
  reactively in the shadow DOM and has no such problem.

## Config that fails in the wrong direction

- **Sheriff has no path-ignore option.** Verified against `@softarc/sheriff-core@0.19.6`'s
  `UserSheriffConfig` / `Configuration` types: the only exclusion keys are `excludeRoot` (which concerns
  the implicit root project, not a path) and `ignoreFileExtensions`. This is why `src/_legacy` is
  **quarantined as its own `type:legacy` module** instead of being skipped the way eslint, stylelint and
  prettier skip it — the seal has to be built out of a dep rule. Do not go looking for the option again.
- **pnpm's `allowBuilds` errors, it does not warn.** A new dependency with a lifecycle script breaks
  **every** `pnpm run` — not just the install — until it is listed in `pnpm-workspace.yaml`. It has bitten
  this repo once already, via a transitive package pulled in by a new dev dependency, and the failure
  surfaced as scripts that had worked ten minutes earlier refusing to start. The deny-by-default is
  deliberate and CI runs without `--ignore-scripts` so the check cannot be hidden; just know what the
  error means.
- **A `.gitignore` entry with no leading slash matches every directory of that name, at every depth.**
  The bare `data` line predates this layout, where the architecture puts a `data/` in each domain
  (`src/app/@shared/data/`, `src/app/filler/data/`) — so the entire facade layer would have been silently
  untracked, with `git status` clean and nothing to notice it by until a clone came up missing. Now
  written `/data`, with the reason in the file.
- **`@typescript-eslint/naming-convention`'s `filter` selects _which names the rule applies to_, so it can
  only ever exempt.** A ban written as a `filter` regex reads exactly like a working rule and catches
  nothing. **`custom` with `match: false`** is the assertion form — that is what makes a pattern an error.
  This is why the `I`/`T`-prefix ban in `eslint.config.js` is spelled the way it is.

## Reading xlsx

- **An `&` in an INLINE string is truncated to the text after it.** A cell holding
  `Fa. Müller GmbH & Co. KG`, stored as `<is><t>Fa. Müller GmbH &amp; Co. KG</t></is>`, comes back
  from `grid::read` as **`Co. KG`** — the file is correct (openpyxl reads it back intact) and nothing
  errors. The **same string in `sharedStrings.xml` reads correctly**, which is why this hides: Excel
  itself writes shared strings, so an Excel-authored file never shows it. Files written by openpyxl,
  and by plenty of ERP and report exporters, use inline strings — and those are exactly the files a
  Werkstatt sends. Measured 2026-08-16 on macOS against `umya-spreadsheet`, with both variants built
  by hand to isolate it.

  It matters more here than the size of the bug suggests: German workshop names routinely end in
  `GmbH & Co. KG`, and a truncated name becomes the partner's `match_key`, its learnt alias and its
  displayed name. Two unrelated firms both ending `& Co. KG` would collapse onto one key.

  `docs/fixtures/problemfaelle_2026-03.xlsx` carries one deliberate row for it. Not yet fixed;
  fixing it means either an upstream `umya` change or reading the sheet XML directly, the same escape
  hatch `workbookPr/@date1904` already needs.

- **umya 3.0.1 PANICS while reading shared formulas with whole-column ranges.** A real 28-sheet
  customer workbook full of `VLOOKUP(A:A,Blatt!A:D,4,0)` died in
  `helper::formula::adjustment_formula_coordinate` — an `unwrap()` on the missing row of `A:A` — on
  every sheet and on both `read` and `lazy_read`. A panic is not an `AppError`: the command never
  answers and the window keeps spinning. 3.1.0 reads all 28 sheets (measured 2026-10-03, macOS). Every
  umya and lopdf read now runs inside `AppError::reading`, so the next parser bug is a German dialog.

- **A full `read` deserialises every sheet, whichever one you wanted.** The same workbook had two
  sheets filled with `0` down to row 1,048,576: 6.5 s and 2.6 GB for a full read, 0.4 s and 300 MB for
  `lazy_read` plus the one sheet. `grid::read` therefore reads one sheet, and so does the master
  refresh, which writes the workbook back out; the filler and the cleaner keep the full read.

- **There is no partial read of a sheet, so a header costs the whole sheet.** `read_sheet(index)`
  deserialises every cell, and the master's five fill-down sheets are up to 72 MB of XML each: one header
  scan of all 28 sheets cost 6.2 s and ~1.7 GB. `trains/master/prepare.rs` pays that once per file
  version and writes a trimmed read copy. Trimming it is its own trap: `Worksheet::cleanup()` walks up
  from the bottom and stops at the first VISUALLY non-empty row, which a `0` is, so it removes nothing
  there; `remove_row` shifts every formula reference in the book for each removal. `remove_cell` per
  cell plus `row_dimensions_to_hashmap_mut().retain(..)` removes the tail and nothing else. The trimmed
  sheets stay in memory until the write, so the pass peaks near 2 GB rather than one sheet's worth.

- **`Workbook::sheet_collection_mut()` deserialises EVERY sheet before it answers.** It looks like a
  plain accessor; it calls `read_sheet_collection()`. On the 28-sheet master, reaching one sheet through
  it cost 4 s, made the write 7 s instead of 1 s — a deserialised sheet is re-serialised rather than
  copied raw — and lost the byte-identical copy of every untouched sheet. `sheet_mut(index)`
  deserialises that one sheet. `grid::heads` had the same call and, despite its header, held every
  sheet of a scanned workbook at once; both now use `sheet_mut`. `sheet_collection_no_check()` is the
  read-only accessor that does not deserialise.

## Writing a workbook back

Measured 2026-10-03 on the real master (20 MB, 28 sheets) with umya 3.1.0, `lazy_read` plus one
deserialised sheet, then opened in Excel for Mac: no repair prompt, the changed cell visible on the
dashboard through its VLOOKUP. 1.9 s and ~500 MB, where a full read alone took 2.6 GB.

- **Undeserialised sheets are written back byte for byte** (`raw_data_of_worksheet`), and the shared
  strings stay a superset with the old indices in place, so those raw sheets still point at the right
  text. `cellXfs` keeps its count and order, so their style indices hold too.
- **What is lost or changed anyway:** `customXml/*` (SharePoint content-type metadata) is dropped;
  `calcChain.xml` is dropped and `calcPr` written as `calcId="122211"`, older than Excel's, which is
  what makes Excel recalculate everything on open — wanted. A `numFmt` that REDEFINES a built-in id is
  dropped: the master redefined id 19 as `dd/mm/yyyy`, and Excel's own 19 is a time, so any cell using
  it would show `0:00:00` instead of a date (no cell in that file used it). A font name containing
  `&quot;` is escaped a second time. A dxf's `<color auto="1"/>` is dropped, which is the default anyway.
- **A written formula cell has no cached value of its own** until Excel recalculates; a cell cloned
  from another row carries that row's stale result. Harmless with the `calcId` above, misleading to
  anything that reads the copy without Excel.

