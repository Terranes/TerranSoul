//! Charisma teaching stats — usage, ratings, maturity, and source-code
//! promotion plumbing for user-taught persona traits, learned expressions,
//! and learned motions (Chunk 30.4).
//!
//! Sits next to `extract.rs` and `drift.rs` because it is the third
//! sense-of-self / mirror-neuron loop: where Master-Echo proposes traits
//! and Drift detects shift, **Charisma** measures how often the user's
//! taught material is actually used and how well it lands. Mature
//! ("proven") items can be promoted to source-code defaults via the
//! multi-agent workflow runner from Chunk 30.3.
//!
//! Persisted as a single atomic JSON file:
//!
//! ```text
//! <app_data_dir>/persona/charisma_stats.json
//! ```
//!
//! See `docs/charisma-teaching-tutorial.md` for the end-to-end flow.

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Asset kinds that participate in the charisma teaching loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CharismaAssetKind {
    /// A persona trait field — bio / tone / quirk / avoid / example dialogue.
    Trait,
    /// A learned facial expression (see `LearnedExpression`).
    Expression,
    /// A learned body motion clip (see `LearnedMotion`).
    Motion,
}

/// Maturity tiers - re-exported from the shared promotion module so all
/// user-taught promotion surfaces use the same bar.
///
/// Local re-export keeps existing call sites (`charisma::Maturity`)
/// working unchanged while the canonical definition lives in
/// [`crate::coding::promotion_plan`].
pub use crate::coding::promotion_plan::Maturity;

/// Per-asset stats. The `(kind, asset_id)` pair is the index key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharismaStat {
    pub kind: CharismaAssetKind,
    pub asset_id: String,
    /// Display label, denormalised from the asset for fast list rendering.
    pub display_name: String,
    /// When the user first taught this asset (ms epoch).
    pub taught_at: u64,
    /// How many times the LLM / runtime triggered this asset.
    pub usage_count: u32,
    /// Last time the asset was used (ms epoch). 0 = never.
    pub last_used_at: u64,
    /// Sum of all 1–5 ratings the user has left.
    pub rating_sum: u32,
    /// Number of ratings recorded.
    pub rating_count: u32,
    /// Promotion bookkeeping — set when `promote_to_source` succeeds.
    pub promoted_at: Option<u64>,
    /// The workflow plan id created for the last promotion attempt.
    pub last_promotion_plan_id: Option<String>,
}

impl CharismaStat {
    pub fn new(kind: CharismaAssetKind, asset_id: String, display_name: String, now_ms: u64) -> Self {
        Self {
            kind,
            asset_id,
            display_name,
            taught_at: now_ms,
            usage_count: 0,
            last_used_at: 0,
            rating_sum: 0,
            rating_count: 0,
            promoted_at: None,
            last_promotion_plan_id: None,
        }
    }

    /// Average rating (0.0 if no ratings yet).
    pub fn avg_rating(&self) -> f32 {
        if self.rating_count == 0 {
            0.0
        } else {
            self.rating_sum as f32 / self.rating_count as f32
        }
    }

    /// Compute maturity tier from current counters.
    pub fn maturity(&self) -> Maturity {
        use crate::coding::promotion_plan::{PROVEN_MIN_AVG_RATING, PROVEN_MIN_USES};
        if self.promoted_at.is_some() {
            return Maturity::Canon;
        }
        if self.usage_count == 0 {
            return Maturity::Untested;
        }
        if self.usage_count >= PROVEN_MIN_USES && self.avg_rating() >= PROVEN_MIN_AVG_RATING {
            return Maturity::Proven;
        }
        Maturity::Learning
    }
}

/// Whole on-disk index. Versioned to allow non-breaking field additions.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CharismaIndex {
    #[serde(default = "default_version")]
    pub version: u32,
    /// Keyed by `format!("{kind}:{asset_id}")` for stable serialisation.
    #[serde(default)]
    pub stats: HashMap<String, CharismaStat>,
}

fn default_version() -> u32 {
    1
}

impl CharismaIndex {
    /// Stable composite key for the JSON map.
    pub fn key(kind: CharismaAssetKind, asset_id: &str) -> String {
        let prefix = match kind {
            CharismaAssetKind::Trait => "trait",
            CharismaAssetKind::Expression => "expression",
            CharismaAssetKind::Motion => "motion",
        };
        format!("{prefix}:{asset_id}")
    }

