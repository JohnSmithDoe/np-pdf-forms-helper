import { sameTag, SheriffConfig } from '@softarc/sheriff-core';

// Lets a domain's feature layer reuse a base page from `@shared/feature`
// (`type:feature` is otherwise not reachable from `type:feature`).
const featureMayUseSharedFeature = (ctx: {
  to: string;
  toModulePath: string;
}): boolean =>
  ctx.to === 'type:feature' && ctx.toModulePath.includes('/@shared/');

/**
 * Sheriff config — Hahnekamp two-axis tagging.
 *
 * Each module carries a `domain:*` tag (vertical) and a `type:*` tag
 * (horizontal). The shell is special: it only carries `type:shell` so
 * the domain-axis doesn't restrict it.
 *
 * Type axis:
 *   shell    → routes, feature, data, util, model
 *   routes   → feature, data, util             (a domain's own route manifest)
 *   feature  → smart-ui, ui, data, util, model (+ shared feature only)
 *   smart-ui → ui, data, util, model           (stateful presentational, leaf)
 *   ui       → self, util, model               (strict — pure dumb)
 *   data     → self, util, model
 *   util     → self, model
 *   model    → self
 *
 * Domain axis: every domain sealed.
 *
 * Three domains today — `filler`, `trains`, `info` — plus `@shared`. Nothing
 * below names any of them: the `src/app/<domain>/<type>` matcher and the
 * `domain:*` rule are generic, so a domain costs a folder and no config change.
 *
 * `trains/master` is a SUBMODULE of trains, not a domain: it renders trains'
 * data, and a sealed domain could not reach it. Patterns form a tree and a
 * literal segment shadows `<domain>`, so naming `trains/master/<type>` also
 * needs `trains/<type>` beside it, or every other trains module falls back to
 * the shell.
 *
 * What does NOT get its own domain is a second view over the same data. The
 * setup and export wizards live inside `filler` with the expert page, because
 * sealing them apart would cut them off from `FillerStore` and the only way back
 * would be pushing that store into `@shared` — the shared bus in front of two
 * halves of one workflow.
 */
export const config: SheriffConfig = {
  entryFile: './src/main.ts',
  enableBarrelLess: true,
  modules: {
    'src/app': ['type:shell'],
    'src/app/trains/master/<type>': ['domain:trains', 'type:<type>'],
    'src/app/trains/<type>': ['domain:trains', 'type:<type>'],
    'src/app/<domain>/<type>': ['domain:<domain>', 'type:<type>'],
  },

  depRules: {
    root: ['type:shell'],

    // ─── Type axis ─────────────────────────────────────────────
    'type:shell': [
      'type:routes',
      'type:feature',
      'type:data',
      'type:model',
      'type:util',
    ],
    'type:routes': ['type:feature', 'type:data', 'type:util'],
    'type:feature': [
      featureMayUseSharedFeature,
      'type:smart-ui',
      'type:ui',
      'type:data',
      'type:util',
      'type:model',
    ],
    'type:smart-ui': ['type:ui', 'type:data', 'type:util', 'type:model'],
    'type:ui': [sameTag, 'type:util', 'type:model'],
    'type:data': [sameTag, 'type:util', 'type:model'],
    'type:util': [sameTag, 'type:model'],
    'type:model': [sameTag],

    // ─── Domain axis ───────────────────────────────────────────
    'domain:*': [sameTag, 'domain:@shared'],
  },
};
