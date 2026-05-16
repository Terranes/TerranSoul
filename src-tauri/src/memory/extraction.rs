//! GRAPHRAG-1b — Structured entity / relationship extraction at ingest.
//!
//! Calls the active brain with a typed JSON-schema prompt to extract named
//! entities (person, place, concept, event, …) and the relationships between
//! them. Results are materialized as:
//! - `memories` rows with `cognitive_kind = semantic` (via tag `semantic:entity`)
//!   and `memory_type = Context`, tagged `entity:<type>`.
//! - `memory_edges` rows linking entity memories to each other and to their
//!   source memory.
//!
//! The extraction is gated by `AppSettings.graph_extract_enabled` (default
//! off for offline-only sessions — the LLM call cost can be significant).
//!
//! Deduplication: before creating a new entity memory, the extractor checks
//! for an existing memory whose `source_hash` matches `entity:<normalised_name>`.
//! If found, the existing memory ID is reused for edge creation (no duplicate
//! entity row is inserted).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::edges::{normalise_rel_type, EdgeSource, NewMemoryEdge};
use super::store::{MemoryEntry, MemoryStore, MemoryType, NewMemory};

// ── Prompt ────────────────────────────────────────────────────────────────────

/// System prompt for entity/relationship extraction.
pub const EXTRACTION_SYSTEM_PROMPT: &str = r#"You are an entity and relationship extraction engine. Given a text passage, extract all named entities and the relationships between them.

Output ONLY valid JSON with this exact schema (no markdown, no explanation):
{
  "entities": [
    {"name": "...", "type": "person|place|organization|concept|event|object|other", "description": "one sentence description"}
  ],
  "relationships": [
    {"source": "entity name", "target": "entity name", "type": "relationship type", "description": "one sentence", "confidence": 0.0-1.0}
  ]
}

Rules:
- Entity names must be canonical (full proper names, not pronouns).
- Relationship types should be short verb phrases (e.g. "works_at", "located_in", "created_by").
- Only extract entities and relationships that are explicitly stated or strongly implied.
- If no entities or relationships are found, return {"entities": [], "relationships": []}.
- Confidence should reflect how certain the relationship is (1.0 = explicitly stated, 0.5 = implied)."#;

// ── Types ─────────────────────────────────────────────────────────────────────

/// A single extracted entity from the LLM response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    pub name: String,
    #[serde(rename = "type")]
    pub entity_type: String,
    #[serde(default)]
    pub description: String,
}

/// A single extracted relationship from the LLM response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedRelationship {
    pub source: String,
    pub target: String,
    #[serde(rename = "type")]
    pub rel_type: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_confidence")]
    pub confidence: f64,
}

fn default_confidence() -> f64 {
    0.7
}

/// Full extraction result from the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    #[serde(default)]
    pub entities: Vec<ExtractedEntity>,
    #[serde(default)]
    pub relationships: Vec<ExtractedRelationship>,
}

/// Report returned after extraction completes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionReport {
    pub entities_found: usize,
    pub entities_created: usize,
    pub entities_deduplicated: usize,
    pub relationships_found: usize,
    pub edges_created: usize,
    pub source_edges_created: usize,
}

// ── Core extraction logic ─────────────────────────────────────────────────────

/// Build the user prompt for extraction from a memory's content.
pub fn build_extraction_prompt(content: &str) -> String {
    // Cap input to ~8k chars to stay within context windows.
    let trimmed = if content.len() > 8000 {
        let mut end = 8000;
        while end > 0 && !content.is_char_boundary(end) {
            end -= 1;
        }
        &content[..end]
    } else {
        content
    };
    format!("Extract entities and relationships from this text:\n\n{trimmed}")
}

/// Parse the LLM's JSON response into an `ExtractionResult`.
/// Tolerant of markdown fences, trailing commas, etc.
pub fn parse_extraction_response(text: &str) -> Option<ExtractionResult> {
    // Strip markdown code fences if present.
    let cleaned = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // Try direct parse first.
    if let Ok(result) = serde_json::from_str::<ExtractionResult>(cleaned) {
        return Some(result);
    }

    // Try to find JSON object boundaries.
    let start = cleaned.find('{')?;
    let end = cleaned.rfind('}')?;
    if end <= start {
        return None;
    }
    serde_json::from_str::<ExtractionResult>(&cleaned[start..=end]).ok()
}

