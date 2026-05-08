//! Plugin host — manages the lifecycle of installed plugins.
//!
//! Inspired by VS Code's Extension Host process model:
//! - Plugins are **lazily activated** based on `ActivationEvent` triggers.
//! - Each plugin gets an isolated `PluginContext` with scoped access to
//!   capabilities, settings, and contributed commands.
//! - WASM plugins run inside the existing `WasmRunner` sandbox.
//! - The host persists installed plugin state to `<data_dir>/plugins/`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::sync::{Mutex as TokioMutex, RwLock};

use super::manifest::{
    ActivationEvent, ContributedCommand, ContributedMemoryHook, ContributedSlashCommand,
    ContributedTheme, Contributions, InstalledPlugin, MemoryHookStage, PluginKind, PluginManifest,
    PluginState,
};
use crate::agent::openclaw_agent::{parse as parse_openclaw, OpenClawTool, ParsedMessage};
use crate::commands::translation::normalize_language_input;
use crate::package_manager::manifest::{Capability as ManifestCapability, InstallMethod};
use crate::sandbox::{Capability as SandboxCapability, CapabilityStore, WasmRunner};

// ── Plugin Host ──────────────────────────────────────────────────────────

/// The plugin host manages the entire plugin lifecycle.
///
/// Thread-safe via `Arc<RwLock<...>>` internally — call `.handle()` to
/// get a cheap clone for passing to async tasks.
#[derive(Debug, Clone)]
pub struct PluginHost {
    inner: Arc<RwLock<HostInner>>,
}

#[derive(Debug)]
struct HostInner {
    /// Root directory for plugin data (`<data_dir>/plugins/`).
    plugins_dir: PathBuf,
    /// All known plugins (installed or active).
    plugins: HashMap<String, InstalledPlugin>,
    /// Aggregated command registry from all active plugins.
    commands: HashMap<String, CommandEntry>,
    /// Aggregated slash-command registry.
    slash_commands: HashMap<String, SlashCommandEntry>,
    /// Aggregated theme registry.
    themes: HashMap<String, ContributedTheme>,
    /// Aggregated memory hook registry from active plugins.
    memory_hooks: HashMap<String, MemoryHookEntry>,
    /// Plugin settings values (plugin_id.key → JSON value).
    settings: HashMap<String, serde_json::Value>,
}

/// A registered command and which plugin owns it.
#[derive(Debug, Clone, Serialize)]
pub struct CommandEntry {
    pub plugin_id: String,
    pub command: ContributedCommand,
}

/// A registered slash-command and which plugin owns it.
#[derive(Debug, Clone, Serialize)]
pub struct SlashCommandEntry {
    pub plugin_id: String,
    pub slash_command: ContributedSlashCommand,
}

/// A registered memory hook and its owning plugin.
#[derive(Debug, Clone, Serialize)]
pub struct MemoryHookEntry {
    pub plugin_id: String,
    pub hook: ContributedMemoryHook,
}

/// Payload sent to memory pipeline hooks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryHookPayload {
    pub stage: MemoryHookStage,
    pub content: String,
    pub tags: String,
    pub importance: i64,
    pub memory_type: String,
    pub entry_id: Option<i64>,
}

/// Patch returned by a PreStore memory hook.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MemoryHookPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub importance: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_type: Option<String>,
}

/// Result of running memory hooks for one stage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryHookRunResult {
    pub payload: MemoryHookPayload,
    pub executed: usize,
    pub errors: Vec<String>,
}

/// Result of executing a plugin command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
}

impl CommandResult {
    fn success(output: impl Into<String>) -> Self {
        Self {
            success: true,
            output: non_empty_string(output.into()),
            error: None,
            exit_code: Some(0),
            stderr: None,
        }
    }

    fn failure(error: impl Into<String>) -> Self {
        Self {
            success: false,
            output: None,
            error: Some(error.into()),
            exit_code: None,
            stderr: None,
        }
    }
}

const PLUGIN_COMMAND_TIMEOUT: Duration = Duration::from_secs(10);

/// Summary of plugin host state for the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct PluginHostStatus {
    pub total_plugins: usize,
    pub active_plugins: usize,
    pub disabled_plugins: usize,
    pub error_plugins: usize,
    pub total_commands: usize,
    pub total_slash_commands: usize,
    pub total_themes: usize,
}

impl PluginHost {
    /// Create a new plugin host with the given data directory.
    pub fn new(data_dir: &Path) -> Self {
        let plugins_dir = data_dir.join("plugins");
        Self::from_inner(HostInner {
            plugins_dir,
            plugins: HashMap::new(),
            commands: HashMap::new(),
            slash_commands: HashMap::new(),
            themes: HashMap::new(),
            memory_hooks: HashMap::new(),
            settings: HashMap::new(),
        })
    }

    /// Create a production plugin host and pre-register built-in reference plugins.
    pub fn with_builtin_plugins(data_dir: &Path) -> Self {
        let host = Self::new(data_dir);
        host.register_builtin_plugins();
        host
    }

    fn from_inner(inner: HostInner) -> Self {
        Self {
            inner: Arc::new(RwLock::new(inner)),
        }
    }

    /// Create an in-memory plugin host for testing.
    pub fn in_memory() -> Self {
        Self::from_inner(HostInner {
            plugins_dir: PathBuf::from(":memory:"),
            plugins: HashMap::new(),
            commands: HashMap::new(),
            slash_commands: HashMap::new(),
            themes: HashMap::new(),
            memory_hooks: HashMap::new(),
            settings: HashMap::new(),
        })
    }

    fn register_builtin_plugins(&self) {
        let Ok(mut inner) = self.inner.try_write() else {
            return;
        };
        for manifest in builtin_manifests() {
            let id = manifest.id.clone();
            let installed = InstalledPlugin {
                manifest: manifest.clone(),
                state: PluginState::Active,
                installed_at: 0,
                last_active_at: Some(0),
            };
            register_contributions(&mut inner, &manifest);
            inner.plugins.entry(id).or_insert(installed);
        }
    }

    /// Cheap clone handle for sharing across tasks.
    pub fn handle(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }

