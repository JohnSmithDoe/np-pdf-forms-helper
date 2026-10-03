### Version 2.0.0

- New desktop shell: Tauri 2 with a Rust backend replaces Electron
  - No external programs needed anymore (pdftk is gone), PDF forms are filled directly
  - German field names and umlauts in PDF fields are read and written correctly
  - Windows installer (NSIS) replaces the zip
- New user interface (Angular 21, Ionic)
  - Two guided wizards: set up documents, fill documents
  - The all-in-one page stays available as expert mode
  - Info page with version, data folders and the facts IT approval asks about
- New: Schattensystem for freight wagon keepers
  - Bereinigen: drop or pick spreadsheets, recognise them by template, review every change, file original and cleaned copy
  - Import: walk one filed document by Partner, Wagen, Radsätze and Instandhaltungen, nothing written before the final confirmation
  - Lists for Wagen, Partner, Radsätze (with their fitting history), Instandhaltungen and Dokumente
  - Setting for how a Wagennummer is written

### Version 1.1.9

- Bugfixes - Profiles and remapping documents

### Version 1.1.8

- Bugfix - Re-Adding Documents
  - Mapped Field Identifiers got mixed up

### Version 1.1.7

- To make redistribution and redeploy easier we added
  - Feature - Import folder (Import a whole folder full of documents)
  - Feature - Reset app (Reset the database)
  - So now all documents can be removed or restored in one action (profiles not)
  - Improved error handling

### Version 1.1.6

- Reworked the export profiles (bugfixes)
  - No export flags in backend anymore, profiles only
  - Larger profile box in the UI for longer names
  - "No Profile" Selection
  - Updates on Document changes (fields get removed...)
- Feature Automapping added (You can not let the app map the form fields for you)
- Clean-Up Backend
  - Refactored the database, pdf, xls and resource service out of the np-assistant

### Version 1.1.5

- First released version
