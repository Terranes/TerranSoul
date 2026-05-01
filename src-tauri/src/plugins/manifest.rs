//! Plugin manifest — VS Code-style extension point declarations.
//!
//! Every TerranSoul plugin ships a `terransoul-plugin.json` manifest that
//! declares its identity, activation events, and contributions. This is
//! directly inspired by VS Code's `package.json` `contributes` section
//! and OpenClaw's capability model.
//!
//! # VS Code extension model parallels
//!
//! | VS Code concept        | TerranSoul equivalent                  |
//! |------------------------|----------------------------------------|
//! | `activationEvents`     | `ActivationEvent` enum                 |
//! | `contributes.commands` | `ContributedCommand`                   |
//! | `contributes.views`    | `ContributedView`                      |
//! | `contributes.settings` | `ContributedSetting`                   |
//! | `contributes.themes`   | `ContributedTheme`                     |
//! | `extensionKind`        | `PluginKind` (Agent/Tool/Theme/Widget) |
//! | `capabilities`         | `Capability` (reused from manifest.rs) |
//! | Extension Host         | `PluginHost` (WASM sandbox + IPC)      |

use serde::{Deserialize, Serialize};

use crate::package_manager::manifest::{Capability, InstallMethod, SystemRequirements};

// ── Plugin Manifest ──────────────────────────────────────────────────────

/// Full plugin manifest (`terransoul-plugin.json`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginManifest {
    /// Unique plugin ID (lowercase, alphanumeric + hyphens, 1–64 chars).
    pub id: String,
    /// Human-readable display name.
    pub display_name: String,
    /// Semantic version string.
    pub version: String,
    /// Short description (shown in marketplace cards).
    pub description: String,
    /// What kind of plugin this is.
    pub kind: PluginKind,
    /// How to install/run the plugin.
    pub install_method: InstallMethod,
    /// Capabilities the plugin requires.
    #[serde(default)]
    pub capabilities: Vec<Capability>,
    /// When this plugin should be activated.
    #[serde(default)]
    pub activation_events: Vec<ActivationEvent>,
    /// What the plugin contributes to the host app.
    #[serde(default)]
    pub contributes: Contributions,
    /// System requirements.
    #[serde(default)]
    pub system_requirements: Option<SystemRequirements>,
    /// Plugin API version this plugin targets.
    #[serde(default = "default_api_version")]
    pub api_version: u32,
    /// Optional homepage URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    /// Optional SPDX license identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// Optional author name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Optional icon path (relative to plugin root).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Publisher ID for Ed25519 verification.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    /// Detached Ed25519 signature (hex, 128 chars).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    /// SHA-256 hash of the plugin binary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    /// Other plugins this one depends on.
    #[serde(default)]
    pub dependencies: Vec<PluginDependency>,
}

fn default_api_version() -> u32 {
    1
}

/// Plugin kind — determines how the host treats the plugin.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginKind {
    /// Chat agent — implements the AgentProvider trait.
    Agent,
    /// Utility tool — contributes commands and/or memory processors.
    Tool,
    /// Visual theme — contributes CSS custom properties.
    Theme,
    /// UI widget — contributes a panel/view to the app shell.
    Widget,
    /// Memory processor — hooks into the RAG pipeline.
    MemoryProcessor,
}

// ── Activation Events ────────────────────────────────────────────────────

/// When should the plugin be activated (loaded into memory)?
///
/// Follows VS Code's activation event model:
/// - Lazy by default — plugins are not loaded until triggered.
/// - `OnStartup` is discouraged (increases launch time).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActivationEvent {
    /// Activate immediately on app start. Use sparingly.
    OnStartup,
    /// Activate when a contributed command is invoked.
    OnCommand { command: String },
    /// Activate when a specific view becomes visible.
    OnView { view_id: String },
    /// Activate when a chat message matches a pattern.
    OnChatMessage { pattern: String },
    /// Activate when a memory is created/updated with a matching tag.
    OnMemoryTag { tag: String },
    /// Activate when the user navigates to the marketplace tab.
    OnMarketplace,
    /// Activate when the brain mode changes.
    OnBrainModeChange,
    /// Activate when a specific capability is granted.
    OnCapabilityGranted { capability: String },
}

// ── Contributions ────────────────────────────────────────────────────────

/// What the plugin contributes to TerranSoul (VS Code's `contributes`).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Contributions {
    /// Commands the plugin registers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<ContributedCommand>,
    /// Views/panels the plugin adds to the UI.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub views: Vec<ContributedView>,
    /// Settings the plugin declares.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub settings: Vec<ContributedSetting>,
    /// CSS theme overrides.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub themes: Vec<ContributedTheme>,
    /// Slash-commands for the chat input.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub slash_commands: Vec<ContributedSlashCommand>,
    /// Memory pipeline hooks.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub memory_hooks: Vec<ContributedMemoryHook>,
}

