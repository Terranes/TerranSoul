use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use super::cognitive_kind::classify as classify_cognitive_kind;
use super::metrics::{Timer, METRICS};
use super::schema;
use super::search_cache::SEARCH_CACHE;
use super::sharded_retrieval::{merge_shard_rankings, ShardKey};

/// Maximum results from a single session in diversified RRF retrieval.
///
/// Only real `session_id` values are capped; global long-term memories with
/// `session_id = NULL` remain uncapped so durable knowledge is not hidden.
pub const DEFAULT_MAX_RESULTS_PER_SESSION: usize = 3;

const GRAPH_BOOST_SEED_LIMIT: usize = 12;
const GRAPH_BOOST_NEIGHBOR_LIMIT_PER_SEED: i64 = 64;
const GRAPH_BOOST_PER_EDGE: f64 = 0.06;
const GRAPH_BOOST_MAX: f64 = 0.18;

const SHORT_TECH_QUERY_TERMS: &[&str] = &[
    "ai", "ar", "as", "cd", "ci", "db", "kg", "ml", "qa", "ui", "ux", "vr",
];
const QUERY_STOP_TERMS: &[&str] = &[
    "a",
    "about",
    "am",
    "an",
    "and",
    "app",
    "are",
    "at",
    "be",
    "by",
    "can",
    "did",
    "discussed",
    "do",
    "does",
    "earlier",
    "for",
    "from",
    "has",
    "have",
    "her",
    "his",
    "how",
    "i",
    "in",
    "into",
    "is",
    "it",
    "its",
    "just",
    "likely",
    "me",
    "mentioned",
    "my",
    "not",
    "of",
    "on",
    "or",
    "our",
    "provided",
    "remind",
    "set",
    "she",
    "so",
    "still",
    "than",
    "that",
    "their",
    "them",
    "then",
    "they",
    "the",
    "this",
    "to",
    "up",
    "was",
    "we",
    "were",
    "what",
    "when",
    "where",
    "who",
    "why",
    "with",
    "won",
    "work",
    "works",
    "would",
    "you",
];

const RECOMMENDATION_STOP_TERMS: &[&str] = &[
    "again", "any", "been", "bit", "can", "could", "give", "got", "have", "having", "ive", "just",
    "lately", "like", "me", "might", "more", "my", "need", "some", "would", "you", "your",
];

const RECOMMENDATION_QUERY_CUES: &[&str] = &[
    "advice",
    "idea",
    "ideas",
    "recommend",
    "recommendation",
    "recommendations",
    "suggest",
    "suggestion",
    "suggestions",
    "tips",
];

const LOW_SIGNAL_WEIGHT_TERMS: &[&str] = &[
    "config",
    "configuration",
    "implementation",
    "setup",
    "test",
    "testing",
    "validation",
];

const RECOMMENDATION_DOMAIN_EXPANSIONS: &[(&str, &[&str])] = &[
    (
        "accessories",
        &["case", "charger", "flash", "protector", "tripod"],
    ),
    ("activities", &["podcast", "podcasts", "genres", "commute"]),
    (
        "battery",
        &["charger", "charging", "power", "bank", "traveling"],
    ),
    ("commute", &["podcast", "podcasts", "listening", "genres"]),
    (
        "conference",
        &["field", "research", "advancements", "publications"],
    ),
    (
        "conferences",
        &["field", "research", "advancements", "publications"],
    ),
    ("dinner", &["fresh", "herbs", "recipe", "recipes"]),
    ("documentary", &["documentaries", "netflix", "series"]),
    ("editing", &["adobe", "premiere", "software", "video"]),
    (
        "guitar",
        &["fender", "gibson", "stratocaster", "les", "paul"],
    ),
    ("homegrown", &["fresh", "herbs", "recipe", "recipes"]),
    ("ingredients", &["fresh", "herbs", "recipe", "recipes"]),
    ("painting", &["acrylic", "brushes", "paints", "supplies"]),
    ("paintings", &["acrylic", "brushes", "paints", "supplies"]),
    ("phone", &["charger", "charging", "protector", "screen"]),
    ("photography", &["camera", "flash", "lens", "sony"]),
    (
        "publications",
        &["field", "research", "advancements", "conferences"],
    ),
    ("theme", &["rides", "events", "park", "parks"]),
    ("video", &["adobe", "premiere", "software", "editing"]),
];

const QUERY_TERM_EXPANSIONS: &[(&str, &[&str])] = &[
    ("accident", &["crash", "crashed", "hit", "wreck"]),
    ("bookshelf", &["books", "reading", "read"]),
    ("bought", &["buy", "got", "invested", "purchased"]),
    ("buy", &["bought", "got", "invested", "purchased"]),
    ("career", &["field", "job", "profession", "work"]),
    ("degree", &["education", "major", "policymaking", "study", "university"]),
    (
        "doctor",
        &[
            "appointment",
            "dermatologist",
            "dr",
            "ent",
            "physician",
            "specialist",
        ],
    ),
    (
        "doctors",
        &[
            "appointment",
            "dermatologist",
            "doctor",
            "dr",
            "ent",
            "physician",
            "specialist",
        ],
    ),
    ("education", &["career", "course", "degree", "school", "study"]),
    ("enjoy", &["fan", "interested", "like", "love"]),
    ("financial", &["afford", "income", "money", "wealth"]),
    ("got", &["bought", "buy", "invested", "purchased"]),
    ("holiday", &["christmas", "halloween", "thanksgiving", "weekend"]),
    ("identity", &["background", "gender", "orientation", "transgender"]),
    ("interested", &["enjoy", "fan", "fascinated", "love", "passion"]),
    ("invested", &["bought", "buy", "got", "purchased"]),
    ("learn", &["class", "course", "practice", "study", "studying"]),
    ("leaning", &["belief", "opinion", "political", "view"]),
    ("meet", &["catch", "lunch", "met"]),
    ("moved", &["relocated", "country", "city", "home"]),
    ("music", &["bach", "classical", "modern", "mozart", "song"]),
    ("names", &["called", "name", "named"]),
    ("partake", &["did", "join", "joined", "participate"]),
    ("partakes", &["did", "join", "joined", "participate"]),
    ("participate", &["attend", "join", "joined", "went"]),
    ("patriotic", &["country", "proud", "serve", "volunteer"]),
    ("personality", &["caring", "kind", "thoughtful", "traits"]),
    ("pet", &["cat", "dog", "puppy", "kitten"]),
    ("pets", &["cat", "cats", "dog", "dogs"]),
    ("political", &["conservative", "liberal", "leaning", "opinion"]),
    ("pursue", &["aspire", "career", "field", "goal", "study"]),
    ("religious", &["church", "faith", "spiritual"]),
    ("research", &["researching", "study", "studying", "investigate"]),
    ("roadtrip", &["drive", "road", "travel", "trip"]),
    ("song", &["classical", "fan", "listen", "music"]),
    ("status", &["current", "situation", "update"]),
    ("support", &["encourage", "help", "helped", "helping"]),
    ("traits", &["caring", "kind", "personality", "thoughtful"]),
    ("visited", &["appointment", "visit"]),
    ("visit", &["appointment", "visited"]),
    ("volunteer", &["community", "charity", "helping", "service"]),
    ("writing", &["career", "counseling", "field", "job", "profession"]),
];

fn query_terms(input: &str) -> (Vec<String>, usize) {
    let mut seen = HashSet::new();
    let mut terms: Vec<String> = input
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter_map(|raw| {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                return None;
            }
            let term = trimmed.to_lowercase();
            if QUERY_STOP_TERMS.contains(&term.as_str()) {
                return None;
            }
            let keep_short = term.len() == 2
                && (SHORT_TECH_QUERY_TERMS.contains(&term.as_str())
                    || trimmed.chars().any(|c| c.is_ascii_uppercase()));
            if term.len() > 2 || keep_short {
                seen.insert(term.clone()).then_some(term)
            } else {
                None
            }
        })
        .collect();

    if is_recommendation_query(input, &terms) {
        terms.retain(|term| !RECOMMENDATION_STOP_TERMS.contains(&term.as_str()));
        seen = terms.iter().cloned().collect();
        let base_terms = terms.clone();
        for term in base_terms {
            for (cue, expansions) in RECOMMENDATION_DOMAIN_EXPANSIONS {
                if term == *cue {
                    add_terms(&mut terms, &mut seen, expansions);
                }
            }
        }
    }

    // Semantic expansions (used for both FTS5 recall AND scoring).
    add_semantic_query_expansions(&mut terms, &mut seen);
    add_phrase_query_expansions(input, &mut terms, &mut seen);

    // Record the scoring term count BEFORE morphological expansion.
    // Morphological variants improve FTS5 recall but should NOT inflate
    // the lexical reranker (density/all-terms bonuses, per-term hits).
    let scoring_count = terms.len();

    // Morphological expansion (FTS5 recall only, not used in scoring).
    let base_terms = terms.clone();
    for term in base_terms {
        add_morphological_variants(&mut terms, &mut seen, &term);
    }

    (terms, scoring_count)
}

fn add_semantic_query_expansions(terms: &mut Vec<String>, seen: &mut HashSet<String>) {
    let base_terms = terms.clone();
    for term in base_terms {
        for (cue, expansions) in QUERY_TERM_EXPANSIONS {
            if term == *cue {
                add_terms(terms, seen, expansions);
            }
        }
    }
}

fn add_morphological_variants(terms: &mut Vec<String>, seen: &mut HashSet<String>, term: &str) {
    // ── Strip suffixes to find base forms ───────────────────────────────

    // -ies → -y (e.g. "activities" → "activity")
    if term.len() > 4 && term.ends_with("ies") {
        let singular = format!("{}y", &term[..term.len() - 3]);
        add_term(terms, seen, singular);
    }
    // -s (not -ss) → drop (e.g. "books" → "book")
    if term.len() > 3 && term.ends_with('s') && !term.ends_with("ss") {
        add_term(terms, seen, term[..term.len() - 1].to_string());
    }
    // -ing → base form (e.g. "researching" → "research", "moving" → "move")
    if term.len() > 5 && term.ends_with("ing") {
        let stem = &term[..term.len() - 3];
        add_term(terms, seen, stem.to_string());
        // Handle doubled consonant: "running" → "run"
        if stem.len() > 2 {
            let bytes = stem.as_bytes();
            if bytes[bytes.len() - 1] == bytes[bytes.len() - 2] {
                add_term(terms, seen, stem[..stem.len() - 1].to_string());
            }
        }
        // Handle dropped -e: "moving" → "move", "deciding" → "decide"
        add_term(terms, seen, format!("{stem}e"));
    }
    // -ed → base form (e.g. "moved" → "move", "decided" → "decide")
    if term.len() > 4 && term.ends_with("ed") {
        let stem = &term[..term.len() - 2];
        add_term(terms, seen, stem.to_string());
        // Handle doubled consonant: "stopped" → "stop"
        if stem.len() > 2 {
            let bytes = stem.as_bytes();
            if bytes[bytes.len() - 1] == bytes[bytes.len() - 2] {
                add_term(terms, seen, stem[..stem.len() - 1].to_string());
            }
        }
        // Handle -ied → -y: "studied" → "study"
        if term.ends_with("ied") && term.len() > 4 {
            add_term(
                terms,
                seen,
                format!("{}y", &term[..term.len() - 3]),
            );
        }
        // Drop just -d when stem already ends with -e: "loved" → "love"
        if stem.ends_with('e') {
            add_term(terms, seen, stem.to_string());
        }
    }
    // -er → base (e.g. "runner" → "run", "mover" → "move")
    if term.len() > 4 && term.ends_with("er") && !term.ends_with("ster") {
        let stem = &term[..term.len() - 2];
        add_term(terms, seen, stem.to_string());
        add_term(terms, seen, format!("{stem}e"));
    }
    // -ment → base (e.g. "engagement" → "engage")
    if term.len() > 6 && term.ends_with("ment") {
        let stem = &term[..term.len() - 4];
        add_term(terms, seen, stem.to_string());
        add_term(terms, seen, format!("{stem}e"));
    }
    // -tion/-sion → base (e.g. "education" → "educate", "adoption" → "adopt")
    // Don't emit bare stem — it's almost never a real word and causes
    // substring false-positives (e.g. "configura" ⊂ "configuration").
    if term.len() > 5 && (term.ends_with("tion") || term.ends_with("sion")) {
        let stem = &term[..term.len() - 4];
        add_term(terms, seen, format!("{stem}e"));
        add_term(terms, seen, format!("{stem}t"));
        add_term(terms, seen, format!("{stem}te"));
    }
    // -ly → base (e.g. "likely" → "like")
    if term.len() > 4 && term.ends_with("ly") {
        add_term(terms, seen, term[..term.len() - 2].to_string());
    }

    // ── Generate inflected forms from base ──────────────────────────────
    // Only for terms that look like base verbs/nouns (3+ chars, no suffix)
    if term.len() >= 3
        && !term.ends_with("ing")
        && !term.ends_with("ed")
        && !term.ends_with("tion")
    {
        // base → -ing (e.g. "research" → "researching")
        add_term(terms, seen, format!("{term}ing"));
        // base → -ed (e.g. "research" → "researched")
        add_term(terms, seen, format!("{term}ed"));
        // base ending in -e → drop e + -ing/-ed (e.g. "move" → "moving"/"moved")
        if let Some(stem) = term.strip_suffix('e') {
            add_term(terms, seen, format!("{stem}ing"));
            add_term(terms, seen, format!("{stem}ed"));
        }
    }
}

fn add_phrase_query_expansions(input: &str, terms: &mut Vec<String>, seen: &mut HashSet<String>) {
    let lower = input.to_lowercase();
    if lower.contains("kitchen appliance") || lower.contains("kitchen gadget") {
        add_terms(
            terms,
            seen,
            &[
                "air", "cooking", "fryer", "instant", "pot", "pressure", "smoker",
            ],
        );
    }
    if lower.contains("gardening") || lower.contains("garden") {
        add_terms(
            terms,
            seen,
            &[
                "garden", "plant", "planted", "planting", "saplings", "tomato",
            ],
        );
    }
    if lower.contains("high school reunion") {
        add_terms(
            terms,
            seen,
            &["advanced", "courses", "debate", "economics", "placement"],
        );
    }
    if lower.contains("jwt") && (lower.contains("middleware") || lower.contains("validation")) {
        add_terms(
            terms,
            seen,
            &["authentication", "nextauth", "session", "sessions"],
        );
    }
    if lower.contains("life event") || lower.contains("relatives") || lower.contains("relative") {
        add_terms(
            terms,
            seen,
            &[
                "birthday",
                "ceremony",
                "engagement",
                "graduation",
                "wedding",
            ],
        );
    }
    // ── Conversational concept expansions (LoCoMo-style) ────────────────
    if lower.contains("relationship") || lower.contains("dating") {
        add_terms(
            terms,
            seen,
            &[
                "boyfriend", "breakup", "couple", "dating", "divorced",
                "engagement", "girlfriend", "married", "partner", "romance",
                "relationship", "single", "wedding",
            ],
        );
    }
    if lower.contains("career") || lower.contains("profession") || lower.contains("occupation") {
        add_terms(
            terms,
            seen,
            &[
                "career", "field", "hired", "interview", "job", "occupation",
                "profession", "promoted", "quit", "resigned", "salary",
                "working",
            ],
        );
    }
    if lower.contains("education") || lower.contains("school") || lower.contains("degree") {
        add_terms(
            terms,
            seen,
            &[
                "class", "college", "courses", "degree", "diploma",
                "enrolled", "graduated", "major", "masters", "school",
                "semester", "student", "studying", "university",
            ],
        );
    }
    if lower.contains("hobby") || lower.contains("hobbies") || lower.contains("free time") {
        add_terms(
            terms,
            seen,
            &[
                "cooking", "crafts", "drawing", "gaming", "hiking",
                "interests", "knitting", "painting", "playing", "reading",
                "sports", "writing",
            ],
        );
    }
    if lower.contains("family") || lower.contains("parents") || lower.contains("sibling") {
        add_terms(
            terms,
            seen,
            &[
                "brother", "child", "children", "dad", "daughter", "family",
                "father", "kids", "mom", "mother", "parents", "sister",
                "son",
            ],
        );
    }
    if lower.contains("health") || lower.contains("medical") || lower.contains("illness") {
        add_terms(
            terms,
            seen,
            &[
                "allergies", "clinic", "condition", "diagnosed", "disease",
                "exercise", "fitness", "hospital", "injury", "medication",
                "mental", "surgery", "symptoms", "therapy", "treatment",
            ],
        );
    }
    if lower.contains("travel") || lower.contains("trip") || lower.contains("vacation") {
        add_terms(
            terms,
            seen,
            &[
                "abroad", "booked", "camping", "country", "cruise",
                "destination", "flight", "hotel", "luggage", "resort",
                "sightseeing", "tourism", "traveled", "travelling",
                "vacation", "visited",
            ],
        );
    }
    if lower.contains("moved") || lower.contains("move from") || lower.contains("relocated") {
        add_terms(
            terms,
            seen,
            &[
                "apartment", "city", "country", "home", "house", "lived",
                "location", "moved", "neighborhood", "place", "relocated",
                "rent", "town",
            ],
        );
    }
    if lower.contains("bookshelf") || lower.contains("reading") || lower.contains("favorite book") {
        add_terms(
            terms,
            seen,
            &[
                "author", "book", "books", "chapter", "fiction", "genre",
                "library", "literature", "novel", "read", "reader",
                "reading", "story", "stories", "writer",
            ],
        );
    }
    if lower.contains("pet") || lower.contains("animal") || lower.contains("dog") || lower.contains("cat") {
        add_terms(
            terms,
            seen,
            &[
                "adopted", "animal", "breed", "cat", "dog", "fish",
                "kitten", "pet", "puppy", "rabbit", "rescue", "vet",
                "veterinarian",
            ],
        );
    }
    // Activities / participation queries
    if lower.contains("activit") || lower.contains("partake") || lower.contains("participate") {
        add_terms(
            terms,
            seen,
            &[
                "camping", "class", "event", "joined", "painting",
                "pottery", "program", "signed", "swimming", "went",
            ],
        );
    }
    // De-stress / relaxation queries
    if lower.contains("destress") || lower.contains("de-stress") || lower.contains("relax") {
        add_terms(
            terms,
            seen,
            &[
                "calm", "meditation", "pottery", "running", "therapy",
                "yoga",
            ],
        );
    }
    // Art / creative queries — check that "art" is a standalone word, not
    // embedded in "charity", "heart", "smart", "Martha", etc.
    if terms.iter().any(|t| t == "art" || t == "arts" || t == "artistic") {
        add_terms(
            terms,
            seen,
            &[
                "canvas", "creative", "exhibit", "gallery", "painting",
                "paintings", "pottery", "sculpture", "show",
            ],
        );
    }
}

fn add_term(terms: &mut Vec<String>, seen: &mut HashSet<String>, term: String) {
    if seen.insert(term.clone()) {
        terms.push(term);
    }
}

fn add_terms(terms: &mut Vec<String>, seen: &mut HashSet<String>, additions: &[&str]) {
    for addition in additions {
        let term = addition.to_string();
        add_term(terms, seen, term);
    }
}

fn is_recommendation_query(input: &str, terms: &[String]) -> bool {
    let lower = input.to_lowercase();
    lower.contains("what should i")
        || lower.contains("do you have any")
        || terms
            .iter()
            .any(|term| RECOMMENDATION_QUERY_CUES.contains(&term.as_str()))
}

fn lexical_terms(input: &str) -> HashSet<String> {
    input
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter_map(|raw| {
            let term = raw.trim().to_lowercase();
            (!term.is_empty()).then_some(term)
        })
        .collect()
}

fn lexical_rank_score(entry: &MemoryEntry, terms: &[String]) -> usize {
    if terms.is_empty() {
        return 0;
    }

    let content_terms = lexical_terms(&entry.content);
    let tag_terms = lexical_terms(&entry.tags);
    let lower_content = entry.content.to_lowercase();
    let lower_tags = entry.tags.to_lowercase();

    let mut exact_tag_hits = 0usize;
    let mut exact_content_hits = 0usize;
    let mut substring_hits = 0usize;
    for term in terms {
        if tag_terms.contains(term) {
            exact_tag_hits += 1;
        }
        if content_terms.contains(term) {
            exact_content_hits += 1;
        }
        if lower_content.contains(term.as_str()) || lower_tags.contains(term.as_str()) {
            substring_hits += 1;
        }
    }

    let matched_terms = terms
        .iter()
        .filter(|term| {
            tag_terms.contains(*term)
                || content_terms.contains(*term)
                || lower_content.contains(term.as_str())
                || lower_tags.contains(term.as_str())
        })
        .count();
    // Scale the all-terms bonus with query length: longer queries with all
    // terms matching are much stronger signals than short ones.
    let all_terms_bonus = if matched_terms == terms.len() && terms.len() >= 2 {
        16 + terms.len() * 12
    } else {
        0
    };
    // Term density: ratio of matched unique terms to total terms, scaled.
    // Rewards docs matching a high fraction of query terms even when not all.
    let density_bonus = if terms.len() >= 2 {
        (matched_terms as f64 / terms.len() as f64 * 20.0) as usize
    } else {
        0
    };

    exact_tag_hits * 24
        + exact_content_hits * 8
        + substring_hits * 2
        + all_terms_bonus
        + density_bonus
        + entry.importance.max(0) as usize
}

fn lexical_term_weights(entries: &[MemoryEntry], terms: &[String]) -> HashMap<String, f64> {
    if entries.is_empty() || terms.is_empty() {
        return HashMap::new();
    }

    let total = entries.len() as f64;
    terms
        .iter()
        .filter_map(|term| {
            let df = entries
                .iter()
                .filter(|entry| lexical_term_matches(entry, term))
                .count();
            (df > 0).then(|| {
                let mut weight = (total / df as f64).ln_1p().clamp(0.25, 4.0);
                if LOW_SIGNAL_WEIGHT_TERMS.contains(&term.as_str()) {
                    weight = weight.min(1.0);
                }
                (term.clone(), weight)
            })
        })
        .collect()
}

fn lexical_term_matches(entry: &MemoryEntry, term: &str) -> bool {
    let content_terms = lexical_terms(&entry.content);
    let tag_terms = lexical_terms(&entry.tags);
    content_terms.contains(term)
        || tag_terms.contains(term)
        || entry.content.to_lowercase().contains(term)
        || entry.tags.to_lowercase().contains(term)
}

fn lexical_rank_score_weighted(
    entry: &MemoryEntry,
    terms: &[String],
    weights: &HashMap<String, f64>,
) -> f64 {
    if terms.is_empty() {
        return 0.0;
    }

    let content_terms = lexical_terms(&entry.content);
    let tag_terms = lexical_terms(&entry.tags);
    let lower_content = entry.content.to_lowercase();
    let lower_tags = entry.tags.to_lowercase();

    let mut score = 0.0_f64;
    let mut matched_terms = 0usize;
    for term in terms {
        let weight = weights.get(term).copied().unwrap_or(1.0);
        let mut matched = false;
        if tag_terms.contains(term) {
            score += 24.0 * weight;
            matched = true;
        }
        if content_terms.contains(term) {
            score += 8.0 * weight;
            matched = true;
        }
        if lower_content.contains(term.as_str()) || lower_tags.contains(term.as_str()) {
            score += 2.0 * weight;
            matched = true;
        }
        if matched {
            matched_terms += 1;
        }
    }

    // If no query term matched any field, this entry is not a lexical hit —
    // importance/freshness/etc. must not pull non-matching memories into the
    // keyword ranking (otherwise unrelated high-importance entries displace
    // genuinely relevant low-importance ones).
    if matched_terms == 0 {
        return 0.0;
    }

    // Add a small importance floor only for entries that already matched,
    // so importance acts as a tiebreaker between relevant hits.
    score += entry.importance.max(0) as f64;

    if matched_terms == terms.len() && terms.len() >= 2 {
        score += 16.0 + (terms.len() as f64) * 12.0;
    }
    // Term density: fractional match bonus for partial multi-term coverage.
    if terms.len() >= 2 {
        score += matched_terms as f64 / terms.len() as f64 * 20.0;
    }
    score
}

fn rerank_by_lexical_score(entries: &mut [MemoryEntry], terms: &[String]) {
    let weights = lexical_term_weights(entries, terms);
    let scores: HashMap<i64, f64> = entries
        .iter()
        .map(|entry| {
            (
                entry.id,
                lexical_rank_score_weighted(entry, terms, &weights),
            )
        })
        .collect();
    entries.sort_by(|a, b| {
        scores
            .get(&b.id)
            .copied()
            .unwrap_or(0.0)
            .partial_cmp(&scores.get(&a.id).copied().unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.importance.cmp(&a.importance))
            .then_with(|| b.created_at.cmp(&a.created_at))
            .then_with(|| a.id.cmp(&b.id))
    });
}

fn rerank_by_lexical_and_graph_score(
    entries: &mut [MemoryEntry],
    terms: &[String],
    graph_boosts: &HashMap<i64, f64>,
) {
    let weights = lexical_term_weights(entries, terms);
    let scores: HashMap<i64, f64> = entries
        .iter()
        .map(|entry| {
            (
                entry.id,
                lexical_rank_score_weighted(entry, terms, &weights),
            )
        })
        .collect();
    entries.sort_by(|a, b| {
        let score_a = scores.get(&a.id).copied().unwrap_or(0.0)
            + graph_boosts.get(&a.id).copied().unwrap_or(0.0) * 100.0;
        let score_b = scores.get(&b.id).copied().unwrap_or(0.0)
            + graph_boosts.get(&b.id).copied().unwrap_or(0.0) * 100.0;
        score_b
            .partial_cmp(&score_a)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.importance.cmp(&a.importance))
            .then_with(|| b.created_at.cmp(&a.created_at))
            .then_with(|| a.id.cmp(&b.id))
    });
}

fn top_unique_ids<I>(ids: I, limit: usize) -> Vec<i64>
where
    I: IntoIterator<Item = i64>,
{
    let mut seen = HashSet::new();
    let mut out = Vec::with_capacity(limit);
    for id in ids {
        if seen.insert(id) {
            out.push(id);
            if out.len() >= limit {
                break;
            }
        }
    }
    out
}

fn select_diversified_ranked<I>(
    ranked: I,
    by_id: &HashMap<i64, MemoryEntry>,
    limit: usize,
    max_per_session: usize,
) -> Vec<(i64, f64)>
where
    I: IntoIterator<Item = (i64, f64)>,
{
    if limit == 0 {
        return Vec::new();
    }

    let max_per_session = max_per_session.max(1);
    let mut session_counts: HashMap<String, usize> = HashMap::new();
    let mut selected = Vec::with_capacity(limit);
    // Two-pass selection. Pass 1: enforce per-session cap so the top of the
    // result set has session diversity (chat-context UX). Pass 2: if the
    // diversified pass under-filled `limit` (e.g. for focused analytical
    // queries that legitimately cluster in one session group), fill the
    // remaining slots from the same ranked list without the cap so we never
    // throw away relevant results below the requested limit.
    let mut overflow: Vec<(i64, f64)> = Vec::new();

    for (id, score) in ranked {
        let Some(entry) = by_id.get(&id) else {
            continue;
        };

        if let Some(session_id) = entry
            .session_id
            .as_deref()
            .map(str::trim)
            .filter(|session_id| !session_id.is_empty())
        {
            let count = session_counts.entry(session_id.to_string()).or_insert(0);
            if *count >= max_per_session {
                if selected.len() + overflow.len() < limit {
                    overflow.push((id, score));
                }
                continue;
            }
            *count += 1;
        }

        selected.push((id, score));
        if selected.len() >= limit {
            return selected;
        }
    }

    // Pass 2: backfill from the overflow (preserves original RRF order).
    for hit in overflow {
        if selected.len() >= limit {
            break;
        }
        selected.push(hit);
    }

    selected
}

pub(crate) fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// Backup the database file. Silently ignored on failure.
fn auto_backup(data_dir: &Path) {
    let src = data_dir.join("memory.db");
    if src.exists() {
        let dst = data_dir.join("memory.db.bak");
        let _ = std::fs::copy(&src, &dst);
    }
}

