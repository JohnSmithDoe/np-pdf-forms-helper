// @ts-check
const { defineConfig, globalIgnores } = require('eslint/config');
const angular = require('angular-eslint');
const sheriff = require('@softarc/eslint-plugin-sheriff');
const prettierPlugin = require('eslint-plugin-prettier');
const prettierConfig = require('eslint-config-prettier');
const unicorn = require('eslint-plugin-unicorn').default;
// Registered by name here because the type-aware rules below are set in THIS
// block: `extends` scopes a plugin to the config element that declared it, so
// inheriting a preset does not make `@typescript-eslint/*` resolvable in a
// sibling `rules`.
const tseslint = require('typescript-eslint');

module.exports = defineConfig(
  globalIgnores(['.angular/**', 'dist/**', 'node_modules/**', 'release/**']),
  sheriff.configs.all,
  {
    files: ['**/*.ts'],
    plugins: { unicorn, '@typescript-eslint': tseslint.plugin },
    extends: [...angular.configs.tsRecommended, unicorn.configs.all],
    processor: angular.processInlineTemplates,
    languageOptions: {
      parserOptions: {
        // Use the TypeScript project service so each file resolves via the
        // tsconfig that owns it instead of one hardcoded `project`.
        project: null,
        projectService: true,
        tsconfigRootDir: __dirname,
      },
    },
    rules: {
      // `Page` for routed components, `Dialog` for the bodies handed to Ionic's
      // ModalController — both are real kinds here and neither reads well as
      // `…Component`.
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
      // These four are the ones with something to catch in an app whose whole
      // IPC surface is promise-returning: an un-awaited promise is how a
      // backend rejection becomes an unhandled-rejection instead of
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
      // Unicorn's `all`, as in np-commlink: every rule on, the exceptions below
      // named one by one. `null` is idiomatic across Angular and RxJS, and it
      // is what serde writes for an absent `Option` on the wire.
      'unicorn/no-null': 'off',
      // `util` is a Sheriff layer name (`util/`, `*.util.ts`), not a shorthand.
      'unicorn/prevent-abbreviations': [
        'error',
        {
          allowList: { util: true, utils: true, prod: true },
          ignore: ['e2e', 'Ref', 'componentProps'],
        },
      ],
      'unicorn/no-useless-undefined': ['error', { checkArguments: false }],
      'unicorn/prefer-export-from': ['error', { checkUsedVariables: false }],
      // Every file opens with a hand-wrapped `─── why ───` block and prettier
      // never reflows a comment, so "unwrapped" means one 300-column line.
      'unicorn/no-manually-wrapped-comments': 'off',
      // A `finally` that balances a counter (`BackendService.call`) has to span
      // the whole body; a one-statement `try` would move work outside it.
      'unicorn/try-complexity': 'off',
      // A published subpath is fine — `@ionic/angular/standalone` and
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
    files: ['**/*.{js,mjs,cjs}'],
    plugins: { unicorn },
    extends: ['unicorn/recommended'],
    rules: {
      // `eslint.config.js` is CommonJS.
      'unicorn/prefer-module': 'off',
      'unicorn/no-null': 'off',
      'unicorn/prevent-abbreviations': [
        'error',
        {
          allowList: { utils: true, prod: true },
          ignore: ['e2e', 'Ref', 'componentProps', 'dir', 'rel', 'doc'],
        },
      ],
      'unicorn/no-useless-undefined': ['error', { checkArguments: false }],
      'unicorn/import-style': [
        'error',
        { styles: { 'node:path': { named: true } } },
      ],
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
