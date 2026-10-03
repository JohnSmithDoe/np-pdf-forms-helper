// ─── why ────────────────────────────────────────────────────────
// Both walks against the faked transport, kept as shallow as the other trains
// specs: it proves the cleaning hub lists what the scan sent, that a recognised
// file reaches the review, that the review's counts are the ones the backend
// sent, that filing ends in the batch summary rather than in an import, that a
// filed document walks to the result by type, and that the guards bounce a cold
// deep link. Whether a cell is a Fehler
// or a Deutung is decided in `trains/clean/` and proved by `cargo test`; the
// fake's `reclean_file` only echoes decisions back, so nothing here can agree
// with a second implementation of the cleaning.
//
// Two assertions carry the rule the feature is built on. A tie between two
// templates must leave „Starten“ dead until the user picks — preselecting one
// would be the guess the import refuses to make. And Weiter on the review must
// stay dead while any Fehler or Deutung is open. The import walk adds a third:
// the steps only collect answers, and the one write is `commit_document`.
//
// Locators are scoped to their own `app-page-…` element, because Ionic keeps
// the outgoing page in the DOM during a route transition (see `wizard.spec.ts`).
// ────────────────────────────────────────────────────────────────

import { expect, test, type Page } from '@playwright/test';
import {
  installFakeBackend,
  recordedCalls,
  type FakeCleanReport,
  type FakeDokument,
  type FakeScanFile,
  type FakeStaging,
} from './fake-backend';

const PLAN: FakeStaging['plan'] = {
  reader: 'headerRow',
  layout: { headerRow: 1, firstDataRow: 2 },
  columns: [
    { header: 'Wagennummer', index: 1, field: 'wagennummer' },
    { header: 'Betrag', index: 2, field: 'betrag' },
  ],
  templateId: 'builtin:werkstattauftraege',
  date1904: false,
};

const RECOGNISED: FakeScanFile = {
  path: '/eingang/auftraege.xlsx',
  name: 'auftraege.xlsx',
  status: 'erkannt',
  matches: [
    {
      templateId: 'builtin:werkstattauftraege',
      templateName: 'Werkstattaufträge',
      sheet: 'Tabelle1',
    },
  ],
  sheets: ['Tabelle1'],
};

const TIED: FakeScanFile = {
  path: '/eingang/radsaetze.xlsx',
  name: 'radsaetze.xlsx',
  status: 'mehrdeutig',
  matches: [
    { templateId: 't1', templateName: 'Radsatzblatt', sheet: 'Tabelle1' },
    {
      templateId: 'builtin:radsatz-monitoring',
      templateName: 'Radsatz-Monitoring',
      sheet: 'Tabelle1',
    },
  ],
  sheets: ['Tabelle1'],
};

const UNKNOWN: FakeScanFile = {
  path: '/eingang/rechnung.xlsx',
  name: 'rechnung.xlsx',
  status: 'unbekannt',
  matches: [],
  sheets: ['Tabelle1'],
};

const UNSUPPORTED: FakeScanFile = {
  path: '/eingang/brief.pdf',
  name: 'brief.pdf',
  status: 'nichtUnterstuetzt',
  matches: [],
  sheets: [],
  message: 'Nur Excel-Dateien (.xlsx) werden eingelesen.',
};

const CLEAN: FakeCleanReport = {
  file: '/eingang/auftraege.xlsx',
  sheet: 'Tabelle1',
  templateId: 'builtin:werkstattauftraege',
  templateName: 'Werkstattaufträge',
  plan: PLAN,
  fehler: [
    {
      row: 4,
      column: 2,
      header: 'Betrag',
      raw: '12O,50',
      message: '„12O,50“ ist kein Betrag.',
      open: true,
    },
  ],
  cards: [
    {
      column: 2,
      header: 'Betrag',
      field: 'betrag',
      reading: { kind: 'decimal', chosen: 'german', alternative: 'english' },
      reason: '„1.250“ kann 1250 oder 1,25 heißen.',
      count: 1,
      examples: [
        { row: 3, raw: '1.250', chosen: '1.250,00', alternative: '1,25' },
      ],
      confirmed: false,
    },
  ],
  formats: [
    {
      column: 1,
      header: 'Wagennummer',
      rule: 'Wagennummer in Blöcken geschrieben',
      count: 5,
      samples: [{ row: 2, raw: '218124712173', clean: '21 81 2471 217-3' }],
    },
  ],
  summary: {
    fehlerOffen: 1,
    deutungenOffen: 1,
    formatierungen: 5,
    korrigiert: 0,
  },
};

