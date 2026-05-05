//! Deterministic fallback embeddings for the headless MCP runner.
//!
//! This is intentionally small and dependency-light: it hashes normalized token
//! unigrams + adjacent bigrams into a fixed-size vector and L2-normalizes the
//! result. It is not a replacement for Ollama/cloud embeddings, but it gives
//! fresh `npm run mcp` sessions a stable vector signal with zero network and no
//! model download.

use sha2::{Digest, Sha256};

/// Dimensionality of the deterministic fallback vector.
pub const OFFLINE_EMBEDDING_DIMS: usize = 256;

/// Build a deterministic embedding for non-empty text.
pub fn embed_text(text: &str) -> Option<Vec<f32>> {
    let tokens = tokenize(text);
    if tokens.is_empty() {
        return None;
    }

    let mut vector = vec![0.0f32; OFFLINE_EMBEDDING_DIMS];
    for token in &tokens {
        add_feature(&mut vector, token, 1.0);
    }
    for pair in tokens.windows(2) {
        add_feature(&mut vector, &format!("{} {}", pair[0], pair[1]), 0.5);
    }

    let norm = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm <= f32::EPSILON {
        return None;
    }
    for value in &mut vector {
        *value /= norm;
    }
    Some(vector)
}

fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for ch in text.chars().flat_map(char::to_lowercase) {
        if ch.is_alphanumeric() {
            current.push(ch);
        } else if current.len() >= 2 {
            tokens.push(std::mem::take(&mut current));
        } else {
            current.clear();
        }
    }
    if current.len() >= 2 {
        tokens.push(current);
    }
    tokens
}

fn add_feature(vector: &mut [f32], feature: &str, weight: f32) {
    let digest = Sha256::digest(feature.as_bytes());
    let mut idx_bytes = [0u8; 8];
    idx_bytes.copy_from_slice(&digest[..8]);
    let idx = u64::from_le_bytes(idx_bytes) as usize % vector.len();
    let sign = if digest[8] & 1 == 0 { 1.0 } else { -1.0 };
    vector[idx] += sign * weight;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embed_text_is_deterministic_and_normalized() {
        let a = embed_text("MCP seed embeddings should work offline.").unwrap();
        let b = embed_text("MCP seed embeddings should work offline.").unwrap();
        assert_eq!(a, b);
        assert_eq!(a.len(), OFFLINE_EMBEDDING_DIMS);

        let norm = a.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.0001, "norm={norm}");
    }

    #[test]
    fn embed_text_rejects_empty_input() {
        assert!(embed_text(" \n\t .,! ").is_none());
    }

    #[test]
    fn related_texts_have_positive_similarity() {
        let a = embed_text("headless mcp deterministic embedder fallback").unwrap();
        let b = embed_text("mcp embedder fallback works without network").unwrap();
        let sim = crate::memory::cosine_similarity(&a, &b);
        assert!(sim > 0.0, "sim={sim}");
    }
}