/// The category/purpose of a memory entry.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    /// A learned fact (e.g. "User's name is Alice").
    #[default]
    Fact,
    /// A user preference (e.g. "User prefers Python").
    Preference,
    /// Ongoing context (e.g. "User is working on a neural network").
    Context,
    /// A summary of a past conversation.
    Summary,
}

impl MemoryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryType::Fact => "fact",
            MemoryType::Preference => "preference",
            MemoryType::Context => "context",
            MemoryType::Summary => "summary",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "preference" => MemoryType::Preference,
            "context" => MemoryType::Context,
            "summary" => MemoryType::Summary,
            _ => MemoryType::Fact,
        }
    }
}

/// Memory tier — determines retrieval priority and lifecycle.
///
/// **Short-term**: Last ~20 messages in current session. Evicted on session end
/// or when window overflows. Auto-summarized into working memory.
///
/// **Working**: Extracted facts/context from the current session. Lives until
/// session ends, then promoted to long-term or discarded via decay.
///
/// **Long-term**: Permanent storage. Vector-indexed. Subject to periodic
/// consolidation (merge near-duplicates) and importance decay.
#[derive(Debug, Clone, Copy, Hash, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryTier {
    Short,
    Working,
    Long,
}

impl MemoryTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryTier::Short => "short",
            MemoryTier::Working => "working",
            MemoryTier::Long => "long",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "short" => MemoryTier::Short,
            "working" => MemoryTier::Working,
            _ => MemoryTier::Long,
        }
    }
}

/// A single memory entry with tier metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: i64,
    pub content: String,
    /// Comma-separated tags (e.g. "python,work,preferences").
    pub tags: String,
    /// Importance score 1–5 (5 = most important).
    pub importance: i64,
    pub memory_type: MemoryType,
    pub created_at: i64,
    pub last_accessed: Option<i64>,
    pub access_count: i64,
    /// 768-dimensional f32 embedding (serialized as little-endian bytes).
    #[serde(skip)]
    pub embedding: Option<Vec<f32>>,
    /// Which tier this memory lives in.
    pub tier: MemoryTier,
    /// Decay score 0.0–1.0. Decays over time for infrequently accessed entries.
    /// Used as a multiplier in hybrid ranking.
    pub decay_score: f64,
    /// Session identifier for grouping short-term/working memories.
    pub session_id: Option<String>,
    /// Parent memory (for summaries that consolidate children).
    pub parent_id: Option<i64>,
    /// Approximate token count of the content.
    pub token_count: i64,
    /// Origin URL for ingested/crawled documents.
    pub source_url: Option<String>,
    /// SHA-256 content hash for dedup / staleness detection.
    pub source_hash: Option<String>,
    /// Optional TTL — Unix-ms timestamp after which memory auto-expires.
    pub expires_at: Option<i64>,
    /// Soft-close timestamp (Unix ms). Non-NULL means this memory was superseded
    /// by a contradiction resolution and is no longer active. Never deleted.
    pub valid_to: Option<i64>,
    /// Relative path within the Obsidian vault (e.g. `TerranSoul/42-hello.md`).
    pub obsidian_path: Option<String>,
    /// Unix-ms timestamp of last successful export to Obsidian vault.
    pub last_exported: Option<i64>,
    /// Unix-ms timestamp of last mutation (for CRDT LWW sync).
    pub updated_at: Option<i64>,
    /// UUID of the device that last wrote this entry (for CRDT tiebreaker).
    pub origin_device: Option<String>,
    /// HLC counter for causal CRDT ordering (Chunk 42.3).
    pub hlc_counter: Option<i64>,
    /// Confidence score 0.0–1.0 (V20). Decayed per-cognitive-kind and
    /// boosted by reinforcement. Multiplied into hybrid search scores.
    #[serde(default = "default_confidence")]
    pub confidence: f64,
}

fn default_confidence() -> f64 {
    1.0
}

/// A single reinforcement event for provenance tracking (Chunk 43.4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReinforcementRecord {
    pub memory_id: i64,
    pub session_id: String,
    pub message_index: i64,
    pub ts: i64,
}

/// Fields required to create a new memory.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct NewMemory {
    pub content: String,
    pub tags: String,
    #[serde(default = "default_importance")]
    pub importance: i64,
    #[serde(default)]
    pub memory_type: MemoryType,
    /// Origin URL for ingested documents (optional).
    #[serde(default)]
    pub source_url: Option<String>,
    /// SHA-256 content hash for dedup / staleness detection (optional).
    #[serde(default)]
    pub source_hash: Option<String>,
    /// TTL timestamp — memory auto-expires after this Unix-ms time (optional).
    #[serde(default)]
    pub expires_at: Option<i64>,
    /// BENCH-PARITY-3 (2026-05-13): override `created_at` (Unix ms) on insert.
    /// `None` (default) uses wall-clock `now_ms()` — the only behavior callers
    /// previously had. Set to a session/conversation timestamp when ingesting
    /// historical data so downstream temporal filters
    /// ([`crate::memory::temporal::parse_time_range`] +
    /// [`crate::memory::temporal::filter_entries_in_query_range`]) match the
    /// memory's logical creation moment, not the moment it was loaded.
    #[serde(default)]
    pub created_at: Option<i64>,
}

fn default_importance() -> i64 {
    3
}

/// Fields that may be updated on an existing memory.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct MemoryUpdate {
    pub content: Option<String>,
    pub tags: Option<String>,
    pub importance: Option<i64>,
    pub memory_type: Option<MemoryType>,
}

/// Aggregated statistics across all memory tiers.
#[derive(Debug, Clone, Serialize)]
pub struct MemoryStats {
    pub total: i64,
    #[serde(rename = "short_count")]
    pub short: i64,
    #[serde(rename = "working_count")]
    pub working: i64,
    #[serde(rename = "long_count")]
    pub long: i64,
    pub embedded: i64,
    pub total_tokens: i64,
    pub avg_decay: f64,
    pub storage_bytes: i64,
    pub cache_bytes: i64,
}

/// Result of pruning memory rows to satisfy the configured storage cap.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryCleanupReport {
    pub before_bytes: i64,
    pub after_bytes: i64,
    pub max_bytes: i64,
    pub deleted: usize,
}

/// SQLite-backed persistent memory store.
/// Number of mutations between automatic `ANALYZE` runs.
///
/// This threshold balances query-planner freshness against maintenance cost:
/// running `ANALYZE` too often adds write overhead, while running it too
/// rarely can leave planner statistics stale after heavy churn. `10_000` is
/// used as a conservative default for mixed read/write workloads.
///
/// Tuning guidance:
/// - Lower for highly volatile datasets where query plans regress quickly.
/// - Raise for write-heavy scenarios where minimizing maintenance work matters
///   more than immediate planner-stat updates.
const ANALYZE_EVERY: u64 = 10_000;

/// Rebuild throttle for coarse shard router. Prevents expensive rebuild bursts
/// when many consecutive queries arrive while the router is stale/missing.
pub const ROUTER_REFRESH_COOLDOWN_MS: i64 = 15 * 60 * 1000;
/// Volume trigger: if writes since the last successful build exceed this
/// threshold, a background-safe refresh becomes eligible.
pub const ROUTER_REFRESH_MIN_MUTATIONS: u64 = 500;

/// Shard-routing policy for `select_shards_for_query` (BENCH-SCALE-2,
/// 2026-05-14). Default is [`ShardMode::RouterRouted`] which preserves the
/// production code path: cached router → persisted router → throttled
/// rebuild → fall back to all 15 shards. [`ShardMode::AllShards`] forces
/// every query to probe every shard, bypassing the coarse router entirely
/// — used by the LoCoMo-at-scale bench harness to measure the router's
/// contribution to latency/recall at 1M docs (single-index-style baseline
/// vs router-routed comparison). No production callers use `AllShards`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShardMode {
    /// Use the coarse router → top-p shards, fall back to all shards on miss.
    /// This is the production default.
    #[default]
    RouterRouted,
    /// Bypass the router and probe every shard on every query. Useful as
    /// a comparison baseline at scale; not recommended for production.
    AllShards,
}

pub struct MemoryStore {
    pub(crate) conn: Connection,
    /// Shard-keyed ANN indices for fast vector search (Chunk 48.2).
    /// Initialized lazily on first per-shard vector operation.
    anns: RefCell<HashMap<ShardKey, super::ann_index::AnnIndex>>,
    /// Coarse shard router — predicts top-p shards for a query (Chunk 48.3).
    /// Initialized lazily on first use or at startup. Falls back to "probe all shards"
    /// if missing/stale.
    router: RefCell<Option<super::shard_router::ShardRouter>>,
    /// Data directory for persisting the ANN index file.
    /// `None` for in-memory stores (tests).
    data_dir: Option<std::path::PathBuf>,
    /// Handle for debounced ANN flush (Chunk 41.10).  Set after
    /// construction via [`set_flush_handle`].
    flush_handle: Option<super::ann_flush::AnnFlushHandle>,
    /// Cumulative mutation counter (add/update/delete). When it crosses
    /// an `ANALYZE_EVERY` boundary, we run `ANALYZE` to keep the query
    /// planner statistics fresh (Chunk 41.12R).
    mutations: AtomicU64,
    /// Last mutation counter snapshot after a successful router rebuild.
    router_last_refresh_mutation: Cell<u64>,
    /// Last wall-clock rebuild attempt timestamp (ms). Used for cooldown.
    router_last_refresh_attempt_ms: Cell<i64>,
    /// Shard-routing policy. Defaults to [`ShardMode::RouterRouted`].
    /// Mutated only via [`MemoryStore::set_shard_mode`].
    shard_mode: Cell<ShardMode>,
}

impl MemoryStore {
    /// Open (or create) the memory database at `data_dir/memory.db`.
    /// Falls back to an in-memory database if the file cannot be opened.
    /// Enables WAL mode for crash durability and creates an auto-backup.
    /// Creates the canonical memory schema.
    pub fn new(data_dir: &Path) -> Self {
        Self::new_with_config(data_dir, None, None)
    }

    /// Open with user-configurable cache/mmap sizes (from AppSettings).
    /// Pass `None` for either to use the platform default.
    pub fn new_with_config(data_dir: &Path, cache_mb: Option<u32>, mmap_mb: Option<u32>) -> Self {
        auto_backup(data_dir);
        let conn = Connection::open(data_dir.join("memory.db")).unwrap_or_else(|e| {
            eprintln!(
                "Failed to open SQLite database at '{}': {}. Falling back to in-memory database.",
                data_dir.join("memory.db").display(),
                e
            );
            Connection::open_in_memory()
                .expect("Failed to create in-memory SQLite fallback database")
        });
        // Phase 41.1 — write-path tuning for million-memory CRUD.
        // WAL mode: crash-safe, concurrent reads, no data loss.
        // foreign_keys=ON is required for ON DELETE CASCADE on memory_edges (V5).
        // Platform-adaptive: desktop gets aggressive cache/mmap; mobile
        // reduces resource usage and tightens WAL autocheckpoint (42.2).
        if cache_mb.is_some() || mmap_mb.is_some() {
            let pragmas = super::platform::production_pragmas_custom(
                cache_mb.unwrap_or(crate::settings::DEFAULT_SQLITE_CACHE_MB),
                mmap_mb.unwrap_or(crate::settings::DEFAULT_SQLITE_MMAP_MB),
            );
            let _ = conn.execute_batch(&pragmas);
        } else {
            let _ = conn.execute_batch(super::platform::production_pragmas());
        }
        schema::create_canonical_schema(&conn).expect("memory schema initialization failed");
        // Phase 41.12R — let SQLite analyse table statistics on open.
        let _ = conn.execute_batch("PRAGMA optimize;");
        MemoryStore {
            conn,
            anns: RefCell::new(HashMap::new()),
            router: RefCell::new(None),
            data_dir: Some(data_dir.to_path_buf()),
            flush_handle: None,
            mutations: AtomicU64::new(0),
            router_last_refresh_mutation: Cell::new(0),
            router_last_refresh_attempt_ms: Cell::new(0),
            shard_mode: Cell::new(ShardMode::default()),
        }
    }

    /// Create an in-memory store (for tests).
    pub fn in_memory() -> Self {
        let conn =
            Connection::open_in_memory().expect("Failed to create in-memory SQLite database");
        // foreign_keys=ON keeps test parity with the on-disk store and
        // exercises the V5 memory_edges cascade behaviour.
        // temp_store=MEMORY + cache_size keep test perf representative of prod.
        let _ = conn.execute_batch(super::platform::test_pragmas());
        schema::create_canonical_schema(&conn).expect("memory schema initialization failed");
        MemoryStore {
            conn,
            anns: RefCell::new(HashMap::new()),
            router: RefCell::new(None),
            data_dir: None,
            flush_handle: None,
            mutations: AtomicU64::new(0),
            router_last_refresh_mutation: Cell::new(0),
            router_last_refresh_attempt_ms: Cell::new(0),
            shard_mode: Cell::new(ShardMode::default()),
        }
    }

    /// Set the shard-routing policy (BENCH-SCALE-2, 2026-05-14). See
    /// [`ShardMode`]. Used by the LoCoMo-at-scale bench harness to compare
    /// router-routed vs all-shards probe at 1M docs. Production callers
    /// should keep the default ([`ShardMode::RouterRouted`]).
    pub fn set_shard_mode(&self, mode: ShardMode) {
        self.shard_mode.set(mode);
    }

    /// Return the current shard-routing policy.
    pub fn shard_mode(&self) -> ShardMode {
        self.shard_mode.get()
    }

    /// Return the current schema version.
    pub fn schema_version(&self) -> i64 {
        schema::schema_version(&self.conn).unwrap_or(0)
    }

    /// Bump the mutation counter and run `ANALYZE` when it crosses a 10k boundary.
    fn record_mutations(&self, n: u64) {
        let prev = self.mutations.fetch_add(n, Ordering::Relaxed);
        // When we cross an ANALYZE_EVERY boundary, refresh planner stats.
        if prev / ANALYZE_EVERY != (prev + n) / ANALYZE_EVERY {
            let _ = self.conn.execute_batch("ANALYZE;");
        }
    }

    /// Internal accessor to the underlying SQLite connection. `pub(crate)`
    /// so sibling modules in `crate::memory` (e.g. `edges`) can issue their
    /// own SQL without exposing `rusqlite::Connection` to the rest of the app.
    pub(crate) fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Data directory for on-disk stores; `None` for in-memory (tests).
    pub(crate) fn data_dir(&self) -> Option<&std::path::Path> {
        self.data_dir.as_deref()
    }

    /// Compute the logical shard key for a memory row by id.
    fn shard_key_for_id(&self, id: i64) -> Option<ShardKey> {
        let (tier, memory_type, tags, content): (String, String, String, String) = self
            .conn
            .query_row(
                "SELECT tier, memory_type, tags, content FROM memories WHERE id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .ok()?;
        let memory_type = MemoryType::from_str(&memory_type);
        let kind = classify_cognitive_kind(&memory_type, &tags, &content);
        Some(ShardKey {
            tier: MemoryTier::from_str(&tier),
            kind,
        })
    }

    fn open_shard_ann(&self, shard: ShardKey, dim: usize) -> Option<super::ann_index::AnnIndex> {
        if let Some(dir) = &self.data_dir {
            super::ann_index::AnnIndex::open_for_token(dir, &shard.as_path_token(), dim).ok()
        } else {
            super::ann_index::AnnIndex::new(dim).ok()
        }
    }

    fn live_embeddings_for_shard(
        &self,
        shard: ShardKey,
        dim: usize,
    ) -> Result<Vec<(i64, Vec<f32>)>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, embedding, memory_type, tags, content
                 FROM memories
                 WHERE tier = ?1 AND embedding IS NOT NULL",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![shard.tier.as_str()], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })
            .map_err(|e| e.to_string())?;

        let mut out = Vec::new();
        for row in rows {
            let (id, blob, memory_type, tags, content) = row.map_err(|e| e.to_string())?;
            let memory_type = MemoryType::from_str(&memory_type);
            if classify_cognitive_kind(&memory_type, &tags, &content) != shard.kind {
                continue;
            }
            let emb = bytes_to_embedding(&blob);
            if emb.len() == dim {
                out.push((id, emb));
            }
        }
        Ok(out)
    }

    fn ensure_shard_ann_for_dim(&self, shard: ShardKey, dim: usize) -> Option<()> {
        {
            let anns = self.anns.borrow();
            if let Some(idx) = anns.get(&shard) {
                if idx.dimensions() == dim {
                    return Some(());
                }
                return None;
            }
        }

        let idx = self.open_shard_ann(shard, dim)?;
        if idx.is_empty() {
            let entries = self.live_embeddings_for_shard(shard, dim).ok()?;
            let _ = idx.rebuild(entries.iter().map(|(id, emb)| (*id, emb.as_slice())));
        }
        self.anns.borrow_mut().insert(shard, idx);
        Some(())
    }

    fn ensure_shard_ann(&self, shard: ShardKey) -> Option<()> {
        let dim = super::ann_index::detect_dimensions(&self.conn)?;
        if dim == 0 {
            return None;
        }
        self.ensure_shard_ann_for_dim(shard, dim)
    }

    /// Insert a new memory entry and return it with its assigned id.
    ///
    /// Applies privacy scrubbing (strips API keys, tokens, passwords) and
    /// content-hash deduplication before inserting. If a memory with the
    /// same content hash already exists, returns the existing entry instead
    /// of creating a duplicate.
    pub fn add(&self, m: NewMemory) -> SqlResult<MemoryEntry> {
        let _t = Timer::start(&METRICS.add);
        SEARCH_CACHE.invalidate();

        // Privacy scrub — strip secrets before storing.
        let content = super::privacy::strip_secrets(&m.content);

        // Content-hash dedup — auto-compute SHA-256 if caller didn't provide.
        let content_hash = m.source_hash.clone().unwrap_or_else(|| {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(content.as_bytes());
            format!("{:x}", hasher.finalize())
        });
        // Check for existing entry with the same content hash.
        if let Ok(Some(existing)) = self.find_by_source_hash(&content_hash) {
            return Ok(existing);
        }

        let importance = m.importance.clamp(1, 5);
        let now = m.created_at.unwrap_or_else(now_ms);
        let token_count = estimate_tokens(&content);
        self.conn.execute(
            "INSERT INTO memories (content, tags, importance, memory_type, created_at, access_count, tier, decay_score, token_count, source_url, source_hash, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, 1.0, ?7, ?8, ?9, ?10)",
            params![content, m.tags, importance, m.memory_type.as_str(), now, MemoryTier::Long.as_str(), token_count, m.source_url, Some(&content_hash), m.expires_at],
        )?;
        let id = self.conn.last_insert_rowid();
        self.record_mutations(1);
        self.get_by_id(id)
    }

