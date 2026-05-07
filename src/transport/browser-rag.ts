import type { MemoryEntry, MemoryStats, NewMemory } from '../types';

const DB_NAME = 'terransoul-browser-rag';
const STORE_NAME = 'memories';
const LOCAL_STORAGE_KEY = 'ts.browser.rag.memories.v1';
const EMBEDDING_DIMS = 128;
const RRF_K = 60;

interface BrowserMemoryRecord extends MemoryEntry {
  embedding: number[];
}

export interface BrowserRagSyncPayload {
  version: 1;
  exported_at: number;
  memories: BrowserMemoryRecord[];
}

function now(): number {
  return Date.now();
}

function canUseIndexedDb(): boolean {
  return typeof indexedDB !== 'undefined';
}

function openDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, 1);
    request.onerror = () => reject(request.error);
    request.onupgradeneeded = () => {
      const db = request.result;
      if (!db.objectStoreNames.contains(STORE_NAME)) {
        db.createObjectStore(STORE_NAME, { keyPath: 'id' });
      }
    };
    request.onsuccess = () => resolve(request.result);
  });
}

async function readIndexedDb(): Promise<BrowserMemoryRecord[]> {
  const db = await openDb();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(STORE_NAME, 'readonly');
    const request = tx.objectStore(STORE_NAME).getAll();
    request.onerror = () => reject(request.error);
    request.onsuccess = () => resolve(request.result as BrowserMemoryRecord[]);
    tx.oncomplete = () => db.close();
  });
}

async function writeIndexedDb(records: BrowserMemoryRecord[]): Promise<void> {
  const db = await openDb();
  await new Promise<void>((resolve, reject) => {
    const tx = db.transaction(STORE_NAME, 'readwrite');
    const store = tx.objectStore(STORE_NAME);
    store.clear();
    for (const record of records) store.put(record);
    tx.onerror = () => reject(tx.error);
    tx.oncomplete = () => resolve();
  });
  db.close();
}

function readLocalStorage(): BrowserMemoryRecord[] {
  if (typeof localStorage === 'undefined') return [];
  try {
    const raw = localStorage.getItem(LOCAL_STORAGE_KEY);
    return raw ? JSON.parse(raw) as BrowserMemoryRecord[] : [];
  } catch {
    return [];
  }
}

function writeLocalStorage(records: BrowserMemoryRecord[]): void {
  if (typeof localStorage === 'undefined') return;
  try {
    localStorage.setItem(LOCAL_STORAGE_KEY, JSON.stringify(records));
  } catch {
    // Browser storage quota can be exceeded; keep in-memory Pinia state.
  }
}

async function readRecords(): Promise<BrowserMemoryRecord[]> {
  if (canUseIndexedDb()) {
    try {
      const records = await readIndexedDb();
      if (records.length > 0) return records;
    } catch {
      // Fall through to localStorage for jsdom/private-mode browsers.
    }
  }
  return readLocalStorage();
}

async function writeRecords(records: BrowserMemoryRecord[]): Promise<void> {
  writeLocalStorage(records);
  if (canUseIndexedDb()) {
    try {
      await writeIndexedDb(records);
    } catch {
      // localStorage already has the same payload.
    }
  }
}

function tokenize(text: string): string[] {
  return text
    .toLowerCase()
    .normalize('NFKD')
    .replace(/[^\p{L}\p{N}\s-]/gu, ' ')
    .split(/\s+/)
    .filter((token) => token.length > 1);
}

function hashToken(token: string): number {
  let hash = 2166136261;
  for (let i = 0; i < token.length; i += 1) {
    hash ^= token.charCodeAt(i);
    hash = Math.imul(hash, 16777619);
  }
  return hash >>> 0;
}

export async function embedTextForBrowser(text: string): Promise<number[]> {
  const vector = new Array<number>(EMBEDDING_DIMS).fill(0);
  for (const token of tokenize(text)) {
    const hash = hashToken(token);
    const index = hash % EMBEDDING_DIMS;
    vector[index] += (hash & 1) === 0 ? 1 : -1;
  }
  const norm = Math.hypot(...vector) || 1;
  return vector.map((value) => value / norm);
}

function cosine(a: number[], b: number[]): number {
  const len = Math.min(a.length, b.length);
  let dot = 0;
  for (let i = 0; i < len; i += 1) dot += a[i] * b[i];
  return dot;
}

function keywordScore(query: string, content: string): number {
  const q = new Set(tokenize(query));
  if (q.size === 0) return 0;
  const c = new Set(tokenize(content));
  let hits = 0;
  for (const token of q) if (c.has(token)) hits += 1;
  return hits / q.size;
}

function recencyScore(createdAt: number): number {
  const ageDays = Math.max(0, (now() - createdAt) / 86_400_000);
  return 1 / (1 + ageDays / 30);
}

function rrfScore(rank: number): number {
  return 1 / (RRF_K + rank + 1);
}

