// ESLint v9 flat config — quality gate for TerranSoul's frontend.
//
// Replaces the previous ad-hoc `scripts/check-file-sizes.mjs` with the
// industry-standard ESLint stack (`eslint` + `eslint-plugin-vue` +
// `typescript-eslint`). The same per-file size budget the script used
// to enforce is now expressed via the built-in `max-lines` rule.
//
// Per-file budgets (matches scripts/check-file-sizes.mjs semantics):
//   - Vue SFC : 800 lines
//   - TS / JS : 1000 lines
//
// Pre-existing oversized files keep working via per-pattern overrides
// at the bottom of this file. The long-term goal is to shrink that
// override list to zero through targeted refactors — never widen it
// without a follow-up issue.

import js from '@eslint/js';
import vue from 'eslint-plugin-vue';
import tseslint from 'typescript-eslint';
import vueParser from 'vue-eslint-parser';
import globals from 'globals';

const MAX_LINES_VUE = 800;
const MAX_LINES_TS = 1000;

const sharedMaxLines = (max) => [
  'error',
  { max, skipBlankLines: true, skipComments: true },
];

export default [
  // ── Ignores ─────────────────────────────────────────────────────────────
  {
    ignores: [
      'dist/**',
      'node_modules/**',
      'src-tauri/**',          // Rust — bounded by clippy
      '*.config.js',
      '*.config.cjs',
      '*.config.mjs',
      'docs/**',
      'tests/**',
      'playwright/**',
      'coverage/**',
      'target/**',             // Rust build artifacts (cargo target dirs)
      'target-copilot-bench/**',
      'target-mcp/**',
      'mcp-data/**',           // Generated MCP runtime data
      '.cache/**',             // Build/benchmark caches
    ],
  },

  // ── Baseline JS rules ───────────────────────────────────────────────────
  js.configs.recommended,

  // ── TypeScript rules (recommended, non-type-checked for speed) ──────────
  ...tseslint.configs.recommended,

  // ── Vue 3 flat/recommended ──────────────────────────────────────────────
  ...vue.configs['flat/recommended'],

  // ── TS/JS files ─────────────────────────────────────────────────────────
  {
    files: ['**/*.{ts,mts,cts,js,mjs,cjs}'],
    languageOptions: {
      parser: tseslint.parser,
      ecmaVersion: 2022,
      sourceType: 'module',
      globals: { ...globals.browser, ...globals.es2022, ...globals.node },
    },
    rules: {
      'max-lines': sharedMaxLines(MAX_LINES_TS),
      // Soften noisy rules for a brownfield codebase that's never been
      // linted. We can tighten these later in dedicated chunks.
      '@typescript-eslint/no-explicit-any': 'off',
      '@typescript-eslint/no-unused-vars': [
        'warn',
        { argsIgnorePattern: '^_', varsIgnorePattern: '^_' },
      ],
      '@typescript-eslint/no-unused-expressions': 'off',
      '@typescript-eslint/ban-ts-comment': 'off',
      'no-empty': ['warn', { allowEmptyCatch: true }],
      'no-control-regex': 'off',
      'no-useless-escape': 'warn',
      'no-prototype-builtins': 'warn',
      // TypeScript itself catches undefined identifiers; ESLint's `no-undef`
      // double-checks but produces false positives on TS-only types like
      // `BlobPart`. The typescript-eslint team explicitly recommends
      // disabling it. https://typescript-eslint.io/troubleshooting/faqs/eslint
      'no-undef': 'off',
      // Allow forward references — common when `let x` is assigned inside
      // a closure declared before the assignment site.
      'prefer-const': ['error', { ignoreReadBeforeAssign: true }],
    },
  },

  // ── Vue SFC files ───────────────────────────────────────────────────────
  {
    files: ['**/*.vue'],
    languageOptions: {
      parser: vueParser,
      parserOptions: {
        parser: tseslint.parser,
        ecmaVersion: 2022,
        sourceType: 'module',
        extraFileExtensions: ['.vue'],
      },
      globals: { ...globals.browser, ...globals.es2022 },
    },
    rules: {
      'max-lines': sharedMaxLines(MAX_LINES_VUE),
      // Mirror the JS overrides above so a TypeScript expression inside a
      // Vue <script> block obeys the same brownfield-friendly defaults.
      '@typescript-eslint/no-explicit-any': 'off',
      '@typescript-eslint/no-unused-vars': [
        'warn',
        { argsIgnorePattern: '^_', varsIgnorePattern: '^_' },
      ],
      'vue/multi-word-component-names': 'off',
      'vue/no-v-html': 'warn',
      'no-undef': 'off',
      'prefer-const': ['error', { ignoreReadBeforeAssign: true }],
    },
  },

  // ── Scripts (Playwright capture, helpers, generators) ────────────────────
  // These are plain JS (not TypeScript) — enable CodeQL-equivalent rules so
  // the CI gate catches the same issues GitHub's quality dashboard flags.
  {
    files: ['scripts/**/*.{mjs,cjs,js}'],
    languageOptions: {
      ecmaVersion: 2022,
      sourceType: 'module',
      globals: { ...globals.node, ...globals.es2022 },
    },
    rules: {
      'max-lines': 'off',
      '@typescript-eslint/no-require-imports': 'off', // CJS scripts need require()
      '@typescript-eslint/no-unused-vars': [
        'error',
        { argsIgnorePattern: '^_', varsIgnorePattern: '^_' },
      ],
      'no-shadow': 'error',
      'no-const-assign': 'error',
      'no-unused-vars': [
        'error',
        { argsIgnorePattern: '^_', varsIgnorePattern: '^_' },
      ],
    },
  },

  // ── Test files — be lenient ─────────────────────────────────────────────
  {
    files: ['**/*.test.{ts,js}', '**/*.spec.{ts,js}'],
    languageOptions: { globals: { ...globals.node } },
    rules: {
      'max-lines': 'off',
      '@typescript-eslint/no-unused-vars': 'off',
    },
  },

  // ── Pre-existing oversized files (allowlist) ────────────────────────────
  // Each entry is a temporary exception: the file already exceeded its
  // budget when ESLint was introduced. Refactor each one in a focused PR
  // and DELETE the entry from this list. Do NOT widen this list — open
  // an issue and discuss instead.
  //
  // Note: Rust files are bounded by `clippy::too_many_lines` (configured
  // via `src-tauri/clippy.toml`) — ESLint never sees them.
  {
    files: [
      'src/components/CharacterViewport.vue',
      'src/components/GraphNodeCrudPanel.vue',
      'src/components/MemoryGraph.vue',
      'src/components/MemoryGalaxy.vue',
      'src/components/ModelPanel.vue',
      'src/components/SkillConstellation.vue',
      'src/components/KnowledgeQuestDialog.vue',
      'src/views/BrainView.vue',
      'src/views/ChatView.vue',
      'src/views/MarketplaceView.vue',
      'src/views/MemoryView.vue',
      'src/views/PetOverlayView.vue',
      'src/components/SplashScreen.vue',
      'src/stores/skill-tree.ts',
      'src/stores/conversation.ts',
    ],
    rules: { 'max-lines': 'off' },
  },
];
