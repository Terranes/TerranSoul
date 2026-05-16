-- TerranSoul MCP Brain Seed Data
-- Applied on first `npm run mcp` when memory.db does not exist yet.
-- Contains architectural knowledge so agents can be productive immediately.
--
-- Schema: see src-tauri/src/memory/schema.rs (version 21)
-- Fields: content, tags, importance, memory_type, created_at, tier, decay_score, token_count, category, cognitive_kind


INSERT INTO memories (content, cognitive_kind, tags, importance, created_at, source_hash)
SELECT
  'TerranSoul pet-mode click-through: do not use gl.readPixels() on the WebGL canvas to refine the hit-test under WebView2 transparent layered windows. Premultiplied alpha plus back-buffer timing quirks make the alpha read return 0 even when the model is visibly opaque, which keeps set_ignore_cursor_events(true) permanently and makes every click fall through to the desktop. Use the .pet-character bounding rect as the deterministic hit area instead. Also: do NOT run ensurePassthroughOff (which stops the cursor poll) when entering pet mode — it races with PetOverlayView.onMounted starting the poll, and if the safety-net wins, the poll stays stopped forever. Only run the safety-net when leaving pet mode.',
  'lesson',
  'ux,pet-mode,tauri,webview2,click-through,cursor-passthrough,webgl',
  9,
  strftime('%s','now'),
  'seed:ux-pet-click-fix-2026-05-16'
WHERE NOT EXISTS (SELECT 1 FROM memories WHERE source_hash = 'seed:ux-pet-click-fix-2026-05-16');






INSERT INTO memories (content, cognitive_kind, tags, importance, created_at, source_hash)
SELECT
  'TerranSoul UI refactor lesson: when extracting an inline menu (CharacterViewport.vue FloatingMenu) into a standalone SettingsPanel + a global SettingsModal opened from AppChromeActions, the new modal must embed the full SettingsPanel — not re-implement a minimal subset. Pre-refactor inline menu had 9 sections (View Mode, Quests, Character, Mood/Pose, Background, BGM, Karaoke, ThemePicker, System Info / Audio Controls); the rewritten modal initially shipped only 2. Production fix: render <SettingsPanel> inside the modal body, share a single BGM player via getSharedBgmPlayer() singleton (factory still returns fresh instances for test isolation), wire toggle-system-info / toggle-audio-controls emits to mount the overlays inside the modal, and use :deep(.floating-menu) to neutralise the panel''s floating chrome so it lays out inline.',
  'lesson',
  'ux,settings,modal,panel-refactor,bgm,singleton,vue',
  9,
  strftime('%s','now'),
  'seed:ux-settings-parity-2026-05-16'
WHERE NOT EXISTS (SELECT 1 FROM memories WHERE source_hash = 'seed:ux-settings-parity-2026-05-16');









INSERT INTO memories (content, cognitive_kind, tags, importance, created_at, source_hash)
SELECT
  'TerranSoul brain-config audit 2026-05-16: bench-validated default for the local Ollama embedder is mxbai-embed-large (1024d, 512 tokens), promoted by BENCH-LCM-5 (+3.7pp R@10 overall on LoCoMo vs nomic-embed-text). Code drift fix: six sites still resolved nomic as the default — brain/provider_policy.rs::task_default_model_ollama, brain/mcp_auto_config.rs::EMBEDDING_MODEL, brain/ollama_agent.rs::PREFERRED_EMBED_MODEL + EMBED_MODEL_FALLBACKS + LATE_CHUNK_MODEL_FALLBACKS, commands/brain.rs preferred-embed fallback string, bin/longmemeval_ipc.rs::DEFAULT_EMBED_MODEL. All updated to mxbai-embed-large; nomic-embed-text demoted to lightweight 768d/8192-token fallback in EMBED_MODEL_FALLBACKS. Existing users keep their persisted ActiveModelState; switch only affects first-run + resolver cache misses. Other bench-validated defaults already correct: RRF k=60, rerank pool=30 (LCM-9: 50 regressed -4pp), rerank threshold=0.55 (gemma3:4b is bimodal at temp 0), relevance_threshold=0.30, HyDE class-gated on Semantic+Episodic only (LCM-10), enable_kg_boost=false (KG-2: 0pp lift / 2x latency on adversarial), contextual_retrieval=false (LCM-11: missed acceptance ~9pp NDCG short).',
  'lesson',
  'brain-config,bench,defaults,mxbai-embed-large,embeddings,rrf,rerank,hyde,2026-05-16',
  9,
  strftime('%s','now'),
  'seed:brain-config-audit-2026-05-16'
WHERE NOT EXISTS (SELECT 1 FROM memories WHERE source_hash = 'seed:brain-config-audit-2026-05-16');

INSERT INTO memories (content, cognitive_kind, tags, importance, created_at, source_hash)
SELECT
  'TerranSoul TTS-SUPERTONIC-1c lesson: Supertonic on-device TTS (~268 MB, OpenRAIL-M v1 weights) ships with explicit consent UX. SupertonicConsentDialog.vue surfaces 6 plain-English OpenRAIL-M restrictions (no discrimination/harassment, no mass surveillance, no mis/disinfo, no CSAM, no automated legal/medical/financial advice, restrictions propagate) plus a link to the upstream Hugging Face model card and docs/licensing-audit.md. Stages: consent | downloading | error | done. Errors mapped to remediation hints: network → check internet; size mismatch → integrity/redownload; io → disk/permissions. Default-provider promotion lives in voice store autoPromoteSupertonicIfReady(): promotes to supertonic only when current tts_provider is null OR ''web-speech'' AND supertonicInstalled — never overrides an explicit cloud choice. revertSupertonicPromotion() restores previousProvider. Architecture pattern: to stay under ESLint MAX_LINES_VUE=800 in VoiceSetupView.vue, the consent flow extracted into useSupertonicConsent composable + SupertonicSection wrapper that uses defineExpose({ consent }) so the parent can call supertonicSectionRef.value?.consent.openIfNeeded(providerId) from onSetTts. Tauri events used: supertonic-download-progress, supertonic-download-complete; commands: supertonic_download_model, supertonic_is_installed, supertonic_install_path, supertonic_remove, supertonic_status. Backend has no cancel primitive — frontend treats consent-stage Cancel as true cancel (no bytes flow) and downloading-stage Hide as dismiss-only (download continues harmlessly).',
  'lesson',
  'voice,tts,supertonic,openrail,consent,ux,vue,licensing,2026-05-16',
  9,
  strftime('%s','now'),
  'seed:tts-supertonic-1c-2026-05-16'
WHERE NOT EXISTS (SELECT 1 FROM memories WHERE source_hash = 'seed:tts-supertonic-1c-2026-05-16');

INSERT INTO memories (content, cognitive_kind, tags, importance, created_at, source_hash)
SELECT
  'TerranSoul BRAIN-REPO-RAG-1 research synthesis 2026-05-16 (docs/repo-rag-systems-research-2026-05-16.md). Field audit covered Aider (PageRank RepoMap over tree-sitter tag co-occurrence graph, budget-fit Lines of Interest), Continue (CodebaseIndexer + LanceDB embedded vector store, typed @codebase/@file/@folder context providers in TipTap composer), Cline (no precomputed index; tool-driven read_file/list_files/search_files with .clineIgnore + Plan/Act approval gates), Cody (ContextRetriever / PromptBuilder separation, pluggable retrievers), LlamaIndex (canonical Reader -> NodeParser -> Embed -> VectorStoreIndex -> Retriever -> ResponseSynthesizer pipeline; CodeHierarchyNodeParser AST chunking), gitingest (parse_remote_repo -> clone_repo with sparse-checkout/submodules -> _scan_directory -> pathspec match -> 10 MB cap -> tiktoken o200k_base; ignore precedence user-includes -> repo .gitignore -> user-excludes -> defaults), repomix (searchFiles -> collectFiles -> validateFileSafety (Secretlint) -> processFiles (tree-sitter signature compression) -> produceOutput -> calculateMetrics; tinypool workers). All licences Apache-2.0 / MIT — patterns adoptable. Decisions for BRAIN-REPO-RAG-1 (storage model: separate SQLite per repo): (1) mcp-data/repos/<source_id>/{checkout,memories.db,ann.usearch,manifest.json} layout; memory_sources registry in memory.db only with kinds self/repo/topic. (2) Ingest via gix shallow clone + ignore-crate walk + secrets-patterns-db scan + tree-sitter AST chunker for code (.rs/.ts/.tsx/.py/.go/.java/.c/.cpp) + text-splitter for prose + 10 MB cap. (3) Retrieval modes Self / Source(id) / All; All fans out + RRF-fuses (k=60) across DBs. (4) MCP surface mirrors GitNexus shape with source_id arg: repo_list_sources, repo_add_source, repo_remove_source, repo_sync, repo_map (Aider PageRank), repo_search, repo_read_file (Cline-style), repo_signatures (repomix-style). (5) Aider RepoMap reimplemented natively on top of coding/symbol_index.rs (no Python port). (6) Continue @codebase pattern surfaces as @source-id mentions in chat composer for one-turn pulls without changing active source. (7) Secret scanning is mandatory before any embed step (repomix Secretlint precedent) — files with detected secrets skipped, count surfaced in MemoryView via 🛡 badge. (8) Memory panel UX: segmented header 🧠 TerranSoul / 📦 owner/repo / 🌐 All sources / ➕ Add source; filters stats/list/search/Add-memory by active source. (9) Quests: new repo-scholar-quest gated on memory_sources.count(kind=repo) >= 1, combos with paid-brain + rag-knowledge. (10) Anti-patterns NOT to copy: Aider auto-commit, Continue mandatory Hub control plane, Cline YOLO --auto-approve-all, GitNexus PolyForm-NC dependency, Cody Sourcegraph backend dependency. Chunk split: 1a UI-first (memory_sources table + picker, no ingest), 1b ingest backend, 1c source-scoped retrieval + chat wiring, 1d Aider repo-map + signatures + quest + Brain Docs Sync. Credits in CREDITS.md row "Top-tier coding-RAG / repo-context comparison sources studied for BRAIN-REPO-RAG-1 (2026-05-16)".',
  'lesson',
  'brain-repo-rag,coding-rag,aider,continue,cline,cody,llamaindex,gitingest,repomix,gitnexus,research,2026-05-16',
  9,
  strftime('%s','now'),
  'seed:repo-rag-research-2026-05-16'
WHERE NOT EXISTS (SELECT 1 FROM memories WHERE source_hash = 'seed:repo-rag-research-2026-05-16');