function rankedIds(records: BrowserMemoryRecord[], score: (record: BrowserMemoryRecord) => number): Map<number, number> {
  return new Map(
    records
      .map((record) => ({ id: record.id, score: score(record) }))
      .filter((item) => item.score > 0)
      .sort((a, b) => b.score - a.score)
      .map((item, index) => [item.id, rrfScore(index)]),
  );
}

export function buildBrowserHydePrompt(query: string): string {
  return `Write a concise hypothetical answer that would contain the facts needed to answer this query. Query: ${query}`;
}

export async function addBrowserMemory(input: NewMemory): Promise<MemoryEntry> {
  const records = await readRecords();
  const timestamp = now();
  const id = Math.max(0, ...records.map((record) => record.id)) + 1;
  const record: BrowserMemoryRecord = {
    id,
    content: input.content,
    tags: input.tags,
    importance: input.importance,
    memory_type: input.memory_type,
    created_at: timestamp,
    last_accessed: null,
    access_count: 0,
    tier: 'long',
    decay_score: 1,
    session_id: null,
    parent_id: null,
    token_count: tokenize(input.content).length,
    confidence: 1.0,
    embedding: await embedTextForBrowser(input.content),
  };
  await writeRecords([record, ...records]);
  return stripEmbedding(record);
}

export async function listBrowserMemories(): Promise<MemoryEntry[]> {
  return (await readRecords()).map(stripEmbedding);
}

export async function clearBrowserMemories(): Promise<void> {
  await writeRecords([]);
}

export async function browserKeywordSearch(query: string, limit = 10): Promise<MemoryEntry[]> {
  const records = await readRecords();
  return records
    .map((record) => ({ record, score: keywordScore(query, record.content) }))
    .filter((item) => item.score > 0)
    .sort((a, b) => b.score - a.score)
    .slice(0, limit)
    .map((item) => stripEmbedding(touchRecord(item.record)));
}

export async function browserHybridSearch(query: string, limit = 5, hypothetical?: string): Promise<MemoryEntry[]> {
  const records = await readRecords();
  if (records.length === 0) return [];
  const queryEmbedding = await embedTextForBrowser(hypothetical || query);
  const vectorRanks = rankedIds(records, (record) => Math.max(0, cosine(queryEmbedding, record.embedding)));
  const keywordRanks = rankedIds(records, (record) => keywordScore(query, record.content));
  const freshnessRanks = rankedIds(records, (record) => recencyScore(record.created_at));
  const scored = records.map((record) => {
    const vector = Math.max(0, cosine(queryEmbedding, record.embedding));
    const keyword = keywordScore(query, record.content);
    const recency = recencyScore(record.created_at);
    const weighted = vector * 0.4 + keyword * 0.2 + recency * 0.15 + (record.importance / 5) * 0.1 + record.decay_score * 0.1 + 0.05;
    const fused = (vectorRanks.get(record.id) ?? 0) + (keywordRanks.get(record.id) ?? 0) + (freshnessRanks.get(record.id) ?? 0);
    return { record, score: weighted + fused * 10 };
  });
  const results = scored
    .filter((item) => item.score > 0.18)
    .sort((a, b) => b.score - a.score)
    .slice(0, limit)
    .map((item) => touchRecord(item.record));
  if (results.length > 0) {
    const touchedIds = new Set(results.map((record) => record.id));
    await writeRecords(records.map((record) => touchedIds.has(record.id) ? results.find((item) => item.id === record.id) ?? record : record));
  }
  return results.map(stripEmbedding);
}

export async function browserMemoryStats(): Promise<MemoryStats> {
  const records = await readRecords();
  const totalTokens = records.reduce((sum, record) => sum + record.token_count, 0);
  const avgDecay = records.length === 0 ? 0 : records.reduce((sum, record) => sum + record.decay_score, 0) / records.length;
  return {
    total: records.length,
    short_count: records.filter((record) => record.tier === 'short').length,
    working_count: records.filter((record) => record.tier === 'working').length,
    long_count: records.filter((record) => record.tier === 'long').length,
    total_tokens: totalTokens,
    avg_decay: avgDecay,
  };
}

export async function exportBrowserRagSyncPayload(): Promise<BrowserRagSyncPayload> {
  return { version: 1, exported_at: now(), memories: await readRecords() };
}

export async function importBrowserRagSyncPayload(payload: BrowserRagSyncPayload): Promise<number> {
  if (payload.version !== 1) return 0;
  const existing = await readRecords();
  const byId = new Map(existing.map((record) => [record.id, record]));
  for (const record of payload.memories) byId.set(record.id, record);
  const merged = [...byId.values()].sort((a, b) => b.created_at - a.created_at);
  await writeRecords(merged);
  return merged.length;
}

function touchRecord(record: BrowserMemoryRecord): BrowserMemoryRecord {
  return { ...record, last_accessed: now(), access_count: record.access_count + 1 };
}

function stripEmbedding(record: BrowserMemoryRecord): MemoryEntry {
  const { embedding: _embedding, ...entry } = record;
  return entry;
}