    /// Load all installed plugins from the plugins directory.
    pub async fn load_installed(&self) -> Result<usize, String> {
        let plugins_dir = {
            let inner = self.inner.read().await;
            inner.plugins_dir.clone()
        };

        if plugins_dir == Path::new(":memory:") {
            return Ok(0);
        }

        if !plugins_dir.exists() {
            std::fs::create_dir_all(&plugins_dir).map_err(|e| e.to_string())?;
            return Ok(0);
        }

        let mut count = 0;
        let entries = std::fs::read_dir(&plugins_dir).map_err(|e| e.to_string())?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json") {
                match std::fs::read_to_string(&path) {
                    Ok(json) => {
                        if let Ok(installed) = serde_json::from_str::<InstalledPlugin>(&json) {
                            let id = installed.manifest.id.clone();
                            let mut inner = self.inner.write().await;
                            if installed.state == PluginState::Active {
                                register_contributions(&mut inner, &installed.manifest);
                            }
                            inner.plugins.insert(id, installed);
                            count += 1;
                        }
                    }
                    Err(_) => continue,
                }
            }
        }
        Ok(count)
    }

    /// Install a plugin from its manifest. Sets state to `Installed`.
    pub async fn install(&self, manifest: PluginManifest) -> Result<InstalledPlugin, String> {
        let id = manifest.id.clone();
        let installed = InstalledPlugin {
            manifest,
            state: PluginState::Installed,
            installed_at: now_secs(),
            last_active_at: None,
        };

        let mut inner = self.inner.write().await;
        if inner.plugins.contains_key(&id) {
            return Err(format!("plugin '{id}' is already installed"));
        }
        inner.plugins.insert(id.clone(), installed.clone());
        persist_plugin(&inner.plugins_dir, &installed)?;
        Ok(installed)
    }

    /// Activate an installed plugin.
    pub async fn activate(&self, plugin_id: &str) -> Result<(), String> {
        let mut inner = self.inner.write().await;
        let manifest = {
            let plugin = inner
                .plugins
                .get(plugin_id)
                .ok_or_else(|| format!("plugin '{plugin_id}' not found"))?;
            if plugin.state == PluginState::Active {
                return Ok(()); // Already active
            }
            plugin.manifest.clone()
        };
        register_contributions(&mut inner, &manifest);
        let plugins_dir = inner.plugins_dir.clone();
        let plugin = inner.plugins.get_mut(plugin_id).unwrap();
        plugin.state = PluginState::Active;
        plugin.last_active_at = Some(now_secs());
        persist_plugin(&plugins_dir, plugin)?;
        Ok(())
    }

    /// Deactivate a plugin (remove its contributions).
    pub async fn deactivate(&self, plugin_id: &str) -> Result<(), String> {
        let mut inner = self.inner.write().await;
        if !inner.plugins.contains_key(plugin_id) {
            return Err(format!("plugin '{plugin_id}' not found"));
        }
        unregister_contributions(&mut inner, plugin_id);
        let plugins_dir = inner.plugins_dir.clone();
        let plugin = inner.plugins.get_mut(plugin_id).unwrap();
        plugin.state = PluginState::Disabled;
        persist_plugin(&plugins_dir, plugin)?;
        Ok(())
    }

    /// Uninstall a plugin completely.
    pub async fn uninstall(&self, plugin_id: &str) -> Result<(), String> {
        let mut inner = self.inner.write().await;
        unregister_contributions(&mut inner, plugin_id);
        inner.plugins.remove(plugin_id);
        remove_plugin_file(&inner.plugins_dir, plugin_id)?;
        Ok(())
    }

    /// List all installed plugins.
    pub async fn list_plugins(&self) -> Vec<InstalledPlugin> {
        let inner = self.inner.read().await;
        inner.plugins.values().cloned().collect()
    }

    /// Get a specific plugin.
    pub async fn get_plugin(&self, plugin_id: &str) -> Option<InstalledPlugin> {
        let inner = self.inner.read().await;
        inner.plugins.get(plugin_id).cloned()
    }

    /// List all registered commands from active plugins.
    pub async fn list_commands(&self) -> Vec<CommandEntry> {
        let inner = self.inner.read().await;
        inner.commands.values().cloned().collect()
    }

    /// List all registered slash-commands.
    pub async fn list_slash_commands(&self) -> Vec<SlashCommandEntry> {
        let inner = self.inner.read().await;
        inner.slash_commands.values().cloned().collect()
    }

    /// List all registered themes.
    pub async fn list_themes(&self) -> Vec<ContributedTheme> {
        let inner = self.inner.read().await;
        inner.themes.values().cloned().collect()
    }

    /// List all registered memory hooks from active plugins.
    pub async fn list_memory_hooks(&self) -> Vec<MemoryHookEntry> {
        let inner = self.inner.read().await;
        inner.memory_hooks.values().cloned().collect()
    }

    /// Get a plugin setting value.
    pub async fn get_setting(&self, key: &str) -> Option<serde_json::Value> {
        let inner = self.inner.read().await;
        inner.settings.get(key).cloned()
    }

    /// Update a plugin setting value.
    pub async fn set_setting(&self, key: &str, value: serde_json::Value) {
        let mut inner = self.inner.write().await;
        inner.settings.insert(key.to_string(), value);
    }

    /// Check if an activation event should trigger any plugins.
    pub async fn check_activation(&self, event: &ActivationEvent) -> Vec<String> {
        let inner = self.inner.read().await;
        let mut to_activate = Vec::new();
        for (id, plugin) in &inner.plugins {
            if plugin.state == PluginState::Installed
                && plugin
                    .manifest
                    .activation_events
                    .iter()
                    .any(|e| matches_event(e, event))
            {
                to_activate.push(id.clone());
            }
        }
        to_activate
    }

    /// Activate plugins whose `OnMemoryTag` events match the comma-separated tags.
    pub async fn activate_for_memory_tags(&self, tags: &str) -> Vec<String> {
        let mut activated = Vec::new();
        for tag in memory_tag_activation_keys(tags) {
            let event = ActivationEvent::OnMemoryTag { tag };
            for plugin_id in self.check_activation(&event).await {
                if self.activate(&plugin_id).await.is_ok() {
                    activated.push(plugin_id);
                }
            }
        }
        activated.sort();
        activated.dedup();
        activated
    }

    /// Run active memory hooks for a single pipeline stage.
    ///
    /// PreStore hooks may return a JSON [`MemoryHookPatch`] to rewrite
    /// content/tags/importance/type before storage. Later stages are
    /// notification-only; their output is ignored.
    pub async fn run_memory_hooks(
        &self,
        stage: MemoryHookStage,
        mut payload: MemoryHookPayload,
    ) -> MemoryHookRunResult {
        let entries = {
            let inner = self.inner.read().await;
            inner
                .memory_hooks
                .values()
                .filter(|entry| entry.hook.stage == stage)
                .cloned()
                .collect::<Vec<_>>()
        };

        let mut errors = Vec::new();
        let mut executed = 0;
        for entry in entries {
            payload.stage = stage.clone();
            match self.run_one_memory_hook(&entry, &payload).await {
                Ok(Some(patch)) => {
                    executed += 1;
                    if stage == MemoryHookStage::PreStore {
                        apply_memory_hook_patch(&mut payload, patch);
                    }
                }
                Ok(None) => executed += 1,
                Err(err) => errors.push(format!("{}: {err}", entry.hook.id)),
            }
        }

        MemoryHookRunResult {
            payload,
            executed,
            errors,
        }
    }

    /// Get host status summary.
    pub async fn status(&self) -> PluginHostStatus {
        let inner = self.inner.read().await;
        let mut active = 0;
        let mut disabled = 0;
        let mut errors = 0;
        for p in inner.plugins.values() {
            match &p.state {
                PluginState::Active => active += 1,
                PluginState::Disabled => disabled += 1,
                PluginState::Error { .. } => errors += 1,
                PluginState::Installed => {}
            }
        }
        PluginHostStatus {
            total_plugins: inner.plugins.len(),
            active_plugins: active,
            disabled_plugins: disabled,
            error_plugins: errors,
            total_commands: inner.commands.len(),
            total_slash_commands: inner.slash_commands.len(),
            total_themes: inner.themes.len(),
        }
    }

    /// Invoke a contributed command by its `command_id`.
    ///
    /// This convenience path uses an empty in-memory capability store, so
    /// commands that require sensitive capabilities are rejected unless the
    /// caller uses [`Self::invoke_command_with_store`].
    pub async fn invoke_command(
        &self,
        command_id: &str,
        args: Option<serde_json::Value>,
    ) -> Result<CommandResult, String> {
        let store = TokioMutex::new(CapabilityStore::in_memory());
        self.invoke_command_with_store(command_id, args, &store)
            .await
    }

    /// Invoke a contributed command with the user's persisted capability store.
    pub async fn invoke_command_with_store(
        &self,
        command_id: &str,
        args: Option<serde_json::Value>,
        capability_store: &TokioMutex<CapabilityStore>,
    ) -> Result<CommandResult, String> {
        let (entry, plugin, plugins_dir) = self.resolve_command(command_id).await?;
        if entry.plugin_id == TRANSLATOR_PLUGIN_ID {
            return Ok(invoke_builtin_translator(command_id, args));
        }

        let cap_snapshot = capability_snapshot(&entry.plugin_id, capability_store).await;
        if entry.plugin_id == OPENCLAW_PLUGIN_ID {
            return invoke_builtin_openclaw(command_id, args, &cap_snapshot);
        }

        match &plugin.manifest.install_method {
            InstallMethod::BuiltIn => Ok(invoke_builtin_fallback(&entry, args)),
            InstallMethod::Wasm { .. } => {
                ensure_command_capabilities(&plugin, &cap_snapshot, false)?;
                run_wasm_command(&plugin, &plugins_dir, command_id, args, cap_snapshot).await
            }
            InstallMethod::Binary { url } => {
                ensure_command_capabilities(&plugin, &cap_snapshot, true)?;
                run_binary_command(&plugin, &plugins_dir, url, command_id, args).await
            }
            InstallMethod::Sidecar { path } => {
                ensure_command_capabilities(&plugin, &cap_snapshot, true)?;
                run_sidecar_command(&plugin, &plugins_dir, path, command_id, args).await
            }
        }
    }

    /// Invoke a slash-command by its bare name (without `/`).
    ///
    /// Resolves the name → `SlashCommandEntry` → `command_id` → calls
    /// [`Self::invoke_command`]. Returns `Err` when the name is not
    /// registered by any active plugin.
    pub async fn invoke_slash_command(
        &self,
        name: &str,
        args: Option<serde_json::Value>,
    ) -> Result<CommandResult, String> {
        let store = TokioMutex::new(CapabilityStore::in_memory());
        self.invoke_slash_command_with_store(name, args, &store)
            .await
    }

    /// Invoke a slash-command with the user's persisted capability store.
    pub async fn invoke_slash_command_with_store(
        &self,
        name: &str,
        args: Option<serde_json::Value>,
        capability_store: &TokioMutex<CapabilityStore>,
    ) -> Result<CommandResult, String> {
        // Hold the read lock only long enough to resolve the name.
        let command_id = {
            let inner = self.inner.read().await;
            inner
                .slash_commands
                .get(name)
                .map(|e| e.slash_command.command_id.clone())
                .ok_or_else(|| format!("unknown slash-command: /{name}"))?
        };
        self.invoke_command_with_store(&command_id, args, capability_store)
            .await
    }

    async fn resolve_command(
        &self,
        command_id: &str,
    ) -> Result<(CommandEntry, InstalledPlugin, PathBuf), String> {
        if let Some(resolved) = self.resolve_active_command(command_id).await {
            return Ok(resolved);
        }

        let event = ActivationEvent::OnCommand {
            command: command_id.to_string(),
        };
        for plugin_id in self.check_activation(&event).await {
            let _ = self.activate(&plugin_id).await;
        }

        self.resolve_active_command(command_id)
            .await
            .ok_or_else(|| format!("unknown command: {command_id}"))
    }

    async fn resolve_active_command(
        &self,
        command_id: &str,
    ) -> Option<(CommandEntry, InstalledPlugin, PathBuf)> {
        let inner = self.inner.read().await;
        let entry = inner.commands.get(command_id)?.clone();
        let plugin = inner.plugins.get(&entry.plugin_id)?.clone();
        Some((entry, plugin, inner.plugins_dir.clone()))
    }

    async fn run_one_memory_hook(
        &self,
        entry: &MemoryHookEntry,
        payload: &MemoryHookPayload,
    ) -> Result<Option<MemoryHookPatch>, String> {
        let plugin = self
            .get_plugin(&entry.plugin_id)
            .await
            .ok_or_else(|| format!("plugin '{}' not found", entry.plugin_id))?;
        let wasm_bytes = load_hook_wasm_bytes(&plugin, &self.plugins_dir().await)?;
        verify_hook_wasm_hash(&plugin, &wasm_bytes)?;
        let payload_json = serde_json::to_vec(payload).map_err(|e| e.to_string())?;
        let plugin_id = entry.plugin_id.clone();
        let handle = tokio::task::spawn_blocking(move || {
            let runner = WasmRunner::new()?;
            let cap_store = Arc::new(Mutex::new(CapabilityStore::in_memory()));
            let output =
                runner.run_memory_hook_json(&wasm_bytes, &plugin_id, cap_store, &payload_json)?;
            output
                .map(|bytes| {
                    serde_json::from_slice::<MemoryHookPatch>(&bytes).map_err(|e| e.to_string())
                })
                .transpose()
        });
        match tokio::time::timeout(Duration::from_millis(200), handle).await {
            Ok(joined) => joined.map_err(|e| e.to_string())?,
            Err(_) => Err("memory hook timed out after 200 ms".to_string()),
        }
    }

    async fn plugins_dir(&self) -> PathBuf {
        let inner = self.inner.read().await;
        inner.plugins_dir.clone()
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────

fn register_contributions(inner: &mut HostInner, manifest: &PluginManifest) {
    let id = &manifest.id;
    for cmd in &manifest.contributes.commands {
        inner.commands.insert(
            cmd.id.clone(),
            CommandEntry {
                plugin_id: id.clone(),
                command: cmd.clone(),
            },
        );
    }
    for sc in &manifest.contributes.slash_commands {
        inner.slash_commands.insert(
            sc.name.clone(),
            SlashCommandEntry {
                plugin_id: id.clone(),
                slash_command: sc.clone(),
            },
        );
    }
    for theme in &manifest.contributes.themes {
        inner.themes.insert(theme.id.clone(), theme.clone());
    }
    for hook in &manifest.contributes.memory_hooks {
        let key = format!("{}:{}", id, hook.id);
        inner.memory_hooks.insert(
            key,
            MemoryHookEntry {
                plugin_id: id.clone(),
                hook: hook.clone(),
            },
        );
    }
    for setting in &manifest.contributes.settings {
        let key = format!("{}.{}", id, setting.key);
        inner
            .settings
            .entry(key)
            .or_insert_with(|| setting.default_value.clone());
    }
}

fn unregister_contributions(inner: &mut HostInner, plugin_id: &str) {
    inner.commands.retain(|_, v| v.plugin_id != plugin_id);
    inner.slash_commands.retain(|_, v| v.plugin_id != plugin_id);
    inner.themes.retain(|_, t| {
        // Theme IDs should be prefixed with plugin ID, but we check the
        // commands map pattern for safety.
        !t.id.starts_with(plugin_id)
    });
    inner.memory_hooks.retain(|_, v| v.plugin_id != plugin_id);
    let prefix = format!("{plugin_id}.");
    inner.settings.retain(|k, _| !k.starts_with(&prefix));
}

async fn capability_snapshot(
    plugin_id: &str,
    capability_store: &TokioMutex<CapabilityStore>,
) -> Arc<Mutex<CapabilityStore>> {
    let records = capability_store.lock().await.list_for_agent(plugin_id);
    let mut snapshot = CapabilityStore::in_memory();
    for record in records {
        if record.granted {
            snapshot.grant(&record.agent_name, record.capability);
        }
    }
    Arc::new(Mutex::new(snapshot))
}

fn ensure_command_capabilities(
    plugin: &InstalledPlugin,
    cap_store: &Arc<Mutex<CapabilityStore>>,
    requires_process_spawn: bool,
) -> Result<(), String> {
    let required =
        required_sandbox_capabilities(&plugin.manifest.capabilities, requires_process_spawn);
    let store = cap_store.lock().map_err(|e| e.to_string())?;
    for capability in required {
        if !store.has_capability(&plugin.manifest.id, &capability) {
            return Err(format!(
                "plugin '{}' requires capability {:?}",
                plugin.manifest.id, capability
            ));
        }
    }
    Ok(())
}

fn required_sandbox_capabilities(
    manifest_capabilities: &[ManifestCapability],
    requires_process_spawn: bool,
) -> Vec<SandboxCapability> {
    let mut required = Vec::new();
    if requires_process_spawn {
        push_unique(&mut required, SandboxCapability::ProcessSpawn);
    }
    for capability in manifest_capabilities {
        match capability {
            ManifestCapability::Filesystem => {
                push_unique(&mut required, SandboxCapability::FileRead);
                push_unique(&mut required, SandboxCapability::FileWrite);
            }
            ManifestCapability::Clipboard => {
                push_unique(&mut required, SandboxCapability::Clipboard)
            }
            ManifestCapability::Network => push_unique(&mut required, SandboxCapability::Network),
            ManifestCapability::RemoteExec => {
                push_unique(&mut required, SandboxCapability::ProcessSpawn)
            }
            ManifestCapability::Chat
            | ManifestCapability::Character
            | ManifestCapability::ConversationHistory => {}
        }
    }
    required
}

fn push_unique(required: &mut Vec<SandboxCapability>, capability: SandboxCapability) {
    if !required.contains(&capability) {
        required.push(capability);
    }
}

async fn run_wasm_command(
    plugin: &InstalledPlugin,
    plugins_dir: &Path,
    command_id: &str,
    args: Option<serde_json::Value>,
    cap_store: Arc<Mutex<CapabilityStore>>,
) -> Result<CommandResult, String> {
    let wasm_bytes = load_plugin_wasm_bytes(plugin, plugins_dir)?;
    verify_hook_wasm_hash(plugin, &wasm_bytes)?;
    let payload = plugin_command_payload(command_id, args);
    let payload_json = serde_json::to_vec(&payload).map_err(|e| e.to_string())?;
    let plugin_id = plugin.manifest.id.clone();
    let handle = tokio::task::spawn_blocking(move || {
        let runner = WasmRunner::new()?;
        runner.run_command_json(&wasm_bytes, &plugin_id, cap_store, &payload_json)
    });
    let output = match tokio::time::timeout(PLUGIN_COMMAND_TIMEOUT, handle).await {
        Ok(joined) => joined.map_err(|e| e.to_string())??,
        Err(_) => return Err("plugin WASM command timed out after 10 seconds".to_string()),
    };
    Ok(CommandResult {
        success: true,
        output: output.and_then(bytes_to_non_empty_string),
        error: None,
        exit_code: Some(0),
        stderr: None,
    })
}

async fn run_binary_command(
    plugin: &InstalledPlugin,
    plugins_dir: &Path,
    url: &str,
    command_id: &str,
    args: Option<serde_json::Value>,
) -> Result<CommandResult, String> {
    let path = resolve_plugin_local_path(url, plugins_dir, &plugin.manifest.id)?;
    let mut command = Command::new(&path);
    command
        .arg(command_id)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    if let Some(args) = args {
        command.arg(serde_json::to_string(&args).map_err(|e| e.to_string())?);
    }
    if let Some(cwd) = plugin_command_cwd(&path, plugins_dir, &plugin.manifest.id) {
        command.current_dir(cwd);
    }
    let output = tokio::time::timeout(PLUGIN_COMMAND_TIMEOUT, command.output())
        .await
        .map_err(|_| "plugin binary command timed out after 10 seconds".to_string())?
        .map_err(|e| format!("spawn plugin binary {}: {e}", path.display()))?;
    Ok(command_result_from_output(
        output.status,
        output.stdout,
        output.stderr,
    ))
}

async fn run_sidecar_command(
    plugin: &InstalledPlugin,
    plugins_dir: &Path,
    path_spec: &str,
    command_id: &str,
    args: Option<serde_json::Value>,
) -> Result<CommandResult, String> {
    let path = resolve_plugin_local_path(path_spec, plugins_dir, &plugin.manifest.id)?;
    let mut command = Command::new(&path);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    if let Some(cwd) = plugin_command_cwd(&path, plugins_dir, &plugin.manifest.id) {
        command.current_dir(cwd);
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("spawn plugin sidecar {}: {e}", path.display()))?;
    let payload = serde_json::to_string(&plugin_command_payload(command_id, args))
        .map_err(|e| e.to_string())?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "plugin sidecar stdin was not available".to_string())?;
    stdin
        .write_all(payload.as_bytes())
        .await
        .map_err(|e| format!("write plugin sidecar stdin: {e}"))?;
    stdin
        .write_all(b"\n")
        .await
        .map_err(|e| format!("write plugin sidecar newline: {e}"))?;
    drop(stdin);

    let output = tokio::time::timeout(PLUGIN_COMMAND_TIMEOUT, child.wait_with_output())
        .await
        .map_err(|_| "plugin sidecar command timed out after 10 seconds".to_string())?
        .map_err(|e| format!("wait plugin sidecar {}: {e}", path.display()))?;
    Ok(command_result_from_output(
        output.status,
        output.stdout,
        output.stderr,
    ))
}