INSERT INTO memories (content, cognitive_kind, tags, importance, created_at, source_hash)
SELECT
  'TerranSoul BRAIN-REPO-RAG-1a frontend slice landed 2026-05-16. Pinia store src/stores/memory-sources.ts owns the active memory-source id: state = {sources, activeId, isLoading, error}; computed = {activeSource, isAllView, repoSources}; actions = fetchAll() / setActive(id) / createSource({id,kind,label,repo_url?,repo_ref?}) / deleteSource(id). Mirrors backend invariants from memory::sources: cannot delete the seeded ''self'' row, cannot create a second kind=''self'', list is sorted ''self''-first then alpha by lower(label). Active id is persisted to localStorage under key terransoul.memory-sources.active.v1 — that is the canonical handoff for any future surface (chat composer @source-id mentions in 1c, MCP repo_search in 1c) that needs source awareness. A sentinel id ''__all__'' is reserved for the cross-source aggregate that lands in BRAIN-REPO-RAG-1c (isAllView==true, activeSource==null). MemoryView.vue gained a <nav class="mv-source-picker"> strip rendering 🧠 TerranSoul / 📦 <repo> / 🌐 All sources / ＋ Add source pills + a modal Add-source dialog (label + optional URL + git ref + slugifySourceId helper that derives repo:<host>/<path>). When a non-self source is active the picker shows "Repo ingest lands in BRAIN-REPO-RAG-1b — this source is registered but not yet indexed." Validation: vitest 5/5 (memory-sources.test.ts), ESLint clean on the touched files, backend memory::sources 10/10 and memory::schema 11/11 unchanged. Pre-existing TS errors in SettingsModal.vue:99 and SupertonicConsentDialog.test.ts:17 are untracked WIP unrelated to this slice. Sub-component extraction in MemoryView.vue is explicitly DEFERRED to BRAIN-REPO-RAG-1b alongside the actual repo-chunk listing surface — MemoryView.vue is already on the ESLint max-lines allowlist so this slice does not regress lint.',
  'lesson',
  'brain-repo-rag,memory-sources,pinia,vue,localStorage,2026-05-16',
  8,
  strftime('%s','now'),
  'seed:brain-repo-rag-1a-frontend-2026-05-16'
WHERE NOT EXISTS (SELECT 1 FROM memories WHERE source_hash = 'seed:brain-repo-rag-1a-frontend-2026-05-16');

INSERT INTO memories (content, cognitive_kind, tags, importance, created_at, source_hash)
SELECT
  'TerranSoul BRAIN-REPO-RAG-1b-i (per-repo ingest backend, foundation slice) shipped 2026-05-16 behind a new repo-rag Cargo feature (desktop default; mobile/headless-mcp builds compile without gix/ignore/regex). New module src-tauri/src/memory/repo_ingest.rs owns the end-to-end pipeline: (1) gix 0.66 shallow clone via prepare_clone(url, dest).with_shallow(Shallow::DepthAtRemote(NonZeroU32::new(1))).fetch_then_checkout(Discard, &gix::interrupt::IS_INTERRUPTED) → PrepareCheckout.main_worktree(...). gix features used: blocking-network-client + blocking-http-transport-reqwest-rust-tls + worktree-mutation + max-performance-safe (the worktree-mutation + blocking-network-client gate is REQUIRED for fetch_then_checkout to be exposed — without both, you get E0599 "method not found"). (2) ignore::WalkBuilder honours .gitignore + .git/info/exclude + .terransoulignore + OverrideBuilder for user includes/excludes (precedence: user-includes → repo ignores → user-excludes → defaults). Always skips .git/. (3) Per-file 10 MiB cap (RepoIngestOptions::max_file_bytes). (4) Secret regex set: PEM private-key headers, AKIA[A-Z0-9]{16}, gh[pousr]_[A-Za-z0-9]{36,}, xox[abprs]-..., AIza[A-Za-z0-9_-]{35}, and (api_key|secret_key|access_token|password)\s*[:=]\s*[A-Za-z0-9_\-+/]{20,}. Compiled once via OnceLock. Scans only first 256 KiB so large fixtures don''t dominate ingest. (5) NUL-byte binary heuristic + UTF-8 decode skip. (6) Text chunker reuses memory::chunking::split_markdown + text-splitter 0.30 — recovers byte_start/byte_end by linearly scanning content with a monotonic cursor (text-splitter chunks are ordered + non-overlapping, so cursor-find is robust). (7) Per-repo SQLite at <data_dir>/repos/<source_id>/memories.db with a single repo_chunks table (id, source_id, file_path, parent_symbol, kind CHECK IN(''text'',''code''), byte_start, byte_end, content, content_hash, embedding BLOB NULL, created_at) + WAL + indices on source_id / (source_id,file_path) / content_hash. (8) manifest.json at <data_dir>/repos/<source_id>/manifest.json with RepoIngestStats + head_commit (12-hex) + last_synced_at + manifest_version=1. (9) Three Tauri commands in src-tauri/src/commands/repos.rs: repo_add_source (registers memory_sources row kind=''repo'' then ingests), repo_sync (re-ingests existing source), repo_remove_source (idempotent: deletes the per-repo dir AND the memory_sources row). Blocking work runs on tokio::task::spawn_blocking. Commands are gated in tauri::generate_handler! via #[cfg(feature = "repo-rag")] on each fully-qualified path — Tauri 2.11''s generate_handler! tolerates per-item #[cfg] attributes on identifier lines. (10) 12 Rust unit tests green (validate_source_id positive+negative, secret-scanner positive+negative, NUL-byte detector, ChunkKind classifier, chunk-span monotonicity, RepoStore round-trip, walk_files .gitignore + .git/ skip + cap, manifest JSON round-trip, idempotent remove_repo). Deferred to 1b-ii: AST tree-sitter chunker (reuse coding/symbol_index.rs), mxbai-embed-large embedding pass + per-repo HNSW, streaming task-progress events with phase enum, incremental sync via content_hash, integration test against a file:// fixture repo.',
  'lesson',
  'brain-repo-rag,repo-ingest,gix,ignore,secret-scanner,tauri-feature-flags,2026-05-16',
  9,
  strftime('%s','now'),
  'seed:brain-repo-rag-1b-i-2026-05-16'
WHERE NOT EXISTS (SELECT 1 FROM memories WHERE source_hash = 'seed:brain-repo-rag-1b-i-2026-05-16');

INSERT INTO memories (content, cognitive_kind, tags, importance, created_at, source_hash)
SELECT
  'TerranSoul BRAIN-REPO-RAG-1b-ii-a (per-repo ingest: AST + incremental sync + progress events) shipped 2026-05-16. Adds three orthogonal capabilities to the 1b-i pipeline without touching the embed/HNSW path (deferred to 1b-ii-b). (1) AST parent_symbol annotation: new ast_annotate_chunks(file_rel, content, &mut chunks) in src-tauri/src/memory/repo_ingest.rs reuses coding::parser_registry::create_parser + the pub(crate) extractors coding::symbol_index::extract_rust_symbols / extract_ts_symbols (both take (source, root_node, file) -> (Vec<Symbol>, Vec<CodeEdge>)). Symbol carries 1-based line + end_line, not byte offsets — so the annotator builds a single byte→line table (Vec<usize> of line-start byte offsets) per file via binary_search, sorts symbols by smallest span (innermost-wins), and labels each chunk Some("<kind>::<parent?>::<name>"). Wired today for .rs and .ts/.tsx via detect_language; optional parsers (parser-python/go/java/c) plug in automatically. Falls back silently when parsing fails so the prose chunker''s heading-derived parent_symbol is preserved. (2) Incremental sync: new repo_files(source_id, file_path, file_hash, last_synced_at, chunk_count) table with composite PK. ingest_repo_with computes SHA-256 hex per file (reuses sha2 + hex — chose over blake3 because blake3 was not yet a direct dep and sha2 is already used by chunking::split_markdown). Files whose hash matches existing_file_hashes() are skipped with stats.files_skipped_unchanged++. INVARIANT: per-file hash upserts (RepoStore::upsert_file_hash) happen AFTER the chunk buffer is flushed — deferring them via Vec<(rel, hash, count)> pending_hashes avoids stranding a hash ahead of its rows if the pipeline errors mid-flow. RepoStore::prune_missing(seen: &HashSet<String>) GCs both repo_files and repo_chunks rows for paths no longer in the walk; stats.files_pruned reports the count. RepoIngestStats grows files_skipped_unchanged + files_pruned, both #[serde(default)] for forward-compat with 1b-i manifest.json snapshots. (3) Progress sink: IngestPhase enum (Clone | Walk | ScanSecrets | Chunk | Embed | Persist | Done — Embed reserved for 1b-ii-b), IngestProgress payload, IngestSink trait (Sync), SilentSink no-op (preserves the old ingest_repo(data_dir, options) signature as a thin wrapper), CapturingSink for tests with Mutex<Vec<IngestProgress>>. ingest_repo_with(data_dir, options, &dyn IngestSink) is the real impl driving the phases. The Supertonic-1c sink pattern (TaskProgressEmitter enum in commands/ingest.rs) is the precedent. Test-only seam ingest_from_checkout_for_tests(data_dir, options, checkout, sink) runs the pipeline against an already-checked-out tempdir to exercise walk+scan+chunk+AST+persist+incremental+prune without a git binary; full file:// integration test is deferred to 1b-ii-b alongside the embed pass. Validation: 8 new tests (20/20 total in memory::repo_ingest) — ast_annotate_sets_parent_symbol_for_rust, ast_annotate_noop_for_unknown_language, file_hash_hex_is_stable, repo_files_table_round_trip, prune_missing_drops_files_not_in_seen, ingest_phase_as_str_round_trip, ingest_from_checkout_emits_phases_and_chunks, ingest_from_checkout_is_incremental (three-pass: initial → unchanged-skip → modify-and-prune). cargo clippy --features repo-rag --lib -- -D warnings clean after a drive-by fix in voice/supertonic_tts.rs:690 (std::iter::repeat(x).take(n) → std::iter::repeat_n(x,n) — Rust 1.95 manual_repeat_n lint upgrade unrelated to 1b-ii but blocked the gate under the same desktop default feature set).',
  'lesson',
  'brain-repo-rag,repo-ingest,ast,tree-sitter,incremental-sync,progress-events,2026-05-16',
  9,
  strftime('%s','now'),
  'seed:brain-repo-rag-1b-ii-a-2026-05-16'
