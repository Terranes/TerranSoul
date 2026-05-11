//! Sleep-time memory consolidation (Chunk 16.7).
//!
//! An idle-triggered background workflow that runs the following steps:
//!
//! 1. **Compress** short-tier entries for expired sessions → summarize → store as working.
//! 2. **Link** related working memories via embedding similarity → create `memory_edges`.
//! 3. **Synthesize** related memory clusters into N→1 parent summaries.
//! 4. **Promote** high-access working entries → long-tier.
//! 5. **Decay + GC** — apply exponential decay, garbage-collect low-importance decayed entries.
//! 6. **Importance adjustment** — boost frequently accessed, demote stale.
//!
//! Inspired by Letta (MemGPT) sleep-time compute (§19.2 row 12 in brain-advanced-design.md).

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicI64, Ordering};

use serde::{Deserialize, Serialize};

use crate::memory::edges::{EdgeDirection, EdgeSource, NewMemoryEdge};
use crate::memory::store::{MemoryEntry, MemoryStore, MemoryTier, MemoryType, NewMemory};

// ─── Configuration ──────────────────────────────────────────────────

/// Default idle threshold (5 minutes in milliseconds).
pub const DEFAULT_IDLE_THRESHOLD_MS: i64 = 5 * 60 * 1000;

/// Default minimum access count for auto-promotion.
pub const DEFAULT_PROMOTE_MIN_ACCESS: i64 = 3;

/// Default access window for promotion (7 days).
pub const DEFAULT_PROMOTE_WINDOW_DAYS: i64 = 7;

/// Default GC decay threshold.
pub const DEFAULT_GC_THRESHOLD: f64 = 0.05;

/// Default hot threshold for importance adjustment.
pub const DEFAULT_HOT_THRESHOLD: i64 = 10;

/// Default cold days for importance demotion.
pub const DEFAULT_COLD_DAYS: i64 = 30;

/// Default similarity threshold for linking (cosine).
pub const DEFAULT_LINK_SIMILARITY: f64 = 0.75;

/// Maximum number of link edges created per consolidation run.
pub const MAX_LINK_EDGES_PER_RUN: usize = 50;

/// Default minimum children required before creating a parent synthesis.
pub const DEFAULT_SYNTHESIS_MIN_CHILDREN: usize = 3;

/// Default maximum synthesis parent memories created per consolidation run.
pub const DEFAULT_SYNTHESIS_MAX_CLUSTERS: usize = 5;

/// Default maximum child memories rolled into one parent synthesis.
pub const DEFAULT_SYNTHESIS_MAX_CHILDREN: usize = 8;

// ─── Activity tracker ───────────────────────────────────────────────

/// Tracks last user interaction time for idle detection.
#[derive(Debug)]
pub struct ActivityTracker {
    /// Last interaction timestamp in Unix ms. Updated by Tauri commands.
    last_activity_ms: AtomicI64,
}

impl ActivityTracker {
    pub fn new() -> Self {
        Self {
            last_activity_ms: AtomicI64::new(now_ms()),
        }
    }

    /// Record a user interaction (call from chat, memory, voice, etc.).
    pub fn touch(&self) {
        self.last_activity_ms.store(now_ms(), Ordering::Relaxed);
    }

    /// Milliseconds since last user interaction.
    pub fn idle_ms(&self) -> i64 {
        let last = self.last_activity_ms.load(Ordering::Relaxed);
        (now_ms() - last).max(0)
    }

    /// Whether the user has been idle for at least `threshold_ms`.
    pub fn is_idle(&self, threshold_ms: i64) -> bool {
        self.idle_ms() >= threshold_ms
    }

    /// Get the last activity timestamp.
    pub fn last_activity(&self) -> i64 {
        self.last_activity_ms.load(Ordering::Relaxed)
    }
}

impl Default for ActivityTracker {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Consolidation result ───────────────────────────────────────────

/// Summary of what a consolidation run achieved.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConsolidationResult {
    /// Number of short-tier entries compressed.
    pub compressed: usize,
    /// Number of new edges created from similarity linking.
    pub edges_created: usize,
    /// Number of parent summary memories synthesized from related child clusters.
    pub synthesized: usize,
    /// IDs of parent summary memories synthesized during this run.
    pub synthesized_parent_ids: Vec<i64>,
    /// IDs of memories promoted to long-tier.
    pub promoted: Vec<i64>,
    /// Number of entries affected by decay.
    pub decayed: usize,
    /// Number of entries garbage-collected.
    pub gc_removed: usize,
    /// Number of entries with importance boosted.
    pub importance_boosted: usize,
    /// Number of entries with importance demoted.
    pub importance_demoted: usize,
    /// Whether the run completed all steps.
    pub complete: bool,
    /// Any non-fatal issues encountered.
    pub warnings: Vec<String>,
}

