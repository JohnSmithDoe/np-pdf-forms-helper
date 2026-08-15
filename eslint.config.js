// @ts-check
const { defineConfig, globalIgnores } = require('eslint/config');
const angular = require('angular-eslint');
const sheriff = require('@softarc/eslint-plugin-sheriff');
const prettierPlugin = require('eslint-plugin-prettier');
const prettierConfig = require('eslint-config-prettier');
// Registered by name here because the type-aware rules below are set in THIS
// block: `extends` scopes a plugin to the config element that declared it, so
// inheriting a preset does not make `@typescript-eslint/*` resolvable in a
// sibling `rules`.
const tseslint = require('typescript-eslint');

module.exports = defineConfig(
  globalIgnores([
    '.angular/**',
    'dist/**',
    'node_modules/**',
    'electron/node_modules/**',
    'release/**',
    // Quarantined Angular 13 code, ported into src/app incrementally. Linting
    // it would produce hundreds of errors nobody is going to fix in place — the
    // fix is the port. Sheriff cannot ignore a path (it has no such option), so
    // it seals `_legacy` with a dep rule instead; see sheriff.config.ts.
    'src/_legacy/**',
  ]),
  sheriff.configs.all,
  {
    files: ['**/*.ts'],
    plugins: { '@typescript-eslint': tseslint.plugin },
    extends: [...angular.configs.tsRecommended],
    processor: angular.processInlineTemplates,
    languageOptions: {
      parserOptions: {
        // Use the TypeScript project service so each file resolves via the
        // tsconfig that owns it (renderer vs. electron main) instead of one
        // hardcoded `project`.
        project: null,
        projectService: true,
        tsconfigRootDir: __dirname,
      },
    },
    rules: {
      // `Page` for routed components, `Dialog` for Material dialog bodies —
      // both are real kinds here and neither reads well as `…Component`.
      '@angular-eslint/component-class-suffix': [
        'error',
        { suffixes: ['Page', 'Dialog', 'Component'] },
      ],
      '@angular-eslint/component-selector': [
        'error',
        { type: 'element', prefix: 'app', style: 'kebab-case' },
      ],
      '@angular-eslint/directive-selector': [
        'error',
        { type: 'attribute', prefix: 'app', style: 'camelCase' },
      ],
      // The type-aware rules. The expensive half — `projectService` above — is
      // already being paid for; nothing was reading the types it produces.
      // These four are the ones with something to catch in an Electron app
      // whose whole IPC surface is promise-returning: an un-awaited promise is
      // how a main-process rejection becomes an unhandled-rejection instead of
      // an error the UI shows, and a promise-returning handler passed where
      // void is expected is how a failure vanishes entirely. Enabled by id
      // rather than via `recommendedTypeChecked` so the set is a decision, not
      // a default that shifts under a minor bump.
      '@typescript-eslint/no-floating-promises': 'error',
      '@typescript-eslint/no-misused-promises': 'error',
      '@typescript-eslint/await-thenable': 'error',
      '@typescript-eslint/require-await': 'error',
      // House naming: a type is named for what it IS (`MappedDocument`), never
      // decorated with its kind (`IMappedDocument`, `TMappedDocument`). Written
      // as `custom` rather than `filter` on purpose — `filter` selects WHICH
      // names a config applies to, so it can only ever exempt the bad names;
      // `custom` is the assertion the name must satisfy, so `match: false`
      // makes the prefix an error.
      //
      // The regex requires a lowercase third character so acronyms survive:
      // `IOError` and `IPCChannel` pass, `IProfile` and `TState` do not.
      //
      // Only these two selectors are configured, which means every other
      // identifier keeps whatever the rest of the ruleset says — in particular
      // `typeParameter` is untouched, so a generic still spells itself `T`.
      '@typescript-eslint/naming-convention': [
        'error',
        {
          selector: ['interface', 'typeAlias'],
          format: ['PascalCase'],
          custom: { regex: '^(I|T)[A-Z][a-z]', match: false },
        },
      ],
      // A published subpath is fine — `@angular/material/dialog` and
      // `rxjs/operators` are the supported way in. A path into a package's
      // BUILD OUTPUT is not, and neither resolution nor tsc will say so:
      // packages without an `exports` map resolve any deep path silently, and
      // rxjs publishes `./internal/*` deliberately.
      //
      // The denylist is segment names that mean "not an entry point" rather
      // than an attempt to resolve each specifier: `lib` is deliberately
      // absent, because plenty of packages publish `pkg/lib/x` as real API.
      //
      // Extending this means EDITING THIS LIST, not adding the rule to a later
      // block — flat config replaces a rule's options rather than merging them,
      // so a second `no-restricted-imports` anywhere below would silently drop
      // these patterns for the files it matches.
      'no-restricted-imports': [
        'error',
        {
          patterns: [
            {
              group: ['**/dist/**', '**/internal/**', '**/src/**', '**/esm/**'],
              message:
                "Import from the package root or a published subpath — a path into a package's build output is not an entry point and can move in a patch release.",
            },
          ],
        },
      ],
    },
  },
  {
    // `util/` is the pure layer, and `type:ui` may reach it. That combination is
    // only sound while `util/` holds no state: the moment a signal lives behind
    // an `@Injectable` there, a dumb component can read store-derived state
    // through a channel Sheriff cannot see — it sees `data -> util` and
    // `ui -> util` as two unrelated legal edges, never the channel between them.
    // Hahnekamp's `type:data` is "state management AND the services that hold
    // it"; his `util` is pure functions. This is that line, enforced.
    //
    // A `@Pipe` is deliberately still legal: a pure pipe IS a pure function with
    // a decorator. Module-level side effects are legal too.
    files: ['src/app/*/util/**/*.ts', 'src/app/@shared/util/**/*.ts'],
    rules: {
      'no-restricted-syntax': [
        'error',
        {
          selector: 'Decorator[expression.callee.name="Injectable"]',
          message:
            'util/ holds no injectable service — a service that holds state or reaches a platform API belongs in data/. See CLAUDE.md.',
        },
      ],
    },
  },
  {
    // ── Transitional. DELETE THIS WHOLE BLOCK WHEN `electron/` GOES. ──
    // `electron/` is the Angular-13-era Electron main process, kept only so the
    // app stays runnable during the Angular port. It is replaced wholesale by
    // `src-tauri/` (Rust), so holding never-formatted 2023 Node code to the same
    // bar as new Angular code buys nothing but churn in files with a delete date.
    //
    // naming-convention: `electron/bridge/shared.model.ts` keeps its I-prefixed
    // names ON PURPOSE. It is the wire mirror of `src/app/@shared/model/`, which
    // carries the same JSON shape under the new names. Two spellings of one
    // contract is the point — renaming here would mean editing the process we
    // are deleting, and the mirror is exactly what the Rust backend will need.
    //
    // require-await / no-floating-promises: pre-existing, non-load-bearing —
    // `async` methods that never await, and unawaited fire-and-forget IPC sends.
    // Real smells, but in code with a delete date; the Rust port makes them
    // unrepresentable via `Result` rather than fixing them here.
    files: ['electron/**/*.ts'],
    rules: {
      '@typescript-eslint/naming-convention': 'off',
      '@typescript-eslint/require-await': 'off',
      '@typescript-eslint/no-floating-promises': 'off',
    },
  },
  {
    files: ['**/*.html'],
    extends: [
      ...angular.configs.templateRecommended,
      ...angular.configs.templateAccessibility,
    ],
  },
  {
    files: ['**/*.ts', '**/*.html'],
    plugins: { prettier: prettierPlugin },
    rules: {
      'prettier/prettier': 'error',
    },
  },
  // Last, so it switches off every stylistic rule the presets above turned on
  // that prettier already decides.
  prettierConfig
);
