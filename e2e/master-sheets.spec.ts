// ─── why ────────────────────────────────────────────────────────
// The master sheet views and the open-run banner, kept as shallow as the rest.
//
// A sheet view is built WHOLE in Rust (`master/sheet_view.rs`, backend for
// frontend), so what this proves is only that the page asks for the routed
// sheet and renders what came back: the header row with its column letters,
// the marker on a column no entity fills, the rows, the key search. Which
// entity a row is and how a cell is spelled are `cargo test`'s.
// ────────────────────────────────────────────────────────────────

import { expect, test, type Page } from '@playwright/test';
import {
  clientMaster,
  installFakeBackend,
  recordedCalls,
  type FakeMasterSettings,
  type FakeSeed,
} from './fake-backend';

const MASTER: FakeMasterSettings = {
  file: 'C:\\Daten\\Übersicht.xlsx',
  bindings: [
    {
      sheet: 'Alle Wagen',
      templateId: '',
      kind: 'wagenliste',
      aliases: [],
    },
    {
      sheet: 'Radsätze',
      templateId: '',
      kind: 'radsatzEinbau',
      aliases: [],
    },
  ],
};

const SEED: FakeSeed = {
  master: MASTER,
  masterSheets: [
    { name: 'Alle Wagen', headers: ['Wagennummer', 'Status'] },
    { name: 'Radsätze', headers: ['an_wagen', 'radsatz'] },
  ],
  masterSheetViews: {
    'Alle Wagen': {
      sheet: 'Alle Wagen',
      kind: 'wagenliste',
      rowLabel: 'Wagen',
      columns: [
        { index: 1, header: 'Wagennummer', filled: true },
        { index: 2, header: 'Status', filled: false },
      ],
      rows: [
        {
          key: '218124712173',
          cells: ['218124712173', ''],
          source: 'Alle Wagen, Zeile 2',
        },
        {
          key: '338080123452',
          cells: ['338080123452', ''],
          source: 'Alle Wagen, Zeile 3',
        },
      ],
    },
  },
};

const sheetPage = (page: Page) => page.locator('app-page-trains-master-sheet');