WHERE NOT EXISTS (SELECT 1 FROM memories WHERE source_hash = 'seed:brain-repo-rag-1b-ii-a-2026-05-16');

INSERT INTO memories (content, cognitive_kind, tags, importance, created_at, source_hash)
SELECT
  'TerranSoul BRAIN-REPO-RAG-1b-ii-b (per-repo ingest: embed pass + per-repo HNSW + Tauri progress sink) shipped 2026-05-16. Closes out 1b-ii by wiring the embedding stage through the IngestSink/IngestPhase scaffolding from 1b-ii-a. (1) embed_repo_with_fn(data_dir, source_id, dimensions, embed_fn: FnMut(&str)->Option<Vec<f32>>, sink) is a sync function in memory::repo_ingest. Loops repo_chunks rows where embedding IS NULL (RepoStore::pending_embedding_rows), persists each vector as little-endian f32 bytes via RepoStore::set_embedding (UPDATE ... SET embedding = ?1), and adds it to a per-repo HNSW opened at <data_dir>/repos/<source_id>/vectors.usearch via AnnIndex::open(repo_root, dimensions). Index is saved once at the end (single fsync). Dim-mismatch and None returns are counted as chunks_failed and do NOT abort the run — important because cloud embedding providers may rate-limit mid-batch and we want to keep partial progress. Empty pending set returns early without creating an empty .usearch file. New RepoEmbedStats { chunks_embedded, chunks_failed } is serde camelCase. New RepoIngestError::AnnIndex(String) variant wraps usearch error strings. (2) commands/repos.rs::TauriIngestSink wraps an AppHandle + task_id and maps each IngestProgress into a TaskProgressEvent emitted on "task-progress" (Supertonic-1c precedent in commands/ingest.rs). Percent is computed via processed.checked_mul(100).and_then(checked_div(total)).unwrap_or(0) — clippy manual_checked_ops fix. sync_inner now runs TWO spawn_blocking stages: first the ingest, then the embed pass. brain_mode + active_brain are snapshotted from AppState BEFORE the first blocking hop (avoids holding the std::sync::Mutex across await). The embed closure inside the second spawn_blocking uses tokio::runtime::Handle::current().block_on(embed_for_mode(text, brain_mode_ref, active_brain_ref)) — block_on inside spawn_blocking is correct because the blocking pool thread is separate from the reactor. When brain_mode is None the closure short-circuits via "brain_mode_ref?;" (clippy question_mark fix) so ingest still finishes with chunks_failed=0/chunks_embedded=0. (3) RepoSyncResponse grew an embed_stats: RepoEmbedStats field (camelCase as embedStats on the wire) so the Memory panel UI can render "embedded N / M chunks" alongside ingest counts. Both repo_add_source and repo_sync now take an additional AppHandle parameter — Tauri 2.11 auto-injects this when declared in the command signature, no generate_handler! change needed. Validation: 23/23 tests in memory::repo_ingest green (3 new: embed_repo_persists_blobs_and_writes_hnsw_index, embed_repo_skips_when_no_pending_rows, embed_repo_records_dim_mismatch_as_failure). cargo clippy --features repo-rag --lib -- -D warnings clean. Pitfall observed during implementation: multi_replace_string_in_file applied only partial matches when oldString contained complex multi-line context that already existed in two near-identical forms (repo_add_source signature + sync_inner signature) — left the file in a half-edited state requiring follow-up cleanup. Lesson: when refactoring two similar function signatures in one file, edit each with a UNIQUE 5-line anchor or read-then-rewrite-block; do not rely on multi_replace overlap heuristics.',
  'lesson',
  'brain-repo-rag,repo-ingest,embeddings,hnsw,usearch,tauri-progress,2026-05-16',
  9,
  strftime('%s','now'),
  'seed:brain-repo-rag-1b-ii-b-2026-05-16'
WHERE NOT EXISTS (SELECT 1 FROM memories WHERE source_hash = 'seed:brain-repo-rag-1b-ii-b-2026-05-16');

INSERT INTO memories (content, cognitive_kind, tags, importance, created_at, source_hash)
SELECT
  'TerranSoul BRAIN-REPO-RAG-1c-a (source-scoped retrieval backend) shipped 2026-05-16. First half of 1c: hybrid search inside a single per-repo SQLite + HNSW source. Cross-source All fan-out, chat prompt citations, @source-id mentions, and BrainGateway/MCP tool registration are queued as 1c-b. (1) RepoStore::hybrid_search(query, _query_embedding, ann_matches: &[(i64,f32)], limit) fuses three independent rankings via Reciprocal Rank Fusion with k=60. Vector ranking comes from caller-supplied ann_matches (typically pre-fetched from AnnIndex::open(<repo_root>, dim).search(emb, limit*5)) — keeping AnnIndex out of the method signature keeps it dep-free and deterministically testable. Keyword ranking uses case-insensitive LOWER(content) LIKE %term% per whitespace-split token >=2 chars, capped at 500 candidates, ranked by hit count then id (no FTS5 on repo_chunks — LIKE keeps the surface lean). Recency ranking is ORDER BY created_at DESC, id DESC LIMIT 100. Repo chunks have no tier/importance/decay_score columns so the main-brain 6-signal formula in memory/store.rs is NOT reused. Results hydrate to RepoSearchHit { id, source_id, file_path, parent_symbol, kind, byte_start, byte_end, content, score: f64 } sorted by fused score desc, truncated to limit. (2) RepoStore helpers: list_files() returns SELECT DISTINCT file_path ORDER BY file_path; read_file(file_path) reassembles chunks ordered by byte_start, id as a fallback when the on-disk checkout has been removed; get_chunk(id) fetches one row. (3) Three Tauri commands in commands/repos.rs all gated #[cfg(feature = "repo-rag")] and registered in lib.rs generate_handler!: repo_search(source_id, query, limit?) snapshots brain_mode + active_brain BEFORE the blocking hop, awaits embed_for_mode (best-effort — None when brain not configured), spawn_blocking #1 opens the per-repo AnnIndex and runs search(emb, limit*5).max(20) (vector signal silently skipped on dim mismatch or empty index), spawn_blocking #2 opens RepoStore and runs hybrid_search; repo_list_files; repo_read_file prefers std::fs::read_to_string from checkout_dir/<file_path> after canonicalize() + starts_with(checkout_root) guard against symlink / .. escapes, rejects empty / absolute / ..-containing paths up-front, falls back to chunk reassembly when checkout is gone. Validation: 28/28 tests in memory::repo_ingest green (5 new: repo_hybrid_search_finds_keyword_hits, repo_hybrid_search_fuses_vector_keyword_and_recency, repo_hybrid_search_respects_top_k_and_empty_query, repo_list_files_returns_distinct_paths_sorted, repo_read_file_reassembles_chunks). cargo clippy --features repo-rag --lib -- -D warnings clean (one drive-by fix: needless_return on the early-exit in repo_read_file). Lesson: when adding a SQL keyword path beside a vector path, accept ann_matches as a parameter rather than embedding the AnnIndex inside the store method — keeps the method free of tokio/usearch deps and makes RRF fusion testable with synthetic (id, score) pairs from real chunk ids.',
  'lesson',
  'brain-repo-rag,repo-search,hybrid-search,rrf,source-scoped,2026-05-16',
  9,
  strftime('%s','now'),
  'seed:brain-repo-rag-1c-a-2026-05-16'
WHERE NOT EXISTS (SELECT 1 FROM memories WHERE source_hash = 'seed:brain-repo-rag-1c-a-2026-05-16');

INSERT INTO memories (content, cognitive_kind, tags, importance, created_at, source_hash)
SELECT
  'TerranSoul BRAIN-REPO-RAG-1c-b-i (BrainGateway MCP surface for per-repo retrieval) shipped 2026-05-16. First half of 1c-b: promotes the three repo Tauri commands from 1c-a (repo_search, repo_list_files, repo_read_file) to first-class BrainGateway methods + identically-named MCP tools so external AI coding agents can reach indexed repositories through MCP, not just the Tauri frontend. Cross-source All fan-out, prompt assembler with grouped citations, @source-id chat-composer mentions, and frontend vitest are deferred to 1c-b-ii. (1) Wire-stable request types in ai_integrations/gateway.rs — RepoSearchRequest { source_id, query, limit: Option<usize> }, RepoListFilesRequest { source_id }, RepoReadFileRequest { source_id, file_path } — plus a wire-stable gateway::RepoSearchHit that mirrors memory::repo_ingest::RepoSearchHit so the trait stays compilable when the repo-rag feature is off (the gateway just maps repo_ingest hits into wire hits). (2) Three new BrainGateway trait methods: repo_search(caps, RepoSearchRequest) -> Vec<RepoSearchHit>, repo_list_files(caps, RepoListFilesRequest) -> Vec<String>, repo_read_file(caps, RepoReadFileRequest) -> String. Trait surface itself is feature-flag-free; impl bodies are #[cfg(feature = "repo-rag")] with #[cfg(not)] stubs that return GatewayError::NotConfigured("repo-rag feature is not enabled in this build"). (3) AppStateGateway implementations mirror commands::repos verbatim: require_read(caps)? → validate_source_id → snapshot brain_mode/active_brain → embed_for_mode (best-effort) → spawn_blocking #1 opens per-repo AnnIndex.search(emb, (limit*5).max(20)) (silently skipped on no embedding / dim mismatch) → spawn_blocking #2 opens RepoStore + hybrid_search. repo_read_file prefers on-disk checkout via std::fs::read_to_string after canonicalize() + starts_with(checkout_root) symlink/.. guard, falls back to RepoStore::read_file chunk reassembly when canonicalize fails. Path-traversal hardening rejects "..", absolute prefixes ("/", "\\"), and OS-absolute PathBuf::is_absolute() up-front as InvalidArgument. (4) Three new MCP tools registered unconditionally in ai_integrations/mcp/tools.rs::definitions alongside the existing brain_* tools (capability gating happens server-side via require_read inside each gateway method, matching the pattern of the other brain tools). Dispatch arms parse args["source_id"]/args["query"]/args["file_path"]/args["limit"] and call the gateway methods. Bumped tools_list count tests 18→21 (brain-only) and 35→38 (with code_read) + added repo_* names to brain_tool_names_match_dispatch_arms expected list + shifted code_tool_names_are_correct slice start from defs[18..] to defs[21..] + updated tools_list_returns_28_tools (now 38) integration test with new index assertions tools[18]=repo_search, tools[19]=repo_list_files, tools[20]=repo_read_file. Validation: 5 new gateway::tests::repo_* tests (permission denial, empty query, invalid source_id, path-traversal x4, list-files invalid source_id). cargo test --features repo-rag --lib ai_integrations: 171 passed, 1 failed (kg_neighbors_reads_shared_seed_lesson_hub_edges — verified pre-existing via git stash before my edits, "table memories has no column named kind" schema drift unrelated to 1c-b-i). cargo clippy --features repo-rag --lib -- -D warnings clean. Lesson: when promoting an internal Tauri command to a BrainGateway trait method, define the wire types (request + response) inside the gateway module so the trait remains feature-flag-free, then conditionally compile the impl bodies — this keeps non-default builds (CI without --features repo-rag) compilable, and lets the dispatch layer (MCP) and frontend (Tauri command) share one canonical schema.',
  'lesson',
  'brain-repo-rag,brain-gateway,mcp-tools,repo-search,2026-05-16',
  9,
  strftime('%s','now'),
  'seed:brain-repo-rag-1c-b-i-2026-05-16'
