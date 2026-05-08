-- 2026-05-07: record Phase 41.6 completion — re-embed on content update.

UPDATE memories
SET content = 'PHASE 41.6 LESSON (2026-05-07): Re-embed on content update shipped. MemoryStore::update now detects content_changed before consuming the Option. After field updates, if content changed: (1) clears embedding column to NULL, (2) removes stale vector from ANN index via idx.remove(id), (3) enqueues the id in pending_embeddings for the background embedding worker to re-process. Non-content updates (tags, importance, memory_type) do not disturb embeddings. MemoryUpdate derives Default for ergonomic test construction. 5 new tests cover: embedding cleared, pending_embeddings enqueued, ANN removal, tags-only preservation, and full round-trip update→clear→re-embed→search. Total: 2320 Rust tests pass, 1738 vitest, clippy clean.',
    created_at = 1746666000000
WHERE content LIKE 'PHASE 41.5 LESSON%';
