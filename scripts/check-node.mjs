#!/usr/bin/env node
// Gate: this repo does not build on a Node older than `engines.node`.
//
// Node >= 22.18 is a hard floor, not a preference. `@angular/build` and
// `@angular/cli` 21.x declare `^20.19.0 || ^22.12.0 || >=24.0.0`, and the lint
// config loads TypeScript sources directly, which needs Node's native type
// stripping (22.18+). Below the floor the failures name none of this — the
// typical one is `Unknown file extension ".ts"` thrown from inside ESLint's
// config loader, so the first hour goes into the wrong file.
//
// `engines` alone does not stop it. Measured against pnpm 11.18: an unmet root
// `engines.node` is a **warning** and the install still exits 0, with or without
// `engine-strict=true` (that setting governs DEPENDENCIES' engines, not the
// consuming project's). So the declaration stays — it is the readable source of
// truth, and this reads the floor OUT of it rather than restating it — and this
// is what actually fails.
//
// Runs as `preinstall`, so it fires before anything else can produce a confusing
// error, and covers CI for free (`pnpm install --frozen-lockfile`) with no extra
// step. Node builtins only: at `preinstall` time there are no dependencies yet.

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const required = JSON.parse(
  readFileSync(path.join(HERE, '..', 'package.json'), 'utf8')
).engines?.node;

// Only the `>=x.y[.z]` form this repo declares. A different range is a change to
// `engines` that should come with a change here, so it says so rather than guess.
const declared = /^>=\s*(\d+)\.(\d+)(?:\.(\d+))?$/.exec(required ?? '');
if (!declared) {
  console.error(
    `check-node: cannot read a ">=x.y" floor out of engines.node (${required ?? 'missing'}).`
  );
  process.exit(1);
}

const floor = [declared[1], declared[2], declared[3] ?? '0'].map(Number);
const actual = process.versions.node.split('.').map(Number);

// The FIRST differing component decides and the rest are irrelevant — which is
// what makes this a comparison and not a per-component test. Node 23.0 is above a
// 22.18 floor even though its minor is lower, and a rule that looked at each
// component independently would reject it.
const compareVersions = (left, right) => {
  for (let index = 0; index < 3; index++) {
    const difference = (left[index] ?? 0) - (right[index] ?? 0);
    if (difference !== 0) return difference;
  }
  return 0;
};

if (compareVersions(actual, floor) < 0) {
  console.error(
    [
      '',
      `This repo needs Node ${required} — you are on v${process.versions.node}.`,
      '',
      'Angular 21 refuses to build below it, and the lint config cannot even load,',
      'with messages that name none of this.',
      '',
      '  nvm install && nvm use     (.nvmrc pins the line)',
      '',
    ].join('\n')
  );
  process.exit(1);
}
