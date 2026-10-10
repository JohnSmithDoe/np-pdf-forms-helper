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
  recordedCalls,
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

  test('der Einstieg „Dokumente“ führt zu Bereinigen, Dokumenten und Master-Datei', async ({
    page,
  }) => {
    await installFakeBackend(page, { master: { file: 'data/Master.xlsx' } });
    await page.goto('/');

    const start = page.locator('app-page-trains-start');
    for (const name of ['Bereinigen', 'Dokumente', 'Master-Datei']) {
      await expect(
        start.getByRole('button', { name, exact: true })
      ).toBeVisible();
    }
    await expect(start.getByText('Master.xlsx', { exact: true })).toBeVisible();
    // The ERP's entities are not on this hub.
    await expect(
      start.getByRole('button', { name: 'Wagen', exact: true })
    ).toHaveCount(0);
  });

  test('ohne npdh.full zeigt das ERP die Entitäten, aber keine Vorlagen und Einstellungen', async ({
    page,
  }) => {
    await installFakeBackend(page);
    await page.goto('/#/trains/erp');

    const dashboard = page.locator('app-page-trains-dashboard');
    // Cleaning and the master file live on the „Dokumente“ hub.
    await expect(
      dashboard.getByRole('button', { name: 'Bereinigen', exact: true })
    ).toHaveCount(0);
    await expect(dashboard.locator('ion-card')).toHaveCount(9);
    await expect(
      dashboard.getByRole('button', { name: 'Dokumente', exact: true })
    ).toBeVisible();
    for (const name of ['Wagen', 'Telematik', 'Radsätze', 'Master-Import']) {
      await expect(
        dashboard.getByRole('button', { name, exact: true })
      ).toBeVisible();
    }
    await expect(
      dashboard.getByRole('button', { name: 'Vorlagen', exact: true })
    ).toHaveCount(0);
    await expect(page.getByTestId('trains-export')).toHaveCount(0);
  });

  test('ein Export meldet sich mit seinem Ordner', async ({ page }) => {
    await installFakeBackend(page);
    // „Export erstellen“ is behind the full-app toggle.
    await page.addInitScript(() => localStorage.setItem('npdh.full', 'on'));
    await page.goto('/#/trains/erp');
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
    // The client master's current version, by name — not a path picked here.
    await expect(screen.getByTestId('master-file')).toContainText(
      'Übersicht.xlsx'
    );
    await expect(screen.getByRole('link', { name: 'Fassungen' })).toBeVisible();
    await expect(screen.getByTestId('master-binding')).toHaveCount(1);
    await expect(
      screen.getByTestId('master-binding').getByText('Telematik').first()
    ).toBeVisible();
  });

  // One question — it still empties — and then the report; the automatic
  // answers themselves are cargo test's (`master::import_all`).
  test('„Alles importieren“ fragt einmal und zeigt danach den Bericht', async ({
    page,
  }) => {
    await installFakeBackend(page, {
      ...master,
      master: {
        file: 'C:/Daten/Übersicht.xlsx',
        bindings: [
          {
            sheet: 'Überblick',
            templateId: '',
            kind: 'wagenliste',
            aliases: [],
          },
          {
            sheet: 'Telematik',
            templateId: 't-telematik',
            aliases: [],
          },
        ],
      },
    });
    await page.goto('/#/trains/master');
    const screen = page.locator('app-page-trains-master');
    await screen.getByTestId('master-import-all').click();
    await page
      .locator('ion-alert')
      .getByRole('button', { name: 'Bestätigen' })
      .click();

    await expect(
      page.getByText('Master-Datei vollständig importiert')
    ).toBeVisible();
    await expect(page.getByText('„Überblick“: übernommen.')).toBeVisible();
    await expect(page).toHaveURL(/#\/trains\/master$/);
  });

  test('ohne Datei lässt sich nichts importieren', async ({ page }) => {
    await installFakeBackend(page);
    await page.goto('/#/trains/master');
    const screen = page.locator('app-page-trains-master');
    await expect(
      screen.getByText('Noch keine Master-Datei übernommen')
    ).toBeVisible();
    await expect(screen.getByTestId('master-import')).toHaveAttribute(
      'disabled',
      ''
    );
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

  // Excel's AutoFilter per column. Which Farbe a fill becomes is Rust's
  // (`farbe::from_argb`); here only that a hand mark wins over the master's,
  // and that the dialog narrows the list and „Filter entfernen“ undoes it.
  test('der Spaltenfilter filtert nach Farbe und nach Wert', async ({
    page,
  }) => {
    await installFakeBackend(page, {
      wagen,
      partners,
      markierungen: {
        hand: { wagen: { '218124712173': 'gruen' }, radsaetze: {} },
        master: {
          wagen: { '218124712173': 'rot', '318047401234': 'rot' },
          radsaetze: {},
        },
      },
    });
    await page.goto('/#/trains/wagen');
    const rows = page.getByTestId('list-row');
    await expect(rows).toHaveCount(2);
    await expect(rows.first()).toHaveAttribute('data-farbe', 'gruen');

    await page.getByTestId('list-column-wagennummer').click();
    const dialog = page.locator('app-column-filter');
    await dialog.getByTestId('column-filter-farbe-rot').click();
    await expect(rows).toHaveCount(1);
    await expect(page.getByTestId('list-row-title')).toHaveText('318047401234');

    await dialog.getByTestId('column-filter-clear').click();
    await expect(rows).toHaveCount(2);

    // The search IS the filter: what it finds is ticked and applied.
    await dialog
      .getByTestId('column-filter-search')
      .locator('input')
      .fill('3180');
    await expect(rows).toHaveCount(1);
    await expect(dialog.getByTestId('column-filter-value')).toHaveCount(1);
    // Emptying the search ticks everything again — nothing stays unticked.
    await dialog.getByTestId('column-filter-search').locator('input').fill('');
    await expect(rows).toHaveCount(2);
    await expect(dialog.getByTestId('column-filter-value')).toHaveCount(2);
    for (const box of await dialog.getByTestId('column-filter-value').all()) {
      await expect(box).toHaveJSProperty('checked', true);
    }
    await dialog.getByTestId('column-filter-clear').click();
    await expect(rows).toHaveCount(2);

    await dialog.getByTestId('column-filter-value').first().click();
    await expect(rows).toHaveCount(1);
    await dialog.getByTestId('column-filter-close').click();
    await expect(page.getByTestId('list-column-wagennummer')).toHaveAttribute(
      'fill',
      'solid'
    );
  });

  test('die Detailseite setzt eine Markierung und nennt die der Master-Datei', async ({
    page,
  }) => {
    await installFakeBackend(page, {
      wagen,
      partners,
      markierungen: {
        master: { wagen: { '318047401234': 'rot' }, radsaetze: {} },
      },
    });
    await page.goto('/#/trains/wagen/w2');
    const farbe = page.getByTestId('detail-farbe');
    await expect(farbe.getByTestId('detail-farbe-master')).toContainText('Rot');
    await farbe.getByTestId('detail-farbe-blau').click();
    await expect(farbe.getByTestId('detail-farbe-master')).toContainText(
      'geht vor'
    );
    const calls = await recordedCalls(page);
    expect(
      calls.filter((call) => call.command === 'set_farbe').map((c) => c.args)
    ).toEqual([{ kind: 'wagen', id: 'w2', farbe: 'blau' }]);
  });

  // The Wagen-Zustand renders what the backend holds: where the Wagen is, that
  // it has been silent too long, and what is open. Whether a reading is newer
  // or older is Rust's and proved by `cargo test`.
  test('ein Wagen zeigt Standort, Funkstille und offene Fälle', async ({
    page,
  }) => {
    const source = {
      file: 'zustand.xlsx',
      sheet: 'Tabelle1',
      row: 2,
      importedAt: '2026-10-06',
    };
    await installFakeBackend(page, {
      wagen,
      partners,
      zustand: {
        meldungen: [
          {
            id: 'm1',
            wagenId: 'w1',
            zeitpunkt: '2020-01-01T08:00:00',
            standort: 'Neuhof (Kr Fulda)',
            land: 'DE',
            source,
          },
        ],
        schaeden: [
          { id: 's1', wagenId: 'w1', schadcode: '3.3.4', source },
          {
            id: 's2',
            wagenId: 'w1',
            schadcode: '1.2.1',
            erledigtAm: '2026-08-20',
            source,
          },
        ],
        pruefungen: [
          {
            id: 'p8',
            wagenId: 'w1',
            art: 'P8',
            faelligAm: '2027-03-31',
            source,
          },
        ],
      },
    });
    await page.goto('/#/trains/wagen');

    const row = page.getByTestId('list-row').first();
    await expect(row.getByTestId('wagen-standort')).toContainText(
      'Neuhof (Kr Fulda), DE'
    );
    await expect(row.getByTestId('wagen-standort')).toHaveAttribute(
      'color',
      'danger'
    );
    await expect(row.getByTestId('wagen-schaden')).toHaveText(
      '1 Schaden offen'
    );
    await expect(row.getByTestId('wagen-pruefung')).toHaveText(
      'P8 fällig 31.03.2027'
    );
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

  // The Telematik list is its own way in: it starts from the devices, in the
  // order Rust built (`trains::telematik`), and the dashboard tile counts the
  // same view. The silence rule itself is cargo test's.
  test('die Telematik-Kachel zählt die stummen Wagen und öffnet die Liste', async ({
    page,
  }) => {
    await installFakeBackend(page, {
      wagen,
      partners,
      telematik: {
        stumm: 1,
        rows: [
          {
            wagenId: 'w2',
            title: '318047401234',
            nummer: '318047401234',
            geraet: 'PTR-17',
            standort: 'Neuhof, DE',
            funk: 'funkte vor 12 Tagen',
            stumm: true,
            tage: 12,
            lines: ['letzte Meldung 26.09.2026 23:59'],
          },
          {
            wagenId: 'w1',
            title: '218124712173',
            nummer: '218124712173',
            standort: 'Altenburg, DE',
            funk: 'funkte heute',
            stumm: false,
            tage: 0,
            lines: [],
          },
        ],
      },
    });
    await page.goto('/#/trains/erp');
    const dashboard = page.locator('app-page-trains-dashboard');
    await expect(dashboard.getByText('2 Wagen · 1 stumm')).toBeVisible();
    await dashboard
      .getByRole('button', { name: 'Telematik', exact: true })
      .click();

    const list = page.locator('app-page-telematik-list');
    await expect(list.getByTestId('list-row-title')).toHaveText([
      '318047401234',
      '218124712173',
    ]);
    await expect(list.getByTestId('telematik-funk').first()).toHaveAttribute(
      'color',
      'danger'
    );
    await expect(list.getByText('Gerät PTR-17')).toBeVisible();

    await list.getByTestId('list-row').first().click();
    await expect(page).toHaveURL(/#\/trains\/wagen\/w2$/);
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

  // The order of the fitted Radsätze is Rust's (`trains::detail`); the card
  // only counts them, and a click opens the Wagen's detail page.
  test('eine Wagenkarte zählt die Radsätze und öffnet die Detailseite', async ({
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

    await page.getByTestId('list-row').click();
    await expect(page).toHaveURL(/#\/trains\/wagen\/w1$/);
    await expect(
      page.locator('app-page-entity-detail').getByTestId('detail-title')
    ).toHaveText('218124712173');
  });

  // The detail page renders what Rust built and follows a row's link to the
  // entity it names.
  test('eine Detailseite führt über einen Link zum Radsatz', async ({
    page,
  }) => {
    await installFakeBackend(page, {
      wagen,
      radsaetze,
      einbauten,
      entityDetails: {
        'wagen:w1': {
          kind: 'wagen',
          id: 'w1',
          title: '218124712173',
          fields: [{ label: 'Bauart', value: 'Tanoos' }],
          sections: [
            {
              title: 'Eingebaute Radsätze',
              empty: 'Kein Radsatz eingebaut.',
              rows: [
                {
                  title: 'RS4711',
                  lines: ['Position 1'],
                  link: { kind: 'radsatz', id: 'r1' },
                },
              ],
            },
            {
              title: 'Schadensmeldungen',
              empty: 'Keine Schadensmeldung.',
              rows: [],
            },
          ],
        },
      },
    });
    await page.goto('/#/trains/wagen/w1');
    const wagenPage = page.locator('app-page-entity-detail').last();
    await expect(wagenPage.getByText('Keine Schadensmeldung.')).toBeVisible();
    await wagenPage.getByTestId('detail-row').first().click();

    await expect(page).toHaveURL(/#\/trains\/radsaetze\/r1$/);
    await expect(
      page.locator('app-page-entity-detail').last().getByTestId('detail-title')
    ).toHaveText('RS4711');
  });

  // Telematik has no page per Wagen: a Telematik row opens the list, searched
  // for this Wagen's number.
  test('eine Telematik-Zeile öffnet die Telematik-Liste auf diesem Wagen', async ({
    page,
  }) => {
    await installFakeBackend(page, {
      wagen,
      entityDetails: {
        'wagen:w1': {
          kind: 'wagen',
          id: 'w1',
          title: '218124712173',
          fields: [],
          sections: [
            {
              title: 'Telematik',
              empty: 'Keine Telematik-Meldung.',
              rows: [
                {
                  title: 'Letzte Meldung 06.10.2026 08:15',
                  lines: ['Neuhof (Kr Fulda), DE'],
                  link: { kind: 'telematik', id: 'w1' },
                },
              ],
            },
          ],
        },
      },
      telematik: {
        stumm: 0,
        rows: [
          {
            wagenId: 'w1',
            title: '218124712173',
            nummer: '218124712173',
            standort: 'Neuhof (Kr Fulda), DE',
            funk: 'funkte heute',
            stumm: false,
            tage: 0,
            lines: [],
          },
          {
            wagenId: 'w2',
            title: '338080123452',
            nummer: '338080123452',
            standort: 'Altenburg, DE',
            funk: 'funkte heute',
            stumm: false,
            tage: 0,
            lines: [],
          },
        ],
      },
    });
    await page.goto('/#/trains/wagen/w1');
    const wagenPage = page.locator('app-page-entity-detail').last();
    await wagenPage.getByTestId('detail-row').first().click();

    await expect(page).toHaveURL(/#\/trains\/telematik\?wagen=w1$/);
    const list = page.locator('app-page-telematik-list');
    await expect(list.getByText('218124712173')).toBeVisible();
    await expect(list.getByText('338080123452')).toHaveCount(0);
  });

  test('ein Radsatz ohne offenen Einbau gilt als ausgebaut', async ({
    page,
  }) => {
    await installFakeBackend(page, { wagen, radsaetze, einbauten });
    await page.goto('/#/trains/radsaetze');
    await expect(page.getByText('ausgebaut')).toBeVisible();
  });
});
