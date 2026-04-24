//! Audio-prosody persona analyzer (Chunk 14.6).
//!
//! When the user's ASR provider is configured, every text user-turn that
//! reached the brain originally came through speech (or at least could
//! have). This module derives **camera-free** prosody-style hints from
//! that text corpus — sentence length, exclamation / question density,
//! ALLCAPS ratio, filler-word usage, emoji density — and renders them
//! as a short hint block that gets folded into the Master-Echo
//! persona-extraction prompt next to the conversation transcript.
//!
//! The module is deliberately **pure** (no I/O, no time, no PRNG) so
//! every signal is exhaustively unit-testable. The thin wiring lives
//! in `commands/persona::extract_persona_from_brain`, which only
//! invokes us when `voice_config.asr_provider.is_some()`.
//!
//! ## Privacy
//!
//! - We **do not** read raw audio. The audio is already long-gone by
//!   the time it has been transcribed into the message log.
//! - We **do not** persist the hints. They are computed on demand at
//!   suggestion time and discarded once the LLM reply is parsed.
//! - The hints are deliberately coarse (single-word adjectives + at
//!   most a small set of quirk strings) so they read as friendly tone
//!   guidance rather than a forensic profile.

/// One concise bundle of prosody-derived suggestions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProsodyHints {
    /// Tone adjectives (e.g. `"energetic"`, `"inquisitive"`,
    /// `"emphatic"`, `"concise"`, `"elaborate"`, `"playful"`).
    /// Capped at 4 entries to stay under the persona-doc budget.
    pub tone: Vec<&'static str>,
    /// Optional pacing label (`"fast"`, `"measured"`, `"slow"`).
    pub pacing: Option<&'static str>,
    /// Quirks (e.g. `"often says 'like'"`, `"frequent emoji use"`).
    /// Capped at 3 entries.
    pub quirks: Vec<String>,
}

impl ProsodyHints {
    /// `true` when no signal at all was strong enough to record.
    /// `render_prosody_block` returns `None` in that case so callers
    /// can skip the prompt section entirely.
    pub fn is_empty(&self) -> bool {
        self.tone.is_empty() && self.pacing.is_none() && self.quirks.is_empty()
    }
}

/// Filler words / phrases — case-insensitive, whole-word matches. Order
/// matters only for the human-readable quirk string we emit below.
const FILLERS: &[&str] = &[
    "um", "uh", "er", "hmm",
    "like", "literally", "basically", "actually",
    "you know", "i mean", "kind of", "sort of",
];

/// Minimum number of utterances to bother analyzing. Below this,
/// signals are too noisy and we return [`ProsodyHints::default`].
pub const MIN_UTTERANCES: usize = 3;

/// Total-character cap to stop pathological inputs from dominating CPU
/// (1 MiB of user text would still finish well under 10 ms with this
/// analyzer's plain `chars()` walk, but the cap keeps the contract
/// explicit and matches `pack.rs`).
pub const MAX_INPUT_BYTES: usize = 1024 * 1024;

