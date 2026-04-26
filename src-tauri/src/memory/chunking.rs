//! Semantic chunking pipeline for large documents (Chunk 16.11).
//!
//! Replaces the naive word-count splitter in `commands::ingest` with
//! semantic-boundary-aware chunking via the `text-splitter` crate.
//!
//! Features:
//! - **Markdown-aware**: uses `MarkdownSplitter` for `.md` content,
//!   splitting at heading / paragraph / sentence boundaries.
//! - **Plain text**: uses `TextSplitter` with Unicode sentence +
//!   paragraph boundary detection.
//! - **Deduplication**: SHA-256 hash per chunk; callers can skip
//!   chunks whose hash already exists in the store.
//! - **Metadata propagation**: extracts the nearest Markdown heading
//!   preceding each chunk so callers can tag memories by section.
//!
//! Maps to `docs/brain-advanced-design.md` §16 Phase 4.

use sha2::{Digest, Sha256};
use text_splitter::{MarkdownSplitter, TextSplitter};

/// Default chunk capacity in characters.  ≈256 tokens at ~4 chars/token.
pub const DEFAULT_CHUNK_CHARS: usize = 1024;

/// Minimum chunk capacity (characters).  Prevents degenerate micro-chunks.
pub const MIN_CHUNK_CHARS: usize = 64;

/// A single chunk produced by the pipeline.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Chunk {
    /// Zero-based chunk index within the document.
    pub index: usize,
    /// The chunk text (trimmed).
    pub text: String,
    /// SHA-256 hex digest of the trimmed text — for dedup.
    pub hash: String,
    /// The nearest Markdown heading preceding this chunk (if any).
    /// Only populated when `split_markdown` is used.
    pub heading: Option<String>,
}

/// Split a Markdown document into semantic chunks.
///
/// Uses heading / paragraph / sentence boundaries.  Each chunk is
/// annotated with the nearest preceding heading (if any).
pub fn split_markdown(text: &str, max_chars: usize) -> Vec<Chunk> {
    let cap = max_chars.max(MIN_CHUNK_CHARS);
    let splitter = MarkdownSplitter::new(cap);
    let raw_chunks: Vec<&str> = splitter.chunks(text).collect();

    // Build a heading map: for each byte-offset in the original text,
    // track the most recent heading.  We'll look up each chunk's
    // start position to find its heading.
    let headings = extract_headings(text);

    raw_chunks
        .into_iter()
        .enumerate()
        .filter_map(|(i, chunk_text)| {
            let trimmed = chunk_text.trim();
            if trimmed.is_empty() {
                return None;
            }
            // Find the chunk's start offset in the original text.
            let offset = text
                .find(trimmed)
                .unwrap_or(0);
            let heading = heading_at_offset(&headings, offset);

            Some(Chunk {
                index: i,
                text: trimmed.to_string(),
                hash: sha256_hex(trimmed),
                heading,
            })
        })
        .collect()
}

/// Split plain text into semantic chunks (sentence / paragraph boundaries).
pub fn split_text(text: &str, max_chars: usize) -> Vec<Chunk> {
    let cap = max_chars.max(MIN_CHUNK_CHARS);
    let splitter = TextSplitter::new(cap);
    let raw_chunks: Vec<&str> = splitter.chunks(text).collect();

    raw_chunks
        .into_iter()
        .enumerate()
        .filter_map(|(i, chunk_text)| {
            let trimmed = chunk_text.trim();
            if trimmed.is_empty() {
                return None;
            }
            Some(Chunk {
                index: i,
                text: trimmed.to_string(),
                hash: sha256_hex(trimmed),
                heading: None,
            })
        })
        .collect()
}

/// Deduplicate chunks by hash, keeping the first occurrence.
pub fn dedup_chunks(chunks: Vec<Chunk>) -> Vec<Chunk> {
    let mut seen = std::collections::HashSet::new();
    chunks
        .into_iter()
        .filter(|c| seen.insert(c.hash.clone()))
        .collect()
}

/// SHA-256 hex digest of a string.
fn sha256_hex(s: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    hex::encode(hasher.finalize())
}

/// (byte_offset, heading_text) pairs, in document order.
struct HeadingEntry {
    offset: usize,
    text: String,
}