const CLEANED: FakeStaging = {
  id: 's-clean',
  file: 'auftraege.bereinigt.xlsx',
  sheet: 'Tabelle1',
  sheets: ['Tabelle1'],
  plan: PLAN,
  candidates: [],
  rows: [
    {
      row: 2,
      status: 'ready',
      cells: [
        {
          column: 1,
          field: 'wagennummer',
          raw: '21 81 2471 217-3',
          parsed: '21 81 2471 217-3',
          ok: true,
        },
      ],
      wagen: { state: 'known', id: 'w1', name: '21 81 2471 217-3' },
      werkstatt: { state: 'missing' },
      halter: { state: 'missing' },
      eigentuemer: { state: 'missing' },
      radsatz: { state: 'missing' },
      issues: [],
    },
  ],
  summary: {
    total: 1,
    ready: 1,
    needsInput: 0,
    duplicates: 0,
    rejected: 0,
    neueWagen: 0,
    neuePartner: 0,
    neueRadsaetze: 0,
  },
};

const DOKUMENT: FakeDokument = {
  id: 'd1',
  name: 'auftraege.xlsx',
  sheet: 'Tabelle1',
  templateId: 't1',
  templateName: 'Werkstattaufträge',
  plan: PLAN,
  originalHash: 'a',
  cleanedHash: 'b',
  folder: '/daten/dokumente/d1',
  original: '/daten/dokumente/d1/auftraege.xlsx',
  cleaned: '/daten/dokumente/d1/auftraege.bereinigt.xlsx',
  summary: {
    fehlerOffen: 0,
    deutungenOffen: 0,
    formatierungen: 5,
    korrigiert: 1,
  },
  bereinigtAm: '2026-10-03',
};

// One known Wagen and nothing else to ask: the defaults answer every group, so
// the walk can be proved end to end without driving a radio button.
const STAGED_DOKUMENT: FakeStaging = {
  ...CLEANED,
  entities: {
    partner: [],
    wagen: [
      {
        key: '21 81 2471 217-3',
        kind: 'wagen',
        spellings: ['21 81 2471 217-3'],
        resolution: { state: 'known', id: 'w1', name: '21 81 2471 217-3' },
        rows: [2],
        changes: [],
      },
    ],
    radsaetze: [],
  },
};

const step = (page: Page, name: string) => page.locator(`app-page-${name}`);

