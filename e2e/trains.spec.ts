// ─── why ────────────────────────────────────────────────────────
// The trains screens against the faked transport, kept DELIBERATELY SHALLOW.
//
// What this layer proves is that the app reaches the right screens and renders
// what the backend sent. It does not drive select popovers, alert inputs or
// anything inside an Ionic shadow root: those assert Ionic's internals, break on
// its upgrades, and say nothing about this app.
//
// The parsing, the resolution and the commit gates are proved by `cargo test`,
// where correctness is a property of bytes. Reproducing any of it here would be
// a second implementation that agrees with itself and drifts from Rust.
//
// The seed supplies a staged preview shaped exactly as `trains::model`
// serialises one, and the assertions are about what appears on screen.
// ────────────────────────────────────────────────────────────────

import { expect, test, type Page } from '@playwright/test';
import {
  installFakeBackend,
  type FakeSeed,
  type FakeStaging,
} from './fake-backend';

const STAGING: FakeStaging = {
  id: 's1',
  file: 'monat.xlsx',
  sheet: 'Tabelle1',
  sheets: ['Tabelle1'],
  plan: {
    reader: 'headerRow',
    layout: { headerRow: 1, firstDataRow: 2 },
    columns: [
      { header: 'Wagennummer', index: 1, field: 'wagennummer' },
      { header: 'Datum', index: 2, field: 'datum' },
    ],
    date1904: false,
  },
  candidates: [
    {
      reader: 'headerRow',
      readerLabel: 'Kopfzeile mit Datenzeilen',
      score: 78,
      reason: 'Kopfzeile in Zeile 1: 2 Spaltentitel, 2 Datenzeile(n) darunter.',
      hint: { headerRow: 1, firstDataRow: 2 },
    },
  ],
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
        {
          column: 2,
          field: 'datum',
          raw: '31.12.2025',
          parsed: '31.12.2025',
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
    {
      row: 3,
      status: 'rejected',
      cells: [
        {
          column: 1,
          field: 'wagennummer',
          raw: 'kein Wagen',
          parsed: '',
          ok: false,
        },
        {
          column: 2,
          field: 'datum',
          raw: '01.01.2026',
          parsed: '01.01.2026',
          ok: true,
        },
      ],
      wagen: { state: 'missing' },
      werkstatt: { state: 'missing' },
      halter: { state: 'missing' },
      eigentuemer: { state: 'missing' },
      radsatz: { state: 'missing' },
      issues: [
        {
          row: 3,
          column: 'Wagennummer',
          raw: 'kein Wagen',
          message:
            '„kein Wagen“ ist keine gültige Wagennummer (12 Ziffern erwartet, 0 gefunden).',
          severity: 'fehler',
        },
      ],
    },
  ],
  summary: {
    total: 2,
    ready: 1,
    needsInput: 0,
    duplicates: 0,
    rejected: 1,
    neueWagen: 0,
    neuePartner: 0,
    neueRadsaetze: 0,
  },
};

async function start(page: Page, seed: FakeSeed = {}): Promise<void> {
  await installFakeBackend(page, seed);
  await page.goto('/#/trains/clean/template');
}

test.describe('Zug-Import', () => {
  test('zeigt den Leerzustand, bis eine Datei gewählt wurde', async ({
    page,
  }) => {
    await start(page);
    await expect(page.getByTestId('import-pick')).toBeVisible();
    await expect(page.getByTestId('template-save')).toHaveCount(0);
  });

  test('ein abgebrochener Datei-Dialog ist kein Fehler', async ({ page }) => {
    await start(page, { staging: null });
    await page.getByTestId('import-pick').click();
    await expect(page.getByTestId('import-pick')).toBeVisible();
  });

  test('zeigt je Spalte den Titel und Beispielwerte aus der Datei', async ({
    page,
  }) => {
    await start(page, { staging: STAGING });
    await page.getByTestId('import-pick').click();

    await expect(page.getByTestId('mapper-header').first()).toHaveText(
      'Wagennummer'
    );
    await expect(page.getByTestId('mapper-sample').first()).toHaveText(
      '21 81 2471 217-3'
    );
  });

  // The mapper's only exit is a template: nothing here commits rows.
  test('die Zuordnung endet in einer Vorlage, nicht in einem Import', async ({
    page,
  }) => {
    await start(page, { staging: STAGING });
    await page.getByTestId('import-pick').click();

    await expect(page.getByTestId('template-save')).toBeEnabled();
    await expect(page.getByTestId('import-commit')).toHaveCount(0);
  });

  test('ein Export meldet sich mit seinem Ordner', async ({ page }) => {
    await installFakeBackend(page);
    await page.goto('/#/trains');
    await page.getByTestId('trains-export').click();
    await expect(
      page.getByText('Export wurde erfolgreich erstellt')
    ).toBeVisible();
  });
});