fn plugin_command_payload(command_id: &str, args: Option<serde_json::Value>) -> serde_json::Value {
    serde_json::json!({
        "command_id": command_id,
        "args": args.unwrap_or(serde_json::Value::Null),
    })
}

fn command_result_from_output(
    status: std::process::ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
) -> CommandResult {
    let output = bytes_to_non_empty_string(stdout);
    let stderr = bytes_to_non_empty_string(stderr);
    if status.success() {
        CommandResult {
            success: true,
            output,
            error: None,
            exit_code: status.code(),
            stderr,
        }
    } else {
        let code = status.code();
        let fallback = code
            .map(|c| format!("plugin command exited with status {c}"))
            .unwrap_or_else(|| "plugin command terminated by signal".to_string());
        CommandResult {
            success: false,
            output,
            error: stderr.clone().or(Some(fallback)),
            exit_code: code,
            stderr,
        }
    }
}

fn bytes_to_non_empty_string(bytes: Vec<u8>) -> Option<String> {
    non_empty_string(String::from_utf8_lossy(&bytes).trim().to_string())
}

fn non_empty_string(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn invoke_builtin_fallback(entry: &CommandEntry, args: Option<serde_json::Value>) -> CommandResult {
    let mut output = format!("[{}] {}", entry.plugin_id, entry.command.title);
    if let Some(args) = args {
        output.push_str(&format!(" — args: {args}"));
    }
    CommandResult::success(output)
}

fn apply_memory_hook_patch(payload: &mut MemoryHookPayload, patch: MemoryHookPatch) {
    if let Some(content) = patch.content {
        payload.content = content;
    }
    if let Some(tags) = patch.tags {
        payload.tags = tags;
    }
    if let Some(importance) = patch.importance {
        payload.importance = importance.clamp(1, 5);
    }
    if let Some(memory_type) = patch.memory_type {
        payload.memory_type = memory_type;
    }
}

fn memory_tag_activation_keys(tags: &str) -> Vec<String> {
    let mut keys = Vec::new();
    for raw in tags.split(',') {
        let tag = raw.trim();
        if tag.is_empty() {
            continue;
        }
        keys.push(tag.to_string());
        if let Some((prefix, _)) = tag.split_once(':') {
            let prefix = prefix.trim();
            if !prefix.is_empty() {
                keys.push(prefix.to_string());
            }
        }
    }
    keys.sort();
    keys.dedup();
    keys
}

fn load_hook_wasm_bytes(plugin: &InstalledPlugin, plugins_dir: &Path) -> Result<Vec<u8>, String> {
    load_plugin_wasm_bytes(plugin, plugins_dir)
}

fn load_plugin_wasm_bytes(plugin: &InstalledPlugin, plugins_dir: &Path) -> Result<Vec<u8>, String> {
    let InstallMethod::Wasm { url } = &plugin.manifest.install_method else {
        return Err("plugin requires a wasm install method".to_string());
    };
    let path = resolve_plugin_local_path(url, plugins_dir, &plugin.manifest.id)?;
    std::fs::read(&path).map_err(|e| format!("read plugin wasm {}: {e}", path.display()))
}

fn resolve_plugin_local_path(
    url: &str,
    plugins_dir: &Path,
    plugin_id: &str,
) -> Result<PathBuf, String> {
    let path = PathBuf::from(url);
    if path.is_absolute() {
        return Ok(path);
    }

    if let Ok(parsed) = url::Url::parse(url) {
        if parsed.scheme() == "file" {
            return parsed
                .to_file_path()
                .map_err(|_| format!("invalid file URL for wasm hook: {url}"));
        }
        return Err(format!(
            "wasm hook URL must point to an installed local file, got: {url}"
        ));
    }

    let plugin_scoped = plugins_dir.join(plugin_id).join(&path);
    if plugin_scoped.exists() {
        return Ok(plugin_scoped);
    }
    let shared = plugins_dir.join(&path);
    if shared.exists() {
        return Ok(shared);
    }
    Ok(path)
}

fn plugin_command_cwd(path: &Path, plugins_dir: &Path, plugin_id: &str) -> Option<PathBuf> {
    let plugin_dir = plugins_dir.join(plugin_id);
    if plugin_dir.is_dir() {
        return Some(plugin_dir);
    }
    path.parent().map(Path::to_path_buf)
}

fn verify_hook_wasm_hash(plugin: &InstalledPlugin, bytes: &[u8]) -> Result<(), String> {
    let Some(expected) = plugin.manifest.sha256.as_deref() else {
        return Ok(());
    };
    use sha2::{Digest, Sha256};
    let actual = hex::encode(Sha256::digest(bytes));
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(format!(
            "wasm hook sha256 mismatch: expected {expected}, got {actual}"
        ))
    }
}

