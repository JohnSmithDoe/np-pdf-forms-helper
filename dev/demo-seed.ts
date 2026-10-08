// ─── why ────────────────────────────────────────────────────────
// The data `pnpm run start:mock` boots with — enough that no part of the UI
// sits behind an empty state. Dev-only content, but typed against the e2e
// fake's mirror of `src-tauri/src/model.rs` and `trains/model.rs`, so it cannot
// drift from the wire.
//
// Two documents share mapped names on purpose: one typed value filling both is
// the app's central idea and only shows up when a name repeats.
//
// One cell per SHEET, because for xlsx the mapped `origId` IS the sheet id — a
// second cell on one sheet reproduces the defects pinned in `filler.spec.ts`.
//
// `nextPickedDocument` stands in for the native picker, which no browser can
// drive. Counted, not random, so the same click always gives the same name.
//
// EVERY FIRM IN HERE IS INVENTED, and deliberately a pun — Wagenmut, Achsentreu,
// Schienenbein, Dreh & Gestell, Rundlauf, Nietenzähler. No real company name
// belongs in this repo, in the seed or anywhere else, and a name that reads as a
// joke is one nobody can mistake for a customer.
//
// THE TRAINS HALF IS SEEDED FOR THE HARD CASES, not for volume. Every screen of
// the Schattensystem has one row that is the interesting one:
//   • a Wagennummer whose CHECK DIGIT is right (they are computed, not typed) and
//     one whose Halter and Eigentümer are different parties, which is the
//     distinction the domain turns on
//   • a partner in TWO roles (a lessor that keeps wagons too), so the dashboard
//     counting it under both is visible rather than theoretical
//   • a Radsatz whose alias belongs to ONE sender — an alias means "this sender
//     calls it this", which is why `aliases` carries a `partnerId`
//   • a CLOSED Einbau beside the open ones, because "ausgebaut" is a state the
//     list has to render and no import produces on its own
//   • an Instandhaltung with no `datum` and one against a Radsatz rather than a
//     Wagen, the two shapes that are easy to forget exist
//
// `demoScan()` is one file of every kind the hub has to show: recognised,
// recognised twice (so the select starts empty), unknown, already owned, and
// not a workbook. Two `dokumente` are seeded, one of them imported, so the
// document list shows both states; `demoDocument()` is the Mai file as the
// import walk stages it, with every resolution the walk renders.
// `demoClean()` is a review with all three tiers — a Fehler per field kind that
// can have one, a Deutung of each of the three readings, several format groups
// — because a clean file shows none of the review screen.
//
// `demoStaging()` builds a FRESH staging per call, and that is not tidiness:
// `discard_import` sets it to null, so a shared constant would make the second
// file of a session answer with nothing. It covers all four row states plus a
// Fehler and a Warnung, because the review screen's chips, its selects and its
// "nur fehlerfreie Zeilen" default cannot be seen with a clean file.
//
// THE MASTER is bound three ways — a Wagen list and a Radsatz list with a kind,
// and an export-only sheet without one, edited by hand so „von Hand angepasst“
// shows and „Standardzuordnung“ visibly undoes it (`masterDefaults` makes it
// view-only again) — and its import run is left HALF done,
// so the „unvollständig“ banners show. The sheet views are written out by hand
// like `demoDocument()`'s groups: building them is `sheet_view.rs`'s, and a
// TypeScript projection here would be a second one. The Werkstattliste has no
// view on purpose, so the fake's headers-only answer — every column „nicht im
// Schattensystem“, no rows — has a sheet to show on. `demoMasterStaging()` is
// the Mai walk with one Einbau conflict, the card no other file produces.
//
// THE MASTER EXPORT of a tpl-bremen document suggests the Werkstattliste,
// with one column to answer (Werkstatt → Notiz or nicht übertragen) and a
// handful of changed cells; „Radsätze aktuell“, ticked by hand, shows a sheet
// that cannot be written. Hand-written for the same reason as the views.
//
// THE WAGEN-ZUSTAND gives most Wagen a Telematik reading — one of them silent
// for twelve days, so the dashboard's red shows — plus an open and a closed
// Schadensmeldung, an open Werkstattauftrag and a P8 beside a revision. The
// readings are dated relative to NOW (`vorTagen`), or „funkte vor N Tagen“
// would grow every day the seed is not touched.
//
// THE MASTER FILE starts with one taken-over version, and a pick answers a
// second one as pending — with a cut tail, a retyped Wagennummer, a text date,
// a note and a formula tail, every kind of line its report renders.
// ────────────────────────────────────────────────────────────────

import type {
  FakeCleanReport,
  FakeDokument,
  FakeDocument,
  FakeEinbau,
  FakeExportSheetRun,
  FakeInstandhaltung,
  FakeMasterFileVersion,
  FakeMasterSettings,
  FakeMasterSheetView,
  FakePartner,
  FakeRadsatz,
  FakeScanFile,
  FakeSeed,
  FakeStaging,
  FakeTemplate,
  FakeWagenZustand,
  FakeWaggon,
} from '../e2e/fake-backend';

const MTIME = 1_770_000_000_000;

const documents: FakeDocument[] = [
  {
    id: 'dok-mietvertrag',
    name: 'Mietvertrag.pdf',
    filename: 'C:\\Vorlagen\\Mietvertrag.pdf',
    mtime: MTIME,
    type: 'pdf',
    previewfile: 'C:\\Vorlagen\\Mietvertrag_Vorschau.pdf',
    fields: [
      { id: 'feld-vorname', path: 'Mieter.Vorname' },
      { id: 'feld-nachname', path: 'Mieter.Nachname' },
      { id: 'feld-strasse', path: 'Mieter.Anschrift.Straße' },
      { id: 'feld-plz', path: 'Mieter.Anschrift.PLZ' },
      { id: 'feld-ort', path: 'Mieter.Anschrift.Ort' },
      { id: 'feld-beginn', path: 'Mietbeginn' },
      { id: 'feld-kaution', path: 'Kaution' },
    ],
    mapped: [
      { origId: 'feld-vorname', mappedName: 'Vorname' },
      { origId: 'feld-nachname', mappedName: 'Nachname' },
      { origId: 'feld-strasse', mappedName: 'Straße' },
      { origId: 'feld-plz', mappedName: 'PLZ' },
      { origId: 'feld-ort', mappedName: 'Ort' },
      { origId: 'feld-beginn', mappedName: 'Mietbeginn' },
    ],
  },
  {
    id: 'dok-uebergabe',
    name: 'Übergabeprotokoll.pdf',
    filename: 'C:\\Vorlagen\\Übergabeprotokoll.pdf',
    mtime: MTIME,
    type: 'pdf',
    previewfile: 'C:\\Vorlagen\\Übergabeprotokoll_Vorschau.pdf',
    fields: [
      { id: 'ueb-vorname', path: 'Übernehmer.Vorname' },
      { id: 'ueb-nachname', path: 'Übernehmer.Nachname' },
      { id: 'ueb-datum', path: 'Übergabedatum' },
      { id: 'ueb-zaehler', path: 'Zählerstand.Strom' },
    ],
    mapped: [
      { origId: 'ueb-vorname', mappedName: 'Vorname' },
      { origId: 'ueb-nachname', mappedName: 'Nachname' },
      { origId: 'ueb-datum', mappedName: 'Übergabedatum' },
    ],
  },
  {
    id: 'dok-nebenkosten',
    name: 'Nebenkosten.xlsx',
    filename: 'C:\\Vorlagen\\Nebenkosten.xlsx',
    mtime: MTIME,
    type: 'xlsx',
    sheets: [
      { id: 'blatt-abrechnung', name: 'Abrechnung' },
      { id: 'blatt-zaehler', name: 'Zählerstände' },
    ],
    mapped: [
      { origId: 'blatt-abrechnung', mappedName: '$Abrechnung.B4' },
      { origId: 'blatt-zaehler', mappedName: '$Zählerstände.C2' },
    ],
  },
  {
    id: 'dok-hausordnung',
    name: 'Hausordnung.pdf',
    filename: 'C:\\Vorlagen\\Hausordnung.pdf',
    mtime: MTIME,
    type: 'resource',
  },
];

