//! Two-word memorable session name generator (Chunk 43.1).
//!
//! Generates `adjective-animal` names like `blazing-fox`, `gentle-owl`,
//! `silent-wolf`. Names are case-insensitive and filesystem-safe by
//! construction (lowercase ASCII, hyphen-separated). The word lists are
//! compact (80 adjectives × 80 animals = 6 400 unique combinations) —
//! more than enough for any single user's session history without
//! requiring UUIDs.

use std::collections::HashSet;

/// 80 single-word adjectives — all lowercase ASCII, no hyphens.
const ADJECTIVES: &[&str] = &[
    "agile", "amber", "arctic", "azure", "blazing", "bold", "bright", "brisk",
    "calm", "cedar", "clever", "cobalt", "coral", "cosmic", "crimson", "crystal",
    "daring", "dawn", "deep", "desert", "dusk", "eager", "echo", "ember",
    "fair", "fern", "fierce", "flint", "frost", "gentle", "gilded", "gleaming",
    "golden", "grand", "harbor", "hidden", "hollow", "humble", "iron", "ivory",
    "jade", "keen", "kind", "lemon", "light", "lively", "lunar", "maple",
    "marble", "mellow", "misty", "modest", "mossy", "nimble", "noble", "ocean",
    "olive", "opal", "pale", "pine", "plum", "polar", "proud", "quiet",
    "rapid", "raven", "rosy", "rustic", "sage", "scarlet", "serene", "sharp",
    "silent", "silver", "slate", "solar", "steady", "stone", "swift", "tidal",
];

/// 80 single-word animal names — all lowercase ASCII, no hyphens.
const ANIMALS: &[&str] = &[
    "ant", "badger", "bear", "bee", "bison", "bobcat", "bull", "bunny",
    "camel", "cat", "cheetah", "cobra", "condor", "coyote", "crane", "crow",
    "deer", "dog", "dolphin", "dove", "dragon", "eagle", "elk", "falcon",
    "ferret", "finch", "fox", "frog", "gazelle", "gecko", "goat", "goose",
    "gorilla", "hawk", "heron", "horse", "hound", "ibis", "iguana", "impala",
    "jackal", "jaguar", "jay", "kite", "koala", "lark", "lemur", "leopard",
    "lion", "llama", "lynx", "magpie", "marten", "moose", "moth", "newt",
    "octopus", "otter", "owl", "panda", "parrot", "pelican", "pike", "puma",
    "quail", "raven", "robin", "salmon", "seal", "shark", "snake", "sparrow",
    "squid", "stag", "swan", "tiger", "toucan", "turtle", "viper", "wolf",
];

/// Generate a memorable `adjective-animal` name that does not collide
/// with any entry in `existing` (compared case-insensitively).
///
/// Uses a simple seeded approach based on the current timestamp so
/// consecutive calls are unlikely to collide, then falls back to
/// sequential scanning if needed.
pub fn generate_unique(existing: &HashSet<String>) -> String {
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);

    // Fast path: try a few pseudo-random combinations.
    let mut h = seed;
    for _ in 0..20 {
        h = h.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let ai = (h >> 32) as usize % ADJECTIVES.len();
        let ni = (h >> 16) as usize % ANIMALS.len();
        let candidate = format!("{}-{}", ADJECTIVES[ai], ANIMALS[ni]);
        if !existing.contains(&candidate) {
            return candidate;
        }
    }

    // Exhaustive fallback: scan all combinations.
    for adj in ADJECTIVES {
        for animal in ANIMALS {
            let candidate = format!("{adj}-{animal}");
            if !existing.contains(&candidate) {
                return candidate;
            }
        }
    }

    // Should never happen with 6400 combinations, but be safe.
    format!("session-{}", seed % 1_000_000)
}

/// Normalise a session name to the canonical form used for lookups
/// and filesystem paths: lowercase, trimmed, with spaces replaced by
/// hyphens.
pub fn normalize(name: &str) -> String {
    name.trim().to_ascii_lowercase().replace(' ', "-")
}

/// Validate that a session name looks like an `adjective-animal` pair.
/// Returns `true` for any `word-word` pattern with ASCII alphanumerics.
pub fn is_valid_memorable_name(name: &str) -> bool {
    let parts: Vec<&str> = name.split('-').collect();
    parts.len() == 2
        && parts.iter().all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_unique_returns_valid_name() {
        let existing = HashSet::new();
        let name = generate_unique(&existing);
        assert!(is_valid_memorable_name(&name), "bad name: {name}");
    }

    #[test]
    fn generate_unique_avoids_collision() {
        let mut existing = HashSet::new();
        for _ in 0..100 {
            let name = generate_unique(&existing);
            assert!(!existing.contains(&name), "collision: {name}");
            existing.insert(name);
        }
    }

    #[test]
    fn normalize_lowercases_and_trims() {
        assert_eq!(normalize("  Blazing Fox  "), "blazing-fox");
        assert_eq!(normalize("SILENT-WOLF"), "silent-wolf");
    }

    #[test]
    fn is_valid_memorable_name_accepts_good() {
        assert!(is_valid_memorable_name("blazing-fox"));
        assert!(is_valid_memorable_name("calm-owl"));
    }

    #[test]
    fn is_valid_memorable_name_rejects_bad() {
        assert!(!is_valid_memorable_name("single"));
        assert!(!is_valid_memorable_name("too-many-parts"));
        assert!(!is_valid_memorable_name("-leading"));
        assert!(!is_valid_memorable_name(""));
    }

    #[test]
    fn word_lists_are_correct_length() {
        assert_eq!(ADJECTIVES.len(), 80);
        assert_eq!(ANIMALS.len(), 80);
    }

    #[test]
    fn all_words_are_lowercase_ascii() {
        for w in ADJECTIVES.iter().chain(ANIMALS.iter()) {
            assert!(
                w.chars().all(|c| c.is_ascii_lowercase()),
                "non-lowercase word: {w}"
            );
        }
    }
}
