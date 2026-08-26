// ─── why ────────────────────────────────────────────────────────
// The wizard's happy path, once, end to end, plus the two things only the wizard
// has. Deliberately not a mirror of `filler.spec.ts`: the steps compose
// components that suite already covers, so what is worth proving here is only
// what is new — that each guard lets a legitimately-reached step through, that
// the result step renders the run's report instead of a dialog opening over it,
// and that this wizard offers no setup control at all. That last one is the
// point of having two modes, and it is an ABSENCE, so nothing else would notice
// it coming back.
//
// The last two assertions are the pair that matters. The run is sent `silent`,
// so its report must appear as the PAGE and `ion-modal` must stay at zero; get
// the flag wrong and both would be true of a dialog covering the page.
//
// Navigation is by hash, matching `trains.spec.ts`. `/` is not used because it
// redirects by the stored view mode, and the point here is the wizard.
// ────────────────────────────────────────────────────────────────

import { expect, test, type Page } from '@playwright/test';
import {
  installFakeBackend,
  type FakeDocument,
  type FakeMappedField,
} from './fake-backend';

const SHEET = 'sheet-1';

const cell = (address: string): FakeMappedField => ({
  origId: SHEET,
  mappedName: address,
});

function workbook(): FakeDocument {
  return {
    id: 'doc-1',
    name: 'Angebot.xlsx',
    filename: '/vorlagen/Angebot.xlsx',
    mtime: 1_700_000_000_000,
    type: 'xlsx',
    sheets: [{ id: SHEET, name: 'Tabelle1' }],
    mapped: [cell('$Tabelle1.A1')],
  };
}

/**
 * Every locator is scoped to its own step's page element. Ionic keeps the
 * OUTGOING page in the DOM for the duration of the route transition, so an
 * unscoped `getByRole('button', { name: 'Weiter' })` matches the step just left
 * as well as the one just entered — and the shell puts a Weiter on nearly all of
 * them.
 */
const step = (page: Page, name: string) => page.locator(`app-page-${name}`);

test.describe('Assistent', () => {
  test('führt von der Auswahl bis zum Ergebnis', async ({ page }) => {
    await installFakeBackend(page, { documents: [workbook()] });
    await page.goto('/#/documents/wizard/selection');

    // The step ticks and nothing else. These three were on it while it composed
    // the expert page's document list, and each one is setup: two pickers and
    // the fold-out that carries automatic mapping and "Alles zurücksetzen".
    const selection = step(page, 'export-selection');
    await expect(selection.getByRole('button', { name: 'Datei' })).toHaveCount(
      0
    );
    await expect(selection.getByRole('button', { name: 'Ordner' })).toHaveCount(
      0
    );
    await expect(
      selection.getByRole('button', { name: 'Optionen einblenden' })
    ).toHaveCount(0);

    // Ticking the document ticks its fields too, which is what fills
    // `exportFields` and so what unlocks the guarded steps behind Weiter.
    await selection.getByRole('checkbox', { name: 'Angebot.xlsx' }).click();
    await selection.getByRole('button', { name: 'Weiter' }).click();

    const values = step(page, 'export-values');
    await expect(values.getByText('Schritt 2 von 4')).toBeVisible();
    await values.getByLabel('$Tabelle1.A1').fill('Musterfirma');
    await values.getByRole('button', { name: 'Weiter' }).click();

    const generate = step(page, 'export-generate');
    await expect(generate.getByText('Schritt 3 von 4')).toBeVisible();
    await expect(generate.getByText('Musterfirma')).toBeVisible();
    await generate.getByRole('button', { name: 'Jetzt erzeugen' }).click();

    const result = step(page, 'export-result');
    await expect(result.getByText('Schritt 4 von 4')).toBeVisible();
    await expect(
      result.getByRole('heading', {
        name: 'Dokumente wurden erfolgreich erstellt',
      })
    ).toBeVisible();
    await expect(
      result.getByText('Alle Dokumente wurden erfolgreich erstellt.')
    ).toBeVisible();
    await expect(page.locator('ion-modal')).toHaveCount(0);
  });

  // With nothing linked there is nothing to tick, and the picker belongs to the
  // other wizard — so the empty state's only way on is the way to setup.
  test('schickt einen leeren Bestand zum Einrichten-Assistenten', async ({
    page,
  }) => {
    await installFakeBackend(page, { documents: [] });
    await page.goto('/#/documents/wizard/selection');

    const selection = step(page, 'export-selection');
    await selection
      .getByRole('button', { name: 'Dokumente einrichten' })
      .click();

    await expect(page).toHaveURL(/#\/documents\/setup\/source$/);
  });

  // The guards are pure predicates over the store, so a cold entry to a later
  // step has nothing to stand on and must land back at step one rather than
  // rendering an empty page.
  test('schickt einen kalten Direktaufruf zurück auf Schritt 1', async ({
    page,
  }) => {
    await installFakeBackend(page, { documents: [workbook()] });
    await page.goto('/#/documents/wizard/result');

    await expect(page).toHaveURL(/#\/documents\/wizard\/selection$/);
    await expect(page.getByText('Schritt 1 von 4')).toBeVisible();
  });
});