const source = (file: string, row: number) => ({
  file,
  sheet: 'Tabelle1',
  row,
  importedAt: '2026-08-20',
});

const partners: FakePartner[] = [
  {
    id: 'p-wagenmut',
    rollen: ['halter'],
    name: 'Wagenmut AG',
    matchKey: 'wagenmut',
    aliases: ['WAGENMUT', 'Wagenmut AG, Zentrale'],
    createdAt: '2026-01-08',
  },
  {
    id: 'p-achsentreu',
    rollen: ['halter', 'eigentuemer'],
    name: 'Achsentreu SA',
    matchKey: 'achsentreu',
    aliases: ['ACHSENTREU'],
    createdAt: '2026-01-08',
  },
  {
    id: 'p-nieten',
    rollen: ['eigentuemer'],
    name: 'Nietenzähler Rail Leasing GmbH',
    matchKey: 'nietenzaehler rail leasing',
    aliases: [],
    createdAt: '2026-01-09',
  },
  {
    id: 'p-werk-schienenbein',
    rollen: ['werkstatt'],
    name: 'Schienenbein Waggonwerk GmbH',
    matchKey: 'schienenbein waggonwerk',
    aliases: ['Schienenbein WW', 'Schienenbein Waggonwerk'],
    createdAt: '2026-02-02',
  },
  {
    id: 'p-werk-drehgestell',
    rollen: ['werkstatt'],
    name: 'Dreh & Gestell Technik',
    matchKey: 'dreh gestell technik',
    aliases: ['D&G Technik'],
    createdAt: '2026-02-11',
  },
  {
    id: 'p-werk-rundlauf',
    rollen: ['werkstatt'],
    name: 'Rundlauf Radsatztechnik',
    matchKey: 'rundlauf radsatztechnik',
    aliases: [],
    createdAt: '2026-03-04',
  },
];

const wagen: FakeWaggon[] = [
  {
    id: 'wg-1',
    nummer: '218124712173',
    halterId: 'p-wagenmut',
    eigentuemerId: 'p-nieten',
    bauart: 'Tanoos',
    createdAt: '2026-02-02',
  },
  {
    id: 'wg-2',
    nummer: '338080123452',
    halterId: 'p-wagenmut',
    eigentuemerId: 'p-achsentreu',
    bauart: 'Zans',
    createdAt: '2026-02-02',
  },
  {
    id: 'wg-3',
    nummer: '378045567815',
    halterId: 'p-achsentreu',
    bauart: 'Habbiins',
    createdAt: '2026-02-14',
  },
  {
    id: 'wg-4',
    nummer: '238566234569',
    halterId: 'p-wagenmut',
    createdAt: '2026-03-04',
  },
  { id: 'wg-5', nummer: '318133445565', createdAt: '2026-04-17' },
];

function vorTagen(tage: number, stunde: number): string {
  const moment = new Date();
  moment.setDate(moment.getDate() - tage);
  const pad = (value: number) => String(value).padStart(2, '0');
  return `${moment.getFullYear()}-${pad(moment.getMonth() + 1)}-${pad(moment.getDate())}T${pad(stunde)}:15:00`;
}

const quelle = (file: string, row: number) => ({
  file,
  sheet: 'Tabelle1',
  row,
  importedAt: '2026-10-06',
});

const zustand: FakeWagenZustand = {
  geraete: [
    {
      id: 'tg-1',
      kennung: 'PI1703372',
      wagenId: 'wg-1',
      angebautAm: '2024-04-15',
      source: quelle('Telematik.xlsx', 2),
    },
    {
      id: 'tg-2',
      kennung: '1102914295',
      wagenId: 'wg-2',
      angebautAm: '2024-09-05',
      source: quelle('Telematik.xlsx', 3),
    },
  ],
  meldungen: [
    {
      id: 'tm-1',
      wagenId: 'wg-1',
      geraetId: 'tg-1',
      zeitpunkt: vorTagen(1, 8),
      stadt: 'Neuhof',
      land: 'DE',
      standort: 'Neuhof (Kr Fulda)',
      laufleistungKm: 227_734,
      energieProzent: 98,
      bewegung: 'BEWEGUNG',
      source: quelle('Telematik.xlsx', 2),
    },
    {
      id: 'tm-2',
      wagenId: 'wg-2',
      geraetId: 'tg-2',
      zeitpunkt: vorTagen(12, 17),
      stadt: 'Bad Hersfeld',
      land: 'DE',
      standort: 'BAD HERSFELD-EICHHOF',
      laufleistungKm: 168_973,
      energieProzent: 41,
      bewegung: 'STILLSTAND',
      source: quelle('Telematik.xlsx', 3),
    },
    {
      id: 'tm-3',
      wagenId: 'wg-3',
      zeitpunkt: vorTagen(0, 6),
      stadt: 'Altenburg',
      land: 'DE',
      source: quelle('Telematik.xlsx', 4),
    },
  ],
  schaeden: [
    {
      id: 'sm-1',
      wagenId: 'wg-1',
      gemeldetAm: '2026-09-29',
      gemeldetVon: 'Disposition',
      schadcode: '3.3.4',
      notiz: 'Bremsklotz fehlt',
      ausgesetzt: true,
      aktion: 'Werkstatt beauftragen',
      source: quelle('Schadensmeldungen.xlsx', 2),
    },
    {
      id: 'sm-2',
      wagenId: 'wg-2',
      gemeldetAm: '2026-08-11',
      schadcode: '1.2.1',
      notiz: 'Griffstange verbogen',
      erledigtAm: '2026-08-20',
      source: quelle('Schadensmeldungen.xlsx', 3),
    },
  ],
  auftraege: [
    {
      id: 'wa-1',
      wagenId: 'wg-1',
      bestellnummer: '24094-26/01',
      werkstattId: 'p-werk-schienenbein',
      status: 'BS_FREIGABE',
      erfasstAm: '2026-09-30',
      source: quelle('Bestellungen.xlsx', 2),
    },
  ],
  pruefungen: [
    {
      id: 'pr-1',
      wagenId: 'wg-1',
      art: 'P8',
      faelligAm: '2027-03-31',
      status: 'G',
      source: quelle('P8.xlsx', 2),
    },
    {
      id: 'pr-2',
      wagenId: 'wg-1',
      art: 'G4.2',
      faelligAm: '2030-06-30',
      status: 'Abgeschlossen',
      source: quelle('Revisionen.xlsx', 2),
    },
    {
      id: 'pr-3',
      wagenId: 'wg-3',
      art: 'P8',
      faelligAm: '2026-11-15',
      status: 'A',
      bestellnummer: '27642-26',
      source: quelle('P8.xlsx', 3),
    },
  ],
};