/// Canonical hash key for entity deduplication.
/// Format: `entity:<lowercase_name>` — used as `source_hash` on entity memories.
pub fn entity_source_hash(name: &str) -> String {
    format!("entity:{}", name.trim().to_lowercase())
}

/// Normalise an entity type string to one of the known categories.
fn normalise_entity_type(raw: &str) -> &'static str {
    match raw.trim().to_lowercase().as_str() {
        "person" | "people" | "character" => "person",
        "place" | "location" | "city" | "country" => "place",
        "organization" | "organisation" | "company" | "group" => "organization",
        "concept" | "idea" | "topic" | "theme" => "concept",
        "event" | "occurrence" | "incident" => "event",
        "object" | "thing" | "item" | "product" => "object",
        _ => "other",
    }
}

/// Materialise extracted entities into the memory store, deduplicating by
/// `source_hash`. Returns a map from normalised entity name → memory ID.
pub fn materialise_entities(
    store: &MemoryStore,
    entities: &[ExtractedEntity],
    source_memory_id: i64,
) -> HashMap<String, i64> {
    let mut name_to_id: HashMap<String, i64> = HashMap::new();

    for entity in entities {
        let name = entity.name.trim().to_string();
        if name.is_empty() || name.len() < 2 {
            continue;
        }
        let hash = entity_source_hash(&name);
        let entity_type = normalise_entity_type(&entity.entity_type);
        let normalised_name = name.to_lowercase();

        // Check if entity already exists (dedup by source_hash).
        if let Some(existing_id) = find_entity_by_hash(store, &hash) {
            name_to_id.insert(normalised_name, existing_id);
            continue;
        }

        // Create new entity memory.
        let content = if entity.description.is_empty() {
            format!("[Entity: {name}] ({entity_type})")
        } else {
            format!("[Entity: {name}] ({entity_type}) — {}", entity.description)
        };

        let new_mem = NewMemory {
            content,
            tags: format!("semantic:entity,entity:{entity_type},entity-name:{name}"),
            importance: 3,
            memory_type: MemoryType::Context,
            source_hash: Some(hash),
            source_url: Some(format!("entity://extracted/{source_memory_id}")),
            ..Default::default()
        };

        match store.add(new_mem) {
            Ok(entry) => {
                name_to_id.insert(normalised_name, entry.id);
            }
            Err(_) => {
                // Insertion failed (likely constraint violation from race);
                // try to find the existing entry by hash.
                if let Some(id) = find_entity_by_hash(store, &entity_source_hash(&name)) {
                    name_to_id.insert(normalised_name, id);
                }
            }
        }
    }

    name_to_id
}

/// Look up an entity memory by its `source_hash`.
fn find_entity_by_hash(store: &MemoryStore, hash: &str) -> Option<i64> {
    store
        .find_by_source_hash(hash)
        .ok()
        .flatten()
        .map(|entry| entry.id)
}

