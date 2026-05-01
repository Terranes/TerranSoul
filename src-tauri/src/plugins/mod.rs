//! Plugin system — VS Code-style extensibility for TerranSoul.
//!
//! Plugins are declared via a `terransoul-plugin.json` manifest that follows
//! the same patterns as VS Code's `package.json` extensions:
//!
//! - **Activation events** — lazy loading based on triggers
//! - **Contributions** — commands, views, settings, themes, slash-commands
//! - **Capability-gated** — sensitive operations require user consent
//! - **WASM sandbox** — untrusted plugins run in wasmtime
//!
//! See `docs/plugin-development.md` for the full developer guide.

pub mod host;
pub mod manifest;

pub use manifest::{
    parse_plugin_manifest, validate_plugin_manifest, ActivationEvent, ContributedCommand,
    ContributedMemoryHook, ContributedSetting, ContributedSlashCommand, ContributedTheme,
    ContributedView, Contributions, InstalledPlugin, MemoryHookStage, PluginDependency, PluginKind,
    PluginManifest, PluginManifestError, PluginState, SettingValueType, ViewLocation,
    PLUGIN_API_VERSION,
};

pub use host::{CommandEntry, CommandResult, PluginHost, PluginHostStatus, SlashCommandEntry};