pub const TRANSLATOR_PLUGIN_ID: &str = "terransoul-translator";
pub const OPENCLAW_PLUGIN_ID: &str = "openclaw-bridge";

fn builtin_manifests() -> Vec<PluginManifest> {
    vec![translator_manifest(), openclaw_manifest()]
}

fn translator_manifest() -> PluginManifest {
    use crate::package_manager::manifest::InstallMethod;

    PluginManifest {
        id: TRANSLATOR_PLUGIN_ID.into(),
        display_name: "Translator Mode".into(),
        version: "1.0.0".into(),
        description:
            "Reference built-in plugin that turns TerranSoul into a worldwide two-person translator."
                .into(),
        kind: PluginKind::Tool,
        install_method: InstallMethod::BuiltIn,
        capabilities: vec![],
        activation_events: vec![ActivationEvent::OnChatMessage {
            pattern: "translator".into(),
        }],
        contributes: Contributions {
            commands: vec![
                ContributedCommand {
                    id: "terransoul-translator.start".into(),
                    title: "Start Translator Mode".into(),
                    icon: Some("🌍".into()),
                    keybinding: None,
                    category: Some("Translation".into()),
                },
                ContributedCommand {
                    id: "terransoul-translator.stop".into(),
                    title: "Stop Translator Mode".into(),
                    icon: Some("🛑".into()),
                    keybinding: None,
                    category: Some("Translation".into()),
                },
                ContributedCommand {
                    id: "terransoul-translator.status".into(),
                    title: "Translator Mode Status".into(),
                    icon: Some("ℹ️".into()),
                    keybinding: None,
                    category: Some("Translation".into()),
                },
            ],
            slash_commands: vec![ContributedSlashCommand {
                name: "translator".into(),
                description:
                    "Start translator mode with any BCP-47 language pair, e.g. /translator en-US vi."
                        .into(),
                command_id: "terransoul-translator.start".into(),
            }],
            ..Contributions::default()
        },
        system_requirements: None,
        api_version: 1,
        homepage: Some("docs/plugin-development.md#translator-mode-reference-plugin".into()),
        license: Some("MIT".into()),
        author: Some("TerranSoul".into()),
        icon: Some("🌍".into()),
        publisher: Some("terransoul".into()),
        signature: None,
        sha256: None,
        dependencies: vec![],
    }
}

fn invoke_builtin_translator(command_id: &str, args: Option<serde_json::Value>) -> CommandResult {
    match command_id {
        "terransoul-translator.start" => {
            let source = args
                .as_ref()
                .and_then(|v| v.get("source"))
                .and_then(|v| v.as_str())
                .unwrap_or("und");
            let target = args
                .as_ref()
                .and_then(|v| v.get("target"))
                .and_then(|v| v.as_str())
                .unwrap_or("und");
            let source = match normalize_language_input(source) {
                Ok(code) => code,
                Err(_) => return CommandResult::failure(format!("unsupported source language: {source}")),
            };
            let target = match normalize_language_input(target) {
                Ok(code) => code,
                Err(_) => return CommandResult::failure(format!("unsupported target language: {target}")),
            };
            CommandResult::success(format!(
                "Translator mode ready between {source} and {target}. If either voice is not installed, install that speech language in your OS/browser language settings."
            ))
        }
        "terransoul-translator.stop" => CommandResult::success("Translator mode stopped."),
        "terransoul-translator.status" => {
            CommandResult::success("Translator mode supports any valid BCP-47 language pair. If a selected speech voice is not installed, install the language voice in OS/browser settings.")
        }
        _ => CommandResult::failure(format!("unsupported translator command: {command_id}")),
    }
}

