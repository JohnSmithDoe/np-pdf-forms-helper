// ─── why ────────────────────────────────────────────────────────
// The happy paths a manual spot-check would walk, plus a block of KNOWN
// DEFECTS marked `test.fail()`.
//
// `test.fail()` earns its place here: the suite stays green while the bug is
// open, and the moment someone fixes it Playwright reports "expected to fail
// but passed" — so the fix cannot land silently and the test is the thing that
// tells you to delete the marker. Each one names the defect it pins.
// ────────────────────────────────────────────────────────────────

import { expect, test, type Locator, type Page } from '@playwright/test';
import {
  installFakeBackend,
  recordedCalls,
  type FakeDocument,
  type FakeMappedField,
  type FakeSeed,
} from './fake-backend';

// For xlsx the mapped `origId` is the SHEET id, so every cell on one sheet
// shares it. That is the root of the known defects at the bottom.
const SHEET = 'sheet-1';

function workbook(mapped: FakeMappedField[] = []): FakeDocument {
  return {
    id: 'doc-1',
    name: 'Angebot.xlsx',
    filename: '/vorlagen/Angebot.xlsx',
    mtime: 1_700_000_000_000,
    type: 'xlsx',
    sheets: [{ id: SHEET, name: 'Tabelle1' }],
    mapped,
  };
}

const cell = (address: string): FakeMappedField => ({
  origId: SHEET,
  mappedName: address,
});

// Straight to the expert page. `/` redirects by the stored view mode, whose
// default is the wizard — this suite is about the all-in-one page, and the
// wizard has its own.
async function start(page: Page, seed: FakeSeed = {}): Promise<void> {
  await installFakeBackend(page, seed);
  await page.goto('/#/documents/expert');
}

/**
 * By element, not by `getByRole('dialog')`: Ionic carries that role on a node
 * inside `ion-modal`'s shadow root, and the dialog's own markup is SLOTTED into
 * it. A screen reader sees the heading inside the dialog — the a11y tree nests
 * correctly — but Playwright chains locators through the DOM, where slotted
 * content is a child of the host and not of the shadow node holding the role.
 */
const modal = (page: Page) => page.locator('ion-modal');
const toast = (page: Page) => page.locator('ion-toast');

/**
 * An acknowledgement is a TOAST, so the test WAITS IT OUT rather than closing
 * it — the name is historical and the assertion is the real one: it was shown,
 * and it took itself away.
 *
 * Clicking "Schließen" here was flaky, and the flake was the point of the
 * feature: the toast dismisses on a timer, so a click issued after the timer
 * fired lands on an element already leaving. Nothing about the app is racy —
 * the test was asserting a button press the app does not require. Waiting for
 * the element to go is also what keeps a later click from being intercepted by
 * a toast still covering the bottom of the page.
 *
 * ATTACHED, not visible, and that is the second thing the flake taught: a toast
 * is only visible once Ionic's enter animation has run, and six parallel
 * workers sharing a machine with `ng serve` starve the frames that animation
 * needs. The element and its text are this app's contract; that Ionic can fade
 * it in is Ionic's.
 *
 * The welcome toast used to be the one place a test asserted a toast is really
 * VISIBLE, so that a toast which never rendered at all could not pass the whole
 * suite. That report is retired, and the guarantee went with it — what remains
 * is `closeReportDialog`, which does assert visibility, but only covers the
 * dialog branch of `needsDialog`.
 *
 * Playwright pierces open shadow roots, so `getByText` reaches the header
 * `ion-toast` renders inside its own — there is no light-DOM node to match.
 */
async function closeReport(page: Page, headline: string): Promise<void> {
  await expect(toast(page).getByText(headline)).toBeAttached();
  await expect(toast(page)).toHaveCount(0);
}

/**
 * The export report keeps the dialog: it carries the output folder, so it has a
 * button to press, and `ion-toast` is `aria-live="polite"` — a button inside one
 * is never announced and may not be the only path to an action.
 */