WHERE NOT EXISTS (SELECT 1 FROM memories WHERE source_hash = 'seed:brain-repo-rag-1c-b-i-2026-05-16');

INSERT INTO memories (content, cognitive_kind, tags, importance, created_at, source_hash)
SELECT
  'TerranSoul BRAIN-REPO-RAG-1c-b-ii-a (cross-source All fan-out backend) shipped 2026-05-16. First slice of 1c-b-ii: lands the gateway/Tauri/MCP backend for cross-source retrieval; frontend prompt-assembler grouping, @source-id chat mentions, citation rendering, and vitest are deferred to 1c-b-ii-b. (1) New wire types in ai_integrations/gateway.rs: CrossSourceSearchRequest { query: String, limit: Option<usize> } and MultiSourceHit { source_id, source_label, local_id, content, score, file_path: Option<String>, parent_symbol: Option<String>, tier: Option<String>, tags } — source_id is "self" for main-brain hits and a memory_sources.id for repo hits; tier is Some for main brain (MemoryTier::as_str()), None for repo; file_path/parent_symbol are Some only for repo hits. (2) New BrainGateway::cross_source_search(caps, req) -> Vec<MultiSourceHit> trait method. Trait surface itself feature-flag-free; AppStateGateway impl: require_read(caps)? → reject empty query → clamp limit 1..=100 default 10 → per-source recall = (limit*5).clamp(20,100) → snapshot brain_mode + active_brain → embed_for_mode (best-effort) → run MemoryStore::hybrid_search_rrf for the self bucket → enumerate repo sources via crate::memory::sources::list_sources(store.conn()).filter(|s| s.kind == MemorySourceKind::Repo) under #[cfg(feature = "repo-rag")] (empty Vec under #[cfg(not)]) → for each repo spawn_blocking #1 AnnIndex::open + search(emb, recall.max(20)) (silently skipped on missing index / dim mismatch / no embedding) → spawn_blocking #2 RepoStore::open + hybrid_search → map results to MultiSourceHit with the repo''s source_label. (3) RRF fusion: each (source_id, local_id) pair gets a stable usize index into a flat arena (Vec<MultiSourceHit>); per-source rankings become Vec<usize>; call crate::memory::fusion::reciprocal_rank_fuse(&[&Vec<usize>], DEFAULT_RRF_K=60). Why usize and not (String, i64) tuples: reciprocal_rank_fuse<T: Copy + Eq + Hash + Ord> requires Copy keys, so tuple-of-String won''t compile — the arena indirection is the idiomatic fix. Top-k slice taken from the fused ranking, RRF score stamped onto each hit, returned already sorted desc. (4) New Tauri command commands::cross_source::cross_source_search(query, limit, state) — registered in lib.rs::generate_handler! between repo_read_file and memory_graph_page (NOT feature-gated since the gateway impl handles cfg internally). New MCP tool cross_source_search registered unconditionally in tools.rs immediately after repo_read_file with required {query} param + optional {limit} (1-100). Tool-count tests bumped: definitions_has_8_brain_tools_without_code_read 21→22; definitions_has_21_tools_with_code_read 38→39; brain_tool_names_match_dispatch_arms includes "cross_source_search"; code_tool_names_are_correct slices defs[22..] (was [21..]); integration_tests::tools_list_returns_28_tools asserts tools.len()=39 with tools[21]=cross_source_search and code tools shifted by one. Validation: 4 new cross_source_search_* gateway tests (permission denied without brain_read, empty query InvalidArgument, self-only fan-out returning source_id="self" / file_path=None / tier=Some(_) sorted desc by RRF score, limit=0 clamps to 1 instead of panicking). cargo test --features repo-rag --lib ai_integrations: 105/105 green. cargo clippy --features repo-rag --lib --tests -- -D warnings clean. Lesson: when implementing RRF across heterogeneous sources whose natural keys are (String, i64), assign each unique key a stable usize index into a flat arena and rank-fuse the indices — reciprocal_rank_fuse<T: Copy> rejects compound non-Copy keys, and the arena indirection is cheaper than per-source key cloning during merge.',
  'lesson',
  'brain-repo-rag,cross-source-search,rrf,brain-gateway,mcp-tools,2026-05-16',
  9,
  strftime('%s','now'),
  'seed:brain-repo-rag-1c-b-ii-a-2026-05-16'
WHERE NOT EXISTS (SELECT 1 FROM memories WHERE source_hash = 'seed:brain-repo-rag-1c-b-ii-a-2026-05-16');

INSERT INTO memories (content, cognitive_kind, tags, importance, created_at, source_hash)
SELECT
  'TerranSoul BRAIN-REPO-RAG-1c-b-ii-b (frontend cross-source chat wiring) shipped 2026-05-16. Second slice of 1c-b-ii: wires the cross_source_search Tauri command + MCP tool (shipped earlier the same day in 1c-b-ii-a) into the desktop chat surface. (1) New TS wire type MultiSourceHit in src/types/index.ts mirroring the Rust gateway::MultiSourceHit (source_id, source_label, local_id, content, score, file_path?, parent_symbol?, tier?, tags); Message gains optional sources: MultiSourceHit[] so each assistant turn can carry its retrieval payload. (2) useMemoryStore().crossSourceSearch(query, limit=5) wrapper calls invoke<MultiSourceHit[]>("cross_source_search", { query, limit }) and falls back to wrapping hybridSearch results as self-source hits when Tauri is unavailable, preserving graceful behaviour in pure-browser builds. (3) Three exported helpers in src/stores/conversation.ts: parseSourceMentions(text) uses the regex /(?:^|\s)@([A-Za-z0-9_.-][A-Za-z0-9_./-]*)/g so email addresses (user@example.com) are not misinterpreted as @source-id mentions — the (?:^|\s) word-boundary is mandatory; strips trailing sentence punctuation [.,!?:;]+$ from the captured id; dedups while preserving first-appearance order. groupHitsBySource(hits) groups by source_id preserving first-appearance order. formatCrossSourceContextPack(hits) — multi-source emits "── 🧠 TerranSoul ──" / "── 📦 owner/repo ──" group headers + repo hits render with [file_path::parent_symbol] preface; single-source collapses to the legacy formatRetrievedContextPack shape so existing prompt-shape tests keep passing without modification. (4) sendMessage browser path (path 2) now calls crossSourceSearch when the user mentions @source-id tokens OR the active source equals ALL_SOURCES_ID, filters hits to the explicitly-mentioned ids when present (one-turn override — never mutates the active source registry, matching the Continue @codebase precedent), and attaches the surviving hits to assistantMsg.sources. (5) src/components/ChatMessageList.vue grows a <details class="sources-footer"> block rendered when item.msg.sources?.length > 0: <summary> shows per-source badges (🧠/📦 + label + count), and the expanded ordered list shows each contributing row''s [file_path::parent_symbol] (or tier for brain hits) + a truncate(content,160) preview. All styles use var(--ts-*) design tokens. Validation: npx vitest run = 1962/1962 passed (153 files, +15 new cases from this chunk including 14 in the new cross-source-rag.test.ts file). npx vue-tsc --noEmit has 2 pre-existing errors in SettingsModal.vue and SupertonicConsentDialog.test.ts unrelated to this chunk. Scope deferral: this slice wires path 2 (browser-side streaming) only. Path 1 (Tauri streaming with backend prompt injection in Rust) still uses single-source RAG; folding cross-source into path 1 is rolled into chunk 1d alongside the Aider-style repo map + repo-scholar quest + Brain Documentation Sync pass, because path 1 needs Rust-side prompt-assembler changes that are naturally bundled with the 1d finalisation. Lesson 1: TerranSoul has two LLM streaming paths — path 1 (Tauri/Rust backend, sees backend-injected RAG) and path 2 (browser-side streamChatCompletion, frontend builds the system prompt). Frontend RAG changes ONLY affect path 2; backend RAG changes ONLY affect path 1. Both must be touched for full feature coverage. Lesson 2: when designing @-mention syntax in chat, require a word boundary ((?:^|\s)@) in the regex to skip email addresses — otherwise me@example.com is misparsed as source id "example.com". Lesson 3: when prompt-shape backward compatibility matters, collapse multi-source rendering to the legacy single-source format whenever the filtered result set ends up in one source — preserves every existing assertion without per-test special-casing.',
  'lesson',
  'brain-repo-rag,cross-source-search,frontend,chat,citations,mentions,2026-05-16',
  9,
  strftime('%s','now'),
  'seed:brain-repo-rag-1c-b-ii-b-2026-05-16'
WHERE NOT EXISTS (SELECT 1 FROM memories WHERE source_hash = 'seed:brain-repo-rag-1c-b-ii-b-2026-05-16');