/// Extract all ATX headings (`# …` through `###### …`) with their byte
/// offsets in the source text.
fn extract_headings(text: &str) -> Vec<HeadingEntry> {
    let mut headings = Vec::new();
    let mut offset = 0;
    for line in text.split('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            // Count leading '#'s (up to 6).
            let hashes = trimmed.chars().take_while(|&c| c == '#').count();
            if hashes <= 6 {
                let heading_text = trimmed[hashes..].trim().trim_end_matches('#').trim();
                if !heading_text.is_empty() {
                    headings.push(HeadingEntry {
                        offset,
                        text: heading_text.to_string(),
                    });
                }
            }
        }
        offset += line.len() + 1; // +1 for the '\n'
    }
    headings
}

/// Find the most recent heading at or before `offset`.
fn heading_at_offset(headings: &[HeadingEntry], offset: usize) -> Option<String> {
    headings
        .iter()
        .rev()
        .find(|h| h.offset <= offset)
        .map(|h| h.text.clone())
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_text_single_chunk() {
        let chunks = split_text("Hello world.", 1024);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "Hello world.");
        assert_eq!(chunks[0].index, 0);
        assert!(!chunks[0].hash.is_empty());
    }

    #[test]
    fn long_text_produces_multiple_chunks() {
        // ~200 sentences → should split into multiple chunks at 256 chars.
        let text = (0..200)
            .map(|i| format!("Sentence number {i} with some extra words to pad it out."))
            .collect::<Vec<_>>()
            .join(" ");
        let chunks = split_text(&text, 256);
        assert!(
            chunks.len() > 1,
            "expected >1 chunk, got {}",
            chunks.len()
        );
        // Every chunk should be within the capacity (roughly).
        for c in &chunks {
            assert!(c.text.len() <= 300, "chunk too large: {} chars", c.text.len());
        }
    }

    #[test]
    fn markdown_heading_extraction() {
        let md = "# Introduction\n\nSome intro text.\n\n## Details\n\nMore details here.";
        let chunks = split_markdown(md, 2000);
        // Should get the whole thing as one chunk since it's small.
        assert!(!chunks.is_empty());
        // The heading should be captured.
        let first = &chunks[0];
        assert!(
            first.heading.is_some(),
            "expected a heading, got None"
        );
    }

    #[test]
    fn markdown_splits_at_heading_boundaries() {
        let sections: Vec<String> = (0..10)
            .map(|i| {
                let body = (0..20)
                    .map(|j| format!("Sentence {j} of section {i} with padding words."))
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("## Section {i}\n\n{body}")
            })
            .collect();
        let md = sections.join("\n\n");
        let chunks = split_markdown(&md, 256);
        assert!(
            chunks.len() > 1,
            "expected >1 markdown chunk, got {}",
            chunks.len()
        );
    }

    #[test]
    fn dedup_removes_duplicates() {
        let chunks = vec![
            Chunk { index: 0, text: "duplicate".into(), hash: sha256_hex("duplicate"), heading: None },
            Chunk { index: 1, text: "unique".into(), hash: sha256_hex("unique"), heading: None },
            Chunk { index: 2, text: "duplicate".into(), hash: sha256_hex("duplicate"), heading: None },
        ];
        let deduped = dedup_chunks(chunks);
        assert_eq!(deduped.len(), 2);
        assert_eq!(deduped[0].text, "duplicate");
        assert_eq!(deduped[1].text, "unique");
    }

    #[test]
    fn sha256_hex_deterministic() {
        let h1 = sha256_hex("test");
        let h2 = sha256_hex("test");
        assert_eq!(h1, h2);
        assert_ne!(sha256_hex("a"), sha256_hex("b"));
    }

    #[test]
    fn empty_text_produces_no_chunks() {
        assert!(split_text("", 1024).is_empty());
        assert!(split_markdown("", 1024).is_empty());
    }

    #[test]
    fn min_chunk_chars_enforced() {
        // Even with max_chars=1, we clamp to MIN_CHUNK_CHARS.
        let chunks = split_text("Hello, this is a test.", 1);
        assert!(!chunks.is_empty());
    }
}