test.describe('Master-Blattansichten', () => {
  test('ein Blatt zeigt seine Spalten und die Zeilen aus dem Schattensystem', async ({
    page,
  }) => {
    await installFakeBackend(page, SEED);
    await page.goto(
      `/#/trains/master/sheets/${encodeURIComponent('Alle Wagen')}`
    );
    const view = sheetPage(page);

    const columns = view.getByTestId('sheet-column');
    await expect(columns).toHaveCount(2);
    await expect(columns.first()).toContainText('A');
    await expect(columns.first()).toContainText('Wagennummer');
    await expect(columns.first()).not.toContainText('nicht im Schattensystem');
    await expect(columns.nth(1)).toContainText('nicht im Schattensystem');

    await expect(view.getByTestId('sheet-row')).toHaveCount(2);
    await expect(view.getByTestId('sheet-count')).toContainText('2 Wagen');
    await expect(view.getByTestId('sheet-row').first()).toContainText(
      'Alle Wagen, Zeile 2'
    );

    const calls = await recordedCalls(page);
    expect(
      calls.find((call) => call.command === 'get_master_sheet')?.args
    ).toMatchObject({ sheet: 'Alle Wagen' });
  });

  test('die Suche filtert auf die Wagennummer', async ({ page }) => {
    await installFakeBackend(page, SEED);
    await page.goto(
      `/#/trains/master/sheets/${encodeURIComponent('Alle Wagen')}`
    );
    const view = sheetPage(page);
    await expect(view.getByTestId('sheet-row')).toHaveCount(2);

    await view.getByTestId('sheet-search').locator('input').fill('3380');

    await expect(view.getByTestId('sheet-row')).toHaveCount(1);
    await expect(view.getByTestId('sheet-row')).toContainText('338080123452');
  });

  test('ein Blatt ohne Daten zeigt nur die Kopfzeile', async ({ page }) => {
    await installFakeBackend(page, SEED);
    await page.goto(
      `/#/trains/master/sheets/${encodeURIComponent('Radsätze')}`
    );
    const view = sheetPage(page);

    await expect(view.getByTestId('sheet-column')).toHaveCount(2);
    await expect(view.getByTestId('sheet-row')).toHaveCount(0);
    await expect(view).toContainText('noch keine Zeilen');
  });

  test('„Ansehen“ auf der Master-Seite öffnet die Ansicht des Blatts', async ({
    page,
  }) => {
    await installFakeBackend(page, SEED);
    await page.goto('/#/trains/master');
    await page
      .locator('app-page-trains-master')
      .getByTestId('master-view-sheet')
      .first()
      .click();

    await expect(page).toHaveURL(/#\/trains\/master\/sheets\/Alle%20Wagen$/);
    await expect(sheetPage(page).getByTestId('sheet-row')).toHaveCount(2);
  });
});

test.describe('Master-Zuordnung', () => {
  // What recognition produces is `kinds::recognise`'s; the fake only applies
  // the seeded defaults, so these tests prove the page's half: the pick and the
  // reset show what came back, and a hand edit is marked as one.
  const DEFAULTS: FakeMasterSettings['bindings'] = [
    {
      sheet: 'Alle Wagen',
      templateId: '',
      kind: 'wagenliste',
      aliases: [],
      auto: true,
    },
    {
      sheet: 'Notizen',
      templateId: '',
      aliases: [],
      auto: true,
    },
  ];
  const masterPage = (page: Page) => page.locator('app-page-trains-master');

  test('eine übernommene Master-Datei ordnet jedes Blatt von selbst zu', async ({
    page,
  }) => {
    await installFakeBackend(page, {
      masterFile: clientMaster('C:\\Daten\\Übersicht.xlsx'),
      masterDefaults: DEFAULTS,
      masterSheets: [
        { name: 'Alle Wagen', headers: ['Wagennummer'] },
        { name: 'Notizen', headers: ['Text'] },
      ],
    });
    await page.goto('/#/trains/master');
    await expect(masterPage(page).getByTestId('master-file')).toContainText(
      'Übersicht.xlsx'
    );

    const rows = masterPage(page).getByTestId('master-binding');
    await expect(rows).toHaveCount(2);
    await expect(rows.first()).toContainText('importiert: Wagen');
    await expect(rows.last()).toContainText('nur Ansicht');
  });

  test('eine Änderung von Hand wird als solche gespeichert', async ({
    page,
  }) => {
    await installFakeBackend(page, {
      master: { file: 'C:\\Daten\\Übersicht.xlsx', bindings: DEFAULTS },
      masterSheets: [{ name: 'Alle Wagen', headers: ['Wagennummer'] }],
    });
    await page.goto('/#/trains/master');
    const first = masterPage(page).getByTestId('master-binding').first();
    await first.getByTestId('master-edit').click();
    await masterPage(page)
      .getByTestId('master-binding-details')
      .getByRole('button', { name: 'Umbenannte Spalte zuordnen' })
      .click();

    await expect(first).toContainText('von Hand angepasst');
    const save = (await recordedCalls(page)).find(
      (call) => call.command === 'save_master'
    );
    expect(save?.args).toMatchObject({
      settings: {
        bindings: [{ sheet: 'Alle Wagen', auto: false }, { auto: true }],
      },
    });
  });

  test('„Standardzuordnung“ stellt die Zuordnung nach Kopfzeile wieder her', async ({
    page,
  }) => {
    await installFakeBackend(page, {
      master: {
        file: 'C:\\Daten\\Übersicht.xlsx',
        bindings: [{ ...DEFAULTS[1]!, kind: 'wagenliste', auto: false }],
      },
      masterDefaults: DEFAULTS,
      masterSheets: [
        { name: 'Alle Wagen', headers: ['Wagennummer'] },
        { name: 'Notizen', headers: ['Text'] },
      ],
    });
    await page.goto('/#/trains/master');
    await masterPage(page).getByTestId('master-reset').click();
    // A hand edit would be lost, so the reset asks first.
    await page.getByRole('button', { name: 'Bestätigen' }).click();

    const rows = masterPage(page).getByTestId('master-binding');
    await expect(rows).toHaveCount(2);
    await expect(rows.last()).toContainText('nur Ansicht');
    await expect(rows.last()).not.toContainText('von Hand angepasst');
  });
});

test.describe('Master-Import auf dem Dashboard', () => {
  test('ein offener Lauf steht als Hinweis über den Kacheln', async ({
    page,
  }) => {
    await installFakeBackend(page, {
      master: {
        ...MASTER,
        importRun: {
          startedAt: '2026-10-04',
          sheets: ['Alle Wagen', 'Radsätze'],
          done: ['Alle Wagen'],
        },
      },
    });
    // The banner is behind the master feature toggle.
    await page.addInitScript(() => localStorage.setItem('npdh.master', 'on'));
    await page.goto('/#/trains');

    await expect(
      page
        .locator('app-page-trains-dashboard')
        .getByTestId('dashboard-master-open')
    ).toContainText('Radsätze');
  });

  test('ohne offenen Lauf gibt es keinen Hinweis', async ({ page }) => {
    await installFakeBackend(page, {
      master: {
        ...MASTER,
        importRun: {
          startedAt: '2026-10-04',
          sheets: ['Alle Wagen'],
          done: ['Alle Wagen'],
        },
      },
    });
    await page.goto('/#/trains');

    const dashboard = page.locator('app-page-trains-dashboard');
    await expect(dashboard.getByRole('heading').first()).toBeVisible();
    await expect(dashboard.getByTestId('dashboard-master-open')).toHaveCount(0);
  });
});