test.describe('Master-Datei', () => {
  const master: FakeSeed = {
    templates: [
      {
        id: 't-telematik',
        name: 'Telematik-Export',
        plan: STAGING.plan,
        builtin: false,
        createdAt: '2026-10-03',
      },
    ],
    master: {
      file: 'C:/Daten/Übersicht.xlsx',
      bindings: [
        {
          sheet: 'Telematik',
          templateId: 't-telematik',
          mode: 'snapshot',
          aliases: [],
        },
      ],
    },
    masterSheets: [
      { name: 'Überblick', headers: ['Wagennummer'] },
      { name: 'Telematik', headers: ['Asset', 'Stadt'] },
    ],
  };

  test('zeigt die Datei und ihre Zuordnungen', async ({ page }) => {
    await installFakeBackend(page, master);
    await page.goto('/#/trains/master');
    const screen = page.locator('app-page-trains-master');
    await expect(screen.getByTestId('master-file')).toHaveText(
      'C:/Daten/Übersicht.xlsx'
    );
    await expect(screen.getByTestId('master-binding')).toHaveCount(1);
    await expect(
      screen.getByTestId('master-binding').getByText('Telematik').first()
    ).toBeVisible();
  });

  test('ohne Datei lässt sich nichts aktualisieren', async ({ page }) => {
    await installFakeBackend(page);
    await page.goto('/#/trains/master');
    const screen = page.locator('app-page-trains-master');
    await expect(
      screen.getByText('Noch keine Master-Datei gewählt')
    ).toBeVisible();
    await expect(screen.getByTestId('master-refresh')).toHaveAttribute(
      'disabled',
      ''
    );
  });

  test('die Aktualisierung meldet sich mit ihrem Bericht', async ({ page }) => {
    await installFakeBackend(page, master);
    await page.goto('/#/trains/master');
    await page
      .locator('app-page-trains-master')
      .getByTestId('master-refresh')
      .click();
    await expect(
      page.getByText('Master-Datei wurde aktualisiert')
    ).toBeVisible();
  });
});

test.describe('Zug-Listen', () => {
  const wagen = [
    {
      id: 'w1',
      nummer: '218124712173',
      halterId: 'p1',
      createdAt: '2026-08-16',
    },
    { id: 'w2', nummer: '318047401234', createdAt: '2026-08-16' },
  ];
  const partners = [
    {
      id: 'p1',
      rollen: ['halter' as const, 'werkstatt' as const],
      name: 'Müller GmbH',
      matchKey: 'mueller',
      aliases: ['fa mueller'],
      createdAt: '2026-08-16',
    },
  ];

  // The Schattensystem setting decides the spelling; compact is the default.
  test('die Wagenliste zeigt die Nummer in der eingestellten Schreibweise', async ({
    page,
  }) => {
    await installFakeBackend(page, { wagen, partners });
    await page.goto('/#/trains/wagen');
    await expect(page.getByTestId('list-row-title').first()).toHaveText(
      '218124712173'
    );
  });

  test('gruppiert eingestellt, zeigt die Liste die Gruppierung', async ({
    page,
  }) => {
    await installFakeBackend(page, {
      wagen,
      partners,
      settings: { wagennummer: 'grouped' },
    });
    await page.goto('/#/trains/wagen');
    await expect(page.getByTestId('list-row-title').first()).toHaveText(
      '21 81 2471 217-3'
    );
  });

  test('die Suche findet einen Wagen über die reinen Ziffern', async ({
    page,
  }) => {
    await installFakeBackend(page, { wagen, partners });
    await page.goto('/#/trains/wagen');
    await page.getByTestId('list-search').locator('input').fill('3180');
    await expect(page.getByTestId('list-row')).toHaveCount(1);
  });

  test('eine leere Liste sagt, woher Einträge kommen', async ({ page }) => {
    await installFakeBackend(page, { wagen: [], partners: [] });
    await page.goto('/#/trains/wagen');
    await expect(page.getByTestId('list-empty')).toContainText('Import');
  });

  test('Werkstätten und Eigentümer sind dieselbe Liste mit anderer Rolle', async ({
    page,
  }) => {
    await installFakeBackend(page, { wagen, partners });
    await page.goto('/#/trains/werkstaetten');
    await expect(page.getByTestId('list-row-title').first()).toHaveText(
      'Müller GmbH'
    );
    await expect(page.getByText('1 bekannte Schreibweise(n)')).toBeVisible();
  });
});

