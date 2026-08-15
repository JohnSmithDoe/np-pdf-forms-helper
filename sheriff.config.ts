import { anyTag, sameTag, SheriffConfig } from '@softarc/sheriff-core';

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
 * There is exactly ONE domain today (`filler`) plus `@shared`, because the app
 * is a single workflow over a single data model — a second sealed domain would
 * buy nothing. Nothing below names it: the `src/app/<domain>/<type>` matcher and
 * the `domain:*` rule are generic, so a second domain costs a folder and no
 * config change.
 */
export const config: SheriffConfig = {
  entryFile: './src/main.ts',
  enableBarrelLess: true,
  modules: {
    // Quarantine. See the `type:legacy` dep rule below.
    'src/_legacy': ['type:legacy'],
    'src/app': ['type:shell'],
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
      // Temporary migration seam: the shell may still route into an un-ported
      // Angular 13 page. Delete this entry when `src/_legacy` is empty.
      'type:legacy',
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

    // Sheriff has NO path-ignore option (verified against
    // `@softarc/sheriff-core`'s `UserSheriffConfig` / `Configuration` types —
    // the only exclusion keys are `excludeRoot`, which concerns the implicit
    // root project, and `ignoreFileExtensions`). So `src/_legacy` is quarantined
    // rather than skipped: it is its own module with `anyTag`, meaning the
    // architecture is not enforced *inside* it and it may still reach into
    // ported code while the port is in flight. The seal that matters is the
    // other direction — no `type:*` above lists `type:legacy` except the shell.
    // The eslint side genuinely does skip it: `src/_legacy/**` is in
    // `globalIgnores` in eslint.config.js.
    'type:legacy': [anyTag],

    // ─── Domain axis ───────────────────────────────────────────
    'domain:*': [sameTag, 'domain:@shared'],
  },
};