    pub fn get(&self, kind: CharismaAssetKind, asset_id: &str) -> Option<&CharismaStat> {
        self.stats.get(&Self::key(kind, asset_id))
    }

    pub fn upsert(&mut self, stat: CharismaStat) {
        let key = Self::key(stat.kind, &stat.asset_id);
        self.stats.insert(key, stat);
    }

    /// Increment usage counter. Creates a stub stat if the asset is unknown
    /// (so the very first runtime trigger still counts).
    pub fn record_usage(
        &mut self,
        kind: CharismaAssetKind,
        asset_id: &str,
        display_name: &str,
        now_ms: u64,
    ) {
        let key = Self::key(kind, asset_id);
        let stat = self
            .stats
            .entry(key)
            .or_insert_with(|| CharismaStat::new(kind, asset_id.to_string(), display_name.to_string(), now_ms));
        stat.usage_count = stat.usage_count.saturating_add(1);
        stat.last_used_at = now_ms;
    }

    /// Add a 1–5 rating. Out-of-range ratings are clamped.
    pub fn add_rating(
        &mut self,
        kind: CharismaAssetKind,
        asset_id: &str,
        display_name: &str,
        rating: u32,
        now_ms: u64,
    ) {
        let clamped = rating.clamp(1, 5);
        let key = Self::key(kind, asset_id);
        let stat = self
            .stats
            .entry(key)
            .or_insert_with(|| CharismaStat::new(kind, asset_id.to_string(), display_name.to_string(), now_ms));
        stat.rating_sum = stat.rating_sum.saturating_add(clamped);
        stat.rating_count = stat.rating_count.saturating_add(1);
    }

    pub fn remove(&mut self, kind: CharismaAssetKind, asset_id: &str) -> Option<CharismaStat> {
        self.stats.remove(&Self::key(kind, asset_id))
    }

    /// Mark an asset as promoted (Canon).
    pub fn mark_promoted(&mut self, kind: CharismaAssetKind, asset_id: &str, plan_id: String, now_ms: u64) {
        let key = Self::key(kind, asset_id);
        if let Some(stat) = self.stats.get_mut(&key) {
            stat.promoted_at = Some(now_ms);
            stat.last_promotion_plan_id = Some(plan_id);
        }
    }

    /// All assets currently meeting the Proven bar. Sorted by avg rating
    /// descending, then usage_count descending.
    pub fn proven(&self) -> Vec<&CharismaStat> {
        let mut out: Vec<&CharismaStat> = self
            .stats
            .values()
            .filter(|s| s.maturity() == Maturity::Proven)
            .collect();
        out.sort_by(|a, b| {
            b.avg_rating()
                .partial_cmp(&a.avg_rating())
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(b.usage_count.cmp(&a.usage_count))
        });
        out
    }
}

/// Aggregate counts surfaced on the dashboard.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CharismaSummary {
    pub total: u32,
    pub untested: u32,
    pub learning: u32,
    pub proven: u32,
    pub canon: u32,
}

impl CharismaSummary {
    pub fn from_index(index: &CharismaIndex) -> Self {
        let mut s = Self::default();
        for stat in index.stats.values() {
            s.total += 1;
            match stat.maturity() {
                Maturity::Untested => s.untested += 1,
                Maturity::Learning => s.learning += 1,
                Maturity::Proven => s.proven += 1,
                Maturity::Canon => s.canon += 1,
            }
        }
        s
    }
}

// ---------------------------------------------------------------------------
// Disk persistence (atomic JSON)
// ---------------------------------------------------------------------------

const STATS_FILE_NAME: &str = "charisma_stats.json";

pub fn stats_path(persona_dir: &Path) -> std::path::PathBuf {
    persona_dir.join(STATS_FILE_NAME)
}

pub fn load_index(persona_dir: &Path) -> Result<CharismaIndex, String> {
    let path = stats_path(persona_dir);
    if !path.exists() {
        return Ok(CharismaIndex {
            version: default_version(),
            stats: HashMap::new(),
        });
    }
    let bytes = fs::read(&path).map_err(|e| format!("read charisma_stats.json: {e}"))?;
    let mut index: CharismaIndex =
        serde_json::from_slice(&bytes).map_err(|e| format!("parse charisma_stats.json: {e}"))?;
    if index.version == 0 {
        index.version = 1;
    }
    Ok(index)
}