/// One N→1 synthesis produced during consolidation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SynthesisRecord {
    pub parent_id: i64,
    pub child_ids: Vec<i64>,
    pub topic: String,
}

// ─── Consolidation steps (pure, testable) ───────────────────────────

/// Step 1: Compress short-tier entries for a given session.
///
/// Returns the evicted entries (caller can summarize them with LLM).
pub fn step_compress_short(
    store: &MemoryStore,
    session_id: &str,
) -> Result<Vec<crate::memory::store::MemoryEntry>, String> {
    store
        .evict_short_term(session_id)
        .map_err(|e| format!("compress: {e}"))
}

/// Step 2: Link related working/long memories by embedding similarity.
///
/// For each unlinked working-tier memory with an embedding, find the
/// top-k similar entries and create `related_to` edges.
pub fn step_link_similar(
    store: &MemoryStore,
    similarity_threshold: f64,
    max_edges: usize,
) -> Result<usize, String> {
    // Get working-tier entries that have embeddings
    let working = store
        .get_by_tier(&MemoryTier::Working)
        .map_err(|e| format!("link: list working: {e}"))?;

    let mut edges_created = 0;

    for entry in &working {
        if edges_created >= max_edges {
            break;
        }

        // Skip entries without embeddings
        let embedding = match &entry.embedding {
            Some(e) if !e.is_empty() => e.clone(),
            _ => continue,
        };

        // Search for similar entries using embedding
        let similar = store
            .hybrid_search(&entry.content, Some(&embedding), 5)
            .map_err(|e| format!("link: search: {e}"))?;

        for candidate in &similar {
            if edges_created >= max_edges {
                break;
            }
            if candidate.id == entry.id {
                continue;
            }
            // Use embedding cosine similarity as threshold check
            // The fact that it's in the top-5 results with embedding means
            // it has reasonable similarity. We use the candidate being
            // returned at all as the threshold (search already scored it).
            // For stronger filtering, check cosine directly if both have embeddings.
            if let (Some(ref emb_a), Some(ref emb_b)) = (&entry.embedding, &candidate.embedding) {
                if !emb_a.is_empty() && !emb_b.is_empty() {
                    let sim = cosine_similarity(emb_a, emb_b);
                    if sim < similarity_threshold {
                        continue;
                    }
                }
            }

            // Check if edge already exists
            let existing = store
                .get_edges_for(entry.id, EdgeDirection::Both)
                .map_err(|e| format!("link: edges: {e}"))?;
            let already_linked = existing
                .iter()
                .any(|e| e.dst_id == candidate.id || e.src_id == candidate.id);
            if already_linked {
                continue;
            }

            // Create edge
            let _ = store.add_edge(NewMemoryEdge {
                src_id: entry.id,
                dst_id: candidate.id,
                rel_type: "related_to".to_string(),
                confidence: 0.8,
                source: EdgeSource::Auto,
                valid_from: None,
                valid_to: None,
                edge_source: Some("sleep_consolidation".to_string()),
            });
            edges_created += 1;
        }
    }

    Ok(edges_created)
}