test.describe('Bereinigen', () => {
  test('die Übersicht listet jede gefundene Datei mit ihrem Status', async ({
    page,
  }) => {
    await installFakeBackend(page, {
      scan: [RECOGNISED, TIED, UNKNOWN, UNSUPPORTED],
    });
    await page.goto('/#/trains/clean');

    const hub = step(page, 'clean-hub');
    await hub.getByTestId('intake-drop').click();

    await expect(hub.getByTestId('intake-row-name')).toHaveText([
      'auftraege.xlsx',
      'radsaetze.xlsx',
      'rechnung.xlsx',
      'brief.pdf',
    ]);
    await expect(hub.getByTestId('intake-row-status')).toHaveText([
      'erkannt',
      'mehrdeutig',
      'unbekannt',
      'nicht unterstützt',
    ]);
    await expect(hub.getByText(UNSUPPORTED.message ?? '')).toBeVisible();
    // A skipped file offers no choice; the other three do.
    await expect(hub.getByTestId('intake-row-pick')).toHaveCount(3);
    // The tie is left for the user, so the walk cannot start yet.
    await expect(page.getByTestId('intake-start')).toHaveAttribute(
      'disabled',
      ''
    );
    await expect(page.getByTestId('intake-unchosen')).toContainText(
      '1 Datei(en) brauchen eine Auswahl'
    );
    await expect(hub.getByTestId('phase-chip')).toContainText('Bereinigen');
  });

  test('eine erkannte Datei führt zur Bereinigung mit ihren Zählern', async ({
    page,
  }) => {
    await installFakeBackend(page, {
      scan: [RECOGNISED, UNSUPPORTED],
      clean: CLEAN,
    });
    await page.goto('/#/trains/clean');

    await step(page, 'clean-hub').getByTestId('intake-drop').click();
    await page.getByTestId('intake-start').click();

    await expect(page).toHaveURL(/#\/trains\/clean\/file$/);
    const clean = step(page, 'clean-file');
    const original = clean.getByTestId('clean-open-original');
    await expect(original).toContainText('auftraege.xlsx');
    await original.click();
    await expect
      .poll(async () =>
        (await recordedCalls(page)).find((call) => call.command === 'open_file')
      )
      .toEqual({
        command: 'open_file',
        args: { filename: '/eingang/auftraege.xlsx' },
      });
    await expect(clean.getByTestId('clean-summary-fehler')).toContainText(
      '1 Fehler offen'
    );
    await expect(clean.getByTestId('clean-summary-deutungen')).toContainText(
      '1 Deutungen zu bestätigen'
    );
    await expect(clean.getByTestId('clean-summary-formate')).toContainText(
      '5 Formatierungen'
    );
    await expect(clean.getByTestId('fehler-row')).toHaveCount(1);
    await expect(clean.getByTestId('deutung-card')).toHaveCount(1);
    await expect(clean.getByTestId('format-group')).toHaveCount(1);
    await expect(
      clean.getByRole('button', { name: 'Bereinigen abschließen' })
    ).toBeDisabled();
  });

  // Filing is the cleaning's end: the batch summary, not an import preview.
  test('erst wenn nichts mehr offen ist, wird die Datei abgelegt und zusammengefasst', async ({
    page,
  }) => {
    await installFakeBackend(page, { scan: [RECOGNISED], clean: CLEAN });
    await page.goto('/#/trains/clean');
    await step(page, 'clean-hub').getByTestId('intake-drop').click();
    await page.getByTestId('intake-start').click();

    const clean = step(page, 'clean-file');
    await clean.getByTestId('fehler-empty').click();
    await clean.getByTestId('deutung-confirm').click();
    await expect(clean.getByTestId('clean-summary-fehler')).toContainText(
      '0 Fehler offen'
    );
    await clean.getByRole('button', { name: 'Bereinigen abschließen' }).click();

    await expect(page).toHaveURL(/#\/trains\/clean\/summary$/);
    const batch = step(page, 'clean-batch');
    await expect(batch.getByTestId('batch-row')).toHaveCount(1);
    await expect(batch.getByTestId('batch-row')).toContainText(
      '„auftraege.xlsx“ wurde bereinigt'
    );
    await expect(batch.getByTestId('batch-import')).toBeVisible();
    await expect(page.locator('ion-modal')).toHaveCount(0);
    const calls = (await recordedCalls(page)).map((call) => call.command);
    expect(calls).not.toContain('commit_document');
  });

  // An unrecognised file is staged by PATH and handed to the template mapper,
  // which must open on its mapping step rather than ask for the file again.
  test('eine unbekannte Datei landet beim Anlegen einer Vorlage', async ({
    page,
  }) => {
    await installFakeBackend(page, { scan: [UNKNOWN], staging: CLEANED });
    await page.goto('/#/trains/clean');
    await step(page, 'clean-hub').getByTestId('intake-drop').click();
    await page.getByTestId('intake-start').click();

    await expect(page).toHaveURL(/#\/trains\/clean\/template$/);
    const mapper = step(page, 'template-mapper');
    await expect(mapper.getByTestId('mapper-header').first()).toHaveText(
      'Wagennummer'
    );
    await expect(mapper.getByTestId('import-pick')).toHaveCount(0);
    await expect(mapper.getByTestId('template-save')).toBeVisible();
  });

  test('ohne bereinigbare Datei gibt es keinen toten Start-Knopf', async ({
    page,
  }) => {
    await installFakeBackend(page, { scan: [UNSUPPORTED] });
    await page.goto('/#/trains/clean');
    await step(page, 'clean-hub').getByTestId('intake-drop').click();

    await expect(page.getByTestId('intake-done')).toBeVisible();
    await expect(page.getByTestId('intake-start')).toHaveCount(0);
  });

  test('ein kalter Direktaufruf der Bereinigung landet in der Übersicht', async ({
    page,
  }) => {
    await installFakeBackend(page);
    await page.goto('/#/trains/clean/file');

    await expect(page).toHaveURL(/#\/trains\/clean$/);
    await expect(
      step(page, 'clean-hub').getByTestId('intake-drop')
    ).toBeVisible();
  });
});

test.describe('Import ins Schattensystem', () => {
  test('ein Dokument wird typweise bis zum Ergebnis importiert', async ({
    page,
  }) => {
    await installFakeBackend(page, {
      dokumente: [DOKUMENT],
      document: STAGED_DOKUMENT,
    });
    await page.goto('/#/trains/documents');
    await step(page, 'document-list').getByTestId('documents-import').click();

    for (const name of ['partners', 'wagons', 'wheelsets']) {
      await expect(page).toHaveURL(new RegExp(`#/trains/import/${name}$`));
      const entities = step(page, 'import-entities').last();
      await expect(entities.getByTestId('wizard-phase')).toContainText(
        'Import ins Schattensystem'
      );
      await entities.getByRole('button', { name: 'Weiter' }).click();
    }

    const entries = step(page, 'import-entries');
    await expect(entries.getByTestId('import-entry')).toHaveCount(1);
    await entries
      .getByRole('button', { name: '1 Zeile(n) übernehmen' })
      .click();

    const summary = step(page, 'import-summary');
    await expect(summary.getByTestId('import-plan-rows')).toContainText(
      '1 von 1'
    );
    const before = (await recordedCalls(page)).map((call) => call.command);
    expect(before).not.toContain('commit_document');
    await summary.getByRole('button', { name: 'Importieren' }).click();

    await expect(
      step(page, 'import-result').getByRole('heading', {
        name: 'Import wurde erfolgreich übernommen',
      })
    ).toBeVisible();
    const commit = (await recordedCalls(page)).find(
      (call) => call.command === 'commit_document'
    );
    expect(commit?.args).toMatchObject({
      decisions: {
        wagen: [
          { key: '21 81 2471 217-3', decision: { action: 'use', id: 'w1' } },
        ],
        rows: [2],
      },
    });
  });

  test('ein importiertes Dokument bietet keinen Import mehr an', async ({
    page,
  }) => {
    await installFakeBackend(page, {
      dokumente: [{ ...DOKUMENT, importiertAm: '2026-10-03' }],
    });
    await page.goto('/#/trains/documents');
    const list = step(page, 'document-list');
    await expect(list.getByTestId('documents-row')).toContainText(
      'importiert am 2026-10-03'
    );
    await expect(list.getByTestId('documents-import')).toHaveCount(0);
  });

  test('ein kalter Direktaufruf eines Import-Schritts landet bei den Dokumenten', async ({
    page,
  }) => {
    await installFakeBackend(page);
    await page.goto('/#/trains/import/partners');
    await expect(page).toHaveURL(/#\/trains\/documents$/);
  });
});
