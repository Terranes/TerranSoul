#!/usr/bin/env node
// Pre-bake translations for Pages/index.html so the language switcher is instant.
//
// - Extracts every data-i18n / data-i18n-html string from the page.
// - Translates each unique English source into every target language via
//   Ollama (free, local). Override with TRANSLATE_PROVIDER=anthropic|claude-code.
// - Writes the resulting map back into the inline
//   <script type="application/json" id="i18n-builtin">{...}</script> block.
//
// Usage:
//   node scripts/build-page-i18n.mjs               # all supported languages
//   node scripts/build-page-i18n.mjs vi zh ja      # subset
//   TRANSLATE_MODEL=gemma4:e4b node scripts/build-page-i18n.mjs vi
//
// Progress is printed to stdout in the form:
//   [i18n] vi (Vietnamese) — batch 3/12 (15 strings) ...
//   [i18n] vi (Vietnamese) done in 92.4s, 178/178 strings, cache hits 0

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO = path.resolve(__dirname, '..');
const PAGE = path.join(REPO, 'Pages', 'index.html');

const SUPPORTED = ['vi','zh','zh-TW','ja','ko','es','fr','de','pt','it','ru','tr','id','th','hi','ar'];
const LANG_NAME = {
  'vi':'Vietnamese','zh':'Simplified Chinese','zh-TW':'Traditional Chinese',
  'ja':'Japanese','ko':'Korean','es':'Spanish','fr':'French','de':'German',
  'pt':'Portuguese','it':'Italian','ru':'Russian','tr':'Turkish',
  'id':'Indonesian','th':'Thai','hi':'Hindi','ar':'Arabic'
};
const KEEP = [
  'TerranSoul','OpenClaw','Hermes','Hermes Agent','ChatGPT','Claude','OpenAI','Anthropic',
  'MCP','RAG','GraphRAG','HNSW','CRDT','SQLite','FTS5','HyDE','RRF','SLO','CAP',
  'Ollama','Pollinations','OpenRouter','Gemini','Groq','Copilot','Cursor','Codex',
  'mxbai-embed-large','nomic-embed-text','BrainGateway','memory_edges','brain_kg_neighbors',
  'brain_search','brain_ingest_lesson','code_query','code_impact','multi_hop_search_memories',
  'episodic','semantic','procedural','judgment','private','LongMemEval-S','LoCoMo','MTEB',
  'Aspen','Maine','Charleston','Savannah','Banff','San Diego','Boston','BOS','YYC','Sam',
  'Cognee','GitHub','Discord','MIT','Tauri'
];

const args = process.argv.slice(2);
const targets = (args.length ? args : SUPPORTED).filter(l => SUPPORTED.includes(l));
if (!targets.length){
  console.error('No valid target languages. Choose from:', SUPPORTED.join(', '));
  process.exit(1);
}

const PROVIDER = process.env.TRANSLATE_PROVIDER || 'ollama';
const OLLAMA_URL = process.env.OLLAMA_URL || 'http://localhost:11434';
const OLLAMA_MODEL = process.env.TRANSLATE_MODEL || 'gemma3:4b';
const BATCH_CHARS = Number(process.env.TRANSLATE_BATCH_CHARS || 1400);

// ---------- extract strings ----------
function extractStrings(html){
  const strings = new Set();
  // Matches <tag ... data-i18n[-html]?...>INNER</tag> for inner without nested same-tag.
  // We rely on the fact that data-i18n appears on leaf-ish elements (span, p, h*, em, strong, a, li, h6, div).
  // For data-i18n-html we keep the inner HTML; for data-i18n we strip child tags too (textContent).
  const TAG_RX = /<(span|p|h[1-6]|em|strong|a|li|div|code)\b([^>]*\bdata-i18n(?:-html)?\b[^>]*)>([\s\S]*?)<\/\1>/g;
  let m;
  while ((m = TAG_RX.exec(html))){
    const isHtml = /\bdata-i18n-html\b/.test(m[2]);
    const inner = m[3].trim();
    if (!inner) continue;
    if (isHtml){
      strings.add(inner);
    } else {
      // strip tags for text mode (textContent equivalent)
      const text = inner.replace(/<[^>]+>/g, '').replace(/\s+/g, ' ').trim();
      if (text) strings.add(text);
    }
  }
  return Array.from(strings);
}

// ---------- ollama batch translator ----------
async function ollamaChat(prompt){
  const res = await fetch(`${OLLAMA_URL}/api/chat`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      model: OLLAMA_MODEL,
      stream: false,
      options: { temperature: 0.2, num_ctx: 8192 },
      messages: [
        { role: 'system', content: 'You are a careful, idiomatic translator. Reply with ONLY a JSON array of strings, same length and order as input. No prose. No markdown fences.' },
        { role: 'user', content: prompt },
      ],
    }),
  });
  if (!res.ok) throw new Error(`ollama ${res.status} ${res.statusText}`);
  const json = await res.json();
  return json.message?.content || '';
}

function buildPrompt(strings, lang){
  const langName = LANG_NAME[lang] || lang;
  return `Translate each item in the JSON array from English to ${langName}.

Rules:
- Return ONLY a JSON array of translated strings. No prose, no markdown, no code fences.
- Same length as input. Same order.
- Preserve any HTML tags exactly as they appear (e.g. <strong>, <em>, <code>, <a href="...">). Translate only visible text inside tags.
- Keep these technical / brand names untranslated: ${KEEP.join(', ')}.
- Keep numbers, currency symbols, and code snippets unchanged.
- Preserve quotation marks and apostrophes.

Input:
${JSON.stringify(strings)}`;
}