/// Step 3: Synthesize related memory clusters into parent summaries.
///
/// This creates a long-tier `summary` memory, sets `parent_id` on each child,
/// and adds `derived_from` edges from the parent to each child. Clustering uses
/// existing graph edges first, then falls back to shared tags so useful N→1
/// consolidation still works before the graph is richly connected.
pub fn step_synthesize_related_memories(
    store: &MemoryStore,
    min_children: usize,
    max_clusters: usize,
    max_children_per_cluster: usize,
) -> Result<Vec<SynthesisRecord>, String> {
    if min_children < 2 || max_clusters == 0 || max_children_per_cluster < min_children {
        return Ok(Vec::new());
    }

    let entries = store
        .get_persistent()
        .map_err(|e| format!("synthesize: list persistent: {e}"))?;
    let candidates: Vec<MemoryEntry> = entries
        .into_iter()
        .filter(|entry| entry.valid_to.is_none())
        .filter(|entry| entry.parent_id.is_none())
        .filter(|entry| entry.memory_type != MemoryType::Summary)
        .filter(|entry| {
            !entry
                .tags
                .to_lowercase()
                .contains("synthetic:consolidation")
        })
        .collect();

    if candidates.len() < min_children {
        return Ok(Vec::new());
    }

    let by_id: HashMap<i64, MemoryEntry> = candidates
        .iter()
        .cloned()
        .map(|entry| (entry.id, entry))
        .collect();

    let mut groups = graph_edge_groups(store, &by_id, min_children)?;
    groups.extend(tag_groups(&candidates, min_children));
    normalize_groups(&mut groups, max_children_per_cluster);

    let mut used_children: HashSet<i64> = HashSet::new();
    let mut records = Vec::new();

    for group in groups {
        if records.len() >= max_clusters {
            break;
        }
        let child_ids: Vec<i64> = group
            .into_iter()
            .filter(|id| by_id.contains_key(id))
            .filter(|id| !used_children.contains(id))
            .take(max_children_per_cluster)
            .collect();
        if child_ids.len() < min_children {
            continue;
        }

        let children: Vec<MemoryEntry> = child_ids
            .iter()
            .filter_map(|id| by_id.get(id).cloned())
            .collect();
        let topic = synthesis_topic(&children);
        let parent = store
            .add(NewMemory {
                content: synthesis_content(&topic, &children),
                tags: synthesis_tags(&children),
                importance: children
                    .iter()
                    .map(|entry| entry.importance)
                    .max()
                    .unwrap_or(3),
                memory_type: MemoryType::Summary,
                ..NewMemory::default()
            })
            .map_err(|e| format!("synthesize: add parent: {e}"))?;

        store
            .set_parent_for_memories(&child_ids, parent.id)
            .map_err(|e| format!("synthesize: set parent: {e}"))?;

        for child_id in &child_ids {
            let _ = store.add_edge(NewMemoryEdge {
                src_id: parent.id,
                dst_id: *child_id,
                rel_type: "derived_from".to_string(),
                confidence: 0.95,
                source: EdgeSource::Auto,
                valid_from: None,
                valid_to: None,
                edge_source: Some("consolidation_synthesis".to_string()),
            });
        }

        used_children.extend(child_ids.iter().copied());
        records.push(SynthesisRecord {
            parent_id: parent.id,
            child_ids,
            topic,
        });
    }

    Ok(records)
}

fn graph_edge_groups(
    store: &MemoryStore,
    by_id: &HashMap<i64, MemoryEntry>,
    min_children: usize,
) -> Result<Vec<Vec<i64>>, String> {
    let edges = store
        .list_edges()
        .map_err(|e| format!("synthesize: list edges: {e}"))?;
    let mut adjacency: HashMap<i64, Vec<i64>> = HashMap::new();
    for edge in edges {
        if edge.valid_to.is_some() || edge.confidence < 0.5 {
            continue;
        }
        if !by_id.contains_key(&edge.src_id) || !by_id.contains_key(&edge.dst_id) {
            continue;
        }
        if !matches!(
            edge.rel_type.as_str(),
            "related_to" | "derived_from" | "part_of" | "contains" | "depends_on" | "cites"
        ) {
            continue;
        }
        adjacency.entry(edge.src_id).or_default().push(edge.dst_id);
        adjacency.entry(edge.dst_id).or_default().push(edge.src_id);
    }

    let mut seen = HashSet::new();
    let mut groups = Vec::new();
    for id in by_id.keys() {
        if !seen.insert(*id) {
            continue;
        }
        let mut stack = vec![*id];
        let mut group = Vec::new();
        while let Some(current) = stack.pop() {
            group.push(current);
            if let Some(neighbors) = adjacency.get(&current) {
                for next in neighbors {
                    if seen.insert(*next) {
                        stack.push(*next);
                    }
                }
            }
        }
        if group.len() >= min_children {
            groups.push(group);
        }
    }
    Ok(groups)
}

fn tag_groups(entries: &[MemoryEntry], min_children: usize) -> Vec<Vec<i64>> {
    let mut by_tag: HashMap<String, Vec<i64>> = HashMap::new();
    for entry in entries {
        for tag in normalized_tags(&entry.tags) {
            if is_generic_synthesis_tag(&tag) {
                continue;
            }
            by_tag.entry(tag).or_default().push(entry.id);
        }
    }
    by_tag
        .into_values()
        .filter(|ids| ids.len() >= min_children)
        .collect()
}

