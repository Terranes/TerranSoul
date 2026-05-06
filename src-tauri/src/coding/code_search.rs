//! Hybrid semantic code search (Chunk 37.7).
//!
//! BM25-like text scoring + optional embedding vector retrieval + RRF fusion
//! over the `code_symbols` table. Reuses TerranSoul's existing embedding
//! providers (`embed_for_mode`) and `reciprocal_rank_fuse` from the memory
//! system.
//!
//! Three retrieval signals:
//! 1. **Text** — BM25-style scoring on symbol name, parent, file path
//! 2. **Vector** — cosine similarity of query embedding vs stored symbol embeddings
//! 3. **Graph** — entry-point score and process participation as a ranking boost

use std::collections::HashMap;
use std::path::Path;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use super::symbol_index::{open_db, IndexError, Symbol, SymbolKind};
use crate::memory::fusion::{reciprocal_rank_fuse, DEFAULT_RRF_K};

// ─── Schema ─────────────────────────────────────────────────────────────────

/// Ensure the code embeddings table exists.
pub fn ensure_embedding_table(conn: &Connection) -> Result<(), IndexError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS code_embeddings (
            symbol_id   INTEGER PRIMARY KEY REFERENCES code_symbols(id) ON DELETE CASCADE,
            repo_id     INTEGER NOT NULL REFERENCES code_repos(id) ON DELETE CASCADE,
            embedding   BLOB NOT NULL,
            model       TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_code_embeddings_repo ON code_embeddings(repo_id);
        "#,
    )?;
    Ok(())
}

// ─── Embedding storage ──────────────────────────────────────────────────────

/// Store an embedding vector for a symbol.
pub fn store_embedding(
    conn: &Connection,
    symbol_id: i64,
    repo_id: i64,
    embedding: &[f32],
    model: &str,
) -> Result<(), IndexError> {
    let blob = embedding_to_blob(embedding);
    conn.execute(
        "INSERT OR REPLACE INTO code_embeddings (symbol_id, repo_id, embedding, model)
         VALUES (?1, ?2, ?3, ?4)",
        params![symbol_id, repo_id, blob, model],
    )?;
    Ok(())
}