async function closeReportDialog(page: Page, headline: string): Promise<void> {
  const dialog = modal(page);
  await expect(dialog.getByRole('heading', { name: headline })).toBeVisible();
  await dialog.getByRole('button', { name: 'Schließen' }).click();
  await expect(modal(page)).toHaveCount(0);
}

/**
 * The field rows are gated on the accordion being open — `ion-accordion` does
 * not defer its content by itself, so the component keeps an `expanded` signal
 * and the rows are behind an `@if` on it. Nothing renders until opened.
 *
 * IDEMPOTENT and VERIFIED, both halves load-bearing. A bare `.click()` is
 * fire-and-forget — it returns once the event is dispatched, so a click landing
 * while the group is still re-rendering after a save is lost, and the assertions
 * that follow wait on rows that were never going to appear. Ionic puts
 * `accordion-expanded` on the host, so that is the state to wait for; clicking
 * only while it still reads `accordion-collapsed` is what stops a retry from
 * toggling it shut again — the same property `tick()` above needs.
 *
 * The test that made this flake reproducible (two cells on one workbook) is
 * gone rather than pinned. This stays because the `test.fail()` block depends on
 * it: there, a lost click fails the test, an expected failure is GREEN, and the
 * "expected to fail but passed" signal those markers exist for would never fire
 * once the defect was actually fixed.
 */
async function openDocument(page: Page): Promise<void> {
  const accordion = page.locator('ion-accordion').first();
  const header = accordion.locator('ion-item[slot="header"]');

  await expect(async () => {
    const open = ((await accordion.getAttribute('class')) ?? '').includes(
      'accordion-expanded'
    );
    if (!open) await header.click();
    await expect(accordion).toHaveClass(/accordion-expanded/, {
      timeout: 1_000,
    });
  }).toPass({ timeout: 15_000 });
}

const documentList = (page: Page) => page.locator('app-document-list');
const exportPanel = (page: Page) => page.locator('app-export-panel');

/**
 * The mapped-name boxes of the open document, BY VALUE and not by label.
 *
 * Their accessible name is the field's ORIGIN ("Feldname für Formularfeld
 * Mieter.Vorname"), because Ionic copies `aria-*` onto the native input once at
 * load: a name derived from the value the user is editing would go stale on the
 * first rename — and a test matching that stale name passes while the app is
 * wrong, which is exactly what the pinned defects below must not do.
 *
 * `getByRole('textbox')` reaches the native input inside `ion-input`'s shadow
 * root; nothing else in the list is a textbox.
 */
const fieldNames = (page: Page) => documentList(page).getByRole('textbox');

/**
 * ENSURE a box is ticked — the semantics of `.check()`, which this replaces.
 *
 * Two properties, and dropping either one breaks a test rather than a
 * selector:
 *
 *  • Idempotent. Ticking a DOCUMENT ticks all of its fields (see
 *    `setDocumentSelected` in filler.store.ts), so a field's box is often
 *    already on by the time the test reaches it. An unconditional click would
 *    turn it OFF and export nothing.
 *  • Polling. `ion-checkbox` is an ARIA checkbox, not a native `<input>`: the
 *    tick is a signal write that goes through the store and comes back as a
 *    `[checked]` binding. `.check()` reads `aria-checked` immediately after its
 *    click and fails without retrying, so it races a round-trip that is correct
 *    but not synchronous. `toBeChecked` retries.
 */
async function tick(box: Locator): Promise<void> {
  if ((await box.getAttribute('aria-checked')) !== 'true') await box.click();
  await expect(box).toBeChecked();
}

/**
 * By `aria-label`, not by role: the accordion HEADER is itself a button, so its
 * accessible name swallows every icon button inside it and a role query for
 * "Feld hinzufügen" matches the header instead.
 *
 * `getByLabel` rather than an attribute selector, because Ionic INHERITS the
 * aria attributes onto the native button in `ion-button`'s shadow root and
 * takes them off the host — so neither `ion-button[aria-label]` nor a bare
 * `button[aria-label]` is reliably the element. Matching the label itself is
 * indifferent to which one Ionic ends up putting it on.
 */