const radsaetze: FakeRadsatz[] = [
  {
    id: 'rs-1',
    nummer: 'RS-2024-0815',
    matchKey: 'rs20240815',
    aliases: [{ matchKey: 'rs 815', partnerId: 'p-werk-schienenbein' }],
    wellennummer: 'W-88431',
    systemId: '180002238',
    bauart: 'BA 004',
    createdAt: '2026-02-02',
  },
  {
    id: 'rs-2',
    nummer: 'RS-2024-0816',
    matchKey: 'rs20240816',
    aliases: [],
    bauart: 'BA 004',
    createdAt: '2026-02-02',
  },
  {
    id: 'rs-3',
    nummer: '4711-B',
    matchKey: '4711b',
    aliases: [
      { matchKey: '4711 b', partnerId: 'p-werk-drehgestell' },
      { matchKey: '04711-b', partnerId: 'p-werk-rundlauf' },
    ],
    wellennummer: 'W-91002',
    createdAt: '2026-03-04',
  },
  {
    id: 'rs-4',
    nummer: 'RS-2025-0002',
    matchKey: 'rs20250002',
    aliases: [],
    createdAt: '2026-04-17',
  },
];

const einbauten: FakeEinbau[] = [
  {
    id: 'eb-1',
    radsatzId: 'rs-1',
    wagenId: 'wg-1',
    position: '1',
    eingebautAm: '2026-02-02',
    source: source('schienenbein-02.xlsx', 4),
  },
  {
    id: 'eb-2',
    radsatzId: 'rs-2',
    wagenId: 'wg-2',
    position: '3',
    eingebautAm: '2026-02-02',
    source: source('schienenbein-02.xlsx', 5),
  },
  {
    id: 'eb-3',
    radsatzId: 'rs-3',
    wagenId: 'wg-4',
    position: '2',
    eingebautAm: '2026-03-04',
    source: source('rundlauf-03.xlsx', 9),
  },
  {
    id: 'eb-4',
    radsatzId: 'rs-4',
    wagenId: 'wg-3',
    position: '4',
    eingebautAm: '2025-11-12',
    ausgebautAm: '2026-04-17',
    source: source('drehgestell-04.xlsx', 12),
  },
];

const events: FakeInstandhaltung[] = [
  {
    id: 'ih-1',
    wagenId: 'wg-1',
    werkstattId: 'p-werk-schienenbein',
    datum: '2026-02-02',
    leistung: 'Inspektion nach IS1',
    betragCent: 128_450,
    dedupeKey: 'wg-1|2026-02-02|is1',
    source: source('schienenbein-02.xlsx', 4),
  },
  {
    id: 'ih-2',
    wagenId: 'wg-1',
    werkstattId: 'p-werk-schienenbein',
    radsatzId: 'rs-1',
    datum: '2026-02-02',
    leistung: 'Radsatz getauscht',
    betragCent: 342_000,
    bemerkung: 'Laufflächenschaden',
    dedupeKey: 'wg-1|2026-02-02|radsatz',
    source: source('schienenbein-02.xlsx', 4),
  },
  {
    id: 'ih-3',
    wagenId: 'wg-2',
    werkstattId: 'p-werk-schienenbein',
    datum: '2026-02-03',
    leistung: 'Wartung Bremsanlage',
    betragCent: 48_900,
    dedupeKey: 'wg-2|2026-02-03|bremse',
    source: source('schienenbein-02.xlsx', 5),
  },
  {
    id: 'ih-4',
    wagenId: 'wg-3',
    werkstattId: 'p-werk-drehgestell',
    datum: '2026-02-14',
    leistung: 'Instandsetzung Seitenwand',
    betragCent: 1_204_000,
    dedupeKey: 'wg-3|2026-02-14|seitenwand',
    source: source('drehgestell-02.xlsx', 7),
  },
  {
    id: 'ih-5',
    wagenId: 'wg-3',
    werkstattId: 'p-werk-drehgestell',
    leistung: 'Sichtprüfung ohne Datum in der Datei',
    dedupeKey: 'wg-3||sicht',
    source: source('drehgestell-02.xlsx', 8),
  },
  {
    id: 'ih-6',
    wagenId: 'wg-4',
    werkstattId: 'p-werk-rundlauf',
    radsatzId: 'rs-3',
    datum: '2026-03-04',
    leistung: 'Radsatz aufgearbeitet',
    betragCent: 276_500,
    dedupeKey: 'wg-4|2026-03-04|aufarbeitung',
    source: source('rundlauf-03.xlsx', 9),
  },
  {
    id: 'ih-7',
    wagenId: 'wg-4',
    werkstattId: 'p-werk-rundlauf',
    datum: '2026-03-05',
    leistung: 'Verbesserung Ladungssicherung',
    betragCent: 96_000,
    dedupeKey: 'wg-4|2026-03-05|ladung',
    source: source('rundlauf-03.xlsx', 10),
  },
  {
    id: 'ih-8',
    wagenId: 'wg-5',
    werkstattId: 'p-werk-drehgestell',
    datum: '2026-04-17',
    leistung: 'Revision',
    betragCent: 2_480_000,
    dedupeKey: 'wg-5|2026-04-17|revision',
    source: source('drehgestell-04.xlsx', 12),
  },
  {
    id: 'ih-9',
    wagenId: 'wg-5',
    werkstattId: 'p-werk-drehgestell',
    radsatzId: 'rs-4',
    datum: '2026-04-17',
    leistung: 'Radsatz ausgebaut',
    dedupeKey: 'wg-5|2026-04-17|ausbau',
    source: source('drehgestell-04.xlsx', 12),
  },
  {
    id: 'ih-10',
    wagenId: 'wg-2',
    werkstattId: 'p-werk-schienenbein',
    datum: '2026-05-06',
    leistung: 'Wartung Kupplung',
    betragCent: 31_250,
    dedupeKey: 'wg-2|2026-05-06|kupplung',
    source: source('schienenbein-05.xlsx', 3),
  },
];

