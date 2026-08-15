# Introduction

This app manages a small amount of documents that need to be filled and bundled.
It helps to fill documents like pdf forms and xlsx files with data.
- You can setup document links to original files 
- You can map fields or cells to custom field names
- Fields with the same name get the same value
- You can create a copy of all documents with the filled in values

Depends on pdftk to be available on your system.
It is only roughly tested on Windows and has a german UI.

Thankfully based on (electron v17 / angular v13 template): [maximegris/angular-electron](https://github.com/maximegris/angular-electron)

# ISSUES

### ISSUE #1: How to get the encoding of a file instead of assuming utf8 || win1252
- To get the encoding of a file we need to "guess" from the header and encoded chars e.g. [jschardet](https://github.com/aadsm/jschardet)
- This is too much for now so we go with win1252 and utf8 only

### ISSUE #2: To add npm packages to the main app we have to add them to both package.jsons
- Maybe its not a good idea to keep the main in the src folder
- Do peerDependencies get installed ... probably not
  - If not just copy the peerDependencies to the normal dependencies

# IMPROVEMENT IDEAS
* typing could be better

# Config file

You can use a config file ".npconfig" in the "exe's" path that contains an IAppConfig json

```
{
  PDFTK_EXE?: string;
  ENCODING?: string;
  DATA_PATH?: string;
  TMP_PATH?: string;
  CACHE_PATH?: string;
  OUTPUT_PATH?: string;
  DB_FILE?: string;
  PROFILE_FILE?: string;
}
```

# Environment variables

You can override each used resource with the following env variables:

| Variable      | Description                             | Default               |
|---------------|-----------------------------------------|-----------------------|
| APP_PDFTK_EXE | Absolute path to the pdftk executable   | ./pdftk/bin/pdftk.exe |
| APP_DATA      | Absolute path to the data folder        | ./data                |
| APP_TEMP      | Absolute path to the temp folder        | ./data/tmp            |
| APP_CACHE     | Absolute path to the cache folder       | ./data/cache          |
| APP_OUTPUT    | Absolute path to the output folder      | ./data/out            |
| APP_CONFIG    | Absolute filename of the config file    | ./data/config.json    |
| APP_ENCODING  | Override the file encoding used for r/w | ./data/config.json    |

# Project structure

| Folder       | Description                                      |
|--------------|--------------------------------------------------|
| src/main     | Electron main process folder (NodeJS)            |
| src/bridge   | Electron and angular shared files (TypeScript)   |
| src/renderer | Electron renderer process folder (Web / Angular) |

# Included Commands

The package manager is **pnpm**. `pnpm install` also installs the runtime dependencies in
`src/main` (the folder electron-builder packages) via the `postinstall` hook.

| Command                        | Description                                                                          |
|--------------------------------|--------------------------------------------------------------------------------------|
| `pnpm start`                   | Starts the main and the renderer in electron / hot reload of the renderer enabled    |
| `pnpm run ng:serve`            | Starts only the renderer in the web browser                                          |
| `pnpm run electron:serve`      | Starts only the main in electron                                                     |
| `pnpm run build:all:prod`      | Run the build process for main and renderer                                          |
| `pnpm run electron:local`      | Run the build on you local machine                                                   |
| `pnpm run electron:unpacked`   | Builds your application to an unpacked executable                                    |
| `pnpm run electron:packaged`   | Builds your application and creates an app consumable based on your operating system |
| `pnpm run install:app`         | Reinstalls only the packaged runtime dependencies in `src/main`                      |