fn openclaw_manifest() -> PluginManifest {
    PluginManifest {
        id: OPENCLAW_PLUGIN_ID.into(),
        display_name: "OpenClaw Bridge".into(),
        version: "1.0.0".into(),
        description:
            "Built-in plugin for invoking OpenClaw-style read, fetch, and chat tools from TerranSoul."
                .into(),
        kind: PluginKind::Tool,
        install_method: InstallMethod::BuiltIn,
        capabilities: vec![
            ManifestCapability::Chat,
            ManifestCapability::Filesystem,
            ManifestCapability::Network,
        ],
        activation_events: vec![
            ActivationEvent::OnChatMessage {
                pattern: "/openclaw".into(),
            },
            ActivationEvent::OnCommand {
                command: "openclaw-bridge.dispatch".into(),
            },
        ],
        contributes: Contributions {
            commands: vec![
                ContributedCommand {
                    id: "openclaw-bridge.dispatch".into(),
                    title: "Run OpenClaw Directive".into(),
                    icon: Some("🧰".into()),
                    keybinding: None,
                    category: Some("OpenClaw".into()),
                },
                ContributedCommand {
                    id: "openclaw-bridge.read".into(),
                    title: "OpenClaw Read".into(),
                    icon: Some("📄".into()),
                    keybinding: None,
                    category: Some("OpenClaw".into()),
                },
                ContributedCommand {
                    id: "openclaw-bridge.fetch".into(),
                    title: "OpenClaw Fetch".into(),
                    icon: Some("🌐".into()),
                    keybinding: None,
                    category: Some("OpenClaw".into()),
                },
                ContributedCommand {
                    id: "openclaw-bridge.chat".into(),
                    title: "OpenClaw Chat".into(),
                    icon: Some("💬".into()),
                    keybinding: None,
                    category: Some("OpenClaw".into()),
                },
                ContributedCommand {
                    id: "openclaw-bridge.status".into(),
                    title: "OpenClaw Bridge Status".into(),
                    icon: Some("ℹ️".into()),
                    keybinding: None,
                    category: Some("OpenClaw".into()),
                },
            ],
            slash_commands: vec![ContributedSlashCommand {
                name: "openclaw".into(),
                description: "Run OpenClaw tools, e.g. /openclaw read README.md".into(),
                command_id: "openclaw-bridge.dispatch".into(),
            }],
            ..Contributions::default()
        },
        system_requirements: None,
        api_version: 1,
        homepage: Some("tutorials/openclaw-plugin-tutorial.md".into()),
        license: Some("MIT".into()),
        author: Some("TerranSoul / OpenClaw Community".into()),
        icon: Some("🧰".into()),
        publisher: Some("terransoul".into()),
        signature: None,
        sha256: None,
        dependencies: vec![],
    }
}

fn invoke_builtin_openclaw(
    command_id: &str,
    args: Option<serde_json::Value>,
    cap_store: &Arc<Mutex<CapabilityStore>>,
) -> Result<CommandResult, String> {
    match command_id {
        "openclaw-bridge.dispatch" => invoke_openclaw_dispatch(args, cap_store),
        "openclaw-bridge.read" => invoke_openclaw_tool(
            OpenClawTool::Read,
            &openclaw_argument(args, &["path", "file", "text", "argument"]),
            cap_store,
        ),
        "openclaw-bridge.fetch" => invoke_openclaw_tool(
            OpenClawTool::Fetch,
            &openclaw_argument(args, &["url", "text", "argument"]),
            cap_store,
        ),
        "openclaw-bridge.chat" => invoke_openclaw_tool(
            OpenClawTool::Chat,
            &openclaw_argument(args, &["prompt", "text", "argument"]),
            cap_store,
        ),
        "openclaw-bridge.status" => Ok(CommandResult::success(openclaw_help_text())),
        _ => Ok(CommandResult::failure(format!(
            "unsupported OpenClaw command: {command_id}"
        ))),
    }
}

fn invoke_openclaw_dispatch(
    args: Option<serde_json::Value>,
    cap_store: &Arc<Mutex<CapabilityStore>>,
) -> Result<CommandResult, String> {
    let text = openclaw_argument(args, &["text", "directive", "argument"]);
    if text.trim().is_empty() {
        return Ok(CommandResult::success(openclaw_help_text()));
    }
    let directive = if text.trim_start().starts_with("/openclaw") {
        text
    } else {
        format!("/openclaw {text}")
    };
    match parse_openclaw(&directive) {
        ParsedMessage::Directive(tool, argument) => invoke_openclaw_tool(tool, argument, cap_store),
        ParsedMessage::UnknownDirective(name) => Ok(CommandResult::failure(format!(
            "openclaw: unknown tool `{name}` — supported tools: read, fetch, chat"
        ))),
        ParsedMessage::Chat(_) => Ok(CommandResult::success(openclaw_help_text())),
    }
}

fn invoke_openclaw_tool(
    tool: OpenClawTool,
    argument: &str,
    cap_store: &Arc<Mutex<CapabilityStore>>,
) -> Result<CommandResult, String> {
    ensure_openclaw_capability(tool, cap_store)?;
    let argument = argument.trim();
    if argument.is_empty() {
        return Ok(CommandResult::failure(format!(
            "openclaw: tool `{}` requires an argument",
            tool.as_str()
        )));
    }
    Ok(CommandResult::success(match tool {
        OpenClawTool::Read => format!(
            "[openclaw/read] would read `{argument}` (capability granted; real plugin sends JSON-RPC `fs.read` to the OpenClaw runtime)"
        ),
        OpenClawTool::Fetch => format!(
            "[openclaw/fetch] would fetch `{argument}` (capability granted; real plugin sends JSON-RPC `net.fetch` to the OpenClaw runtime)"
        ),
        OpenClawTool::Chat => format!(
            "[openclaw/chat] forwarded prompt to the OpenClaw chat tool: {argument}"
        ),
    }))
}

fn ensure_openclaw_capability(
    tool: OpenClawTool,
    cap_store: &Arc<Mutex<CapabilityStore>>,
) -> Result<(), String> {
    let Some(required) = openclaw_sandbox_capability(tool) else {
        return Ok(());
    };
    let store = cap_store.lock().map_err(|e| e.to_string())?;
    if store.has_capability(OPENCLAW_PLUGIN_ID, &required) {
        Ok(())
    } else {
        Err(format!(
            "plugin '{}' requires capability {:?}",
            OPENCLAW_PLUGIN_ID, required
        ))
    }
}

fn openclaw_sandbox_capability(tool: OpenClawTool) -> Option<SandboxCapability> {
    match tool {
        OpenClawTool::Read => Some(SandboxCapability::FileRead),
        OpenClawTool::Fetch => Some(SandboxCapability::Network),
        OpenClawTool::Chat => None,
    }
}

fn openclaw_argument(args: Option<serde_json::Value>, keys: &[&str]) -> String {
    let Some(args) = args else {
        return String::new();
    };
    if let Some(text) = args.as_str() {
        return text.to_string();
    }
    keys.iter()
        .find_map(|key| args.get(key).and_then(|value| value.as_str()))
        .unwrap_or("")
        .to_string()
}

fn openclaw_help_text() -> &'static str {
    "OpenClaw bridge ready. Use `/openclaw read <relative-path>`, `/openclaw fetch <url>`, or `/openclaw chat <prompt>`."
}

fn matches_event(declared: &ActivationEvent, fired: &ActivationEvent) -> bool {
    match (declared, fired) {
        (ActivationEvent::OnStartup, ActivationEvent::OnStartup) => true,
        (ActivationEvent::OnCommand { command: a }, ActivationEvent::OnCommand { command: b }) => {
            a == b
        }
        (ActivationEvent::OnView { view_id: a }, ActivationEvent::OnView { view_id: b }) => a == b,
        (
            ActivationEvent::OnChatMessage { pattern: a },
            ActivationEvent::OnChatMessage { pattern: b },
        ) => b.contains(a.as_str()),
        (ActivationEvent::OnMemoryTag { tag: a }, ActivationEvent::OnMemoryTag { tag: b }) => {
            a == b
        }
        (ActivationEvent::OnMarketplace, ActivationEvent::OnMarketplace) => true,
        (ActivationEvent::OnBrainModeChange, ActivationEvent::OnBrainModeChange) => true,
        (
            ActivationEvent::OnCapabilityGranted { capability: a },
            ActivationEvent::OnCapabilityGranted { capability: b },
        ) => a == b,
        _ => false,
    }
}