const templates: FakeTemplate[] = [
  {
    id: 'tpl-bremen',
    name: 'Schienenbein Waggonwerk — Monatsliste',
    partnerId: 'p-werk-schienenbein',
    builtin: false,
    createdAt: '2026-02-02',
    plan: {
      reader: 'headerRow',
      layout: { headerRow: 1, firstDataRow: 2 },
      columns: [
        { header: 'Wagen-Nr.', index: 1, field: 'wagennummer' },
        { header: 'Datum', index: 2, field: 'datum', dateOrder: 'dayFirst' },
        { header: 'Leistung', index: 3, field: 'leistung' },
        { header: 'Betrag', index: 4, field: 'betrag', decimal: 'german' },
      ],
      date1904: false,
    },
  },
  {
    id: 'tpl-hamm',
    name: 'Rundlauf Radsatztechnik — Radsatzblatt',
    partnerId: 'p-werk-rundlauf',
    builtin: false,
    createdAt: '2026-03-04',
    plan: {
      reader: 'headerRow',
      layout: { headerRow: 2, firstDataRow: 3 },
      columns: [
        { header: 'Wagen', index: 1, field: 'wagennummer' },
        { header: 'Radsatz', index: 2, field: 'radsatznummer' },
        { header: 'Pos.', index: 3, field: 'einbauposition' },
        { header: 'Einbau', index: 4, field: 'eingebautAm' },
      ],
      date1904: false,
    },
  },
  {
    id: 'builtin:radsatz-monitoring',
    name: 'Radsatz-Monitoring',
    builtin: true,
    createdAt: '2026-10-03',
    plan: {
      reader: 'headerRow',
      layout: { headerRow: 1, firstDataRow: 2 },
      columns: [
        { header: 'Wagennummer', index: 1, field: 'wagennummer' },
        { header: 'Radsatznummer', index: 2, field: 'radsatznummer' },
        { header: 'Position', index: 3, field: 'einbauposition' },
        { header: 'Einbaudatum', index: 4, field: 'eingebautAm' },
      ],
      date1904: false,
    },
  },
];

export function demoScan(): FakeScanFile[] {
  return [
    {
      path: 'C:\\Eingang\\Schienenbein Mai 2026.xlsx',
      name: 'Schienenbein Mai 2026.xlsx',
      status: 'erkannt',
      matches: [
        {
          templateId: 'tpl-bremen',
          templateName: 'Schienenbein Waggonwerk — Monatsliste',
          sheet: 'Mai',
        },
      ],
      sheets: ['Mai', 'April', 'Deckblatt'],
    },
    {
      path: 'C:\\Eingang\\Radsatzblatt KW 38.xlsx',
      name: 'Radsatzblatt KW 38.xlsx',
      status: 'mehrdeutig',
      matches: [
        {
          templateId: 'tpl-hamm',
          templateName: 'Rundlauf Radsatztechnik — Radsatzblatt',
          sheet: 'Radsätze',
        },
        {
          templateId: 'builtin:radsatz-monitoring',
          templateName: 'Radsatz-Monitoring',
          sheet: 'Radsätze',
        },
      ],
      sheets: ['Radsätze', 'Notizen'],
    },
    {
      path: 'C:\\Eingang\\Rechnung Dreh & Gestell.xlsx',
      name: 'Rechnung Dreh & Gestell.xlsx',
      status: 'unbekannt',
      matches: [],
      sheets: ['Tabelle1'],
    },
    {
      path: 'C:\\Eingang\\Schienenbein April 2026.xlsx',
      name: 'Schienenbein April 2026.xlsx',
      status: 'vorhanden',
      matches: [],
      sheets: ['April'],
      message: 'Bereits bereinigt am 2026-09-04 und importiert am 2026-09-04.',
      vorhanden: {
        dokumentId: 'dok-april',
        bereinigtAm: '2026-09-04',
        importiertAm: '2026-09-04',
      },
    },
    {
      path: 'C:\\Eingang\\Begleitschreiben.pdf',
      name: 'Begleitschreiben.pdf',
      status: 'nichtUnterstuetzt',
      matches: [],
      sheets: [],
      message: 'Nur Excel-Dateien (.xlsx) werden eingelesen.',
    },
  ];
}

