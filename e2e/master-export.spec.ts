// ─── why ────────────────────────────────────────────────────────
// The master update wizard, as shallow as the rest: it reaches its four steps
// from the document list — offered only while a client master exists — ticks what the backend suggested, renders the cell
// changes it was sent and keeps Weiter dead while a column is unanswered.
//
// Which sheets are suggested, what a structural conflict is and which cells a
// paste changes are all Rust's (`master/export`) and proved by `cargo test`;
// the fake serves hand-written runs and only echoes answers back. The column
// selects are `ion-select` popovers and deliberately not driven here.
// ────────────────────────────────────────────────────────────────

import { expect, test, type Page } from '@playwright/test';
import {
  installFakeBackend,
  recordedCalls,
  type FakeDokument,
  type FakeExportSheetRun,
  type FakeSeed,
} from './fake-backend';

const DOKUMENT: FakeDokument = {
  id: 'd1',
  name: 'assets.xlsx',
  sheet: 'Sheet1',
  templateId: 't-telematik',
  templateName: 'Telematik-Export',
  plan: {
    reader: 'headerRow',
    layout: { headerRow: 1, firstDataRow: 2 },
    columns: [
      { header: 'Asset', index: 1, field: 'wagennummer' },
      { header: 'Stadt', index: 2, field: 'ignorieren' },
    ],
    templateId: 't-telematik',
    date1904: false,
  },
  originalHash: 'a',
  cleanedHash: 'b',
  folder: '/daten/dokumente/d1',
  original: '/daten/dokumente/d1/assets.xlsx',
  cleaned: '/daten/dokumente/d1/assets.bereinigt.xlsx',
  summary: {
    fehlerOffen: 0,
    deutungenOffen: 0,
    formatierungen: 0,
    korrigiert: 0,
  },
  bereinigtAm: '2026-10-03',
};

const TELEMATIK: FakeExportSheetRun = {
  matched: ['Asset', 'Stadt'],
  targets: [],
  open: [],
  line: '„Telematik“: 1 Zeile(n) aus „assets.xlsx“ übernommen (vorher 2).',
  changed: 2,
  changes: [
    {
      cell: 'B2',
      row: 2,
      column: 'Stadt',
      key: '338506591522',
      before: 'Altstadt',
      after: 'Neuhof',
    },
    {
      cell: 'A3',
      row: 3,
      column: 'Asset',
      key: '338506590011',
      before: '338506590011',
      after: '',
    },
  ],
};

const seed = (
  telematik: FakeExportSheetRun,
  removeFor: string[] = []
): FakeSeed => ({
  dokumente: [DOKUMENT],
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
        key: 'Asset',
        aliases: [],
        removeFor,
      },
    ],
  },
  masterSheets: [
    { name: 'Überblick', headers: ['Wagennummer'] },
    { name: 'Telematik', headers: ['Asset', 'Stadt'] },
  ],
  masterExport: { Telematik: telematik },
});

const step = (page: Page, name: string) => page.locator(`app-page-${name}`);