INSERT INTO memories (content, cognitive_kind, tags, importance, created_at, source_hash)
SELECT
  'TerranSoul BRAIN-REPO-RAG-1d shipped 2026-05-16 (Aider-style repo map + signatures + repo-scholar quest + Brain Documentation Sync). New surfaces: RepoStore::build_repo_map(budget_tokens) and RepoStore::build_file_signatures(file_path) in src-tauri/src/memory/repo_ingest.rs, exposed as BrainGateway::repo_map / repo_signatures trait methods + Tauri commands + identically-named MCP tools. MCP tool count is now 24 brain-only + 17 code = 41 total. Repo-map output uses the Aider shape: leading ⋮ per file boundary, │-prefixed lines for up to REPO_MAP_MAX_SYMBOLS_PER_FILE=8 unique parent_symbols (HashSet dedup, earliest occurrence wins), each rendered as signature_preview = first REPO_MAP_SIGNATURE_LINES=3 non-blank trimmed lines of the chunk. Budget bounds clamp 64..=16384 tokens default 1024; char-budget = tokens * 4; budget 0 returns empty string; tiny budgets always keep the top file. Repo-scholar-quest skill in src/stores/skill-tree.ts (tier advanced, requires rag-knowledge) activates when brain.brainMode !== null && useMemorySourcesStore().repoSources.length > 0; combos with paid-brain (Repo Sage) and rag-knowledge (Polyglot Librarian). Scope deviation: importance proxy, NOT PageRank. Aider runs PageRank over a symbol call-graph; TerranSoul has no per-repo repo_edges table yet (main brain ships code_symbols / code_edges in coding/symbol_index.rs, but those operate on code_repos not memory_sources). Chunk count per file is a robust proxy until a future slice extracts the per-repo call graph. Scope deferral: OAuth device flow for private repos split into chunk 1e because it needs orthogonal HTTP scaffolding (verification_uri prompts, token persistence, FS-permission hardening, refresh on expiry) that should not block the compression surfaces. Lesson 1: NEVER use the id column with string literals in memory-seed.sql — the schema is id INTEGER PRIMARY KEY AUTOINCREMENT and SQLite rejects string ids with SQLITE_MISMATCH (extended code 20). Always use no-id INSERT keyed on source_hash = ''seed:<original-id>'' — matches the runtime appender pattern at gateway.rs:1442. Fixed 13 pre-existing INSERTs out-of-scope during 1d. Lesson 2: when ranking files for token-budgeted compression, chunk count per file is a useful PageRank proxy when no call-graph exists yet — pair it with a per-file symbol cap (8) so a single wide module cannot monopolise the budget. Lesson 3: Aider-style ⋮ / │ glyph language has become a de-facto interchange shape for repo-map tools — preserving it makes the output familiar to any agent that has already learned Aider conventions. Lesson 4: when splitting a 5-part chunk, ship the orthogonal-scaffolding part (OAuth in this case) as its own follow-up chunk instead of stalling the compression surfaces behind it.',
  'lesson',
  'brain-repo-rag,repo-map,signatures,aider,quest,2026-05-16',
  9,
  strftime('%s','now'),
  'seed:brain-repo-rag-1d-2026-05-16'
WHERE NOT EXISTS (SELECT 1 FROM memories WHERE source_hash = 'seed:brain-repo-rag-1d-2026-05-16');
INSERT INTO memories (content, cognitive_kind, tags, importance, created_at, source_hash)
SELECT
  'TerranSoul BRAIN-REPO-RAG-1e shipped 2026-05-16 (private-repo OAuth device flow, final chunk of the BRAIN-REPO-RAG phase). New surfaces: (1) src-tauri/src/memory/repo_oauth.rs — RepoOAuthToken { access_token, token_type, scope, created_at, expires_at }, token_path/load_token/save_token/clear_token persist to <data_dir>/oauth/github.json with FS-permission hardening (Unix PermissionsExt 0o700 dir + 0o600 file; non-Unix permissions.set_readonly(true) best-effort matching VS Code/npm/pip posture; full icacls ACL tightening intentionally deferred to keep the path dependency-free). Atomic .json.tmp + rename writes. inject_https_token(remote_url, token) rewrites https://github.com/owner/repo.git → https://x-access-token:<token>@github.com/owner/repo.git; SSH URLs, non-GitHub URLs, and empty tokens pass through unchanged; URLs that already contain userinfo (https://olduser:oldpass@github.com/...) get their stale credentials replaced not duplicated — detection must accept both bare prefix (starts_with(\"https://github.com/\")) AND userinfo forms (contains(\"@github.com/\")). (2) repo_ingest::ingest_repo_with loads the token via repo_oauth::load_token(data_dir) and passes the rewritten URL to shallow_clone — gix credentials Cascade is deliberately bypassed because x-access-token: is GitHub''s own documented HTTPS auth pattern and is accepted by every git client. (3) Four feature-gated Tauri commands in commands/repos.rs: repo_oauth_github_start(scopes) → DeviceCodeResponse (reuses crate::coding::request_device_code from the self-improve scaffolding); repo_oauth_github_poll(device_code) → DevicePollResult, persists token under spawn_blocking on Success; repo_oauth_github_status() → RepoOAuthStatus { linked, token_type, scope, created_at, expires_at, expired } — never exposes the token itself; repo_oauth_github_clear() — idempotent. RepoOAuthToken::redacted() returns \"<redacted N chars>\" — the only serialisation that touches the token. (4) Frontend: src/components/RepoOAuthDialog.vue mounts in MemoryView via a 🔐 GitHub auth pill in the source-picker nav; shows verification_uri + large monospace user_code with copy button, auto-polls every device_code.interval seconds, displays linked/scope/expired status + unlink button. Uses var(--ts-*) design tokens. src/stores/memory-sources.ts gains 4 wrappers (startGitHubOAuth, pollGitHubOAuth, fetchGitHubOAuthStatus, clearGitHubOAuth) + 3 refs + types. Validation: 5/5 new Rust unit tests pass (roundtrip persistence under oauth/, redaction never leaks, idempotent clear, expiry semantics, URL rewriting); 4/4 new vitest cases pass; cargo clippy --features repo-rag --lib --tests -- -D warnings clean; cargo test --features repo-rag --lib → 2964 passed, 3 failed (3 pre-existing seed/schema-version failures unrelated to 1e: compiled_seed_applies_to_canonical_schema, kg_neighbors_reads_shared_seed_lesson_hub_edges, schema_version_is_21). Lesson 1: URL-rewriting https://x-access-token:<token>@github.com/... is simpler than gix Credentials Cascade and accepted by every git client; avoid the trait-object plumbing layer when a documented URL form does the same job. Lesson 2: FS hardening on Windows via set_readonly(true) is best-effort and matches the posture of VS Code/npm/pip; full ACL tightening would require an icacls shellout — defer the dependency unless a threat model demands it. Lesson 3: when matching GitHub HTTPS URLs for credential injection, the detection regex/predicate must accept BOTH the bare prefix (https://github.com/) AND userinfo-containing forms (https://user:pass@github.com/) — otherwise URLs that have been rewritten once (e.g. a stale token from a prior session) will not be re-rewritten and the user appears to be using stale credentials forever. Lesson 4: any token-bearing struct should have a redacted() / Debug impl that emits length but never the raw value — and a unit test that asserts the raw token does not appear in the redacted output. Lesson 5: when a chunk splits a 5-part feature (1d compression surfaces + 1e OAuth scaffolding), shipping the orthogonal-scaffolding part as its own follow-up chunk keeps the original chunk''s test surface clean and lets reviewers reason about each concern independently — pattern worth repeating when a chunk grows beyond ~6 files.',
  'lesson',
  'brain-repo-rag,oauth,device-flow,private-repos,github,fs-hardening,2026-05-16',
  9,
  strftime('%s','now'),
  'seed:brain-repo-rag-1e-2026-05-16'
WHERE NOT EXISTS (SELECT 1 FROM memories WHERE source_hash = 'seed:brain-repo-rag-1e-2026-05-16');
INSERT INTO memories (content, cognitive_kind, tags, importance, created_at, source_hash)
SELECT
  'TerranSoul BRAIN-REPO-RAG-2a shipped 2026-05-16 (per-source knowledge-graph visualization). Before 2a the 2D MemoryGraph + 3D MemoryGalaxy rendered only personal memories; every repo source registered through 1a-1e was retrievable from chat but invisible in every graph view. Closing pipeline: (1) RepoStore::recent_chunks(limit) in src-tauri/src/memory/repo_ingest.rs queries per-repo SQLite ORDER BY (parent_symbol IS NULL) ASC, created_at DESC, id DESC so AST-annotated function/class boundaries surface first. (2) New Tauri command cross_source_graph_nodes(per_source_limit?) in commands/repos.rs lists every non-self row from memory_sources, opens each RepoStore, takes up to per_source_limit (default 64, clamped [1,512]) recent chunks, and emits CrossSourceGraph { nodes, perSourceCounts } with each node tagged { graphId, sourceId, sourceLabel, localId, content, filePath, parentSymbol, createdAt }. (3) Collision-free numeric id space: positive memories.id stays personal; the projection hands out negative graphIds -1, -2, -3, ... per repo chunk — d3-force numeric node identity stays unique across sources without a string-id migration, and the id space stays well inside JS 2^53. (4) Frontend wiring: useMemoryStore.fetchCrossSourceGraph wraps the command and returns {nodes:[], perSourceCounts:[]} on invoke failure so non-RAG / feature-off builds stay functional; MemoryView.vue watches sourcesStore.isAllView and concatenates the projected MemoryEntry rows onto displayedMemories before binding to <MemoryGraph :memories>. (5) MemoryGraph.vue extends its theme with repo: tok(--ts-warning, #d4a14a) and recolours nodes whose sourceId is set; SelectedNodesPanel shows the 📦 source · file::symbol provenance row. MemoryGalaxy.vue mirrors with REPO_NODE_COLOR. Lesson 1: project unknown sources into a unified node space rather than growing the consumer surface — the graph stayed source-unaware through 1a-1e because the projection step was missing; once cross_source_graph_nodes existed, MemoryGraph + MemoryGalaxy only needed optional-field plumbing on MemoryEntry (no API breakage). Lesson 2: synthesised graph nodes should claim a disjoint numeric id range (negatives) so d3-force collision-free identity comes for free without forcing a string-id migration through every renderer. Lesson 3: when projecting many-source rows under a limit, sort structurally meaningful rows first ((parent_symbol IS NULL) ASC then created_at DESC, id DESC) so AST-annotated function/class chunks dominate the visible set under tight per-source budgets. Lesson 4: chat ([LONG-TERM MEMORY] block via cross_source_search + prompt assembler) and graph render are independent surfaces that both need wiring whenever a new source kind ships — visual parity is not a free side-effect of retrieval parity.',
  'lesson',
  'brain-repo-rag,knowledge-graph,visualization,per-source,cross-source,d3-force,2026-05-16',
  9,
  strftime('%s','now'),
  'seed:brain-repo-rag-2a-2026-05-16'
WHERE NOT EXISTS (SELECT 1 FROM memories WHERE source_hash = 'seed:brain-repo-rag-2a-2026-05-16');