export function demoClean(): FakeCleanReport {
  return {
    file: 'C:\\Eingang\\Schienenbein Mai 2026.xlsx',
    sheet: 'Mai',
    templateId: 'tpl-bremen',
    templateName: 'Schienenbein Waggonwerk — Monatsliste',
    plan: demoStaging().plan,
    fehler: [
      {
        row: 7,
        column: 2,
        header: 'Datum',
        raw: '31.13.2026',
        message:
          '„31.13.2026“ ist kein gültiges Datum (es gibt keinen 13. Monat).',
        open: true,
      },
      {
        row: 12,
        column: 4,
        header: 'Betrag',
        raw: '12O,50',
        message: '„12O,50“ ist kein Betrag.',
        open: true,
      },
    ],
    cards: [
      {
        column: 4,
        header: 'Betrag',
        field: 'betrag',
        reading: { kind: 'decimal', chosen: 'german', alternative: 'english' },
        reason:
          'Jeder Betrag hat genau drei Stellen nach dem Trennzeichen — „1.250“ kann 1250 oder 1,25 heißen, und kein Wert der Spalte entscheidet es.',
        count: 3,
        examples: [
          { row: 3, raw: '1.250', chosen: '1.250,00', alternative: '1,25' },
          { row: 5, raw: '2.480', chosen: '2.480,00', alternative: '2,48' },
          { row: 9, raw: '875', chosen: '875,00', alternative: '875,00' },
        ],
        confirmed: false,
      },
      {
        column: 2,
        header: 'Datum',
        field: 'datum',
        reading: {
          kind: 'dateOrder',
          chosen: 'dayFirst',
          alternative: 'monthFirst',
        },
        reason:
          'Die Daten sind mit „/“ geschrieben und in keinem ist Tag oder Monat größer als 12 — die Reihenfolge lässt sich nicht ablesen.',
        count: 4,
        examples: [
          {
            row: 3,
            raw: '04/05/2026',
            chosen: '04.05.2026',
            alternative: '05.04.2026',
          },
          {
            row: 4,
            raw: '06/05/2026',
            chosen: '06.05.2026',
            alternative: '05.06.2026',
          },
          {
            row: 8,
            raw: '11/05/2026',
            chosen: '11.05.2026',
            alternative: '05.11.2026',
          },
        ],
        confirmed: false,
      },
      {
        column: 1,
        header: 'Wagen-Nr.',
        field: 'wagennummer',
        reading: { kind: 'hinweis' },
        reason:
          'Bei diesen Wagennummern stimmt die Prüfziffer nicht. Sie werden so übernommen, wie sie dastehen — bitte mit dem Wagen vergleichen.',
        count: 2,
        examples: [
          {
            row: 6,
            raw: '21 81 2471 217-4',
            chosen: '21 81 2471 217-4',
            message: 'Prüfziffer 4 erwartet 3.',
          },
          {
            row: 10,
            raw: '33 80 8012 345-0',
            chosen: '33 80 8012 345-0',
            message: 'Prüfziffer 0 erwartet 2.',
          },
        ],
        confirmed: false,
      },
    ],
    formats: [
      {
        column: 1,
        header: 'Wagen-Nr.',
        rule: 'Wagennummer in Blöcken geschrieben',
        count: 18,
        samples: [
          { row: 2, raw: '218124712173', clean: '21 81 2471 217-3' },
          { row: 3, raw: '33808012345-2', clean: '33 80 8012 345-2' },
          { row: 4, raw: '378045567815', clean: '37 80 4556 781-5' },
        ],
      },
      {
        column: 2,
        header: 'Datum',
        rule: 'Jahr vierstellig ergänzt',
        count: 6,
        samples: [
          { row: 13, raw: '02.05.26', clean: '02.05.2026' },
          { row: 14, raw: '03.05.26', clean: '03.05.2026' },
          { row: 15, raw: '07.05.26', clean: '07.05.2026' },
        ],
      },
      {
        column: 3,
        header: 'Werkstatt',
        rule: 'Leerzeichen am Rand entfernt',
        count: 4,
        samples: [
          {
            row: 2,
            raw: ' Schienenbein WW ',
            clean: 'Schienenbein WW',
          },
        ],
      },
    ],
    summary: {
      fehlerOffen: 2,
      deutungenOffen: 3,
      formatierungen: 28,
      korrigiert: 0,
    },
  };
}

export function demoStaging(): FakeStaging {
  const cell = (
    column: number,
    field: string,
    raw: string,
    parsed = raw,
    ok = true
  ) => ({ column, field, raw, parsed, ok });

  const missing = { state: 'missing' };

  return {
    id: 'stg-demo',
    file: 'Schienenbein Mai 2026.xlsx',
    sheet: 'Mai',
    sheets: ['Mai', 'April', 'Deckblatt'],
    plan: {
      reader: 'headerRow',
      layout: { headerRow: 1, firstDataRow: 2 },
      columns: [
        { header: 'Wagen-Nr.', index: 1, field: 'wagennummer' },
        { header: 'Datum', index: 2, field: 'datum', dateOrder: 'dayFirst' },
        { header: 'Werkstatt', index: 3, field: 'werkstatt' },
        { header: 'Leistung', index: 4, field: 'leistung' },
        { header: 'Betrag', index: 5, field: 'betrag', decimal: 'german' },
        { header: 'Radsatz', index: 6, field: 'radsatznummer' },
        { header: 'Interne Nr.', index: 7, field: 'ignorieren' },
      ],
      templateId: 'tpl-bremen',
      date1904: false,
    },
    candidates: [
      {
        reader: 'headerRow',
        readerLabel: 'Kopfzeile mit Datenzeilen',
        score: 92,
        reason:
          'Kopfzeile in Zeile 1: 7 Spaltentitel, 5 Datenzeile(n) darunter.',
        hint: { headerRow: 1, firstDataRow: 2 },
      },
      {
        reader: 'manual',
        readerLabel: 'Ohne Kopfzeile',
        score: 24,
        reason: 'Keine Kopfzeile erkannt — Spalten werden nach Lage benannt.',
        hint: { firstDataRow: 1 },
      },
    ],
    rows: [
      {
        row: 2,
        status: 'ready',
        cells: [
          cell(1, 'wagennummer', '21 81 2471 217-3', '218124712173'),
          cell(2, 'datum', '02.05.2026', '2026-05-02'),
          cell(
            3,
            'werkstatt',
            'Schienenbein WW',
            'Schienenbein Waggonwerk GmbH'
          ),
          cell(4, 'leistung', 'Wartung Bremsanlage'),
          cell(5, 'betrag', '1.284,50', '128450'),
          cell(6, 'radsatznummer', 'RS 815', 'RS-2024-0815'),
          cell(7, 'ignorieren', 'B-2026-0041'),
        ],
        wagen: { state: 'known', id: 'wg-1', name: '21 81 2471 217-3' },
        werkstatt: {
          state: 'known',
          id: 'p-werk-schienenbein',
          name: 'Schienenbein Waggonwerk GmbH',
        },
        halter: missing,
        eigentuemer: missing,
        radsatz: { state: 'known', id: 'rs-1', name: 'RS-2024-0815' },
        issues: [],
      },
      {
        row: 3,
        status: 'needsInput',
        cells: [
          cell(1, 'wagennummer', '33 80 8012 345-2', '338080123452'),
          cell(2, 'datum', '03.05.2026', '2026-05-03'),
          cell(
            3,
            'werkstatt',
            'Schienenb. Waggonwerk Süd',
            'Schienenb. Waggonwerk Süd'
          ),
          cell(4, 'leistung', 'Inspektion'),
          cell(5, 'betrag', '842,00', '84200'),
          cell(6, 'radsatznummer', '04711-B', '04711-B'),
          cell(7, 'ignorieren', 'B-2026-0042'),
        ],
        wagen: { state: 'known', id: 'wg-2', name: '33 80 8012 345-2' },
        werkstatt: {
          state: 'ambiguous',
          candidates: [
            {
              id: 'p-werk-schienenbein',
              name: 'Schienenbein Waggonwerk GmbH',
              score: 72,
              why: 'Zwei bekannte Schreibweisen liegen nah an diesem Namen.',
            },
            {
              id: 'p-werk-rundlauf',
              name: 'Rundlauf Radsatztechnik',
              score: 31,
              why: 'Teilweise gleiche Wörter.',
            },
          ],
        },
        halter: missing,
        eigentuemer: missing,
        radsatz: {
          state: 'likely',
          id: 'rs-3',
          name: '4711-B',
          hint: 'Unterscheidet sich nur in führenden Nullen.',
        },
        issues: [
          {
            row: 3,
            column: 'Werkstatt',
            raw: 'Schienenb. Waggonwerk Süd',
            message:
              '„Schienenb. Waggonwerk Süd“ ist keiner bekannten Werkstatt eindeutig zuzuordnen.',
            severity: 'warnung',
          },
        ],
      },
      {
        row: 4,
        status: 'ready',
        cells: [
          cell(1, 'wagennummer', '37 80 4556 781-5', '378045567815'),
          cell(2, 'datum', '05.05.2026', '2026-05-05'),
          cell(
            3,
            'werkstatt',
            'Schienenbein WW',
            'Schienenbein Waggonwerk GmbH'
          ),
          cell(4, 'leistung', 'Verbesserung Ladungssicherung'),
          cell(5, 'betrag', '2.960,00', '296000'),
          cell(6, 'radsatznummer', '', ''),
          cell(7, 'ignorieren', 'B-2026-0043'),
        ],
        wagen: { state: 'known', id: 'wg-3', name: '37 80 4556 781-5' },
        werkstatt: {
          state: 'known',
          id: 'p-werk-schienenbein',
          name: 'Schienenbein Waggonwerk GmbH',
        },
        halter: missing,
        eigentuemer: missing,
        radsatz: missing,
        issues: [],
      },
      {
        row: 5,
        status: 'duplicate',
        cells: [
          cell(1, 'wagennummer', '33 80 8012 345-2', '338080123452'),
          cell(2, 'datum', '06.05.2026', '2026-05-06'),
          cell(
            3,
            'werkstatt',
            'Schienenbein WW',
            'Schienenbein Waggonwerk GmbH'
          ),
          cell(4, 'leistung', 'Wartung Kupplung'),
          cell(5, 'betrag', '312,50', '31250'),
          cell(6, 'radsatznummer', '', ''),
          cell(7, 'ignorieren', 'B-2026-0044'),
        ],
        wagen: { state: 'known', id: 'wg-2', name: '33 80 8012 345-2' },
        werkstatt: {
          state: 'known',
          id: 'p-werk-schienenbein',
          name: 'Schienenbein Waggonwerk GmbH',
        },
        halter: missing,
        eigentuemer: missing,
        radsatz: missing,
        issues: [
          {
            row: 5,
            column: 'Zeile',
            raw: '33 80 8012 345-2 · 06.05.2026',
            message:
              'Diese Instandhaltung ist bereits übernommen (Wartung Kupplung am 06.05.2026).',
            severity: 'warnung',
          },
        ],
      },
      {
        row: 6,
        status: 'rejected',
        cells: [
          cell(1, 'wagennummer', 'Summe', '', false),
          cell(2, 'datum', '', ''),
          cell(
            3,
            'werkstatt',
            'Schienenbein WW',
            'Schienenbein Waggonwerk GmbH'
          ),
          cell(4, 'leistung', ''),
          cell(5, 'betrag', '5.399,00', '539900'),
          cell(6, 'radsatznummer', '', ''),
          cell(7, 'ignorieren', ''),
        ],
        wagen: missing,
        werkstatt: {
          state: 'known',
          id: 'p-werk-schienenbein',
          name: 'Schienenbein Waggonwerk GmbH',
        },
        halter: missing,
        eigentuemer: missing,
        radsatz: missing,
        issues: [
          {
            row: 6,
            column: 'Wagen-Nr.',
            raw: 'Summe',
            message:
              '„Summe“ ist keine gültige Wagennummer (12 Ziffern erwartet, 0 gefunden).',
            severity: 'fehler',
          },
        ],
      },
    ],
    summary: {
      total: 5,
      ready: 2,
      needsInput: 1,
      duplicates: 1,
      rejected: 1,
      neueWagen: 0,
      neuePartner: 1,
      neueRadsaetze: 0,
    },
  };
}

