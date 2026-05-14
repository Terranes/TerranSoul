#!/usr/bin/env node
// Dump unique English strings from Pages/index.html to a JSON file for manual translation.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO = path.resolve(__dirname, '..');
const PAGE = path.join(REPO, 'Pages', 'index.html');
const OUT = path.join(REPO, 'Pages', 'i18n-strings.json');

const html = fs.readFileSync(PAGE, 'utf8');
const strings = new Set();
const TAG_RX = /<(span|p|h[1-6]|em|strong|a|li|div|code)\b([^>]*\bdata-i18n(?:-html)?\b[^>]*)>([\s\S]*?)<\/\1>/g;
let m;
while ((m = TAG_RX.exec(html))) {
  const isHtml = /\bdata-i18n-html\b/.test(m[2]);
  const inner = m[3].trim();
  if (!inner) continue;
  if (isHtml) {
    strings.add(inner);
  } else {
    const text = inner.replace(/<[^>]+>/g, '').replace(/\s+/g, ' ').trim();
    if (text) strings.add(text);
  }
}
const arr = Array.from(strings);
fs.writeFileSync(OUT, JSON.stringify(arr, null, 2), 'utf8');
console.log(`[i18n] wrote ${arr.length} unique strings to ${path.relative(REPO, OUT)}`);
