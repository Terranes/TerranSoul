//! Teachable capability Tauri commands (Chunk 30.5).

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;

use crate::teachable_capabilities::registry::{
    build_promotion_plan, load_index, save_index, CapabilitySummary, Maturity, TeachableCapability,
};
use crate::AppState;

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityListResponse {
    pub capabilities: Vec<TeachableCapability>,
    pub summary: CapabilitySummary,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CapabilitySetEnabledArgs {
    pub id: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CapabilitySetConfigArgs {
    pub id: String,
    pub config: Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CapabilityUsageArgs {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CapabilityRatingArgs {
    pub id: String,
    pub rating: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CapabilityResetArgs {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CapabilityPromoteArgs {
    pub id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CapabilityPromoteResponse {
    pub plan_id: String,
    pub capability: TeachableCapability,
}

pub(crate) fn list_inner(data_dir: &Path) -> Result<CapabilityListResponse, String> {
    let index = load_index(data_dir, now_ms())?;
    let summary = CapabilitySummary::from_index(&index);
    let mut capabilities: Vec<TeachableCapability> = index.capabilities.values().cloned().collect();
    capabilities.sort_by(|a, b| {
        let ca = a.category.label();
        let cb = b.category.label();
        ca.cmp(cb).then(a.display_name.cmp(&b.display_name))
    });
    Ok(CapabilityListResponse {
        capabilities,
        summary,
    })
}

pub(crate) fn set_enabled_inner(
    data_dir: &Path,
    args: &CapabilitySetEnabledArgs,
) -> Result<TeachableCapability, String> {
    let mut index = load_index(data_dir, now_ms())?;
    let cap = index
        .capabilities
        .get_mut(&args.id)
        .ok_or_else(|| format!("Unknown capability id: {}", args.id))?;
    cap.enabled = args.enabled;
    let snapshot = cap.clone();
    save_index(data_dir, &index)?;
    Ok(snapshot)
}

pub(crate) fn set_config_inner(
    data_dir: &Path,
    args: &CapabilitySetConfigArgs,
) -> Result<TeachableCapability, String> {
    if !args.config.is_object() {
        return Err("config must be a JSON object".to_string());
    }
    let mut index = load_index(data_dir, now_ms())?;
    let cap = index
        .capabilities
        .get_mut(&args.id)
        .ok_or_else(|| format!("Unknown capability id: {}", args.id))?;
    cap.config = args.config.clone();
    let snapshot = cap.clone();
    save_index(data_dir, &index)?;
    Ok(snapshot)
}

pub(crate) fn record_usage_inner(
    data_dir: &Path,
    args: &CapabilityUsageArgs,
) -> Result<TeachableCapability, String> {
    let mut index = load_index(data_dir, now_ms())?;
    let cap = index
        .capabilities
        .get_mut(&args.id)
        .ok_or_else(|| format!("Unknown capability id: {}", args.id))?;
    cap.usage_count = cap.usage_count.saturating_add(1);
    cap.last_used_at = now_ms();
    let snapshot = cap.clone();
    save_index(data_dir, &index)?;
    Ok(snapshot)
}

pub(crate) fn set_rating_inner(
    data_dir: &Path,
    args: &CapabilityRatingArgs,
) -> Result<TeachableCapability, String> {
    let mut index = load_index(data_dir, now_ms())?;
    let cap = index
        .capabilities
        .get_mut(&args.id)
        .ok_or_else(|| format!("Unknown capability id: {}", args.id))?;
    let clamped = args.rating.clamp(1, 5);
    cap.rating_sum = cap.rating_sum.saturating_add(clamped);
    cap.rating_count = cap.rating_count.saturating_add(1);
    let snapshot = cap.clone();
    save_index(data_dir, &index)?;
    Ok(snapshot)
}

pub(crate) fn reset_inner(
    data_dir: &Path,
    args: &CapabilityResetArgs,
) -> Result<TeachableCapability, String> {
    use crate::teachable_capabilities::registry::seed_catalogue;
    let mut index = load_index(data_dir, now_ms())?;
    let seeds = seed_catalogue(now_ms());
    let seed = seeds
        .into_iter()
        .find(|c| c.id == args.id)
        .ok_or_else(|| format!("No factory default for capability id: {}", args.id))?;
    if let Some(existing) = index.capabilities.get(&args.id) {
        let mut merged = seed;
        merged.usage_count = existing.usage_count;
        merged.last_used_at = existing.last_used_at;
        merged.rating_sum = existing.rating_sum;
        merged.rating_count = existing.rating_count;
        merged.promoted_at = existing.promoted_at;
        merged.last_promotion_plan_id = existing.last_promotion_plan_id.clone();
        index.capabilities.insert(args.id.clone(), merged);
    }
    let snapshot = index
        .capabilities
        .get(&args.id)
        .cloned()
        .ok_or_else(|| "capability missing after reset".to_string())?;
    save_index(data_dir, &index)?;
    Ok(snapshot)
}

pub(crate) fn promote_inner(
    data_dir: &Path,
    args: &CapabilityPromoteArgs,
) -> Result<CapabilityPromoteResponse, String> {
    let mut index = load_index(data_dir, now_ms())?;
    let cap = index
        .capabilities
        .get(&args.id)
        .cloned()
        .ok_or_else(|| format!("Unknown capability id: {}", args.id))?;

    if cap.maturity() != Maturity::Proven {
        return Err(format!(
            "Capability is not yet Proven (current maturity: {:?}). Need at least 10 uses, average rating at least 4.0, and the capability enabled.",
            cap.maturity()
        ));
    }

    let plan_id = crate::coding::multi_agent::new_plan_id();
    let plan = build_promotion_plan(&cap, plan_id.clone(), now_ms());
    crate::coding::multi_agent::save_plan(data_dir, &plan)?;

    if let Some(stored) = index.capabilities.get_mut(&args.id) {
        stored.promoted_at = Some(now_ms());
        stored.last_promotion_plan_id = Some(plan_id.clone());
    }
    save_index(data_dir, &index)?;

    let capability = index
        .capabilities
        .get(&args.id)
        .cloned()
        .ok_or_else(|| "capability missing after promotion".to_string())?;
    Ok(CapabilityPromoteResponse {
        plan_id,
        capability,
    })
}

pub(crate) fn summary_inner(data_dir: &Path) -> Result<CapabilitySummary, String> {
    let index = load_index(data_dir, now_ms())?;
    Ok(CapabilitySummary::from_index(&index))
}

#[tauri::command]
pub async fn teachable_capabilities_list(
    state: State<'_, AppState>,
) -> Result<CapabilityListResponse, String> {
    list_inner(&state.data_dir)
}

#[tauri::command]
pub async fn teachable_capabilities_set_enabled(
    args: CapabilitySetEnabledArgs,
    state: State<'_, AppState>,
) -> Result<TeachableCapability, String> {
    set_enabled_inner(&state.data_dir, &args)
}

#[tauri::command]
pub async fn teachable_capabilities_set_config(
    args: CapabilitySetConfigArgs,
    state: State<'_, AppState>,
) -> Result<TeachableCapability, String> {
    set_config_inner(&state.data_dir, &args)
}

#[tauri::command]
pub async fn teachable_capabilities_record_usage(
    args: CapabilityUsageArgs,
    state: State<'_, AppState>,
) -> Result<TeachableCapability, String> {
    record_usage_inner(&state.data_dir, &args)
}

#[tauri::command]
pub async fn teachable_capabilities_set_rating(
    args: CapabilityRatingArgs,
    state: State<'_, AppState>,
) -> Result<TeachableCapability, String> {
    set_rating_inner(&state.data_dir, &args)
}

#[tauri::command]
pub async fn teachable_capabilities_reset(
    args: CapabilityResetArgs,
    state: State<'_, AppState>,
) -> Result<TeachableCapability, String> {
    reset_inner(&state.data_dir, &args)
}

#[tauri::command]
pub async fn teachable_capabilities_promote(
    args: CapabilityPromoteArgs,
    state: State<'_, AppState>,
) -> Result<CapabilityPromoteResponse, String> {
    promote_inner(&state.data_dir, &args)
}

#[tauri::command]
pub async fn teachable_capabilities_summary(
    state: State<'_, AppState>,
) -> Result<CapabilitySummary, String> {
    summary_inner(&state.data_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    fn tmp() -> TempDir {
        TempDir::new().unwrap()
    }

    #[test]
    fn list_returns_all_seeded_capabilities_sorted_by_category() {
        let t = tmp();
        let resp = list_inner(t.path()).unwrap();
        assert_eq!(resp.capabilities.len(), 17);
        assert_eq!(resp.summary.total, 17);

        let labels: Vec<&str> = resp
            .capabilities
            .iter()
            .map(|c| c.category.label())
            .collect();
        let mut groups: Vec<&str> = labels.clone();
        groups.dedup();
        let mut distinct = labels.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(groups.len(), distinct.len());
    }

    #[test]
    fn set_enabled_persists() {
        let t = tmp();
        let _ = list_inner(t.path()).unwrap();
        let cap = set_enabled_inner(
            t.path(),
            &CapabilitySetEnabledArgs {
                id: "wake_word".into(),
                enabled: true,
            },
        )
        .unwrap();
        assert!(cap.enabled);

        let resp = list_inner(t.path()).unwrap();
        let wake = resp
            .capabilities
            .iter()
            .find(|c| c.id == "wake_word")
            .unwrap();
        assert!(wake.enabled);
    }

    #[test]
    fn set_config_rejects_non_object() {
        let t = tmp();
        let err = set_config_inner(
            t.path(),
            &CapabilitySetConfigArgs {
                id: "wake_word".into(),
                config: json!("a string"),
            },
        )
        .unwrap_err();
        assert!(err.contains("must be a JSON object"));
    }

    #[test]
    fn set_config_persists_user_changes() {
        let t = tmp();
        let cap = set_config_inner(
            t.path(),
            &CapabilitySetConfigArgs {
                id: "wake_word".into(),
                config: json!({ "phrase": "hi terra", "sensitivity": 0.7, "engine": "porcupine" }),
            },
        )
        .unwrap();
        assert_eq!(cap.config["phrase"], "hi terra");
    }

    #[test]
    fn record_usage_and_rating_drives_maturity() {
        let t = tmp();
        set_enabled_inner(
            t.path(),
            &CapabilitySetEnabledArgs {
                id: "wake_word".into(),
                enabled: true,
            },
        )
        .unwrap();

        for _ in 0..12 {
            record_usage_inner(
                t.path(),
                &CapabilityUsageArgs {
                    id: "wake_word".into(),
                },
            )
            .unwrap();
        }
        for _ in 0..3 {
            set_rating_inner(
                t.path(),
                &CapabilityRatingArgs {
                    id: "wake_word".into(),
                    rating: 5,
                },
            )
            .unwrap();
        }

        let resp = list_inner(t.path()).unwrap();
        let wake = resp
            .capabilities
            .iter()
            .find(|c| c.id == "wake_word")
            .unwrap();
        assert_eq!(wake.maturity(), Maturity::Proven);
    }

    #[test]
    fn promote_rejects_non_proven() {
        let t = tmp();
        let err = promote_inner(
            t.path(),
            &CapabilityPromoteArgs {
                id: "wake_word".into(),
            },
        )
        .unwrap_err();
        assert!(err.contains("not yet Proven"));
    }

    #[test]
    fn promote_succeeds_for_proven_and_marks_canon() {
        let t = tmp();
        set_enabled_inner(
            t.path(),
            &CapabilitySetEnabledArgs {
                id: "wake_word".into(),
                enabled: true,
            },
        )
        .unwrap();
        for _ in 0..10 {
            record_usage_inner(
                t.path(),
                &CapabilityUsageArgs {
                    id: "wake_word".into(),
                },
            )
            .unwrap();
        }
        for _ in 0..2 {
            set_rating_inner(
                t.path(),
                &CapabilityRatingArgs {
                    id: "wake_word".into(),
                    rating: 5,
                },
            )
            .unwrap();
        }

        let resp = promote_inner(
            t.path(),
            &CapabilityPromoteArgs {
                id: "wake_word".into(),
            },
        )
        .unwrap();
        assert!(uuid::Uuid::parse_str(&resp.plan_id).is_ok());
        assert_eq!(resp.capability.maturity(), Maturity::Canon);
    }

    #[test]
    fn reset_keeps_usage_history_but_restores_default_config() {
        let t = tmp();
        set_config_inner(
            t.path(),
            &CapabilitySetConfigArgs {
                id: "wake_word".into(),
                config: json!({ "phrase": "custom", "sensitivity": 0.9, "engine": "porcupine" }),
            },
        )
        .unwrap();
        record_usage_inner(
            t.path(),
            &CapabilityUsageArgs {
                id: "wake_word".into(),
            },
        )
        .unwrap();
        record_usage_inner(
            t.path(),
            &CapabilityUsageArgs {
                id: "wake_word".into(),
            },
        )
        .unwrap();

        let cap = reset_inner(
            t.path(),
            &CapabilityResetArgs {
                id: "wake_word".into(),
            },
        )
        .unwrap();
        assert_eq!(cap.config["phrase"], "hey terra");
        assert_eq!(cap.usage_count, 2);
    }
}
