#!/usr/bin/env node
// Combines Pages/i18n/<lang>.json files into the <script id="i18n-builtin"> block.
// Usage: node scripts/combine-page-i18n.mjs
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const PAGE = path.join(ROOT, 'Pages', 'index.html');
const I18N_DIR = path.join(ROOT, 'Pages', 'i18n');
const SOURCE = path.join(ROOT, 'Pages', 'i18n-strings.json');

const sourceStrings = JSON.parse(fs.readFileSync(SOURCE, 'utf8'));
const sourceSet = new Set(sourceStrings);

// Discover language files.
const files = fs.existsSync(I18N_DIR)
  ? fs.readdirSync(I18N_DIR).filter(f => f.endsWith('.json')).sort()
  : [];

const builtin = {};
let totalKeys = 0;
let totalExtras = 0;
for (const file of files) {
  const lang = file.replace(/\.json$/, '');
  const data = JSON.parse(fs.readFileSync(path.join(I18N_DIR, file), 'utf8'));
  const filtered = {};
  let extras = 0;
  for (const [k, v] of Object.entries(data)) {
    if (sourceSet.has(k) && typeof v === 'string' && v.length > 0) {
      filtered[k] = v;
    } else if (!sourceSet.has(k)) {
      extras++;
    }
  }
  builtin[lang] = filtered;
  totalKeys += Object.keys(filtered).length;
  totalExtras += extras;
  const missing = sourceStrings.length - Object.keys(filtered).length;
  console.log(`[i18n] ${lang.padEnd(6)} — ${Object.keys(filtered).length}/${sourceStrings.length} strings${missing ? ` (${missing} missing)` : ''}${extras ? ` (${extras} unused keys ignored)` : ''}`);
}

// One language per line for diff friendliness.
const langKeys = Object.keys(builtin).sort();
const lines = langKeys.map(lang => `  ${JSON.stringify(lang)}: ${JSON.stringify(builtin[lang])}`);
const json = `{\n${lines.join(',\n')}\n}`;

const html = fs.readFileSync(PAGE, 'utf8');
const next = html.replace(
  /<script type="application\/json" id="i18n-builtin">[\s\S]*?<\/script>/,
  `<script type="application/json" id="i18n-builtin">${json}</script>`
);
if (next === html) {
  console.error('[i18n] ERROR: could not find <script id="i18n-builtin"> block');
  process.exit(1);
}
fs.writeFileSync(PAGE, next, 'utf8');
console.log(`[i18n] injected ${langKeys.length} languages, ${totalKeys} total strings into Pages/index.html`);