/** The cleaned copy of the Mai file as `stage_document` would stage it:
 *  `demoStaging()` plus the entity groups `entities::group` computes in Rust —
 *  written out by hand here, because grouping is not the fake's to re-derive.
 *  The third Wagen is new to the store, so every resolution the walk has to
 *  render appears at least once. */
export function demoDocument(): FakeStaging {
  const staging = demoStaging();
  const fresh = { state: 'new', proposal: '37 80 4556 781-5' };
  const third = staging.rows.find((row) => row.row === 4);
  if (third) third.wagen = fresh;
  const format = (
    row: number,
    column: number,
    header: string,
    raw: string,
    clean: string,
    rule: string
  ) => ({ row, column, header, raw, clean, tier: 'format', rule });

  return {
    ...staging,
    id: 'stg-dokument',
    entities: {
      partner: [
        {
          key: 'werkstatt:SCHIENENBEINWW',
          kind: 'partner',
          rolle: 'werkstatt',
          spellings: ['Schienenbein WW'],
          resolution: {
            state: 'known',
            id: 'p-werk-schienenbein',
            name: 'Schienenbein Waggonwerk GmbH',
          },
          rows: [2, 4, 5],
          changes: [
            format(
              2,
              3,
              'Werkstatt',
              ' Schienenbein WW ',
              'Schienenbein WW',
              'Leerzeichen entfernt'
            ),
          ],
        },
        {
          key: 'werkstatt:SCHIENENBWAGGONWERKSUED',
          kind: 'partner',
          rolle: 'werkstatt',
          spellings: ['Schienenb. Waggonwerk Süd'],
          resolution: staging.rows[1]?.werkstatt ?? { state: 'missing' },
          rows: [3],
          changes: [],
        },
      ],
      wagen: [
        {
          key: '21 81 2471 217-3',
          kind: 'wagen',
          spellings: ['21 81 2471 217-3'],
          resolution: { state: 'known', id: 'wg-1', name: '21 81 2471 217-3' },
          rows: [2],
          changes: [
            format(
              2,
              1,
              'Wagen-Nr.',
              '21812471217-3',
              '21 81 2471 217-3',
              'Wagennummer vereinheitlicht'
            ),
          ],
        },
        {
          key: '33 80 8012 345-2',
          kind: 'wagen',
          spellings: ['33 80 8012 345-2'],
          resolution: { state: 'known', id: 'wg-2', name: '33 80 8012 345-2' },
          rows: [3, 5],
          changes: [],
        },
        {
          key: '37 80 4556 781-5',
          kind: 'wagen',
          spellings: ['37 80 4556 781-5'],
          resolution: fresh,
          rows: [4],
          changes: [],
        },
      ],
      radsaetze: [
        {
          key: 'RS815@p-werk-schienenbein',
          kind: 'radsatz',
          spellings: ['RS 815'],
          resolution: { state: 'known', id: 'rs-1', name: 'RS-2024-0815' },
          rows: [2],
          changes: [],
        },
        {
          key: '04711B@',
          kind: 'radsatz',
          spellings: ['04711-B'],
          resolution: staging.rows[1]?.radsatz ?? { state: 'missing' },
          rows: [3],
          changes: [],
        },
      ],
    },
  };
}