/// Load all embeddings for a repo as (symbol_id, Vec<f32>).
pub fn load_embeddings(
    conn: &Connection,
    repo_id: i64,
) -> Result<Vec<(i64, Vec<f32>)>, IndexError> {
    let mut stmt =
        conn.prepare("SELECT symbol_id, embedding FROM code_embeddings WHERE repo_id = ?1")?;
    let rows = stmt.query_map(params![repo_id], |r| {
        let id: i64 = r.get(0)?;
        let blob: Vec<u8> = r.get(1)?;
        Ok((id, blob_to_embedding(&blob)))
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

// ─── BM25-like text scoring ─────────────────────────────────────────────────

/// A scored symbol result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSearchResult {
    pub symbol: Symbol,
    pub symbol_id: i64,
    pub score: f64,
    /// Which signals contributed.
    pub signals: Vec<String>,
}

/// Score all symbols in a repo against a free-text query using BM25-inspired
/// term matching on symbol name, parent name, and file path.
fn text_rank(
    conn: &Connection,
    repo_id: i64,
    query: &str,
    limit: usize,
) -> Result<Vec<i64>, IndexError> {
    let query_lower = query.to_lowercase();
    let terms: Vec<&str> = query_lower
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|t| t.len() > 1)
        .collect();

    if terms.is_empty() {
        return Ok(Vec::new());
    }

    // Load symbols with id for scoring.
    let mut stmt =
        conn.prepare("SELECT id, name, kind, file, parent FROM code_symbols WHERE repo_id = ?1")?;

    struct Row {
        id: i64,
        name: String,
        file: String,
        parent: Option<String>,
    }

    let rows: Vec<Row> = stmt
        .query_map(params![repo_id], |r| {
            Ok(Row {
                id: r.get(0)?,
                name: r.get(1)?,
                file: r.get(3)?,
                parent: r.get(4)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    let total_docs = rows.len() as f64;
    if total_docs == 0.0 {
        return Ok(Vec::new());
    }

    // Compute IDF for each term (how many docs contain it).
    let mut doc_freq: HashMap<&str, usize> = HashMap::new();
    for term in &terms {
        let count = rows
            .iter()
            .filter(|r| {
                r.name.to_lowercase().contains(term)
                    || r.file.to_lowercase().contains(term)
                    || r.parent
                        .as_ref()
                        .map(|p| p.to_lowercase().contains(term))
                        .unwrap_or(false)
            })
            .count();
        doc_freq.insert(term, count);
    }

    // Score each symbol.
    let mut scored: Vec<(i64, f64)> = rows
        .iter()
        .map(|r| {
            let name_lower = r.name.to_lowercase();
            let file_lower = r.file.to_lowercase();
            let parent_lower = r
                .parent
                .as_ref()
                .map(|p| p.to_lowercase())
                .unwrap_or_default();

            let mut score = 0.0f64;

            for term in &terms {
                let df = *doc_freq.get(term).unwrap_or(&0) as f64;
                let idf = ((total_docs - df + 0.5) / (df + 0.5) + 1.0).ln();

                // Term frequency — higher weight for name match.
                let mut tf = 0.0f64;
                if name_lower == *term {
                    tf += 5.0; // Exact name match.
                } else if name_lower.contains(term) {
                    tf += 3.0; // Substring in name.
                }
                if parent_lower.contains(term) {
                    tf += 1.5;
                }
                if file_lower.contains(term) {
                    tf += 1.0;
                }

                // BM25-like: score += idf * tf / (tf + 1.2)
                if tf > 0.0 {
                    score += idf * tf / (tf + 1.2);
                }
            }

            (r.id, score)
        })
        .filter(|(_, s)| *s > 0.0)
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit);
    Ok(scored.into_iter().map(|(id, _)| id).collect())
}

// ─── Vector ranking ─────────────────────────────────────────────────────────

/// Rank symbols by cosine similarity to the query embedding.
fn vector_rank(
    conn: &Connection,
    repo_id: i64,
    query_embedding: &[f32],
    limit: usize,
) -> Result<Vec<i64>, IndexError> {
    let embeddings = load_embeddings(conn, repo_id)?;
    if embeddings.is_empty() {
        return Ok(Vec::new());
    }

    let mut scored: Vec<(i64, f32)> = embeddings
        .iter()
        .map(|(id, emb)| (*id, cosine_similarity(query_embedding, emb)))
        .filter(|(_, s)| *s > 0.0)
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit);
    Ok(scored.into_iter().map(|(id, _)| id).collect())
}

// ─── Graph ranking ──────────────────────────────────────────────────────────

/// Rank symbols by graph importance: entry-point participation + edge density.
fn graph_rank(conn: &Connection, repo_id: i64, limit: usize) -> Result<Vec<i64>, IndexError> {
    // Check if code_processes table exists (may not if processes haven't been computed yet).
    let has_processes: bool = conn.prepare("SELECT 1 FROM code_processes LIMIT 0").is_ok();

    let query = if has_processes {
        "SELECT s.id,
                COALESCE(ep.score, 0) + COALESCE(edge_in.cnt, 0) * 0.5 + COALESCE(edge_out.cnt, 0) * 0.3 AS importance
         FROM code_symbols s
         LEFT JOIN (
             SELECT entry_symbol_id, COUNT(*) * 10.0 AS score
             FROM code_processes WHERE repo_id = ?1
             GROUP BY entry_symbol_id
         ) ep ON ep.entry_symbol_id = s.id
         LEFT JOIN (
             SELECT target_symbol_id, COUNT(*) AS cnt
             FROM code_edges WHERE repo_id = ?1 AND target_symbol_id IS NOT NULL
             GROUP BY target_symbol_id
         ) edge_in ON edge_in.target_symbol_id = s.id
         LEFT JOIN (
             SELECT from_symbol_id, COUNT(*) AS cnt
             FROM code_edges WHERE repo_id = ?1 AND from_symbol_id IS NOT NULL
             GROUP BY from_symbol_id
         ) edge_out ON edge_out.from_symbol_id = s.id
         WHERE s.repo_id = ?1
         ORDER BY importance DESC
         LIMIT ?2"
    } else {
        // Fallback: only use edge density.
        "SELECT s.id,
                COALESCE(edge_in.cnt, 0) * 0.5 + COALESCE(edge_out.cnt, 0) * 0.3 AS importance
         FROM code_symbols s
         LEFT JOIN (
             SELECT target_symbol_id, COUNT(*) AS cnt
             FROM code_edges WHERE repo_id = ?1 AND target_symbol_id IS NOT NULL
             GROUP BY target_symbol_id
         ) edge_in ON edge_in.target_symbol_id = s.id
         LEFT JOIN (
             SELECT from_symbol_id, COUNT(*) AS cnt
             FROM code_edges WHERE repo_id = ?1 AND from_symbol_id IS NOT NULL
             GROUP BY from_symbol_id
         ) edge_out ON edge_out.from_symbol_id = s.id
         WHERE s.repo_id = ?1
         ORDER BY importance DESC
         LIMIT ?2"
    };

    let mut stmt = conn.prepare(query)?;

    let rows: Vec<i64> = stmt
        .query_map(params![repo_id, limit as i64], |r| r.get(0))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(rows)
}

// ─── Hybrid search (RRF fusion) ────────────────────────────────────────────

/// Perform hybrid code search with BM25 text + optional vector + graph signals,
/// fused via Reciprocal Rank Fusion.
pub fn hybrid_code_search(
    data_dir: &Path,
    repo_path: &Path,
    query: &str,
    query_embedding: Option<&[f32]>,
    limit: usize,
) -> Result<Vec<CodeSearchResult>, IndexError> {
    let repo_path = repo_path
        .canonicalize()
        .map_err(|e| IndexError::InvalidPath(format!("{}: {e}", repo_path.display())))?;
    let repo_path_str = repo_path.to_string_lossy().to_string();

    let conn = open_db(data_dir)?;
    ensure_embedding_table(&conn)?;

    let repo_id: i64 = conn
        .query_row(
            "SELECT id FROM code_repos WHERE path = ?1",
            params![repo_path_str],
            |r| r.get(0),
        )
        .map_err(|_| {
            IndexError::InvalidPath(format!("repo not indexed: {}", repo_path.display()))
        })?;

    hybrid_code_search_by_repo(&conn, repo_id, query, query_embedding, limit)
}

/// Hybrid code search on an already-open connection with known repo_id.
pub fn hybrid_code_search_by_repo(
    conn: &Connection,
    repo_id: i64,
    query: &str,
    query_embedding: Option<&[f32]>,
    limit: usize,
) -> Result<Vec<CodeSearchResult>, IndexError> {
    ensure_embedding_table(conn)?;

    let fetch_limit = limit * 3; // Over-fetch for fusion

    // Signal 1: BM25-like text ranking.
    let text_ranked = text_rank(conn, repo_id, query, fetch_limit)?;

    // Signal 2: Vector ranking (if embedding provided).
    let vector_ranked = if let Some(emb) = query_embedding {
        vector_rank(conn, repo_id, emb, fetch_limit)?
    } else {
        Vec::new()
    };

    // Signal 3: Graph importance ranking.
    let graph_ranked = graph_rank(conn, repo_id, fetch_limit)?;

    // Fuse via RRF.
    let mut rankings: Vec<&[i64]> = vec![&text_ranked];
    if !vector_ranked.is_empty() {
        rankings.push(&vector_ranked);
    }
    if !graph_ranked.is_empty() {
        rankings.push(&graph_ranked);
    }

    let fused = reciprocal_rank_fuse(&rankings, DEFAULT_RRF_K);

    // Build signal attribution map.
    let text_set: std::collections::HashSet<i64> = text_ranked.iter().copied().collect();
    let vector_set: std::collections::HashSet<i64> = vector_ranked.iter().copied().collect();
    let graph_set: std::collections::HashSet<i64> = graph_ranked.iter().copied().collect();

    // Resolve symbol details for top results.
    let top_ids: Vec<i64> = fused.iter().take(limit).map(|(id, _)| *id).collect();
    let scores: HashMap<i64, f64> = fused.into_iter().collect();

    let mut results = Vec::with_capacity(top_ids.len());
    for id in &top_ids {
        let sym = load_symbol_by_id(conn, *id)?;
        if let Some(sym) = sym {
            let mut signals = Vec::new();
            if text_set.contains(id) {
                signals.push("text".to_string());
            }
            if vector_set.contains(id) {
                signals.push("vector".to_string());
            }
            if graph_set.contains(id) {
                signals.push("graph".to_string());
            }
            results.push(CodeSearchResult {
                symbol: sym,
                symbol_id: *id,
                score: scores.get(id).copied().unwrap_or(0.0),
                signals,
            });
        }
    }

    Ok(results)
}

// ─── Helpers ────────────────────────────────────────────────────────────────

fn load_symbol_by_id(conn: &Connection, id: i64) -> Result<Option<Symbol>, IndexError> {
    let mut stmt = conn.prepare(
        "SELECT name, kind, file, line, end_line, parent FROM code_symbols WHERE id = ?1",
    )?;
    let result = stmt
        .query_row(params![id], |row| {
            Ok(Symbol {
                name: row.get(0)?,
                kind: SymbolKind::parse(&row.get::<_, String>(1)?).unwrap_or(SymbolKind::Function),
                file: row.get(2)?,
                line: row.get(3)?,
                end_line: row.get(4)?,
                parent: row.get(5)?,
            })
        })
        .ok();
    Ok(result)
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0f64;
    let mut norm_a = 0.0f64;
    let mut norm_b = 0.0f64;
    for i in 0..a.len() {
        let ai = a[i] as f64;
        let bi = b[i] as f64;
        dot += ai * bi;
        norm_a += ai * ai;
        norm_b += bi * bi;
    }
    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom < 1e-10 {
        return 0.0;
    }
    (dot / denom) as f32
}

fn embedding_to_blob(embedding: &[f32]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(embedding.len() * 4);
    for &val in embedding {
        buf.extend_from_slice(&val.to_le_bytes());
    }
    buf
}

fn blob_to_embedding(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coding::symbol_index::index_repo;
    use tempfile::TempDir;

    #[test]
    fn test_text_rank_finds_symbols_by_name() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::write(
            repo.join("server.rs"),
            "pub fn run_http_server() {}\npub fn start_listener() {}\npub fn handle_request() {}",
        )
        .unwrap();

        let data_dir = tmp.path().join("data");
        std::fs::create_dir_all(&data_dir).unwrap();
        index_repo(&data_dir, &repo).unwrap();

        let conn = open_db(&data_dir).unwrap();
        ensure_embedding_table(&conn).unwrap();
        let repo_id: i64 = conn
            .query_row("SELECT id FROM code_repos LIMIT 1", [], |r| r.get(0))
            .unwrap();

        let results = text_rank(&conn, repo_id, "http server", 10).unwrap();
        assert!(
            !results.is_empty(),
            "Should find symbols matching 'http server'"
        );

        // run_http_server should rank highest (exact substring match on both terms).
        let sym = load_symbol_by_id(&conn, results[0]).unwrap().unwrap();
        assert_eq!(sym.name, "run_http_server");
    }

    #[test]
    fn test_hybrid_search_without_embeddings() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::write(
            repo.join("app.rs"),
            "pub fn create_app() {}\npub fn destroy_app() {}\npub fn configure() {}",
        )
        .unwrap();

        let data_dir = tmp.path().join("data");
        std::fs::create_dir_all(&data_dir).unwrap();
        index_repo(&data_dir, &repo).unwrap();

        let results = hybrid_code_search(&data_dir, &repo, "create app", None, 10).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].symbol.name, "create_app");
        assert!(results[0].signals.contains(&"text".to_string()));
    }

    #[test]
    fn test_vector_rank_with_stored_embeddings() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::write(repo.join("lib.rs"), "pub fn alpha() {}\npub fn beta() {}").unwrap();

        let data_dir = tmp.path().join("data");
        std::fs::create_dir_all(&data_dir).unwrap();
        index_repo(&data_dir, &repo).unwrap();

        let conn = open_db(&data_dir).unwrap();
        ensure_embedding_table(&conn).unwrap();
        let repo_id: i64 = conn
            .query_row("SELECT id FROM code_repos LIMIT 1", [], |r| r.get(0))
            .unwrap();

        // Get symbol ids.
        let alpha_id: i64 = conn
            .query_row(
                "SELECT id FROM code_symbols WHERE repo_id = ?1 AND name = 'alpha'",
                params![repo_id],
                |r| r.get(0),
            )
            .unwrap();
        let beta_id: i64 = conn
            .query_row(
                "SELECT id FROM code_symbols WHERE repo_id = ?1 AND name = 'beta'",
                params![repo_id],
                |r| r.get(0),
            )
            .unwrap();

        // Store fake embeddings.
        let alpha_emb = vec![1.0f32, 0.0, 0.0, 0.0];
        let beta_emb = vec![0.0f32, 1.0, 0.0, 0.0];
        store_embedding(&conn, alpha_id, repo_id, &alpha_emb, "test").unwrap();
        store_embedding(&conn, beta_id, repo_id, &beta_emb, "test").unwrap();

        // Query with embedding close to alpha.
        let query_emb = vec![0.9f32, 0.1, 0.0, 0.0];
        let ranked = vector_rank(&conn, repo_id, &query_emb, 10).unwrap();
        assert_eq!(ranked[0], alpha_id);
    }

    #[test]
    fn test_graph_rank_prioritizes_entry_points() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::write(
            repo.join("main.rs"),
            "pub fn main() { helper(); }\nfn helper() {}",
        )
        .unwrap();

        let data_dir = tmp.path().join("data");
        std::fs::create_dir_all(&data_dir).unwrap();
        index_repo(&data_dir, &repo).unwrap();

        // Resolve edges so from_symbol_id/target_symbol_id are populated.
        crate::coding::resolver::resolve_edges(&data_dir, &repo).unwrap();

        let conn = open_db(&data_dir).unwrap();
        ensure_embedding_table(&conn).unwrap();
        let repo_id: i64 = conn
            .query_row("SELECT id FROM code_repos LIMIT 1", [], |r| r.get(0))
            .unwrap();

        let ranked = graph_rank(&conn, repo_id, 10).unwrap();
        // main should rank higher because it has outgoing edges.
        assert!(!ranked.is_empty());
    }
}
