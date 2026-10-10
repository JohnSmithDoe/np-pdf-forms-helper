// ─── why ────────────────────────────────────────────────────────
// The master file, shallow: the user picks the customer's own workbook where
// it lies, the page shows that path, and the document list pins it on top and
// offers „In Master übertragen“. Nothing is copied or cleaned — that the write
// goes into this very file after a backup is `master_file::write_in_place`'s
// and proved by `cargo test`.
// ────────────────────────────────────────────────────────────────

import { expect, test } from '@playwright/test';
import { installFakeBackend, recordedCalls } from './fake-backend';

const TARGET = 'C:\\Kunde\\Master\\Wagenmut Master.xlsx';

test.describe('Master-Datei', () => {
  test('wird gewählt, wo sie liegt, und steht oben in den Dokumenten', async ({
    page,
  }) => {
    await installFakeBackend(page, { masterTargetPick: TARGET });
    await page.goto('/#/trains/master-file');
    const view = page.locator('app-page-master-file');

    await expect(view.getByTestId('master-file-empty')).toBeVisible();
    await view.getByTestId('master-file-pick').click();

    const current = view.getByTestId('master-file-current');
    await expect(current).toContainText('Wagenmut Master.xlsx');
    await expect(current).toContainText(TARGET);
    await expect(current).toContainText('Sicherung');
    expect((await recordedCalls(page)).map((call) => call.command)).toContain(
      'pick_master_target'
    );

    await page.goto('/#/trains/documents');
    await expect(
      page.locator('app-page-document-list').getByTestId('documents-master-row')
    ).toContainText(TARGET);
  });

  test('ein abgebrochener Dialog lässt alles, wie es war', async ({ page }) => {
    await installFakeBackend(page, { masterTargetPick: null });
    await page.goto('/#/trains/master-file');
    const view = page.locator('app-page-master-file');

    await view.getByTestId('master-file-pick').click();
    await expect(view.getByTestId('master-file-empty')).toBeVisible();
  });
});