const dokumente: FakeDokument[] = [
  {
    id: 'dok-mai',
    name: 'Schienenbein Mai 2026.xlsx',
    sheet: 'Mai',
    templateId: 'tpl-bremen',
    templateName: 'Schienenbein Waggonwerk — Monatsliste',
    plan: demoStaging().plan,
    originalHash: 'a1',
    cleanedHash: 'b1',
    folder: 'data/trains/dokumente/dok-mai',
    original: 'data/trains/dokumente/dok-mai/Schienenbein Mai 2026.xlsx',
    cleaned:
      'data/trains/dokumente/dok-mai/Schienenbein Mai 2026.bereinigt.xlsx',
    summary: {
      fehlerOffen: 0,
      deutungenOffen: 0,
      formatierungen: 28,
      korrigiert: 2,
    },
    bereinigtAm: '2026-10-02',
  },
  {
    id: 'dok-april',
    name: 'Schienenbein April 2026.xlsx',
    sheet: 'April',
    templateId: 'tpl-bremen',
    templateName: 'Schienenbein Waggonwerk — Monatsliste',
    plan: demoStaging().plan,
    originalHash: 'a2',
    cleanedHash: 'b2',
    folder: 'data/trains/dokumente/dok-april',
    original: 'data/trains/dokumente/dok-april/Schienenbein April 2026.xlsx',
    cleaned:
      'data/trains/dokumente/dok-april/Schienenbein April 2026.bereinigt.xlsx',
    summary: {
      fehlerOffen: 0,
      deutungenOffen: 0,
      formatierungen: 12,
      korrigiert: 0,
    },
    bereinigtAm: '2026-09-04',
    importiertAm: '2026-09-04',
  },
];

let committed = 0;

/** What a committed row "becomes". The fake does not run `commit.rs` — that is
 *  the module whose gates are proved by `cargo test` — so the dev shell invents a
 *  plausible Instandhaltung, and the counts and the list then move. */
export function demoMasterStaging(): FakeStaging {
  const staged = demoDocument();
  return {
    ...staged,
    id: 'stg-master',
    entities: staged.entities && {
      ...staged.entities,
      einbauten: [
        {
          key: 'rs-1',
          radsatz: 'RS-2024-0815',
          wagen: '218124712173',
          bisher: '2026-02-02',
          bisherQuelle: 'Radsätze aktuell, Zeile 2',
          neu: '2026-06-19',
          rows: [2],
        },
      ],
    },
  };
}

export function committedEvent(row: number): FakeInstandhaltung {
  const nth = ++committed;
  return {
    id: `ih-neu-${nth}`,
    wagenId: wagen[nth % wagen.length]?.id ?? 'wg-1',
    werkstattId: 'p-werk-schienenbein',
    datum: '2026-05-02',
    leistung: `Übernommen aus Zeile ${row}`,
    betragCent: 100_000 + nth * 1_337,
    dedupeKey: `neu-${nth}`,
    source: source('Schienenbein Mai 2026.xlsx', row),
  };
}

const master: FakeMasterSettings = {
  file: 'C:\\Daten\\masterdatei\\m-2026-09\\Wagenmut Übersicht bereinigt.xlsx',
  bindings: [
    {
      sheet: 'Alle Wagen',
      templateId: '',
      kind: 'wagenliste',
      aliases: [],
      auto: true,
    },
    {
      sheet: 'Radsätze aktuell',
      templateId: '',
      kind: 'radsatzEinbau',
      aliases: [],
      auto: true,
    },
    {
      sheet: 'Werkstattliste',
      templateId: 'tpl-bremen',
      aliases: [],
      auto: false,
    },
  ],
  importRun: {
    startedAt: '2026-10-04',
    sheets: ['Alle Wagen', 'Radsätze aktuell'],
    done: ['Alle Wagen'],
  },
};

const masterDefaults: FakeMasterSettings['bindings'] = [
  ...master.bindings.slice(0, 2),
  {
    sheet: 'Werkstattliste',
    templateId: '',
    aliases: [],
    auto: true,
  },
];

const masterSheetViews: Record<string, FakeMasterSheetView> = {
  'Alle Wagen': {
    sheet: 'Alle Wagen',
    kind: 'wagenliste',
    rowLabel: 'Wagen',
    columns: [
      { index: 1, header: 'Wagen-Nr.', filled: true },
      { index: 2, header: 'Status', filled: false },
      { index: 3, header: 'Bemerkung', filled: false },
    ],
    rows: [
      ['218124712173', 2],
      ['238566234569', 3],
      ['318133445565', 4],
      ['338080123452', 5],
      ['378045567815', 6],
    ].map(([nummer, row]) => ({
      key: String(nummer),
      cells: [String(nummer), '', ''],
      source: `Alle Wagen, Zeile ${row}`,
    })),
  },
  'Radsätze aktuell': {
    sheet: 'Radsätze aktuell',
    kind: 'radsatzEinbau',
    rowLabel: 'Radsätze',
    columns: [
      { index: 1, header: 'Wagen', filled: true },
      { index: 2, header: 'Radsatz', filled: true },
      { index: 3, header: 'Eingebaut', filled: true },
      { index: 4, header: 'Position', filled: true },
    ],
    rows: [
      {
        key: '218124712173 · RS-2024-0815',
        cells: ['218124712173', 'RS-2024-0815', '02.02.2026', '1'],
        source: 'Radsätze aktuell, Zeile 2',
      },
      {
        key: '238566234569 · 4711-B',
        cells: ['238566234569', '4711-B', '04.03.2026', '2'],
        source: 'Radsätze aktuell, Zeile 4',
      },
      {
        key: '338080123452 · RS-2024-0816',
        cells: ['338080123452', 'RS-2024-0816', '02.02.2026', '3'],
        source: 'Radsätze aktuell, Zeile 3',
      },
      {
        key: 'RS-2025-0002',
        cells: ['', 'RS-2025-0002', '', ''],
      },
    ],
  },
};

