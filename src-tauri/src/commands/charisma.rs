//! Charisma teaching Tauri commands (Chunk 30.4).
//!
//! Surface the [`crate::persona::charisma`] index as a set of Tauri
//! commands so the management panel and the runtime (chat / animation
//! pipeline) can record usage, ratings, and trigger source-code
//! promotion via the multi-agent workflow runner from Chunk 30.3.

use std::collections::HashSet;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::persona::charisma::{
    build_promotion_plan, load_index, save_index, CharismaAssetKind, CharismaStat, CharismaSummary,
    Maturity,
};
use crate::AppState;

const PERSONA_DIR: &str = "persona";

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn persona_dir(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join(PERSONA_DIR)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharismaListResponse {
    pub stats: Vec<CharismaStat>,
    pub summary: CharismaSummary,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CharismaUsageArgs {
    pub kind: CharismaAssetKind,
    pub asset_id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CharismaRatingArgs {
    pub kind: CharismaAssetKind,
    pub asset_id: String,
    pub display_name: String,
    pub rating: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CharismaTurnAssetArgs {
    pub kind: CharismaAssetKind,
    pub asset_id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CharismaTurnRatingArgs {
    pub assets: Vec<CharismaTurnAssetArgs>,
    pub rating: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CharismaDeleteArgs {
    pub kind: CharismaAssetKind,
    pub asset_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CharismaPromoteArgs {
    pub kind: CharismaAssetKind,
    pub asset_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CharismaPromoteResponse {
    pub plan_id: String,
    pub stat: CharismaStat,
}

// ---------------------------------------------------------------------------
// Inner helpers (testable without `tauri::State<AppState>`)
// ---------------------------------------------------------------------------

pub(crate) fn list_inner(data_dir: &Path) -> Result<CharismaListResponse, String> {
    let index = load_index(&persona_dir(data_dir))?;
    let summary = CharismaSummary::from_index(&index);
    let mut stats: Vec<CharismaStat> = index.stats.values().cloned().collect();
    // Most-recently-taught first, with promoted (Canon) items pinned to the
    // bottom so the user's attention falls on Learning / Proven items they
    // can act on.
    stats.sort_by(|a, b| {
        let canon_a = a.promoted_at.is_some();
        let canon_b = b.promoted_at.is_some();
        canon_a.cmp(&canon_b).then(b.taught_at.cmp(&a.taught_at))
    });
    Ok(CharismaListResponse { stats, summary })
}

pub(crate) fn record_usage_inner(
    data_dir: &Path,
    args: &CharismaUsageArgs,
) -> Result<CharismaStat, String> {
    let mut index = load_index(&persona_dir(data_dir))?;
    index.record_usage(args.kind, &args.asset_id, &args.display_name, now_ms());
    save_index(&persona_dir(data_dir), &index)?;
    index
        .get(args.kind, &args.asset_id)
        .cloned()
        .ok_or_else(|| "stat missing after record_usage".to_string())
}

pub(crate) fn set_rating_inner(
    data_dir: &Path,
    args: &CharismaRatingArgs,
) -> Result<CharismaStat, String> {
    let mut index = load_index(&persona_dir(data_dir))?;
    index.add_rating(
        args.kind,
        &args.asset_id,
        &args.display_name,
        args.rating,
        now_ms(),
    );
    save_index(&persona_dir(data_dir), &index)?;
    index
        .get(args.kind, &args.asset_id)
        .cloned()
        .ok_or_else(|| "stat missing after rating".to_string())
}

pub(crate) fn rate_turn_inner(
    data_dir: &Path,
    args: &CharismaTurnRatingArgs,
) -> Result<Vec<CharismaStat>, String> {
    let mut index = load_index(&persona_dir(data_dir))?;
    let mut seen = HashSet::new();
    let mut rated = Vec::new();
    let now = now_ms();

    for asset in &args.assets {
        let key = crate::persona::charisma::CharismaIndex::key(asset.kind, &asset.asset_id);
        if !seen.insert(key) {
            continue;
        }
        index.add_rating(
            asset.kind,
            &asset.asset_id,
            &asset.display_name,
            args.rating,
            now,
        );
        if let Some(stat) = index.get(asset.kind, &asset.asset_id).cloned() {
            rated.push(stat);
        }
    }

    save_index(&persona_dir(data_dir), &index)?;
    Ok(rated)
}

pub(crate) fn delete_inner(data_dir: &Path, args: &CharismaDeleteArgs) -> Result<(), String> {
    let mut index = load_index(&persona_dir(data_dir))?;
    index.remove(args.kind, &args.asset_id);
    save_index(&persona_dir(data_dir), &index)
}

pub(crate) fn promote_inner(
    data_dir: &Path,
    args: &CharismaPromoteArgs,
) -> Result<CharismaPromoteResponse, String> {
    let mut index = load_index(&persona_dir(data_dir))?;
    let stat = index
        .get(args.kind, &args.asset_id)
        .cloned()
        .ok_or_else(|| {
            format!(
                "Asset not found: {} {}",
                asset_kind_label(args.kind),
                args.asset_id
            )
        })?;

    if stat.maturity() != Maturity::Proven {
        return Err(format!(
            "Asset is not yet Proven (current maturity: {:?}). \
             Need ≥10 uses and average rating ≥4.0.",
            stat.maturity()
        ));
    }

    // Build the plan and save it through the multi_agent storage layer so
    // it appears in the Workflows panel automatically.
    let plan_id = crate::coding::multi_agent::new_plan_id();
    let plan = build_promotion_plan(&stat, plan_id.clone(), now_ms());
    crate::coding::multi_agent::save_plan(data_dir, &plan)?;

    // Mark Canon **only** after the plan is saved; if save fails we leave
    // the asset Proven so the user can retry.
    index.mark_promoted(args.kind, &args.asset_id, plan_id.clone(), now_ms());
    save_index(&persona_dir(data_dir), &index)?;

    let stat = index
        .get(args.kind, &args.asset_id)
        .cloned()
        .ok_or_else(|| "stat missing after promotion".to_string())?;
    Ok(CharismaPromoteResponse { plan_id, stat })
}

pub(crate) fn summary_inner(data_dir: &Path) -> Result<CharismaSummary, String> {
    let index = load_index(&persona_dir(data_dir))?;
    Ok(CharismaSummary::from_index(&index))
}

fn asset_kind_label(kind: CharismaAssetKind) -> &'static str {
    match kind {
        CharismaAssetKind::Trait => "trait",
        CharismaAssetKind::Expression => "expression",
        CharismaAssetKind::Motion => "motion",
    }
}

// ---------------------------------------------------------------------------
// Tauri command wrappers
// ---------------------------------------------------------------------------

/// Return all charisma stats plus a summary roll-up.
#[tauri::command]
pub async fn charisma_list(state: State<'_, AppState>) -> Result<CharismaListResponse, String> {
    list_inner(&state.data_dir)
}

/// Increment usage for a taught asset. Called from the runtime whenever
/// the LLM emits a trigger that maps to a learned expression / motion.
#[tauri::command]
pub async fn charisma_record_usage(
    args: CharismaUsageArgs,
    state: State<'_, AppState>,
) -> Result<CharismaStat, String> {
    record_usage_inner(&state.data_dir, &args)
}

/// Add a 1–5 rating for a taught asset.
#[tauri::command]
pub async fn charisma_set_rating(
    args: CharismaRatingArgs,
    state: State<'_, AppState>,
) -> Result<CharismaStat, String> {
    set_rating_inner(&state.data_dir, &args)
}

/// Add the same 1–5 rating to every Charisma asset that fired in one
/// assistant turn. Duplicate `(kind, asset_id)` pairs are rated once.
#[tauri::command]
pub async fn charisma_rate_turn(
    args: CharismaTurnRatingArgs,
    state: State<'_, AppState>,
) -> Result<Vec<CharismaStat>, String> {
    rate_turn_inner(&state.data_dir, &args)
}

/// Remove the stat row. Does **not** delete the underlying learned
/// expression / motion file — callers that want a full delete also call
/// `delete_learned_expression` / `delete_learned_motion`.
#[tauri::command]
pub async fn charisma_delete(
    args: CharismaDeleteArgs,
    state: State<'_, AppState>,
) -> Result<(), String> {
    delete_inner(&state.data_dir, &args)
}

/// Promote a Proven asset into a multi-agent workflow plan that will
/// edit source code to add it as a bundled default. The plan is saved
/// under `<data_dir>/workflow_plans/<id>.yaml` for the user to review
/// and run from the Multi-Agent Workflows panel.
#[tauri::command]
pub async fn charisma_promote(
    args: CharismaPromoteArgs,
    state: State<'_, AppState>,
) -> Result<CharismaPromoteResponse, String> {
    promote_inner(&state.data_dir, &args)
}

/// Aggregate counts only (useful for badges / dashboards).
#[tauri::command]
pub async fn charisma_summary(state: State<'_, AppState>) -> Result<CharismaSummary, String> {
    summary_inner(&state.data_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn dir() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    fn record(d: &TempDir, kind: CharismaAssetKind, id: &str, name: &str) {
        record_usage_inner(
            d.path(),
            &CharismaUsageArgs {
                kind,
                asset_id: id.into(),
                display_name: name.into(),
            },
        )
        .unwrap();
    }

    fn rate(d: &TempDir, kind: CharismaAssetKind, id: &str, name: &str, r: u32) {
        set_rating_inner(
            d.path(),
            &CharismaRatingArgs {
                kind,
                asset_id: id.into(),
                display_name: name.into(),
                rating: r,
            },
        )
        .unwrap();
    }

    #[test]
    fn list_returns_empty_initially() {
        let d = dir();
        let resp = list_inner(d.path()).unwrap();
        assert_eq!(resp.summary.total, 0);
        assert!(resp.stats.is_empty());
    }

    #[test]
    fn record_usage_then_rating_progresses_to_proven() {
        let d = dir();
        for _ in 0..10 {
            record(&d, CharismaAssetKind::Expression, "lex_smug", "Smug");
        }
        for _ in 0..3 {
            rate(&d, CharismaAssetKind::Expression, "lex_smug", "Smug", 5);
        }
        let summary = summary_inner(d.path()).unwrap();
        assert_eq!(summary.proven, 1);
    }

    #[test]
    fn promote_rejects_unproven_assets() {
        let d = dir();
        record(&d, CharismaAssetKind::Trait, "tone_warm", "warm");
        let err = promote_inner(
            d.path(),
            &CharismaPromoteArgs {
                kind: CharismaAssetKind::Trait,
                asset_id: "tone_warm".into(),
            },
        )
        .unwrap_err();
        assert!(err.contains("not yet Proven"), "{err}");
    }

    #[test]
    fn rate_turn_rates_each_asset_once() {
        let d = dir();
        let stats = rate_turn_inner(
            d.path(),
            &CharismaTurnRatingArgs {
                rating: 5,
                assets: vec![
                    CharismaTurnAssetArgs {
                        kind: CharismaAssetKind::Expression,
                        asset_id: "lex_smug".into(),
                        display_name: "Smug".into(),
                    },
                    CharismaTurnAssetArgs {
                        kind: CharismaAssetKind::Expression,
                        asset_id: "lex_smug".into(),
                        display_name: "Smug".into(),
                    },
                    CharismaTurnAssetArgs {
                        kind: CharismaAssetKind::Motion,
                        asset_id: "lmo_bow".into(),
                        display_name: "Bow".into(),
                    },
                ],
            },
        )
        .unwrap();

        assert_eq!(stats.len(), 2);
        let resp = list_inner(d.path()).unwrap();
        let smug = resp
            .stats
            .iter()
            .find(|s| s.asset_id == "lex_smug")
            .unwrap();
        assert_eq!(smug.rating_count, 1);
        assert_eq!(smug.rating_sum, 5);
        assert_eq!(resp.summary.total, 2);
    }

    #[test]
    fn promote_creates_plan_and_marks_canon() {
        let d = dir();
        for _ in 0..12 {
            record(&d, CharismaAssetKind::Motion, "lmo_bow", "Bow");
        }
        for _ in 0..2 {
            rate(&d, CharismaAssetKind::Motion, "lmo_bow", "Bow", 5);
        }
        let resp = promote_inner(
            d.path(),
            &CharismaPromoteArgs {
                kind: CharismaAssetKind::Motion,
                asset_id: "lmo_bow".into(),
            },
        )
        .unwrap();
        assert!(!resp.plan_id.is_empty());
        assert!(resp.stat.promoted_at.is_some());
        let plan_path =
            crate::coding::multi_agent::plans_dir(d.path()).join(format!("{}.yaml", resp.plan_id));
        assert!(plan_path.exists(), "promotion plan YAML missing");
    }

    #[test]
    fn delete_removes_stat() {
        let d = dir();
        record(&d, CharismaAssetKind::Expression, "lex_x", "X");
        delete_inner(
            d.path(),
            &CharismaDeleteArgs {
                kind: CharismaAssetKind::Expression,
                asset_id: "lex_x".into(),
            },
        )
        .unwrap();
        let resp = list_inner(d.path()).unwrap();
        assert_eq!(resp.summary.total, 0);
    }

    #[test]
    fn list_pins_canon_below_others() {
        let d = dir();
        // Untested (newly taught) first
        record(&d, CharismaAssetKind::Expression, "fresh", "Fresh");
        // Promote an older asset to canon
        for _ in 0..12 {
            record(&d, CharismaAssetKind::Motion, "old", "Old");
        }
        for _ in 0..2 {
            rate(&d, CharismaAssetKind::Motion, "old", "Old", 5);
        }
        promote_inner(
            d.path(),
            &CharismaPromoteArgs {
                kind: CharismaAssetKind::Motion,
                asset_id: "old".into(),
            },
        )
        .unwrap();

        let resp = list_inner(d.path()).unwrap();
        assert_eq!(resp.stats.len(), 2);
        assert_eq!(resp.stats[0].asset_id, "fresh");
        assert_eq!(resp.stats[1].asset_id, "old");
    }
}
