-- 2026-05-06: durable verdict + 12-chunk plan for TerranSoul DB strategy
-- (offline mobile + future hive). See rules/milestones.md Phase 42.

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'DB STRATEGY VERDICT (2026-05-06): SQLite is NOT a bottleneck for TerranSoul as the local engine, even at 1M+ memories and on offline mobile. After Phase 41 tuning, the companion is CPU/embedding-bound long before SQLite-bound. SQLite IS the wrong shape for "hive" multi-user federation and distributed jobs. Final posture is two-layer storage: (1) Local layer on every device (desktop + iOS + Android) keeps tuned SQLite + WAL as authoritative source of truth; pure-Rust ANN fallback ships on mobile because usearch C++ build is fragile there; (2) Sync layer between a single user own devices promotes memories + KG edges to CRDTs (LWW for memory rows, 2P-Set/OR-Set for edges) replicated as op-logs over the existing QUIC/WS LinkManager — no server required; (3) Hive layer is opt-in: a reference Tonic gRPC relay backed by Postgres + pgvector accepts Ed25519-signed knowledge bundles, runs a job queue, and federates only when configured. The local app never depends on the hive. Reject standalone vector services (Qdrant/Milvus/Pinecone) for the local app per existing decision (brain-advanced-design.md row 18). Keep usearch HNSW locally; pgvector HNSW on the hive layer. Existing alt backends (postgres.rs/mssql.rs/cassandra.rs) currently lack RRF / FTS5 / KG / contextual-retrieval parity; bringing Postgres to parity is a hive prerequisite. Memory store rows already carry updated_at + origin_device columns but no merge function — wire that.',
  'verdict,database,sqlite,postgres,hive,mobile,crdt,federation,phase-42,non-negotiable',
  10, 'fact', 1746489600000, 'long', 1.0, 'memory', 'semantic'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'PHASE 42 PLAN (2026-05-06) — DB strategy for offline mobile + future hive: 12 ordered chunks across 4 sub-phases, lands AFTER Phase 41. A. Mobile-safe local engine: 42.1 pure-Rust ANN fallback for iOS/Android, 42.2 mobile SQLite/WAL hardening. B. Memory CRDT: 42.3 memory rows as LWW CRDT, 42.4 KG edges as 2P-Set/OR-Set CRDT, 42.5 op-log replication over LinkManager. C. Distributed backend parity: 42.6 Postgres parity for RRF/FTS/KG/contextual, 42.7 pgvector HNSW + bench, 42.8 SQLite+Postgres CI matrix. D. Hive layer (opt-in): 42.9 hive protocol spec with Ed25519-signed bundles, 42.10 reference Tonic gRPC relay in crates/hive-relay/ with Postgres+pgvector, 42.11 job queue + capability gates, 42.12 share_scope ACL (private/paired/hive) with redaction tests.',
  'plan,phase-42,database,mobile,crdt,hive,federation,postgres,pgvector,job-distribution,non-negotiable',
  10, 'fact', 1746489600000, 'long', 1.0, 'memory', 'procedural'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT plan.id, verdict.id, 'derived_from', 1.0, 'auto', 1746489600000, 'auto'
FROM memories plan
CROSS JOIN memories verdict
WHERE plan.content LIKE 'PHASE 42 PLAN (2026-05-06)%'
  AND verdict.content LIKE 'DB STRATEGY VERDICT (2026-05-06)%';