fn persist_plugin(plugins_dir: &Path, plugin: &InstalledPlugin) -> Result<(), String> {
    if plugins_dir == Path::new(":memory:") {
        return Ok(());
    }
    std::fs::create_dir_all(plugins_dir).map_err(|e| e.to_string())?;
    let path = plugins_dir.join(format!("{}.json", plugin.manifest.id));
    let json = serde_json::to_string_pretty(plugin).map_err(|e| e.to_string())?;
    // Atomic write via tmp + rename
    let tmp = plugins_dir.join(format!("{}.json.tmp", plugin.manifest.id));
    std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    Ok(())
}

fn remove_plugin_file(plugins_dir: &Path, plugin_id: &str) -> Result<(), String> {
    if plugins_dir == Path::new(":memory:") {
        return Ok(());
    }
    let path = plugins_dir.join(format!("{plugin_id}.json"));
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::super::manifest::*;
    use super::*;
    use crate::package_manager::manifest::InstallMethod;
    use crate::sandbox::Capability;
    use std::io::Write;

    fn test_manifest(id: &str) -> PluginManifest {
        PluginManifest {
            id: id.into(),
            display_name: format!("Test {id}"),
            version: "1.0.0".into(),
            description: "Test plugin".into(),
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

    fn add_command(manifest: &mut PluginManifest, command_id: &str, title: &str) {
        manifest.contributes.commands.push(ContributedCommand {
            id: command_id.into(),
            title: title.into(),
            icon: None,
            keybinding: None,
            category: None,
        });
    }

    async fn store_with_capability(
        plugin_id: &str,
        capability: Capability,
    ) -> TokioMutex<CapabilityStore> {
        let store = TokioMutex::new(CapabilityStore::in_memory());
        store.lock().await.grant(plugin_id, capability);
        store
    }

    fn write_executable(path: &Path, body: &str) {
        let mut file = std::fs::File::create(path).unwrap();
        file.write_all(body.as_bytes()).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = file.metadata().unwrap().permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(path, permissions).unwrap();
        }
    }

    fn native_echo_script(dir: &Path) -> PathBuf {
        #[cfg(windows)]
        {
            let path = dir.join("native-echo.cmd");
            write_executable(&path, "@echo off\r\necho native-ok %1 %2\r\n");
            path
        }
        #[cfg(not(windows))]
        {
            let path = dir.join("native-echo.sh");
            write_executable(&path, "#!/bin/sh\necho native-ok \"$1\" \"$2\"\n");
            path
        }
    }

    fn sidecar_echo_script(dir: &Path) -> PathBuf {
        #[cfg(windows)]
        {
            let path = dir.join("sidecar-echo.cmd");
            write_executable(
                &path,
                "@echo off\r\nset /p line=\r\necho sidecar:%line%\r\n",
            );
            path
        }
        #[cfg(not(windows))]
        {
            let path = dir.join("sidecar-echo.sh");
            write_executable(
                &path,
                "#!/bin/sh\nIFS= read -r line\nprintf 'sidecar:%s\\n' \"$line\"\n",
            );
            path
        }
    }

    #[cfg(feature = "wasm-sandbox")]
    fn command_wasm(output_text: &str) -> Vec<u8> {
        use wasm_encoder::{
            CodeSection, ConstExpr, DataSection, ExportKind, ExportSection, Function,
            FunctionSection, Instruction, MemorySection, MemoryType, Module, TypeSection, ValType,
        };

        const OUTPUT_OFFSET: u64 = 1024;
        let packed = ((OUTPUT_OFFSET << 32) | output_text.len() as u64) as i64;
        let mut module = Module::new();
        let mut types = TypeSection::new();
        types
            .ty()
            .function([ValType::I32, ValType::I32], [ValType::I64]);
        module.section(&types);

        let mut functions = FunctionSection::new();
        functions.function(0);
        module.section(&functions);

        let mut memories = MemorySection::new();
        memories.memory(MemoryType {
            minimum: 1,
            maximum: None,
            memory64: false,
            shared: false,
            page_size_log2: None,
        });
        module.section(&memories);

        let mut exports = ExportSection::new();
        exports.export("memory", ExportKind::Memory, 0);
        exports.export("handle_command", ExportKind::Func, 0);
        module.section(&exports);

        let mut code = CodeSection::new();
        let mut function = Function::new([]);
        function.instruction(&Instruction::I64Const(packed));
        function.instruction(&Instruction::End);
        code.function(&function);
        module.section(&code);

        let mut data = DataSection::new();
        data.active(
            0,
            &ConstExpr::i32_const(OUTPUT_OFFSET as i32),
            output_text.as_bytes().iter().copied(),
        );
        module.section(&data);
        module.finish()
    }

    #[tokio::test]
    async fn install_and_list() {
        let host = PluginHost::in_memory();
        let m = test_manifest("my-plugin");
        host.install(m).await.unwrap();
        let list = host.list_plugins().await;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].manifest.id, "my-plugin");
        assert_eq!(list[0].state, PluginState::Installed);
    }

    #[tokio::test]
    async fn duplicate_install_rejected() {
        let host = PluginHost::in_memory();
        let m = test_manifest("dup");
        host.install(m.clone()).await.unwrap();
        let err = host.install(m).await.unwrap_err();
        assert!(err.contains("already installed"));
    }

    #[tokio::test]
    async fn activate_registers_commands() {
        let host = PluginHost::in_memory();
        let mut m = test_manifest("cmd-plugin");
        m.contributes.commands.push(ContributedCommand {
            id: "cmd-plugin.greet".into(),
            title: "Greet".into(),
            icon: None,
            keybinding: None,
            category: None,
        });
        host.install(m).await.unwrap();
        assert!(host.list_commands().await.is_empty());
        host.activate("cmd-plugin").await.unwrap();
        let cmds = host.list_commands().await;
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].command.id, "cmd-plugin.greet");
    }

    #[tokio::test]
    async fn deactivate_removes_commands() {
        let host = PluginHost::in_memory();
        let mut m = test_manifest("deact");
        m.contributes.commands.push(ContributedCommand {
            id: "deact.run".into(),
            title: "Run".into(),
            icon: None,
            keybinding: None,
            category: None,
        });
        host.install(m).await.unwrap();
        host.activate("deact").await.unwrap();
        assert_eq!(host.list_commands().await.len(), 1);
        host.deactivate("deact").await.unwrap();
        assert!(host.list_commands().await.is_empty());
        let p = host.get_plugin("deact").await.unwrap();
        assert_eq!(p.state, PluginState::Disabled);
    }

    #[tokio::test]
    async fn uninstall_removes_everything() {
        let host = PluginHost::in_memory();
        host.install(test_manifest("rm-me")).await.unwrap();
        host.activate("rm-me").await.unwrap();
        host.uninstall("rm-me").await.unwrap();
        assert!(host.list_plugins().await.is_empty());
    }

    #[tokio::test]
    async fn slash_commands_registered() {
        let host = PluginHost::in_memory();
        let mut m = test_manifest("slash");
        m.contributes.slash_commands.push(ContributedSlashCommand {
            name: "translate".into(),
            description: "Translate text".into(),
            command_id: "slash.translate".into(),
        });
        host.install(m).await.unwrap();
        host.activate("slash").await.unwrap();
        let scs = host.list_slash_commands().await;
        assert_eq!(scs.len(), 1);
        assert_eq!(scs[0].slash_command.name, "translate");
    }

    #[tokio::test]
    async fn themes_registered() {
        let host = PluginHost::in_memory();
        let mut m = test_manifest("theme-plugin");
        let mut tokens = std::collections::HashMap::new();
        tokens.insert("--ts-accent".into(), "#ff0000".into());
        m.contributes.themes.push(ContributedTheme {
            id: "theme-plugin.red".into(),
            label: "Red Theme".into(),
            tokens,
        });
        host.install(m).await.unwrap();
        host.activate("theme-plugin").await.unwrap();
        let themes = host.list_themes().await;
        assert_eq!(themes.len(), 1);
        assert_eq!(themes[0].tokens["--ts-accent"], "#ff0000");
    }

    #[tokio::test]
    async fn memory_hooks_registered_for_active_plugins() {
        let host = PluginHost::in_memory();
        let mut m = test_manifest("memory-plugin");
        m.contributes.memory_hooks.push(ContributedMemoryHook {
            id: "memory-plugin.tag".into(),
            stage: MemoryHookStage::PreStore,
            description: "Tag memories".into(),
        });
        host.install(m).await.unwrap();
        assert!(host.list_memory_hooks().await.is_empty());
        host.activate("memory-plugin").await.unwrap();
        let hooks = host.list_memory_hooks().await;
        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0].plugin_id, "memory-plugin");
        assert_eq!(hooks[0].hook.stage, MemoryHookStage::PreStore);
        host.deactivate("memory-plugin").await.unwrap();
        assert!(host.list_memory_hooks().await.is_empty());
    }

    #[tokio::test]
    async fn memory_tag_activation_matches_prefix_tags() {
        let host = PluginHost::in_memory();
        let mut m = test_manifest("lazy-memory");
        m.activation_events = vec![ActivationEvent::OnMemoryTag { tag: "code".into() }];
        m.contributes.memory_hooks.push(ContributedMemoryHook {
            id: "lazy-memory.tag".into(),
            stage: MemoryHookStage::PreStore,
            description: "Tag code memories".into(),
        });
        host.install(m).await.unwrap();
        let activated = host
            .activate_for_memory_tags("personal:name, code:rust")
            .await;
        assert_eq!(activated, vec!["lazy-memory"]);
        let plugin = host.get_plugin("lazy-memory").await.unwrap();
        assert_eq!(plugin.state, PluginState::Active);
        assert_eq!(host.list_memory_hooks().await.len(), 1);
    }

    #[tokio::test]
    async fn deactivate_removes_themes_for_plugin() {
        let host = PluginHost::in_memory();
        let mut m = test_manifest("theme-plugin");
        m.contributes.themes.push(ContributedTheme {
            id: "theme-plugin.red".into(),
            label: "Red Theme".into(),
            tokens: std::collections::HashMap::new(),
        });
        host.install(m).await.unwrap();
        host.activate("theme-plugin").await.unwrap();
        assert_eq!(host.list_themes().await.len(), 1);
        host.deactivate("theme-plugin").await.unwrap();
        assert!(host.list_themes().await.is_empty());
    }

    #[tokio::test]
    async fn production_host_registers_translator_plugin() {
        let tmp = tempfile::TempDir::new().unwrap();
        let host = PluginHost::with_builtin_plugins(tmp.path());
        let translator = host.get_plugin(TRANSLATOR_PLUGIN_ID).await.unwrap();
        assert_eq!(translator.state, PluginState::Active);
        let slash = host.list_slash_commands().await;
        assert!(slash.iter().any(|s| s.slash_command.name == "translator"));
        let result = host
            .invoke_command(
                "terransoul-translator.start",
                Some(serde_json::json!({ "source": "English", "target": "Vietnamese" })),
            )
            .await
            .unwrap();
        assert!(result.success);
        let output = result.output.unwrap();
        assert!(output.contains("en and vi"));
        assert!(output.contains("If either voice is not installed"));
    }

    #[tokio::test]
    async fn translator_plugin_accepts_worldwide_bcp47_language_pair() {
        let tmp = tempfile::TempDir::new().unwrap();
        let host = PluginHost::with_builtin_plugins(tmp.path());
        let result = host
            .invoke_command(
                "terransoul-translator.start",
                Some(serde_json::json!({ "source": "pt-BR", "target": "zu" })),
            )
            .await
            .unwrap();
        assert!(result.success);
        let output = result.output.unwrap();
        assert!(output.contains("pt-br and zu"));
        assert!(output.contains("install that speech language"));
    }

    #[tokio::test]
    async fn translator_plugin_rejects_invalid_language_pair() {
        let tmp = tempfile::TempDir::new().unwrap();
        let host = PluginHost::with_builtin_plugins(tmp.path());
        let result = host
            .invoke_command(
                "terransoul-translator.start",
                Some(serde_json::json!({ "source": "en", "target": "bad-" })),
            )
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result
            .error
            .unwrap()
            .contains("unsupported target language"));
    }

    #[tokio::test]
    async fn production_host_registers_openclaw_plugin() {
        let tmp = tempfile::TempDir::new().unwrap();
        let host = PluginHost::with_builtin_plugins(tmp.path());
        let openclaw = host.get_plugin(OPENCLAW_PLUGIN_ID).await.unwrap();
        assert_eq!(openclaw.state, PluginState::Active);
        assert!(openclaw
            .manifest
            .capabilities
            .contains(&ManifestCapability::Filesystem));
        assert!(openclaw
            .manifest
            .capabilities
            .contains(&ManifestCapability::Network));

        let slash = host.list_slash_commands().await;
        assert!(slash.iter().any(|s| s.slash_command.name == "openclaw"));
        let commands = host.list_commands().await;
        assert!(commands
            .iter()
            .any(|entry| entry.command.id == "openclaw-bridge.dispatch"));
    }

    #[tokio::test]
    async fn openclaw_read_requires_file_read_capability() {
        let tmp = tempfile::TempDir::new().unwrap();
        let host = PluginHost::with_builtin_plugins(tmp.path());
        let store = TokioMutex::new(CapabilityStore::in_memory());

        let error = host
            .invoke_command_with_store(
                "openclaw-bridge.read",
                Some(serde_json::json!({ "path": "README.md" })),
                &store,
            )
            .await
            .unwrap_err();

        assert!(error.contains("openclaw-bridge"));
        assert!(error.contains("FileRead"));
    }

    #[tokio::test]
    async fn openclaw_slash_dispatch_runs_with_file_read_grant() {
        let tmp = tempfile::TempDir::new().unwrap();
        let host = PluginHost::with_builtin_plugins(tmp.path());
        let store = store_with_capability(OPENCLAW_PLUGIN_ID, Capability::FileRead).await;

        let result = host
            .invoke_slash_command_with_store(
                "openclaw",
                Some(serde_json::json!({ "text": "read README.md" })),
                &store,
            )
            .await
            .unwrap();

        assert!(result.success);
        let output = result.output.unwrap();
        assert!(output.contains("openclaw/read"));
        assert!(output.contains("README.md"));
    }

    #[tokio::test]
    async fn openclaw_fetch_requires_network_capability() {
        let tmp = tempfile::TempDir::new().unwrap();
        let host = PluginHost::with_builtin_plugins(tmp.path());
        let store = store_with_capability(OPENCLAW_PLUGIN_ID, Capability::FileRead).await;

        let error = host
            .invoke_slash_command_with_store(
                "openclaw",
                Some(serde_json::json!({ "text": "fetch https://example.com" })),
                &store,
            )
            .await
            .unwrap_err();

        assert!(error.contains("Network"));
    }

    #[tokio::test]
    async fn openclaw_chat_command_uses_no_sensitive_capability() {
        let tmp = tempfile::TempDir::new().unwrap();
        let host = PluginHost::with_builtin_plugins(tmp.path());
        let store = TokioMutex::new(CapabilityStore::in_memory());

        let result = host
            .invoke_command_with_store(
                "openclaw-bridge.chat",
                Some(serde_json::json!({ "prompt": "summarise this" })),
                &store,
            )
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.unwrap().contains("summarise this"));
    }

    #[tokio::test]
    async fn settings_initialized_with_defaults() {
        let host = PluginHost::in_memory();
        let mut m = test_manifest("settings-plugin");
        m.contributes.settings.push(ContributedSetting {
            key: "maxRetries".into(),
            label: "Max Retries".into(),
            description: "How many".into(),
            default_value: serde_json::json!(3),
            value_type: SettingValueType::Number,
        });
        host.install(m).await.unwrap();
        host.activate("settings-plugin").await.unwrap();
        let val = host.get_setting("settings-plugin.maxRetries").await;
        assert_eq!(val, Some(serde_json::json!(3)));
    }

    #[tokio::test]
    async fn settings_user_override() {
        let host = PluginHost::in_memory();
        let mut m = test_manifest("cfg");
        m.contributes.settings.push(ContributedSetting {
            key: "debug".into(),
            label: "Debug".into(),
            description: "Enable debug".into(),
            default_value: serde_json::json!(false),
            value_type: SettingValueType::Boolean,
        });
        host.install(m).await.unwrap();
        host.activate("cfg").await.unwrap();
        host.set_setting("cfg.debug", serde_json::json!(true)).await;
        assert_eq!(
            host.get_setting("cfg.debug").await,
            Some(serde_json::json!(true))
        );
    }

    #[tokio::test]
    async fn activation_event_matching() {
        let host = PluginHost::in_memory();
        let mut m = test_manifest("lazy");
        m.activation_events = vec![ActivationEvent::OnCommand {
            command: "lazy.run".into(),
        }];
        host.install(m).await.unwrap();

        // Startup event should not match
        let startup = host.check_activation(&ActivationEvent::OnStartup).await;
        assert!(startup.is_empty());

        // Matching command event should trigger
        let cmd = host
            .check_activation(&ActivationEvent::OnCommand {
                command: "lazy.run".into(),
            })
            .await;
        assert_eq!(cmd, vec!["lazy"]);
    }

    #[tokio::test]
    async fn status_counts() {
        let host = PluginHost::in_memory();
        host.install(test_manifest("a")).await.unwrap();
        host.install(test_manifest("b")).await.unwrap();
        host.install(test_manifest("c")).await.unwrap();
        host.activate("a").await.unwrap();
        host.deactivate("b").await.unwrap();
        let st = host.status().await;
        assert_eq!(st.total_plugins, 3);
        assert_eq!(st.active_plugins, 1);
        assert_eq!(st.disabled_plugins, 1);
    }

    #[tokio::test]
    async fn disk_persistence_roundtrip() {
        let tmp = tempfile::TempDir::new().unwrap();
        let host1 = PluginHost::new(tmp.path());
        let mut m = test_manifest("persist");
        m.contributes.commands.push(ContributedCommand {
            id: "persist.hello".into(),
            title: "Hello".into(),
            icon: None,
            keybinding: None,
            category: None,
        });
        host1.install(m).await.unwrap();
        host1.activate("persist").await.unwrap();
        drop(host1);

        // Second host loads from disk
        let host2 = PluginHost::new(tmp.path());
        let count = host2.load_installed().await.unwrap();
        assert_eq!(count, 1);
        let p = host2.get_plugin("persist").await.unwrap();
        assert_eq!(p.state, PluginState::Active);
        // Commands should be restored for active plugins
        let cmds = host2.list_commands().await;
        assert_eq!(cmds.len(), 1);
    }

    #[tokio::test]
    async fn invoke_command_resolves_active_command() {
        let host = PluginHost::in_memory();
        let mut m = test_manifest("inv");
        m.contributes.commands.push(ContributedCommand {
            id: "inv.greet".into(),
            title: "Greet".into(),
            icon: None,
            keybinding: None,
            category: None,
        });
        host.install(m).await.unwrap();
        host.activate("inv").await.unwrap();
        let result = host.invoke_command("inv.greet", None).await.unwrap();
        assert!(result.success);
        let out = result.output.unwrap();
        assert!(out.contains("inv"));
        assert!(out.contains("Greet"));
    }

    #[tokio::test]
    async fn invoke_command_unknown_id_errors() {
        let host = PluginHost::in_memory();
        let err = host
            .invoke_command("does.not.exist", None)
            .await
            .unwrap_err();
        assert!(err.contains("unknown command"));
    }

    #[tokio::test]
    async fn invoke_command_includes_args_in_output() {
        let host = PluginHost::in_memory();
        let mut m = test_manifest("argp");
        m.contributes.commands.push(ContributedCommand {
            id: "argp.run".into(),
            title: "Run".into(),
            icon: None,
            keybinding: None,
            category: None,
        });
        host.install(m).await.unwrap();
        host.activate("argp").await.unwrap();
        let args = serde_json::json!({"text": "hello"});
        let r = host.invoke_command("argp.run", Some(args)).await.unwrap();
        assert!(r.output.unwrap().contains("hello"));
    }

    #[tokio::test]
    async fn invoke_command_activates_on_command_plugin() {
        let host = PluginHost::in_memory();
        let mut manifest = test_manifest("lazy");
        manifest.activation_events = vec![ActivationEvent::OnCommand {
            command: "lazy.run".into(),
        }];
        add_command(&mut manifest, "lazy.run", "Lazy Run");
        host.install(manifest).await.unwrap();

        let result = host.invoke_command("lazy.run", None).await.unwrap();

        assert!(result.success);
        assert_eq!(
            host.get_plugin("lazy").await.unwrap().state,
            PluginState::Active
        );
        assert!(result.output.unwrap().contains("Lazy Run"));
    }

    #[tokio::test]
    #[cfg(feature = "wasm-sandbox")]
    async fn invoke_wasm_command_returns_output() {
        let tmp = tempfile::TempDir::new().unwrap();
        let wasm_path = tmp.path().join("command.wasm");
        std::fs::write(&wasm_path, command_wasm("wasm-command-ok")).unwrap();
        let host = PluginHost::new(tmp.path());
        let mut manifest = test_manifest("wasm-tool");
        manifest.install_method = InstallMethod::Wasm {
            url: wasm_path.to_string_lossy().into_owned(),
        };
        add_command(&mut manifest, "wasm-tool.run", "Run Wasm");
        host.install(manifest).await.unwrap();
        host.activate("wasm-tool").await.unwrap();

        let store = TokioMutex::new(CapabilityStore::in_memory());
        let result = host
            .invoke_command_with_store("wasm-tool.run", None, &store)
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.output.as_deref(), Some("wasm-command-ok"));
        assert_eq!(result.exit_code, Some(0));
    }

    #[tokio::test]
    async fn invoke_binary_command_requires_process_spawn() {
        let tmp = tempfile::TempDir::new().unwrap();
        let binary_path = native_echo_script(tmp.path());
        let host = PluginHost::new(tmp.path());
        let mut manifest = test_manifest("native-tool");
        manifest.install_method = InstallMethod::Binary {
            url: binary_path.to_string_lossy().into_owned(),
        };
        add_command(&mut manifest, "native-tool.run", "Run Native");
        host.install(manifest).await.unwrap();
        host.activate("native-tool").await.unwrap();

        let store = TokioMutex::new(CapabilityStore::in_memory());
        let error = host
            .invoke_command_with_store("native-tool.run", None, &store)
            .await
            .unwrap_err();

        assert!(error.contains("ProcessSpawn"));
    }

    #[tokio::test]
    async fn invoke_binary_command_captures_stdout() {
        let tmp = tempfile::TempDir::new().unwrap();
        let binary_path = native_echo_script(tmp.path());
        let host = PluginHost::new(tmp.path());
        let mut manifest = test_manifest("native-ok");
        manifest.install_method = InstallMethod::Binary {
            url: binary_path.to_string_lossy().into_owned(),
        };
        add_command(&mut manifest, "native-ok.run", "Run Native");
        host.install(manifest).await.unwrap();
        host.activate("native-ok").await.unwrap();
        let store = store_with_capability("native-ok", Capability::ProcessSpawn).await;

        let result = host
            .invoke_command_with_store(
                "native-ok.run",
                Some(serde_json::json!({"text":"hello"})),
                &store,
            )
            .await
            .unwrap();

        assert!(result.success);
        let output = result.output.unwrap();
        assert!(output.contains("native-ok"));
        assert!(output.contains("native-ok.run"));
    }

    #[tokio::test]
    async fn invoke_sidecar_command_sends_json_on_stdin() {
        let tmp = tempfile::TempDir::new().unwrap();
        let sidecar_path = sidecar_echo_script(tmp.path());
        let host = PluginHost::new(tmp.path());
        let mut manifest = test_manifest("sidecar-tool");
        manifest.install_method = InstallMethod::Sidecar {
            path: sidecar_path.to_string_lossy().into_owned(),
        };
        add_command(&mut manifest, "sidecar-tool.run", "Run Sidecar");
        host.install(manifest).await.unwrap();
        host.activate("sidecar-tool").await.unwrap();
        let store = store_with_capability("sidecar-tool", Capability::ProcessSpawn).await;

        let result = host
            .invoke_command_with_store(
                "sidecar-tool.run",
                Some(serde_json::json!({"text":"hello"})),
                &store,
            )
            .await
            .unwrap();

        assert!(result.success);
        let output = result.output.unwrap();
        assert!(output.contains("sidecar:"));
        assert!(output.contains("sidecar-tool.run"));
        assert!(output.contains("hello"));
    }

    #[tokio::test]
    async fn invoke_slash_command_resolves_to_command() {
        let host = PluginHost::in_memory();
        let mut m = test_manifest("sl");
        m.contributes.commands.push(ContributedCommand {
            id: "sl.translate".into(),
            title: "Translate".into(),
            icon: None,
            keybinding: None,
            category: None,
        });
        m.contributes.slash_commands.push(ContributedSlashCommand {
            name: "translate".into(),
            description: "Translate text".into(),
            command_id: "sl.translate".into(),
        });
        host.install(m).await.unwrap();
        host.activate("sl").await.unwrap();
        let r = host.invoke_slash_command("translate", None).await.unwrap();
        assert!(r.success);
        assert!(r.output.unwrap().contains("Translate"));
    }

    #[tokio::test]
    async fn invoke_slash_command_unknown_name_errors() {
        let host = PluginHost::in_memory();
        let err = host.invoke_slash_command("nope", None).await.unwrap_err();
        assert!(err.contains("unknown slash-command"));
    }

    #[tokio::test]
    async fn invoke_command_disabled_plugin_errors() {
        let host = PluginHost::in_memory();
        let mut m = test_manifest("dis");
        m.contributes.commands.push(ContributedCommand {
            id: "dis.x".into(),
            title: "X".into(),
            icon: None,
            keybinding: None,
            category: None,
        });
        host.install(m).await.unwrap();
        host.activate("dis").await.unwrap();
        host.deactivate("dis").await.unwrap();
        // After deactivation, command should no longer be invokable.
        let err = host.invoke_command("dis.x", None).await.unwrap_err();
        assert!(err.contains("unknown command"));
    }
}
