-- ============================================================================
-- Migration 003 — Graphify (safishamsi/graphify) reverse-engineering research
-- ============================================================================
-- Date: 2026-05-05
-- Trigger: User request to reverse-engineer https://github.com/safishamsi/graphify
-- Source: DeepWiki (https://deepwiki.com/safishamsi/graphify) + GitHub README
-- License: MIT
-- Stars: 42.9k | Forks: 4.7k | Version: v0.7.5
--
-- Graphify is a Python tool (PyPI: graphifyy) that maps code, docs, PDFs,
-- images, and videos into a queryable knowledge graph. It integrates with
-- Claude Code, Codex, Cursor, Gemini CLI, and other AI coding agents via
-- an MCP stdio server and agent "skills" (slash commands).
--
-- Key architecture: tree-sitter AST extraction (local, no API) for 26+
-- languages, LLM-based semantic extraction for unstructured data, Leiden
-- community detection, "God Nodes" + "Surprising Connections" analysis,
-- SHA256 semantic cache, incremental updates, and graph diffs.
-- ============================================================================

-- ─── Core concept ───────────────────────────────────────────────────────────

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'GRAPHIFY OVERVIEW: safishamsi/graphify (MIT, 42.9k stars, Python) is a knowledge-graph builder for AI coding agents. It maps code (26 languages via tree-sitter AST), docs, PDFs, images, and video into a queryable NetworkX graph. Outputs: graph.json (full KG), graph.html (interactive vis.js treemap), GRAPH_REPORT.md (highlights). Achieves 71.5x token reduction vs raw file reading for complex queries. Works via slash-command skill (/graphify .) or CLI. MCP stdio server exposes tools: query_graph, get_node, get_neighbors, get_community, god_nodes, graph_stats, shortest_path.',
  'graphify,research,knowledge-graph,reverse-engineering,mcp,tree-sitter,overview',
  5, 'fact', 1746489600000, 'long', 1.0, 'research', 'semantic'
);

-- ─── Pipeline architecture ──────────────────────────────────────────────────

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'GRAPHIFY PIPELINE: Linear 8-stage flow — collect_files() → extract() [tree-sitter AST for code, LLM for docs/PDFs/images] → validate_extraction() → build_graph() [NetworkX, ID normalization, dedup] → cluster() [Leiden community detection, fallback Louvain, oversized splitting at 25%] → analyze() [God Nodes by degree centrality, Surprising Connections by cross-community bridge scoring] → render_report() → export() [HTML/SVG/Obsidian/wiki/Neo4j/GraphML]. Each stage communicates via plain dicts and nx.Graph objects.',
  'graphify,pipeline,architecture,knowledge-graph,leiden,community-detection',
  4, 'fact', 1746489600000, 'long', 1.0, 'research', 'semantic'
);

-- ─── Extraction schema ──────────────────────────────────────────────────────

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'GRAPHIFY SCHEMA: Nodes have {id, label, file_type (code|document|paper|image|concept), source_file}. Edges have {source, target, relation, confidence (EXTRACTED|INFERRED|AMBIGUOUS), weight}. IDs are normalized to lowercase alphanumeric via _make_id/_normalize_id. Deduplication: 3-layer — file-level seen_ids set, cross-file idempotent add_node (last attr wins), semantic merge via explicit seen set. Edge direction preserved via _src/_tgt attributes even in undirected graphs. Hyperedges stored in G.graph["hyperedges"] for group relationships.',
  'graphify,schema,extraction,confidence-tags,deduplication,edges',
  4, 'fact', 1746489600000, 'long', 1.0, 'research', 'semantic'
);

-- ─── Surprise scoring algorithm ─────────────────────────────────────────────

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'GRAPHIFY SURPRISE SCORING: Detects non-obvious cross-boundary edges. Multi-signal formula: confidence_bonus (+3 Ambiguous, +2 Inferred), cross_type (+2 for code↔paper/image), cross_repo (+2 for different top-level dirs), cross_community (+1 for different Leiden clusters), semantic_similarity_multiplier (x1.5 for relation=semantically_similar_to), peripheral_to_hub (+1 when degree≤2 node connects to degree≥5 node). Strategy: multi-file corpora use cross-file surprises ranked by score; single-file corpora use betweenness centrality across communities.',
  'graphify,surprise-scoring,analysis,cross-community,knowledge-graph,algorithm',
  4, 'fact', 1746489600000, 'long', 1.0, 'research', 'semantic'
);

