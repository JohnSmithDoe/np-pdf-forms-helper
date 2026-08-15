# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

"npAusfüllhilfe" — an Electron + Angular desktop app that links original PDF/XLSX documents, maps
their form fields / cells to user-chosen field names, and produces filled copies. Fields sharing a
mapped name across documents receive the same value on export.

**The UI, all user-facing strings, and `docs/` are German.** Keep new user-facing text German.

Targets **Windows** (only roughly tested there) and depends on an external **pdftk** executable for
all PDF work — the app **quits on startup** if `PDFTK_EXE` does not resolve, so it cannot run on a
machine without pdftk.

**Mid-migration.** The frontend is Angular 21 (standalone, zoneless, Material 21) in the np-commlink
layout. The Electron main process is scheduled for replacement by a Rust/Tauri 2 backend — see
`docs/` and treat everything under `electron/` as transitional code with a delete date.

## Commands

| Command | Purpose |
|---|---|
| `pnpm start` | Angular dev server only (`ng serve`), no Electron |
| `pnpm run electron:start` | Dev: `ng serve` on :4200 + Electron with `--serve` |
| `pnpm run electron:serve:debug` | Electron with devtools and `--npdebug` (dumps + writes resolved config) |
| `pnpm run build` | Angular production build → `dist/renderer` |
| `pnpm run build:all:prod` | `tsc -p tsconfig.main.json` + `ng build -c production` |
| `pnpm run electron:local` | Prod build, then run Electron from `dist/` |
| `pnpm run electron:packaged` | Prod build + electron-builder installer (NSIS on Windows) |
| `pnpm run lint` | eslint + stylelint |
| `pnpm run verify` | **Sheriff** — module boundaries (`sheriff verify src/main.ts`) |
| `pnpm run format` / `format:check` | prettier over `src/**` and `electron/**` |
| `pnpm run install:app` | Reinstall only `electron/`'s runtime deps (also run by `postinstall`) |

There are **no tests** and that is deliberate — do not add a test runner. Verification is manual
spot-checking. `lefthook` runs prettier/eslint/stylelint pre-commit and lint+sheriff pre-push.

## Architecture

Two source roots, two builds:

| Folder | Process | Built by |
|---|---|---|
| `src/` | Angular 21 renderer | `@angular/build:application` → `dist/renderer` |
| `electron/` | Electron main (Node), incl. `electron/bridge/` | `tsc -p tsconfig.main.json` → `dist/main` |
| `src/_legacy/` | Quarantined Angular 13 code, ported out incrementally | nothing — excluded everywhere |

`src/_legacy/` is excluded from `tsconfig`, eslint (`globalIgnores`), stylelint, prettier, and is
tagged `type:legacy` in Sheriff. Porting means moving one component out of it at a time. When it is
empty, delete the `type:legacy` entries in `sheriff.config.ts` and `eslint.config.js`.

### The Angular app

**One domain (`filler`) plus `@shared`**, on the np-commlink layer axis. One domain is deliberate:
documents, field mapping, profiles and export are a single workflow over a single data model, so a
second sealed domain would earn nothing. `sheriff.config.ts` names no domain — adding one costs a
folder, not a config change.

```
src/app/@shared/{model,ui,util,data}
src/app/filler/{routes,feature,smart-ui,ui,data,util,model}
```

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

`src/app/@shared/model/` and `electron/bridge/shared.model.ts` describe the **same JSON shape** under
different TypeScript names — `MappedDocument` on the Angular side, `IMappedDocument` on the Electron
side. That is not drift to be fixed: it is the hand-written mirror the Rust backend will need, and
the `I`-prefix ban is switched off for `electron/**` for exactly this reason. **Change one, change
the other**, and keep the JSON shape identical.

`AppChannel` is a frozen const object + union rather than an `enum` (an enum is not erasable syntax);
all 13 channel strings are byte-identical to the Electron side.

### IPC: one request → one broadcast

`ApiController` (`electron/api.ts`) registers an `ipcMain.handle` for every channel via a typed map.
Every handler returns an `IClientData`; the controller then **pushes** it back on `CLIENT_UPDATE` (or
the error message, split on `||`, on `CLIENT_ERROR`) — the `invoke` promise itself resolves to
nothing useful. This is being redesigned to plain request/response when the renderer is ported.

`nodeIntegration: true` / `contextIsolation: false`; there is no preload script.

### Main process

`NpAssistant` (`electron/np-assistant.ts`) is the orchestrator: it owns the API controller, the
database, and the three document services, and contains the top-level use cases.

`NPService` (`electron/services/np-services.ts`) is the abstract base; implementations are picked by
file extension (`getFileInfo` in `file.utils.ts`):

