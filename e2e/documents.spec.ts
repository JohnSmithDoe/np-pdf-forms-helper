// ─── why ────────────────────────────────────────────────────────
// „Archivieren“ on the document list, as shallow as the other trains specs: a
// row's button and „Alle archivieren“ send their command and the row leaves the
// list, „Archivierte zeigen“ brings the archive back from the backend, and
// „Wiederherstellen“ returns a row. Which list a Dokument is in is decided by
// Rust (`TrainsDb::dokumente`/`archiv`) and proved by `cargo test` — so is the
// rule that an archived document's bytes stay `vorhanden`. The fake only splits
// its seed the same way.
//
// Locators are scoped to `app-page-document-list`, because Ionic keeps the
// outgoing page in the DOM during a route transition (see `wizard.spec.ts`).
// ────────────────────────────────────────────────────────────────

import { expect, test, type Page } from '@playwright/test';
import {
  installFakeBackend,
  recordedCalls,
  type FakeDokument,
} from './fake-backend';

const dokument = (id: string, name: string): FakeDokument => ({
  id,
  name,
  sheet: 'Tabelle1',
  templateId: 't1',
  templateName: 'Werkstattaufträge',
  plan: {
    reader: 'headerRow',
    layout: { headerRow: 1, firstDataRow: 2 },
    columns: [{ header: 'Wagennummer', index: 1, field: 'wagennummer' }],
    date1904: false,
  },
  originalHash: id,
  cleanedHash: `${id}-c`,
  folder: `/daten/dokumente/${id}`,
  original: `/daten/dokumente/${id}/${name}`,
  cleaned: `/daten/dokumente/${id}/bereinigt.xlsx`,
  summary: {
    fehlerOffen: 0,
    deutungenOffen: 0,
    formatierungen: 0,
    korrigiert: 0,
  },
  bereinigtAm: '2026-10-03',
});

const AUFTRAEGE = dokument('d1', 'auftraege.xlsx');
const RECHNUNGEN = dokument('d2', 'rechnungen.xlsx');

const list = (page: Page) => page.locator('app-page-document-list');

const commands = async (page: Page) =>
  (await recordedCalls(page)).map((call) => call.command);

test.describe('Dokumente archivieren', () => {
  test('ein archiviertes Dokument verschwindet aus der Liste und kommt zurück', async ({
    page,
  }) => {
    await installFakeBackend(page, { dokumente: [AUFTRAEGE, RECHNUNGEN] });
    await page.goto('/#/trains/documents');
    const documents = list(page);
    await expect(documents.getByTestId('documents-row')).toHaveCount(2);
    // Nothing archived yet: no toggle to offer.
    await expect(documents.getByTestId('documents-archive-toggle')).toHaveCount(
      0
    );

    await documents
      .getByTestId('documents-row')
      .filter({ hasText: 'auftraege.xlsx' })
      .getByTestId('documents-archive')
      .click();

    await expect(documents.getByTestId('documents-row')).toHaveCount(1);
    await expect(documents.getByTestId('documents-row')).toContainText(
      'rechnungen.xlsx'
    );
    expect(
      (await recordedCalls(page)).find(
        (call) => call.command === 'archive_dokument'
      )?.args
    ).toEqual({ id: 'd1' });

    const toggle = documents.getByTestId('documents-archive-toggle');
    await expect(toggle).toContainText('Archivierte zeigen (1)');
    await toggle.locator('ion-toggle').click();
    await expect.poll(() => commands(page)).toContain('get_dokument_archiv');

    const archived = documents.getByTestId('documents-archived-row');
    await expect(archived).toHaveCount(1);
    await expect(archived).toContainText('archiviert am 2026-10-10');
    // An archived row is restored before it is imported or exported again.
    await expect(archived.getByTestId('documents-import')).toHaveCount(0);

    await archived.getByTestId('documents-restore').click();
    await expect(documents.getByTestId('documents-row')).toHaveCount(2);
    await expect(documents.getByTestId('documents-archived-row')).toHaveCount(
      0
    );
  });

  test('„Alle archivieren“ leert die Liste ohne Rückfrage', async ({
    page,
  }) => {
    await installFakeBackend(page, { dokumente: [AUFTRAEGE, RECHNUNGEN] });
    await page.goto('/#/trains/documents');
    const documents = list(page);
    await expect(documents.getByTestId('documents-row')).toHaveCount(2);

    await documents.getByTestId('documents-archive-all').click();

    await expect(documents.getByTestId('documents-row')).toHaveCount(0);
    await expect(documents.getByTestId('documents-empty')).toContainText(
      'Alle Dokumente sind archiviert.'
    );
    await expect(documents.getByTestId('documents-archive-all')).toHaveCount(0);
    await expect(
      documents.getByTestId('documents-archive-toggle')
    ).toContainText('Archivierte zeigen (2)');
    expect(await commands(page)).toContain('archive_all_dokumente');
  });
});