INSERT INTO memories (content, cognitive_kind, tags, importance, created_at, source_hash)
SELECT
  'TerranSoul BRAIN-REPO-RAG-2b shipped 2026-05-16 (deep-scan ingest visibility + debug log). Before 2b, repo-RAG ingest silently dropped 4 classes of files — over-cap (files_skipped_size), binary (files_skipped_binary), unchanged-since-last-sync (files_skipped_unchanged), and likely-secret (files_skipped_secret). Counters were incremented inside ingest_repo_with (and the test-only ingest_from_checkout_for_tests twin), but the IngestSink::progress channel only fired on walk/scan/chunk/persist phase boundaries. Users running a Memory-panel repo scan saw nothing for skipped files — a deep-scan-visibility hole that violated the no-partial-scans rule. Closing pipeline: (1) Rust IngestPhase gains Skip (per-file skip events) and Summary (one final event before Done carrying scanned=N indexed=N skipped_size=N skipped_binary=N skipped_unchanged=N skipped_secret=N pruned=N chunks=N). (2) IngestProgress gains skip_reason: Option<&static str> with the four values too_large | binary | unchanged | secret. (3) Both ingest loops in src-tauri/src/memory/repo_ingest.rs were rewritten so rel is computed BEFORE the size check, then every skip branch fires emit_skip(processed, total, rel, reason) before continue. (4) TaskProgressEvent adds optional log_line: Option<String> with #[serde(default, skip_serializing_if = Option::is_none)] for back-compat. (5) TauriIngestSink::progress in src-tauri/src/commands/repos.rs formats every event into a stable log line: skip[<reason>]: <rel> for skips, summary: <counters> for summary, and <phase> (<p>/<t>): <msg> for normal phases. (6) Frontend useTaskStore keeps a per-task ring buffer taskLogs: Map<string, string[]> capped at TASK_LOG_MAX_LINES=500; the task-progress listener appends every log_line. (7) TaskProgressBar.vue renders a collapsible Debug log disclosure under each running task with sticky counter chips (indexed, skip-size, skip-binary, skip-unchanged, skip-secret) coloured via --ts-accent / --ts-warning / --ts-accent-error design tokens; the open <pre> auto-scrolls. Tests: memory::repo_ingest::tests::ingest_emits_skip_and_summary_events_for_every_decision + 3 vitest cases in src/stores/tasks.test.ts. Lesson 1: counters alone are not progress — every silent skip path must emit a visible event in the same channel as the success path, otherwise users see a stalled pipeline. Lesson 2: when a generic event struct (here TaskProgressEvent) gains an optional field, use #[serde(skip_serializing_if = Option::is_none)] so wire payloads stay back-compat AND add log_line: None to every existing struct-literal site in one pass (in TerranSoul that was 12 sites across brain.rs/ingest.rs/repos.rs). Lesson 3: a per-task ring buffer beats unbounded debug history — TASK_LOG_MAX_LINES=500 + slice(0, len-cap) on overflow gives constant memory under any repo size. Lesson 4: format the wire log line in the sink (server side), not in every UI consumer — the frontend just appends a string, which keeps the consumer trivial and lets every other task kind opt-in to logs by populating log_line.',
  'lesson',
  'brain-repo-rag,ingest,progress,debug-log,deep-scan,no-silent-skips,2026-05-16',
  9,
  strftime('%s','now'),
  'seed:brain-repo-rag-2b-2026-05-16'
WHERE NOT EXISTS (SELECT 1 FROM memories WHERE source_hash = 'seed:brain-repo-rag-2b-2026-05-16');


-- ============================================================================
-- Knowledge-graph hub anchors (seed contract for ai_integrations gateway tests)
--
-- The shared MCP seed exposes a small hub-and-spoke graph so coding agents
-- (and the gateway::kg_neighbors tests) can rely on the following structure
-- being present after `run_all`:
--
--   <any LESSON: row>  --part_of-->  seed:lessons-learned-hub
--   seed:lessons-learned-hub  --covers-->  seed:stack-coverage-anchor
--
-- This gives every lesson a single two-hop walk to the stack-coverage anchor
-- without requiring per-row edge wiring at seed time. All inserts here are
-- idempotent via `WHERE NOT EXISTS`, so re-running the seed is safe.
-- ============================================================================

INSERT INTO memories (content, cognitive_kind, tags, importance, created_at, source_hash)
SELECT
  'Durable gotchas, decisions, and lessons learned from past agent sessions. This memory is the lessons-learned hub anchored in the shared MCP seed. Every LESSON: row in mcp-data/shared/memory-seed.sql should be wired part_of this hub so that knowledge-graph traversal can surface durable institutional memory with a single hop, and so that a two-hop walk reaches the stack-coverage anchor (seed:stack-coverage-anchor). Treat this hub as the canonical entry point when an agent asks for past lessons.',
  'principle',
  'mcp,seed,knowledge-graph,hub,lessons-learned',
  9,
  strftime('%s','now'),
  'seed:lessons-learned-hub'
WHERE NOT EXISTS (SELECT 1 FROM memories WHERE source_hash = 'seed:lessons-learned-hub');

INSERT INTO memories (content, cognitive_kind, tags, importance, created_at, source_hash)
SELECT
  'STACK COVERAGE: the mcp-data seed exercises every TerranSoul stack layer it documents — Rust async/Tokio + rusqlite + thiserror on the backend, Vue 3.5 + Pinia + Vitest on the frontend, Tauri 2 IPC between them, Three.js/VRM for the avatar, Ollama + cloud LLM providers for the brain, FTS5 + HNSW for retrieval, and the CRDT sync core for device link. Every LESSON: row in the seed must remain reachable from this anchor through the lessons-learned hub (seed:lessons-learned-hub) so a coding agent can pull a representative stack-spanning slice with a single two-hop traversal.',
  'principle',
  'mcp,seed,knowledge-graph,stack-coverage,anchor',
  9,
  strftime('%s','now'),
  'seed:stack-coverage-anchor'
WHERE NOT EXISTS (SELECT 1 FROM memories WHERE source_hash = 'seed:stack-coverage-anchor');

INSERT INTO memories (content, cognitive_kind, tags, importance, created_at, source_hash)
SELECT
  'LESSON: The MCP seed (mcp-data/shared/memory-seed.sql) is the single source of truth for durable agent memory shipped with TerranSoul. Rules: (1) every lesson row uses INSERT ... SELECT ... WHERE NOT EXISTS keyed on a unique source_hash so re-running the seed is safe; (2) lesson rows should be wired part_of the seed:lessons-learned-hub memory so KG traversal can surface them; (3) the hub itself is wired to the seed:stack-coverage-anchor memory so a two-hop walk gives an agent both a topic-grouped lesson view and a stack-coverage view; (4) never treat Markdown rules files as MCP memory — sync durable knowledge into this SQL file and let memory_edges define the structure; (5) when adding a new lesson, also append a corresponding INSERT INTO memory_edges row at the bottom of this file wiring the new lesson part_of seed:lessons-learned-hub so the KG stays connected.',
  'lesson',
  'mcp,seed,lessons,rules,idempotency,knowledge-graph',
  9,
  strftime('%s','now'),
  'seed:lesson-mcp-seed-rules'
WHERE NOT EXISTS (SELECT 1 FROM memories WHERE source_hash = 'seed:lesson-mcp-seed-rules');

-- Edge: seed:lesson-mcp-seed-rules --part_of--> seed:lessons-learned-hub
INSERT INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at)
SELECT lesson.id, hub.id, 'part_of', 1.0, 'seed', strftime('%s','now')
FROM memories lesson
JOIN memories hub ON hub.source_hash = 'seed:lessons-learned-hub'
WHERE lesson.source_hash = 'seed:lesson-mcp-seed-rules'
  AND NOT EXISTS (
    SELECT 1 FROM memory_edges e
    WHERE e.src_id = lesson.id AND e.dst_id = hub.id AND e.rel_type = 'part_of'
  );

-- Edge: seed:lessons-learned-hub --covers--> seed:stack-coverage-anchor
INSERT INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at)
SELECT hub.id, anchor.id, 'covers', 1.0, 'seed', strftime('%s','now')
FROM memories hub
JOIN memories anchor ON anchor.source_hash = 'seed:stack-coverage-anchor'
WHERE hub.source_hash = 'seed:lessons-learned-hub'
  AND NOT EXISTS (
    SELECT 1 FROM memory_edges e
    WHERE e.src_id = hub.id AND e.dst_id = anchor.id AND e.rel_type = 'covers'
  );


-- ── WORKSPACE-0 (2026-05-16) ──────────────────────────────────────────
INSERT INTO memories (content, cognitive_kind, tags, importance, created_at, source_hash)
SELECT
  'TerranSoul WORKSPACE-0 (2026-05-16) — modular-monolithic foundation: introducing a Cargo workspace to a repo that already has a dominant member crate (src-tauri) requires four discipline rules to be non-breaking. (1) Pin target-dir to the old location via a new root .cargo/config.toml ([build] target-dir = "src-tauri/target"); without this, cargo relocates artifacts to <root>/target/ which orphans the existing src-tauri/target/ dir, forces a full rebuild, breaks the CodeQL exclusion list, and breaks sibling --target-dir builds like target-mcp/ that key off the same layout. (2) Move every [profile.*] block from the member manifest to the workspace root; cargo silently ignores profile blocks in non-root members and emits unused_manifest_key warnings. In our case that meant moving [profile.dev], [profile.dev.build-override], [profile.dev.package."*"], and [profile.dev.package.scrypt] from src-tauri/Cargo.toml to Cargo.toml. (3) git mv src-tauri/Cargo.lock to root Cargo.lock so the existing resolved versions are reused — cargo only consults the workspace-root lock once a workspace exists, and skipping this triggers a re-resolve. (4) Validate via cargo metadata --no-deps first (no compile cost) then cargo check on the smallest member (here hive-relay, ~36s) before claiming the workspace works — full terransoul build is ~30 min and not needed for parse validation. Phase 0 ships zero code moves; subsequent WORKSPACE-1+ phases will progressively extract leaf modules (identity → resilience → routing → memory → brain → persona → link → ai_integrations → coding) so incremental rebuilds only touch the changed crate plus downstream consumers.',
  'lesson',
  'cargo,workspace,modular-monolithic,build-perf,target-dir,profile-tables,2026-05-16',
  9,
  strftime('%s','now'),
  'seed:lesson-workspace-0-2026-05-16'
WHERE NOT EXISTS (SELECT 1 FROM memories WHERE source_hash = 'seed:lesson-workspace-0-2026-05-16');

