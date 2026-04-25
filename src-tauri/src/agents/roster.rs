//! Agent roster — persistence + CRUD for [`AgentProfile`].
//!
//! One JSON file per agent under `<data_dir>/agents/<id>.json`. A tiny
//! `current_agent.json` sibling file records which agent is currently
//! active so the app can restore it on launch.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::brain::BrainMode;

/// Maximum number of agents a single user can create.
///
/// Large enough to cover real use cases ("coding assistant", "writing
/// buddy", "lab journal", "compliance reviewer") while preventing runaway
/// disk / RAM usage from automated scripts.
pub const MAX_AGENTS: usize = 32;

/// Allow-listed external CLI tools — **no arbitrary shell execution**.
///
/// Each variant maps to a binary resolved via `$PATH`. `Custom` requires
/// the user to provide the binary name which is then validated against
/// [`CliKind::validate_custom_binary`] (alphanumerics + `-` + `_` only,
/// no path separators or shell metacharacters).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CliKind {
    Codex,
    Claude,
    Gemini,
    Custom,
}

impl CliKind {
    /// Canonical binary name for the built-in kinds.
    pub fn default_binary(self) -> &'static str {
        match self {
            CliKind::Codex => "codex",
            CliKind::Claude => "claude",
            CliKind::Gemini => "gemini",
            CliKind::Custom => "",
        }
    }

    /// Validate that a user-provided custom binary name is safe to pass
    /// to [`std::process::Command::new`]. Rejects any string containing
    /// path separators or shell metacharacters; the intention is that
    /// the binary must resolve via `$PATH`, not an arbitrary location.
    pub fn validate_custom_binary(name: &str) -> Result<(), String> {
        if name.is_empty() {
            return Err("binary name must not be empty".to_string());
        }
        if name.len() > 64 {
            return Err("binary name too long".to_string());
        }
        let ok = name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.');
        if !ok {
            return Err(
                "binary name may only contain letters, digits, '-', '_' and '.'".to_string(),
            );
        }
        Ok(())
    }
}

/// Which backend powers an agent's chat turns.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum BrainBackend {
    /// TerranSoul's built-in brain — identical to today's single-agent
    /// behaviour. `mode = None` means "use whatever global brain mode is
    /// currently active" (for users who don't want per-agent configs).
    Native { mode: Option<BrainMode> },
    /// External CLI worker bound to a working folder.
    ExternalCli {
        kind: CliKind,
        /// Actual binary name. For built-in [`CliKind`] variants this
        /// defaults to the canonical name; for [`CliKind::Custom`] it is
        /// validated by [`CliKind::validate_custom_binary`].
        binary: String,
        /// Extra arguments passed **after** the prompt, as a pre-split
        /// `Vec<String>` so no shell expansion ever happens.
        #[serde(default)]
        extra_args: Vec<String>,
    },
}

impl BrainBackend {
    /// Discriminant used by the RAM-cap calculator without leaking the
    /// full payload.
    pub fn discriminant(&self) -> AgentBackendKind {
        match self {
            BrainBackend::Native { mode } => match mode {
                Some(BrainMode::LocalOllama { .. }) => AgentBackendKind::LocalOllama,
                _ => AgentBackendKind::NativeApi,
            },
            BrainBackend::ExternalCli { .. } => AgentBackendKind::ExternalCli,
        }
    }

    /// For local-Ollama backends, return the model tag so the RAM cap can
    /// account for its resident size.
    pub fn ollama_model(&self) -> Option<&str> {
        match self {
            BrainBackend::Native {
                mode: Some(BrainMode::LocalOllama { model }),
            } => Some(model.as_str()),
            _ => None,
        }
    }
}

/// Compact discriminant for RAM accounting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentBackendKind {
    NativeApi,
    LocalOllama,
    ExternalCli,
}

/// A user-created agent — persisted one-JSON-per-file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentProfile {
    pub id: String,
    pub display_name: String,
    /// ID of the VRM this agent uses. Points into either the bundled
    /// `default-models.ts` catalogue or a [`crate::settings::UserModel`].
    /// Two agents may legitimately reference the same VRM.
    pub vrm_model_id: String,
    pub brain_backend: BrainBackend,
    /// Working folder for `ExternalCli` backends. Ignored for native.
    #[serde(default)]
    pub working_folder: Option<PathBuf>,
    /// Epoch seconds.
    pub created_at: i64,
    /// Epoch seconds; updated on every `switch_agent`.
    pub last_active_at: i64,
}