/// Create edges between entities based on extracted relationships, plus
/// "mentions" edges from the source memory to each extracted entity.
pub fn materialise_edges(
    store: &MemoryStore,
    relationships: &[ExtractedRelationship],
    name_to_id: &HashMap<String, i64>,
    source_memory_id: i64,
) -> (usize, usize) {
    let mut entity_edges: Vec<NewMemoryEdge> = Vec::new();
    let mut source_edges: Vec<NewMemoryEdge> = Vec::new();

    // Relationship edges between entities.
    for rel in relationships {
        let src_name = rel.source.trim().to_lowercase();
        let dst_name = rel.target.trim().to_lowercase();
        let src_id = name_to_id.get(&src_name).copied();
        let dst_id = name_to_id.get(&dst_name).copied();

        if let (Some(src), Some(dst)) = (src_id, dst_id) {
            if src != dst {
                entity_edges.push(NewMemoryEdge {
                    src_id: src,
                    dst_id: dst,
                    rel_type: normalise_rel_type(&rel.rel_type),
                    confidence: rel.confidence.clamp(0.0, 1.0),
                    source: EdgeSource::Llm,
                    valid_from: None,
                    valid_to: None,
                    edge_source: Some("graphrag:extraction".to_string()),
                });
            }
        }
    }

    // "mentions" edges from the source memory to each extracted entity.
    for &entity_id in name_to_id.values() {
        if entity_id != source_memory_id {
            source_edges.push(NewMemoryEdge {
                src_id: source_memory_id,
                dst_id: entity_id,
                rel_type: "mentions".to_string(),
                confidence: 1.0,
                source: EdgeSource::Auto,
                valid_from: None,
                valid_to: None,
                edge_source: Some("graphrag:extraction".to_string()),
            });
        }
    }

    let entity_count = store.add_edges_batch(&entity_edges).unwrap_or(0);
    let source_count = store.add_edges_batch(&source_edges).unwrap_or(0);
    (entity_count, source_count)
}

/// Run the full extraction pipeline on a single memory entry.
///
/// 1. Calls the LLM via `complete_via_mode` to extract entities + relationships.
/// 2. Deduplicates entities against existing ones (by `source_hash`).
/// 3. Creates entity memories and edges.
///
/// Returns `None` if the LLM call fails or returns unparseable output.
pub async fn extract_from_memory(
    store: &MemoryStore,
    entry: &MemoryEntry,
    brain_mode: &crate::brain::BrainMode,
    rotator: &std::sync::Mutex<crate::brain::ProviderRotator>,
) -> Option<ExtractionReport> {
    let user_prompt = build_extraction_prompt(&entry.content);
    let reply = crate::memory::brain_memory::complete_via_mode(
        brain_mode,
        EXTRACTION_SYSTEM_PROMPT,
        &user_prompt,
        rotator,
    )
    .await
    .ok()?;

    let result = parse_extraction_response(&reply)?;

    let entities_found = result.entities.len();
    let relationships_found = result.relationships.len();

    if entities_found == 0 {
        return Some(ExtractionReport {
            entities_found: 0,
            entities_created: 0,
            entities_deduplicated: 0,
            relationships_found,
            edges_created: 0,
            source_edges_created: 0,
        });
    }

    // Track how many entities exist before materialisation.
    let existing_count = result
        .entities
        .iter()
        .filter(|e| {
            find_entity_by_hash(store, &entity_source_hash(&e.name)).is_some()
        })
        .count();

    let name_to_id = materialise_entities(store, &result.entities, entry.id);
    let entities_created = entities_found.saturating_sub(existing_count);
    let entities_deduplicated = existing_count;

    let (edges_created, source_edges_created) =
        materialise_edges(store, &result.relationships, &name_to_id, entry.id);

    Some(ExtractionReport {
        entities_found,
        entities_created,
        entities_deduplicated,
        relationships_found,
        edges_created,
        source_edges_created,
    })
}

/// Run extraction on a batch of memory entries. Used by the Tauri command
/// and the auto-fire path. Skips entries that already have extraction edges
/// (idempotent re-runs don't re-extract).
pub async fn extract_from_batch(
    store: &MemoryStore,
    entries: &[MemoryEntry],
    brain_mode: &crate::brain::BrainMode,
    rotator: &std::sync::Mutex<crate::brain::ProviderRotator>,
) -> ExtractionReport {
    let mut total = ExtractionReport {
        entities_found: 0,
        entities_created: 0,
        entities_deduplicated: 0,
        relationships_found: 0,
        edges_created: 0,
        source_edges_created: 0,
    };

    for entry in entries {
        // Skip if this memory already had extraction run (check for outgoing
        // "mentions" edges with edge_source = "graphrag:extraction").
        if has_extraction_edges(store, entry.id) {
            continue;
        }

        // Skip very short entries — not enough content for meaningful extraction.
        if entry.content.len() < 20 {
            continue;
        }

        if let Some(report) = extract_from_memory(store, entry, brain_mode, rotator).await {
            total.entities_found += report.entities_found;
            total.entities_created += report.entities_created;
            total.entities_deduplicated += report.entities_deduplicated;
            total.relationships_found += report.relationships_found;
            total.edges_created += report.edges_created;
            total.source_edges_created += report.source_edges_created;
        }
    }

    total
}