function tryParseArray(out){
  try { return JSON.parse(out); } catch {}
  const m = out.match(/\[[\s\S]*\]/);
  if (m) { try { return JSON.parse(m[0]); } catch {} }
  return null;
}

const VIET_RX = /([aeiouyăâêôơưAEIOUYĂÂÊÔƠƯ])/;
function fixUnicode(s){
  if (typeof s !== 'string') return s;
  return s
    .replace(new RegExp(VIET_RX.source + '\u00B4', 'g'), '$1\u0301')
    .replace(new RegExp(VIET_RX.source + '\u0060', 'g'), '$1\u0300')
    .replace(new RegExp(VIET_RX.source + '\u02DC', 'g'), '$1\u0303')
    .replace(new RegExp(VIET_RX.source + '\u02C7', 'g'), '$1\u0309')
    .normalize('NFC');
}

async function translateBatch(strings, lang, attempt = 0){
  const prompt = buildPrompt(strings, lang);
  let out;
  try { out = await ollamaChat(prompt); }
  catch (e){
    if (attempt < 2){
      console.log(`    [i18n] ${lang} batch error (${e.message}) — retrying ${attempt+1}/2`);
      await new Promise(r => setTimeout(r, 1500));
      return translateBatch(strings, lang, attempt + 1);
    }
    throw e;
  }
  const arr = tryParseArray(out);
  if (!Array.isArray(arr) || arr.length !== strings.length){
    if (attempt < 2){
      console.log(`    [i18n] ${lang} parse mismatch (got ${Array.isArray(arr)?arr.length:'non-array'}, want ${strings.length}) — retrying ${attempt+1}/2`);
      return translateBatch(strings, lang, attempt + 1);
    }
    console.warn(`    [i18n] ${lang} giving up on batch of ${strings.length} — keeping English`);
    return strings.slice();
  }
  return arr.map(fixUnicode);
}

function makeBatches(strings){
  const batches = [];
  let cur = [], curLen = 0;
  for (const s of strings){
    if (curLen && (curLen + s.length > BATCH_CHARS || cur.length >= 20)){
      batches.push(cur); cur = []; curLen = 0;
    }
    cur.push(s); curLen += s.length + 4;
  }
  if (cur.length) batches.push(cur);
  return batches;
}

// ---------- main ----------
async function main(){
  const html = fs.readFileSync(PAGE, 'utf8');
  const strings = extractStrings(html);
  console.log(`[i18n] extracted ${strings.length} unique English strings from ${path.relative(REPO, PAGE)}`);
  console.log(`[i18n] provider=${PROVIDER} model=${OLLAMA_MODEL}`);
  console.log(`[i18n] target languages (${targets.length}): ${targets.join(', ')}`);

  // Read existing builtin block so we can preserve work-in-progress entries.
  const blockRx = /<script type="application\/json" id="i18n-builtin">([\s\S]*?)<\/script>/;
  const blockMatch = html.match(blockRx);
  if (!blockMatch){
    console.error('[i18n] could not find <script type="application/json" id="i18n-builtin"> in page');
    process.exit(2);
  }
  let existing = {};
  try { existing = JSON.parse(blockMatch[1].trim() || '{}'); }
  catch { existing = {}; console.warn('[i18n] existing builtin block was unparseable, starting fresh'); }

  const out = Object.assign({}, existing);

  for (let li = 0; li < targets.length; li++){
    const lang = targets[li];
    const langName = LANG_NAME[lang] || lang;
    const have = out[lang] || {};
    const need = strings.filter(s => !have[s]);
    const t0 = Date.now();
    console.log(`\n[i18n] ${li+1}/${targets.length} ${lang} (${langName}) — ${have && Object.keys(have).length} cached, ${need.length} to translate`);

    if (need.length === 0){
      out[lang] = have;
      console.log(`[i18n] ${lang} already complete`);
      continue;
    }

    const batches = makeBatches(need);
    const map = Object.assign({}, have);
    for (let i = 0; i < batches.length; i++){
      const batch = batches[i];
      process.stdout.write(`    [i18n] ${lang} batch ${i+1}/${batches.length} (${batch.length} strings, ~${batch.reduce((a,s)=>a+s.length,0)} chars)... `);
      const bt0 = Date.now();
      const translated = await translateBatch(batch, lang);
      batch.forEach((src, k) => { map[src] = translated[k] || src; });
      out[lang] = map;
      // Persist after each batch so a crash doesn't lose work.
      writeBuiltin(html, out);
      console.log(`${((Date.now()-bt0)/1000).toFixed(1)}s`);
    }
    const secs = ((Date.now()-t0)/1000).toFixed(1);
    console.log(`[i18n] ${lang} done in ${secs}s — ${Object.keys(map).length}/${strings.length} translated`);
  }

  console.log(`\n[i18n] all done. Languages baked: ${Object.keys(out).join(', ')}`);
}

function writeBuiltin(originalHtml, mapObj){
  const blockRx = /<script type="application\/json" id="i18n-builtin">([\s\S]*?)<\/script>/;
  // Read current page from disk so concurrent edits don't get clobbered.
  const cur = fs.readFileSync(PAGE, 'utf8');
  // Compact JSON — one line per top-level lang for readable diffs.
  const langs = Object.keys(mapObj).sort();
  const lines = langs.map(l => '  ' + JSON.stringify(l) + ': ' + JSON.stringify(mapObj[l]));
  const json = '{\n' + lines.join(',\n') + '\n}';
  const next = cur.replace(blockRx, `<script type="application/json" id="i18n-builtin">${json}</script>`);
  fs.writeFileSync(PAGE, next, 'utf8');
}

main().catch(e => { console.error('[i18n] fatal:', e); process.exit(1); });