async function open(page: Page): Promise<void> {
  // „Master aktualisieren“ is behind the full-app toggle for now.
  await page.addInitScript(() => localStorage.setItem('npdh.full', 'on'));
  await page.goto('/#/trains/documents');
  await step(page, 'document-list')
    .getByTestId('documents-export-master')
    .click();
  await expect(page).toHaveURL(/#\/trains\/master\/export\/sheets$/);
}

test.describe('Master aktualisieren', () => {
  test('führt vom Dokument über Vorschau zur Zusammenfassung', async ({
    page,
  }) => {
    await installFakeBackend(page, seed(TELEMATIK));
    await open(page);

    const sheets = step(page, 'export-sheets');
    await expect(sheets.getByTestId('wizard-phase')).toContainText(
      'Master aktualisieren'
    );
    await expect(sheets.getByTestId('export-base')).toContainText(
      'Übersicht.xlsx'
    );
    const rows = sheets.getByTestId('export-sheet');
    await expect(rows).toHaveCount(2);
    // The template's sheet first, pre-ticked, taking new rows; the other one
    // can still be ticked.
    await expect(rows.nth(0)).toContainText('Telematik');
    await expect(rows.nth(0)).toContainText('betroffen');
    await expect(rows.nth(0)).toContainText('neue Zeilen werden angehängt');
    await expect(sheets.getByTestId('export-sheet-append')).toHaveCount(1);
    await expect(
      sheets.getByTestId('export-sheet-append').locator('ion-toggle')
    ).toHaveAttribute('aria-checked', 'true');
    // Deleting rows is never a default: off until remembered for the template.
    await expect(
      sheets.getByTestId('export-sheet-remove').locator('ion-toggle')
    ).toHaveAttribute('aria-checked', 'false');
    await expect(rows.nth(1).locator('ion-checkbox')).not.toHaveAttribute(
      'aria-disabled',
      'true'
    );
    // The row toggles, not only the box: tick the overview, untick it again.
    await rows.nth(1).getByText('Überblick').click();
    await expect(rows.nth(1).locator('ion-checkbox')).toHaveAttribute(
      'aria-checked',
      'true'
    );
    await rows.nth(1).getByText('Überblick').click();
    await sheets.getByRole('button', { name: 'Weiter' }).click();

    const structure = step(page, 'export-structure');
    await expect(structure.getByTestId('export-structure-clear')).toBeVisible();
    await structure.getByRole('button', { name: 'Weiter' }).click();

    const preview = step(page, 'export-preview');
    await expect(preview.getByTestId('cell-change')).toHaveCount(2);
    await expect(preview.getByTestId('cell-change').first()).toContainText(
      'Neuhof'
    );
    await preview.getByRole('button', { name: 'Übernehmen' }).click();

    const result = step(page, 'export-result');
    await expect(result.getByTestId('export-result')).toContainText(
      'Neue Fassung „Übersicht.xlsx“'
    );
    await expect(result.getByTestId('export-result-sheet')).toContainText(
      '2 Zelle(n) aktualisiert'
    );

    const write = (await recordedCalls(page)).find(
      (call) => call.command === 'write_master_export'
    );
    const request = write?.args['request'] as {
      sheets: {
        sheet: string;
        key?: string;
        append: boolean;
        remove: boolean;
      }[];
      remember: boolean;
    };
    expect(request.sheets).toEqual([
      expect.objectContaining({
        sheet: 'Telematik',
        key: 'Asset',
        append: true,
        remove: false,
      }),
    ]);
    expect(request.remember).toBe(true);

    await result.getByRole('button', { name: 'Fertig' }).click();
    await expect(page).toHaveURL(/#\/trains\/documents$/);
    // The written version is the current master now, named after its document.
    await expect(
      step(page, 'document-list').getByTestId('documents-master-row')
    ).toContainText('aktualisiert mit „assets.xlsx“');
  });

  test('leert fehlende Zeilen, wenn es für die Vorlage gemerkt ist', async ({
    page,
  }) => {
    await installFakeBackend(
      page,
      seed({ ...TELEMATIK, removed: ['338506590011'] }, ['t-telematik'])
    );
    await open(page);

    const sheets = step(page, 'export-sheets');
    await expect(
      sheets.getByTestId('export-sheet-remove').locator('ion-toggle')
    ).toHaveAttribute('aria-checked', 'true');
    await sheets.getByRole('button', { name: 'Weiter' }).click();

    const structure = step(page, 'export-structure');
    await expect(structure.getByTestId('export-structure-sheet')).toContainText(
      'fehlende Zeilen werden geleert'
    );
    await structure.getByRole('button', { name: 'Weiter' }).click();

    // The rows to be emptied are listed by key before anything is written.
    await expect(
      step(page, 'export-preview').getByTestId('export-preview-removed')
    ).toContainText('1 Zeile(n) werden geleert: 338506590011');
    const previewed = (await recordedCalls(page)).find(
      (call) => call.command === 'preview_master_export'
    );
    expect(
      (previewed?.args['request'] as { sheets: { remove: boolean }[] })
        .sheets[0].remove
    ).toBe(true);
  });

  test('ohne Master-Datei gibt es nichts zu aktualisieren', async ({
    page,
  }) => {
    await installFakeBackend(page, { dokumente: [DOKUMENT] });
    await page.goto('/#/trains/documents');
    const list = step(page, 'document-list');
    await expect(list.getByTestId('documents-row')).toHaveCount(1);
    await expect(list.getByTestId('documents-export-master')).toHaveCount(0);
  });

  test('eine Spalte ohne Gegenstück hält den Abgleich an', async ({ page }) => {
    await installFakeBackend(
      page,
      seed({
        ...TELEMATIK,
        matched: ['Asset'],
        targets: ['Ort'],
        open: ['Stadt'],
      })
    );
    await open(page);
    await step(page, 'export-sheets')
      .getByRole('button', { name: 'Weiter' })
      .click();

    const structure = step(page, 'export-structure');
    await expect(structure.getByTestId('export-structure-column')).toHaveCount(
      1
    );
    await expect(
      structure.getByRole('button', { name: 'Weiter' })
    ).toBeDisabled();
  });

  // A header spelled differently in the master is fed by a pair set by hand,
  // on any sheet: the step lists every pair, offers a new one while a
  // document column and a hand-kept column are free, and names the rest.
  test('der Abgleich zeigt jede Zuordnung und bietet eine neue an', async ({
    page,
  }) => {
    await installFakeBackend(
      page,
      seed({
        ...TELEMATIK,
        matched: ['Asset'],
        sources: ['Asset', 'RadsatzID'],
        targets: ['Radsatz ID'],
        open: [],
      })
    );
    await open(page);
    await step(page, 'export-sheets')
      .getByRole('button', { name: 'Weiter' })
      .click();

    const structure = step(page, 'export-structure');
    await expect(structure.getByTestId('export-structure-key')).toHaveCount(1);
    await expect(structure.getByTestId('export-structure-pair')).toHaveCount(1);
    await expect(structure.getByTestId('export-structure-add')).toHaveCount(1);
    await expect(
      structure.getByTestId('export-structure-untransferred')
    ).toContainText('RadsatzID');
    await expect(
      structure.getByRole('button', { name: 'Weiter' })
    ).toBeEnabled();
  });

  test('ein kalter Link in den Assistenten führt zu den Dokumenten', async ({
    page,
  }) => {
    await installFakeBackend(page, seed(TELEMATIK));
    await page.goto('/#/trains/master/export/preview');
    await expect(page).toHaveURL(/#\/trains\/documents$/);
  });
});