test.describe('Radsätze', () => {
  const radsaetze = [
    {
      id: 'r1',
      nummer: 'RS4711',
      matchKey: 'RS4711',
      aliases: [{ matchKey: 'RS4711', partnerId: 'p1' }],
      createdAt: '2026-08-16',
    },
    {
      id: 'r2',
      nummer: 'RS0815',
      matchKey: 'RS0815',
      aliases: [{ matchKey: 'RS0815', partnerId: 'p1' }],
      createdAt: '2026-08-16',
    },
  ];
  const einbauten = [
    {
      id: 'm1',
      radsatzId: 'r1',
      wagenId: 'w1',
      position: '1',
      eingebautAm: '2025-03-01',
      source: {
        file: 'a.xlsx',
        sheet: 'Tabelle1',
        row: 2,
        importedAt: '2026-08-16',
      },
    },
  ];
  const wagen = [{ id: 'w1', nummer: '218124712173', createdAt: '2026-08-16' }];

  test('zeigt, in welchem Wagen ein Radsatz gerade steckt', async ({
    page,
  }) => {
    await installFakeBackend(page, {
      wagen,
      radsaetze,
      einbauten,
      settings: { wagennummer: 'grouped' },
    });
    await page.goto('/#/trains/radsaetze');
    await expect(page.getByTestId('list-row-title')).toHaveText([
      'RS0815',
      'RS4711',
    ]);
    await expect(page.getByText('eingebaut in 21 81 2471 217-3')).toBeVisible();
  });

  test('ein Wagen zeigt seine eingebauten Radsätze, ohne Position zuletzt', async ({
    page,
  }) => {
    // A master radsatz sheet without positions adds a second open Einbau.
    const unplaced = {
      id: 'm2',
      radsatzId: 'r2',
      wagenId: 'w1',
      eingebautAm: '2026-06-19',
      source: {
        file: 'Übersicht.xlsx',
        sheet: 'Radsätze',
        row: 7,
        importedAt: '2026-10-04',
      },
    };
    await installFakeBackend(page, {
      wagen,
      radsaetze,
      einbauten: [unplaced, ...einbauten],
    });
    await page.goto('/#/trains/wagen');
    await expect(page.getByTestId('wagen-fitted-count')).toHaveText(
      '2 Radsätze eingebaut'
    );
    await expect(page.getByTestId('wagen-fitted')).toHaveCount(0);

    await page.getByTestId('list-row-title').click();
    const fitted = page.getByTestId('wagen-fitted');
    await expect(fitted).toHaveCount(2);
    await expect(fitted.first()).toContainText('Position 1 · RS4711');
    await expect(fitted.first()).toContainText('Tabelle1, Zeile 2');
    await expect(fitted.last()).toContainText('ohne Position · RS0815');
    await expect(fitted.last()).toContainText('Radsätze, Zeile 7');
  });

  test('ein Radsatz ohne offenen Einbau gilt als ausgebaut', async ({
    page,
  }) => {
    await installFakeBackend(page, { wagen, radsaetze, einbauten });
    await page.goto('/#/trains/radsaetze');
    await expect(page.getByText('ausgebaut')).toBeVisible();
  });
});
