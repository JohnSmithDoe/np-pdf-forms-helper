// ─── why ────────────────────────────────────────────────────────
// The master file walk, shallow: a pick shows its cleaning as pending, taking
// it over makes it the current version and pins it on top of the document
// list. What the cleaning changes — and that it never touches a formula or
// moves a row — is `master_file::clean`'s and proved by `cargo test`; the
// fake serves one hand-written cleaned version.
// ────────────────────────────────────────────────────────────────

import { expect, test } from '@playwright/test';
import {
  installFakeBackend,
  recordedCalls,
  type FakeMasterFileVersion,
} from './fake-backend';

const PICK: FakeMasterFileVersion = {
  id: 'm1',
  name: 'Master.xlsx',
  folder: '/daten/masterdatei/m1',
  original: '/daten/masterdatei/m1/Master.xlsx',
  cleaned: '/daten/masterdatei/m1/Master bereinigt.xlsx',
  originalHash: 'a',
  cleanedHash: 'b',
  bereinigtAm: '2026-10-04',
  report: {
    sheets: [
      {
        sheet: 'Wagenliste',
        rowsCut: 1046846,
        tailRowsCut: 1048000,
        formulaTail: 176,
        trimmed: 0,
        numbers: 1,
        dates: 0,
        examples: [
          {
            row: 4,
            column: 1,
            header: 'Wagennummer',
            raw: '3385 0659 152-2',
            clean: '338506591522',
            rule: 'number',
          },
        ],
        notes: [
          {
            row: 9,
            column: 2,
            header: 'Auftrag',
            raw: '000001102',
            reason: 'Zahl als Text, nicht eindeutig lesbar',
          },
        ],
        noteCount: 1,
      },
      {
        sheet: 'Übersicht',
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
      rowsCut: 1046846,
      tailRowsCut: 1048000,
      trimmed: 0,
      numbers: 1,
      dates: 0,
      notes: 1,
    },
  },
};

test.describe('Master-Datei', () => {
  test('bereinigt, übernimmt und zeigt sie oben in den Dokumenten', async ({
    page,
  }) => {
    await installFakeBackend(page, { masterFilePick: PICK });
    await page.goto('/#/trains/master-file');
    const view = page.locator('app-page-master-file');

    await expect(view.getByTestId('master-file-empty')).toBeVisible();
    await view.getByTestId('master-file-pick').click();

    const pending = view.getByTestId('master-file-pending');
    await expect(pending).toContainText('noch nicht übernommen');
    // Only the sheet with something to say is listed.
    await expect(pending.getByTestId('master-file-sheet')).toHaveCount(1);
    await expect(pending).toContainText('338506591522');
    await expect(pending).toContainText('000001102');
    // The one place formulas go is said, per sheet and in the totals.
    await expect(pending.getByTestId('master-file-tail')).toContainText(
      'Zeilen 176 bis 178 bleiben'
    );
    await expect(pending.getByTestId('master-file-tail-total')).toContainText(
      '1048000 Formelzeilen'
    );

    await view.getByTestId('master-file-accept').click();
    await expect(view.getByTestId('master-file-pending')).toHaveCount(0);
    await expect(view.getByTestId('master-file-current')).toContainText(
      'übernommen am 2026-10-04'
    );
    expect((await recordedCalls(page)).map((call) => call.command)).toContain(
      'accept_master_file'
    );

    await page.goto('/#/trains/documents');
    await expect(
      page.locator('app-page-document-list').getByTestId('documents-master-row')
    ).toContainText('Master.xlsx');
  });

  test('verwirft eine nicht übernommene Bereinigung', async ({ page }) => {
    await installFakeBackend(page, { masterFilePick: PICK });
    await page.goto('/#/trains/master-file');
    const view = page.locator('app-page-master-file');

    await view.getByTestId('master-file-pick').click();
    await view.getByTestId('master-file-discard').click();
    await expect(view.getByTestId('master-file-pending')).toHaveCount(0);
    await expect(view.getByTestId('master-file-empty')).toBeVisible();
  });
});
