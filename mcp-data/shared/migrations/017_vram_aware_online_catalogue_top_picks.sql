-- Migration 017: Record VRAM-aware online catalogue top-pick fix for existing MCP databases.
-- Applied: 2026-05-09
--
-- Bug: The online model catalogue path (build_top_picks in
-- src-tauri/src/brain/doc_catalogue.rs) used a "biggest model that fits
-- in system RAM" heuristic, which on a 32+ GB system promoted gemma4:31b
-- (20 GB weight, requires 24 GB RAM) to the VeryHigh top-pick. The
-- desktop FirstLaunchWizard then auto-pulled and activated it, even
-- though no consumer GPU (12 GB VRAM RTX 3080 Ti, etc.) can host the
-- weights — producing multi-second TTFT or outright OOM. This contradicts
-- the durable rule already recorded in migration 015.
--
-- Fix:
-- (1) build_top_picks() now uses a VRAM-aware curated tier→tag map that
--     mirrors model_recommender::recommend(): VeryHigh→gemma4:e4b,
--     High→gemma3:4b, Medium→gemma3:1b, Low→gemma3:1b, VeryLow→tinyllama.
-- (2) When the curated tag is missing from the catalogue, the fallback
--     never picks a model with required_ram_mb > 12_288 (12 GB VRAM
--     ceiling), so even an experimental fork of the catalogue cannot
--     trick the wizard into auto-pulling a 20 GB weight.
-- (3) load_cached_catalogue() runs sanitize_top_picks(), which scans
--     every tier's top-pick and replaces any pick whose required_ram_mb
--     exceeds 12 GB with the largest fitting model. This lets users on
--     a stale online cache (~/.local/share/com.terranes.terransoul/
--     model-catalogue.md or %LOCALAPPDATA%\com.terranes.terransoul\
--     model-catalogue.md) get the corrected pick on the very next
--     launch with no manual cache delete.
-- (4) docs/brain-advanced-design.md TOP_PICKS table updated to match
--     so the bundled fallback also reflects VRAM-aware policy.
--
-- Regression coverage in src-tauri/src/brain/doc_catalogue.rs::tests:
--   sanitize_top_picks_repairs_oversized_cached_pick,
--   recommend_high_tier_picks_small_not_31b,
--   build_top_picks_matches_hardcoded_recommend,
--   build_top_picks_online_picks_correct_models.

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, token_count, category, cognitive_kind)
VALUES (
  'LESSON: VRAM-aware online catalogue top-picks (2026-05-09): src-tauri/src/brain/doc_catalogue.rs::build_top_picks must use a curated VRAM-aware tier→tag map (VeryHigh→gemma4:e4b, High→gemma3:4b, Medium/Low→gemma3:1b, VeryLow→tinyllama), NOT "largest model that fits in system RAM" — the latter promoted gemma4:31b (20 GB) on 32+ GB systems and the desktop FirstLaunchWizard then auto-pulled and activated a model no consumer 12 GB GPU can host, producing multi-second TTFT or OOM. The fallback path also enforces a 12 GB required_ram_mb ceiling so an experimental catalogue fork cannot subvert the policy. load_cached_catalogue() runs sanitize_top_picks() which replaces any cached pick with required_ram_mb > 12_288 by the largest fitting model so existing users on a stale online cache get the corrected pick on the very next launch with zero manual intervention. The doc-side TOP_PICKS table in docs/brain-advanced-design.md and SAMPLE_MD test fixture were updated to match. Regression tests: sanitize_top_picks_repairs_oversized_cached_pick, recommend_high_tier_picks_small_not_31b, build_top_picks_matches_hardcoded_recommend, build_top_picks_online_picks_correct_models.',
  'lesson,perf,vram,ollama,model-selection,gemma3,gemma4,doc-catalogue,sanitize,first-launch-wizard,latency,rtx3080ti',
  10, 'procedure', 1746748800000, 'long', 1.0, 220, 'brain', 'procedural'
);
