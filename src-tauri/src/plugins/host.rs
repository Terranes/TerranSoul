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
use std::sync::Arc;
use tokio::sync::RwLock;

use super::manifest::{
    ActivationEvent, ContributedCommand, ContributedSlashCommand, ContributedTheme,
    InstalledPlugin, PluginManifest, PluginState,
};

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

/// Result of executing a plugin command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
}

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
        Self {
            inner: Arc::new(RwLock::new(HostInner {
                plugins_dir,
                plugins: HashMap::new(),
                commands: HashMap::new(),
                slash_commands: HashMap::new(),
                themes: HashMap::new(),
                settings: HashMap::new(),
            })),
        }
    }

    /// Create an in-memory plugin host for testing.
    pub fn in_memory() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HostInner {
                plugins_dir: PathBuf::from(":memory:"),
                plugins: HashMap::new(),
                commands: HashMap::new(),
                slash_commands: HashMap::new(),
                themes: HashMap::new(),
                settings: HashMap::new(),
            })),
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
    /// **Chunk 22.4** — for now this resolves the `command_id` to a
    /// `CommandEntry` and returns the command's metadata as a
    /// [`CommandResult`] success payload. The full execution path
    /// (sidecar / WASM / native) lands in Chunk 22.7. Until then this
    /// gives the frontend a stable IPC surface and lets ChatView
    /// surface plugin slash-commands without crashing.
    ///
    /// Returns `Err` if the command is not registered (i.e. no active
    /// plugin contributes it).
    pub async fn invoke_command(
        &self,
        command_id: &str,
        args: Option<serde_json::Value>,
    ) -> Result<CommandResult, String> {
        let inner = self.inner.read().await;
        let entry = inner
            .commands
            .get(command_id)
            .ok_or_else(|| format!("unknown command: {command_id}"))?;
        // Stub execution: echo the command's title and any args back.
        let mut output = format!("[{}] {}", entry.plugin_id, entry.command.title,);
        if let Some(a) = args {
            output.push_str(&format!(" — args: {a}"));
        }
        Ok(CommandResult {
            success: true,
            output: Some(output),
            error: None,
        })
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
        // Hold the read lock only long enough to resolve the name.
        let command_id = {
            let inner = self.inner.read().await;
            inner
                .slash_commands
                .get(name)
                .map(|e| e.slash_command.command_id.clone())
                .ok_or_else(|| format!("unknown slash-command: /{name}"))?
        };
        self.invoke_command(&command_id, args).await
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
        t.id.starts_with(plugin_id)
    });
    let prefix = format!("{plugin_id}.");
    inner.settings.retain(|k, _| !k.starts_with(&prefix));
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