/// A command contributed by a plugin (like VS Code's `commands`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContributedCommand {
    /// Unique command ID (e.g. "myplugin.doSomething").
    pub id: String,
    /// Human-readable title shown in the UI.
    pub title: String,
    /// Optional icon (SVG path or emoji).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Optional keyboard shortcut.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keybinding: Option<String>,
    /// Optional category for grouping in command palette.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

/// A view/panel contributed by a plugin.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContributedView {
    /// Unique view ID.
    pub id: String,
    /// Title shown in the tab/panel header.
    pub title: String,
    /// Where this view appears.
    pub location: ViewLocation,
    /// Optional icon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

/// Where a contributed view appears in the app shell.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ViewLocation {
    /// A new tab in the main tab bar.
    MainTab,
    /// A panel inside the chat view sidebar.
    ChatSidebar,
    /// A panel inside the brain view.
    BrainPanel,
    /// A panel inside the memory view.
    MemoryPanel,
    /// A floating overlay widget.
    Overlay,
}

/// A setting contributed by a plugin.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContributedSetting {
    /// Setting key (e.g. "myplugin.maxRetries").
    pub key: String,
    /// Human-readable label.
    pub label: String,
    /// Description shown in the settings UI.
    pub description: String,
    /// Default value (JSON).
    pub default_value: serde_json::Value,
    /// Value type for validation.
    pub value_type: SettingValueType,
}

/// Types for plugin settings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SettingValueType {
    String,
    Number,
    Boolean,
    /// Enum: list of allowed string values.
    Enum {
        values: Vec<String>,
    },
}

/// A CSS theme contributed by a plugin.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContributedTheme {
    /// Theme ID.
    pub id: String,
    /// Display name.
    pub label: String,
    /// CSS custom property overrides (key = `--ts-*` token, value = CSS value).
    pub tokens: std::collections::HashMap<String, String>,
}

/// A slash-command contributed by a plugin (e.g. `/translate`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContributedSlashCommand {
    /// The command word (without slash), e.g. "translate".
    pub name: String,
    /// Description shown in the autocomplete popup.
    pub description: String,
    /// The command ID to invoke when this slash-command is triggered.
    pub command_id: String,
}

/// A memory pipeline hook contributed by a plugin.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContributedMemoryHook {
    /// Hook ID.
    pub id: String,
    /// When this hook fires.
    pub stage: MemoryHookStage,
    /// Description.
    pub description: String,
}

/// When a memory hook fires in the RAG pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryHookStage {
    /// Before a memory is stored (can transform or reject).
    PreStore,
    /// After a memory is stored (for indexing, notifications).
    PostStore,
    /// During retrieval (can rerank or filter results).
    OnRetrieve,
    /// During consolidation (sleep-time processing).
    OnConsolidate,
}

/// A dependency on another plugin.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginDependency {
    /// Plugin ID of the dependency.
    pub id: String,
    /// Minimum version (semver).
    pub version: String,
}

// ── Plugin State ─────────────────────────────────────────────────────────

/// Runtime state of a loaded plugin.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PluginState {
    /// Plugin manifest is known but not activated yet.
    Installed,
    /// Plugin is loaded and running.
    Active,
    /// Plugin was deactivated by the user.
    Disabled,
    /// Plugin failed to activate.
    Error { message: String },
}

/// Installed plugin record (manifest + runtime state).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPlugin {
    pub manifest: PluginManifest,
    pub state: PluginState,
    /// Epoch seconds when installed.
    pub installed_at: i64,
    /// Epoch seconds of last activation.
    pub last_active_at: Option<i64>,
}

// ── Validation ───────────────────────────────────────────────────────────

/// Plugin manifest validation errors.
#[derive(Debug, Clone, PartialEq)]
pub enum PluginManifestError {
    ParseError(String),
    InvalidId(String),
    InvalidVersion(String),
    EmptyDescription,
    EmptyDisplayName,
    UnsupportedApiVersion(u32),
    InvalidCommandId(String),
    InvalidDependency(String),
}

impl std::fmt::Display for PluginManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseError(e) => write!(f, "plugin manifest: parse error: {e}"),
            Self::InvalidId(id) => write!(f, "plugin manifest: invalid id: {id}"),
            Self::InvalidVersion(v) => write!(f, "plugin manifest: invalid version: {v}"),
            Self::EmptyDescription => write!(f, "plugin manifest: description is empty"),
            Self::EmptyDisplayName => write!(f, "plugin manifest: display_name is empty"),
            Self::UnsupportedApiVersion(v) => {
                write!(
                    f,
                    "plugin manifest: unsupported api_version {v} (supported: 1)"
                )
            }
            Self::InvalidCommandId(id) => {
                write!(
                    f,
                    "plugin manifest: invalid command id '{id}' (must contain a dot)"
                )
            }
            Self::InvalidDependency(d) => {
                write!(f, "plugin manifest: invalid dependency: {d}")
            }
        }
    }
}