const iconButton = (page: Page, label: string) => page.getByLabel(label);

async function fillFieldDialog(
  page: Page,
  column: string,
  row: string,
  name: string
): Promise<void> {
  await iconButton(page, 'Feld hinzufügen').first().click();
  const dialog = modal(page);
  await dialog.locator('ion-select').click();
  // `interface="popover"` renders the choices as radios in a popover, not as
  // listbox options — the sheet picker is single-select.
  await page.getByRole('radio', { name: 'Tabelle1' }).click();
  await dialog.getByLabel('Spalte (A-ZZ)').fill(column);
  await dialog.getByLabel('Bezeichnung', { exact: true }).fill(row);
  await dialog.getByLabel('Eigene Bezeichnung').fill(name);
  await dialog.getByRole('button', { name: 'Hinzufügen' }).click();
}

async function addCell(
  page: Page,
  column: string,
  row: string,
  name: string
): Promise<void> {
  await fillFieldDialog(page, column, row, name);
  await closeReport(page, 'Dokument wurde erfolgreich gespeichert');
}

test.describe('npDokumentenhilfe', () => {
  // No welcome report any more: it rode the first `get_client_data` response,
  // and that load moved into a route resolver which runs before any page is
  // subscribed. Its absence is asserted, because a `ReplaySubject` slipped into
  // the transport would silently bring it back.
  test('zeigt den Leerzustand ohne Begrüßung', async ({ page }) => {
    await installFakeBackend(page);
    await page.goto('/#/documents/expert');

    await expect(
      page.getByText('Noch keine Dokumente hinzugefügt')
    ).toBeVisible();
    await expect(toast(page)).toHaveCount(0);
  });

  test('listet ein verknüpftes Dokument', async ({ page }) => {
    await start(page, { documents: [workbook()] });

    await expect(documentList(page).getByText('Angebot.xlsx')).toBeVisible();
    await expect(
      page.getByText('Noch keine Dokumente hinzugefügt')
    ).toBeHidden();
  });

  test('verknüpft ein Dokument über den Datei-Dialog', async ({ page }) => {
    await start(page, { picker: workbook() });

    await page
      .getByRole('button', { name: 'Einzelne Datei hinzufügen' })
      .click();
    await closeReport(page, 'Dokument wurde erfolgreich hinzugefügt');

    await expect(documentList(page).getByText('Angebot.xlsx')).toBeVisible();
    const calls = await recordedCalls(page);
    expect(calls.map((call) => call.command)).toContain('add_documents');
  });

  test('fügt eine Zelle hinzu und speichert sie als Adresse', async ({
    page,
  }) => {
    await start(page, { documents: [workbook()] });

    await addCell(page, 'A', '1', 'Kundenname');
    await openDocument(page);

    await expect(fieldNames(page)).toHaveValue('$Tabelle1.A1');

    // For xlsx the ADDRESS is the mapped name — the typed label is discarded.
    const saved = (await recordedCalls(page))
      .filter((call) => call.command === 'save_document')
      .pop();
    expect(saved?.args['document']).toMatchObject({
      mapped: [{ origId: SHEET, mappedName: '$Tabelle1.A1' }],
    });
  });

  test('exportiert die getippten Werte', async ({ page }) => {
    await start(page, { documents: [workbook([cell('$Tabelle1.A1')])] });

    await tick(page.getByRole('checkbox', { name: 'Angebot.xlsx' }));
    await openDocument(page);
    await tick(
      documentList(page).getByRole('checkbox', { name: 'Exportieren' })
    );

    await exportPanel(page).getByLabel('$Tabelle1.A1').fill('Müller');
    await page.getByRole('button', { name: 'Dokumente erstellen' }).click();
    await closeReportDialog(page, 'Dokumente wurden erfolgreich erstellt');

    const exported = (await recordedCalls(page))
      .filter((call) => call.command === 'create_documents')
      .pop();
    expect(exported?.args['documentIds']).toEqual(['doc-1']);
    expect(exported?.args['inputs']).toEqual([
      { identifiers: [SHEET], value: 'Müller' },
    ]);
  });

  test('zeigt einen einzeiligen Backend-Fehler als Toast', async ({ page }) => {
    await start(page, {
      documents: [workbook()],
      failures: {
        save_document: ['Die Datei konnte nicht geschrieben werden.'],
      },
    });

    await fillFieldDialog(page, 'A', '1', 'Kundenname');

    const error = toast(page);
    await expect(
      error.getByText('Es ist ein Problem aufgetreten')
    ).toBeVisible();
    await expect(
      error.getByText('Die Datei konnte nicht geschrieben werden.')
    ).toBeVisible();
  });

  // The other half of `needsDialog`: several lines are a list to read, and a
  // toast that takes itself away again loses all but the first.
  test('zeigt einen mehrzeiligen Backend-Fehler als Dialog', async ({
    page,
  }) => {
    await start(page, {
      documents: [workbook()],
      failures: {
        save_document: [
          'Die Datei konnte nicht geschrieben werden.',
          'Zugriff verweigert.',
        ],
      },
    });

    await fillFieldDialog(page, 'A', '1', 'Kundenname');

    const error = modal(page);
    await expect(
      error.getByRole('heading', { name: 'Es ist ein Problem aufgetreten' })
    ).toBeVisible();
    await expect(error.getByText('Zugriff verweigert.')).toBeVisible();
  });
});

