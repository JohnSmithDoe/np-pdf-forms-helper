// ─── why ────────────────────────────────────────────────────────
// The master update wizard, as shallow as the rest: it reaches its three steps
// from the document list — offered only while a client master is chosen —
// shows only the template's sheet with its key pair and problems, renders the
// cell changes it was sent and keeps Weiter dead while the sheet cannot be
// written.
//
// Which sheets are suggested, what a structural conflict is and which cells a
// paste changes are all Rust's (`master/export`) and proved by `cargo test`;
// the fake serves hand-written runs and only echoes the plan back.
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
    // Only the template's sheet is shown — one document updates one sheet —
    // taking new rows; the other sheet the backend sent is not offered.
    const rows = sheets.getByTestId('export-sheet');
    await expect(rows).toHaveCount(1);
    await expect(rows.nth(0)).toContainText('Telematik');
    await expect(rows.nth(0)).toContainText('neue Zeilen werden angehängt');
    await expect(sheets.getByText('Überblick')).toHaveCount(0);
    await expect(sheets.locator('ion-checkbox')).toHaveCount(0);
    await expect(sheets.getByTestId('export-sheet-append')).toHaveCount(1);
    await expect(
      sheets.getByTestId('export-sheet-append').locator('ion-toggle')
    ).toHaveAttribute('aria-checked', 'true');
    // Deleting rows is never a default: off until remembered for the template.
    await expect(
      sheets.getByTestId('export-sheet-remove').locator('ion-toggle')
    ).toHaveAttribute('aria-checked', 'false');
    // The mapping is the template's: only the key pair is stated, read-only.
    await expect(sheets.getByTestId('export-sheet-key')).toContainText(
      '„Asset“ → „Asset“'
    );
    await expect(sheets.locator('ion-select')).toHaveCount(0);
    await expect(sheets.getByTestId('export-sheet-open')).toHaveCount(0);
    await sheets.getByRole('button', { name: 'Weiter' }).click();

    const preview = step(page, 'export-preview');
    await expect(preview.getByTestId('cell-change')).toHaveCount(2);
    await expect(preview.getByTestId('cell-change').first()).toContainText(
      'Neuhof'
    );
    await preview
      .getByRole('button', { name: 'In Master-Datei schreiben' })
      .click();

    const result = step(page, 'export-result');
    await expect(result.getByTestId('export-result')).toContainText(
      '„Übersicht.xlsx“ geschrieben'
    );
    // Written into the customer's own file, the old state backed up first.
    await expect(result.getByTestId('export-result-backup')).toContainText(
      'Übersicht 2026-10-10 120000.xlsx'
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
    // Still the same file: the next document goes into it too.
    await expect(
      step(page, 'document-list').getByTestId('documents-master-row')
    ).toContainText('C:/Daten/Übersicht.xlsx');
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

  // With no mapping step there is nothing to answer a column with: it is
  // named as not transferred and Weiter stays open.
  test('nennt Spalten ohne Gegenstück, ohne anzuhalten', async ({ page }) => {
    await installFakeBackend(
      page,
      seed({ ...TELEMATIK, matched: ['Asset'], targets: [], open: ['Stadt'] })
    );
    await open(page);

    const sheets = step(page, 'export-sheets');
    await expect(sheets.getByTestId('export-sheet-open')).toContainText(
      'wird nicht übertragen: Stadt'
    );
    await expect(sheets.getByRole('button', { name: 'Weiter' })).toBeEnabled();
  });

  test('ein Blatt, das nicht geschrieben werden kann, hält an', async ({
    page,
  }) => {
    await installFakeBackend(
      page,
      seed({
        ...TELEMATIK,
        problem: 'Im Blatt „Telematik“ fehlt eine Schlüsselspalte.',
      })
    );
    await open(page);

    const sheets = step(page, 'export-sheets');
    await expect(sheets.getByTestId('export-sheet-problem')).toContainText(
      'fehlt eine Schlüsselspalte'
    );
    await expect(sheets.getByRole('button', { name: 'Weiter' })).toBeDisabled();
  });

  test('ein kalter Link in den Assistenten führt zu den Dokumenten', async ({
    page,
  }) => {
    await installFakeBackend(page, seed(TELEMATIK));
    await page.goto('/#/trains/master/export/preview');
    await expect(page).toHaveURL(/#\/trains\/documents$/);
  });
});
