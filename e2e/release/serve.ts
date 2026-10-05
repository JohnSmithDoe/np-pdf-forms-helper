// ─── why ────────────────────────────────────────────────────────
// Serves the PRODUCTION build the way the packaged app receives it: with the
// CSP from `src-tauri/tauri.conf.json`, rewritten as Tauri rewrites it.
//
// v2.0.1 shipped unstyled because of two things only this artefact has, and
// neither `ng serve` nor `tauri dev` (which loads `devUrl`) ever showed them:
// Angular's critical-CSS `<link onload>` is an inline handler that
// `script-src 'self'` refuses, and Tauri stamps a nonce on every `<style>` in
// index.html and appends it to `style-src` — and a nonce makes the browser
// IGNORE `'unsafe-inline'`, so every component style Angular injects later is
// refused. See tauri `manager::set_csp` / tauri-utils `html::inject_nonce_token`.
//
// The policy is READ from tauri.conf.json, never restated, so this server
// cannot drift from what ships. `dangerousDisableAssetCspModification` is
// honoured the way Tauri's `can_modify` reads it (a flag or a directive list).
// Tauri also hashes inline `<script>`s into `script-src`; that is NOT emulated,
// so an inline script fails here loudly although Tauri would let it run.
// ────────────────────────────────────────────────────────────────

import { randomInt } from 'node:crypto';
import { existsSync, readFileSync, statSync } from 'node:fs';
import { createServer } from 'node:http';
import { extname, join, normalize } from 'node:path';

const ROOT = join(import.meta.dirname, '../../dist/renderer');
const PORT = Number(process.env['PORT'] ?? 4401);

const TYPES: Record<string, string> = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript',
  '.css': 'text/css',
  '.json': 'application/json',
  '.png': 'image/png',
  '.svg': 'image/svg+xml',
  '.ico': 'image/x-icon',
  '.woff2': 'font/woff2',
};

interface Security {
  csp?: string;
  dangerousDisableAssetCspModification?: boolean | string[];
}

const security: Security = JSON.parse(
  readFileSync(
    join(import.meta.dirname, '../../src-tauri/tauri.conf.json'),
    'utf8'
  )
).app.security;

function canModify(directive: string): boolean {
  const disabled = security.dangerousDisableAssetCspModification ?? false;
  return Array.isArray(disabled) ? !disabled.includes(directive) : !disabled;
}

function directives(csp: string): Map<string, string[]> {
  const map = new Map<string, string[]>();
  for (const part of csp.split(';')) {
    const [name, ...sources] = part.trim().split(/\s+/);
    if (name) map.set(name, sources);
  }
  return map;
}

function withCsp(html: string): { html: string; csp: string } {
  const csp = directives(security.csp ?? '');
  if (canModify('style-src')) {
    const nonces: string[] = [];
    html = html.replace(/<style(?![^>]*\bnonce=)/g, () => {
      const nonce = String(randomInt(2 ** 47));
      nonces.push(`'nonce-${nonce}'`);
      return `<style nonce="${nonce}"`;
    });
    if (nonces.length) {
      const sources = csp.get('style-src') ?? [];
      if (!sources.includes("'self'")) sources.push("'self'");
      csp.set('style-src', [...sources, ...nonces]);
    }
  }
  const header = [...csp]
    .map(([name, sources]) => [name, ...sources].join(' '))
    .join('; ');
  return { html, csp: header };
}

createServer((request, response) => {
  const path = decodeURIComponent(new URL(request.url ?? '/', 'http://x').pathname);
  let file = normalize(join(ROOT, path));
  if (!file.startsWith(ROOT) || !existsSync(file) || statSync(file).isDirectory()) {
    file = join(ROOT, 'index.html');
  }
  const type = TYPES[extname(file)] ?? 'application/octet-stream';
  if (!file.endsWith('.html')) {
    response.writeHead(200, { 'content-type': type });
    response.end(readFileSync(file));
    return;
  }
  const { html, csp } = withCsp(readFileSync(file, 'utf8'));
  response.writeHead(200, {
    'content-type': type,
    'content-security-policy': csp,
  });
  response.end(html);
}).listen(PORT, () => {
  console.log(`release build on http://localhost:${PORT}`);
});
