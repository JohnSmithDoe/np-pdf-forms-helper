### Version 2.1.0

- MVP: clean a sender's sheet, then write it into the customer's own master file
  - „Master-Datei wählen“ only remembers where the file lies; nothing is copied or cleaned
  - „In Master übertragen“ on every document: one sheet per document, the column mapping comes from the template
  - A backup goes to `Sicherungen/` beside the master before every write; a file open in Excel or changed since the preview is refused
  - Only the edited cells are written; every other part of the workbook stays byte for byte as it was, and the file is verified before it replaces the original
  - Preview by row: changed cells marked, new and emptied rows marked whole, a click shows the old row underneath
  - Rows the document lacks can be emptied in place, per sheet, off by default
  - A transferred document is archived
- New: „Archivieren“ in the Dokumente list hides a document without deleting it
- Schattensystem
  - The import writes only into the Schattensystem; the master import empties it and rebuilds it from the master's sheets
  - Wagen, Radsätze and Partner as cards with a detail page; every named Wagen, Radsatz or Werkstatt links to its own
  - Wagen-Zustand: Telematik, Schadensmeldungen, Werkstattaufträge and Prüfungen, plus a Telematik list with the longest silent first
  - Excel-like column filter and Farben on Wagen and Radsätze
- The app is scaled back to Bereinigen, Dokumente and the master file; the rest sits behind a switch
- Fixed: the installed 2.0.1 rendered unstyled
- Fixed: `ECHO_Eingänge` is recognised as Werkstattaufträge although it has no Bestelldatum column

### Version 2.0.1

- New: the customer's master file (Master-Datei)
  - Pick it once, it is copied into the app and cleaned: empty rows below the data cut, numbers and dates stored as text retyped where unambiguous, edge whitespace trimmed — formulas inside the data and row order are never touched
  - Kept in versions; the current one is pinned on top of the Dokumente list
- New: „Master aktualisieren“ on every document, once a master file exists
  - Wizard: choose sheets, match columns, preview every changed cell, approve, summary
  - Always incremental: rows are matched by key, existing rows updated, nothing deleted; new rows only where switched on
  - Every sheet the document's data would change is pre-selected
  - The result is saved as the next version of the master file
- „Importieren“ and the status chip on the Dokumente list are hidden for now

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
