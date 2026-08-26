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
// `demoStaging()` builds a FRESH staging per call, and that is not tidiness:
// `discard_import` sets it to null, so a shared constant would make the second
// file of a session answer with nothing. It covers all four row states plus a
// Fehler and a Warnung, because the review screen's chips, its selects and its
// "nur fehlerfreie Zeilen" default cannot be seen with a clean file.
// ────────────────────────────────────────────────────────────────

import type {
  FakeDocument,
  FakeEinbau,
  FakeInstandhaltung,
  FakePartner,
  FakeRadsatz,
  FakeSeed,
  FakeStaging,
  FakeTemplate,
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

const radsaetze: FakeRadsatz[] = [
  {
    id: 'rs-1',
    nummer: 'RS-2024-0815',
    matchKey: 'rs20240815',
    aliases: [{ matchKey: 'rs 815', partnerId: 'p-werk-schienenbein' }],
    wellennummer: 'W-88431',
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
    fingerprint: 'wagennummer|datum|leistung|betrag',
    partnerId: 'p-werk-schienenbein',
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
    fingerprint: 'wagennummer|radsatznummer|einbauposition|eingebautam',
    partnerId: 'p-werk-rundlauf',
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
];

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

let committed = 0;

/** What a committed row "becomes". The fake does not run `commit.rs` — that is
 *  the module whose gates are proved by `cargo test` — so the dev shell invents a
 *  plausible Instandhaltung, and the counts and the list then move. */
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

export const DEMO_SEED: FakeSeed = {
  wagen,
  partners,
  radsaetze,
  einbauten,
  events,
  templates,
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