/// Check if a memory already has extraction-sourced outgoing edges.
fn has_extraction_edges(store: &MemoryStore, memory_id: i64) -> bool {
    let sql = "SELECT COUNT(*) FROM memory_edges WHERE src_id = ?1 AND edge_source = 'graphrag:extraction'";
    store
        .conn()
        .query_row(sql, [memory_id], |row| row.get::<_, i64>(0))
        .unwrap_or(0)
        > 0
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_extraction_response_valid_json() {
        let json = r#"{"entities": [{"name": "Alice", "type": "person", "description": "A programmer"}], "relationships": [{"source": "Alice", "target": "Rust", "type": "programs_in", "confidence": 0.9}]}"#;
        let result = parse_extraction_response(json).unwrap();
        assert_eq!(result.entities.len(), 1);
        assert_eq!(result.entities[0].name, "Alice");
        assert_eq!(result.entities[0].entity_type, "person");
        assert_eq!(result.relationships.len(), 1);
        assert_eq!(result.relationships[0].source, "Alice");
        assert_eq!(result.relationships[0].confidence, 0.9);
    }

    #[test]
    fn parse_extraction_response_with_markdown_fences() {
        let json = "```json\n{\"entities\": [{\"name\": \"Bob\", \"type\": \"person\", \"description\": \"\"}], \"relationships\": []}\n```";
        let result = parse_extraction_response(json).unwrap();
        assert_eq!(result.entities.len(), 1);
        assert_eq!(result.entities[0].name, "Bob");
    }

    #[test]
    fn parse_extraction_response_with_preamble_text() {
        let json = "Here are the extracted entities:\n{\"entities\": [{\"name\": \"TerranSoul\", \"type\": \"concept\", \"description\": \"AI companion\"}], \"relationships\": []}";
        let result = parse_extraction_response(json).unwrap();
        assert_eq!(result.entities[0].name, "TerranSoul");
    }

    #[test]
    fn parse_extraction_response_returns_none_for_garbage() {
        assert!(parse_extraction_response("no json here").is_none());
        assert!(parse_extraction_response("").is_none());
    }

    #[test]
    fn parse_extraction_response_handles_empty_result() {
        let json = r#"{"entities": [], "relationships": []}"#;
        let result = parse_extraction_response(json).unwrap();
        assert!(result.entities.is_empty());
        assert!(result.relationships.is_empty());
    }

    #[test]
    fn entity_source_hash_normalises_names() {
        assert_eq!(entity_source_hash("Alice"), "entity:alice");
        assert_eq!(entity_source_hash("  Bob Smith  "), "entity:bob smith");
        assert_eq!(entity_source_hash("COMPANY"), "entity:company");
    }

    #[test]
    fn normalise_entity_type_maps_variants() {
        assert_eq!(normalise_entity_type("Person"), "person");
        assert_eq!(normalise_entity_type("LOCATION"), "place");
        assert_eq!(normalise_entity_type("Company"), "organization");
        assert_eq!(normalise_entity_type("idea"), "concept");
        assert_eq!(normalise_entity_type("incident"), "event");
        assert_eq!(normalise_entity_type("product"), "object");
        assert_eq!(normalise_entity_type("unknown_thing"), "other");
    }

    #[test]
    fn materialise_entities_deduplicates() {
        let store = MemoryStore::in_memory();
        // Pre-insert an entity.
        store
            .add(NewMemory {
                content: "[Entity: Alice] (person) — A developer".to_string(),
                tags: "semantic:entity,entity:person".to_string(),
                importance: 3,
                memory_type: MemoryType::Context,
                source_hash: Some("entity:alice".to_string()),
                ..Default::default()
            })
            .unwrap();

        let entities = vec![
            ExtractedEntity {
                name: "Alice".to_string(),
                entity_type: "person".to_string(),
                description: "A developer".to_string(),
            },
            ExtractedEntity {
                name: "Bob".to_string(),
                entity_type: "person".to_string(),
                description: "Her colleague".to_string(),
            },
        ];

        // Source memory.
        let src = store
            .add(NewMemory {
                content: "Alice works with Bob on the project".to_string(),
                tags: "test".to_string(),
                importance: 4,
                memory_type: MemoryType::Context,
                ..Default::default()
            })
            .unwrap();

        let name_to_id = materialise_entities(&store, &entities, src.id);
        assert_eq!(name_to_id.len(), 2);
        assert!(name_to_id.contains_key("alice"));
        assert!(name_to_id.contains_key("bob"));
        // Alice was deduplicated: total memories = pre-existing Alice + source + new Bob = 3
        assert_eq!(store.count(), 3);
    }

    #[test]
    fn materialise_edges_creates_relationships_and_mentions() {
        let store = MemoryStore::in_memory();
        let a = store
            .add(NewMemory {
                content: "[Entity: Alice] (person)".to_string(),
                tags: "semantic:entity".to_string(),
                importance: 3,
                memory_type: MemoryType::Context,
                source_hash: Some("entity:alice".to_string()),
                ..Default::default()
            })
            .unwrap();
        let b = store
            .add(NewMemory {
                content: "[Entity: Bob] (person)".to_string(),
                tags: "semantic:entity".to_string(),
                importance: 3,
                memory_type: MemoryType::Context,
                source_hash: Some("entity:bob".to_string()),
                ..Default::default()
            })
            .unwrap();
        let src = store
            .add(NewMemory {
                content: "Alice and Bob work together".to_string(),
                tags: "test".to_string(),
                importance: 4,
                memory_type: MemoryType::Context,
                ..Default::default()
            })
            .unwrap();

        let name_to_id: HashMap<String, i64> =
            [("alice".to_string(), a.id), ("bob".to_string(), b.id)]
                .into_iter()
                .collect();

        let rels = vec![ExtractedRelationship {
            source: "Alice".to_string(),
            target: "Bob".to_string(),
            rel_type: "works_with".to_string(),
            description: "Colleagues".to_string(),
            confidence: 0.9,
        }];

        let (entity_edges, source_edges) =
            materialise_edges(&store, &rels, &name_to_id, src.id);
        assert_eq!(entity_edges, 1); // Alice -> Bob "works_with"
        assert_eq!(source_edges, 2); // src -> Alice, src -> Bob "mentions"
    }

    #[test]
    fn materialise_edges_skips_self_loops() {
        let store = MemoryStore::in_memory();
        let a = store
            .add(NewMemory {
                content: "[Entity: Alice] (person)".to_string(),
                tags: "semantic:entity".to_string(),
                importance: 3,
                memory_type: MemoryType::Context,
                source_hash: Some("entity:alice".to_string()),
                ..Default::default()
            })
            .unwrap();

        let name_to_id: HashMap<String, i64> =
            [("alice".to_string(), a.id)].into_iter().collect();

        let rels = vec![ExtractedRelationship {
            source: "Alice".to_string(),
            target: "Alice".to_string(),
            rel_type: "self_reference".to_string(),
            description: "".to_string(),
            confidence: 1.0,
        }];

        let (entity_edges, _source_edges) =
            materialise_edges(&store, &rels, &name_to_id, a.id);
        assert_eq!(entity_edges, 0);
    }

    #[test]
    fn build_extraction_prompt_truncates_long_content() {
        let long = "x".repeat(10_000);
        let prompt = build_extraction_prompt(&long);
        // Should be capped to ~8000 chars of content + prefix.
        assert!(prompt.len() < 8200);
    }

    #[test]
    fn has_extraction_edges_returns_false_initially() {
        let store = MemoryStore::in_memory();
        let entry = store
            .add(NewMemory {
                content: "Test memory".to_string(),
                tags: "test".to_string(),
                importance: 3,
                memory_type: MemoryType::Context,
                ..Default::default()
            })
            .unwrap();
        assert!(!has_extraction_edges(&store, entry.id));
    }
}