-- ─── MCP server tools ───────────────────────────────────────────────────────

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'GRAPHIFY MCP SERVER: stdio JSON-RPC server (serve.py) loads graph.json into nx.Graph. 7 tools: (1) query_graph — keyword scoring (1.0 for label match, 0.5 for source_file match, +100 exact bonus) then BFS/DFS traversal with token budget (3 chars ≈ 1 token, priority by degree); (2) get_node — full metadata for a node; (3) get_neighbors — direct neighbors + edge relations; (4) get_community — all nodes in a Leiden cluster; (5) god_nodes — top-N by degree, filtering file-level hubs and method stubs; (6) graph_stats — node/edge counts + community distribution; (7) shortest_path — logic path between two entities. Sanitizes labels to prevent prompt injection.',
  'graphify,mcp-server,tools,query,traversal,token-budget',
  4, 'fact', 1746489600000, 'long', 1.0, 'research', 'semantic'
);

-- ─── Incremental updates & caching ──────────────────────────────────────────

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'GRAPHIFY CACHING & INCREMENTAL: SHA256-based semantic cache stores extractions in graphify-out/cache/{ast|semantic}/{hash}.json. Markdown files hash only body (skipping YAML frontmatter). graph_diff() compares old vs new graph to produce {new_nodes, removed_nodes, new_edges, summary_text}. File watcher (watch.py) auto-syncs on changes. Git hooks (post-commit + post-checkout) auto-rebuild AST-only (no API cost). Git merge driver union-merges graph.json to avoid conflicts. --update flag re-extracts only changed files.',
  'graphify,caching,incremental,sha256,git-hooks,file-watcher,diff',
  3, 'fact', 1746489600000, 'long', 1.0, 'research', 'semantic'
);

-- ─── Ideas for TerranSoul improvement ───────────────────────────────────────

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'GRAPHIFY IDEAS FOR TERRANSOUL: (1) Add Leiden/Louvain community detection to memory_edges KG — cluster related memories into communities, expose via brain_kg_communities MCP tool. (2) Add confidence level (extracted/inferred/ambiguous) to memory_edges — currently edges have weight+rel_type but no provenance confidence. (3) Implement surprise scoring — identify cross-community bridge memories that connect distant topics (high-value for RAG). (4) Add graph_diff capability — diff memory state between sessions to show what changed. (5) Improve brain_suggest_context to use God Node analysis (highest-degree memories are most contextually relevant). (6) Consider auto-generating suggested questions from hub+bridge analysis. (7) Token-budgeted subgraph serialization already exists in context_budget.rs — validate it matches Graphify''s priority-by-degree approach.',
  'graphify,improvement-ideas,terransoul,knowledge-graph,community-detection,surprise-scoring',
  5, 'fact', 1746489600000, 'long', 1.0, 'self-improve', 'procedural'
);

-- ─── Edges linking research entries ─────────────────────────────────────────

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'informs', 0.9, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'GRAPHIFY IDEAS FOR TERRANSOUL:%'
  AND d.content LIKE 'Brain module map:%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'related_to', 0.8, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'GRAPHIFY MCP SERVER:%'
  AND d.content LIKE 'MCP EVERY-SESSION RULE:%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'part_of', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'GRAPHIFY PIPELINE:%'
  AND d.content LIKE 'GRAPHIFY OVERVIEW:%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'part_of', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'GRAPHIFY SCHEMA:%'
  AND d.content LIKE 'GRAPHIFY OVERVIEW:%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'part_of', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'GRAPHIFY SURPRISE SCORING:%'
  AND d.content LIKE 'GRAPHIFY OVERVIEW:%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'part_of', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'GRAPHIFY MCP SERVER:%'
  AND d.content LIKE 'GRAPHIFY OVERVIEW:%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'part_of', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'GRAPHIFY CACHING%'
  AND d.content LIKE 'GRAPHIFY OVERVIEW:%';
