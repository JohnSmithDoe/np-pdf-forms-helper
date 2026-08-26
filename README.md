# npDokumentenhilfe

A desktop app for filling a small set of forms that always want the same data. Link the original PDF and
XLSX documents once, map their form fields and cells to your own field names, and export a filled copy of
every document from a single set of inputs.

- link documents to their original files
- map PDF form fields and spreadsheet cells to custom field names
- fields sharing a name across documents receive the same value
- export filled copies of all documents at once, optionally through a saved profile

**The UI and the end-user documentation are German**, and the app targets **Windows** (only roughly tested
there). For users: [Handbuch](docs/handbuch.md) · [Installationsanleitung](docs/installations-anleitung.md).

## Requirements

- Node `>=22.18` (pinned in `.nvmrc`, enforced by `scripts/check-node.mjs` on install) and **pnpm**, via
  corepack.
- A **Rust** toolchain, for the Tauri backend.

No external binaries. The app used to shell out to **pdftk** for all PDF work and quit on startup when it
was missing; that dependency is gone with the Electron process.

## The migration, and what is left of it

The frontend is Angular 21 (standalone, zoneless) in the np-commlink layout, and the backend is Rust
behind Tauri 2. Both shells no longer exist side by side — Electron has been removed outright.

**The port is functionally complete.** PDF, XLSX and resource documents all work end to end; PDF form
filling is hand-written against `lopdf` in `src-tauri/src/doc/pdf/`. What is still open is coverage:
**nothing has ever run on Windows and `pnpm run tauri:build` has never run at all** — see
[docs/state.md](docs/state.md).

Four documents carry what the code cannot say. Read the one that matches your question:

| Document                               | Holds                                                        |
| -------------------------------------- | ------------------------------------------------------------ |
| [CLAUDE.md](CLAUDE.md)                 | how the app is built and what not to break — start here      |
| [docs/decisions.md](docs/decisions.md) | settled questions, so they are not re-flagged as work        |
| [docs/footguns.md](docs/footguns.md)   | failures that do not reproduce from a read of the source     |
| [docs/state.md](docs/state.md)         | unverified work and known limitations — check before proposing any |

Everything else lives next to the code it governs: module boundaries in `sheriff.config.ts`, each lint
rule's reason in its own comment, the browser floor in `.browserslistrc`, CI's shape in
`.github/workflows/ci.yml`.

## Layout

| Path         | What                                                                        |
| ------------ | --------------------------------------------------------------------------- |
| `src/`       | Angular 21 renderer → `dist/renderer`; one domain (`filler`) plus `@shared` |
| `src-tauri/` | Rust backend and the Tauri shell                                            |
| `e2e/`       | Playwright specs, driven against a faked Tauri transport                    |
| `docs/`      | German end-user docs, the three engineering documents above, and the PDF/XLSX fixtures |

## Commands

The package manager is **pnpm**.

| Command                      | Description                                                |
| ---------------------------- | ---------------------------------------------------------- |
| `pnpm start`                 | Angular dev server only (`ng serve`), no desktop shell     |
| `pnpm run tauri:dev`         | The real app: Rust backend plus the dev server             |
| `pnpm run tauri:build`       | Packaged application                                       |
| `pnpm run build`             | Angular production build → `dist/renderer`                 |
| `pnpm run lint`              | eslint, then stylelint                                     |
| `pnpm run typecheck`         | `ngc` over all of `src/**`, including files not yet routed |
| `pnpm run verify`            | Sheriff — module boundaries                                |
| `pnpm run format` / `:check` | prettier                                                   |
| `pnpm run rust:check`        | `cargo check`                                              |
| `pnpm run rust:lint`         | `cargo clippy`                                             |
| `pnpm run e2e`               | Playwright, against the faked transport                    |

`lefthook` runs prettier, eslint and stylelint pre-commit, and lint, Sheriff and the type-check pre-push.
The Playwright suite is not part of CI — see the comment in `.github/workflows/ci.yml`.

## Config file

A `.npconfig` JSON file is read at startup from the process working directory. Every key is optional and
takes precedence over the matching environment variable.

```json
{
  "DATA_PATH": "./data",
  "CACHE_PATH": "./data/cache",
  "OUTPUT_PATH": "./data/out",
  "DB_FILE": "./data/data.db",
  "PROFILE_FILE": "./data/profiles.db"
}
```

## Environment variables

Resolution order per key is `.npconfig` → environment variable → default. Missing folders are created on
startup.

| Variable            | `.npconfig` key | Description                          | Default              |
| ------------------- | --------------- | ------------------------------------ | -------------------- |
| `APP_DATA`          | `DATA_PATH`     | Data folder                          | `./data`             |
| `APP_CACHE`         | `CACHE_PATH`    | Cache folder                         | `<data>/cache`       |
| `APP_OUTPUT`        | `OUTPUT_PATH`   | Output folder for exported documents | `<data>/out`         |
| `APP_DB_FILE`       | `DB_FILE`       | Document database file               | `<data>/data.db`     |
| `APP_PROFILE_FILE`  | `PROFILE_FILE`  | Profile database file                | `<data>/profiles.db` |

The base directory is the process **working** directory, not the executable's, so existing installs that
keep `.npconfig` and `data/` beside the binary and launch it from there keep working.

`PDFTK_EXE`, `ENCODING` and `TMP_PATH` no longer exist, and `DB_FILE`/`PROFILE_FILE` no longer share one
variable — both databases used to fall back to `APP_CONFIG`, which pointed them at the same file. Unknown
`.npconfig` keys are ignored, so a config from an older install still loads; the `data/tmp` folder it
names is simply no longer created or used.

## Licence and credit

GPL v2 — see [LICENSE.txt](LICENSE.txt). Originally scaffolded from
[maximegris/angular-electron](https://github.com/maximegris/angular-electron) (the Electron 17 / Angular 13
template); nothing of that template survives — neither its structure, after the Angular 21 migration, nor
Electron itself.