/// Current plugin API version.
pub const PLUGIN_API_VERSION: u32 = 1;

/// Parse a JSON string into a `PluginManifest`.
pub fn parse_plugin_manifest(json: &str) -> Result<PluginManifest, PluginManifestError> {
    let manifest: PluginManifest =
        serde_json::from_str(json).map_err(|e| PluginManifestError::ParseError(e.to_string()))?;
    validate_plugin_manifest(&manifest)?;
    Ok(manifest)
}

/// Validate an already-deserialized plugin manifest.
pub fn validate_plugin_manifest(m: &PluginManifest) -> Result<(), PluginManifestError> {
    // ID: 1-64 chars, lowercase alphanumeric + hyphens
    if m.id.is_empty() || m.id.len() > 64 {
        return Err(PluginManifestError::InvalidId(format!(
            "id must be 1–64 chars, got {}",
            m.id.len()
        )));
    }
    if !m
        .id
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(PluginManifestError::InvalidId(
            "id must be lowercase alphanumeric and hyphens".into(),
        ));
    }
    if m.display_name.trim().is_empty() {
        return Err(PluginManifestError::EmptyDisplayName);
    }
    if m.description.trim().is_empty() {
        return Err(PluginManifestError::EmptyDescription);
    }
    // Version: basic semver check
    let parts: Vec<&str> = m.version.split('.').collect();
    if parts.len() != 3 || parts.iter().any(|p| p.parse::<u32>().is_err()) {
        return Err(PluginManifestError::InvalidVersion(m.version.clone()));
    }
    // API version
    if m.api_version != PLUGIN_API_VERSION {
        return Err(PluginManifestError::UnsupportedApiVersion(m.api_version));
    }
    // Command IDs must contain a dot (namespace.command)
    for cmd in &m.contributes.commands {
        if !cmd.id.contains('.') {
            return Err(PluginManifestError::InvalidCommandId(cmd.id.clone()));
        }
    }
    // Dependencies must have non-empty id and valid semver
    for dep in &m.dependencies {
        if dep.id.is_empty() {
            return Err(PluginManifestError::InvalidDependency(
                "dependency id is empty".into(),
            ));
        }
        let dp: Vec<&str> = dep.version.split('.').collect();
        if dp.len() != 3 || dp.iter().any(|p| p.parse::<u32>().is_err()) {
            return Err(PluginManifestError::InvalidDependency(format!(
                "invalid version '{}' for dependency '{}'",
                dep.version, dep.id
            )));
        }
    }
    Ok(())
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_manifest() -> PluginManifest {
        PluginManifest {
            id: "test-plugin".into(),
            display_name: "Test Plugin".into(),
            version: "1.0.0".into(),
            description: "A test plugin".into(),
            kind: PluginKind::Tool,
            install_method: InstallMethod::BuiltIn,
            capabilities: vec![],
            activation_events: vec![],
            contributes: Contributions::default(),
            system_requirements: None,
            api_version: 1,
            homepage: None,
            license: None,
            author: None,
            icon: None,
            publisher: None,
            signature: None,
            sha256: None,
            dependencies: vec![],
        }
    }

    #[test]
    fn valid_manifest_passes() {
        assert!(validate_plugin_manifest(&minimal_manifest()).is_ok());
    }

    #[test]
    fn empty_id_rejected() {
        let mut m = minimal_manifest();
        m.id = String::new();
        assert!(matches!(
            validate_plugin_manifest(&m),
            Err(PluginManifestError::InvalidId(_))
        ));
    }

    #[test]
    fn uppercase_id_rejected() {
        let mut m = minimal_manifest();
        m.id = "Test-Plugin".into();
        assert!(matches!(
            validate_plugin_manifest(&m),
            Err(PluginManifestError::InvalidId(_))
        ));
    }

    #[test]
    fn empty_display_name_rejected() {
        let mut m = minimal_manifest();
        m.display_name = "  ".into();
        assert!(matches!(
            validate_plugin_manifest(&m),
            Err(PluginManifestError::EmptyDisplayName)
        ));
    }

    #[test]
    fn empty_description_rejected() {
        let mut m = minimal_manifest();
        m.description = String::new();
        assert!(matches!(
            validate_plugin_manifest(&m),
            Err(PluginManifestError::EmptyDescription)
        ));
    }

    #[test]
    fn bad_version_rejected() {
        let mut m = minimal_manifest();
        m.version = "1.0".into();
        assert!(matches!(
            validate_plugin_manifest(&m),
            Err(PluginManifestError::InvalidVersion(_))
        ));
    }

    #[test]
    fn unsupported_api_version_rejected() {
        let mut m = minimal_manifest();
        m.api_version = 99;
        assert!(matches!(
            validate_plugin_manifest(&m),
            Err(PluginManifestError::UnsupportedApiVersion(99))
        ));
    }

    #[test]
    fn command_without_dot_rejected() {
        let mut m = minimal_manifest();
        m.contributes.commands.push(ContributedCommand {
            id: "no-dot".into(),
            title: "Bad".into(),
            icon: None,
            keybinding: None,
            category: None,
        });
        assert!(matches!(
            validate_plugin_manifest(&m),
            Err(PluginManifestError::InvalidCommandId(_))
        ));
    }

    #[test]
    fn valid_command_accepted() {
        let mut m = minimal_manifest();
        m.contributes.commands.push(ContributedCommand {
            id: "test.doSomething".into(),
            title: "Do Something".into(),
            icon: None,
            keybinding: None,
            category: Some("Test".into()),
        });
        assert!(validate_plugin_manifest(&m).is_ok());
    }

    #[test]
    fn valid_dependency_accepted() {
        let mut m = minimal_manifest();
        m.dependencies.push(PluginDependency {
            id: "base-plugin".into(),
            version: "1.0.0".into(),
        });
        assert!(validate_plugin_manifest(&m).is_ok());
    }

    #[test]
    fn bad_dependency_version_rejected() {
        let mut m = minimal_manifest();
        m.dependencies.push(PluginDependency {
            id: "base-plugin".into(),
            version: "bad".into(),
        });
        assert!(matches!(
            validate_plugin_manifest(&m),
            Err(PluginManifestError::InvalidDependency(_))
        ));
    }

    #[test]
    fn json_roundtrip() {
        let mut m = minimal_manifest();
        m.activation_events = vec![
            ActivationEvent::OnStartup,
            ActivationEvent::OnCommand {
                command: "test.run".into(),
            },
        ];
        m.contributes.views.push(ContributedView {
            id: "test.panel".into(),
            title: "Test Panel".into(),
            location: ViewLocation::ChatSidebar,
            icon: None,
        });
        m.contributes.settings.push(ContributedSetting {
            key: "test.maxRetries".into(),
            label: "Max Retries".into(),
            description: "How many times to retry".into(),
            default_value: serde_json::json!(3),
            value_type: SettingValueType::Number,
        });
        m.contributes.slash_commands.push(ContributedSlashCommand {
            name: "translate".into(),
            description: "Translate text".into(),
            command_id: "test.translate".into(),
        });
        let json = serde_json::to_string(&m).unwrap();
        let parsed = parse_plugin_manifest(&json).unwrap();
        assert_eq!(m, parsed);
    }

    #[test]
    fn all_plugin_kinds_roundtrip() {
        for kind in [
            PluginKind::Agent,
            PluginKind::Tool,
            PluginKind::Theme,
            PluginKind::Widget,
            PluginKind::MemoryProcessor,
        ] {
            let mut m = minimal_manifest();
            m.kind = kind.clone();
            let json = serde_json::to_string(&m).unwrap();
            let parsed: PluginManifest = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed.kind, kind);
        }
    }

    #[test]
    fn all_activation_events_roundtrip() {
        let events = vec![
            ActivationEvent::OnStartup,
            ActivationEvent::OnCommand {
                command: "x.y".into(),
            },
            ActivationEvent::OnView {
                view_id: "v".into(),
            },
            ActivationEvent::OnChatMessage {
                pattern: "hello".into(),
            },
            ActivationEvent::OnMemoryTag {
                tag: "personal".into(),
            },
            ActivationEvent::OnMarketplace,
            ActivationEvent::OnBrainModeChange,
            ActivationEvent::OnCapabilityGranted {
                capability: "network".into(),
            },
        ];
        for event in events {
            let json = serde_json::to_string(&event).unwrap();
            let parsed: ActivationEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, event);
        }
    }

    #[test]
    fn memory_hook_stages_roundtrip() {
        for stage in [
            MemoryHookStage::PreStore,
            MemoryHookStage::PostStore,
            MemoryHookStage::OnRetrieve,
            MemoryHookStage::OnConsolidate,
        ] {
            let json = serde_json::to_string(&stage).unwrap();
            let parsed: MemoryHookStage = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, stage);
        }
    }

    #[test]
    fn contributes_defaults_to_empty() {
        let json = r#"{"commands":[]}"#;
        let c: Contributions = serde_json::from_str(json).unwrap();
        assert!(c.views.is_empty());
        assert!(c.settings.is_empty());
    }
}