pub fn save_index(persona_dir: &Path, index: &CharismaIndex) -> Result<(), String> {
    fs::create_dir_all(persona_dir).map_err(|e| format!("mkdir persona dir: {e}"))?;
    let path = stats_path(persona_dir);
    let tmp = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(index).map_err(|e| format!("serialise charisma_stats: {e}"))?;
    {
        let mut f = fs::File::create(&tmp).map_err(|e| format!("create temp: {e}"))?;
        f.write_all(&bytes).map_err(|e| format!("write temp: {e}"))?;
        f.sync_all().map_err(|e| format!("sync temp: {e}"))?;
    }
    fs::rename(&tmp, &path).map_err(|e| format!("rename temp: {e}"))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Promotion plan generation (delegates to the shared promotion-plan helper)
// ---------------------------------------------------------------------------

use crate::coding::multi_agent::WorkflowPlan;
use crate::coding::promotion_plan::{build_promotion_plan as build_shared_plan, PromotionPlanSpec};

/// Build a coding-kind workflow plan that promotes a proven charisma asset
/// into source-code defaults. Thin domain-specific wrapper around
/// [`crate::coding::promotion_plan::build_promotion_plan`] so every
/// user-taught promotion surface shares the same DAG, retry policy, and
/// approval gates.
///
/// Targets file (Researcher will confirm the exact insertion point):
///
/// | Kind | Likely target |
/// |---|---|
/// | Trait | `src-tauri/src/commands/persona.rs` (default_persona_json) |
/// | Expression | `src/renderer/face-mirror.ts` (DEFAULT_LEARNED_EXPRESSIONS) |
/// | Motion | `src/renderer/vrma-manager.ts` (VRMA_ANIMATIONS registry) |
pub fn build_promotion_plan(
    stat: &CharismaStat,
    plan_id: String,
    now_ms: u64,
) -> WorkflowPlan {
    let asset_label = match stat.kind {
        CharismaAssetKind::Trait => "persona trait",
        CharismaAssetKind::Expression => "facial expression",
        CharismaAssetKind::Motion => "body motion",
    };

    let likely_targets: Vec<String> = match stat.kind {
        CharismaAssetKind::Trait => vec!["src-tauri/src/commands/persona.rs".into()],
        CharismaAssetKind::Expression => vec!["src/renderer/face-mirror.ts".into()],
        CharismaAssetKind::Motion => vec!["src/renderer/vrma-manager.ts".into()],
    };

    let title = format!("Promote {asset_label} '{}' to source defaults", stat.display_name);
    let user_request = format!(
        "User-taught {} '{}' has been used {} times with average rating {:.1}/5. \
         Promote it to a bundled default so future installs ship with it.\n\n\
         Asset id: {}\nMaturity: proven",
        asset_label,
        stat.display_name,
        stat.usage_count,
        stat.avg_rating(),
        stat.asset_id
    );

    build_shared_plan(PromotionPlanSpec {
        plan_id,
        now_ms,
        title,
        user_request,
        research_description: format!(
            "Locate the source file that owns the bundled {asset_label} defaults \
             and read the current entries so the new addition is consistent."
        ),
        code_description: format!(
            "Emit a `<file path=...>` block adding the new {asset_label} \
             '{}' (asset id `{}`) to the bundled defaults. Preserve existing \
             entries verbatim. Do not introduce new public API.",
            stat.display_name, stat.asset_id
        ),
        test_description:
            "Run `npx vitest run src/stores/persona.test.ts` and `cargo test --lib persona` \
             to verify the new default is loaded and serialisation round-trips."
                .to_string(),
        review_description:
            "Final verdict — security audit (no PII in defaults), style consistency \
             with existing bundled assets, no breaking schema changes."
                .to_string(),
        tags: vec![
            "charisma".to_string(),
            "promotion".to_string(),
            asset_label.replace(' ', "-"),
        ],
        target_files: &likely_targets,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coding::{AgentRole, WorkflowKind, WorkflowPlanStatus};
    use tempfile::TempDir;

    #[test]
    fn maturity_progresses_with_usage_and_rating() {
        let mut s = CharismaStat::new(CharismaAssetKind::Expression, "lex_a".into(), "Smug".into(), 1_000);
        assert_eq!(s.maturity(), Maturity::Untested);

        s.usage_count = 1;
        s.last_used_at = 1_100;
        assert_eq!(s.maturity(), Maturity::Learning);

        s.usage_count = 12;
        s.rating_sum = 16;
        s.rating_count = 4; // avg 4.0
        assert_eq!(s.maturity(), Maturity::Proven);

        s.promoted_at = Some(2_000);
        s.last_promotion_plan_id = Some("plan_x".into());
        assert_eq!(s.maturity(), Maturity::Canon);
    }

    #[test]
    fn proven_filter_requires_both_usage_and_rating() {
        let mut s = CharismaStat::new(CharismaAssetKind::Motion, "lmo_a".into(), "Shrug".into(), 0);
        s.usage_count = 50; // lots of usage
        s.rating_sum = 6;
        s.rating_count = 2; // avg 3.0 — below the 4.0 bar
        assert_eq!(s.maturity(), Maturity::Learning);
    }

    #[test]
    fn record_usage_increments_and_creates_stub() {
        let mut idx = CharismaIndex::default();
        idx.record_usage(CharismaAssetKind::Expression, "lex_a", "Smug", 1_000);
        let stat = idx.get(CharismaAssetKind::Expression, "lex_a").unwrap();
        assert_eq!(stat.usage_count, 1);
        assert_eq!(stat.last_used_at, 1_000);
        assert_eq!(stat.display_name, "Smug");

        idx.record_usage(CharismaAssetKind::Expression, "lex_a", "Smug", 2_000);
        let stat = idx.get(CharismaAssetKind::Expression, "lex_a").unwrap();
        assert_eq!(stat.usage_count, 2);
        assert_eq!(stat.last_used_at, 2_000);
    }

    #[test]
    fn add_rating_clamps_out_of_range() {
        let mut idx = CharismaIndex::default();
        idx.add_rating(CharismaAssetKind::Trait, "tone_warm", "warm", 10, 0);
        idx.add_rating(CharismaAssetKind::Trait, "tone_warm", "warm", 0, 0);
        let stat = idx.get(CharismaAssetKind::Trait, "tone_warm").unwrap();
        assert_eq!(stat.rating_count, 2);
        // 10 clamped to 5, 0 clamped to 1, sum = 6
        assert_eq!(stat.rating_sum, 6);
        assert!((stat.avg_rating() - 3.0).abs() < 0.01);
    }

    #[test]
    fn summary_counts_each_tier() {
        let mut idx = CharismaIndex::default();
        // Untested
        idx.upsert(CharismaStat::new(
            CharismaAssetKind::Expression,
            "u1".into(),
            "u1".into(),
            0,
        ));
        // Learning (1 use, 1 rating @ 3)
        idx.record_usage(CharismaAssetKind::Expression, "l1", "l1", 0);
        idx.add_rating(CharismaAssetKind::Expression, "l1", "l1", 3, 0);
        // Proven (10 uses, avg 4.5)
        for _ in 0..10 {
            idx.record_usage(CharismaAssetKind::Motion, "p1", "p1", 0);
        }
        for _ in 0..2 {
            idx.add_rating(CharismaAssetKind::Motion, "p1", "p1", 5, 0);
            idx.add_rating(CharismaAssetKind::Motion, "p1", "p1", 4, 0);
        }
        // Canon (proven + promoted)
        idx.upsert(CharismaStat {
            kind: CharismaAssetKind::Trait,
            asset_id: "c1".into(),
            display_name: "c1".into(),
            taught_at: 0,
            usage_count: 50,
            last_used_at: 0,
            rating_sum: 25,
            rating_count: 5,
            promoted_at: Some(123),
            last_promotion_plan_id: Some("plan_x".into()),
        });

        let s = CharismaSummary::from_index(&idx);
        assert_eq!(s.total, 4);
        assert_eq!(s.untested, 1);
        assert_eq!(s.learning, 1);
        assert_eq!(s.proven, 1);
        assert_eq!(s.canon, 1);
    }

    #[test]
    fn round_trip_through_disk() {
        let dir = TempDir::new().unwrap();
        let mut idx = CharismaIndex::default();
        idx.record_usage(CharismaAssetKind::Expression, "lex_a", "Smug", 1_000);
        idx.add_rating(CharismaAssetKind::Expression, "lex_a", "Smug", 5, 1_000);

        save_index(dir.path(), &idx).unwrap();
        let loaded = load_index(dir.path()).unwrap();
        let stat = loaded.get(CharismaAssetKind::Expression, "lex_a").unwrap();
        assert_eq!(stat.usage_count, 1);
        assert_eq!(stat.rating_count, 1);
        assert_eq!(stat.rating_sum, 5);
    }

    #[test]
    fn missing_file_returns_empty_index() {
        let dir = TempDir::new().unwrap();
        let idx = load_index(dir.path()).unwrap();
        assert!(idx.stats.is_empty());
        assert_eq!(idx.version, 1);
    }

    #[test]
    fn proven_sort_by_rating_then_usage() {
        let mut idx = CharismaIndex::default();
        // Higher rating, lower usage
        idx.upsert(CharismaStat {
            kind: CharismaAssetKind::Expression,
            asset_id: "high_rating".into(),
            display_name: "hr".into(),
            taught_at: 0,
            usage_count: 10,
            last_used_at: 0,
            rating_sum: 25, // avg 5.0
            rating_count: 5,
            promoted_at: None,
            last_promotion_plan_id: None,
        });
        // Lower rating, higher usage
        idx.upsert(CharismaStat {
            kind: CharismaAssetKind::Motion,
            asset_id: "high_usage".into(),
            display_name: "hu".into(),
            taught_at: 0,
            usage_count: 100,
            last_used_at: 0,
            rating_sum: 20, // avg 4.0
            rating_count: 5,
            promoted_at: None,
            last_promotion_plan_id: None,
        });
        let proven = idx.proven();
        assert_eq!(proven.len(), 2);
        assert_eq!(proven[0].asset_id, "high_rating");
        assert_eq!(proven[1].asset_id, "high_usage");
    }

    #[test]
    fn build_promotion_plan_has_research_code_test_review_dag() {
        let mut s = CharismaStat::new(CharismaAssetKind::Expression, "lex_smug".into(), "Smug".into(), 1_000);
        s.usage_count = 12;
        s.rating_sum = 50;
        s.rating_count = 10;

        let plan = build_promotion_plan(&s, "plan_charisma_001".into(), 2_000);
        assert_eq!(plan.id, "plan_charisma_001");
        assert_eq!(plan.kind, WorkflowKind::Coding);
        assert_eq!(plan.status, WorkflowPlanStatus::PendingReview);
        assert_eq!(plan.steps.len(), 4);
        assert_eq!(plan.steps[0].id, "research");
        assert_eq!(plan.steps[0].agent, AgentRole::Researcher);
        assert_eq!(plan.steps[1].id, "code");
        assert_eq!(plan.steps[1].agent, AgentRole::Coder);
        assert!(plan.steps[1].requires_approval, "code step must require approval");
        assert_eq!(plan.steps[2].id, "test");
        assert_eq!(plan.steps[3].id, "review");
        assert!(plan.steps[3].requires_approval, "review step must require approval");
        assert!(plan.tags.contains(&"charisma".to_string()));
        assert!(plan.tags.contains(&"promotion".to_string()));
        assert!(plan.user_request.contains("12 times"));
        assert!(plan.user_request.contains("5.0/5"));
    }

    #[test]
    fn mark_promoted_flips_to_canon() {
        let mut idx = CharismaIndex::default();
        idx.record_usage(CharismaAssetKind::Motion, "lmo_x", "Bow", 0);
        idx.mark_promoted(CharismaAssetKind::Motion, "lmo_x", "plan_42".into(), 99);
        let stat = idx.get(CharismaAssetKind::Motion, "lmo_x").unwrap();
        assert_eq!(stat.promoted_at, Some(99));
        assert_eq!(stat.last_promotion_plan_id.as_deref(), Some("plan_42"));
        assert_eq!(stat.maturity(), Maturity::Canon);
    }

    #[test]
    fn remove_returns_stat_when_present() {
        let mut idx = CharismaIndex::default();
        idx.record_usage(CharismaAssetKind::Trait, "t1", "t1", 0);
        let removed = idx.remove(CharismaAssetKind::Trait, "t1");
        assert!(removed.is_some());
        assert!(idx.get(CharismaAssetKind::Trait, "t1").is_none());
        assert!(idx.remove(CharismaAssetKind::Trait, "t1").is_none());
    }
}