const masterExport: Record<string, FakeExportSheetRun> = {
  Werkstattliste: {
    matched: ['Wagen-Nr.', 'Datum', 'Leistung', 'Betrag'],
    targets: ['Notiz'],
    open: ['Werkstatt'],
    line: '„Werkstattliste“: 2 Zeile(n) angehängt, 1 aktualisiert aus „Schienenbein Mai 2026.xlsx“.',
    changed: 9,
    removed: ['318133445565', '378045567815'],
    changes: [
      [
        'C4',
        4,
        'Leistung',
        '218124712173',
        'Radsatztausch',
        'Radsatztausch, Achse 2',
      ],
      ['D4', 4, 'Betrag', '218124712173', '1.180,00', '1.240,50'],
      ['A9', 9, 'Wagen-Nr.', '238566234569', '', '238566234569'],
      ['B9', 9, 'Datum', '238566234569', '', '12.05.2026'],
      ['C9', 9, 'Leistung', '238566234569', '', 'Bremsprüfung'],
      ['D9', 9, 'Betrag', '238566234569', '', '320,00'],
      ['A10', 10, 'Wagen-Nr.', '338080123452', '', '338080123452'],
      ['B10', 10, 'Datum', '338080123452', '', '19.05.2026'],
      ['C10', 10, 'Leistung', '338080123452', '', 'Revision R2'],
    ].map(([cell, row, column, key, before, after]) => ({
      cell: String(cell),
      row: Number(row),
      column: String(column),
      key: String(key),
      before: String(before),
      after: String(after),
    })),
  },
  'Radsätze aktuell': {
    matched: ['Wagen'],
    targets: ['Position'],
    open: [],
    problem:
      '„Radsätze aktuell“ wird fortlaufend ergänzt und braucht dafür eine Schlüsselspalte.',
    line: '',
    changed: 0,
    changes: [],
  },
};

function masterFileVersion(
  id: string,
  bereinigtAm: string,
  uebernommenAm?: string
): FakeMasterFileVersion {
  const folder = `C:\\Daten\\masterdatei\\${id}`;
  return {
    id,
    name: 'Wagenmut Übersicht.xlsx',
    folder,
    original: `${folder}\\Wagenmut Übersicht.xlsx`,
    cleaned: `${folder}\\Wagenmut Übersicht bereinigt.xlsx`,
    originalHash: id,
    cleanedHash: `${id}-b`,
    bereinigtAm,
    ...(uebernommenAm ? { uebernommenAm } : {}),
    report: {
      sheets: [
        {
          sheet: 'Alle Wagen',
          rowsCut: 1046846,
          tailRowsCut: 0,
          trimmed: 2,
          numbers: 1,
          dates: 0,
          examples: [
            {
              row: 14,
              column: 1,
              header: 'Wagen-Nr.',
              raw: '3385.065.9151-4',
              clean: '338506591514',
              rule: 'number',
            },
            {
              row: 20,
              column: 3,
              header: 'Bemerkung',
              raw: 'Abstellort Schadwagen: ',
              clean: 'Abstellort Schadwagen:',
              rule: 'trimmed',
            },
          ],
          notes: [],
          noteCount: 0,
        },
        {
          sheet: 'Werkstattliste',
          rowsCut: 5,
          tailRowsCut: 0,
          trimmed: 0,
          numbers: 0,
          dates: 1,
          examples: [
            {
              row: 7,
              column: 2,
              header: 'Datum',
              raw: '17.09.2024',
              clean: '17.09.2024',
              rule: 'date',
            },
          ],
          notes: [
            {
              row: 9,
              column: 4,
              header: 'Betrag',
              raw: '1,234',
              reason: 'Zahl als Text, nicht eindeutig lesbar',
            },
          ],
          noteCount: 1,
        },
        {
          sheet: 'Radsätze aktuell',
          rowsCut: 0,
          tailRowsCut: 1048398,
          formulaTail: 175,
          trimmed: 0,
          numbers: 0,
          dates: 0,
          examples: [],
          notes: [],
          noteCount: 0,
        },
        {
          sheet: 'Dashboard',
          rowsCut: 0,
          tailRowsCut: 0,
          trimmed: 0,
          numbers: 0,
          dates: 0,
          examples: [],
          notes: [],
          noteCount: 0,
        },
      ],
      totals: {
        rowsCut: 1046851,
        tailRowsCut: 1048398,
        trimmed: 2,
        numbers: 1,
        dates: 1,
        notes: 2,
      },
    },
  };
}

export const DEMO_SEED: FakeSeed = {
  dokumente,
  wagen,
  partners,
  radsaetze,
  einbauten,
  zustand,
  events,
  templates,
  master,
  masterDefaults,
  masterSheetViews,
  masterExport,
  masterFile: {
    versions: [masterFileVersion('m-2026-09', '2026-09-15', '2026-09-15')],
  },
  masterFilePick: masterFileVersion('m-2026-10', '2026-10-04'),
  masterSheets: [
    { name: 'Alle Wagen', headers: ['Wagen-Nr.', 'Status', 'Bemerkung'] },
    {
      name: 'Werkstattliste',
      headers: ['Wagen-Nr.', 'Datum', 'Leistung', 'Betrag', 'Notiz'],
    },
    {
      name: 'Radsätze aktuell',
      headers: ['Wagen', 'Radsatz', 'Eingebaut', 'Position'],
    },
  ],
  documents,
  profiles: [
    {
      id: 'profil-standard',
      name: 'Standard',
      documentIds: ['dok-mietvertrag', 'dok-hausordnung'],
      fieldIds: ['feld-vorname', 'feld-nachname', 'feld-strasse', 'feld-ort'],
    },
    {
      id: 'profil-uebergabe',
      name: 'Wohnungsübergabe',
      documentIds: ['dok-uebergabe', 'dok-nebenkosten', 'dok-hausordnung'],
      fieldIds: [
        'ueb-vorname',
        'ueb-nachname',
        'ueb-datum',
        'blatt-zaehler',
        'feld-vorname',
      ],
    },
  ],
};

let picked = 0;

export function nextPickedDocument(): FakeDocument {
  const nth = ++picked;
  switch (nth % 3) {
    case 1:
      return {
        id: `dok-neu-${nth}`,
        name: `Anschreiben ${nth}.pdf`,
        filename: `C:\\Vorlagen\\Anschreiben ${nth}.pdf`,
        mtime: MTIME,
        type: 'pdf',
        previewfile: `C:\\Vorlagen\\Anschreiben ${nth}_Vorschau.pdf`,
        fields: [
          { id: `neu-${nth}-anrede`, path: 'Empfänger.Anrede' },
          { id: `neu-${nth}-name`, path: 'Empfänger.Name' },
          { id: `neu-${nth}-datum`, path: 'Datum' },
        ],
        mapped: [{ origId: `neu-${nth}-name`, mappedName: 'Nachname' }],
      };
    case 2:
      return {
        id: `dok-neu-${nth}`,
        name: `Aufstellung ${nth}.xlsx`,
        filename: `C:\\Vorlagen\\Aufstellung ${nth}.xlsx`,
        mtime: MTIME,
        type: 'xlsx',
        sheets: [{ id: `neu-${nth}-blatt`, name: 'Tabelle1' }],
        mapped: [],
      };
    default:
      return {
        id: `dok-neu-${nth}`,
        name: `Anlage ${nth}.png`,
        filename: `C:\\Vorlagen\\Anlage ${nth}.png`,
        mtime: MTIME,
        type: 'resource',
      };
  }
}