-- Edge: seed:lesson-workspace-0-2026-05-16 --part_of--> seed:lessons-learned-hub
INSERT INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at)
SELECT lesson.id, hub.id, 'part_of', 1.0, 'seed', strftime('%s','now')
FROM memories lesson
JOIN memories hub ON hub.source_hash = 'seed:lessons-learned-hub'
WHERE lesson.source_hash = 'seed:lesson-workspace-0-2026-05-16'
  AND NOT EXISTS (
    SELECT 1 FROM memory_edges e
    WHERE e.src_id = lesson.id AND e.dst_id = hub.id AND e.rel_type = 'part_of'
  );


-- ── KNOWLEDGE-SPLIT-1 (2026-05-16) ────────────────────────────────────
-- Register a dedicated `terransoul` repo source so project-coding
-- knowledge can be isolated from generic / shared lessons. The `self`
-- brain stays the durable seat of meta-lessons (build rules, conventions,
-- audit findings). The `terransoul` source is the seat of structural
-- code knowledge (AST chunks, file maps, signatures) populated by the
-- existing BRAIN-REPO-RAG ingest pipeline.
INSERT OR IGNORE INTO memory_sources (id, kind, label, repo_url, repo_ref, created_at, last_synced_at)
VALUES (
  'terransoul',
  'repo',
  'TerranSoul repo',
  'https://github.com/TerranSoul/TerranSoul',
  'main',
  CAST(strftime('%s','now') AS INTEGER) * 1000,
  NULL
);

-- Tag every TerranSoul-specific lesson row with `terransoul-repo` so MCP
-- agents can scope a search to project-coding knowledge ("brain_search
-- ... tag:terransoul-repo") vs generic meta-lessons. Idempotent — the
-- LIKE guard skips rows already carrying the marker.
UPDATE memories
SET tags = CASE
  WHEN tags = '' OR tags IS NULL THEN 'terransoul-repo'
  ELSE tags || ',terransoul-repo'
END
WHERE content LIKE 'TerranSoul %'
  AND ',' || tags || ',' NOT LIKE '%,terransoul-repo,%';

-- Durable lesson: how the knowledge split works.
INSERT INTO memories (content, cognitive_kind, tags, importance, created_at, source_hash)
SELECT
  'TerranSoul KNOWLEDGE-SPLIT-1 (2026-05-16) — project-coding knowledge isolation: TerranSoul-specific lessons in mcp-data/shared/memory-seed.sql are now tagged `terransoul-repo` and a dedicated `memory_sources` row (id=''terransoul'', kind=''repo'') is registered so the MCP source picker and cross-source search can isolate project-coding context from generic agent meta-lessons. Structural code knowledge (AST chunks, signatures, file maps) still flows through the existing BRAIN-REPO-RAG-1b ingest pipeline into mcp-data/repos/terransoul/memory.sqlite — this seed block only registers the source; the user runs repo ingest from the Knowledge Graphs panel to populate it. The `self` brain remains the canonical seat of meta-lessons (build rules, audit findings, completion entries); the `terransoul` repo source is the canonical seat of code knowledge. Filter pattern for MCP agents: brain_search with tag filter `terransoul-repo` returns only project-coding context.',
  'lesson',
  'mcp,seed,knowledge-split,memory-sources,terransoul-repo,2026-05-16',
  9,
  strftime('%s','now'),
  'seed:lesson-knowledge-split-1-2026-05-16'
WHERE NOT EXISTS (
  SELECT 1 FROM memories WHERE source_hash = 'seed:lesson-knowledge-split-1-2026-05-16'
);

-- Edge: seed:lesson-knowledge-split-1-2026-05-16 --part_of--> seed:lessons-learned-hub
INSERT INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at)
SELECT lesson.id, hub.id, 'part_of', 1.0, 'seed', strftime('%s','now')
FROM memories lesson
JOIN memories hub ON hub.source_hash = 'seed:lessons-learned-hub'
WHERE lesson.source_hash = 'seed:lesson-knowledge-split-1-2026-05-16'
  AND NOT EXISTS (
    SELECT 1 FROM memory_edges e
    WHERE e.src_id = lesson.id AND e.dst_id = hub.id AND e.rel_type = 'part_of'
  );


-- ── KNOWLEDGE-SPLIT-1 (2026-05-16) ────────────────────────────────────
-- Register a dedicated `terransoul` repo source so project-coding
-- knowledge can be isolated from generic / shared lessons. The `self`
-- brain stays the durable seat of meta-lessons (build rules, conventions,
-- audit findings). The `terransoul` source is the seat of structural
-- code knowledge (AST chunks, file maps, signatures) populated by the
-- existing BRAIN-REPO-RAG ingest pipeline.
INSERT OR IGNORE INTO memory_sources (id, kind, label, repo_url, repo_ref, created_at, last_synced_at)
VALUES (
  'terransoul',
  'repo',
  'TerranSoul repo',
  'https://github.com/TerranSoul/TerranSoul',
  'main',
  CAST(strftime('%s','now') AS INTEGER) * 1000,
  NULL
);

-- Tag every TerranSoul-specific lesson row with `terransoul-repo` so MCP
-- agents can scope a search to project-coding knowledge ("brain_search
-- ... tag:terransoul-repo") vs generic meta-lessons. Idempotent — the
-- LIKE guard skips rows already carrying the marker.
UPDATE memories
SET tags = CASE
  WHEN tags = '' OR tags IS NULL THEN 'terransoul-repo'
  ELSE tags || ',terransoul-repo'
END
WHERE content LIKE 'TerranSoul %'
  AND ',' || tags || ',' NOT LIKE '%,terransoul-repo,%';

-- Durable lesson: how the knowledge split works.
INSERT INTO memories (content, cognitive_kind, tags, importance, created_at, source_hash)
SELECT
  'TerranSoul KNOWLEDGE-SPLIT-1 (2026-05-16) — project-coding knowledge isolation: TerranSoul-specific lessons in mcp-data/shared/memory-seed.sql are now tagged `terransoul-repo` and a dedicated `memory_sources` row (id=''terransoul'', kind=''repo'') is registered so the MCP source picker and cross-source search can isolate project-coding context from generic agent meta-lessons. Structural code knowledge (AST chunks, signatures, file maps) still flows through the existing BRAIN-REPO-RAG-1b ingest pipeline into mcp-data/repos/terransoul/memory.sqlite — this seed block only registers the source; the user runs repo ingest from the Knowledge Graphs panel to populate it. The `self` brain remains the canonical seat of meta-lessons (build rules, audit findings, completion entries); the `terransoul` repo source is the canonical seat of code knowledge. Filter pattern for MCP agents: brain_search with tag filter `terransoul-repo` returns only project-coding context.',
  'lesson',
  'mcp,seed,knowledge-split,memory-sources,terransoul-repo,2026-05-16',
  9,
  strftime('%s','now'),
  'seed:lesson-knowledge-split-1-2026-05-16'
WHERE NOT EXISTS (
  SELECT 1 FROM memories WHERE source_hash = 'seed:lesson-knowledge-split-1-2026-05-16'
);

-- Edge: seed:lesson-knowledge-split-1-2026-05-16 --part_of--> seed:lessons-learned-hub
INSERT INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at)
SELECT lesson.id, hub.id, 'part_of', 1.0, 'seed', strftime('%s','now')
FROM memories lesson
JOIN memories hub ON hub.source_hash = 'seed:lessons-learned-hub'
WHERE lesson.source_hash = 'seed:lesson-knowledge-split-1-2026-05-16'
  AND NOT EXISTS (
    SELECT 1 FROM memory_edges e
    WHERE e.src_id = lesson.id AND e.dst_id = hub.id AND e.rel_type = 'part_of'
  );


-- ── GRAPHRAG-1 (2026-05-16) ───────────────────────────────────────────
-- Durable cross-system-comparison lesson from the microsoft/graphrag
-- audit. The full design analysis lives in docs/graphrag-comparison.md;
-- this row captures the adoption decisions + deferrals so future agent
-- sessions can find them via brain_search without re-reading the doc.
INSERT INTO memories (content, cognitive_kind, tags, importance, created_at, source_hash)
SELECT
  'TerranSoul GRAPHRAG-1 (2026-05-16) — microsoft/graphrag cross-system comparison: docs/graphrag-comparison.md maps their pipeline (extract_graph -> summarize_descriptions -> cluster_graph Leiden -> create_community_reports -> generate_text_embeddings; four-strategy query system Global/Local/DRIFT/Basic) against TerranSoul''s hybrid 6-signal RRF + HyDE + cross-encoder + memory_edges KG + cognitive-kind retrieval intent stack. Three adoptions queued as sub-chunks in rules/milestones.md: (1a) hierarchical community summaries — recurse Leiden so memory_communities.level carries levels 0..N (cap N=4); LLM-generated per-level summaries through the active brain provider; new Tauri command graph_rag_build_hierarchy + optional level parameter on graph_rag_search. (1b) structured entity/relationship extraction at ingest — new memory::extraction::extract_entities_relationships(text, kind) writes typed (entity, rel_type, entity, description) quads into memories+memory_edges; gated by BrainConfig.graph_extract_enabled default off so offline-only sessions stay zero-cost. (1c) global vs local query routing — extend the Chunk 16.6b query-intent classifier with a scope axis (global, local, mixed); global routes to top-level community summaries (depends on 1a), local routes to entity-walk + hybrid_search_rrf, mixed routes to current dual-level RRF fusion. Deferred: DRIFT iterative refinement (conflicts with single-stream chat UX), FastGraphRAG NLP-only fallback (local Ollama makes LLM extraction near-free). Rejected: Parquet output format (SQLite + per-repo SQLite already the source of truth), settings.yaml config (Tauri commands + Pinia store cover this). Key insight: memory_communities.level was already a schema column added in Chunk 16.6 but only populated at level=0; hierarchical communities is a schema-aligned extension, not a new column. DeepWiki source URL is deepwiki.com/microsoft/graphrag (the workspace-rule deepwiki.org host redirects there); upstream commit studied: 0da2a4dd.',
  'lesson',
  'mcp,seed,graphrag,knowledge-graph,community-detection,leiden,entity-extraction,query-routing,terransoul-repo,2026-05-16',
  9,
  strftime('%s','now'),
  'seed:lesson-graphrag-1-2026-05-16'
WHERE NOT EXISTS (
  SELECT 1 FROM memories WHERE source_hash = 'seed:lesson-graphrag-1-2026-05-16'
);