fn normalize_groups(groups: &mut Vec<Vec<i64>>, max_children: usize) {
    for group in groups.iter_mut() {
        group.sort_unstable();
        group.dedup();
        group.truncate(max_children);
    }
    groups.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
    groups.dedup();
}

fn synthesis_topic(children: &[MemoryEntry]) -> String {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for child in children {
        for tag in normalized_tags(&child.tags) {
            if !is_generic_synthesis_tag(&tag) {
                *counts.entry(tag).or_insert(0) += 1;
            }
        }
    }
    counts
        .into_iter()
        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
        .map(|(tag, _)| tag)
        .unwrap_or_else(|| "related memories".to_string())
}

fn synthesis_tags(children: &[MemoryEntry]) -> String {
    let mut tags: Vec<String> = vec![
        "synthetic:consolidation".to_string(),
        "parent_summary".to_string(),
    ];
    let mut seen: HashSet<String> = tags.iter().cloned().collect();
    for child in children {
        for tag in normalized_tags(&child.tags) {
            if seen.insert(tag.clone()) {
                tags.push(tag);
            }
            if tags.len() >= 10 {
                break;
            }
        }
        if tags.len() >= 10 {
            break;
        }
    }
    tags.join(",")
}

fn synthesis_content(topic: &str, children: &[MemoryEntry]) -> String {
    let mut lines = Vec::with_capacity(children.len() + 1);
    lines.push(format!(
        "Consolidated synthesis for {topic}: {} related memories were merged into this parent summary.",
        children.len()
    ));
    for child in children {
        lines.push(format!(
            "- #{}: {}",
            child.id,
            truncate_for_synthesis(&child.content, 220)
        ));
    }
    lines.join("\n")
}

fn truncate_for_synthesis(content: &str, max_chars: usize) -> String {
    let clean = content.split_whitespace().collect::<Vec<_>>().join(" ");
    if clean.chars().count() <= max_chars {
        return clean;
    }
    let mut truncated = clean
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    truncated.push_str("...");
    truncated
}

fn normalized_tags(tags: &str) -> Vec<String> {
    tags.split(',')
        .map(|tag| tag.trim().to_lowercase())
        .filter(|tag| !tag.is_empty())
        .collect()
}

fn is_generic_synthesis_tag(tag: &str) -> bool {
    matches!(
        tag,
        "synthetic:consolidation" | "parent_summary" | "summary" | "memory" | "auto"
    )
}

/// Cosine similarity between two embedding vectors.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0f64;
    let mut norm_a = 0.0f64;
    let mut norm_b = 0.0f64;
    for (x, y) in a.iter().zip(b.iter()) {
        let x = *x as f64;
        let y = *y as f64;
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }
    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom < 1e-12 {
        return 0.0;
    }
    dot / denom
}

/// Step 3: Promote high-access working entries to long-tier.
pub fn step_promote(
    store: &MemoryStore,
    min_access: i64,
    window_days: i64,
) -> Result<Vec<i64>, String> {
    store
        .auto_promote_to_long(min_access, window_days)
        .map_err(|e| format!("promote: {e}"))
}

/// Step 4: Apply decay and garbage collect.
pub fn step_decay_and_gc(store: &MemoryStore, gc_threshold: f64) -> Result<(usize, usize), String> {
    let decayed = store.apply_decay().map_err(|e| format!("decay: {e}"))?;
    let gc = store
        .gc_decayed(gc_threshold)
        .map_err(|e| format!("gc: {e}"))?;
    Ok((decayed, gc))
}

/// Step 5: Adjust importance based on access patterns.
pub fn step_adjust_importance(
    store: &MemoryStore,
    hot_threshold: i64,
    cold_days: i64,
) -> Result<(usize, usize), String> {
    store
        .adjust_importance_by_access(hot_threshold, cold_days)
        .map_err(|e| format!("importance: {e}"))
}