/// Analyze a list of user utterances (raw chat-message bodies, ALREADY
/// trimmed by the caller) and return the derived hints.
///
/// Empty / whitespace-only utterances are skipped. When fewer than
/// [`MIN_UTTERANCES`] real utterances are available — or when the
/// total input exceeds [`MAX_INPUT_BYTES`] — returns the empty hints
/// (callers should treat that as "no prosody section").
pub fn analyze_user_utterances(utterances: &[&str]) -> ProsodyHints {
    // Pre-filter to non-empty trimmed utterances so the divisors below
    // never see a blank.
    let cleaned: Vec<&str> = utterances
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if cleaned.len() < MIN_UTTERANCES {
        return ProsodyHints::default();
    }
    let total_bytes: usize = cleaned.iter().map(|s| s.len()).sum();
    if total_bytes > MAX_INPUT_BYTES {
        return ProsodyHints::default();
    }

    let mut hints = ProsodyHints::default();

    // ── Word / char totals ─────────────────────────────────────────
    let mut words_total: usize = 0;
    let mut alpha_letters_total: usize = 0;
    let mut allcaps_letters: usize = 0;
    let mut exclamation_marks: usize = 0;
    let mut question_marks: usize = 0;
    let mut emoji_count: usize = 0;
    let mut filler_hits: std::collections::HashMap<&'static str, usize> =
        std::collections::HashMap::new();

    for utter in &cleaned {
        words_total += utter.split_whitespace().count();
        for ch in utter.chars() {
            if ch.is_alphabetic() {
                alpha_letters_total += 1;
                if ch.is_uppercase() {
                    allcaps_letters += 1;
                }
            }
            match ch {
                '!' => exclamation_marks += 1,
                '?' => question_marks += 1,
                _ => {
                    if is_emoji(ch) {
                        emoji_count += 1;
                    }
                }
            }
        }
        // Whole-word filler scan (case-insensitive). `format!` is
        // cheap relative to the prompt that wraps this.
        let lc = utter.to_lowercase();
        for filler in FILLERS {
            if contains_whole_word(&lc, filler) {
                *filler_hits.entry(filler).or_insert(0) += 1;
            }
        }
    }

    let n = cleaned.len() as f64;
    let avg_words_per_utter = words_total as f64 / n;
    let exclam_per_utter = exclamation_marks as f64 / n;
    let question_per_utter = question_marks as f64 / n;
    let allcaps_ratio = if alpha_letters_total > 0 {
        allcaps_letters as f64 / alpha_letters_total as f64
    } else {
        0.0
    };

    // ── Tone signals ────────────────────────────────────────────────
    // Length: short → concise, long → elaborate. Thresholds picked
    // from common "tweet vs paragraph" intuition; recorded here for
    // future tuning rather than baked into prompts.
    if avg_words_per_utter <= 6.0 {
        push_unique(&mut hints.tone, "concise");
    } else if avg_words_per_utter >= 25.0 {
        push_unique(&mut hints.tone, "elaborate");
    }

    if exclam_per_utter >= 0.4 {
        push_unique(&mut hints.tone, "energetic");
    }
    if question_per_utter >= 0.3 {
        push_unique(&mut hints.tone, "inquisitive");
    }
    if allcaps_ratio >= 0.20 && alpha_letters_total >= 50 {
        push_unique(&mut hints.tone, "emphatic");
    }
    if (emoji_count as f64) / n >= 0.5 {
        push_unique(&mut hints.tone, "playful");
    }

    // ── Pacing ─────────────────────────────────────────────────────
    hints.pacing = Some(if avg_words_per_utter <= 6.0 {
        "fast"
    } else if avg_words_per_utter <= 18.0 {
        "measured"
    } else {
        "slow"
    });

    // ── Quirks: surface the strongest filler usage (≥1/3 of utterances). ─
    let mut filler_sorted: Vec<(&&'static str, &usize)> =
        filler_hits.iter().collect();
    filler_sorted.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    for (filler, count) in filler_sorted.iter().take(2) {
        if (**count as f64) / n >= 1.0 / 3.0 {
            hints.quirks.push(format!("often uses \"{}\"", filler));
        }
    }
    // Emoji-as-quirk is recorded separately so users who type sparse
    // single-word turns still get the cue.
    if (emoji_count as f64) / n >= 1.0 {
        hints.quirks.push("frequent emoji use".to_string());
    }

    // Hard caps (defensive — push_unique already keeps `tone` small,
    // but we belt-and-braces to honour the doc-stated 4/3 limits).
    hints.tone.truncate(4);
    hints.quirks.truncate(3);
    hints
}

/// Render the hints into a short prompt block. Returns `None` when no
/// signal is recorded (caller skips the section entirely so the LLM
/// doesn't hallucinate from an empty cue).
pub fn render_prosody_block(hints: &ProsodyHints) -> Option<String> {
    if hints.is_empty() {
        return None;
    }
    let mut parts: Vec<String> = Vec::new();
    if !hints.tone.is_empty() {
        parts.push(format!("tone: {}", hints.tone.join(", ")));
    }
    if let Some(pacing) = hints.pacing {
        parts.push(format!("pacing: {}", pacing));
    }
    if !hints.quirks.is_empty() {
        parts.push(format!("quirks: {}", hints.quirks.join("; ")));
    }
    Some(format!(
        "Voice-derived hints (the user has ASR configured, so their typed turns reflect spoken patterns): {}.",
        parts.join(" · ")
    ))
}

// ── helpers ─────────────────────────────────────────────────────────

fn push_unique(into: &mut Vec<&'static str>, val: &'static str) {
    if !into.contains(&val) {
        into.push(val);
    }
}

/// Cheap whole-word check: needle bounded by non-alphanumeric on both
/// sides (or string boundary). Avoids the regex crate for a one-shot.
fn contains_whole_word(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() || haystack.len() < needle.len() {
        return false;
    }
    let bytes = haystack.as_bytes();
    let nbytes = needle.as_bytes();
    let mut i = 0;
    while i + nbytes.len() <= bytes.len() {
        if &bytes[i..i + nbytes.len()] == nbytes {
            let before_ok = i == 0 || !is_word_byte(bytes[i - 1]);
            let after_ok = i + nbytes.len() == bytes.len()
                || !is_word_byte(bytes[i + nbytes.len()]);
            if before_ok && after_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Coarse "is this codepoint an emoji?" check that covers the common
/// pictograph / supplemental-symbol blocks. Not exhaustive (Unicode
/// emoji are sequences, not single codepoints) but correct enough for
/// "did the user pepper their messages with 😄 / 🎉 / 🔥 / etc."
fn is_emoji(c: char) -> bool {
    let cp = c as u32;
    matches!(
        cp,
        0x1F300..=0x1F5FF // misc symbols + pictographs
            | 0x1F600..=0x1F64F // emoticons
            | 0x1F680..=0x1F6FF // transport & map
            | 0x1F700..=0x1F77F // alchemical
            | 0x1F900..=0x1F9FF // supplemental symbols + pictographs
            | 0x1FA70..=0x1FAFF // symbols & pictographs extended-A
            | 0x2600..=0x26FF   // misc symbols
            | 0x2700..=0x27BF   // dingbats
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyze_empty_returns_empty() {
        assert!(analyze_user_utterances(&[]).is_empty());
    }

    #[test]
    fn fewer_than_min_utterances_returns_empty() {
        let utters = ["yo", "hey"];
        assert!(analyze_user_utterances(&utters).is_empty());
    }

    #[test]
    fn whitespace_only_utterances_are_dropped() {
        let utters = ["yo", "   ", "\n\t", "hey"];
        // Only 2 real → below MIN_UTTERANCES → empty.
        assert!(analyze_user_utterances(&utters).is_empty());
    }

    #[test]
    fn oversize_total_input_returns_empty() {
        // 4 utterances × 300 KB each → 1.2 MiB total → over cap.
        let big = "a ".repeat(150_000);
        let slice = big.as_str();
        let utters = [slice, slice, slice, slice];
        assert!(analyze_user_utterances(&utters).is_empty());
    }

    #[test]
    fn concise_short_utterances_get_concise_and_fast() {
        let utters = ["yes", "okay sure", "sounds good", "lol", "noted"];
        let h = analyze_user_utterances(&utters);
        assert!(h.tone.contains(&"concise"));
        assert_eq!(h.pacing, Some("fast"));
    }

    #[test]
    fn long_winded_utterances_get_elaborate_and_slow() {
        let long = "I think the most fascinating part of the whole \
            architecture is how the dataflow lazily resolves dependencies \
            even when the upstream module changes its signature without warning";
        let utters = [long, long, long, long];
        let h = analyze_user_utterances(&utters);
        assert!(h.tone.contains(&"elaborate"));
        assert_eq!(h.pacing, Some("slow"));
    }

    #[test]
    fn high_exclamation_density_marks_energetic() {
        let utters = [
            "wow that's amazing!",
            "love it!",
            "let's go!",
            "yes please!",
        ];
        let h = analyze_user_utterances(&utters);
        assert!(h.tone.contains(&"energetic"));
    }

    #[test]
    fn high_question_density_marks_inquisitive() {
        let utters = [
            "what about timeouts?",
            "how does that scale?",
            "where does the cache live?",
            "why use SQLite here?",
        ];
        let h = analyze_user_utterances(&utters);
        assert!(h.tone.contains(&"inquisitive"));
    }

    #[test]
    fn allcaps_marks_emphatic_only_when_enough_letters() {
        // Many ALLCAPS letters (>50 alpha + >=20% caps).
        let utters = [
            "OKAY THIS IS REALLY IMPORTANT",
            "PLEASE DO NOT FORGET",
            "I MEAN IT",
            "thanks",
        ];
        let h = analyze_user_utterances(&utters);
        assert!(h.tone.contains(&"emphatic"));

        // Tiny corpus with caps but < 50 alpha letters → not emphatic.
        let small = ["YES", "NO", "OK", "GO"];
        let h2 = analyze_user_utterances(&small);
        assert!(!h2.tone.contains(&"emphatic"));
    }

    #[test]
    fn frequent_emoji_marks_playful_and_quirk() {
        let utters = [
            "love it 😄🎉",
            "haha 😂😂",
            "amazing 🔥",
            "yay 🎈",
        ];
        let h = analyze_user_utterances(&utters);
        assert!(h.tone.contains(&"playful"));
        assert!(h.quirks.iter().any(|q| q.contains("emoji")));
    }

    #[test]
    fn filler_words_become_quirks() {
        let utters = [
            "like, I think we should refactor",
            "like the new flow is cleaner",
            "I mean, like, it just makes sense",
            "yeah, sounds good",
            "I dunno",
            "okay",
        ];
        let h = analyze_user_utterances(&utters);
        assert!(
            h.quirks.iter().any(|q| q.contains("like")),
            "expected 'like' quirk, got {:?}",
            h.quirks
        );
    }

    #[test]
    fn quirks_are_capped_at_three() {
        // Many filler types + emoji + everything → still ≤3 quirks.
        let utters = [
            "like um uh literally basically actually 😄",
            "like um uh literally basically actually 🎉",
            "like um uh literally basically actually 🔥",
            "like um uh literally basically actually 🎈",
            "like um uh literally basically actually 🚀",
            "like um uh literally basically actually 💡",
        ];
        let h = analyze_user_utterances(&utters);
        assert!(h.quirks.len() <= 3, "got {:?}", h.quirks);
    }

    #[test]
    fn tone_is_capped_at_four() {
        // Force every tone signal at once.
        let utters = [
            "WOW WHAT A MOMENT! HOW DOES THIS WORK?! 😄🎉",
            "AMAZING! WHY THOUGH? 😂",
            "INCREDIBLE!! HOW?! 🔥",
            "YES YES YES! WHERE? 🎈",
            "GO GO GO! WHAT?! 🚀",
            "WHEN?! 💡",
        ];
        let h = analyze_user_utterances(&utters);
        assert!(h.tone.len() <= 4, "got {:?}", h.tone);
    }

    #[test]
    fn medium_length_gets_measured_pacing() {
        let utters = [
            "okay let me think about that for a moment",
            "I have a few thoughts on the matter",
            "I think we should plan ahead a bit",
            "let's circle back tomorrow morning",
        ];
        let h = analyze_user_utterances(&utters);
        assert_eq!(h.pacing, Some("measured"));
    }

    #[test]
    fn render_prosody_block_returns_none_for_empty_hints() {
        assert!(render_prosody_block(&ProsodyHints::default()).is_none());
    }

    #[test]
    fn render_prosody_block_includes_all_present_sections() {
        let mut h = ProsodyHints::default();
        h.tone.push("concise");
        h.pacing = Some("fast");
        h.quirks.push("often uses \"like\"".to_string());
        let rendered = render_prosody_block(&h).unwrap();
        assert!(rendered.contains("Voice-derived hints"));
        assert!(rendered.contains("tone: concise"));
        assert!(rendered.contains("pacing: fast"));
        assert!(rendered.contains("often uses"));
    }

    #[test]
    fn contains_whole_word_does_not_match_substrings() {
        // "like" must not match "alike" or "likeness".
        assert!(!contains_whole_word("alike", "like"));
        assert!(!contains_whole_word("likeness", "like"));
        assert!(contains_whole_word("yeah, like that", "like"));
        assert!(contains_whole_word("like!", "like"));
        assert!(contains_whole_word("'like'", "like"));
    }

    #[test]
    fn contains_whole_word_handles_multiword_phrase() {
        assert!(contains_whole_word("you know what i mean", "you know"));
        assert!(contains_whole_word("sort of, yeah", "sort of"));
    }

    #[test]
    fn is_emoji_recognises_common_blocks() {
        assert!(is_emoji('😄'));
        assert!(is_emoji('🎉'));
        assert!(is_emoji('🔥'));
        assert!(is_emoji('☀'));
        assert!(!is_emoji('a'));
        assert!(!is_emoji('!'));
        assert!(!is_emoji('1'));
    }

    #[test]
    fn analyze_returns_pacing_only_for_borderline_corpus() {
        // 3 short bland utterances → just enough to pass MIN_UTTERANCES,
        // no exclamations / questions / fillers / emojis. Should yield
        // at minimum a pacing label so the caller can choose to render
        // a useful (but tiny) hint.
        let utters = ["yes", "okay", "noted"];
        let h = analyze_user_utterances(&utters);
        assert!(h.pacing.is_some());
    }
}
