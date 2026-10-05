import { defineConfig } from '@playwright/test';

const PORT = 4401;
const URL = `http://localhost:${PORT}`;

// `e2e/release/` against the production build under the packaged CSP — see
// `e2e/release/serve.ts`. Its own port and never a reused server: whatever
// else listens there is not the release build.
export default defineConfig({
  testDir: './e2e/release',
  forbidOnly: Boolean(process.env['CI']),
  reporter: process.env['CI'] ? 'github' : 'list',
  use: {
    baseURL: URL,
    trace: 'retain-on-failure',
  },
  projects: [{ name: 'chrome', use: { channel: 'chrome' } }],
  webServer: {
    command: 'node e2e/release/serve.ts',
    url: URL,
    reuseExistingServer: false,
    env: { PORT: String(PORT) },
  },
});