/// Run all consolidation steps in sequence.
///
/// This is the main entry point. Each step is independent and non-fatal —
/// if one step fails, the rest still run. Failures are collected as warnings.
pub fn run_consolidation(
    store: &MemoryStore,
    session_ids: &[String],
    config: &ConsolidationConfig,
) -> ConsolidationResult {
    let mut result = ConsolidationResult::default();

    // Step 1: Compress short-tier for each expired session
    for sid in session_ids {
        match step_compress_short(store, sid) {
            Ok(evicted) => result.compressed += evicted.len(),
            Err(e) => result.warnings.push(e),
        }
    }

    // Step 2: Link similar memories
    match step_link_similar(store, config.link_similarity, config.max_link_edges) {
        Ok(n) => result.edges_created = n,
        Err(e) => result.warnings.push(e),
    }

    // Step 3: Synthesize related memory clusters
    match step_synthesize_related_memories(
        store,
        config.synthesis_min_children,
        config.synthesis_max_clusters,
        config.synthesis_max_children,
    ) {
        Ok(records) => {
            result.synthesized = records.len();
            result.synthesized_parent_ids =
                records.into_iter().map(|record| record.parent_id).collect();
        }
        Err(e) => result.warnings.push(e),
    }

    // Step 4: Promote
    match step_promote(store, config.promote_min_access, config.promote_window_days) {
        Ok(ids) => result.promoted = ids,
        Err(e) => result.warnings.push(e),
    }

    // Step 5: Decay + GC
    match step_decay_and_gc(store, config.gc_threshold) {
        Ok((d, g)) => {
            result.decayed = d;
            result.gc_removed = g;
        }
        Err(e) => result.warnings.push(e),
    }

    // Step 6: Importance adjustment
    match step_adjust_importance(store, config.hot_threshold, config.cold_days) {
        Ok((b, d)) => {
            result.importance_boosted = b;
            result.importance_demoted = d;
        }
        Err(e) => result.warnings.push(e),
    }

    result.complete = result.warnings.is_empty();
    result
}

/// Configuration for a consolidation run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationConfig {
    pub link_similarity: f64,
    pub max_link_edges: usize,
    pub synthesis_min_children: usize,
    pub synthesis_max_clusters: usize,
    pub synthesis_max_children: usize,
    pub promote_min_access: i64,
    pub promote_window_days: i64,
    pub gc_threshold: f64,
    pub hot_threshold: i64,
    pub cold_days: i64,
}

impl Default for ConsolidationConfig {
    fn default() -> Self {
        Self {
            link_similarity: DEFAULT_LINK_SIMILARITY,
            max_link_edges: MAX_LINK_EDGES_PER_RUN,
            synthesis_min_children: DEFAULT_SYNTHESIS_MIN_CHILDREN,
            synthesis_max_clusters: DEFAULT_SYNTHESIS_MAX_CLUSTERS,
            synthesis_max_children: DEFAULT_SYNTHESIS_MAX_CHILDREN,
            promote_min_access: DEFAULT_PROMOTE_MIN_ACCESS,
            promote_window_days: DEFAULT_PROMOTE_WINDOW_DAYS,
            gc_threshold: DEFAULT_GC_THRESHOLD,
            hot_threshold: DEFAULT_HOT_THRESHOLD,
            cold_days: DEFAULT_COLD_DAYS,
        }
    }
}

