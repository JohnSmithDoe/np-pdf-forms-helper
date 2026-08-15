// np-pdf-forms-helper — the style layer's gate. `.mjs` rather than
// `.stylelintrc.json` because every switched-off rule below is a decision, and
// JSON cannot hold the reason next to it (stylelint rejects unknown keys, so a
// "_comment" comes back as "Unknown rule").
//
// Why stylelint at all, when every other gate is an eslint rule: ESLint cannot
// read SCSS. `@eslint/css` is a CSS parser and chokes on `//` line comments,
// which are not CSS at all — with `tolerant: true` it silently skips whatever
// it failed to parse, which is worse than not running. postcss-scss understands
// SCSS by design, so stylelint sees the file the compiler sees.

export default {
  extends: ['stylelint-config-standard-scss'],
  // Quarantined Angular 13 styles, ported into src/app incrementally — same
  // reasoning as the eslint `globalIgnores` entry. Stylelint's own key for it.
  // `_theme-colors.scss` is emitted by `ng generate @angular/material:m3-theme`
  // and says so in its own header. Hand-formatting it would be reverted the next
  // time the palette is regenerated from a new seed colour, so the file is not
  // ours to style — only to re-generate.
  ignoreFiles: ['src/_legacy/**', 'src/theme/_theme-colors.scss'],
  rules: {
    // Prettier owns formatting, and already runs on SCSS in the pre-commit hook
    // and in CI. Leaving these on means two tools with opinions about blank
    // lines, and prettier is the one that can fix them.
    'rule-empty-line-before': null,
    'at-rule-empty-line-before': null,
    'declaration-empty-line-before': null,
    'custom-property-empty-line-before': null,
    'comment-empty-line-before': null,
    'scss/double-slash-comment-empty-line-before': null,

    // The house style is a `why` banner above the first code token, and a
    // banner separates its paragraphs with a bare `//`. That reads as an empty
    // comment to stylelint, so the rule fires on every well-formed banner.
    'scss/comment-no-empty': null,

    // Reconfigured, NOT disabled: the app is kebab-case BEM everywhere.
    'selector-class-pattern': [
      '^[a-z][a-z0-9]*(-[a-z0-9]+)*(__[a-z0-9]+(-[a-z0-9]+)*)?(--[a-z0-9]+(-[a-z0-9]+)*)?$',
      {
        message:
          'Expected class selector to be kebab-case BEM: block__element--modifier',
      },
    ],
  },
};
