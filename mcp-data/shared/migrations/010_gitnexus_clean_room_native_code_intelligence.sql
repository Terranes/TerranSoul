-- ============================================================================
-- Migration 010 — GitNexus clean-room native code-intelligence pivot
-- ============================================================================
-- Date: 2026-05-05
-- Trigger: GitNexus is PolyForm Noncommercial, so TerranSoul must not install,
-- bundle, vendor, or default-spawn it. Public docs and DeepWiki can inform
-- clean-room product and architecture research only.
-- ============================================================================

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES
(
  'LESSON: GitNexus is PolyForm Noncommercial. TerranSoul must not bundle, vendor, auto-install, or default-spawn GitNexus packages, binaries, Docker images, prompts, skills, or UI assets. Treat GitNexus and its DeepWiki pages as credited public product and architecture research only, then implement neutral TerranSoul-native Rust/Vue code-intelligence features. Sidecar bridge code is removed entirely; the only supported MCP/code path is native TerranSoul code intelligence.',
  'lesson,gitnexus,license,clean-room,code-intelligence,mcp,native,noncommercial',
  10, 'procedure', 1746489600000, 'long', 1.0, 'lessons', 'procedural'
),
(
  'CODE INTELLIGENCE ROADMAP: Native TerranSoul parity targets learned from public GitNexus docs are repo registry, incremental content-hash indexing, broader Tree-sitter language coverage, import/re-export resolution, heritage and receiver/type inference, confidence-scored code relations, functional clusters, execution processes, hybrid BM25 plus semantic plus RRF code search, diff impact, graph-backed rename, MCP resources/prompts, generated agent skills, and code-wiki generation. Implement these under neutral names using existing coding/symbol_index.rs, coding/processes.rs, memory RAG, and MCP tool surfaces.',
  'roadmap,gitnexus,code-intelligence,native,mcp,symbol-index,processes,search',
  9, 'fact', 1746489600000, 'long', 1.0, 'code-intelligence', 'semantic'
),
(
  'CODE WORKBENCH UX LESSON: GitNexus public Web UI shows a useful AI-development pattern to reimplement natively: graph canvas as the primary structural map, file tree as physical navigation, code references panel as grounded evidence, right-side chat with visible tool-call cards, clickable file/node citations that focus graph and code, process diagrams, repo switcher, status bar, and blast-radius highlights for change risk. TerranSoul should adapt this as a dense Brain/Coding workbench using Vue, Pinia, Cytoscape or Three.js, and existing design tokens, not copy React components or visual identity.',
  'lesson,gitnexus,ui-ux,code-workbench,graph,chat,citations,blast-radius',
  9, 'fact', 1746489600000, 'long', 1.0, 'frontend', 'semantic'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'related_to', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'LESSON: GitNexus is PolyForm Noncommercial%'
  AND (
       d.content LIKE 'Code intelligence pipeline:%'
    OR d.content LIKE 'MCP SELF-LEARNING RULE:%'
    OR d.content LIKE 'DEEPWIKI REVERSE-ENGINEERING RULE:%'
    OR d.content LIKE 'LESSON: Per the Brain Documentation Sync rule%'
  );

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'CODE INTELLIGENCE ROADMAP:%'
  AND d.content LIKE 'Code intelligence pipeline:%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'CODE WORKBENCH UX LESSON:%'
  AND (
       d.content LIKE 'Frontend Pinia stores in src/stores/:%'
    OR d.content LIKE 'Design docs (docs/):%'
  );