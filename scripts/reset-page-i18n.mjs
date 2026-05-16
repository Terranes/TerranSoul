#!/usr/bin/env node
// Wipe the i18n-builtin block in Pages/index.html back to {} (used after bad Ollama runs).
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const PAGE = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', 'Pages', 'index.html');
const html = fs.readFileSync(PAGE, 'utf8');
const next = html.replace(
  /<script type="application\/json" id="i18n-builtin">[\s\S]*?<\/script>/,
  '<script type="application/json" id="i18n-builtin">{}</script>'
);
fs.writeFileSync(PAGE, next, 'utf8');
console.log('[i18n] reset i18n-builtin block to {}');