// ─── known defects ────────────────────────────────────────────────
// All three are the same root cause: for xlsx the mapped `origId` is the SHEET
// id, shared by every cell on it, while the store keys rename, remove and
// export selection on `origId` as if it identified one field. Delete the
// `test.fail()` line when the id becomes per-cell — Playwright will tell you.

test.describe('bekannte Defekte (xlsx teilt sich eine origId)', () => {
  const twoCells = () => [
    workbook([cell('$Tabelle1.A1'), cell('$Tabelle1.B2')]),
  ];

  test('Umbenennen trifft nur die bearbeitete Zelle', async ({ page }) => {
    test.fail();
    await start(page, { documents: twoCells() });
    await openDocument(page);

    const input = fieldNames(page).first();
    await input.fill('$Tabelle1.C3');
    await input.blur();
    await closeReport(page, 'Dokument wurde erfolgreich gespeichert');

    // The other cell must be untouched. It is not: both rows carry the sheet's
    // id, so the rename lands on both and this box reads `$Tabelle1.C3` too.
    await expect(fieldNames(page).nth(1)).toHaveValue('$Tabelle1.B2');
  });

  test('Entfernen trifft nur die gewählte Zelle', async ({ page }) => {
    test.fail();
    await start(page, { documents: twoCells() });
    await openDocument(page);

    await iconButton(page, 'Feld entfernen').first().click();
    await page.getByRole('button', { name: 'Bestätigen' }).click();
    await closeReport(page, 'Dokument wurde erfolgreich gespeichert');

    // Removing one cell removes both, so no box is left to hold this value.
    await expect(fieldNames(page)).toHaveValue('$Tabelle1.B2');
  });

  test('Export-Haken gilt nur für die angehakte Zelle', async ({ page }) => {
    test.fail();
    await start(page, { documents: twoCells() });
    await tick(page.getByRole('checkbox', { name: 'Angebot.xlsx' }));
    await openDocument(page);

    const ticks = documentList(page).getByRole('checkbox', {
      name: 'Exportieren',
    });
    await tick(ticks.first());

    await expect(ticks.nth(1)).not.toBeChecked();
  });
});