-- Edge: seed:lesson-graphrag-1-2026-05-16 --part_of--> seed:lessons-learned-hub
INSERT INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at)
SELECT lesson.id, hub.id, 'part_of', 1.0, 'seed', strftime('%s','now')
FROM memories lesson
JOIN memories hub ON hub.source_hash = 'seed:lessons-learned-hub'
WHERE lesson.source_hash = 'seed:lesson-graphrag-1-2026-05-16'
  AND NOT EXISTS (
    SELECT 1 FROM memory_edges e
    WHERE e.src_id = lesson.id AND e.dst_id = hub.id AND e.rel_type = 'part_of'
  );

-- ──────────────────────────────────────────────────────────────────────────
-- Lesson: THEME-COCKPIT-1a — design tokens vs composition (2026-05-16)
-- ──────────────────────────────────────────────────────────────────────────
INSERT INTO memories (
  content, source_hash, cognitive_kind, tier, importance, created_at, updated_at
)
SELECT
  'THEME-COCKPIT-1a lesson (2026-05-16). When a user says a reference UI ' ||
  'looks ''much better'' than ours, FIRST diff the design tokens before ' ||
  'rewriting components. In the Rag Brain reference port, the --accent / ' ||
  '--bg-base / --border-strong / --r-xl values were ALREADY identical to ' ||
  'TerranSoul''s --ts-* tokens (same #00d4ff, #040a12, rgba(0,212,255,0.34)). ' ||
  'The perceived gap was pure composition: layered linear-on-rgba backgrounds, ' ||
  'triple-shadow (inset highlight + cyan soft bloom + deep navy drop), ' ||
  '::before corner reticles, ::after radial halo blob, and bright selected-' ||
  'state border + halo. Solution pattern: capture the composition as a ' ||
  'reusable utility class (.ts-cockpit-card + .ts-cockpit-label + ' ||
  '.ts-cockpit-crumb) plus pre-composed shadow tokens ' ||
  '(--ts-shadow-cockpit{,-hover,-selected}), so later view ports just add ' ||
  'class="ts-cockpit-card" instead of restyling each container. Always ' ||
  'add light-theme overrides for new dark-HUD utilities (corporate/pastel) ' ||
  'or they turn muddy on white. Phase the rollout: 1a = primitives ' ||
  '(this chunk), 1b = brain panel port, 1c = audit/spread.',
  'seed:lesson-theme-cockpit-1a-2026-05-16',
  'procedural',
  'long',
  0.85,
  strftime('%s','now'),
  strftime('%s','now')
WHERE NOT EXISTS (
  SELECT 1 FROM memories WHERE source_hash = 'seed:lesson-theme-cockpit-1a-2026-05-16'
);

INSERT INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at)
SELECT lesson.id, hub.id, 'part_of', 1.0, 'seed', strftime('%s','now')
FROM memories lesson
JOIN memories hub ON hub.source_hash = 'seed:lessons-learned-hub'
WHERE lesson.source_hash = 'seed:lesson-theme-cockpit-1a-2026-05-16'
  AND NOT EXISTS (
    SELECT 1 FROM memory_edges e
    WHERE e.src_id = lesson.id AND e.dst_id = hub.id AND e.rel_type = 'part_of'
  );

-- ──────────────────────────────────────────────────────────────────────────
-- Lesson: THEME-COCKPIT-1a — design tokens vs composition (2026-05-16)
-- ──────────────────────────────────────────────────────────────────────────
INSERT INTO memories (
  content, source_hash, cognitive_kind, tier, importance, created_at, updated_at
)
SELECT
  'THEME-COCKPIT-1a lesson (2026-05-16). When a user says a reference UI ' ||
  'looks ''much better'' than ours, FIRST diff the design tokens before ' ||
  'rewriting components. In the Rag Brain reference port, the --accent / ' ||
  '--bg-base / --border-strong / --r-xl values were ALREADY identical to ' ||
  'TerranSoul''s --ts-* tokens (same #00d4ff, #040a12, rgba(0,212,255,0.34)). ' ||
  'The perceived gap was pure composition: layered linear-on-rgba backgrounds, ' ||
  'triple-shadow (inset highlight + cyan soft bloom + deep navy drop), ' ||
  '::before corner reticles, ::after radial halo blob, and bright selected-' ||
  'state border + halo. Solution pattern: capture the composition as a ' ||
  'reusable utility class (.ts-cockpit-card + .ts-cockpit-label + ' ||
  '.ts-cockpit-crumb) plus pre-composed shadow tokens ' ||
  '(--ts-shadow-cockpit{,-hover,-selected}), so later view ports just add ' ||
  'class="ts-cockpit-card" instead of restyling each container. Always ' ||
  'add light-theme overrides for new dark-HUD utilities (corporate/pastel) ' ||
  'or they turn muddy on white. Phase the rollout: 1a = primitives ' ||
  '(this chunk), 1b = brain panel port, 1c = audit/spread.',
  'seed:lesson-theme-cockpit-1a-2026-05-16',
  'procedural',
  'long',
  0.85,
  strftime('%s','now'),
  strftime('%s','now')
WHERE NOT EXISTS (
  SELECT 1 FROM memories WHERE source_hash = 'seed:lesson-theme-cockpit-1a-2026-05-16'
);

INSERT INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at)
SELECT lesson.id, hub.id, 'part_of', 1.0, 'seed', strftime('%s','now')
FROM memories lesson
JOIN memories hub ON hub.source_hash = 'seed:lessons-learned-hub'
WHERE lesson.source_hash = 'seed:lesson-theme-cockpit-1a-2026-05-16'
  AND NOT EXISTS (
    SELECT 1 FROM memory_edges e
    WHERE e.src_id = lesson.id AND e.dst_id = hub.id AND e.rel_type = 'part_of'
  );
-- ──────────────────────────────────────────────────────────────────────────
-- Lesson: THEME-COCKPIT-1b — alias layer enables mood-driven palette swap (2026-05-16)
-- ──────────────────────────────────────────────────────────────────────────
INSERT INTO memories (
  content, source_hash, cognitive_kind, tier, importance, created_at, updated_at
)
SELECT
  'THEME-COCKPIT-1b lesson (2026-05-16). When porting a designed UI ' ||
  'component, build a thin ALIAS layer between the imported tokens and ' ||
  'your app tokens (here: --bp-* in src/styles/brain-panel.css reading ' ||
  'from --ts-*). That alias layer pays off later when visual variants ' ||
  '(mood/theme/accent attributes) need to retint the whole component: ' ||
  'redefine the --bp-* aliases under .bp-shell[data-accent=violet|green|' ||
  'amber|pink] and every descendant border/glow/active-state cascades ' ||
  'automatically — no component edits, no hunting through rules. ' ||
  'BrainView wires it up via a computed accentKey from moodKey: free->' ||
  'green, paid->violet, local->amber, none->'''' (default cyan), bound ' ||
  ':data-accent on .bp-shell. Without the alias layer the same change ' ||
  'would require editing every border/glow/accent rule in a 1249-line ' ||
  'stylesheet. Also: do a SELECTOR DIFF before assuming a port is ' ||
  'incomplete — the brain panel was 145/147 selectors at parity; the ' ||
  'real gap was three mood variants, not a rewrite.',
  'seed:lesson-theme-cockpit-1b-2026-05-16',
  'procedural',
  'long',
  0.85,
  strftime('%s','now'),
  strftime('%s','now')
WHERE NOT EXISTS (
  SELECT 1 FROM memories WHERE source_hash = 'seed:lesson-theme-cockpit-1b-2026-05-16'
);

INSERT INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at)
SELECT lesson.id, hub.id, 'part_of', 1.0, 'seed', strftime('%s','now')
FROM memories lesson
JOIN memories hub ON hub.source_hash = 'seed:lessons-learned-hub'
WHERE lesson.source_hash = 'seed:lesson-theme-cockpit-1b-2026-05-16'
  AND NOT EXISTS (
    SELECT 1 FROM memory_edges e
    WHERE e.src_id = lesson.id AND e.dst_id = hub.id AND e.rel_type = 'part_of'
  );

INSERT INTO memories (
  content, source_hash, cognitive_kind, tier, importance, created_at, updated_at
)
SELECT
  'TerranSoul bench resume pattern (BENCH-SCALE-3, 2026-05-16): any ' ||
  'long-running bench that talks to longmemeval-ipc can become ' ||
  'resume-safe by combining four primitives. (1) Expose a `count` op ' ||
  'on the IPC server that returns MemoryStore::count() — general, ' ||
  'reusable across all bench harnesses. (2) Make the corpus ' ||
  'DETERMINISTIC (mulberry32 seed 0x5ca1e1 in locomo-ivfpq.mjs) so row ' ||
  'N is identical between runs and can be safely skipped. (3) Add a ' ||
  '`--resume` flag that preserves the bench store dir (gate the ' ||
  '`rmSync` on `!resume && !reuseStore`), queries the count, and ' ||
  'slices `corpus.slice(count)` before ingest; `--reuse-store` and ' ||
  '`--resume` are mutually exclusive (full-skip vs partial-skip). ' ||
  '(4) Register SIGINT + SIGTERM handlers that flush the progress ' ||
  'snapshot and exit 130/143; SQLite WAL keeps the on-disk store ' ||
  'valid through any signal. Build phase resumes by detecting ' ||
  'sidecar files (idempotent). Query phase resumes via a per-query ' ||
  'JSONL checkpoint (skip query ids already present). The ingest ' ||
  'helper must accept {total, offset} so progress percentages and ' ||
  'question_id namespacing (`scale-${globalOff}`) stay globally ' ||
  'correct after a resume.',
  'seed:lesson-bench-resume-pattern-2026-05-16',
  'procedural',
  'long',
  0.85,
  strftime('%s','now'),
  strftime('%s','now')
WHERE NOT EXISTS (
  SELECT 1 FROM memories WHERE source_hash = 'seed:lesson-bench-resume-pattern-2026-05-16'
);

INSERT INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at)
SELECT lesson.id, hub.id, 'part_of', 1.0, 'seed', strftime('%s','now')
FROM memories lesson
JOIN memories hub ON hub.source_hash = 'seed:lessons-learned-hub'
WHERE lesson.source_hash = 'seed:lesson-bench-resume-pattern-2026-05-16'
  AND NOT EXISTS (
    SELECT 1 FROM memory_edges e
    WHERE e.src_id = lesson.id AND e.dst_id = hub.id AND e.rel_type = 'part_of'
  );