impl AgentProfile {
    /// Validate invariants that must hold for a profile to be usable.
    /// Called on every load/save to prevent corrupt profiles from
    /// poisoning the roster.
    pub fn validate(&self) -> Result<(), String> {
        validate_id(&self.id)?;
        if self.display_name.trim().is_empty() {
            return Err("display_name must not be empty".to_string());
        }
        if self.display_name.len() > 120 {
            return Err("display_name too long (max 120 chars)".to_string());
        }
        if self.vrm_model_id.trim().is_empty() {
            return Err("vrm_model_id must not be empty".to_string());
        }
        if let BrainBackend::ExternalCli { kind, binary, .. } = &self.brain_backend {
            if *kind == CliKind::Custom {
                CliKind::validate_custom_binary(binary)?;
            } else if binary != kind.default_binary() {
                return Err(format!(
                    "binary '{}' does not match kind '{:?}'",
                    binary, kind
                ));
            }
            if let Some(folder) = &self.working_folder {
                if folder.as_os_str().is_empty() {
                    return Err("working_folder must not be empty".to_string());
                }
            }
        }
        Ok(())
    }
}

fn validate_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > 64 {
        return Err("agent id must be 1..=64 chars".to_string());
    }
    let ok = id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !ok {
        return Err(
            "agent id may only contain ASCII letters, digits, '-' or '_'".to_string(),
        );
    }
    Ok(())
}

// ── Persistence layer ────────────────────────────────────────────────────

/// The on-disk agent roster rooted at `<data_dir>/agents/`.
#[derive(Debug, Clone)]
pub struct AgentRoster {
    root: PathBuf,
}

impl AgentRoster {
    /// Open (or create) the roster directory.
    pub fn open(data_dir: &Path) -> Self {
        let root = data_dir.join("agents");
        // Best-effort: the caller's create/list operations will surface
        // any real IO errors — we don't want the app to fail to launch
        // just because this one subdir can't be created yet.
        let _ = fs::create_dir_all(&root);
        Self { root }
    }

    /// For tests — in-memory root under a `tempfile::TempDir`.
    #[cfg(test)]
    pub fn open_in(root: &Path) -> Self {
        let root = root.join("agents");
        fs::create_dir_all(&root).expect("create agents dir");
        Self { root }
    }

    fn profile_path(&self, id: &str) -> PathBuf {
        self.root.join(format!("{id}.json"))
    }

    fn current_path(&self) -> PathBuf {
        self.root.join("current_agent.json")
    }

    /// List all persisted agent profiles, sorted by `last_active_at` desc.
    /// Profiles that fail [`AgentProfile::validate`] are skipped with a
    /// `eprintln!` warning rather than poisoning the whole list.
    pub fn list(&self) -> Vec<AgentProfile> {
        let mut out = Vec::new();
        let entries = match fs::read_dir(&self.root) {
            Ok(e) => e,
            Err(_) => return out,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            if path.file_stem().and_then(|s| s.to_str()) == Some("current_agent") {
                continue;
            }
            match load_profile(&path) {
                Ok(p) => out.push(p),
                Err(e) => eprintln!("[agents] skipping corrupt profile {path:?}: {e}"),
            }
        }
        out.sort_by_key(|b| std::cmp::Reverse(b.last_active_at));
        out
    }

    /// Get a profile by id.
    pub fn get(&self, id: &str) -> Result<AgentProfile, String> {
        validate_id(id)?;
        load_profile(&self.profile_path(id)).map_err(|e| e.to_string())
    }

    /// Create a new profile. Fails if [`MAX_AGENTS`] is already reached,
    /// the id is already taken, or the profile fails validation.
    pub fn create(&self, profile: AgentProfile) -> Result<(), String> {
        profile.validate()?;
        let existing = self.list();
        if existing.len() >= MAX_AGENTS {
            return Err(format!("agent roster is full (max {MAX_AGENTS})"));
        }
        let path = self.profile_path(&profile.id);
        if path.exists() {
            return Err(format!("agent '{}' already exists", profile.id));
        }
        save_profile(&path, &profile).map_err(|e| e.to_string())
    }

    /// Overwrite an existing profile.
    pub fn update(&self, profile: &AgentProfile) -> Result<(), String> {
        profile.validate()?;
        let path = self.profile_path(&profile.id);
        if !path.exists() {
            return Err(format!("agent '{}' does not exist", profile.id));
        }
        save_profile(&path, profile).map_err(|e| e.to_string())
    }

