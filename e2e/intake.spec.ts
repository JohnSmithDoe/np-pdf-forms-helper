// ─── why ────────────────────────────────────────────────────────
// The guided import against the faked transport, kept as shallow as the other
// trains specs: it proves the hub lists what the scan sent, that a recognised
// file reaches the review, that the review's counts are the ones the backend
// sent, and that the guards bounce a cold deep link. Whether a cell is a Fehler
// or a Deutung is decided in `trains/clean/` and proved by `cargo test`; the
// fake's `reclean_file` only echoes decisions back, so nothing here can agree
// with a second implementation of the cleaning.
//
// Two assertions carry the rule the feature is built on. A tie between two
// templates must leave „Starten“ dead until the user picks — preselecting one
// would be the guess the import refuses to make. And Weiter on the review must
// stay dead while any Fehler or Deutung is open.
//
// Locators are scoped to their own `app-page-…` element, because Ionic keeps
// the outgoing page in the DOM during a route transition (see `wizard.spec.ts`).
// ────────────────────────────────────────────────────────────────

import { expect, test, type Page } from '@playwright/test';
import {
  installFakeBackend,
  type FakeCleanReport,
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
  file: 'auftraege.xlsx',
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

const step = (page: Page, name: string) => page.locator(`app-page-${name}`);

test.describe('Geführter Import', () => {
  test('die Übersicht listet jede gefundene Datei mit ihrem Status', async ({
    page,
  }) => {
    await installFakeBackend(page, {
      scan: [RECOGNISED, TIED, UNKNOWN, UNSUPPORTED],
    });
    await page.goto('/#/trains/import');

    const hub = step(page, 'import-hub');
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
  });

  test('eine erkannte Datei führt zur Bereinigung mit ihren Zählern', async ({
    page,
  }) => {
    await installFakeBackend(page, {
      scan: [RECOGNISED, UNSUPPORTED],
      clean: CLEAN,
    });
    await page.goto('/#/trains/import');

    await step(page, 'import-hub').getByTestId('intake-drop').click();
    await page.getByTestId('intake-start').click();

    await expect(page).toHaveURL(/#\/trains\/import\/guided\/clean$/);
    const clean = step(page, 'guided-clean');
    await expect(clean.getByTestId('clean-file')).toHaveText('auftraege.xlsx');
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
      clean.getByRole('button', { name: 'Bereinigte Datei schreiben' })
    ).toBeDisabled();
  });

  test('erst wenn nichts mehr offen ist, geht es über die Vorschau zum Ergebnis', async ({
    page,
  }) => {
    await installFakeBackend(page, {
      scan: [RECOGNISED],
      clean: CLEAN,
      cleaned: CLEANED,
    });
    await page.goto('/#/trains/import');
    await step(page, 'import-hub').getByTestId('intake-drop').click();
    await page.getByTestId('intake-start').click();

    const clean = step(page, 'guided-clean');
    await clean.getByTestId('fehler-empty').click();
    await clean.getByTestId('deutung-confirm').click();
    await expect(clean.getByTestId('clean-summary-fehler')).toContainText(
      '0 Fehler offen'
    );
    await expect(clean.getByTestId('deutung-confirmed')).toBeVisible();
    await clean
      .getByRole('button', { name: 'Bereinigte Datei schreiben' })
      .click();

    const preview = step(page, 'guided-preview');
    await expect(preview.getByTestId('preview-row')).toHaveCount(1);
    await preview
      .getByRole('button', { name: '1 Zeile(n) übernehmen' })
      .click();

    const result = step(page, 'guided-result');
    await expect(
      result.getByRole('heading', {
        name: 'Bereinigte Datei wurde geschrieben',
      })
    ).toBeVisible();
    await expect(
      result.getByRole('heading', {
        name: 'Import wurde erfolgreich übernommen',
      })
    ).toBeVisible();
    await expect(page.locator('ion-modal')).toHaveCount(0);
  });

  // An unrecognised file is staged by PATH and handed to the manual import,
  // which must open on its mapping step rather than ask for the file again.
  test('eine unbekannte Datei landet in der Zuordnung des Imports von Hand', async ({
    page,
  }) => {
    await installFakeBackend(page, { scan: [UNKNOWN], staging: CLEANED });
    await page.goto('/#/trains/import');
    await step(page, 'import-hub').getByTestId('intake-drop').click();
    await page.getByTestId('intake-start').click();

    await expect(page).toHaveURL(/#\/trains\/import\/manual$/);
    const manual = step(page, 'import');
    await expect(manual.getByTestId('mapper-header').first()).toHaveText(
      'Wagennummer'
    );
    await expect(manual.getByTestId('import-pick')).toHaveCount(0);
  });

  // Choosing „Nicht importieren“ means driving an ion-select popover, which this
  // layer does not do; what it can prove is the end state a skip leads to —
  // nothing left in the walk is a sentence, not a dead button.
  test('ohne importierbare Datei gibt es keinen toten Start-Knopf', async ({
    page,
  }) => {
    await installFakeBackend(page, { scan: [UNSUPPORTED] });
    await page.goto('/#/trains/import');
    await step(page, 'import-hub').getByTestId('intake-drop').click();

    await expect(page.getByTestId('intake-done')).toBeVisible();
    await expect(page.getByTestId('intake-start')).toHaveCount(0);
  });

  test('ein kalter Direktaufruf der Vorschau landet in der Übersicht', async ({
    page,
  }) => {
    await installFakeBackend(page);
    await page.goto('/#/trains/import/guided/preview');

    await expect(page).toHaveURL(/#\/trains\/import$/);
    await expect(
      step(page, 'import-hub').getByTestId('intake-drop')
    ).toBeVisible();
  });
});
