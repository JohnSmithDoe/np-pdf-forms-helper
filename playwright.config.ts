// ─── why ────────────────────────────────────────────────────────
// Drives the Angular app in a real browser against a FAKE Tauri transport
// (`e2e/fake-backend.ts`), not against a shell.
//
// Playwright cannot drive the real app: Tauri speaks WebDriver, and driving it
// directly is Windows/Linux only because macOS ships no WKWebView driver — the
// dev machine here is macOS. What is reachable is everything in `src/`, which
// is where the store, the facade, the dialogs and the export grouping live.
//
// `channel: 'chrome'` uses the installed Google Chrome instead of downloading
// a bundled browser, which is why `@playwright/test` needs no postinstall here.
// ────────────────────────────────────────────────────────────────

import { defineConfig } from '@playwright/test';

const PORT = 4200;
const URL = `http://localhost:${PORT}`;

export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  forbidOnly: Boolean(process.env['CI']),
  retries: process.env['CI'] ? 1 : 0,
  reporter: process.env['CI'] ? 'github' : 'list',
  use: {
    baseURL: URL,
    trace: 'on-first-retry',
  },
  projects: [{ name: 'chrome', use: { channel: 'chrome' } }],
  // `ng serve`, reused if one is already up — `tauri dev` starts one too, and
  // two servers on 4200 would just fight.
  webServer: {
    command: 'pnpm start',
    url: URL,
    reuseExistingServer: !process.env['CI'],
    timeout: 180_000,
  },
});