- **pdf** — `PdfService`. Shells out to pdftk (`generate_fdf` / `fill_form`) and parses the FDF text
  itself in `pdf.utils.ts`, through `iconv-lite` with `config.ENCODING` (win1252 on Windows, utf8
  elsewhere). On add it writes a *preview* PDF with every field filled with its own field path.
  **`updateFDFValuePaths` (`pdf.utils.ts:111-119`) joins ancestor `/T` values with NO separator**, so
  stored `fields[].path` is not a valid PDF fully-qualified name. Every existing `data.db` depends on
  that exact string.
- **xlsx** — `XlsService` (ExcelJS). `mappedName` doubles as the address, format `$Sheetname.A1`
  (built at `field-dialog.component.ts:55`, parsed by slicing on the first `.`). Because the address
  *is* the mapped name, two xlsx cells can never share a name — the headline "same name, same value"
  feature is PDF-only. `remapDocument` throws "Not implemented yet".
- **resource** — `ResourceService`, the fallback for any other extension: plain copy, no fields.

All services work on a **temp copy** and compare `mtime` on export, warning if the original changed.

### Persistence

`NpDatabase` — two JSON files (`data.db`, `profiles.db`) written synchronously on every mutation.
`version` is 1 and `migrateDatabase()` is a stub — bump both together if the shape changes.

Profiles reference documents and mapped fields by id, so document changes must cascade:
`updateProfilesOnDocumentChange` / `…OnDocumentRemove` prune dangling `fieldIds`. This has been the
source of several past bugfixes — touch document add/remove/remap and you must re-check profiles.

### Configuration

Resolution order per key, in `np-assistant.ts` (top-level, at import time): `.npconfig` JSON →
`APP_*` env var → default under `./data`. Folders are created on startup.

Gotcha: `DB_FILE` and `PROFILE_FILE` both fall back to the *same* env var `APP_CONFIG`
(`np-assistant.ts:40-41`), so setting it points both databases at one file.

### Styling

Two tiers. `src/theme/_theme-colors.scss` is **generated** — `ng generate @angular/material:m3-theme`
from `#3f51b5` / `#ff4081`, the indigo-pink prebuilt theme this app shipped on Angular 13 — and is
excluded from stylelint and prettier because regenerating it would revert any edits.

`_material.scss` is the only file naming Material: it feeds those palettes to `mat.theme()`, which
owns colour outright and emits it through `light-dark()`. `_tokens.scss` aliases `--np-*` **from**
`--mat-sys-*`, never the reverse — components read `--np-*` and never name Material. Only shape is
overridden. Dark is `:root[data-theme='dark']`, which sets `color-scheme: dark` and nothing else,
because every M3 role flips itself.

Two traps, both cost real time: `mat.theme`'s typography keys are `plain-family`/`brand-family`, and
the short spelling compiles clean while emitting every `--mat-sys-*-font` **empty**;
`mat.theme-overrides()` **silently drops** an unknown token rather than erroring.

Icons: the app ships **only** `material-icons/iconfont/sharp.css` (one woff2), registered as the
default font set in `AppComponent`, so no `<mat-icon>` needs a `fontSet`.

## Conventions

**Node dependencies must be listed twice.** `electron/package.json` is the packaged app manifest
(electron-builder's `directories.app`), so anything the main process needs (`uuid`, `iconv-lite`,
`exceljs`) belongs in both it and the root `package.json`.

**Two independent pnpm installs.** `electron/` has its own `pnpm-lock.yaml` and is installed
standalone by `install:app` (`--ignore-workspace --node-linker=hoisted`) — it must be a *flat*
`node_modules` because electron-builder copies that folder into the package and would otherwise drag
in dangling symlinks into the pnpm store. The root install uses pnpm's default symlinked layout.
`pnpm-workspace.yaml` exists only to hold `allowBuilds`, pnpm's deny-by-default allowlist for
dependency lifecycle scripts — pnpm **errors** rather than warns on an unlisted one, so a new
dependency with a postinstall breaks every `pnpm run` until it is added.

**`.browserslistrc` and `electron` move together.** Material's `mat.theme()` emits `light-dark()`,
which no build step can polyfill and which needs Chromium 123 — so the Electron floor is 30, and the
browserslist line is written as an Electron version to keep the two adjacent.

**Version bumps** touch three places: root `package.json`, `APP_VERSION` in
`electron/bridge/shared.model.ts` (shown in the welcome dialog), and `CHANGELOG.md`.
`electron/package.json` has drifted behind. Release commits follow `release(vX.Y.Z): …`.

**Errors are user-facing German strings.** `throw new Error('…')` in the main process is surfaced
verbatim in a dialog; join multiple lines with `||`.
