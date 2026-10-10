// ─── why ────────────────────────────────────────────────────────
// The one check that opens the artefact that SHIPS: `dist/renderer` from a
// production build, under the packaged app's CSP (`serve.ts`), against the
// same faked transport as the rest of `e2e/`.
//
// It exists because v2.0.1 was compiled, bundled and released without ever
// being opened, and came up on the customer's PC with no styles at all — full
// width cards, a serif font, and navigation that seemed to do nothing. Every
// other spec runs against `ng serve`, which has neither the CSP nor the
// critical-CSS inlining that caused it.
//
// Run by `pnpm run e2e:release` (builds first) and by CI after its production
// build. Deliberately shallow: it asserts that the styles APPLY, not how they
// look.
// ────────────────────────────────────────────────────────────────

import { expect, test } from '@playwright/test';
import { installFakeBackend } from '../fake-backend';

test('the release build renders and navigates under the shipped CSP', async ({
  page,
}) => {
  const violations: string[] = [];
  page.on('console', (message) => {
    if (/Content Security Policy/.test(message.text())) {
      violations.push(message.text());
    }
  });
  await installFakeBackend(page);
  await page.goto('/#/trains');

  const start = page.locator('app-page-trains-start');
  // The global stylesheet: Ionic's core CSS positions every `.ion-page`.
  await expect(start).toHaveCSS('position', 'absolute');
  // An Angular component style, injected at runtime as a `<style>`.
  await expect(start.locator('.start__tiles')).toHaveCSS('display', 'grid');

  await start.getByRole('button', { name: 'Bereinigen', exact: true }).click();
  // Without Ionic's page CSS the next page lands BELOW the old one, off-screen.
  await expect(page.locator('app-page-clean-hub')).toBeInViewport();

  expect(violations).toEqual([]);
});