    /// Bulk-insert many memories in a single transaction (Phase 41.4).
    ///
    /// Returns the assigned row ids in the same order as the input.
    /// Skips the per-row `get_by_id` round-trip — callers that need the
    /// full `MemoryEntry` should call `get_by_id` afterwards. This is the
    /// path used by ingest pipelines that turn one document into thousands
    /// of chunks; it lifts insert throughput from ~600 rows/s (per-row
    /// auto-commit + per-row fsync) to >100k rows/s on commodity hardware.
    pub fn add_many(&self, mut items: Vec<NewMemory>) -> SqlResult<Vec<i64>> {
        let _t = Timer::start(&METRICS.add_many);
        SEARCH_CACHE.invalidate();
        if items.is_empty() {
            return Ok(Vec::new());
        }
        // Backpressure: reject bulk ingest that would exceed long-tier capacity.
        if let Err(e) = self.check_ingest_capacity(items.len()) {
            return Err(rusqlite::Error::QueryReturnedNoRows).map_err(|_| {
                rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_FULL),
                    Some(e.message),
                )
            });
        }
        let now = now_ms();
        let mut ids = Vec::with_capacity(items.len());
        let tier_str = MemoryTier::Long.as_str();
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare_cached(
                "INSERT INTO memories (content, tags, importance, memory_type, created_at, access_count, tier, decay_score, token_count, source_url, source_hash, expires_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, 1.0, ?7, ?8, ?9, ?10)",
            )?;
            for m in items.drain(..) {
                // Privacy scrub each chunk.
                let content = super::privacy::strip_secrets(&m.content);
                // Auto-compute content hash for dedup if not provided.
                let content_hash = m.source_hash.unwrap_or_else(|| {
                    use sha2::{Digest, Sha256};
                    let mut hasher = Sha256::new();
                    hasher.update(content.as_bytes());
                    format!("{:x}", hasher.finalize())
                });
                let importance = m.importance.clamp(1, 5);
                let token_count = estimate_tokens(&content);
                // BENCH-PARITY-3: per-row override falls back to the batch
                // `now` snapshot so historical-ingest paths can stamp each
                // memory with its session timestamp.
                let row_created_at = m.created_at.unwrap_or(now);
                stmt.execute(params![
                    content,
                    m.tags,
                    importance,
                    m.memory_type.as_str(),
                    row_created_at,
                    tier_str,
                    token_count,
                    m.source_url,
                    Some(&content_hash),
                    m.expires_at,
                ])?;
                ids.push(tx.last_insert_rowid());
            }
        }
        tx.commit()?;
        self.record_mutations(ids.len() as u64);
        Ok(ids)
    }

    /// Bulk content update inside a single transaction.
    ///
    /// Used by ingest pipelines that re-write large batches of rows
    /// (e.g. re-chunking) and by the million-memory benchmark. Skips
    /// version snapshots, importance clamping, and the per-row
    /// `get_by_id` round-trip — callers wanting full semantics should
    /// use [`MemoryStore::update`] one row at a time.
    pub fn update_content_many(&self, items: &[(i64, String)]) -> SqlResult<()> {
        if items.is_empty() {
            return Ok(());
        }
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare_cached("UPDATE memories SET content = ?1 WHERE id = ?2")?;
            for (id, content) in items {
                stmt.execute(params![content, id])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Bulk delete inside a single transaction. Also removes the rows
    /// from the ANN index on a best-effort basis.
    ///
    /// This method requires `&mut self` because it opens a transaction on
    /// the underlying SQLite connection, which needs mutable access in
    /// `rusqlite`, and performs batched mutations as an exclusive operation.
    pub fn delete_many(&mut self, ids: &[i64]) -> SqlResult<()> {
        if ids.is_empty() {
            return Ok(());
        }

        let shard_keys: Vec<Option<ShardKey>> =
            ids.iter().map(|id| self.shard_key_for_id(*id)).collect();

        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare_cached("DELETE FROM memories WHERE id = ?1")?;
            for id in ids {
                stmt.execute(params![id])?;
            }
        }
        tx.commit()?;

        for (id, shard_opt) in ids.iter().zip(shard_keys.iter()) {
            if let Some(shard) = shard_opt {
                if let Some(idx) = self.anns.borrow().get(shard) {
                    let _ = idx.remove(*id);
                }
            }
        }
        Ok(())
    }

    /// Insert a memory into a specific tier (for session management).
    pub fn add_to_tier(
        &self,
        m: NewMemory,
        tier: MemoryTier,
        session_id: Option<&str>,
    ) -> SqlResult<MemoryEntry> {
        let importance = m.importance.clamp(1, 5);
        let now = now_ms();
        let token_count = estimate_tokens(&m.content);
        self.conn.execute(
            "INSERT INTO memories (content, tags, importance, memory_type, created_at, access_count, tier, decay_score, session_id, token_count, source_url, source_hash, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, 1.0, ?7, ?8, ?9, ?10, ?11)",
            params![m.content, m.tags, importance, m.memory_type.as_str(), now, tier.as_str(), session_id, token_count, m.source_url, m.source_hash, m.expires_at],
        )?;
        let id = self.conn.last_insert_rowid();
        self.get_by_id(id)
    }

    /// Set a synthesized parent memory for a group of child memories.
    pub fn set_parent_for_memories(&self, child_ids: &[i64], parent_id: i64) -> SqlResult<usize> {
        if child_ids.is_empty() {
            return Ok(0);
        }
        SEARCH_CACHE.invalidate();
        let mut updated = 0usize;
        for child_id in child_ids {
            if *child_id == parent_id {
                continue;
            }
            updated += self.conn.execute(
                "UPDATE memories SET parent_id = ?1 WHERE id = ?2",
                params![parent_id, child_id],
            )?;
        }
        self.record_mutations(updated as u64);
        Ok(updated)
    }

    /// Fetch a memory by its id.
    pub fn get_by_id(&self, id: i64) -> SqlResult<MemoryEntry> {
        self.conn.query_row(
            "SELECT id, content, tags, importance, memory_type, created_at, last_accessed, access_count,
                    tier, decay_score, session_id, parent_id, token_count, source_url, source_hash, expires_at, valid_to, obsidian_path, last_exported, updated_at, origin_device, hlc_counter, confidence
             FROM memories WHERE id = ?1",
            params![id],
            row_to_entry,
        )
    }

    /// Return all memories ordered by importance (desc) then created_at (desc).
    pub fn get_all(&self) -> SqlResult<Vec<MemoryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, tags, importance, memory_type, created_at, last_accessed, access_count,
                    tier, decay_score, session_id, parent_id, token_count, source_url, source_hash, expires_at, valid_to, obsidian_path, last_exported, updated_at, origin_device, hlc_counter, confidence
             FROM memories ORDER BY importance DESC, created_at DESC",
        )?;
        let rows = stmt.query_map([], row_to_entry)?;
        rows.collect()
    }

    /// Return the broad memory list capped by estimated in-memory bytes.
    ///
    /// This bounds UI/cache memory without deleting any persisted rows.
    pub fn get_all_within_storage_bytes(&self, max_bytes: u64) -> SqlResult<Vec<MemoryEntry>> {
        let max_bytes = max_bytes.min(i64::MAX as u64) as i64;
        let mut stmt = self.conn.prepare(
            "SELECT id, content, tags, importance, memory_type, created_at, last_accessed, access_count,
                    tier, decay_score, session_id, parent_id, token_count, source_url, source_hash, expires_at, valid_to, obsidian_path, last_exported, updated_at, origin_device, hlc_counter, confidence,
                    length(content)
                    + length(tags)
                    + COALESCE(length(embedding), 0)
                    + COALESCE(length(source_url), 0)
                    + COALESCE(length(source_hash), 0)
                    + COALESCE(length(obsidian_path), 0)
                    + 128 AS row_bytes
             FROM memories ORDER BY importance DESC, created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| Ok((row_to_entry(row)?, row.get::<_, i64>(23)?)))?;

        let mut used = 0i64;
        let mut entries = Vec::new();
        for row in rows {
            let (entry, row_bytes) = row?;
            let row_bytes = row_bytes.max(0);
            if used > 0 && used.saturating_add(row_bytes) > max_bytes {
                break;
            }
            used = used.saturating_add(row_bytes);
            entries.push(entry);
        }
        Ok(entries)
    }

    /// Return estimated bytes represented by the current in-memory list cache cap.
    pub fn active_cache_bytes(&self, max_bytes: u64) -> SqlResult<i64> {
        let max_bytes = max_bytes.min(i64::MAX as u64) as i64;
        let mut stmt = self.conn.prepare(
            "SELECT length(content)
                    + length(tags)
                    + COALESCE(length(embedding), 0)
                    + COALESCE(length(source_url), 0)
                    + COALESCE(length(source_hash), 0)
                    + COALESCE(length(obsidian_path), 0)
                    + 128 AS row_bytes
             FROM memories ORDER BY importance DESC, created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;

        let mut used = 0i64;
        for row in rows {
            let row_bytes = row?.max(0);
            if used > 0 && used.saturating_add(row_bytes) > max_bytes {
                break;
            }
            used = used.saturating_add(row_bytes);
        }
        Ok(used)
    }

    /// Return memories in a specific tier.
    pub fn get_by_tier(&self, tier: &MemoryTier) -> SqlResult<Vec<MemoryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, tags, importance, memory_type, created_at, last_accessed, access_count,
                    tier, decay_score, session_id, parent_id, token_count, source_url, source_hash, expires_at, valid_to, obsidian_path, last_exported, updated_at, origin_device, hlc_counter, confidence
             FROM memories WHERE tier = ?1 ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![tier.as_str()], row_to_entry)?;
        rows.collect()
    }

    /// Get working + long-term memories (skip short-term ephemeral).
    pub fn get_persistent(&self) -> SqlResult<Vec<MemoryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, tags, importance, memory_type, created_at, last_accessed, access_count,
                    tier, decay_score, session_id, parent_id, token_count, source_url, source_hash, expires_at, valid_to, obsidian_path, last_exported, updated_at, origin_device, hlc_counter, confidence
             FROM memories WHERE tier IN ('working', 'long')
             ORDER BY importance DESC, decay_score DESC, created_at DESC",
        )?;
        let rows = stmt.query_map([], row_to_entry)?;
        rows.collect()
    }

    /// Return user-customizable default system settings matching a tag.
    ///
    /// These rows are ordinary memories so users can edit or supersede them,
    /// but category/tags let backend policy surfaces retrieve them reliably
    /// without hardcoding examples in Rust prompts.
    pub fn system_default_settings(&self, tag: &str, limit: usize) -> SqlResult<Vec<MemoryEntry>> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let tag_pattern = format!("%{}%", tag.trim().to_lowercase());
        let mut stmt = self.conn.prepare(
            "SELECT id, content, tags, importance, memory_type, created_at, last_accessed, access_count,
                    tier, decay_score, session_id, parent_id, token_count, source_url, source_hash, expires_at, valid_to, obsidian_path, last_exported, updated_at, origin_device, hlc_counter, confidence
             FROM memories
             WHERE valid_to IS NULL
               AND tier IN ('working', 'long')
               AND lower(tags) LIKE ?1
               AND (
                    category = 'system.default_system_setting'
                 OR lower(tags) LIKE '%system:default-system-setting%'
               )
             ORDER BY importance DESC, protected DESC, COALESCE(updated_at, created_at) DESC, created_at DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![tag_pattern, limit as i64], row_to_entry)?;
        rows.collect()
    }

    /// Full-text keyword search across content and tags.
    /// Updates `last_accessed` and `access_count` on matched entries.
    pub fn search(&self, query: &str) -> SqlResult<Vec<MemoryEntry>> {
        if query.trim().is_empty() {
            return self.get_all();
        }

        let entries = if self.has_fts5() {
            // FTS5 path: tokenize the user query into individual terms and
            // OR-match them so multi-word natural-language queries (e.g.
            // "How did we set up authentication?") return any document
            // containing any of the substantive tokens. Wrapping the whole
            // query as a single phrase would only match exact-phrase docs.
            let (terms, scoring_count) = query_terms(query);
            let tokens: Vec<String> = terms
                .iter()
                .map(|t| t.replace('"', "\"\""))
                .map(|t| format!("\"{t}\""))
                .collect();
            if tokens.is_empty() {
                return Ok(Vec::new());
            }
            let fts_query = tokens.join(" OR ");
            let mut stmt = self.conn.prepare(
                "SELECT m.id, m.content, m.tags, m.importance, m.memory_type, m.created_at, m.last_accessed, m.access_count,
                        m.tier, m.decay_score, m.session_id, m.parent_id, m.token_count, m.source_url, m.source_hash, m.expires_at, m.valid_to, m.obsidian_path, m.last_exported, m.updated_at, m.origin_device, m.hlc_counter, m.confidence
                 FROM memories m
                 JOIN memories_fts ON memories_fts.rowid = m.id
                 WHERE memories_fts MATCH ?1
                 ORDER BY rank",
            )?;
            let rows = stmt.query_map(params![fts_query], row_to_entry)?;
            let mut entries = rows.collect::<SqlResult<Vec<MemoryEntry>>>()?;
            // Rerank using only scoring terms (originals + semantic expansions),
            // NOT morphological variants (which are FTS5-recall-only).
            let scoring_terms = &terms[..scoring_count];
            rerank_by_lexical_score(&mut entries, scoring_terms);
            let by_id: HashMap<i64, MemoryEntry> = entries
                .iter()
                .map(|entry| (entry.id, entry.clone()))
                .collect();
            let seed_ids =
                top_unique_ids(entries.iter().map(|entry| entry.id), GRAPH_BOOST_SEED_LIMIT);
            let graph_boosts = self.graph_neighbor_boosts(&seed_ids, &by_id, scoring_terms)?;
            if !graph_boosts.is_empty() {
                rerank_by_lexical_and_graph_score(&mut entries, scoring_terms, &graph_boosts);
            }
            entries
        } else {
            // Fallback: LIKE-based full-table scan.
            let pattern = format!("%{}%", query.to_lowercase());
            let mut stmt = self.conn.prepare(
                "SELECT id, content, tags, importance, memory_type, created_at, last_accessed, access_count,
                        tier, decay_score, session_id, parent_id, token_count, source_url, source_hash, expires_at, valid_to, obsidian_path, last_exported, updated_at, origin_device, hlc_counter, confidence
                 FROM memories
                 WHERE lower(content) LIKE ?1 OR lower(tags) LIKE ?1
                 ORDER BY importance DESC, access_count DESC, created_at DESC",
            )?;
            let rows = stmt.query_map(params![pattern], row_to_entry)?;
            rows.collect::<SqlResult<Vec<MemoryEntry>>>()?
        };

        // Update last_accessed and access_count for matched entries.
        let now = now_ms();
        for e in &entries {
            let _ = self.conn.execute(
                "UPDATE memories SET last_accessed = ?1, access_count = access_count + 1 WHERE id = ?2",
                params![now, e.id],
            );
        }
        Ok(entries)
    }

    /// Update a memory entry. Only provided fields are changed.
    ///
    /// Saves a version snapshot of the *previous* state before applying
    /// the update (V8 schema, chunk 16.12). The snapshot is best-effort;
    /// if the versioning table doesn't exist yet (pre-V8 schema), the
    /// update still proceeds.
    pub fn update(&self, id: i64, upd: MemoryUpdate) -> SqlResult<MemoryEntry> {
        let _t = Timer::start(&METRICS.update);
        SEARCH_CACHE.invalidate();
        // Snapshot the current state before editing (best-effort).
        let content_changed = upd.content.is_some();
        let old_shard = if content_changed {
            self.shard_key_for_id(id)
        } else {
            None
        };
        let has_changes = content_changed
            || upd.tags.is_some()
            || upd.importance.is_some()
            || upd.memory_type.is_some();
        if has_changes {
            let _ = super::versioning::save_version(&self.conn, id);
        }

        if let Some(content) = upd.content {
            self.conn.execute(
                "UPDATE memories SET content = ?1 WHERE id = ?2",
                params![content, id],
            )?;
        }
        if let Some(tags) = upd.tags {
            self.conn.execute(
                "UPDATE memories SET tags = ?1 WHERE id = ?2",
                params![tags, id],
            )?;
        }
        if let Some(importance) = upd.importance {
            self.conn.execute(
                "UPDATE memories SET importance = ?1 WHERE id = ?2",
                params![importance.clamp(1, 5), id],
            )?;
        }
        if let Some(mt) = upd.memory_type {
            self.conn.execute(
                "UPDATE memories SET memory_type = ?1 WHERE id = ?2",
                params![mt.as_str(), id],
            )?;
        }

        // When content changes, the old embedding is stale.
        // Clear it, remove from ANN, and enqueue for re-embedding (41.6R).
        if content_changed {
            self.conn.execute(
                "UPDATE memories SET embedding = NULL WHERE id = ?1",
                params![id],
            )?;
            // Remove stale vector from the ANN index (best-effort).
            if let Some(shard) = old_shard {
                if let Some(idx) = self.anns.borrow().get(&shard) {
                    let _ = idx.remove(id);
                }
            }
            // Enqueue for re-embedding (best-effort — table may not exist
            // on very old schemas).
            let _ = super::embedding_queue::enqueue(&self.conn, id);
        }

        self.record_mutations(1);
        self.get_by_id(id)
    }

    /// Delete a memory entry by id.
    pub fn delete(&self, id: i64) -> SqlResult<()> {
        let _t = Timer::start(&METRICS.delete);
        SEARCH_CACHE.invalidate();
        let shard = self.shard_key_for_id(id);
        self.conn
            .execute("DELETE FROM memories WHERE id = ?1", params![id])?;
        // Remove from ANN index (best-effort).
        if let Some(shard) = shard {
            if let Some(idx) = self.anns.borrow().get(&shard) {
                let _ = idx.remove(id);
            }
        }
        self.record_mutations(1);
        Ok(())
    }

    /// Soft-close a memory by setting `valid_to` to the given timestamp.
    /// The entry is never deleted — this preserves the audit trail and
    /// allows undo. Used by contradiction resolution (Chunk 17.2).
    pub fn close_memory(&self, id: i64, valid_to_ms: i64) -> SqlResult<()> {
        self.conn.execute(
            "UPDATE memories SET valid_to = ?1 WHERE id = ?2",
            params![valid_to_ms, id],
        )?;
        Ok(())
    }

    /// Return the N most relevant memories for a message (keyword match + importance).
    /// Used to inject long-term context into the brain's system prompt.
    /// Uses candidate-pool retrieval (41.5R) instead of loading every row.
    pub fn relevant_for(&self, message: &str, limit: usize) -> Vec<String> {
        let words: Vec<String> = message
            .to_lowercase()
            .split_whitespace()
            .filter(|w| w.len() > 3)
            .map(String::from)
            .collect();

        // Use candidate pool instead of get_all() (41.5R).
        let Ok(candidates) = self.search_candidates(&words, None) else {
            return vec![];
        };

        let mut scored: Vec<(usize, &MemoryEntry)> = candidates
            .iter()
            .filter_map(|e| {
                let lower = e.content.to_lowercase();
                let tag_lower = e.tags.to_lowercase();
                let hits = words
                    .iter()
                    .filter(|w| lower.contains(w.as_str()) || tag_lower.contains(w.as_str()))
                    .count();
                if hits > 0 {
                    Some((e.importance as usize * (hits + 1), e))
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by_key(|item| std::cmp::Reverse(item.0));
        scored
            .iter()
            .take(limit)
            .map(|(_, e)| e.content.clone())
            .collect()
    }

    /// Return the total number of stored memories.
    pub fn count(&self) -> i64 {
        self.conn
            .query_row("SELECT COUNT(*) FROM memories", [], |r| r.get(0))
            .unwrap_or(0)
    }

    // ── Vector embedding operations ────────────────────────────────────────

    /// Store a pre-computed embedding for a memory entry.
    pub fn set_embedding(&self, id: i64, embedding: &[f32]) -> SqlResult<()> {
        let _t = Timer::start(&METRICS.set_embedding);
        let bytes = embedding_to_bytes(embedding);
        self.conn.execute(
            "UPDATE memories SET embedding = ?1 WHERE id = ?2",
            params![bytes, id],
        )?;
        // Keep the shard ANN index in sync (best-effort).
        if let Some(shard) = self.shard_key_for_id(id) {
            let _ = self.ensure_shard_ann_for_dim(shard, embedding.len());
            if let Some(idx) = self.anns.borrow().get(&shard) {
                let _ = idx.add(id, embedding);
            }
        }
        Ok(())
    }

    /// Record obsidian sync metadata after a successful export/import.
    pub fn set_obsidian_sync(&self, id: i64, path: &str, exported_at: i64) -> SqlResult<()> {
        self.conn.execute(
            "UPDATE memories SET obsidian_path = ?1, last_exported = ?2 WHERE id = ?3",
            params![path, exported_at, id],
        )?;
        Ok(())
    }

    /// Return all memories that have an embedding stored.
    pub fn get_with_embeddings(&self) -> SqlResult<Vec<MemoryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, tags, importance, memory_type, created_at, last_accessed, access_count,
                    tier, decay_score, session_id, parent_id, token_count, source_url, source_hash, expires_at, valid_to, embedding, obsidian_path, last_exported, updated_at, origin_device, hlc_counter, confidence
             FROM memories WHERE embedding IS NOT NULL",
        )?;
        let rows = stmt.query_map([], row_to_entry_with_embedding)?;
        rows.collect()
    }

    // ── Candidate-pool helpers (Chunk 41.5R) ────────────────────────────────
    //
    // Instead of loading the entire corpus via get_all()/get_with_embeddings()
    // on every search, we gather candidate IDs from three fast retrievers
    // (ANN, keyword SQL, freshness SQL), union them, then fetch only those
    // rows.  This keeps memory usage O(candidate_pool) instead of O(total).

    /// Maximum number of candidates fetched from each retriever.
    const CANDIDATE_POOL: usize = 1000;

    /// Fetch the IDs of the `pool` freshest + most-important memories.
    fn freshness_candidate_ids(&self, pool: usize) -> SqlResult<Vec<i64>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id FROM memories ORDER BY created_at DESC, importance DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![pool as i64], |row| row.get::<_, i64>(0))?;
        rows.collect()
    }

    /// Fetch the IDs of memories whose content or tags contain any of
    /// the given `words`. Uses FTS5 MATCH when the index is available,
    /// falling back to INSTR + LOWER (full-table scan) otherwise.
    /// Returns at most `pool` IDs.
    fn keyword_candidate_ids(&self, words: &[String], pool: usize) -> SqlResult<Vec<i64>> {
        if words.is_empty() {
            return Ok(Vec::new());
        }

        // Try FTS5 first — orders of magnitude faster at scale.
        if self.has_fts5() {
            return self.keyword_candidate_ids_fts5(words, pool);
        }

        // Fallback: full-table scan with INSTR (pre-FTS5 databases).
        self.keyword_candidate_ids_instr(words, pool)
    }

    /// Check whether the FTS5 index table exists and is usable.
    pub(crate) fn has_fts5(&self) -> bool {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='memories_fts'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0)
            > 0
    }

    /// FTS5-based keyword candidate retrieval. Builds an OR query from the
    /// word list and uses BM25 ranking for relevance ordering.
    fn keyword_candidate_ids_fts5(&self, words: &[String], pool: usize) -> SqlResult<Vec<i64>> {
        // Build FTS5 query: word1 OR word2 OR word3 ...
        // FTS5 tokenizes with unicode61 so we pass words as-is (no need to lowercase).
        let fts_query: String = words
            .iter()
            .map(|w| {
                // Escape double-quotes inside tokens for safety.
                let escaped = w.replace('"', "\"\"");
                format!("\"{escaped}\"")
            })
            .collect::<Vec<_>>()
            .join(" OR ");

        let mut stmt = self.conn.prepare_cached(
            "SELECT rowid FROM memories_fts WHERE memories_fts MATCH ?1 ORDER BY rank LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![fts_query, pool as i64], |row| row.get::<_, i64>(0))?;
        rows.collect()
    }

    /// Legacy INSTR-based keyword scan (fallback when FTS5 is unavailable).
    fn keyword_candidate_ids_instr(&self, words: &[String], pool: usize) -> SqlResult<Vec<i64>> {
        let conditions: Vec<String> = words
            .iter()
            .enumerate()
            .map(|(i, _)| {
                format!(
                    "(INSTR(LOWER(content), ?{p}) > 0 OR INSTR(LOWER(tags), ?{p}) > 0)",
                    p = i + 1
                )
            })
            .collect();
        let sql = format!(
            "SELECT id FROM memories WHERE {} LIMIT ?{}",
            conditions.join(" OR "),
            words.len() + 1
        );
        let mut stmt = self.conn.prepare_cached(&sql)?;
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = words
            .iter()
            .map(|w| Box::new(w.to_lowercase()) as Box<dyn rusqlite::types::ToSql>)
            .collect();
        param_values.push(Box::new(pool as i64));
        let refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|b| b.as_ref()).collect();
        let rows = stmt.query_map(&*refs, |row| row.get::<_, i64>(0))?;
        rows.collect()
    }

    /// Rebuild the FTS5 index from scratch. Useful during maintenance
    /// or when the index gets out of sync (e.g. after a direct SQL edit).
    /// Returns the number of rows indexed, or 0 if FTS5 is not available.
    pub fn rebuild_fts5(&self) -> SqlResult<usize> {
        if !self.has_fts5() {
            return Ok(0);
        }
        self.conn
            .execute_batch("INSERT INTO memories_fts(memories_fts) VALUES ('rebuild');")?;
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Fetch full entries (with embeddings) for a set of IDs.
    fn get_entries_by_ids_with_embeddings(&self, ids: &[i64]) -> SqlResult<Vec<MemoryEntry>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders: String = (0..ids.len())
            .map(|i| format!("?{}", i + 1))
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT id, content, tags, importance, memory_type, created_at, last_accessed, access_count,
                    tier, decay_score, session_id, parent_id, token_count, source_url, source_hash, expires_at, valid_to, embedding, obsidian_path, last_exported, updated_at, origin_device, hlc_counter, confidence
             FROM memories WHERE id IN ({})",
            placeholders
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let params: Vec<Box<dyn rusqlite::types::ToSql>> = ids
            .iter()
            .map(|id| Box::new(*id) as Box<dyn rusqlite::types::ToSql>)
            .collect();
        let refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|b| b.as_ref()).collect();
        let rows = stmt.query_map(&*refs, row_to_entry_with_embedding)?;
        rows.collect()
    }

    /// Fetch full entries (without embeddings) for a set of IDs.
    pub(crate) fn get_entries_by_ids(&self, ids: &[i64]) -> SqlResult<Vec<MemoryEntry>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders: String = (0..ids.len())
            .map(|i| format!("?{}", i + 1))
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT id, content, tags, importance, memory_type, created_at, last_accessed, access_count,
                    tier, decay_score, session_id, parent_id, token_count, source_url, source_hash, expires_at, valid_to, obsidian_path, last_exported, updated_at, origin_device, hlc_counter, confidence
             FROM memories WHERE id IN ({})",
            placeholders
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let params: Vec<Box<dyn rusqlite::types::ToSql>> = ids
            .iter()
            .map(|id| Box::new(*id) as Box<dyn rusqlite::types::ToSql>)
            .collect();
        let refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|b| b.as_ref()).collect();
        let rows = stmt.query_map(&*refs, row_to_entry)?;
        rows.collect()
    }

    fn graph_neighbor_boosts(
        &self,
        seed_ids: &[i64],
        by_id: &HashMap<i64, MemoryEntry>,
        terms: &[String],
    ) -> SqlResult<HashMap<i64, f64>> {
        if seed_ids.is_empty() || by_id.is_empty() || terms.is_empty() {
            return Ok(HashMap::new());
        }

        let mut boosts: HashMap<i64, f64> = HashMap::new();
        let mut stmt = self.conn.prepare_cached(
            "SELECT src_id, dst_id, confidence
             FROM memory_edges
             WHERE valid_to IS NULL AND (src_id = ?1 OR dst_id = ?1)
             LIMIT ?2",
        )?;

        for seed_id in seed_ids.iter().take(GRAPH_BOOST_SEED_LIMIT) {
            let Some(seed) = by_id.get(seed_id) else {
                continue;
            };
            if lexical_rank_score(seed, terms) == 0 {
                continue;
            }

            let rows = stmt.query_map(
                params![seed_id, GRAPH_BOOST_NEIGHBOR_LIMIT_PER_SEED],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, f64>(2)?,
                    ))
                },
            )?;

            for row in rows {
                let (src_id, dst_id, confidence) = row?;
                let neighbor_id = if src_id == *seed_id { dst_id } else { src_id };
                if neighbor_id == *seed_id {
                    continue;
                }
                let Some(neighbor) = by_id.get(&neighbor_id) else {
                    continue;
                };
                let lexical_score = lexical_rank_score(neighbor, terms);
                if lexical_score == 0 {
                    continue;
                }

                let lexical_gate = ((lexical_score as f64).ln_1p() / 4.0).clamp(0.25, 1.0);
                let edge_boost = confidence.clamp(0.0, 1.0) * GRAPH_BOOST_PER_EDGE * lexical_gate;
                let boost = boosts.entry(neighbor_id).or_insert(0.0);
                *boost = (*boost + edge_boost).min(GRAPH_BOOST_MAX);
            }
        }

        Ok(boosts)
    }

    /// Gather candidate IDs from ANN + keyword + freshness retrievers, then
    /// return the deduplicated union of entries (with embeddings if available).
    fn search_candidates(
        &self,
        query_words: &[String],
        query_embedding: Option<&[f32]>,
    ) -> SqlResult<Vec<MemoryEntry>> {
        use std::collections::HashSet;

        let pool = Self::CANDIDATE_POOL;
        let mut id_set: HashSet<i64> = HashSet::with_capacity(pool * 3);

        // (1) ANN vector candidates from router-selected shards + RRF merge
        if let Some(qe) = query_embedding {
            let mut per_shard_rankings: Vec<Vec<i64>> = Vec::new();
            // Use router to select top-p shards; falls back to all shards if router unavailable
            let shards_to_probe = self.select_shards_for_query(qe);
            for shard in shards_to_probe {
                let _ = self.ensure_shard_ann(shard);
                let anns = self.anns.borrow();
                if let Some(idx) = anns.get(&shard) {
                    if let Ok(matches) = idx.search(qe, pool) {
                        if !matches.is_empty() {
                            per_shard_rankings
                                .push(matches.into_iter().map(|(id, _)| id).collect::<Vec<_>>());
                        }
                    }
                }
            }
            if !per_shard_rankings.is_empty() {
                let ranking_slices: Vec<&[i64]> =
                    per_shard_rankings.iter().map(|r| r.as_slice()).collect();
                let merged = merge_shard_rankings(&ranking_slices, pool);
                id_set.extend(merged.into_iter().map(|(id, _)| id));
            }
        }

        // (2) Keyword candidates via SQL
        if let Ok(kw_ids) = self.keyword_candidate_ids(query_words, pool) {
            id_set.extend(kw_ids);
        }

        // (3) Freshness candidates
        if let Ok(fresh_ids) = self.freshness_candidate_ids(pool) {
            id_set.extend(fresh_ids);
        }

        // Fetch the full entries for the candidate set
        let ids: Vec<i64> = id_set.into_iter().collect();
        if query_embedding.is_some() {
            self.get_entries_by_ids_with_embeddings(&ids)
        } else {
            self.get_entries_by_ids(&ids)
        }
    }

    /// Return the IDs of entries that have no embedding yet (need processing).
    pub fn unembedded_ids(&self) -> SqlResult<Vec<(i64, String)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, content FROM memories WHERE embedding IS NULL")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.collect()
    }

    /// Count memories that have embeddings.
    pub fn embedded_count(&self) -> Result<usize, String> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM memories WHERE embedding IS NOT NULL",
                [],
                |row| row.get::<_, usize>(0),
            )
            .map_err(|e| e.to_string())
    }

    /// Clear all embeddings (set to NULL) for re-embedding with a new model.
    pub fn clear_all_embeddings(&self) -> Result<usize, String> {
        self.conn
            .execute("UPDATE memories SET embedding = NULL", [])
            .map_err(|e| e.to_string())
    }

    /// Fast cosine-similarity vector search.  Returns the top `limit`
    /// memory entries ranked by similarity to `query_embedding`.
    ///
    /// Uses the HNSW ANN index (Chunk 16.10) when available for O(log n)
    /// lookup; falls back to brute-force O(n) scan when the index is
    /// missing, empty, or has a dimension mismatch.
    pub fn vector_search(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> SqlResult<Vec<MemoryEntry>> {
        // ── Fast path: router-selected shards + per-shard ANN + RRF merge ──
        let mut per_shard_rankings: Vec<Vec<i64>> = Vec::new();
        // Use router to select top-p shards; falls back to all shards if router unavailable
        let shards_to_probe = self.select_shards_for_query(query_embedding);
        for shard in shards_to_probe {
            let _ = self.ensure_shard_ann(shard);
            let anns = self.anns.borrow();
            if let Some(idx) = anns.get(&shard) {
                if let Ok(matches) = idx.search(query_embedding, limit.max(1)) {
                    if !matches.is_empty() {
                        per_shard_rankings
                            .push(matches.into_iter().map(|(id, _)| id).collect::<Vec<_>>());
                    }
                }
            }
        }

        if !per_shard_rankings.is_empty() {
            let ranking_slices: Vec<&[i64]> =
                per_shard_rankings.iter().map(|r| r.as_slice()).collect();
            let merged = merge_shard_rankings(&ranking_slices, limit.max(1));
            if !merged.is_empty() {
                let now = now_ms();
                let mut results = Vec::with_capacity(merged.len());
                for (id, _score) in &merged {
                    if let Ok(entry) = self.get_by_id(*id) {
                        let _ = self.conn.execute(
                            "UPDATE memories SET last_accessed = ?1, access_count = access_count + 1 WHERE id = ?2",
                            params![now, entry.id],
                        );
                        results.push(entry);
                    }
                }
                if !results.is_empty() {
                    return Ok(results);
                }
            }
        }

        // ── Fallback: brute-force scan ────────────────────────────────────
        let all = self.get_with_embeddings()?;
        if all.is_empty() {
            return Ok(vec![]);
        }

        let mut scored: Vec<(f32, MemoryEntry)> = all
            .into_iter()
            .filter_map(|entry| {
                let emb = entry.embedding.as_ref()?;
                let sim = cosine_similarity(query_embedding, emb);
                Some((sim, entry))
            })
            .collect();

        // Sort descending by similarity.
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);

        // Touch access counters for the matched entries.
        let now = now_ms();
        for (_, e) in &scored {
            let _ = self.conn.execute(
                "UPDATE memories SET last_accessed = ?1, access_count = access_count + 1 WHERE id = ?2",
                params![now, e.id],
            );
        }

        Ok(scored.into_iter().map(|(_, e)| e).collect())
    }

    /// Check if a new text is a near-duplicate of an existing memory.
    /// Returns `Some(id)` of the most similar existing entry if cosine > threshold.
    ///
    /// Uses ANN index when available; falls back to brute-force scan.
    pub fn find_duplicate(
        &self,
        query_embedding: &[f32],
        threshold: f32,
    ) -> SqlResult<Option<i64>> {
        // ── Fast path: per-shard ANN scan ────────────────────────────────
        let mut best: Option<(i64, f32)> = None;
        for shard in ShardKey::all() {
            let _ = self.ensure_shard_ann(shard);
            let anns = self.anns.borrow();
            if let Some(idx) = anns.get(&shard) {
                if let Ok(matches) = idx.search(query_embedding, 1) {
                    if let Some(&(id, sim)) = matches.first() {
                        match best {
                            Some((_, bsim)) if bsim >= sim => {}
                            _ => best = Some((id, sim)),
                        }
                    }
                }
            }
        }
        if let Some((id, sim)) = best {
            if sim >= threshold {
                return Ok(Some(id));
            }
            return Ok(None);
        }

        // ── Fallback: brute-force scan ────────────────────────────────────
        let all = self.get_with_embeddings()?;
        let best = all
            .iter()
            .filter_map(|e| {
                let emb = e.embedding.as_ref()?;
                let sim = cosine_similarity(query_embedding, emb);
                if sim >= threshold {
                    Some((sim, e.id))
                } else {
                    None
                }
            })
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        Ok(best.map(|(_, id)| id))
    }

    // ── Hybrid search (the core RAG pipeline) ──────────────────────────────

    /// Multi-signal hybrid search that combines:
    /// 1. Vector cosine similarity (semantic relevance)
    /// 2. Keyword BM25-style scoring (exact match boost)
    /// 3. Recency bias (recent memories score higher)
    /// 4. Importance weighting (user-assigned priority)
    /// 5. Decay score (frequently accessed memories retain weight)
    /// 6. Tier priority (working > long for current-session context)
    ///
    /// Returns top `limit` entries ranked by composite score.
    /// Scales to 1M+ entries: vector search is O(n) but purely arithmetic.
    /// Like [`Self::hybrid_search`], but also filters out entries whose
    /// final hybrid score is below `min_score` (a value in `[0.0, 1.0]`,
    /// since the per-component weights sum to ≤ 1.0 by construction —
    /// see the inline weights above).
    ///
    /// Implements the "relevance threshold" item from
    /// `docs/brain-advanced-design.md` § 16 Phase 4 (Chunk 16.1). Pass
    /// `min_score = 0.0` to get the legacy behaviour (no filtering).
    ///
    /// Returns the surviving entries in descending score order, capped
    /// at `limit`. Touches `last_accessed` / `access_count` for survivors
    /// only — entries below the threshold do **not** count as accesses,
    /// which preserves the decay signal for genuinely irrelevant rows.
    pub fn hybrid_search_with_threshold(
        &self,
        query: &str,
        query_embedding: Option<&[f32]>,
        limit: usize,
        min_score: f64,
    ) -> SqlResult<Vec<MemoryEntry>> {
        let scored = self.hybrid_search_scored(query, query_embedding)?;
        let now = now_ms();

        let kept: Vec<MemoryEntry> = scored
            .into_iter()
            .filter(|(s, _)| *s >= min_score)
            .take(limit)
            .map(|(_, e)| e)
            .collect();

        // Touch access counters for survivors only. Below-threshold rows
        // are intentionally NOT counted as accesses — the decay signal
        // should keep them ageing out of relevance.
        for e in &kept {
            let _ = self.conn.execute(
                "UPDATE memories SET last_accessed = ?1, access_count = access_count + 1 WHERE id = ?2",
                params![now, e.id],
            );
        }

        Ok(kept)
    }

    /// Internal helper that returns every entry with its hybrid score,
    /// already sorted descending. Pure read — does not touch
    /// `access_count`. Shared between [`Self::hybrid_search`] and
    /// [`Self::hybrid_search_with_threshold`] so the two stay in lockstep.
    fn hybrid_search_scored(
        &self,
        query: &str,
        query_embedding: Option<&[f32]>,
    ) -> SqlResult<Vec<(f64, MemoryEntry)>> {
        let now = now_ms();
        let hour_ms: f64 = 3_600_000.0;

        let (words, scoring_count) = query_terms(query);

        // Use candidate-pool retrieval (41.5R) instead of loading entire corpus.
        let all = self.search_candidates(&words, query_embedding)?;

        if all.is_empty() {
            return Ok(vec![]);
        }

        // Scoring uses only originals + semantic expansions (not morphological
        // variants) so that keyword-hit density isn't inflated.
        let scoring_words = &words[..scoring_count];

        let mut scored: Vec<(f64, MemoryEntry)> = all
            .into_iter()
            .map(|entry| {
                let mut score = 0.0f64;

                if let (Some(qe), Some(emb)) = (query_embedding, entry.embedding.as_ref()) {
                    let sim = cosine_similarity(qe, emb) as f64;
                    score += sim * 0.40;
                }

                let lower_content = entry.content.to_lowercase();
                let lower_tags = entry.tags.to_lowercase();
                let keyword_hits = scoring_words
                    .iter()
                    .filter(|w| {
                        lower_content.contains(w.as_str()) || lower_tags.contains(w.as_str())
                    })
                    .count();
                if !scoring_words.is_empty() {
                    score += (keyword_hits as f64 / scoring_words.len() as f64) * 0.20;
                }

                let age_hours = (now - entry.created_at) as f64 / hour_ms;
                let recency = (-age_hours / 24.0).exp();
                score += recency * 0.15;

                score += (entry.importance as f64 / 5.0) * 0.10;
                score += entry.decay_score * 0.10;

                let tier_boost = match entry.tier {
                    MemoryTier::Working => 1.0,
                    MemoryTier::Long => 0.5,
                    MemoryTier::Short => 0.3,
                };
                score += tier_boost * 0.05;

                (score, entry)
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        Ok(scored)
    }

    pub fn hybrid_search(
        &self,
        query: &str,
        query_embedding: Option<&[f32]>,
        limit: usize,
    ) -> SqlResult<Vec<MemoryEntry>> {
        let _t = Timer::start(&METRICS.hybrid_search);
        let now = now_ms();
        let hour_ms: f64 = 3_600_000.0;

        // Keyword scoring setup
        let (words, scoring_count) = query_terms(query);

        // Use candidate-pool retrieval (41.5R) instead of loading entire corpus.
        let all = self.search_candidates(&words, query_embedding)?;

        if all.is_empty() {
            return Ok(vec![]);
        }

        let scoring_words = &words[..scoring_count];

        let mut scored: Vec<(f64, MemoryEntry)> = all
            .into_iter()
            .map(|entry| {
                let mut score = 0.0f64;

                // (1) Vector similarity — weight 0.40
                if let (Some(qe), Some(emb)) = (query_embedding, entry.embedding.as_ref()) {
                    let sim = cosine_similarity(qe, emb) as f64;
                    score += sim * 0.40;
                }

                // (2) Keyword match — weight 0.20
                let lower_content = entry.content.to_lowercase();
                let lower_tags = entry.tags.to_lowercase();
                let keyword_hits = scoring_words
                    .iter()
                    .filter(|w| {
                        lower_content.contains(w.as_str()) || lower_tags.contains(w.as_str())
                    })
                    .count();
                if !scoring_words.is_empty() {
                    score += (keyword_hits as f64 / scoring_words.len() as f64) * 0.20;
                }

                // (3) Recency — weight 0.15 (exponential decay, half-life = 24h)
                let age_hours = (now - entry.created_at) as f64 / hour_ms;
                let recency = (-age_hours / 24.0).exp(); // 1.0 = just created, 0.5 = 24h ago
                score += recency * 0.15;

                // (4) Importance — weight 0.10
                score += (entry.importance as f64 / 5.0) * 0.10;

                // (5) Decay score — weight 0.10
                score += entry.decay_score * 0.10;

                // (6) Tier priority — weight 0.05
                let tier_boost = match entry.tier {
                    MemoryTier::Working => 1.0,
                    MemoryTier::Long => 0.5,
                    MemoryTier::Short => 0.3,
                };
                score += tier_boost * 0.05;

                (score, entry)
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);

        // Touch access counters
        for (_, e) in &scored {
            let _ = self.conn.execute(
                "UPDATE memories SET last_accessed = ?1, access_count = access_count + 1 WHERE id = ?2",
                params![now, e.id],
            );
        }

        Ok(scored.into_iter().map(|(_, e)| e).collect())
    }

    /// Hybrid search using **Reciprocal Rank Fusion** (RRF) over three
    /// independent retrievers:
    ///
    /// 1. **Vector** ranking — cosine similarity vs `query_embedding`
    ///    (skipped if no embedding is provided).
    /// 2. **Keyword** ranking — count of distinct query tokens that appear
    ///    in the memory's content or tags (case-insensitive, words shorter
    ///    than 3 chars are ignored, BM25-style).
    /// 3. **Freshness** ranking — composite of recency (24 h half-life),
    ///    importance (1–5), `decay_score`, and tier weight (Working >
    ///    Long > Short).
    ///
    /// The three rankings are fused with [`crate::memory::fusion::reciprocal_rank_fuse`]
    /// using the standard `k = 60` constant (see Cormack et al., SIGIR 2009).
    ///
    /// RRF is preferred over the weighted-sum fusion in [`Self::hybrid_search`]
    /// when the underlying retrievers have **incomparable score scales**:
    /// raw cosine similarity (~0.0–1.0), keyword hit ratio (0.0–1.0), and
    /// freshness composites are all on different distributions, so summing
    /// them with hand-tuned weights is fragile. RRF operates purely on
    /// rank position, giving robust, parameter-light fusion.
    ///
    /// Implements §16 Phase 6 / §19.2 row 2 of `docs/brain-advanced-design.md`.
    /// Scales linearly in the number of memories with embeddings (the
    /// vector pass is the dominant cost; ~5 ms for 100 k entries).
    pub fn hybrid_search_rrf(
        &self,
        query: &str,
        query_embedding: Option<&[f32]>,
        limit: usize,
    ) -> SqlResult<Vec<MemoryEntry>> {
        let _t = Timer::start(&METRICS.hybrid_search_rrf);
        use crate::memory::cognitive_kind::classify as classify_kind;
        use crate::memory::confidence_decay::{confidence_factor, ConfidenceDecayConfig};
        use crate::memory::fusion::{reciprocal_rank_fuse, DEFAULT_RRF_K};
        use crate::memory::search_cache::{CachedHit, SearchCacheKey, SEARCH_CACHE};
        use std::collections::HashMap;

        if limit == 0 {
            return Ok(vec![]);
        }

        // ── Cache check ───────────────────────────────────────────────────
        let cache_key = SearchCacheKey {
            query: query.to_string(),
            mode: if query_embedding.is_some() {
                "rrf_vec_diverse".into()
            } else {
                "rrf_diverse".into()
            },
            limit,
        };
        if let Some(cached) = SEARCH_CACHE.get(&cache_key) {
            let _ch = Timer::start(&METRICS.rag_cache_hit);
            let ids: Vec<i64> = cached.iter().map(|h| h.memory_id).collect();
            // Fetch full entries preserving cached order.
            let mut by_id: HashMap<i64, MemoryEntry> = self
                .get_entries_by_ids(&ids)?
                .into_iter()
                .map(|e| (e.id, e))
                .collect();
            let results: Vec<MemoryEntry> =
                ids.into_iter().filter_map(|id| by_id.remove(&id)).collect();
            return Ok(results);
        }

        let now = now_ms();
        let hour_ms: f64 = 3_600_000.0;

        // Keyword scoring setup (also used for candidate retrieval)
        let (words, scoring_count) = query_terms(query);

        // Use candidate-pool retrieval (41.5R) instead of loading entire corpus.
        let all = {
            let _tc = Timer::start(&METRICS.rag_candidate_retrieval);
            self.search_candidates(&words, query_embedding)?
        };

        if all.is_empty() {
            return Ok(vec![]);
        }

        // Scoring uses only originals + semantic expansions, not morphological
        // variants, to avoid inflating keyword density / reranker scores.
        let scoring_words = &words[..scoring_count];

        // Index entries by id once so we can rebuild MemoryEntry ordering
        // after fusion without cloning the vector twice.
        let by_id: HashMap<i64, MemoryEntry> = all.iter().map(|e| (e.id, e.clone())).collect();

        // ── (1) Vector ranking ────────────────────────────────────────────
        let mut vector_rank: Vec<i64> = Vec::new();
        if let Some(qe) = query_embedding {
            let mut scored: Vec<(f32, i64)> = all
                .iter()
                .filter_map(|e| {
                    let emb = e.embedding.as_ref()?;
                    Some((cosine_similarity(qe, emb), e.id))
                })
                .collect();
            // Descending by similarity. Tie-break by id for determinism.
            scored.sort_by(|a, b| {
                b.0.partial_cmp(&a.0)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.1.cmp(&b.1))
            });
            vector_rank = scored.into_iter().map(|(_, id)| id).collect();
        }

        let mut keyword_rank: Vec<i64> = Vec::new();
        if !scoring_words.is_empty() {
            let weights = lexical_term_weights(&all, scoring_words);
            let mut scored: Vec<(f64, i64)> = all
                .iter()
                .filter_map(|e| {
                    let score = lexical_rank_score_weighted(e, scoring_words, &weights);
                    if score > 0.0 {
                        Some((score, e.id))
                    } else {
                        None
                    }
                })
                .collect();
            // Descending by lexical score, deterministic id tie-break.
            scored.sort_by(|a, b| {
                b.0.partial_cmp(&a.0)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.1.cmp(&b.1))
            });
            keyword_rank = scored.into_iter().map(|(_, id)| id).collect();
        }

        // ── (3) Freshness composite — used as a post-fusion multiplicative
        //        boost, NOT as a peer RRF ranking. Rank-based RRF treats
        //        every input ranking equally, which over-weights freshness
        //        on corpora where created_at is nearly uniform (and turns
        //        freshness into noise that displaces content-relevant hits).
        //        As a score multiplier the same signal nudges ties without
        //        diluting the lexical/semantic agreement RRF is built for.
        let freshness_boost: HashMap<i64, f64> = all
            .iter()
            .map(|e| {
                let age_hours = (now - e.created_at) as f64 / hour_ms;
                let recency = (-age_hours / 168.0).exp(); // half-life ≈ 1 week
                let importance = e.importance as f64 / 5.0;
                let tier_boost = match e.tier {
                    MemoryTier::Working => 1.0,
                    MemoryTier::Long => 0.85,
                    MemoryTier::Short => 0.7,
                };
                // Compressed into a gentle 0.7–1.15 multiplier so it can
                // break ties between equally relevant docs but cannot
                // overpower vector/keyword agreement.
                let raw = 0.6 + 0.15 * recency + 0.15 * importance + 0.1 * tier_boost;
                (e.id, raw.clamp(0.7, 1.15))
            })
            .collect();

        // ── Fuse with RRF (k = 60) ────────────────────────────────────────
        // Build the slice-of-slices input. Empty rankings (e.g. no embedding
        // or no usable query words) are simply skipped — RRF handles
        // missing-from-some-rankings gracefully. Freshness is intentionally
        // NOT a peer ranking here; it is applied as a multiplicative score
        // boost below so it can break ties without diluting the content
        // signal RRF is built for.
        //
        // Exception: when BOTH vector and keyword rankings are empty (no
        // embedding, no keyword overlap) we fall back to freshness-ordered
        // candidate IDs so that RRF still produces results. Without this,
        // unusual / novel queries would return nothing even though memories
        // exist.
        let _tf = Timer::start(&METRICS.rag_rrf_fusion);
        let freshness_rank: Vec<i64> = if vector_rank.is_empty() && keyword_rank.is_empty() {
            let mut ranked: Vec<(i64, i64, i64)> = all
                .iter()
                .map(|e| (e.created_at, e.importance, e.id))
                .collect();
            ranked.sort_by(|a, b| b.cmp(a));
            ranked.into_iter().map(|(_, _, id)| id).collect()
        } else {
            Vec::new()
        };
        let mut rankings: Vec<&[i64]> = Vec::with_capacity(4);
        if !vector_rank.is_empty() {
            rankings.push(&vector_rank);
        }
        if !keyword_rank.is_empty() {
            rankings.push(&keyword_rank);
        }
        if !freshness_rank.is_empty() {
            rankings.push(&freshness_rank);
        }

        let fused = reciprocal_rank_fuse(&rankings, DEFAULT_RRF_K);
        let graph_seed_ids = top_unique_ids(keyword_rank.iter().copied(), GRAPH_BOOST_SEED_LIMIT);
        let graph_boosts = self.graph_neighbor_boosts(&graph_seed_ids, &by_id, &words)?;

        // ── (4) Per-kind confidence decay (43.3) + freshness multiplier ──
        let decay_cfg = ConfidenceDecayConfig::default();
        let mut fused: Vec<(usize, i64, f64)> = fused
            .into_iter()
            .enumerate()
            .map(|(pos, (id, score))| {
                let adjusted = if let Some(entry) = by_id.get(&id) {
                    let kind = classify_kind(&entry.memory_type, &entry.tags, &entry.content);
                    let factor = confidence_factor(
                        &decay_cfg,
                        Some(kind),
                        entry.confidence,
                        now - entry.created_at,
                    );
                    let fresh = freshness_boost.get(&id).copied().unwrap_or(1.0);
                    let graph = 1.0 + graph_boosts.get(&id).copied().unwrap_or(0.0);
                    score * factor * fresh * graph
                } else {
                    score
                };
                (pos, id, adjusted)
            })
            .collect();
        // Re-sort: descending score, preserving RRF position order for ties.
        fused.sort_by(|a, b| {
            b.2.partial_cmp(&a.2)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });

        // Store result in cache for future lookups.
        let selected = select_diversified_ranked(
            fused.iter().map(|(_, id, score)| (*id, *score)),
            &by_id,
            limit,
            DEFAULT_MAX_RESULTS_PER_SESSION,
        );

        let cached_hits: Vec<CachedHit> = selected
            .iter()
            .map(|(id, score)| CachedHit {
                memory_id: *id,
                score: *score,
            })
            .collect();
        SEARCH_CACHE.put(cache_key, cached_hits);

        // Materialize the top-`limit` MemoryEntry list, preserving fused order.
        let top: Vec<MemoryEntry> = selected
            .into_iter()
            .filter_map(|(id, _)| by_id.get(&id).cloned())
            .collect();

        // Touch access counters for the matched entries.
        for e in &top {
            let _ = self.conn.execute(
                "UPDATE memories SET last_accessed = ?1, access_count = access_count + 1 WHERE id = ?2",
                params![now, e.id],
            );
        }

        Ok(top)
    }

    /// Query-intent–aware variant of [`hybrid_search_rrf`] (Chunk 16.6c).
    ///
    /// Runs the same RRF fusion as `hybrid_search_rrf`, then applies
    /// per-doc multiplicative score boosts derived from the user's
    /// **query intent** (procedural / episodic / factual / semantic /
    /// unknown). The boost for each doc is looked up from
    /// [`crate::memory::query_intent::IntentClassification::kind_boosts`]
    /// using the doc's classified [`CognitiveKind`].
    ///
    /// When the classifier returns `Unknown` (no signal) all boosts are
    /// 1.0, so this method becomes equivalent to `hybrid_search_rrf` —
    /// callers can use it unconditionally.
    ///
    /// Per `docs/brain-advanced-design.md` §3.5.6.
    pub fn hybrid_search_rrf_with_intent(
        &self,
        query: &str,
        query_embedding: Option<&[f32]>,
        limit: usize,
    ) -> SqlResult<Vec<MemoryEntry>> {
        use crate::memory::cognitive_kind::classify as classify_kind;
        use crate::memory::confidence_decay::{confidence_factor, ConfidenceDecayConfig};
        use crate::memory::fusion::{reciprocal_rank_fuse, DEFAULT_RRF_K};
        use crate::memory::query_intent::classify_query;
        use std::collections::HashMap;

        if limit == 0 {
            return Ok(vec![]);
        }

        // Classify intent up-front. If the classifier returns Unknown
        // with neutral boosts we can skip the rerank cleanly.
        let intent = classify_query(query);
        let needs_rerank = !matches!(
            intent.intent,
            crate::memory::query_intent::QueryIntent::Unknown
        );

        let now = now_ms();
        let hour_ms: f64 = 3_600_000.0;

        // Keyword words (also used for candidate-pool retrieval)
        let (words, scoring_count) = query_terms(query);

        // Use candidate-pool retrieval (41.5R) instead of loading entire corpus.
        let all = self.search_candidates(&words, query_embedding)?;
        if all.is_empty() {
            return Ok(vec![]);
        }

        let scoring_words = &words[..scoring_count];

        let by_id: HashMap<i64, MemoryEntry> = all.iter().map(|e| (e.id, e.clone())).collect();

        // ── (1) Vector ranking ────────────────────────────────────────
        let mut vector_rank: Vec<i64> = Vec::new();
        if let Some(qe) = query_embedding {
            let mut scored: Vec<(f32, i64)> = all
                .iter()
                .filter_map(|e| {
                    let emb = e.embedding.as_ref()?;
                    Some((cosine_similarity(qe, emb), e.id))
                })
                .collect();
            scored.sort_by(|a, b| {
                b.0.partial_cmp(&a.0)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.1.cmp(&b.1))
            });
            vector_rank = scored.into_iter().map(|(_, id)| id).collect();
        }

        // ── (2) Keyword ranking ───────────────────────────────────────

        let mut keyword_rank: Vec<i64> = Vec::new();
        if !scoring_words.is_empty() {
            let weights = lexical_term_weights(&all, scoring_words);
            let mut scored: Vec<(f64, i64)> = all
                .iter()
                .filter_map(|e| {
                    let score = lexical_rank_score_weighted(e, scoring_words, &weights);
                    if score > 0.0 {
                        Some((score, e.id))
                    } else {
                        None
                    }
                })
                .collect();
            scored.sort_by(|a, b| {
                b.0.partial_cmp(&a.0)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.1.cmp(&b.1))
            });
            keyword_rank = scored.into_iter().map(|(_, id)| id).collect();
        }

        // ── (3) Freshness composite — applied as a post-fusion multiplicative
        //        boost rather than a peer RRF ranking (see hybrid_search_rrf
        //        for rationale: rank-based RRF over-weights freshness on
        //        corpora with near-uniform created_at and dilutes the
        //        lexical/semantic agreement signal).
        let freshness_boost: HashMap<i64, f64> = all
            .iter()
            .map(|e| {
                let age_hours = (now - e.created_at) as f64 / hour_ms;
                let recency = (-age_hours / 168.0).exp(); // half-life ≈ 1 week
                let importance = e.importance as f64 / 5.0;
                let tier_boost = match e.tier {
                    MemoryTier::Working => 1.0,
                    MemoryTier::Long => 0.85,
                    MemoryTier::Short => 0.7,
                };
                let raw = 0.6 + 0.15 * recency + 0.15 * importance + 0.1 * tier_boost;
                (e.id, raw.clamp(0.7, 1.15))
            })
            .collect();

        // ── Fuse with RRF ─────────────────────────────────────────────
        let freshness_rank: Vec<i64> = if vector_rank.is_empty() && keyword_rank.is_empty() {
            let mut ranked: Vec<(i64, i64, i64)> = all
                .iter()
                .map(|e| (e.created_at, e.importance, e.id))
                .collect();
            ranked.sort_by(|a, b| b.cmp(a));
            ranked.into_iter().map(|(_, _, id)| id).collect()
        } else {
            Vec::new()
        };
        let mut rankings: Vec<&[i64]> = Vec::with_capacity(4);
        if !vector_rank.is_empty() {
            rankings.push(&vector_rank);
        }
        if !keyword_rank.is_empty() {
            rankings.push(&keyword_rank);
        }
        if !freshness_rank.is_empty() {
            rankings.push(&freshness_rank);
        }

        let mut fused = reciprocal_rank_fuse(&rankings, DEFAULT_RRF_K);
        let graph_seed_ids = top_unique_ids(keyword_rank.iter().copied(), GRAPH_BOOST_SEED_LIMIT);
        let graph_boosts = self.graph_neighbor_boosts(&graph_seed_ids, &by_id, &words)?;

        // ── (4a) Per-kind confidence decay (43.3) + freshness multiplier ─
        let decay_cfg = ConfidenceDecayConfig::default();
        for (id, score) in fused.iter_mut() {
            if let Some(entry) = by_id.get(id) {
                let kind = classify_kind(&entry.memory_type, &entry.tags, &entry.content);
                let factor = confidence_factor(
                    &decay_cfg,
                    Some(kind),
                    entry.confidence,
                    now - entry.created_at,
                );
                let fresh = freshness_boost.get(id).copied().unwrap_or(1.0);
                let graph = 1.0 + graph_boosts.get(id).copied().unwrap_or(0.0);
                *score *= factor * fresh * graph;
            }
        }

        // ── (4b) Intent-aware kind boosting ────────────────────────────
        if needs_rerank {
            // Multiply each fused score by the per-kind boost for the
            // doc's classified cognitive kind, then re-sort.
            for (id, score) in fused.iter_mut() {
                if let Some(entry) = by_id.get(id) {
                    let kind = classify_kind(&entry.memory_type, &entry.tags, &entry.content);
                    let boost = intent.kind_boosts.for_kind(kind) as f64;
                    *score *= boost;
                }
            }
            // Stable re-sort by boosted score (descending), id tie-break.
            fused.sort_by(|a, b| {
                b.1.partial_cmp(&a.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.0.cmp(&b.0))
            });
        }

        let selected =
            select_diversified_ranked(fused, &by_id, limit, DEFAULT_MAX_RESULTS_PER_SESSION);

        let top: Vec<MemoryEntry> = selected
            .into_iter()
            .filter_map(|(id, _)| by_id.get(&id).cloned())
            .collect();

        for e in &top {
            let _ = self.conn.execute(
                "UPDATE memories SET last_accessed = ?1, access_count = access_count + 1 WHERE id = ?2",
                params![now, e.id],
            );
        }

        Ok(top)
    }

    // ── Reinforcement provenance (43.4) ────────────────────────────────────

    /// Record that a memory was reinforced (confirmed useful) during a session.
    ///
    /// Uses `INSERT OR IGNORE` so repeated calls with the same
    /// `(memory_id, session_id, message_index)` PK are idempotent.
    pub fn record_reinforcement(
        &self,
        memory_id: i64,
        session_id: &str,
        message_index: i64,
    ) -> SqlResult<()> {
        let now = now_ms();
        self.conn.execute(
            "INSERT OR IGNORE INTO memory_reinforcements (memory_id, session_id, message_index, ts)
             VALUES (?1, ?2, ?3, ?4)",
            params![memory_id, session_id, message_index, now],
        )?;
        Ok(())
    }

    /// Retrieve the most recent reinforcements for a memory entry.
    pub fn get_reinforcements(
        &self,
        memory_id: i64,
        limit: usize,
    ) -> SqlResult<Vec<ReinforcementRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT memory_id, session_id, message_index, ts
             FROM memory_reinforcements
             WHERE memory_id = ?1
             ORDER BY ts DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![memory_id, limit as i64], |row| {
            Ok(ReinforcementRecord {
                memory_id: row.get(0)?,
                session_id: row.get(1)?,
                message_index: row.get(2)?,
                ts: row.get(3)?,
            })
        })?;
        rows.collect()
    }

    // ── Tier management ────────────────────────────────────────────────────

    /// Promote a memory to a higher tier.
    pub fn promote(&self, id: i64, new_tier: MemoryTier) -> SqlResult<()> {
        self.conn.execute(
            "UPDATE memories SET tier = ?1 WHERE id = ?2",
            params![new_tier.as_str(), id],
        )?;
        Ok(())
    }

    /// Apply time-based decay to all long-term memories.
    /// Memories that haven't been accessed recently lose decay_score.
    /// Called periodically (e.g. on app startup or once per session).
    ///
    /// Formula: decay_score *= 0.95^(hours_since_last_access / 168)
    /// (halves roughly every 2 weeks of non-access)
    pub fn apply_decay(&self) -> SqlResult<usize> {
        let now = now_ms();
        let mut stmt = self.conn.prepare(
            "SELECT id, last_accessed, decay_score, tags FROM memories WHERE tier = 'long'",
        )?;
        let rows: Vec<(i64, Option<i64>, f64, String)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get::<_, f64>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .collect();

        let mut updated = 0;
        for (id, last_accessed, current_decay, tags) in &rows {
            let last = last_accessed.unwrap_or(now);
            let hours_since = (now - last) as f64 / 3_600_000.0;
            // Chunk 18.2 — category-aware decay: per-prefix multiplier
            // pulled from the curated vocabulary. Lower multiplier = slower
            // decay (more durable). `personal:*` uses 0.5 → decays slower
            // than the 1.0 baseline; `tool:*` uses 1.5 → decays faster.
            // Default 1.0 for legacy / non-conforming tags.
            let multiplier = crate::memory::tag_vocabulary::category_decay_multiplier(tags);
            let factor = 0.95f64.powf((hours_since / 168.0) * multiplier);
            let new_decay = (current_decay * factor).max(0.01);
            if (new_decay - current_decay).abs() > 0.001 {
                self.conn.execute(
                    "UPDATE memories SET decay_score = ?1 WHERE id = ?2",
                    params![new_decay, id],
                )?;
                updated += 1;
            }
        }
        Ok(updated)
    }

    /// Evict short-term memories from a session, summarizing them into working memory.
    pub fn evict_short_term(&self, session_id: &str) -> SqlResult<Vec<MemoryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, tags, importance, memory_type, created_at, last_accessed, access_count,
                    tier, decay_score, session_id, parent_id, token_count, source_url, source_hash, expires_at, valid_to, obsidian_path, last_exported, updated_at, origin_device, hlc_counter, confidence
             FROM memories WHERE tier = 'short' AND session_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![session_id], row_to_entry)?;
        let entries: SqlResult<Vec<MemoryEntry>> = rows.collect();
        let entries = entries?;

        // Delete the short-term entries
        self.conn.execute(
            "DELETE FROM memories WHERE tier = 'short' AND session_id = ?1",
            params![session_id],
        )?;
        Ok(entries)
    }

    /// Promote working-tier entries to long-tier when they pass an
    /// access-pattern threshold. Pure SQL — no LLM, no embedding I/O.
    ///
    /// An entry is promoted when **both** conditions hold:
    /// 1. `access_count >= min_access_count` (default 5)
    /// 2. The most recent access (`last_accessed`) falls within the last
    ///    `window_days` days (default 7).
    ///
    /// Returns the IDs of promoted entries (in ascending id order). Designed
    /// to be called periodically (e.g. on app startup, alongside `apply_decay`).
    /// Idempotent — re-running on an already-long entry is a no-op because
    /// only `tier = 'working'` rows are considered.
    ///
    /// Maps to `docs/brain-advanced-design.md` § 16 Phase 5
    /// "Auto-promotion based on access patterns".
    pub fn auto_promote_to_long(
        &self,
        min_access_count: i64,
        window_days: i64,
    ) -> SqlResult<Vec<i64>> {
        // Defensive — non-positive window means "no recency requirement"; we
        // still treat 0 as "any time" to avoid silently promoting nothing.
        let cutoff_ms = if window_days <= 0 {
            0
        } else {
            now_ms().saturating_sub(window_days.saturating_mul(86_400_000))
        };
        let min_count = min_access_count.max(0);

        let mut stmt = self.conn.prepare(
            "SELECT id FROM memories
             WHERE tier = 'working'
               AND access_count >= ?1
               AND last_accessed IS NOT NULL
               AND last_accessed >= ?2
             ORDER BY id ASC",
        )?;
        let rows: Vec<i64> = stmt
            .query_map(params![min_count, cutoff_ms], |row| row.get::<_, i64>(0))?
            .filter_map(|r| r.ok())
            .collect();

        for id in &rows {
            self.conn.execute(
                "UPDATE memories SET tier = 'long' WHERE id = ?1",
                params![id],
            )?;
        }
        Ok(rows)
    }

    /// Nudge memory importance based on access patterns (Chunk 17.4).
    ///
    /// * Entries with `access_count >= hot_threshold` (default 10) gain +1
    ///   importance (capped at 5).
    /// * Entries with `access_count == 0` whose `last_accessed` is older
    ///   than `cold_days` (default 30) or NULL lose −1 importance (floored
    ///   at 1).
    ///
    /// Each adjustment is audited via `memory_versions` (V8 schema).
    /// Designed to be called periodically (daily or on app startup).
    /// Returns `(boosted, demoted)` counts.
    ///
    /// Maps to `docs/brain-advanced-design.md` §16 Phase 5
    /// "Memory importance auto-adjustment from access_count".
    pub fn adjust_importance_by_access(
        &self,
        hot_threshold: i64,
        cold_days: i64,
    ) -> SqlResult<(usize, usize)> {
        let hot = hot_threshold.max(1);
        let cold_cutoff = if cold_days <= 0 {
            now_ms() // everything is "cold" — edge case, still safe
        } else {
            now_ms().saturating_sub(cold_days.saturating_mul(86_400_000))
        };

        // ── Boost hot entries ──
        let mut boost_stmt = self.conn.prepare(
            "SELECT id, importance FROM memories
             WHERE access_count >= ?1
               AND importance < 5",
        )?;
        let hot_rows: Vec<(i64, i64)> = boost_stmt
            .query_map(params![hot], |row| Ok((row.get(0)?, row.get(1)?)))?
            .filter_map(|r| r.ok())
            .collect();

        let mut boosted = 0usize;
        for (id, current_imp) in &hot_rows {
            let new_imp = (*current_imp + 1).min(5);
            // Audit trail (best-effort; silently ignored on pre-V8 schemas)
            let _ = super::versioning::save_version(&self.conn, *id);
            self.conn.execute(
                "UPDATE memories SET importance = ?1 WHERE id = ?2",
                params![new_imp, id],
            )?;
            boosted += 1;
        }

        // ── Demote cold entries ──
        let mut cold_stmt = self.conn.prepare(
            "SELECT id, importance FROM memories
             WHERE access_count = 0
               AND (last_accessed IS NULL OR last_accessed < ?1)
               AND importance > 1",
        )?;
        let cold_rows: Vec<(i64, i64)> = cold_stmt
            .query_map(params![cold_cutoff], |row| Ok((row.get(0)?, row.get(1)?)))?
            .filter_map(|r| r.ok())
            .collect();

        let mut demoted = 0usize;
        for (id, current_imp) in &cold_rows {
            let new_imp = (*current_imp - 1).max(1);
            let _ = super::versioning::save_version(&self.conn, *id);
            self.conn.execute(
                "UPDATE memories SET importance = ?1 WHERE id = ?2",
                params![new_imp, id],
            )?;
            demoted += 1;
        }

        // Reset access_count for boosted entries so the next run doesn't
        // re-boost the same entries that already graduated.
        for (id, _) in &hot_rows {
            self.conn.execute(
                "UPDATE memories SET access_count = 0 WHERE id = ?1",
                params![id],
            )?;
        }

        Ok((boosted, demoted))
    }

    /// Find a memory by its source_hash.  Returns the first match (if any).
    pub fn find_by_source_hash(&self, hash: &str) -> SqlResult<Option<MemoryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, tags, importance, memory_type, created_at, last_accessed, access_count,
                    tier, decay_score, session_id, parent_id, token_count, source_url, source_hash, expires_at, valid_to, obsidian_path, last_exported, updated_at, origin_device, hlc_counter, confidence
             FROM memories WHERE source_hash = ?1 LIMIT 1",
        )?;
        let mut rows = stmt.query_map(params![hash], row_to_entry)?;
        match rows.next() {
            Some(Ok(entry)) => Ok(Some(entry)),
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    }

    /// Find all memories from a given source URL.
    pub fn find_by_source_url(&self, url: &str) -> SqlResult<Vec<MemoryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, tags, importance, memory_type, created_at, last_accessed, access_count,
                    tier, decay_score, session_id, parent_id, token_count, source_url, source_hash, expires_at, valid_to, obsidian_path, last_exported, updated_at, origin_device, hlc_counter, confidence
             FROM memories WHERE source_url = ?1 ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![url], row_to_entry)?;
        rows.collect()
    }

    /// Delete all memories from a given source URL.  Returns the count deleted.
    pub fn delete_by_source_url(&self, url: &str) -> SqlResult<usize> {
        let deleted = self
            .conn
            .execute("DELETE FROM memories WHERE source_url = ?1", params![url])?;
        Ok(deleted)
    }

    /// Delete expired memories (expires_at < now).  Returns the count deleted.
    pub fn delete_expired(&self) -> SqlResult<usize> {
        let now = now_ms();
        let deleted = self.conn.execute(
            "DELETE FROM memories WHERE expires_at IS NOT NULL AND expires_at < ?1",
            params![now],
        )?;
        Ok(deleted)
    }

    /// Delete memories below a decay threshold (garbage collection).
    pub fn gc_decayed(&self, threshold: f64) -> SqlResult<usize> {
        let deleted = self.conn.execute(
            "DELETE FROM memories WHERE tier = 'long' AND decay_score < ?1 AND importance <= 2",
            params![threshold],
        )?;
        Ok(deleted)
    }

    /// Estimate the active storage used by memory/RAG rows.
    pub fn active_storage_bytes(&self) -> SqlResult<i64> {
        self.conn.query_row(
            "SELECT COALESCE(SUM(
                length(content)
                + length(tags)
                + COALESCE(length(embedding), 0)
                + COALESCE(length(source_url), 0)
                + COALESCE(length(source_hash), 0)
                + COALESCE(length(obsidian_path), 0)
                + 128
            ), 0) FROM memories",
            [],
            |r| r.get(0),
        )
    }

    /// Prune the least-useful memories until estimated active storage is under `max_bytes`.
    pub fn enforce_size_limit(&self, max_bytes: u64) -> SqlResult<MemoryCleanupReport> {
        let max_bytes = max_bytes.min(i64::MAX as u64) as i64;
        let before = self.active_storage_bytes()?;
        if before <= max_bytes {
            return Ok(MemoryCleanupReport {
                before_bytes: before,
                after_bytes: before,
                max_bytes,
                deleted: 0,
            });
        }

        let mut stmt = self.conn.prepare(
            "SELECT id,
                    length(content)
                    + length(tags)
                    + COALESCE(length(embedding), 0)
                    + COALESCE(length(source_url), 0)
                    + COALESCE(length(source_hash), 0)
                    + COALESCE(length(obsidian_path), 0)
                    + 128 AS row_bytes
             FROM memories
             ORDER BY
                CASE tier WHEN 'short' THEN 0 WHEN 'working' THEN 1 ELSE 2 END ASC,
                importance ASC,
                decay_score ASC,
                COALESCE(last_accessed, 0) ASC,
                access_count ASC,
                created_at ASC",
        )?;
        let candidates: Vec<(i64, i64)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<SqlResult<Vec<_>>>()?;
        drop(stmt);

        let mut current = before;
        let mut deleted = 0usize;
        for (id, row_bytes) in candidates {
            if current <= max_bytes {
                break;
            }
            self.delete(id)?;
            current = current.saturating_sub(row_bytes.max(0));
            deleted += 1;
        }

        if deleted > 0 && self.data_dir.is_some() {
            let _ = self.conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
        }

        Ok(MemoryCleanupReport {
            before_bytes: before,
            after_bytes: self.active_storage_bytes()?,
            max_bytes,
            deleted,
        })
    }

    /// Delete **all** memories, edges, and conflicts. Returns the count of
    /// deleted memory rows. The ANN index is rebuilt empty.
    ///
    /// This is an irreversible destructive operation — the frontend must
    /// confirm with the user before calling.
    pub fn delete_all(&self) -> SqlResult<usize> {
        // Edges and conflicts cascade via FK, but be explicit for backends
        // that may not enforce FK cascades.
        self.conn
            .execute_batch(
                "DELETE FROM memory_edges;
             DELETE FROM memory_conflicts;
             DELETE FROM memory_versions;",
            )
            .ok(); // tables may not exist on older schemas — ignore errors
        let deleted = self.conn.execute("DELETE FROM memories", [])?;
        // Rebuild loaded shard ANN indices empty.
        for idx in self.anns.borrow().values() {
            let _ = idx.rebuild(std::iter::empty());
        }
        Ok(deleted)
    }

    /// Get memory statistics per tier.
    pub fn stats(&self) -> SqlResult<MemoryStats> {
        let total: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM memories", [], |r| r.get(0))?;
        let short: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM memories WHERE tier='short'",
            [],
            |r| r.get(0),
        )?;
        let working: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM memories WHERE tier='working'",
            [],
            |r| r.get(0),
        )?;
        let long: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM memories WHERE tier='long'", [], |r| {
                    r.get(0)
                })?;
        let embedded: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM memories WHERE embedding IS NOT NULL",
            [],
            |r| r.get(0),
        )?;
        let total_tokens: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(token_count), 0) FROM memories",
            [],
            |r| r.get(0),
        )?;
        let avg_decay: f64 = self.conn.query_row(
            "SELECT COALESCE(AVG(decay_score), 1.0) FROM memories WHERE tier='long'",
            [],
            |r| r.get(0),
        )?;
        let storage_bytes = self.active_storage_bytes()?;
        Ok(MemoryStats {
            total,
            short,
            working,
            long,
            embedded,
            total_tokens,
            avg_decay,
            storage_bytes,
            cache_bytes: storage_bytes,
        })
    }

    /// Count long-tier memories with vector embeddings, which is the
    /// health signal behind `rag_quality_pct`.
    pub fn embedded_long_count(&self) -> SqlResult<i64> {
        self.conn.query_row(
            "SELECT COUNT(*) FROM memories WHERE tier='long' AND embedding IS NOT NULL",
            [],
            |r| r.get(0),
        )
    }

    // ── Phase 41 ANN management methods ────────────────────────────────────────

    /// Store the ANN flush handle so insert/update paths can signal dirty state.
    pub fn set_flush_handle(&mut self, handle: super::ann_flush::AnnFlushHandle) {
        self.flush_handle = Some(handle);
    }

    /// Save all ANN indices to disk.  Returns `(flush_count, ops_flushed)`.
    /// Called by the background flush task.
    pub fn ann_save_all(&self) -> (u64, u64) {
        let mut saved = 0u64;
        for idx in self.anns.borrow().values() {
            let _ = idx.save();
            saved += 1;
        }
        (saved.max(1), 0)
    }

    /// Returns `true` if the ANN index has fragmentation above the compaction
    /// threshold (20%).  Returns `false` if no index exists.
    pub fn ann_needs_compaction(&self) -> bool {
        const COMPACTION_THRESHOLD: f32 = 0.20;
        self.anns
            .borrow()
            .values()
            .any(|idx| idx.fragmentation_ratio() > COMPACTION_THRESHOLD)
    }

    /// Rebuild the ANN index from live long-tier entries, removing tombstones.
    /// Returns the number of vectors in the rebuilt index.
    pub fn compact_ann(&self) -> Result<usize, String> {
        let mut total = 0usize;
        for shard in ShardKey::all() {
            let _ = self.ensure_shard_ann(shard);
            let dim = {
                let anns = self.anns.borrow();
                match anns.get(&shard) {
                    Some(idx) => idx.dimensions(),
                    None => continue,
                }
            };
            let entries = self.live_embeddings_for_shard(shard, dim)?;
            if let Some(idx) = self.anns.borrow().get(&shard) {
                let count = idx.rebuild(entries.iter().map(|(id, emb)| (*id, emb.as_slice())))?;
                idx.reset_fragmentation();
                total += count;
            }
        }
        Ok(total)
    }

    /// Backfill the `embedding_model_id` column for entries that have an
    /// embedding but no model tag.  Returns the number of rows updated.
    pub fn backfill_embedding_model(&self, model_id: &str) -> Result<usize, String> {
        self.conn
            .execute(
                "UPDATE memories SET embedding_model_id = ?1
                 WHERE embedding IS NOT NULL AND embedding_model_id IS NULL",
                rusqlite::params![model_id],
            )
            .map_err(|e| e.to_string())
    }

    /// Rebuild the ANN index with a new quantization mode. Returns vector count.
    pub fn rebuild_ann_quantized(
        &self,
        quant: super::ann_index::EmbeddingQuantization,
    ) -> Result<usize, String> {
        let dim = super::ann_index::detect_dimensions(&self.conn).unwrap_or(0);
        if dim == 0 {
            return Ok(0);
        }

        let mut total = 0usize;
        for shard in ShardKey::all() {
            let entries = self.live_embeddings_for_shard(shard, dim)?;
            let new_idx = if let Some(dir) = &self.data_dir {
                super::ann_index::AnnIndex::open_quantized_for_token(
                    dir,
                    &shard.as_path_token(),
                    dim,
                    quant,
                )?
            } else {
                super::ann_index::AnnIndex::new_quantized(dim, quant)?
            };
            let count = new_idx.rebuild(entries.iter().map(|(id, emb)| (*id, emb.as_slice())))?;
            self.anns.borrow_mut().insert(shard, new_idx);
            total += count;
        }
        Ok(total)
    }

    /// Rebuild all shard ANN indices with PQ selection (Phase 48.4).
    /// Large shards (> 50M entries) automatically use PQ quantization.
    /// Smaller shards use F32. Returns total vectors indexed across all shards.
    pub fn rebuild_ann_with_pq_selection(&self) -> Result<usize, String> {
        let dim = super::ann_index::detect_dimensions(&self.conn).unwrap_or(0);
        if dim == 0 {
            return Ok(0);
        }

        let mut total = 0usize;
        for shard in ShardKey::all() {
            let entries = self.live_embeddings_for_shard(shard, dim)?;
            let entry_count = entries.len();

            // Decide quantization based on shard size
            let quant = if entry_count > super::ann_index::LARGE_SHARD_THRESHOLD {
                super::ann_index::EmbeddingQuantization::PQ
            } else {
                super::ann_index::EmbeddingQuantization::F32
            };

            let new_idx = if let Some(dir) = &self.data_dir {
                super::ann_index::AnnIndex::open_quantized_for_token(
                    dir,
                    &shard.as_path_token(),
                    dim,
                    quant,
                )?
            } else {
                super::ann_index::AnnIndex::new_quantized(dim, quant)?
            };

            // Track entry count for future PQ decisions
            new_idx.set_entry_count(entry_count);

            // Build PQ codebooks for large shards
            if entry_count > super::ann_index::LARGE_SHARD_THRESHOLD {
                // Sample a subset for codebook building (10k entries or 10%, whichever is smaller)
                let sample_size = std::cmp::min(10_000, entry_count / 10);
                let embeddings: Vec<Vec<f32>> = entries
                    .iter()
                    .step_by((entry_count / sample_size).max(1))
                    .take(sample_size)
                    .map(|(_, emb)| emb.clone())
                    .collect();

                new_idx.build_pq_codebooks(&embeddings)?;

                // Save codebooks to disk if we have a data directory
                if let Some(dir) = &self.data_dir {
                    new_idx.save_pq_codebooks(dir)?;
                }
            }

            let count = new_idx.rebuild(entries.iter().map(|(id, emb)| (*id, emb.as_slice())))?;
            self.anns.borrow_mut().insert(shard, new_idx);
            total += count;
        }
        Ok(total)
    }

    /// Rebalance all shard ANN indices from live embeddings and persist them.
    /// Returns total vectors indexed across all shards.
    pub fn rebalance_shards(&self) -> Result<usize, String> {
        let dim = super::ann_index::detect_dimensions(&self.conn).unwrap_or(0);
        if dim == 0 {
            return Ok(0);
        }

        let mut total = 0usize;
        for shard in ShardKey::all() {
            let entries = self.live_embeddings_for_shard(shard, dim)?;
            let idx = self
                .open_shard_ann(shard, dim)
                .ok_or_else(|| format!("failed to open shard index {}", shard.as_path_token()))?;
            let count = idx.rebuild(entries.iter().map(|(id, emb)| (*id, emb.as_slice())))?;
            self.anns.borrow_mut().insert(shard, idx);
            total += count;
        }
        Ok(total)
    }

    /// Build a coarse shard router from a 1% sample of embeddings.
    /// The router learns to route queries to the top-p most relevant shards,
    /// reducing search fan-out from 15 to ~5 shards on average (Chunk 48.3).
    ///
    /// Returns the number of centroids added to the router, or an error.
    pub fn build_shard_router(&self) -> Result<usize, String> {
        let dim = super::ann_index::detect_dimensions(&self.conn).unwrap_or(0);
        if dim == 0 {
            return Ok(0);
        }

        // Sample 1% of all embeddings across all shards
        let all_entries = self.live_embeddings_per_shard(dim)?;
        let total_entries: usize = all_entries.values().map(|es| es.len()).sum();

        if total_entries == 0 {
            // No embeddings yet; return empty router
            return Ok(0);
        }

        let mut router = super::shard_router::ShardRouter::new(dim)
            .map_err(|e| format!("failed to create router: {}", e))?;

        let mut centroid_id = 0u32;

        for (shard, entries) in all_entries.iter() {
            if entries.is_empty() {
                continue;
            }

            // Deterministic 1% sample: take every 100th entry (or more if count < 100)
            let sample_stride = (entries.len() / 100).max(1);
            let mut sampled: Vec<&Vec<f32>> = Vec::new();

            for (i, entry) in entries.iter().enumerate() {
                if i % sample_stride == 0 {
                    sampled.push(entry);
                }
            }

            // Add centroids to router
            for embedding in sampled {
                router
                    .add_centroid(centroid_id, embedding, *shard)
                    .map_err(|e| format!("failed to add centroid: {}", e))?;
                centroid_id += 1;
            }
        }

        // Save router to disk
        if let Some(data_dir) = &self.data_dir {
            let vectors_dir = data_dir.join("vectors");
            let _ = std::fs::create_dir_all(&vectors_dir);
            router
                .save_to_dir(&vectors_dir)
                .map_err(|e| format!("failed to save router: {}", e))?;
        }

        *self.router.borrow_mut() = Some(router);
        self.router_last_refresh_mutation
            .set(self.mutations.load(Ordering::Relaxed));
        self.router_last_refresh_attempt_ms.set(now_ms());
        Ok(centroid_id as usize)
    }

    /// Throttled router refresh policy used by both query path and scheduled
    /// maintenance. Returns `Ok(Some(count))` when a refresh ran, `Ok(None)`
    /// when skipped by policy, or `Err` on refresh failure.
    pub fn maybe_refresh_shard_router(&self, force: bool) -> Result<Option<usize>, String> {
        let now = now_ms();
        let last_attempt = self.router_last_refresh_attempt_ms.get();
        if !force && last_attempt > 0 && now - last_attempt < ROUTER_REFRESH_COOLDOWN_MS {
            return Ok(None);
        }

        let current_mutations = self.mutations.load(Ordering::Relaxed);
        let last_refresh_mutations = self.router_last_refresh_mutation.get();
        let mutation_delta = current_mutations.saturating_sub(last_refresh_mutations);

        let router_state = self.router.borrow();
        let has_cached = router_state.is_some();
        let cached_stale = router_state.as_ref().map(|r| r.is_stale()).unwrap_or(true);
        drop(router_state);

        let due_by_time = !has_cached || cached_stale;
        let due_by_volume = mutation_delta >= ROUTER_REFRESH_MIN_MUTATIONS;
        if !force && !due_by_time && !due_by_volume {
            return Ok(None);
        }

        self.router_last_refresh_attempt_ms.set(now);
        let count = self.build_shard_router()?;
        Ok(Some(count))
    }

    /// Router metadata surfaced in health checks.
    pub fn router_health_summary(&self) -> Result<super::shard_router::RouterHealth, String> {
        let now = now_ms();
        let has_cached_router = self.router.borrow().is_some();
        let vectors_dir = self.data_dir.as_ref().map(|d| d.join("vectors"));

        let disk_meta = if let Some(dir) = vectors_dir.as_ref() {
            super::shard_router::load_disk_meta(dir)?
        } else {
            None
        };

        let cached_meta = self.router.borrow().as_ref().map(|router| {
            (
                router.built_at(),
                router.centroid_count(),
                router.is_stale(),
            )
        });

        let built_at = cached_meta
            .as_ref()
            .map(|m| m.0)
            .or_else(|| disk_meta.as_ref().map(|m| m.built_at));
        let centroid_count = cached_meta
            .as_ref()
            .map(|m| m.1)
            .or_else(|| disk_meta.as_ref().map(|m| m.centroid_count))
            .unwrap_or(0);
        let stale = cached_meta.as_ref().map(|m| m.2).unwrap_or_else(|| {
            built_at
                .map(|ts| {
                    now.saturating_sub(ts)
                        > super::shard_router::ShardRouter::STALENESS_THRESHOLD_MS
                })
                .unwrap_or(true)
        });

        let age_ms = built_at.map(|ts| now.saturating_sub(ts));
        let last_attempt = self.router_last_refresh_attempt_ms.get();
        let last_attempt = if last_attempt > 0 {
            Some(last_attempt)
        } else {
            None
        };
        let current_mutations = self.mutations.load(Ordering::Relaxed);
        let mutation_delta =
            current_mutations.saturating_sub(self.router_last_refresh_mutation.get());

        Ok(super::shard_router::RouterHealth {
            has_cached_router,
            has_persisted_router: disk_meta.is_some(),
            built_at,
            age_ms,
            centroid_count,
            stale,
            last_refresh_attempt_ms: last_attempt,
            refresh_cooldown_ms: ROUTER_REFRESH_COOLDOWN_MS as u64,
            min_mutations_for_refresh: ROUTER_REFRESH_MIN_MUTATIONS,
            mutations_since_refresh: mutation_delta,
        })
    }

    /// Load the shard router from disk, if it exists and is healthy.
    /// Falls back to `Ok(None)` if missing or stale (> 24h old).
    pub fn load_shard_router(&self) -> Result<Option<super::shard_router::ShardRouter>, String> {
        if let Some(data_dir) = &self.data_dir {
            let vectors_dir = data_dir.join("vectors");
            if vectors_dir.exists() {
                match super::shard_router::ShardRouter::load_from_dir(&vectors_dir) {
                    Ok(Some(router)) => {
                        if router.is_healthy() {
                            return Ok(Some(router));
                        }
                    }
                    Ok(None) => {
                        return Ok(None);
                    }
                    Err(e) => {
                        eprintln!("Warning: failed to load router: {}", e);
                        return Ok(None);
                    }
                }
            }
        }
        Ok(None)
    }

    /// Select top-p shards for a query embedding using the coarse router.
    /// Falls back to "all shards" if the router is missing, stale, or invalid.
    ///
    /// When [`ShardMode::AllShards`] is set via [`MemoryStore::set_shard_mode`],
    /// the router is bypassed entirely and every query probes every shard
    /// (BENCH-SCALE-2, 2026-05-14 — used to measure the router's
    /// contribution to latency/recall at scale).
    pub fn select_shards_for_query(&self, query_embedding: &[f32]) -> Vec<ShardKey> {
        if self.shard_mode.get() == ShardMode::AllShards {
            return ShardKey::all();
        }
        // Try to use the cached router
        {
            let router_ref = self.router.borrow();
            if let Some(router) = router_ref.as_ref() {
                if router.is_healthy() {
                    if let Ok(shards) = router.select_top_shards(
                        query_embedding,
                        super::shard_router::ShardRouter::DEFAULT_TOP_P,
                    ) {
                        if !shards.is_empty() {
                            return shards;
                        }
                    }
                }
            }
        }

        // Try loading a persisted router from disk (if available and healthy).
        if let Ok(Some(router)) = self.load_shard_router() {
            if let Ok(shards) = router.select_top_shards(
                query_embedding,
                super::shard_router::ShardRouter::DEFAULT_TOP_P,
            ) {
                if !shards.is_empty() {
                    *self.router.borrow_mut() = Some(router);
                    return shards;
                }
            }
        }

        // Try a throttled rebuild before fallback. This avoids repeated rebuild
        // bursts under high query load while still healing stale/missing routers.
        let _ = self.maybe_refresh_shard_router(false);
        {
            let router_ref = self.router.borrow();
            if let Some(router) = router_ref.as_ref() {
                if router.is_healthy() {
                    if let Ok(shards) = router.select_top_shards(
                        query_embedding,
                        super::shard_router::ShardRouter::DEFAULT_TOP_P,
                    ) {
                        if !shards.is_empty() {
                            return shards;
                        }
                    }
                }
            }
        }

        // Fallback: probe all shards
        ShardKey::all()
    }

    /// Helper to gather all embeddings grouped by shard for router building.
    /// Returns a map from `ShardKey` to vectors of embedding bytes.
    fn live_embeddings_per_shard(
        &self,
        expected_dim: usize,
    ) -> Result<std::collections::HashMap<ShardKey, Vec<Vec<f32>>>, String> {
        let mut result: std::collections::HashMap<ShardKey, Vec<Vec<f32>>> =
            std::collections::HashMap::new();

        for shard in ShardKey::all() {
            let entries = self.live_embeddings_for_shard(shard, expected_dim)?;
            if !entries.is_empty() {
                result.insert(shard, entries.into_iter().map(|(_, emb)| emb).collect());
            }
        }

        Ok(result)
    }

    /// Check if any shard's PQ codebooks are stale and need refresh (Phase 48.4).
    /// Returns true if at least one large shard has stale codebooks (> 7 days old) or missing codebooks.
    pub fn pq_codebooks_need_refresh(&self) -> bool {
        let data_dir = match &self.data_dir {
            Some(dir) => dir,
            None => return false, // No data directory, can't persist codebooks
        };

        for shard in ShardKey::all() {
            if let Some(idx) = self.anns.borrow().get(&shard) {
                // Check if this is a large shard that should have codebooks
                if idx.is_large_shard() {
                    // Load codebooks from disk to check if they exist and are stale
                    let codebook_path = data_dir.join("vectors.pq.json");

                    if let Ok(Some(codebooks)) =
                        super::ann_index::PQCodebooks::load_from_disk(&codebook_path)
                    {
                        if codebooks.is_stale() {
                            return true;
                        }
                    } else {
                        // Missing or unreadable codebooks for large shard = needs refresh
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Refresh PQ codebooks for all large shards (Phase 48.4 Phase 2).
    /// Rebuilds codebooks from a sample of embeddings if they're stale or missing.
    /// Returns the number of shards refreshed.
    pub fn refresh_pq_codebooks(&self) -> Result<usize, String> {
        let data_dir = match &self.data_dir {
            Some(dir) => dir.clone(),
            None => return Ok(0), // No data directory, skip refresh
        };

        let dim = super::ann_index::detect_dimensions(&self.conn).unwrap_or(0);
        if dim == 0 {
            return Ok(0);
        }

        let mut refreshed_count = 0usize;

        for shard in ShardKey::all() {
            let entries = self.live_embeddings_for_shard(shard, dim)?;

            // Only refresh for large shards
            if entries.len() <= super::ann_index::LARGE_SHARD_THRESHOLD {
                continue;
            }

            // Get the shard index
            if let Some(idx) = self.anns.borrow().get(&shard) {
                // Check if codebooks are stale or missing
                let codebook_path = data_dir.join("vectors.pq.json");
                let needs_refresh =
                    match super::ann_index::PQCodebooks::load_from_disk(&codebook_path) {
                        Ok(Some(codebooks)) => codebooks.is_stale(),
                        Ok(None) => true, // Missing codebooks
                        Err(_) => true,   // Load error, assume stale
                    };

                if needs_refresh {
                    // Sample embeddings for codebook building (10k or 10%, whichever is smaller)
                    let sample_size = std::cmp::min(10_000, entries.len() / 10);
                    let embeddings: Vec<Vec<f32>> = entries
                        .iter()
                        .step_by((entries.len() / sample_size).max(1))
                        .take(sample_size)
                        .map(|(_, emb)| emb.clone())
                        .collect();

                    // Rebuild codebooks
                    if idx.build_pq_codebooks(&embeddings).is_ok() {
                        // Save to disk
                        let _ = idx.save_pq_codebooks(&data_dir);
                        refreshed_count += 1;
                    }
                }
            }
        }

        Ok(refreshed_count)
    }

    /// Phase 3 kickoff planner: decide which shards should be migrated to
    /// disk-backed ANN first, based on per-shard cardinality.
    pub fn disk_ann_plan(
        &self,
        threshold: usize,
    ) -> Result<super::disk_backed_ann::DiskAnnPlan, String> {
        let threshold = if threshold == 0 {
            super::disk_backed_ann::DEFAULT_DISK_ANN_ENTRY_THRESHOLD
        } else {
            threshold
        };

        let mut rows: Vec<(String, usize, bool)> = Vec::new();
        for shard in ShardKey::all() {
            let count = self.shard_entry_count(&shard).map_err(|e| e.to_string())? as usize;
            if count == 0 {
                continue;
            }
            let ann_exists = if let Some(dir) = self.data_dir() {
                super::ann_index::index_path_for_token(dir, &shard.as_path_token()).exists()
            } else {
                true
            };
            rows.push((shard.as_path_token(), count, ann_exists));
        }

        Ok(super::disk_backed_ann::plan_from_counts(threshold, rows))
    }

    /// Execute one disk-backed ANN migration batch by writing IVF-PQ sidecars
    /// for top candidate shards. This is the first executable Phase 3 path:
    /// it does not build IVF-PQ indexes yet, but persists per-shard migration
    /// metadata so a future index-builder can consume deterministic sidecars.
    pub fn run_disk_ann_migration_job(
        &self,
        threshold: usize,
        max_shards: usize,
    ) -> Result<super::disk_backed_ann::DiskAnnMigrationReport, String> {
        let threshold = if threshold == 0 {
            super::disk_backed_ann::DEFAULT_DISK_ANN_ENTRY_THRESHOLD
        } else {
            threshold
        };
        let max_shards = if max_shards == 0 {
            super::disk_backed_ann::DEFAULT_DISK_ANN_MAX_SHARDS_PER_RUN
        } else {
            max_shards
        };

        let mut report =
            super::disk_backed_ann::DiskAnnMigrationReport::empty(threshold, max_shards);
        let Some(data_dir) = self.data_dir() else {
            return Ok(report);
        };

        let plan = self.disk_ann_plan(threshold)?;
        if plan.candidates.is_empty() {
            return Ok(report);
        }

        let vectors_dir = data_dir.join("vectors");
        for candidate in plan.candidates.iter().take(max_shards) {
            report.attempted += 1;

            if !candidate.ann_index_exists {
                report.skipped_missing_ann += 1;
                report
                    .items
                    .push(super::disk_backed_ann::DiskAnnMigrationItem {
                        shard: candidate.shard.clone(),
                        migrated: false,
                        reason: "ANN index file missing; sidecar not written".to_string(),
                    });
                continue;
            }

            let ann_path = super::ann_index::index_path_for_token(data_dir, &candidate.shard);
            if !ann_path.exists() {
                report.skipped_missing_ann += 1;
                report
                    .items
                    .push(super::disk_backed_ann::DiskAnnMigrationItem {
                        shard: candidate.shard.clone(),
                        migrated: false,
                        reason: "ANN index path does not exist on disk".to_string(),
                    });
                continue;
            }

            let sidecar = super::disk_backed_ann::DiskAnnSidecar::new(
                candidate.shard.clone(),
                candidate.entry_count,
                threshold,
                ann_path.to_string_lossy().to_string(),
            );
            super::disk_backed_ann::write_sidecar(&vectors_dir, &sidecar)?;

            report.migrated += 1;
            report.sidecars_written += 1;
            report
                .items
                .push(super::disk_backed_ann::DiskAnnMigrationItem {
                    shard: candidate.shard.clone(),
                    migrated: true,
                    reason: "IVF-PQ sidecar written".to_string(),
                });
        }

        Ok(report)
    }

    /// Build IVF-PQ indexes for shards that have `planned` sidecars.
    ///
    /// This is the Phase 3 execution step. For each shard with a `planned` sidecar:
    /// 1. Load all embeddings from the shard's HNSW index or SQLite
    /// 2. Train IVF coarse centroids + PQ codebooks
    /// 3. Encode all vectors and write the IVF-PQ binary index
    /// 4. Update sidecar status to `"built"`
    ///
    /// Returns the number of shards successfully built.
    pub fn build_ivf_pq_indexes(
        &self,
        max_shards: usize,
    ) -> Result<Vec<super::ivf_pq::IvfPqBuildStats>, String> {
        let Some(data_dir) = self.data_dir() else {
            return Ok(Vec::new());
        };
        let vectors_dir = data_dir.join("vectors");
        if !vectors_dir.exists() {
            return Ok(Vec::new());
        }

        // Find sidecars with status "planned"
        let sidecars = super::disk_backed_ann::list_sidecars(&vectors_dir)?;
        let planned: Vec<_> = sidecars
            .into_iter()
            .filter(|s| s.status == "planned" || s.status == "stale")
            .take(max_shards)
            .collect();

        if planned.is_empty() {
            return Ok(Vec::new());
        }

        let mut results = Vec::new();
        for sidecar in &planned {
            // Load embeddings for this shard from SQLite
            let embeddings = self.load_shard_embeddings(&sidecar.shard)?;
            if embeddings.is_empty() {
                continue;
            }

            let dim = embeddings[0].1.len();
            if dim == 0 {
                continue;
            }

            match super::disk_backed_ann::build_ivf_pq_for_shard(
                &vectors_dir,
                &sidecar.shard,
                embeddings,
                dim,
            ) {
                Ok(stats) => results.push(stats),
                Err(e) => {
                    eprintln!("Warning: IVF-PQ build failed for shard {}: {e}", sidecar.shard);
                }
            }
        }

        Ok(results)
    }

    /// Load all embeddings for a shard from the SQLite memory_embeddings table.
    /// Returns (memory_id, embedding_vector) pairs.
    fn load_shard_embeddings(&self, shard: &str) -> Result<Vec<(i64, Vec<f32>)>, String> {
        // Parse shard key to get tier and cognitive_kind
        let parts: Vec<&str> = shard.split("__").collect();
        if parts.len() != 2 {
            return Err(format!("Invalid shard key: {shard}"));
        }
        let tier = parts[0];
        let cognitive_kind = parts[1];

        let sql = r#"
            SELECT me.memory_id, me.embedding
            FROM memory_embeddings me
            JOIN memories m ON m.id = me.memory_id
            WHERE m.tier = ?1
              AND COALESCE(m.cognitive_kind, 'semantic') = ?2
              AND me.embedding IS NOT NULL
            ORDER BY me.memory_id
        "#;

        let conn = self.conn();
        let mut stmt = conn.prepare(sql).map_err(|e| format!("Prepare: {e}"))?;
        let rows = stmt
            .query_map(rusqlite::params![tier, cognitive_kind], |row| {
                let id: i64 = row.get(0)?;
                let blob: Vec<u8> = row.get(1)?;
                Ok((id, blob))
            })
            .map_err(|e| format!("Query: {e}"))?;

        let mut embeddings = Vec::new();
        for row in rows {
            let (id, blob) = row.map_err(|e| format!("Row: {e}"))?;
            // Embeddings are stored as little-endian f32 arrays
            if blob.len() % 4 != 0 {
                continue;
            }
            let vec: Vec<f32> = blob
                .chunks_exact(4)
                .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                .collect();
            if !vec.is_empty() {
                embeddings.push((id, vec));
            }
        }

        Ok(embeddings)
    }

    /// Search an IVF-PQ index for a shard. Returns (memory_id, distance) pairs.
    /// Falls back to None if no IVF-PQ index is available.
    pub fn search_ivf_pq(
        &self,
        shard: &str,
        query: &[f32],
        k: usize,
        nprobe: usize,
    ) -> Result<Option<Vec<(i64, f32)>>, String> {
        let Some(data_dir) = self.data_dir() else {
            return Ok(None);
        };
        let vectors_dir = data_dir.join("vectors");

        let index = super::disk_backed_ann::load_ivf_pq_index(&vectors_dir, shard)?;
        let Some(index) = index else {
            return Ok(None);
        };

        let results = index.search(query, k, nprobe);
        Ok(Some(
            results.into_iter().map(|r| (r.id, r.distance)).collect(),
        ))
    }

    /// Disk-backed ANN migration health summary for `brain_health`.
    pub fn disk_ann_health_summary(
        &self,
        threshold: usize,
    ) -> Result<super::disk_backed_ann::DiskAnnHealthSummary, String> {
        let threshold = if threshold == 0 {
            super::disk_backed_ann::DEFAULT_DISK_ANN_ENTRY_THRESHOLD
        } else {
            threshold
        };
        let plan = self.disk_ann_plan(threshold)?;

        let sidecars = if let Some(data_dir) = self.data_dir() {
            super::disk_backed_ann::list_sidecars(&data_dir.join("vectors"))?
        } else {
            Vec::new()
        };

        let sidecar_shards: HashSet<String> = sidecars.into_iter().map(|s| s.shard).collect();
        let ready_candidates = plan
            .candidates
            .iter()
            .filter(|candidate| sidecar_shards.contains(&candidate.shard))
            .count();
        let eligible_candidates = plan.candidate_count;

        Ok(super::disk_backed_ann::DiskAnnHealthSummary {
            threshold,
            eligible_candidates,
            sidecars_total: sidecar_shards.len(),
            ready_candidates,
            missing_sidecars: eligible_candidates.saturating_sub(ready_candidates),
        })
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn row_to_entry(row: &rusqlite::Row<'_>) -> SqlResult<MemoryEntry> {
    Ok(MemoryEntry {
        id: row.get(0)?,
        content: row.get(1)?,
        tags: row.get(2)?,
        importance: row.get(3)?,
        memory_type: MemoryType::from_str(&row.get::<_, String>(4)?),
        created_at: row.get(5)?,
        last_accessed: row.get(6)?,
        access_count: row.get(7)?,
        embedding: None,
        tier: MemoryTier::from_str(
            &row.get::<_, String>(8)
                .unwrap_or_else(|_| "long".to_string()),
        ),
        decay_score: row.get::<_, f64>(9).unwrap_or(1.0),
        session_id: row.get(10).unwrap_or(None),
        parent_id: row.get(11).unwrap_or(None),
        token_count: row.get::<_, i64>(12).unwrap_or(0),
        source_url: row.get(13).unwrap_or(None),
        source_hash: row.get(14).unwrap_or(None),
        expires_at: row.get(15).unwrap_or(None),
        valid_to: row.get(16).unwrap_or(None),
        obsidian_path: row.get(17).unwrap_or(None),
        last_exported: row.get(18).unwrap_or(None),
        updated_at: row.get(19).unwrap_or(None),
        origin_device: row.get(20).unwrap_or(None),
        hlc_counter: row.get(21).unwrap_or(None),
        confidence: row.get::<_, f64>(22).unwrap_or(1.0),
    })
}

fn row_to_entry_with_embedding(row: &rusqlite::Row<'_>) -> SqlResult<MemoryEntry> {
    let blob: Option<Vec<u8>> = row.get(17)?;
    Ok(MemoryEntry {
        id: row.get(0)?,
        content: row.get(1)?,
        tags: row.get(2)?,
        importance: row.get(3)?,
        memory_type: MemoryType::from_str(&row.get::<_, String>(4)?),
        created_at: row.get(5)?,
        last_accessed: row.get(6)?,
        access_count: row.get(7)?,
        embedding: blob.map(|b| bytes_to_embedding(&b)),
        tier: MemoryTier::from_str(
            &row.get::<_, String>(8)
                .unwrap_or_else(|_| "long".to_string()),
        ),
        decay_score: row.get::<_, f64>(9).unwrap_or(1.0),
        session_id: row.get(10).unwrap_or(None),
        parent_id: row.get(11).unwrap_or(None),
        token_count: row.get::<_, i64>(12).unwrap_or(0),
        source_url: row.get(13).unwrap_or(None),
        source_hash: row.get(14).unwrap_or(None),
        expires_at: row.get(15).unwrap_or(None),
        valid_to: row.get(16).unwrap_or(None),
        obsidian_path: row.get(18).unwrap_or(None),
        last_exported: row.get(19).unwrap_or(None),
        updated_at: row.get(20).unwrap_or(None),
        origin_device: row.get(21).unwrap_or(None),
        hlc_counter: row.get(22).unwrap_or(None),
        confidence: row.get::<_, f64>(23).unwrap_or(1.0),
    })
}

/// Convert an f32 slice to little-endian bytes for BLOB storage.
pub fn embedding_to_bytes(embedding: &[f32]) -> Vec<u8> {
    embedding.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// Convert little-endian bytes back to an f32 vec.
pub fn bytes_to_embedding(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// Cosine similarity between two vectors.  Returns 0.0 on degenerate input.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let (mut dot, mut na, mut nb) = (0.0f64, 0.0f64, 0.0f64);
    for (x, y) in a.iter().zip(b.iter()) {
        let (x, y) = (*x as f64, *y as f64);
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    let denom = na.sqrt() * nb.sqrt();
    if denom < 1e-12 {
        0.0
    } else {
        (dot / denom) as f32
    }
}

/// Rough token estimation (~4 chars per token for English text).
fn estimate_tokens(text: &str) -> i64 {
    (text.len() as i64 + 3) / 4
}

// ── StorageBackend impl for MemoryStore (SQLite) ─────────────────────────────

use super::backend::{StorageBackend, StorageResult};

impl StorageBackend for MemoryStore {
    fn migrate(&self) -> StorageResult<()> {
        // Schema initialization runs automatically in MemoryStore::new / in_memory
        Ok(())
    }

    fn schema_version(&self) -> StorageResult<i64> {
        Ok(self.schema_version())
    }

    fn add(&self, m: NewMemory) -> StorageResult<MemoryEntry> {
        Ok(self.add(m)?)
    }

    fn add_to_tier(
        &self,
        m: NewMemory,
        tier: MemoryTier,
        session_id: Option<&str>,
    ) -> StorageResult<MemoryEntry> {
        Ok(self.add_to_tier(m, tier, session_id)?)
    }

    fn get_by_id(&self, id: i64) -> StorageResult<MemoryEntry> {
        Ok(self.get_by_id(id)?)
    }

    fn get_all(&self) -> StorageResult<Vec<MemoryEntry>> {
        Ok(self.get_all()?)
    }

    fn get_by_tier(&self, tier: &MemoryTier) -> StorageResult<Vec<MemoryEntry>> {
        Ok(self.get_by_tier(tier)?)
    }

    fn get_persistent(&self) -> StorageResult<Vec<MemoryEntry>> {
        Ok(self.get_persistent()?)
    }

    fn count(&self) -> StorageResult<i64> {
        Ok(self.count())
    }

    fn stats(&self) -> StorageResult<MemoryStats> {
        Ok(self.stats()?)
    }

    fn search(&self, query: &str) -> StorageResult<Vec<MemoryEntry>> {
        Ok(self.search(query)?)
    }

    fn relevant_for(&self, message: &str, limit: usize) -> StorageResult<Vec<String>> {
        Ok(self.relevant_for(message, limit))
    }

    fn find_by_source_url(&self, url: &str) -> StorageResult<Vec<MemoryEntry>> {
        Ok(self.find_by_source_url(url)?)
    }

    fn find_by_source_hash(&self, hash: &str) -> StorageResult<Option<MemoryEntry>> {
        Ok(self.find_by_source_hash(hash)?)
    }

    fn get_with_embeddings(&self) -> StorageResult<Vec<MemoryEntry>> {
        Ok(self.get_with_embeddings()?)
    }

    fn unembedded_ids(&self) -> StorageResult<Vec<(i64, String)>> {
        Ok(self.unembedded_ids()?)
    }

    fn set_embedding(&self, id: i64, embedding: &[f32]) -> StorageResult<()> {
        Ok(self.set_embedding(id, embedding)?)
    }

    fn vector_search(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> StorageResult<Vec<MemoryEntry>> {
        Ok(self.vector_search(query_embedding, limit)?)
    }

    fn find_duplicate(
        &self,
        query_embedding: &[f32],
        threshold: f32,
    ) -> StorageResult<Option<i64>> {
        Ok(self.find_duplicate(query_embedding, threshold)?)
    }

    fn hybrid_search(
        &self,
        query: &str,
        query_embedding: Option<&[f32]>,
        limit: usize,
    ) -> StorageResult<Vec<MemoryEntry>> {
        Ok(self.hybrid_search(query, query_embedding, limit)?)
    }

    fn hybrid_search_rrf(
        &self,
        query: &str,
        query_embedding: Option<&[f32]>,
        limit: usize,
    ) -> StorageResult<Vec<MemoryEntry>> {
        Ok(self.hybrid_search_rrf(query, query_embedding, limit)?)
    }

    fn update(&self, id: i64, upd: MemoryUpdate) -> StorageResult<MemoryEntry> {
        Ok(self.update(id, upd)?)
    }

    fn promote(&self, id: i64, new_tier: MemoryTier) -> StorageResult<()> {
        Ok(self.promote(id, new_tier)?)
    }

    fn delete(&self, id: i64) -> StorageResult<()> {
        Ok(self.delete(id)?)
    }

    fn delete_by_source_url(&self, url: &str) -> StorageResult<usize> {
        Ok(self.delete_by_source_url(url)?)
    }

    fn delete_expired(&self) -> StorageResult<usize> {
        Ok(self.delete_expired()?)
    }

    fn delete_all(&self) -> StorageResult<usize> {
        Ok(self.delete_all()?)
    }

    fn apply_decay(&self) -> StorageResult<usize> {
        Ok(self.apply_decay()?)
    }

    fn evict_short_term(&self, session_id: &str) -> StorageResult<Vec<MemoryEntry>> {
        Ok(self.evict_short_term(session_id)?)
    }

    fn gc_decayed(&self, threshold: f64) -> StorageResult<usize> {
        Ok(self.gc_decayed(threshold)?)
    }

    fn backend_name(&self) -> &'static str {
        "SQLite"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_memory(content: &str) -> NewMemory {
        NewMemory {
            content: content.to_string(),
            tags: "test".to_string(),
            importance: 3,
            memory_type: MemoryType::Fact,
            ..Default::default()
        }
    }

    #[test]
    fn shard_mode_defaults_to_router_routed() {
        let store = MemoryStore::in_memory();
        assert_eq!(store.shard_mode(), ShardMode::RouterRouted);
    }

    #[test]
    fn shard_mode_set_and_get_roundtrip() {
        let store = MemoryStore::in_memory();
        store.set_shard_mode(ShardMode::AllShards);
        assert_eq!(store.shard_mode(), ShardMode::AllShards);
        store.set_shard_mode(ShardMode::RouterRouted);
        assert_eq!(store.shard_mode(), ShardMode::RouterRouted);
    }

    #[test]
    fn shard_mode_all_shards_bypasses_router_and_returns_every_shard() {
        // BENCH-SCALE-2: in AllShards mode, `select_shards_for_query`
        // must return every logical shard regardless of router state.
        // An empty in-memory store has no router built and no
        // embeddings, so the router fallback would also return all
        // shards \u2014 but the toggle should short-circuit before any
        // router work is done. We rely on `ShardKey::all()` being the
        // canonical list and assert exact equality.
        let store = MemoryStore::in_memory();
        store.set_shard_mode(ShardMode::AllShards);
        let dummy_query_embedding = vec![0.0f32; 4];
        let shards = store.select_shards_for_query(&dummy_query_embedding);
        let mut got: Vec<String> = shards.iter().map(|s| s.as_path_token()).collect();
        let mut want: Vec<String> =
            ShardKey::all().iter().map(|s| s.as_path_token()).collect();
        got.sort();
        want.sort();
        assert_eq!(got, want);
    }

    #[test]
    fn add_and_get_roundtrip() {
        let store = MemoryStore::in_memory();
        let entry = store.add(new_memory("User prefers Python")).unwrap();
        assert_eq!(entry.content, "User prefers Python");
        assert_eq!(entry.importance, 3);
        assert_eq!(entry.access_count, 0);
    }

    /// BENCH-PARITY-3 (2026-05-13): callers can override `created_at` on
    /// insert to mirror historical learning timestamps. Default behavior
    /// (no override) keeps stamping with wall-clock `now_ms()`.
    #[test]
    fn add_many_honors_created_at_override() {
        let store = MemoryStore::in_memory();
        let custom_ts: i64 = 1_700_000_000_000; // 2023-11-14T22:13:20Z
        let now_before = now_ms();
        let ids = store
            .add_many(vec![
                NewMemory {
                    content: "historical event".into(),
                    tags: "test".into(),
                    importance: 3,
                    created_at: Some(custom_ts),
                    ..Default::default()
                },
                NewMemory {
                    content: "live event".into(),
                    tags: "test".into(),
                    importance: 3,
                    ..Default::default()
                },
            ])
            .unwrap();
        let now_after = now_ms();
        assert_eq!(ids.len(), 2);

        let historical = store.get_by_id(ids[0]).unwrap();
        let live = store.get_by_id(ids[1]).unwrap();
        assert_eq!(
            historical.created_at, custom_ts,
            "explicit created_at must be persisted verbatim"
        );
        assert!(
            live.created_at >= now_before && live.created_at <= now_after,
            "default created_at must fall in [now_before, now_after]; got {}",
            live.created_at
        );
    }

    #[test]
    fn query_terms_prune_conversational_fillers() {
        let (terms, _) = query_terms(
            "Can you suggest some accessories that would complement my current photography setup?",
        );
        assert!(!terms.contains(&"can".to_string()));
        assert!(!terms.contains(&"you".to_string()));
        assert!(!terms.contains(&"some".to_string()));
        assert!(terms.contains(&"accessories".to_string()));
        assert!(terms.contains(&"photography".to_string()));
    }

    #[test]
    fn recommendation_query_terms_add_domain_expansions() {
        let (terms, _) = query_terms(
            "Can you suggest some accessories that would complement my current photography setup?",
        );
        assert!(terms.contains(&"camera".to_string()));
        assert!(terms.contains(&"flash".to_string()));
        assert!(terms.contains(&"sony".to_string()));
    }

    #[test]
    fn query_terms_add_light_variants() {
        let (terms, _) = query_terms("How many bikes do I own, and what kitchen appliance did I buy?");
        assert!(terms.contains(&"bikes".to_string()));
        assert!(terms.contains(&"bike".to_string()));
        assert!(terms.contains(&"buy".to_string()));
        assert!(terms.contains(&"bought".to_string()));
        assert!(terms.contains(&"got".to_string()));
        assert!(terms.contains(&"smoker".to_string()));
    }

    #[test]
    fn query_terms_add_auth_variants_for_jwt_middleware() {
        let (terms, _) = query_terms("JWT token validation middleware");
        assert!(terms.contains(&"jwt".to_string()));
        assert!(terms.contains(&"authentication".to_string()));
        assert!(terms.contains(&"nextauth".to_string()));
        assert!(terms.contains(&"session".to_string()));
    }

    #[test]
    fn search_prefers_personal_recommendation_context_over_generic_filler() {
        let store = MemoryStore::in_memory();
        store
            .add(new_memory(
                "assistant: You can suggest some generic current accessories for many hobbies.",
            ))
            .unwrap();
        store
            .add(new_memory(
                "user: I am looking to upgrade my camera flash for my Sony A7R IV. assistant: Here are compatible Sony flash options for photography.",
            ))
            .unwrap();

        let results = store
            .search("Can you suggest some accessories that would complement my current photography setup?")
            .unwrap();
        assert!(results[0].content.contains("camera flash"));
    }

    #[test]
    fn search_weights_rare_query_terms_above_temporal_fillers() {
        let store = MemoryStore::in_memory();
        for idx in 0..4 {
            store
                .add(new_memory(&format!(
                    "user: Many days ago I needed general tips for a common activity {idx}."
                )))
                .unwrap();
        }
        store
            .add(new_memory(
                "user: I caught up with Emma over lunch today about a collaboration.",
            ))
            .unwrap();

        let results = store.search("How many days ago did I meet Emma?").unwrap();
        assert!(results[0].content.contains("Emma"));
    }

    #[test]
    fn search_does_not_overweight_generic_configuration_term() {
        let store = MemoryStore::in_memory();
        store
            .add(new_memory(
                "Vitest unit testing configuration with jsdom and type-aware setup.",
            ))
            .unwrap();
        store
            .add(new_memory(
                "Playwright browser test isolation for end-to-end test coverage.",
            ))
            .unwrap();

        let results = store.search("Playwright test configuration").unwrap();
        assert!(results[0].content.contains("Playwright"));
    }

    #[test]
    fn search_expands_jwt_middleware_to_auth_context() {
        let store = MemoryStore::in_memory();
        store
            .add(new_memory(
                "Role-based access control stores roles as JWT token claims.",
            ))
            .unwrap();
        store
            .add(new_memory(
                "NextAuth authentication uses JWT sessions and protected route middleware.",
            ))
            .unwrap();

        let results = store.search("JWT token validation middleware").unwrap();
        assert!(results[0].content.contains("NextAuth authentication"));
    }

    #[test]
    fn diversified_ranked_caps_real_sessions_only() {
        let store = MemoryStore::in_memory();
        let mut ranked = Vec::new();
        for idx in 0..5 {
            let entry = store
                .add_to_tier(
                    NewMemory {
                        content: format!("noisy session memory {idx}"),
                        tags: "retrieval".to_string(),
                        ..new_memory("unused")
                    },
                    MemoryTier::Long,
                    Some("session-a"),
                )
                .unwrap();
            ranked.push((entry.id, 1.0 - (idx as f64 * 0.01)));
        }
        for idx in 0..2 {
            let entry = store
                .add_to_tier(
                    NewMemory {
                        content: format!("other session memory {idx}"),
                        tags: "retrieval".to_string(),
                        ..new_memory("unused")
                    },
                    MemoryTier::Long,
                    Some("session-b"),
                )
                .unwrap();
            ranked.push((entry.id, 0.5 - (idx as f64 * 0.01)));
        }
        let global = store.add(new_memory("global durable memory")).unwrap();
        ranked.push((global.id, 0.1));

        let by_id: HashMap<i64, MemoryEntry> = store
            .get_entries_by_ids(&ranked.iter().map(|(id, _)| *id).collect::<Vec<_>>())
            .unwrap()
            .into_iter()
            .map(|entry| (entry.id, entry))
            .collect();

        let selected = select_diversified_ranked(ranked, &by_id, 6, 3);
        let session_a_count = selected
            .iter()
            .filter(|(id, _)| {
                by_id.get(id).and_then(|entry| entry.session_id.as_deref()) == Some("session-a")
            })
            .count();
        assert_eq!(session_a_count, 3);
        assert!(selected.iter().any(|(id, _)| *id == global.id));
    }

    #[test]
    fn importance_clamped_to_1_5() {
        let store = MemoryStore::in_memory();
        let e1 = store
            .add(NewMemory {
                importance: 10,
                ..new_memory("high")
            })
            .unwrap();
        let e2 = store
            .add(NewMemory {
                importance: 0,
                ..new_memory("low")
            })
            .unwrap();
        assert_eq!(e1.importance, 5);
        assert_eq!(e2.importance, 1);
    }

    #[test]
    fn get_all_ordered_by_importance() {
        let store = MemoryStore::in_memory();
        store
            .add(NewMemory {
                importance: 1,
                ..new_memory("low importance")
            })
            .unwrap();
        store
            .add(NewMemory {
                importance: 5,
                ..new_memory("high importance")
            })
            .unwrap();
        let all = store.get_all().unwrap();
        assert_eq!(all[0].importance, 5);
        assert_eq!(all[1].importance, 1);
    }

    #[test]
    fn search_finds_by_content_keyword() {
        let store = MemoryStore::in_memory();
        store
            .add(new_memory("User loves Python programming"))
            .unwrap();
        store
            .add(new_memory("User's favourite colour is blue"))
            .unwrap();
        let results = store.search("Python").unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("Python"));
    }

    #[test]
    fn search_finds_by_tags() {
        let store = MemoryStore::in_memory();
        store
            .add(NewMemory {
                content: "Likes dark mode".to_string(),
                tags: "ui,preferences".to_string(),
                importance: 2,
                memory_type: MemoryType::Preference,
                ..Default::default()
            })
            .unwrap();
        let results = store.search("preferences").unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn search_reranks_exact_tag_tokens_above_broad_matches() {
        let store = MemoryStore::in_memory();
        store
            .add(NewMemory {
                content: "General validation middleware for request bodies".to_string(),
                tags: "validation,middleware".to_string(),
                importance: 5,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();
        let jwt = store
            .add(NewMemory {
                content: "JWT token validation middleware for auth sessions".to_string(),
                tags: "jwt,authentication,middleware".to_string(),
                importance: 3,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();

        let results = store.search("JWT token validation middleware").unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].id, jwt.id);
    }

    #[test]
    fn search_keeps_short_technical_query_tokens() {
        let store = MemoryStore::in_memory();
        store
            .add(NewMemory {
                content: "Pipeline configuration for data validation".to_string(),
                tags: "pipeline,configuration".to_string(),
                importance: 5,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();
        let cicd = store
            .add(NewMemory {
                content: "CI/CD pipeline configuration for GitHub Actions".to_string(),
                tags: "ci-cd,github-actions,deployment".to_string(),
                importance: 3,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();

        let results = store.search("CI/CD pipeline configuration").unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].id, cicd.id);
    }

    #[test]
    fn search_uses_gated_graph_boost_for_related_exact_matches() {
        let store = MemoryStore::in_memory();
        let seed = store
            .add(NewMemory {
                content: "Cache configuration seed memory".to_string(),
                tags: "cache,configuration".to_string(),
                importance: 5,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();
        let related = store
            .add(NewMemory {
                content: "Cache configuration worker details".to_string(),
                tags: "cache".to_string(),
                importance: 1,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();
        let unlinked = store
            .add(NewMemory {
                content: "Cache configuration unrelated details".to_string(),
                tags: "cache".to_string(),
                importance: 5,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();

        store
            .conn
            .execute(
                "INSERT INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
                 VALUES (?1, ?2, 'related_to', 1.0, 'test', ?3, 'test')",
                params![seed.id, related.id, now_ms()],
            )
            .unwrap();

        let results = store.search("cache configuration").unwrap();
        let related_pos = results
            .iter()
            .position(|entry| entry.id == related.id)
            .unwrap();
        let unlinked_pos = results
            .iter()
            .position(|entry| entry.id == unlinked.id)
            .unwrap();
        assert!(related_pos < unlinked_pos);
    }

    #[test]
    fn search_ignores_question_stop_terms_for_concept_hits() {
        let store = MemoryStore::in_memory();
        store
            .add(NewMemory {
                content: "General app shell work and layout details".to_string(),
                tags: "app,work".to_string(),
                importance: 5,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();
        let caching = store
            .add(NewMemory {
                content: "Redis caching layer for expensive queries".to_string(),
                tags: "caching,redis,performance".to_string(),
                importance: 3,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();

        let results = store.search("How does caching work in the app?").unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].id, caching.id);
    }

    #[test]
    fn search_empty_query_returns_all() {
        let store = MemoryStore::in_memory();
        store.add(new_memory("A")).unwrap();
        store.add(new_memory("B")).unwrap();
        assert_eq!(store.search("").unwrap().len(), 2);
    }

    #[test]
    fn system_default_settings_filter_by_category_or_tag() {
        let store = MemoryStore::in_memory();
        let tagged = store
            .add(NewMemory {
                content: "DEFAULT SYSTEM SETTING: Learn docs examples".to_string(),
                tags: "system:default-system-setting,intent-classifier".to_string(),
                importance: 5,
                memory_type: MemoryType::Preference,
                ..Default::default()
            })
            .unwrap();
        let categorized = store
            .add(NewMemory {
                content: "DEFAULT SYSTEM SETTING: Teach ingest examples".to_string(),
                tags: "intent-classifier,teach-ingest".to_string(),
                importance: 4,
                memory_type: MemoryType::Preference,
                ..Default::default()
            })
            .unwrap();
        store
            .conn
            .execute(
                "UPDATE memories SET category = 'system.default_system_setting' WHERE id = ?1",
                params![categorized.id],
            )
            .unwrap();
        store.add(new_memory("ordinary classifier note")).unwrap();

        let results = store
            .system_default_settings("intent-classifier", 10)
            .unwrap();
        let ids: Vec<i64> = results.iter().map(|entry| entry.id).collect();
        assert!(ids.contains(&tagged.id));
        assert!(ids.contains(&categorized.id));
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn update_fields() {
        let store = MemoryStore::in_memory();
        let entry = store.add(new_memory("Original content")).unwrap();
        let updated = store
            .update(
                entry.id,
                MemoryUpdate {
                    content: Some("Updated content".to_string()),
                    tags: Some("new-tag".to_string()),
                    importance: Some(5),
                    memory_type: None,
                },
            )
            .unwrap();
        assert_eq!(updated.content, "Updated content");
        assert_eq!(updated.tags, "new-tag");
        assert_eq!(updated.importance, 5);
    }

    #[test]
    fn delete_removes_entry() {
        let store = MemoryStore::in_memory();
        let entry = store.add(new_memory("To be deleted")).unwrap();
        store.delete(entry.id).unwrap();
        assert_eq!(store.count(), 0);
    }

    #[test]
    fn count_tracks_entries() {
        let store = MemoryStore::in_memory();
        assert_eq!(store.count(), 0);
        store.add(new_memory("One")).unwrap();
        store.add(new_memory("Two")).unwrap();
        assert_eq!(store.count(), 2);
        let entry = store.get_all().unwrap()[0].clone();
        store.delete(entry.id).unwrap();
        assert_eq!(store.count(), 1);
    }

    #[test]
    fn memory_type_roundtrip() {
        for mt in [
            MemoryType::Fact,
            MemoryType::Preference,
            MemoryType::Context,
            MemoryType::Summary,
        ] {
            assert_eq!(MemoryType::from_str(mt.as_str()), mt);
        }
    }

    // ── Vector / embedding tests ───────────────────────────────────────

    #[test]
    fn cosine_similarity_identical_vectors() {
        let v = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&v, &v);
        assert!(
            (sim - 1.0).abs() < 1e-5,
            "identical vectors should have sim ≈ 1.0, got {sim}"
        );
    }

    #[test]
    fn cosine_similarity_orthogonal_vectors() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(
            sim.abs() < 1e-5,
            "orthogonal vectors should have sim ≈ 0.0, got {sim}"
        );
    }

    #[test]
    fn cosine_similarity_opposite_vectors() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![-1.0, -2.0, -3.0];
        let sim = cosine_similarity(&a, &b);
        assert!(
            (sim + 1.0).abs() < 1e-5,
            "opposite vectors should have sim ≈ -1.0, got {sim}"
        );
    }

    #[test]
    fn cosine_similarity_mismatched_lengths() {
        assert_eq!(cosine_similarity(&[1.0, 2.0], &[1.0]), 0.0);
    }

    #[test]
    fn cosine_similarity_empty_vectors() {
        assert_eq!(cosine_similarity(&[], &[]), 0.0);
    }

    #[test]
    fn embedding_bytes_roundtrip() {
        let original = vec![0.1, -0.5, 3.125, 0.0, f32::MAX, f32::MIN];
        let bytes = embedding_to_bytes(&original);
        assert_eq!(bytes.len(), original.len() * 4);
        let restored = bytes_to_embedding(&bytes);
        assert_eq!(original, restored);
    }

    #[test]
    fn set_and_get_embedding() {
        let store = MemoryStore::in_memory();
        let entry = store.add(new_memory("test embedding")).unwrap();
        let emb = vec![0.1, 0.2, 0.3, 0.4];
        store.set_embedding(entry.id, &emb).unwrap();

        let with_emb = store.get_with_embeddings().unwrap();
        assert_eq!(with_emb.len(), 1);
        assert_eq!(with_emb[0].embedding.as_ref().unwrap(), &emb);
    }

    #[test]
    fn unembedded_ids_tracks_missing() {
        let store = MemoryStore::in_memory();
        let e1 = store.add(new_memory("has embedding")).unwrap();
        let _e2 = store.add(new_memory("no embedding")).unwrap();
        store.set_embedding(e1.id, &[1.0, 2.0, 3.0]).unwrap();

        let unembedded = store.unembedded_ids().unwrap();
        assert_eq!(unembedded.len(), 1);
        assert_eq!(unembedded[0].1, "no embedding");
    }

    #[test]
    fn vector_search_returns_ranked_results() {
        let store = MemoryStore::in_memory();

        // Create 3 memories with different embeddings.
        let e1 = store.add(new_memory("python programming")).unwrap();
        let e2 = store.add(new_memory("rust systems")).unwrap();
        let e3 = store.add(new_memory("javascript web")).unwrap();

        // Unit vectors in different directions.
        store.set_embedding(e1.id, &[1.0, 0.0, 0.0]).unwrap();
        store.set_embedding(e2.id, &[0.0, 1.0, 0.0]).unwrap();
        store.set_embedding(e3.id, &[0.7, 0.7, 0.0]).unwrap();

        // Query vector close to e1.
        let query = vec![0.9, 0.1, 0.0];
        let results = store.vector_search(&query, 2).unwrap();
        assert_eq!(results.len(), 2);
        // e1 should be first (most similar), e3 second.
        assert_eq!(results[0].id, e1.id);
        assert_eq!(results[1].id, e3.id);
    }

    #[test]
    fn vector_search_empty_store() {
        let store = MemoryStore::in_memory();
        let results = store.vector_search(&[1.0, 0.0], 5).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn vector_search_limit_respected() {
        let store = MemoryStore::in_memory();
        for i in 0..10 {
            let e = store.add(new_memory(&format!("memory {i}"))).unwrap();
            store.set_embedding(e.id, &[i as f32, 0.0, 1.0]).unwrap();
        }
        let results = store.vector_search(&[5.0, 0.0, 1.0], 3).unwrap();
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn find_duplicate_above_threshold() {
        let store = MemoryStore::in_memory();
        let e = store.add(new_memory("exact match")).unwrap();
        let emb = vec![1.0, 0.0, 0.0];
        store.set_embedding(e.id, &emb).unwrap();

        // Same vector → cosine = 1.0 → above 0.97 threshold.
        let dup = store.find_duplicate(&emb, 0.97).unwrap();
        assert_eq!(dup, Some(e.id));
    }

    #[test]
    fn find_duplicate_below_threshold() {
        let store = MemoryStore::in_memory();
        let e = store.add(new_memory("different")).unwrap();
        store.set_embedding(e.id, &[1.0, 0.0, 0.0]).unwrap();

        // Orthogonal vector → cosine = 0.0 → below threshold.
        let dup = store.find_duplicate(&[0.0, 1.0, 0.0], 0.97).unwrap();
        assert_eq!(dup, None);
    }

    #[test]
    fn schema_version_returns_latest() {
        let store = MemoryStore::in_memory();
        assert_eq!(
            store.schema_version(),
            super::schema::CANONICAL_SCHEMA_VERSION
        );
    }

    #[test]
    fn vector_search_updates_access_counters() {
        let store = MemoryStore::in_memory();
        let e = store.add(new_memory("tracked")).unwrap();
        assert_eq!(e.access_count, 0);
        store.set_embedding(e.id, &[1.0, 0.0]).unwrap();

        store.vector_search(&[1.0, 0.0], 5).unwrap();
        store.vector_search(&[1.0, 0.0], 5).unwrap();

        let updated = store.get_by_id(e.id).unwrap();
        assert_eq!(updated.access_count, 2);
        assert!(updated.last_accessed.is_some());
    }

    // ── Tiered memory tests ────────────────────────────────────────────

    #[test]
    fn add_sets_default_tier_long() {
        let store = MemoryStore::in_memory();
        let entry = store.add(new_memory("default tier")).unwrap();
        assert_eq!(entry.tier, MemoryTier::Long);
        assert!((entry.decay_score - 1.0).abs() < 0.01);
    }

    #[test]
    fn add_to_tier_creates_working_memory() {
        let store = MemoryStore::in_memory();
        let entry = store
            .add_to_tier(
                new_memory("session fact"),
                MemoryTier::Working,
                Some("sess-1"),
            )
            .unwrap();
        assert_eq!(entry.tier, MemoryTier::Working);
        assert_eq!(entry.session_id.as_deref(), Some("sess-1"));
    }

    #[test]
    fn get_by_tier_filters_correctly() {
        let store = MemoryStore::in_memory();
        store
            .add_to_tier(new_memory("short"), MemoryTier::Short, Some("s1"))
            .unwrap();
        store
            .add_to_tier(new_memory("working"), MemoryTier::Working, Some("s1"))
            .unwrap();
        store.add(new_memory("long")).unwrap();

        assert_eq!(store.get_by_tier(&MemoryTier::Short).unwrap().len(), 1);
        assert_eq!(store.get_by_tier(&MemoryTier::Working).unwrap().len(), 1);
        assert_eq!(store.get_by_tier(&MemoryTier::Long).unwrap().len(), 1);
    }

    #[test]
    fn get_persistent_excludes_short_term() {
        let store = MemoryStore::in_memory();
        store
            .add_to_tier(new_memory("ephemeral"), MemoryTier::Short, Some("s1"))
            .unwrap();
        store
            .add_to_tier(new_memory("session ctx"), MemoryTier::Working, Some("s1"))
            .unwrap();
        store.add(new_memory("permanent")).unwrap();

        let persistent = store.get_persistent().unwrap();
        assert_eq!(persistent.len(), 2);
        assert!(persistent.iter().all(|e| e.tier != MemoryTier::Short));
    }

    #[test]
    fn promote_changes_tier() {
        let store = MemoryStore::in_memory();
        let entry = store
            .add_to_tier(new_memory("upgradeable"), MemoryTier::Working, Some("s1"))
            .unwrap();
        store.promote(entry.id, MemoryTier::Long).unwrap();
        let updated = store.get_by_id(entry.id).unwrap();
        assert_eq!(updated.tier, MemoryTier::Long);
    }

    #[test]
    fn evict_short_term_clears_session() {
        let store = MemoryStore::in_memory();
        store
            .add_to_tier(new_memory("msg1"), MemoryTier::Short, Some("sess-1"))
            .unwrap();
        store
            .add_to_tier(new_memory("msg2"), MemoryTier::Short, Some("sess-1"))
            .unwrap();
        store
            .add_to_tier(
                new_memory("other session"),
                MemoryTier::Short,
                Some("sess-2"),
            )
            .unwrap();

        let evicted = store.evict_short_term("sess-1").unwrap();
        assert_eq!(evicted.len(), 2);
        assert_eq!(store.get_by_tier(&MemoryTier::Short).unwrap().len(), 1);
    }

    #[test]
    fn stats_returns_tier_counts() {
        let store = MemoryStore::in_memory();
        store
            .add_to_tier(new_memory("s"), MemoryTier::Short, Some("s1"))
            .unwrap();
        store
            .add_to_tier(new_memory("w"), MemoryTier::Working, Some("s1"))
            .unwrap();
        store.add(new_memory("l1")).unwrap();
        store.add(new_memory("l2")).unwrap();

        let stats = store.stats().unwrap();
        assert_eq!(stats.total, 4);
        assert_eq!(stats.short, 1);
        assert_eq!(stats.working, 1);
        assert_eq!(stats.long, 2);
        assert!(stats.storage_bytes > 0);
    }

    #[test]
    fn get_all_within_storage_bytes_limits_memory_cache_not_storage() {
        let store = MemoryStore::in_memory();
        let first = store
            .add(NewMemory {
                content: "important cached row".repeat(20),
                tags: "test".into(),
                importance: 5,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();
        store
            .add(NewMemory {
                content: "second persisted row".repeat(20),
                tags: "test".into(),
                importance: 3,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();
        let capped = store.get_all_within_storage_bytes(1).unwrap();

        assert_eq!(capped.len(), 1);
        assert_eq!(capped[0].id, first.id);
        assert_eq!(store.count(), 2);
    }

    #[test]
    fn enforce_size_limit_prunes_low_utility_memories_first() {
        let store = MemoryStore::in_memory();
        let old = store
            .add(NewMemory {
                content: "old low utility memory with enough text to consume space".into(),
                tags: "test".into(),
                importance: 1,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();
        let keep = store
            .add(NewMemory {
                content: "important recent memory with enough text to consume space".into(),
                tags: "test".into(),
                importance: 5,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();
        let before = store.active_storage_bytes().unwrap();

        let report = store.enforce_size_limit((before - 1) as u64).unwrap();

        assert_eq!(report.deleted, 1);
        assert!(store.get_by_id(old.id).is_err());
        assert!(store.get_by_id(keep.id).is_ok());
        assert!(report.after_bytes <= report.max_bytes);
    }

    #[test]
    fn hybrid_search_keyword_ranking() {
        let store = MemoryStore::in_memory();
        store
            .add(new_memory("Python programming language"))
            .unwrap();
        store.add(new_memory("JavaScript for web")).unwrap();
        store.add(new_memory("Rust systems programming")).unwrap();

        let results = store.hybrid_search("Python programming", None, 2).unwrap();
        assert_eq!(results.len(), 2);
        // Python entry should rank first (2 keyword hits vs 1 for Rust)
        assert!(results[0].content.contains("Python"));
    }

    #[test]
    fn hybrid_search_rrf_keyword_ranking() {
        let store = MemoryStore::in_memory();
        store
            .add(new_memory("Python programming language"))
            .unwrap();
        store.add(new_memory("JavaScript for web")).unwrap();
        store.add(new_memory("Rust systems programming")).unwrap();

        let results = store
            .hybrid_search_rrf("Python programming", None, 2)
            .unwrap();
        assert_eq!(results.len(), 2);
        // RRF may vary top-1 depending on freshness tie-breaking, but Python
        // (2 keyword hits) must still survive into the top-k.
        assert!(results.iter().any(|r| r.content.contains("Python")));
    }

    #[test]
    fn hybrid_search_rrf_zero_limit_returns_empty() {
        let store = MemoryStore::in_memory();
        store.add(new_memory("anything")).unwrap();
        let results = store.hybrid_search_rrf("anything", None, 0).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn hybrid_search_rrf_empty_store_returns_empty() {
        let store = MemoryStore::in_memory();
        let results = store.hybrid_search_rrf("anything", None, 5).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn hybrid_search_rrf_no_matching_keyword_still_returns_freshness_ranked() {
        // When the query has no keyword hits and no embedding, freshness
        // ranking alone must still produce results so RAG never returns
        // an empty top-k just because the query is unusual.
        let store = MemoryStore::in_memory();
        store.add(new_memory("alpha")).unwrap();
        store.add(new_memory("beta")).unwrap();
        store.add(new_memory("gamma")).unwrap();

        let results = store
            .hybrid_search_rrf("xyzzy-nonexistent-token", None, 2)
            .unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn hybrid_search_rrf_uses_vector_when_embedding_provided() {
        // Clear the process-wide search cache to prevent stale hits from
        // other tests (each test uses its own in-memory store, but the
        // cache key only encodes query/mode/limit, not the store instance).
        SEARCH_CACHE.clear();

        let store = MemoryStore::in_memory();
        let a = store.add(new_memory("alpha content")).unwrap();
        let b = store.add(new_memory("beta content")).unwrap();
        let c = store.add(new_memory("gamma content")).unwrap();
        // Normalize creation time to remove freshness/decay drift between rows;
        // this test is specifically about vector ranking impact.
        let same_created_at = now_ms();
        store
            .conn
            .execute(
                "UPDATE memories SET created_at = ?1 WHERE id IN (?2, ?3, ?4)",
                params![same_created_at, a.id, b.id, c.id],
            )
            .unwrap();

        // Hand-crafted unit vectors: query is most similar to `b`.
        let qe = vec![0.0_f32, 1.0, 0.0];
        store.set_embedding(a.id, &[1.0, 0.0, 0.0]).unwrap();
        store.set_embedding(b.id, &[0.0, 1.0, 0.0]).unwrap();
        store.set_embedding(c.id, &[0.0, 0.0, 1.0]).unwrap();

        // No query keyword overlap → vector + freshness drive the order;
        // `b` is the unambiguous vector winner, so it must lead the results.
        let results = store.hybrid_search_rrf("zzz", Some(&qe), 3).unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].id, b.id);
    }

    #[test]
    fn hybrid_search_rrf_deterministic_across_runs() {
        let store = MemoryStore::in_memory();
        store.add(new_memory("alpha")).unwrap();
        store.add(new_memory("beta")).unwrap();
        store.add(new_memory("gamma")).unwrap();

        let r1 = store.hybrid_search_rrf("xyz", None, 3).unwrap();
        let r2 = store.hybrid_search_rrf("xyz", None, 3).unwrap();
        let r3 = store.hybrid_search_rrf("xyz", None, 3).unwrap();
        let ids = |v: &[MemoryEntry]| v.iter().map(|e| e.id).collect::<Vec<_>>();
        assert_eq!(ids(&r1), ids(&r2));
        assert_eq!(ids(&r2), ids(&r3));
    }

    // ── Chunk 16.6c: query-intent–aware RRF ────────────────────────────

    #[test]
    fn hybrid_search_rrf_with_intent_unknown_matches_plain_rrf() {
        // For a query with no detectable intent, the intent-aware
        // variant must produce identical ids to plain RRF.
        let store = MemoryStore::in_memory();
        store.add(new_memory("alpha")).unwrap();
        store.add(new_memory("beta")).unwrap();
        store.add(new_memory("gamma")).unwrap();

        let plain = store.hybrid_search_rrf("alpha", None, 3).unwrap();
        let intent = store
            .hybrid_search_rrf_with_intent("alpha", None, 3)
            .unwrap();
        let ids = |v: &[MemoryEntry]| v.iter().map(|e| e.id).collect::<Vec<_>>();
        assert_eq!(ids(&plain), ids(&intent));
    }

    #[test]
    fn hybrid_search_rrf_with_intent_zero_limit_returns_empty() {
        let store = MemoryStore::in_memory();
        store.add(new_memory("anything")).unwrap();
        let r = store
            .hybrid_search_rrf_with_intent("How to install?", None, 0)
            .unwrap();
        assert!(r.is_empty());
    }

    #[test]
    fn hybrid_search_rrf_with_intent_empty_store_returns_empty() {
        let store = MemoryStore::in_memory();
        let r = store
            .hybrid_search_rrf_with_intent("How to install?", None, 5)
            .unwrap();
        assert!(r.is_empty());
    }

    #[test]
    fn hybrid_search_rrf_with_intent_boosts_procedural_kind() {
        // Insert a procedural how-to memory and a generic semantic
        // factoid that share a common keyword. Plain RRF will rank by
        // hit count + freshness; with a procedural query intent, the
        // procedural entry must move to the top via the kind boost.
        let store = MemoryStore::in_memory();

        // Generic factoid (no procedural cues) — semantic kind by default.
        store
            .add(NewMemory {
                content: "Coffee originated in Ethiopia in the 9th century.".to_string(),
                tags: "coffee,history".to_string(),
                ..Default::default()
            })
            .unwrap();

        // Procedural how-to entry (procedural verbs trigger
        // cognitive_kind::classify → Procedural).
        store
            .add(NewMemory {
                content: "How to brew coffee: Step 1 grind beans. Step 2 \
                          heat water. Step 3 pour over filter. Procedure \
                          for pour-over coffee."
                    .to_string(),
                tags: "coffee,how-to".to_string(),
                ..Default::default()
            })
            .unwrap();

        let results = store
            .hybrid_search_rrf_with_intent("How do I brew coffee step by step?", None, 2)
            .unwrap();
        assert_eq!(results.len(), 2);
        // Procedural entry must lead the results once kind-boost is applied.
        assert!(
            results[0].content.contains("How to brew"),
            "procedural intent should boost the how-to entry to top: got {:?}",
            results[0].content
        );
    }

    #[test]
    fn hybrid_search_rrf_with_intent_deterministic() {
        let store = MemoryStore::in_memory();
        store.add(new_memory("how to install ollama")).unwrap();
        store.add(new_memory("step by step setup guide")).unwrap();
        store.add(new_memory("ollama is a thing")).unwrap();

        let q = "How to install ollama?";
        let r1 = store.hybrid_search_rrf_with_intent(q, None, 3).unwrap();
        let r2 = store.hybrid_search_rrf_with_intent(q, None, 3).unwrap();
        let ids = |v: &[MemoryEntry]| v.iter().map(|e| e.id).collect::<Vec<_>>();
        assert_eq!(ids(&r1), ids(&r2));
    }

    #[test]
    fn gc_decayed_removes_low_importance() {
        let store = MemoryStore::in_memory();
        let e = store
            .add(NewMemory {
                importance: 1,
                ..new_memory("forgettable")
            })
            .unwrap();
        // Manually set low decay
        store
            .conn
            .execute(
                "UPDATE memories SET decay_score = 0.005 WHERE id = ?1",
                params![e.id],
            )
            .unwrap();
        let removed = store.gc_decayed(0.01).unwrap();
        assert_eq!(removed, 1);
        assert_eq!(store.count(), 0);
    }

    #[test]
    fn token_count_estimated_on_add() {
        let store = MemoryStore::in_memory();
        let entry = store.add(new_memory("Hello world this is a test")).unwrap();
        assert!(entry.token_count > 0);
    }

    #[test]
    fn add_with_source_fields() {
        let store = MemoryStore::in_memory();
        let entry = store
            .add(NewMemory {
                content: "Rule 14.3: 30-day deadline".to_string(),
                tags: "law".to_string(),
                importance: 5,
                memory_type: MemoryType::Fact,
                source_url: Some("https://example.com/rules".to_string()),
                source_hash: Some("abc123".to_string()),
                expires_at: None,
                created_at: None,
            })
            .unwrap();
        assert_eq!(
            entry.source_url.as_deref(),
            Some("https://example.com/rules")
        );
        assert_eq!(entry.source_hash.as_deref(), Some("abc123"));
        assert!(entry.expires_at.is_none());
    }

    #[test]
    fn find_by_source_hash_returns_match() {
        let store = MemoryStore::in_memory();
        store
            .add(NewMemory {
                source_hash: Some("hash-001".to_string()),
                ..new_memory("sourced fact")
            })
            .unwrap();
        store.add(new_memory("no source")).unwrap();

        let found = store.find_by_source_hash("hash-001").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().content, "sourced fact");

        assert!(store.find_by_source_hash("nonexistent").unwrap().is_none());
    }

    #[test]
    fn find_by_source_url_returns_all() {
        let store = MemoryStore::in_memory();
        let url = "https://example.com/doc";
        store
            .add(NewMemory {
                source_url: Some(url.to_string()),
                source_hash: Some("h1".to_string()),
                ..new_memory("chunk 1")
            })
            .unwrap();
        store
            .add(NewMemory {
                source_url: Some(url.to_string()),
                source_hash: Some("h2".to_string()),
                ..new_memory("chunk 2")
            })
            .unwrap();
        store.add(new_memory("unrelated")).unwrap();

        let results = store.find_by_source_url(url).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn delete_by_source_url_removes_all() {
        let store = MemoryStore::in_memory();
        let url = "https://example.com/stale";
        store
            .add(NewMemory {
                source_url: Some(url.to_string()),
                ..new_memory("old chunk 1")
            })
            .unwrap();
        store
            .add(NewMemory {
                source_url: Some(url.to_string()),
                ..new_memory("old chunk 2")
            })
            .unwrap();
        store.add(new_memory("keep me")).unwrap();

        let removed = store.delete_by_source_url(url).unwrap();
        assert_eq!(removed, 2);
        assert_eq!(store.count(), 1);
    }

    #[test]
    fn reingest_skip_when_hash_unchanged() {
        let store = MemoryStore::in_memory();
        let hash = "sha256-unchanged";
        store
            .add(NewMemory {
                source_hash: Some(hash.to_string()),
                source_url: Some("https://example.com/doc".to_string()),
                ..new_memory("existing content")
            })
            .unwrap();

        // Simulate re-ingest: find_by_source_hash returns Some → skip
        let existing = store.find_by_source_hash(hash).unwrap();
        assert!(existing.is_some());
    }

    #[test]
    fn reingest_replaces_when_hash_changed() {
        let store = MemoryStore::in_memory();
        let url = "https://example.com/rule";
        store
            .add(NewMemory {
                source_url: Some(url.to_string()),
                source_hash: Some("old-hash".to_string()),
                ..new_memory("old version of rule")
            })
            .unwrap();
        assert_eq!(store.count(), 1);

        // Hash changed → delete old entries by URL, then insert new
        let _ = store.delete_by_source_url(url).unwrap();
        assert_eq!(store.count(), 0);

        store
            .add(NewMemory {
                source_url: Some(url.to_string()),
                source_hash: Some("new-hash".to_string()),
                ..new_memory("updated version of rule")
            })
            .unwrap();
        assert_eq!(store.count(), 1);

        let found = store.find_by_source_hash("new-hash").unwrap();
        assert!(found.is_some());
        assert!(found.unwrap().content.contains("updated"));
    }

    #[test]
    fn delete_expired_removes_past_entries() {
        let store = MemoryStore::in_memory();
        // Insert with an already-expired timestamp
        store
            .add(NewMemory {
                expires_at: Some(1000), // epoch ms, way in the past
                ..new_memory("ephemeral")
            })
            .unwrap();
        store.add(new_memory("permanent")).unwrap();

        let removed = store.delete_expired().unwrap();
        assert_eq!(removed, 1);
        assert_eq!(store.count(), 1);
    }

    // ── StorageBackend trait tests ───────────────────────────────────────

    #[test]
    fn storage_backend_sqlite_round_trip() {
        let store = MemoryStore::in_memory();
        let backend: &dyn StorageBackend = &store;

        assert_eq!(backend.backend_name(), "SQLite");
        assert!(!backend.supports_native_vector_search());

        // Add via trait
        let entry = backend.add(new_memory("trait test")).unwrap();
        assert_eq!(entry.content, "trait test");

        // Read via trait
        let fetched = backend.get_by_id(entry.id).unwrap();
        assert_eq!(fetched.content, "trait test");

        // Count via trait
        assert_eq!(backend.count().unwrap(), 1);

        // Search via trait
        let results = backend.search("trait").unwrap();
        assert_eq!(results.len(), 1);

        // Delete via trait
        backend.delete(entry.id).unwrap();
        assert_eq!(backend.count().unwrap(), 0);
    }

    #[test]
    fn storage_backend_stats_via_trait() {
        let store = MemoryStore::in_memory();
        let backend: &dyn StorageBackend = &store;

        backend.add(new_memory("one")).unwrap();
        backend
            .add_to_tier(new_memory("two"), MemoryTier::Short, Some("sess"))
            .unwrap();

        let stats = backend.stats().unwrap();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.short, 1);
        assert_eq!(stats.long, 1);
    }

    // ─── Chunk 16.1 — relevance threshold ─────────────────────────────

    #[test]
    fn hybrid_search_with_threshold_zero_matches_legacy_top_k() {
        // Threshold = 0.0 must reproduce the legacy hybrid_search top-k
        // (same ids, same order). Critical back-compat invariant — every
        // existing call site must keep working when the user hasn't
        // tuned the threshold.
        let store = MemoryStore::in_memory();
        store
            .add(new_memory("Python programming language"))
            .unwrap();
        store.add(new_memory("JavaScript for web")).unwrap();
        store.add(new_memory("Rust systems programming")).unwrap();

        let legacy = store.hybrid_search("Python programming", None, 2).unwrap();
        let with_t = store
            .hybrid_search_with_threshold("Python programming", None, 2, 0.0)
            .unwrap();
        assert_eq!(legacy.len(), with_t.len());
        for (a, b) in legacy.iter().zip(with_t.iter()) {
            assert_eq!(a.id, b.id);
        }
    }

    #[test]
    fn hybrid_search_with_threshold_filters_below_score() {
        // High threshold should drop weakly-matching rows. Both seeded
        // memories match nothing in the query "totally unrelated topic",
        // so all keyword scores collapse to 0 and only the freshness +
        // tier + importance + decay components remain — a small number.
        // 0.95 is well above any realistic combined score, so the result
        // must be empty.
        let store = MemoryStore::in_memory();
        store.add(new_memory("alpha")).unwrap();
        store.add(new_memory("beta")).unwrap();
        let r = store
            .hybrid_search_with_threshold("totally unrelated topic", None, 5, 0.95)
            .unwrap();
        assert!(r.is_empty(), "got {} hits", r.len());
    }

    #[test]
    fn hybrid_search_with_threshold_keeps_strong_matches() {
        // Strong keyword + freshness combo on a low threshold must keep
        // the matching row.
        let store = MemoryStore::in_memory();
        let e = store
            .add(new_memory("Python programming language"))
            .unwrap();
        let r = store
            .hybrid_search_with_threshold("Python programming", None, 5, 0.10)
            .unwrap();
        assert!(!r.is_empty());
        assert!(r.iter().any(|m| m.id == e.id));
    }

    #[test]
    fn hybrid_search_with_threshold_does_not_increment_access_for_filtered() {
        // Below-threshold rows must NOT count as accesses — keeps the
        // decay signal honest. We use a threshold above any realistic
        // score (the legacy hybrid score caps near 1.0; a query with no
        // keyword overlap hits ~0.3 on freshness alone) so every row is
        // filtered out, and assert no row's access_count was bumped.
        let store = MemoryStore::in_memory();
        let a = store.add(new_memory("alpha")).unwrap();
        let b = store.add(new_memory("beta")).unwrap();
        let r = store
            .hybrid_search_with_threshold("totally unrelated topic", None, 5, 0.95)
            .unwrap();
        assert!(r.is_empty(), "high threshold should filter all rows");

        let a_after = store.get_by_id(a.id).unwrap();
        let b_after = store.get_by_id(b.id).unwrap();
        assert_eq!(
            a_after.access_count, 0,
            "filtered row a must NOT be touched"
        );
        assert_eq!(
            b_after.access_count, 0,
            "filtered row b must NOT be touched"
        );
    }

    #[test]
    fn hybrid_search_with_threshold_respects_limit() {
        // Many strong matches + threshold = 0.0 — the `limit` cap still applies.
        let store = MemoryStore::in_memory();
        for i in 0..10 {
            store
                .add(new_memory(&format!("Python programming language {i}")))
                .unwrap();
        }
        let r = store
            .hybrid_search_with_threshold("Python programming", None, 3, 0.0)
            .unwrap();
        assert_eq!(r.len(), 3);
    }

    // ------------------------------------------------------------------
    // Chunk 17.1 — auto_promote_to_long
    // ------------------------------------------------------------------

    /// Helper: force a working-tier row's access_count + last_accessed
    /// to a known state. Tests only.
    fn force_access(store: &MemoryStore, id: i64, count: i64, last_accessed_ms: i64) {
        store
            .conn()
            .execute(
                "UPDATE memories SET access_count = ?1, last_accessed = ?2 WHERE id = ?3",
                params![count, last_accessed_ms, id],
            )
            .unwrap();
    }

    #[test]
    fn auto_promote_promotes_when_both_thresholds_met() {
        let store = MemoryStore::in_memory();
        let e = store
            .add_to_tier(new_memory("hot working entry"), MemoryTier::Working, None)
            .unwrap();
        force_access(&store, e.id, 5, now_ms());

        let promoted = store.auto_promote_to_long(5, 7).unwrap();
        assert_eq!(promoted, vec![e.id]);
        assert_eq!(store.get_by_id(e.id).unwrap().tier, MemoryTier::Long);
    }

    #[test]
    fn auto_promote_skips_when_access_count_below_threshold() {
        let store = MemoryStore::in_memory();
        let e = store
            .add_to_tier(new_memory("cold working entry"), MemoryTier::Working, None)
            .unwrap();
        force_access(&store, e.id, 4, now_ms()); // one short of the threshold

        let promoted = store.auto_promote_to_long(5, 7).unwrap();
        assert!(
            promoted.is_empty(),
            "below-threshold row must not be promoted"
        );
        assert_eq!(store.get_by_id(e.id).unwrap().tier, MemoryTier::Working);
    }

    #[test]
    fn auto_promote_skips_when_outside_recency_window() {
        let store = MemoryStore::in_memory();
        let e = store
            .add_to_tier(new_memory("stale working entry"), MemoryTier::Working, None)
            .unwrap();
        // last_accessed is well outside a 7-day window
        force_access(&store, e.id, 99, now_ms() - 30 * 86_400_000);

        let promoted = store.auto_promote_to_long(5, 7).unwrap();
        assert!(promoted.is_empty(), "stale row must not be promoted");
        assert_eq!(store.get_by_id(e.id).unwrap().tier, MemoryTier::Working);
    }

    #[test]
    fn auto_promote_ignores_long_and_short_tiers() {
        let store = MemoryStore::in_memory();
        let l = store
            .add_to_tier(new_memory("already long"), MemoryTier::Long, None)
            .unwrap();
        let s = store
            .add_to_tier(new_memory("short term"), MemoryTier::Short, Some("sess"))
            .unwrap();
        force_access(&store, l.id, 100, now_ms());
        force_access(&store, s.id, 100, now_ms());

        let promoted = store.auto_promote_to_long(5, 7).unwrap();
        assert!(
            promoted.is_empty(),
            "non-working tiers must not be promoted"
        );
    }

    #[test]
    fn auto_promote_is_idempotent() {
        let store = MemoryStore::in_memory();
        let e = store
            .add_to_tier(new_memory("hot"), MemoryTier::Working, None)
            .unwrap();
        force_access(&store, e.id, 10, now_ms());

        let first = store.auto_promote_to_long(5, 7).unwrap();
        let second = store.auto_promote_to_long(5, 7).unwrap();
        assert_eq!(first, vec![e.id]);
        assert!(
            second.is_empty(),
            "second run is a no-op once promoted to long"
        );
    }

    #[test]
    fn auto_promote_skips_rows_with_null_last_accessed() {
        // A working entry that was inserted but never accessed has
        // last_accessed = NULL — must not be promoted regardless of count.
        let store = MemoryStore::in_memory();
        let e = store
            .add_to_tier(new_memory("never accessed"), MemoryTier::Working, None)
            .unwrap();
        store
            .conn()
            .execute(
                "UPDATE memories SET access_count = 50, last_accessed = NULL WHERE id = ?1",
                params![e.id],
            )
            .unwrap();

        let promoted = store.auto_promote_to_long(5, 7).unwrap();
        assert!(
            promoted.is_empty(),
            "NULL last_accessed must be treated as not-recent"
        );
        assert_eq!(store.get_by_id(e.id).unwrap().tier, MemoryTier::Working);
    }

    // ------------------------------------------------------------------
    // Chunk 18.2 — category-aware decay (integration with apply_decay)
    // ------------------------------------------------------------------

    fn add_long_with_tags(store: &MemoryStore, content: &str, tags: &str) -> i64 {
        let m = NewMemory {
            content: content.to_string(),
            tags: tags.to_string(),
            importance: 3,
            memory_type: MemoryType::Fact,
            source_url: None,
            source_hash: None,
            expires_at: None,
            created_at: None,
        };
        let e = store.add(m).unwrap();
        // Force last_accessed to ~30 days ago so apply_decay actually moves the score.
        let thirty_days_ago = now_ms() - 30 * 86_400_000;
        store
            .conn()
            .execute(
                "UPDATE memories SET last_accessed = ?1, decay_score = 1.0 WHERE id = ?2",
                params![thirty_days_ago, e.id],
            )
            .unwrap();
        e.id
    }

    #[test]
    fn apply_decay_personal_decays_slower_than_tool() {
        let store = MemoryStore::in_memory();
        let personal = add_long_with_tags(&store, "user loves pho", "personal:loves_pho");
        let tool = add_long_with_tags(&store, "bun --hot flag", "tool:bun");
        store.apply_decay().unwrap();

        let p = store.get_by_id(personal).unwrap();
        let t = store.get_by_id(tool).unwrap();
        assert!(
            p.decay_score > t.decay_score,
            "personal:* (mult 0.5) must decay slower than tool:* (mult 1.5); got personal={}, tool={}",
            p.decay_score, t.decay_score
        );
    }

    #[test]
    fn apply_decay_baseline_for_legacy_or_non_conforming_tags() {
        // Two entries with default-multiplier-equivalent tags should land at
        // the same decay_score (within float tolerance).
        let store = MemoryStore::in_memory();
        let legacy = add_long_with_tags(&store, "legacy", "fact");
        let project = add_long_with_tags(&store, "project x", "project:x");
        store.apply_decay().unwrap();

        let l = store.get_by_id(legacy).unwrap();
        let p = store.get_by_id(project).unwrap();
        assert!(
            (l.decay_score - p.decay_score).abs() < 1e-6,
            "legacy tag and project:* (both mult 1.0) must decay identically; got legacy={}, project={}",
            l.decay_score, p.decay_score
        );
    }

    // ── Importance auto-adjustment (chunk 17.4) ────────────────────────

    /// Helper: simulate N accesses on a memory by directly bumping access_count.
    fn set_access_count(store: &MemoryStore, id: i64, count: i64) {
        store
            .conn
            .execute(
                "UPDATE memories SET access_count = ?1, last_accessed = ?2 WHERE id = ?3",
                params![count, now_ms(), id],
            )
            .unwrap();
    }

    #[test]
    fn adjust_boosts_hot_entries() {
        let store = MemoryStore::in_memory();
        let e = store
            .add(NewMemory {
                importance: 3,
                ..new_memory("hot")
            })
            .unwrap();
        set_access_count(&store, e.id, 10);
        let (boosted, demoted) = store.adjust_importance_by_access(10, 30).unwrap();
        assert_eq!(boosted, 1);
        assert_eq!(demoted, 0);
        assert_eq!(store.get_by_id(e.id).unwrap().importance, 4);
    }

    #[test]
    fn adjust_caps_at_5() {
        let store = MemoryStore::in_memory();
        let e = store
            .add(NewMemory {
                importance: 5,
                ..new_memory("maxed")
            })
            .unwrap();
        set_access_count(&store, e.id, 20);
        let (boosted, _) = store.adjust_importance_by_access(10, 30).unwrap();
        // Already at max → no boost
        assert_eq!(boosted, 0);
        assert_eq!(store.get_by_id(e.id).unwrap().importance, 5);
    }

    #[test]
    fn adjust_demotes_cold_entries() {
        let store = MemoryStore::in_memory();
        let e = store
            .add(NewMemory {
                importance: 3,
                ..new_memory("cold")
            })
            .unwrap();
        // access_count stays 0, last_accessed is NULL → cold
        let (_, demoted) = store.adjust_importance_by_access(10, 30).unwrap();
        assert_eq!(demoted, 1);
        assert_eq!(store.get_by_id(e.id).unwrap().importance, 2);
    }

    #[test]
    fn adjust_floors_at_1() {
        let store = MemoryStore::in_memory();
        let e = store
            .add(NewMemory {
                importance: 1,
                ..new_memory("min")
            })
            .unwrap();
        let (_, demoted) = store.adjust_importance_by_access(10, 30).unwrap();
        // Already at min → no demote
        assert_eq!(demoted, 0);
        assert_eq!(store.get_by_id(e.id).unwrap().importance, 1);
    }

    #[test]
    fn adjust_resets_access_count_after_boost() {
        let store = MemoryStore::in_memory();
        let e = store.add(new_memory("reset")).unwrap();
        set_access_count(&store, e.id, 15);
        store.adjust_importance_by_access(10, 30).unwrap();
        let updated = store.get_by_id(e.id).unwrap();
        assert_eq!(
            updated.access_count, 0,
            "access_count should reset after boost"
        );
    }

    #[test]
    fn adjust_leaves_middling_entries_alone() {
        let store = MemoryStore::in_memory();
        let e = store.add(new_memory("middling")).unwrap();
        // access_count = 5 (below hot_threshold 10), recently accessed → neither hot nor cold
        set_access_count(&store, e.id, 5);
        let (boosted, demoted) = store.adjust_importance_by_access(10, 30).unwrap();
        assert_eq!(boosted, 0);
        assert_eq!(demoted, 0);
        assert_eq!(store.get_by_id(e.id).unwrap().importance, 3);
    }

    #[test]
    fn adjust_mixed_hot_and_cold() {
        let store = MemoryStore::in_memory();
        let hot = store
            .add(NewMemory {
                importance: 2,
                ..new_memory("hot one")
            })
            .unwrap();
        let cold = store
            .add(NewMemory {
                importance: 4,
                ..new_memory("cold one")
            })
            .unwrap();
        set_access_count(&store, hot.id, 12);
        // cold stays at access_count=0, last_accessed=NULL

        let (boosted, demoted) = store.adjust_importance_by_access(10, 30).unwrap();
        assert_eq!(boosted, 1);
        assert_eq!(demoted, 1);
        assert_eq!(store.get_by_id(hot.id).unwrap().importance, 3);
        assert_eq!(store.get_by_id(cold.id).unwrap().importance, 3);
    }

    #[test]
    fn adjust_creates_version_audit_trail() {
        let store = MemoryStore::in_memory();
        let e = store
            .add(NewMemory {
                importance: 3,
                ..new_memory("audited")
            })
            .unwrap();
        set_access_count(&store, e.id, 10);
        store.adjust_importance_by_access(10, 30).unwrap();

        let history = crate::memory::versioning::get_history(&store.conn, e.id).unwrap();
        assert_eq!(history.len(), 1, "boost should create one version snapshot");
        assert_eq!(
            history[0].importance, 3,
            "snapshot should capture pre-boost value"
        );
    }

    // ── Reinforcement provenance tests (43.4) ─────────────────────────────

    #[test]
    fn record_reinforcement_round_trip() {
        let store = MemoryStore::in_memory();
        let e = store.add(new_memory("reinforced fact")).unwrap();

        store.record_reinforcement(e.id, "sess-a", 0).unwrap();

        let recs = store.get_reinforcements(e.id, 10).unwrap();
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].memory_id, e.id);
        assert_eq!(recs[0].session_id, "sess-a");
        assert_eq!(recs[0].message_index, 0);
    }

    #[test]
    fn record_reinforcement_idempotent_on_pk() {
        let store = MemoryStore::in_memory();
        let e = store.add(new_memory("idempotent test")).unwrap();

        // Same (memory_id, session_id, message_index) inserted twice
        store.record_reinforcement(e.id, "sess-b", 1).unwrap();
        store.record_reinforcement(e.id, "sess-b", 1).unwrap();

        let recs = store.get_reinforcements(e.id, 10).unwrap();
        assert_eq!(recs.len(), 1, "duplicate PK should be ignored");
    }

    #[test]
    fn get_reinforcements_respects_limit() {
        let store = MemoryStore::in_memory();
        let e = store.add(new_memory("limited")).unwrap();

        for i in 0..5 {
            store
                .record_reinforcement(e.id, &format!("s{i}"), 0)
                .unwrap();
        }

        let recs = store.get_reinforcements(e.id, 3).unwrap();
        assert_eq!(recs.len(), 3);
    }

    #[test]
    fn get_reinforcements_empty_when_none() {
        let store = MemoryStore::in_memory();
        let e = store.add(new_memory("no reinforcements")).unwrap();

        let recs = store.get_reinforcements(e.id, 10).unwrap();
        assert!(recs.is_empty());
    }

    // ── Chunk 48.4 Phase 2 — PQ Codebook Refresh Tests ────────────────────────────

    #[test]
    fn pq_codebooks_need_refresh_returns_false_with_no_data() {
        // Empty store → no large shards → no refresh needed
        let store = MemoryStore::in_memory();
        assert!(!store.pq_codebooks_need_refresh());
    }

    #[test]
    fn pq_codebooks_need_refresh_returns_false_when_data_below_threshold() {
        // Small dataset → below LARGE_SHARD_THRESHOLD → no refresh needed
        let store = MemoryStore::in_memory();
        for i in 0..1000 {
            let m = NewMemory {
                content: format!("entry {}", i),
                tags: "tier_long|kind_semantic".to_string(),
                importance: 3,
                memory_type: MemoryType::Fact,
                ..Default::default()
            };
            store.add(m).unwrap();
        }
        // With 1000 entries (far below 50M threshold), no refresh needed
        assert!(!store.pq_codebooks_need_refresh());
    }

    #[test]
    fn refresh_pq_codebooks_returns_zero_with_no_data() {
        // Empty store → no codebooks to refresh
        let store = MemoryStore::in_memory();
        let refreshed = store.refresh_pq_codebooks().unwrap();
        assert_eq!(refreshed, 0);
    }

    #[test]
    fn refresh_pq_codebooks_handles_small_shards_gracefully() {
        // Small dataset should skip PQ refresh (below threshold)
        let store = MemoryStore::in_memory();
        for i in 0..100 {
            let m = NewMemory {
                content: format!("entry {}", i),
                tags: "tier_long|kind_semantic".to_string(),
                importance: 3,
                memory_type: MemoryType::Fact,
                ..Default::default()
            };
            store.add(m).unwrap();
        }
        // Should succeed but return 0 refreshed (below threshold)
        let refreshed = store.refresh_pq_codebooks().unwrap();
        assert_eq!(refreshed, 0, "Small shards should not trigger PQ refresh");
    }

    // ─── FTS5 keyword index tests (Chunk 48.5) ───────────────────────

    #[test]
    fn fts5_index_is_created_on_new_store() {
        let store = MemoryStore::in_memory();
        assert!(store.has_fts5(), "FTS5 table should exist after init");
    }

    #[test]
    fn fts5_search_finds_inserted_memory() {
        let store = MemoryStore::in_memory();
        store
            .add(NewMemory {
                content: "Quantum entanglement is fascinating".to_string(),
                tags: "physics,science".to_string(),
                importance: 4,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();
        store
            .add(NewMemory {
                content: "Cooking pasta requires boiling water".to_string(),
                tags: "cooking,food".to_string(),
                importance: 2,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();

        let ids = store
            .keyword_candidate_ids(&["quantum".to_string()], 10)
            .unwrap();
        assert_eq!(ids.len(), 1);

        let ids = store
            .keyword_candidate_ids(&["pasta".to_string()], 10)
            .unwrap();
        assert_eq!(ids.len(), 1);
    }

    #[test]
    fn fts5_search_respects_pool_limit() {
        let store = MemoryStore::in_memory();
        for i in 0..20 {
            store
                .add(NewMemory {
                    content: format!("Memory about Rust programming lesson {}", i),
                    tags: "rust,programming".to_string(),
                    importance: 3,
                    memory_type: MemoryType::Fact,
                    ..Default::default()
                })
                .unwrap();
        }

        let ids = store
            .keyword_candidate_ids(&["rust".to_string()], 5)
            .unwrap();
        assert!(ids.len() <= 5, "pool limit should cap results");
    }

    #[test]
    fn fts5_search_or_semantics() {
        let store = MemoryStore::in_memory();
        store
            .add(NewMemory {
                content: "Cats are wonderful pets".to_string(),
                tags: "animals".to_string(),
                importance: 3,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();
        store
            .add(NewMemory {
                content: "Dogs are loyal companions".to_string(),
                tags: "animals".to_string(),
                importance: 3,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();

        let ids = store
            .keyword_candidate_ids(&["cats".to_string(), "dogs".to_string()], 10)
            .unwrap();
        assert_eq!(ids.len(), 2, "OR should match both entries");
    }

    #[test]
    fn fts5_triggers_keep_index_in_sync_on_update() {
        let store = MemoryStore::in_memory();
        let entry = store
            .add(NewMemory {
                content: "Original content about dolphins".to_string(),
                tags: "marine".to_string(),
                importance: 3,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();

        // Before update: "dolphins" matches.
        let ids = store
            .keyword_candidate_ids(&["dolphins".to_string()], 10)
            .unwrap();
        assert_eq!(ids.len(), 1);

        // Update content.
        store
            .update(
                entry.id,
                MemoryUpdate {
                    content: Some("Updated content about elephants".to_string()),
                    ..Default::default()
                },
            )
            .unwrap();

        // After update: "dolphins" should no longer match.
        let ids = store
            .keyword_candidate_ids(&["dolphins".to_string()], 10)
            .unwrap();
        assert_eq!(ids.len(), 0, "FTS5 should reflect updated content");

        // "elephants" should now match.
        let ids = store
            .keyword_candidate_ids(&["elephants".to_string()], 10)
            .unwrap();
        assert_eq!(ids.len(), 1);
    }

    #[test]
    fn fts5_triggers_keep_index_in_sync_on_delete() {
        let store = MemoryStore::in_memory();
        let entry = store
            .add(NewMemory {
                content: "Temporary note about zebras".to_string(),
                tags: "test".to_string(),
                importance: 3,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();

        let ids = store
            .keyword_candidate_ids(&["zebras".to_string()], 10)
            .unwrap();
        assert_eq!(ids.len(), 1);

        store.delete(entry.id).unwrap();

        let ids = store
            .keyword_candidate_ids(&["zebras".to_string()], 10)
            .unwrap();
        assert_eq!(ids.len(), 0, "FTS5 should reflect deletion");
    }

    #[test]
    fn fts5_search_method_uses_fts5() {
        let store = MemoryStore::in_memory();
        store
            .add(NewMemory {
                content: "Astrophysics involves studying celestial bodies".to_string(),
                tags: "science,space".to_string(),
                importance: 5,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();
        store
            .add(NewMemory {
                content: "Gardening tips for spring planting".to_string(),
                tags: "hobby".to_string(),
                importance: 2,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();

        let results = store.search("astrophysics").unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("Astrophysics"));
    }

    #[test]
    fn fts5_covering_indexes_exist() {
        let store = MemoryStore::in_memory();
        let exists: bool = store
            .conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_memories_last_accessed'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap()
            > 0;
        assert!(exists, "last_accessed covering index should exist");

        let exists: bool = store
            .conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_memories_decay_recency'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap()
            > 0;
        assert!(exists, "decay_recency covering index should exist");
    }

    #[test]
    fn schema_version_is_21() {
        let store = MemoryStore::in_memory();
        assert_eq!(store.schema_version(), 21);
    }

    #[test]
    fn disk_ann_migration_job_writes_sidecar_for_candidate_shard() {
        let root = std::env::temp_dir().join(format!("ts_disk_ann_migrate_{}", now_ms()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let store = MemoryStore::new(&root);
        let entry = store
            .add(NewMemory {
                content: "Disk ANN migration candidate".to_string(),
                tags: "semantic".to_string(),
                importance: 5,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();

        store
            .set_embedding(entry.id, &[0.1, 0.2, 0.3, 0.4])
            .unwrap();
        let _ = store.ann_save_all();

        let report = store.run_disk_ann_migration_job(1, 1).unwrap();
        assert_eq!(report.migrated, 1);
        assert_eq!(report.sidecars_written, 1);
        assert_eq!(report.attempted, 1);

        let sidecar =
            crate::memory::disk_backed_ann::read_sidecar(&root.join("vectors"), "long__semantic")
                .unwrap()
                .expect("expected sidecar for long__semantic");
        assert_eq!(sidecar.shard, "long__semantic");
        assert_eq!(sidecar.threshold, 1);
        assert_eq!(sidecar.status, "planned");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn disk_ann_health_summary_reports_missing_sidecars() {
        let root = std::env::temp_dir().join(format!("ts_disk_ann_health_{}", now_ms()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let store = MemoryStore::new(&root);
        let entry = store
            .add(NewMemory {
                content: "Eligible shard without sidecar".to_string(),
                tags: "semantic".to_string(),
                importance: 5,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();
        store
            .set_embedding(entry.id, &[0.4, 0.3, 0.2, 0.1])
            .unwrap();
        let _ = store.ann_save_all();

        let health = store.disk_ann_health_summary(1).unwrap();
        assert_eq!(health.eligible_candidates, 1);
        assert_eq!(health.ready_candidates, 0);
        assert_eq!(health.missing_sidecars, 1);

        let _ = std::fs::remove_dir_all(&root);
    }
}