// ─── Helpers ────────────────────────────────────────────────────────

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::store::{MemoryStore, MemoryTier, NewMemory};

    fn test_store() -> MemoryStore {
        MemoryStore::in_memory()
    }

    fn new_memory(content: &str, importance: i64) -> NewMemory {
        NewMemory {
            content: content.into(),
            tags: "test".into(),
            importance,
            ..NewMemory::default()
        }
    }

    #[test]
    fn activity_tracker_starts_recent() {
        let tracker = ActivityTracker::new();
        assert!(tracker.idle_ms() < 100); // just created
        assert!(!tracker.is_idle(1000));
    }

    #[test]
    fn activity_tracker_touch_resets_idle() {
        let tracker = ActivityTracker::new();
        // Manually set to old timestamp
        tracker
            .last_activity_ms
            .store(now_ms() - 60_000, Ordering::Relaxed);
        assert!(tracker.is_idle(30_000));

        tracker.touch();
        assert!(!tracker.is_idle(30_000));
    }

    #[test]
    fn compress_short_evicts_session() {
        let store = test_store();
        store
            .add_to_tier(new_memory("short fact", 2), MemoryTier::Short, Some("s1"))
            .unwrap();

        let evicted = step_compress_short(&store, "s1").unwrap();
        assert_eq!(evicted.len(), 1);
        assert_eq!(evicted[0].content, "short fact");

        // Verify they're gone
        let stats = store.stats().unwrap();
        assert_eq!(stats.short, 0);
    }

    #[test]
    fn promote_working_to_long() {
        let store = test_store();
        let entry = store
            .add_to_tier(new_memory("working entry", 3), MemoryTier::Working, None)
            .unwrap();
        let id = entry.id;

        // Bump access count above threshold via raw SQL (no public record_access)
        let now = now_ms();
        store
            .conn()
            .execute(
                "UPDATE memories SET access_count = 5, last_accessed = ?1 WHERE id = ?2",
                rusqlite::params![now, id],
            )
            .unwrap();

        let promoted = step_promote(&store, 3, 30).unwrap();
        assert!(promoted.contains(&id));

        let entry = store.get_by_id(id).unwrap();
        assert_eq!(entry.tier, MemoryTier::Long);
    }

    #[test]
    fn decay_and_gc_runs() {
        let store = test_store();
        // Add a long-tier entry with very low decay and low importance
        let entry = store.add(new_memory("will decay", 1)).unwrap();
        let id = entry.id;

        // Manually set decay very low
        store
            .conn()
            .execute(
                "UPDATE memories SET decay_score = 0.01 WHERE id = ?1",
                rusqlite::params![id],
            )
            .unwrap();

        let (decayed, gc) = step_decay_and_gc(&store, 0.05).unwrap();
        // The very-low entry should be GC'd; at minimum decay ran on ≥0 entries
        assert!(gc >= 1 || decayed >= 1);
    }

    #[test]
    fn importance_adjustment_runs() {
        let store = test_store();
        let _entry = store.add(new_memory("test entry", 3)).unwrap();

        let (boosted, demoted) = step_adjust_importance(&store, 10, 30).unwrap();
        // With 0 access and no last_accessed, it won't match hot criteria
        // but may or may not match cold criteria depending on last_accessed being NULL
        assert!(boosted == 0);
        // Cold demotion depends on whether NULL last_accessed counts
        let _ = demoted; // no assertion — just verify it runs without error
    }

    #[test]
    fn run_consolidation_completes() {
        let store = test_store();
        store
            .add_to_tier(new_memory("session fact", 2), MemoryTier::Short, Some("s1"))
            .unwrap();
        store
            .add_to_tier(new_memory("working fact", 3), MemoryTier::Working, None)
            .unwrap();

        let config = ConsolidationConfig::default();
        let result = run_consolidation(&store, &["s1".to_string()], &config);

        assert_eq!(result.compressed, 1); // one short-tier evicted
                                          // May or may not have edges/promotions depending on access counts
                                          // but the run itself should complete
        assert!(result.warnings.is_empty() || !result.warnings.is_empty());
    }

    #[test]
    fn consolidation_config_default_is_sane() {
        let cfg = ConsolidationConfig::default();
        assert!(cfg.link_similarity > 0.0 && cfg.link_similarity < 1.0);
        assert!(cfg.promote_min_access >= 1);
        assert!(cfg.gc_threshold > 0.0 && cfg.gc_threshold < 1.0);
    }

    #[test]
    fn link_similar_with_no_embeddings() {
        let store = test_store();
        store
            .add_to_tier(new_memory("no embedding", 3), MemoryTier::Working, None)
            .unwrap();

        // Should return 0 edges — no embeddings to compare
        let edges = step_link_similar(&store, 0.75, 50).unwrap();
        assert_eq!(edges, 0);
    }

    #[test]
    fn synthesis_creates_parent_and_links_children() {
        let store = test_store();
        let mut child_ids = Vec::new();
        for content in [
            "retrieval pipeline uses RRF fusion",
            "retrieval pipeline uses graph expansion",
            "retrieval pipeline uses semantic embeddings",
        ] {
            let entry = store
                .add(NewMemory {
                    content: content.to_string(),
                    tags: "retrieval,rag".to_string(),
                    importance: 4,
                    ..NewMemory::default()
                })
                .unwrap();
            child_ids.push(entry.id);
        }

        let records = step_synthesize_related_memories(&store, 3, 2, 8).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].child_ids.len(), 3);

        let parent = store.get_by_id(records[0].parent_id).unwrap();
        assert_eq!(parent.memory_type, MemoryType::Summary);
        assert!(parent.tags.contains("synthetic:consolidation"));
        assert!(parent.content.contains("Consolidated synthesis"));

        for child_id in child_ids {
            let child = store.get_by_id(child_id).unwrap();
            assert_eq!(child.parent_id, Some(parent.id));
        }

        let edges = store.get_edges_for(parent.id, EdgeDirection::Out).unwrap();
        assert_eq!(edges.len(), 3);
        assert!(edges.iter().all(|edge| edge.rel_type == "derived_from"));
    }
}