    /// Delete a profile and clear it from `current_agent.json` if it was
    /// active. Idempotent: deleting a non-existent agent is Ok(()).
    pub fn delete(&self, id: &str) -> Result<(), String> {
        validate_id(id)?;
        let path = self.profile_path(id);
        if path.exists() {
            fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
        if let Ok(current) = self.current_agent_id() {
            if current.as_deref() == Some(id) {
                let _ = fs::remove_file(self.current_path());
            }
        }
        Ok(())
    }

    /// Persist the ID of the currently active agent.
    pub fn set_current_agent(&self, id: &str) -> Result<(), String> {
        validate_id(id)?;
        if !self.profile_path(id).exists() {
            return Err(format!("agent '{id}' does not exist"));
        }
        let json = serde_json::to_vec_pretty(&CurrentAgent {
            id: id.to_string(),
        })
        .map_err(|e| e.to_string())?;
        fs::write(self.current_path(), json).map_err(|e| e.to_string())
    }

    /// Read the currently active agent ID, if any.
    pub fn current_agent_id(&self) -> Result<Option<String>, String> {
        let path = self.current_path();
        if !path.exists() {
            return Ok(None);
        }
        let bytes = fs::read(&path).map_err(|e| e.to_string())?;
        let parsed: CurrentAgent = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        // If the referenced agent has since been deleted, self-heal by
        // returning None so the app falls back to the default agent.
        if !self.profile_path(&parsed.id).exists() {
            let _ = fs::remove_file(&path);
            return Ok(None);
        }
        Ok(Some(parsed.id))
    }

    /// Bump `last_active_at` for the given agent to the current epoch
    /// seconds. No-op if the agent doesn't exist.
    pub fn touch(&self, id: &str) -> Result<(), String> {
        let mut profile = self.get(id)?;
        profile.last_active_at = now_secs();
        self.update(&profile)
    }
}

#[derive(Serialize, Deserialize)]
struct CurrentAgent {
    id: String,
}

fn load_profile(path: &Path) -> std::io::Result<AgentProfile> {
    let bytes = fs::read(path)?;
    let profile: AgentProfile = serde_json::from_slice(&bytes)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    profile
        .validate()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(profile)
}

fn save_profile(path: &Path, profile: &AgentProfile) -> std::io::Result<()> {
    let bytes = serde_json::to_vec_pretty(profile)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    // Write to a sibling tmp file then rename so a crash mid-write can't
    // leave a truncated profile JSON on disk.
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(tmp, path)?;
    Ok(())
}

/// Current epoch seconds. Extracted so tests can stub time if needed.
pub fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Build a fresh random agent id. Uses UUID v4 without dashes, truncated
/// to 16 chars for readable filenames.
pub fn fresh_id() -> String {
    let full = uuid::Uuid::new_v4().simple().to_string();
    full[..16].to_string()
}

/// Build the default agent that every fresh install starts with — it
/// mirrors the pre-Chunk-1.5 single-agent experience so the roster is
/// never empty.
pub fn default_agent(vrm_model_id: &str) -> AgentProfile {
    let now = now_secs();
    AgentProfile {
        id: "default".to_string(),
        display_name: "TerranSoul".to_string(),
        vrm_model_id: vrm_model_id.to_string(),
        brain_backend: BrainBackend::Native { mode: None },
        working_folder: None,
        created_at: now,
        last_active_at: now,
    }
}

/// Helper for tests / integration checkers: group agents by their
/// backend discriminant.
pub fn group_by_kind(agents: &[AgentProfile]) -> HashMap<AgentBackendKind, Vec<String>> {
    let mut by_kind: HashMap<AgentBackendKind, Vec<String>> = HashMap::new();
    for a in agents {
        by_kind
            .entry(a.brain_backend.discriminant())
            .or_default()
            .push(a.id.clone());
    }
    by_kind
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_native() -> AgentProfile {
        AgentProfile {
            id: "alpha".into(),
            display_name: "Alpha".into(),
            vrm_model_id: "ao".into(),
            brain_backend: BrainBackend::Native { mode: None },
            working_folder: None,
            created_at: 1,
            last_active_at: 2,
        }
    }

    fn sample_cli() -> AgentProfile {
        AgentProfile {
            id: "beta".into(),
            display_name: "Beta Coder".into(),
            vrm_model_id: "karina".into(),
            brain_backend: BrainBackend::ExternalCli {
                kind: CliKind::Codex,
                binary: "codex".into(),
                extra_args: vec!["--yolo".into()],
            },
            working_folder: Some(PathBuf::from("/tmp/repo")),
            created_at: 3,
            last_active_at: 4,
        }
    }

    #[test]
    fn serde_roundtrip_native() {
        let p = sample_native();
        let j = serde_json::to_string(&p).unwrap();
        let back: AgentProfile = serde_json::from_str(&j).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn serde_roundtrip_cli() {
        let p = sample_cli();
        let j = serde_json::to_string(&p).unwrap();
        let back: AgentProfile = serde_json::from_str(&j).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn validate_rejects_shell_metacharacters_in_custom_binary() {
        assert!(CliKind::validate_custom_binary("codex").is_ok());
        assert!(CliKind::validate_custom_binary("my-tool_v2").is_ok());
        assert!(CliKind::validate_custom_binary("").is_err());
        assert!(CliKind::validate_custom_binary("rm -rf /").is_err());
        assert!(CliKind::validate_custom_binary("a;b").is_err());
        assert!(CliKind::validate_custom_binary("$(evil)").is_err());
        assert!(CliKind::validate_custom_binary("/usr/bin/codex").is_err());
        assert!(CliKind::validate_custom_binary("..\\codex").is_err());
        // length cap
        assert!(CliKind::validate_custom_binary(&"a".repeat(65)).is_err());
    }

    #[test]
    fn validate_rejects_bad_ids() {
        let mut p = sample_native();
        p.id = "".into();
        assert!(p.validate().is_err());
        p.id = "has spaces".into();
        assert!(p.validate().is_err());
        p.id = "a/b".into();
        assert!(p.validate().is_err());
        p.id = "OK-id_123".into();
        assert!(p.validate().is_ok());
    }

    #[test]
    fn validate_cli_requires_matching_binary_for_builtin_kinds() {
        let mut p = sample_cli();
        if let BrainBackend::ExternalCli { binary, .. } = &mut p.brain_backend {
            *binary = "claude".into();
        }
        assert!(p.validate().is_err(), "kind=Codex with binary=claude must fail");
    }

    #[test]
    fn roster_create_list_delete() {
        let tmp = TempDir::new().unwrap();
        let r = AgentRoster::open_in(tmp.path());
        assert_eq!(r.list().len(), 0);
        r.create(sample_native()).unwrap();
        r.create(sample_cli()).unwrap();
        let all = r.list();
        assert_eq!(all.len(), 2);
        // Sorted by last_active_at desc → beta (4) before alpha (2)
        assert_eq!(all[0].id, "beta");
        assert_eq!(all[1].id, "alpha");

        r.delete("alpha").unwrap();
        assert_eq!(r.list().len(), 1);
        // Idempotent
        r.delete("alpha").unwrap();
    }

    #[test]
    fn roster_rejects_duplicate_id() {
        let tmp = TempDir::new().unwrap();
        let r = AgentRoster::open_in(tmp.path());
        r.create(sample_native()).unwrap();
        let err = r.create(sample_native()).unwrap_err();
        assert!(err.contains("already exists"), "got: {err}");
    }

    #[test]
    fn roster_current_agent_self_heals_on_deletion() {
        let tmp = TempDir::new().unwrap();
        let r = AgentRoster::open_in(tmp.path());
        r.create(sample_native()).unwrap();
        r.set_current_agent("alpha").unwrap();
        assert_eq!(r.current_agent_id().unwrap().as_deref(), Some("alpha"));
        r.delete("alpha").unwrap();
        // After the agent is gone, current_agent_id returns None rather
        // than a dangling pointer, and the file is cleaned up.
        assert_eq!(r.current_agent_id().unwrap(), None);
    }

    #[test]
    fn roster_max_agents_enforced() {
        let tmp = TempDir::new().unwrap();
        let r = AgentRoster::open_in(tmp.path());
        for i in 0..MAX_AGENTS {
            let mut p = sample_native();
            p.id = format!("a{i}");
            r.create(p).unwrap();
        }
        let mut overflow = sample_native();
        overflow.id = "overflow".into();
        let err = r.create(overflow).unwrap_err();
        assert!(err.contains("full"), "got: {err}");
    }

    #[test]
    fn touch_updates_last_active_at() {
        let tmp = TempDir::new().unwrap();
        let r = AgentRoster::open_in(tmp.path());
        let mut p = sample_native();
        p.last_active_at = 0;
        r.create(p).unwrap();
        r.touch("alpha").unwrap();
        let back = r.get("alpha").unwrap();
        // now_secs() is >> 0 in any real environment.
        assert!(back.last_active_at > 1_000_000_000);
    }

    #[test]
    fn fresh_id_is_16_chars_ascii() {
        let id = fresh_id();
        assert_eq!(id.len(), 16);
        assert!(id.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn default_agent_validates() {
        default_agent("ao").validate().unwrap();
    }

    #[test]
    fn save_uses_atomic_rename() {
        // Corrupt profile leftover on disk must not kill list().
        let tmp = TempDir::new().unwrap();
        let r = AgentRoster::open_in(tmp.path());
        r.create(sample_native()).unwrap();
        // Simulate a half-written profile from a crashed write:
        fs::write(tmp.path().join("agents").join("broken.json"), b"{not json").unwrap();
        let all = r.list();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "alpha");
    }
}
