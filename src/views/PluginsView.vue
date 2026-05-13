<template>
  <section
    class="plugins-view"
    data-testid="plugins-view"
  >
    <header class="pv-header">
      <h3>🔌 Plugins</h3>
      <div class="pv-summary">
        <span data-testid="pv-count-active">{{ activeCount }} active</span>
        <span class="pv-divider">·</span>
        <span data-testid="pv-count-total">{{ plugins.length }} installed</span>
        <button
          class="pv-link"
          data-testid="pv-refresh"
          :disabled="loading"
          @click="onRefresh"
        >
          {{ loading ? 'Refreshing…' : 'Refresh' }}
        </button>
      </div>
    </header>

    <p class="pv-help">
      Plugins extend TerranSoul with new agents, tools, themes, widgets, and
      memory processors. Drop a <code>terransoul-plugin.json</code> manifest
      below to install it. Capabilities marked
      <span class="pv-cap-warn">⚠</span> require explicit consent before the
      plugin can use them. See
      <a
        href="https://github.com/Terranes/TerranSoul/blob/main/docs/plugin-development.md"
        target="_blank"
        rel="noopener"
      >plugin development</a>.
    </p>

    <!-- ── Install (drop / pick) ─────────────────────────────────────── -->
    <div
      class="pv-install"
      data-testid="pv-install"
      :class="{ 'pv-install-dragover': isDragOver }"
      @dragover.prevent="isDragOver = true"
      @dragleave.prevent="isDragOver = false"
      @drop.prevent="onFileDrop"
    >
      <p class="pv-install-line">
        <strong>Install plugin</strong>
        — drop a <code>.json</code> manifest here or
        <button
          type="button"
          class="pv-link"
          data-testid="pv-pick"
          @click="onPickFile"
        >
          choose a file
        </button>
        .
      </p>
      <input
        ref="fileInput"
        type="file"
        accept=".json,application/json"
        style="display:none"
        data-testid="pv-file-input"
        @change="onFileChange"
      >
      <p
        v-if="installError"
        class="pv-install-error"
        data-testid="pv-install-error"
      >
        Install failed: {{ installError }}
      </p>
      <p
        v-else-if="lastInstalled"
        class="pv-install-ok"
        data-testid="pv-install-ok"
      >
        ✓ Installed <strong>{{ lastInstalled }}</strong>
      </p>
    </div>

    <!-- ── Theme picker (Chunk 22.3) ─────────────────────────────────── -->
    <div
      v-if="availableThemes.length > 0"
      class="pv-theme-picker"
      data-testid="pv-theme-picker"
    >
      <label class="pv-theme-label">
        <strong>Active plugin theme</strong>
        <select
          :value="activeThemeId ?? ''"
          data-testid="pv-theme-select"
          @change="onThemeChange(($event.target as HTMLSelectElement).value)"
        >
          <option value="">— None (use built-in theme) —</option>
          <option
            v-for="t in availableThemes"
            :key="t.id"
            :value="t.id"
          >
            {{ t.label }}
          </option>
        </select>
      </label>
      <p class="pv-theme-help">
        Themes contributed by active plugins override the built-in
        <code>--ts-*</code> tokens. Disable a plugin to remove its theme.
      </p>
    </div>

    <!-- ── Plugin list ───────────────────────────────────────────────── -->
    <div
      v-if="plugins.length === 0"
      class="pv-empty"
      data-testid="pv-empty"
    >
      No plugins installed yet. Drop a manifest above to get started.
    </div>
    <ul
      v-else
      class="pv-list"
      data-testid="pv-list"
    >
      <li
        v-for="plugin in plugins"
        :key="plugin.manifest.id"
        class="pv-card"
        :data-testid="`pv-card-${plugin.manifest.id}`"
      >
        <div class="pv-card-head">
          <div class="pv-card-title">
            <strong>{{ plugin.manifest.display_name }}</strong>
            <span class="pv-card-id">{{ plugin.manifest.id }} · v{{ plugin.manifest.version }}</span>
          </div>
          <span
            :class="['pv-pill', `pv-pill-${stateLabel(plugin).toLowerCase()}`]"
            :data-testid="`pv-state-${plugin.manifest.id}`"
          >
            {{ stateLabel(plugin) }}
          </span>
        </div>
        <p class="pv-card-desc">
          {{ plugin.manifest.description }}
        </p>
        <p class="pv-card-meta">
          <span>kind: <code>{{ plugin.manifest.kind }}</code></span>
          <span class="pv-divider">·</span>
          <span>installed {{ formatTimestamp(plugin.installed_at) }}</span>
          <template v-if="plugin.last_active_at">
            <span class="pv-divider">·</span>
            <span>last active {{ formatTimestamp(plugin.last_active_at) }}</span>
          </template>
        </p>

        <!-- ── Capability grants ─────────────────────────────────────── -->
        <div
          v-if="plugin.manifest.capabilities.length > 0"
          class="pv-caps"
          :data-testid="`pv-caps-${plugin.manifest.id}`"
        >
          <h4>Capabilities</h4>
          <ul class="pv-cap-list">
            <li
              v-for="cap in plugin.manifest.capabilities"
              :key="cap"
              class="pv-cap-item"
            >
              <label>
                <input
                  type="checkbox"
                  :checked="grants[plugin.manifest.id]?.[cap] ?? !capabilityRequiresConsent(cap)"
                  :disabled="!capabilityRequiresConsent(cap) || isPluginActive(plugin)"
                  :data-testid="`pv-grant-${plugin.manifest.id}-${cap}`"
                  @change="onGrantToggle(plugin.manifest.id, cap, ($event.target as HTMLInputElement).checked)"
                >
                <span class="pv-cap-name">{{ cap }}</span>
                <span
                  v-if="capabilityRequiresConsent(cap)"
                  class="pv-cap-warn"
                  title="Requires explicit consent"
                >⚠</span>
                <span
                  v-else
                  class="pv-cap-auto"
                  title="Auto-granted (low risk)"
                >·</span>
              </label>
            </li>
          </ul>
        </div>

        <!-- ── Settings (Chunk 22.6) ─────────────────────────────────── -->
        <div
          v-if="(plugin.manifest.contributes.settings ?? []).length > 0"
          class="pv-settings"
          :data-testid="`pv-settings-${plugin.manifest.id}`"
        >
          <h4>Settings</h4>
          <ul class="pv-setting-list">
            <li
              v-for="setting in (plugin.manifest.contributes.settings ?? [])"
              :key="setting.key"
              class="pv-setting-item"
            >
              <label class="pv-setting-label">
                <span class="pv-setting-name">{{ setting.label }}</span>
                <span class="pv-setting-key">{{ plugin.manifest.id }}.{{ setting.key }}</span>
              </label>
              <p class="pv-setting-desc">
                {{ setting.description }}
              </p>

              <!-- Boolean → toggle -->
              <input
                v-if="settingTypeKind(setting.value_type) === 'boolean'"
                type="checkbox"
                :checked="readSetting(plugin.manifest.id, setting.key) === true"
                :data-testid="`pv-setting-${plugin.manifest.id}-${setting.key}`"
                @change="writeSetting(plugin.manifest.id, setting.key, ($event.target as HTMLInputElement).checked)"
              >

              <!-- Number → number input -->
              <input
                v-else-if="settingTypeKind(setting.value_type) === 'number'"
                type="number"
                :value="settingInputValue(plugin.manifest.id, setting.key)"
                :data-testid="`pv-setting-${plugin.manifest.id}-${setting.key}`"
                @change="writeSetting(plugin.manifest.id, setting.key, parseSettingNumber(($event.target as HTMLInputElement).value))"
              >

              <!-- Enum → select -->
              <select
                v-else-if="settingTypeKind(setting.value_type) === 'enum'"
                :value="(readSetting(plugin.manifest.id, setting.key) ?? '') as string"
                :data-testid="`pv-setting-${plugin.manifest.id}-${setting.key}`"
                @change="writeSetting(plugin.manifest.id, setting.key, ($event.target as HTMLSelectElement).value)"
              >
                <option
                  v-for="opt in enumValues(setting.value_type)"
                  :key="opt"
                  :value="opt"
                >
                  {{ opt }}
                </option>
              </select>

              <!-- String → text input (default) -->
              <input
                v-else
                type="text"
                :value="(readSetting(plugin.manifest.id, setting.key) ?? '') as string"
                :data-testid="`pv-setting-${plugin.manifest.id}-${setting.key}`"
                @change="writeSetting(plugin.manifest.id, setting.key, ($event.target as HTMLInputElement).value)"
              >

              <p
                v-if="settingErrors[`${plugin.manifest.id}.${setting.key}`]"
                class="pv-setting-error"
                :data-testid="`pv-setting-error-${plugin.manifest.id}-${setting.key}`"
              >
                {{ settingErrors[`${plugin.manifest.id}.${setting.key}`] }}
              </p>
            </li>
          </ul>
        </div>

        <!-- ── Actions ───────────────────────────────────────────────── -->
        <div class="pv-actions">
          <button
            v-if="!isPluginActive(plugin)"
            class="pv-btn pv-btn-primary"
            :disabled="!canActivate(plugin)"
            :data-testid="`pv-activate-${plugin.manifest.id}`"
            @click="onActivate(plugin.manifest.id)"
          >
            Activate
          </button>
          <button
            v-else
            class="pv-btn"
            :data-testid="`pv-disable-${plugin.manifest.id}`"
            @click="onDeactivate(plugin.manifest.id)"
          >
            Disable
          </button>
          <button
            class="pv-btn pv-btn-danger"
            :data-testid="`pv-uninstall-${plugin.manifest.id}`"
            @click="onUninstall(plugin.manifest.id)"
          >
            Uninstall
          </button>
        </div>

        <p
          v-if="actionErrors[plugin.manifest.id]"
          class="pv-action-error"
          :data-testid="`pv-error-${plugin.manifest.id}`"
        >
          {{ actionErrors[plugin.manifest.id] }}
        </p>
        <p
          v-if="errorMessage(plugin)"
          class="pv-action-error"
          :data-testid="`pv-state-error-${plugin.manifest.id}`"
        >
          Activation error: {{ errorMessage(plugin) }}
        </p>
      </li>
    </ul>
  </section>
</template>

<script setup lang="ts">
/**
 * **Chunk 22.2** — PluginsView UI surface.
 *
 * List installed plugins, install new ones via drag-and-drop or file picker,
 * grant capabilities, activate / disable / uninstall. Embedded inside
 * `BrainView.vue` rather than a standalone tab per the milestone spec.
 */
import { computed, onMounted, ref } from 'vue';
import { usePluginStore, type InstalledPlugin } from '../stores/plugins';
import { useActivePluginTheme } from '../composables/useActivePluginTheme';
import { usePluginCapabilityGrants } from '../composables/usePluginCapabilityGrants';

const store = usePluginStore();
const plugins = computed(() => store.plugins);
const loading = computed(() => store.loading);

const { activeThemeId, availableThemes, setActivePluginTheme } = useActivePluginTheme();

function onThemeChange(value: string) {
  setActivePluginTheme(value === '' ? null : value);
}

const fileInput = ref<HTMLInputElement | null>(null);
const isDragOver = ref(false);
const installError = ref<string | null>(null);
const lastInstalled = ref<string | null>(null);
const actionErrors = ref<Record<string, string>>({});
const {
  grants,
  capabilityRequiresConsent,
  canActivate,
  loadAllGrants,
  onGrantToggle,
} = usePluginCapabilityGrants(plugins, store, actionErrors);

// ── Settings (Chunk 22.6) ────────────────────────────────────────
/** Map of `${pluginId}.${key}` → current value, kept in sync with backend. */
const settingValues = ref<Record<string, unknown>>({});
/** Per-setting error message keyed by `${pluginId}.${key}`. */
const settingErrors = ref<Record<string, string>>({});

/**
 * Discriminate the `value_type` shape.  Backend `SettingValueType` is
 * a tagged enum: `"string" | "number" | "boolean" | { enum: { values } }`.
 */
type SettingValueTypeT =
  | string
  | { enum?: { values: string[] }; Enum?: { values: string[] } };
function settingTypeKind(t: SettingValueTypeT): 'string' | 'number' | 'boolean' | 'enum' {
  if (typeof t === 'string') {
    if (t === 'boolean' || t === 'number') return t;
    return 'string';
  }
  if (t && (t.enum || t.Enum)) return 'enum';
  return 'string';
}
function enumValues(t: SettingValueTypeT): string[] {
  if (typeof t === 'object' && t) {
    return (t.enum?.values ?? t.Enum?.values) ?? [];
  }
  return [];
}
function readSetting(pluginId: string, key: string): unknown {
  return settingValues.value[`${pluginId}.${key}`];
}
function settingInputValue(pluginId: string, key: string): string | number {
  const value = readSetting(pluginId, key);
  return typeof value === 'number' || typeof value === 'string' ? value : '';
}
function parseSettingNumber(raw: string): number | null {
  if (raw.trim() === '') return null;
  const n = Number(raw);
  return Number.isFinite(n) ? n : null;
}
async function writeSetting(pluginId: string, key: string, value: unknown) {
  const fullKey = `${pluginId}.${key}`;
  delete settingErrors.value[fullKey];
  // Optimistic update so the input reflects the new value immediately.
  settingValues.value[fullKey] = value;
  try {
    await store.setSetting(fullKey, value);
  } catch (e) {
    settingErrors.value[fullKey] = String(e);
  }
}
async function loadSettingsForPlugin(plugin: InstalledPlugin) {
  const id = plugin.manifest.id;
  for (const setting of plugin.manifest.contributes.settings ?? []) {
    const fullKey = `${id}.${setting.key}`;
    try {
      const v = await store.getSetting(fullKey);
      settingValues.value[fullKey] = v ?? setting.default_value;
    } catch (e) {
      settingErrors.value[fullKey] = String(e);
    }
  }
}
async function loadAllSettings() {
  await Promise.all(plugins.value.map(loadSettingsForPlugin));
}

function stateLabel(plugin: InstalledPlugin): string {
  const s = plugin.state;
  if (typeof s === 'string') {
    // Backend serializes `PluginState` with `rename_all = "snake_case"` →
    // 'installed' | 'active' | 'disabled'.
    return s.charAt(0).toUpperCase() + s.slice(1);
  }
  return 'Error';
}

function isPluginActive(plugin: InstalledPlugin): boolean {
  return typeof plugin.state === 'string' && plugin.state.toLowerCase() === 'active';
}

function errorMessage(plugin: InstalledPlugin): string | null {
  const s = plugin.state;
  if (typeof s === 'object' && s !== null && 'Error' in s) {
    return s.Error.message;
  }
  return null;
}

const activeCount = computed(() => plugins.value.filter(isPluginActive).length);

function formatTimestamp(epoch: number | null): string {
  if (!epoch) return 'never';
  // Backend stores epoch *seconds* (i64).
  const ms = epoch < 1e12 ? epoch * 1000 : epoch;
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: 'short',
    timeStyle: 'short',
  }).format(new Date(ms));
}

async function onRefresh() {
  await store.refresh();
  await Promise.all([loadAllSettings(), loadAllGrants()]);
}

async function onPickFile() {
  fileInput.value?.click();
}

async function onFileChange(ev: Event) {
  const target = ev.target as HTMLInputElement;
  const file = target.files?.[0];
  if (file) {
    await installFromFile(file);
    target.value = '';
  }
}

async function onFileDrop(ev: DragEvent) {
  isDragOver.value = false;
  const file = ev.dataTransfer?.files?.[0];
  if (file) {
    await installFromFile(file);
  }
}

async function installFromFile(file: File) {
  installError.value = null;
  lastInstalled.value = null;
  if (!file.name.endsWith('.json')) {
    installError.value = `Expected a .json manifest, got ${file.name}.`;
    return;
  }
  try {
    const text = await file.text();
    const installed = await store.install(text);
    lastInstalled.value = installed.manifest.display_name;
  } catch (e) {
    installError.value = String(e);
  }
}

async function onActivate(pluginId: string) {
  delete actionErrors.value[pluginId];
  try {
    await store.activate(pluginId);
  } catch (e) {
    actionErrors.value[pluginId] = String(e);
  }
}

async function onDeactivate(pluginId: string) {
  delete actionErrors.value[pluginId];
  try {
    await store.deactivate(pluginId);
  } catch (e) {
    actionErrors.value[pluginId] = String(e);
  }
}

async function onUninstall(pluginId: string) {
  delete actionErrors.value[pluginId];
  if (!confirm(`Uninstall plugin "${pluginId}"? This cannot be undone.`)) return;
  try {
    await store.uninstall(pluginId);
  } catch (e) {
    actionErrors.value[pluginId] = String(e);
  }
}

onMounted(() => {
  store
    .refresh()
    .then(() => Promise.all([loadAllSettings(), loadAllGrants()]))
    .catch((e) => {
      console.warn('[PluginsView] initial refresh failed:', e);
    });
});

defineExpose({
  // For tests.
  capabilityRequiresConsent,
  stateLabel,
  isPluginActive,
  canActivate,
  formatTimestamp,
  errorMessage,
  settingTypeKind,
  enumValues,
  parseSettingNumber,
});
</script>

<style scoped>
.plugins-view {
  background: var(--ts-bg-surface);
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-md);
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  color: var(--ts-text-primary);
}

.pv-header,
.pv-card-head,
.pv-setting-label,
.pv-actions {
  display: flex;
  gap: 8px;
}

.pv-header,
.pv-card-head,
.pv-setting-label {
  align-items: center;
  justify-content: space-between;
}

.pv-header,
.pv-actions {
  flex-wrap: wrap;
}

.pv-header h3,
.pv-caps h4,
.pv-settings h4 {
  margin: 0;
}

.pv-header h3 { font-size: 1.05rem; }

.pv-summary,
.pv-card-meta,
.pv-theme-label,
.pv-cap-list,
.pv-list,
.pv-setting-list {
  display: flex;
}

.pv-summary,
.pv-card-meta,
.pv-cap-list {
  flex-wrap: wrap;
}

.pv-summary {
  align-items: center;
  gap: 8px;
  font-size: 0.85rem;
  color: var(--ts-text-muted);
}

.pv-divider,
.pv-empty,
.pv-theme-help,
.pv-card-id,
.pv-card-meta,
.pv-caps h4,
.pv-settings h4,
.pv-setting-key,
.pv-setting-desc,
.pv-help {
  color: var(--ts-text-muted);
}

.pv-help {
  margin: 0;
  font-size: 0.85rem;
  line-height: 1.5;
}

.pv-help a,
.pv-link,
.pv-btn-primary {
  color: var(--ts-accent-blue);
}

.pv-link {
  background: transparent;
  border: none;
  cursor: pointer;
  font: inherit;
  padding: 0;
  text-decoration: underline;
}
.pv-link:disabled {
  color: var(--ts-text-muted);
  cursor: not-allowed;
}

.pv-install {
  border: 2px dashed var(--ts-border-medium);
  border-radius: var(--ts-radius-md);
  padding: 14px;
  text-align: center;
  transition: border-color var(--ts-transition-fast), background-color var(--ts-transition-fast);
}
.pv-install-dragover {
  border-color: var(--ts-accent-blue);
  background: var(--ts-bg-hover);
}
.pv-install-line,
.pv-setting-error,
.pv-action-error,
.pv-install-error,
.pv-install-ok {
  margin: 0;
}
.pv-install-error,
.pv-setting-error,
.pv-action-error { color: var(--ts-error); }
.pv-install-ok { color: var(--ts-success); }

.pv-empty {
  text-align: center;
  padding: 24px;
  font-style: italic;
}

.pv-theme-picker,
.pv-card {
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-md);
  background: var(--ts-bg-panel);
}
.pv-theme-picker { padding: 10px 12px; }
.pv-theme-label {
  align-items: center;
  gap: 10px;
  font-size: 0.9rem;
}
.pv-theme-label select,
.pv-setting-item input[type='text'],
.pv-setting-item input[type='number'],
.pv-setting-item select {
  padding: 4px 8px;
  border-radius: var(--ts-radius-sm);
  border: 1px solid var(--ts-border);
  background: var(--ts-bg-input);
  color: var(--ts-text-primary);
  font: inherit;
}
.pv-theme-label select { flex: 1; }
.pv-theme-help,
.pv-install-error,
.pv-install-ok,
.pv-setting-error,
.pv-action-error { font-size: 0.8rem; }
.pv-theme-help { margin: 6px 0 0; }

.pv-list,
.pv-setting-list {
  list-style: none;
  margin: 0;
  padding: 0;
  flex-direction: column;
}
.pv-list { gap: 10px; }
.pv-setting-list { gap: 8px; }

.pv-card {
  padding: 12px;
}
.pv-card-title strong { font-size: 1rem; }
.pv-card-id {
  font-size: 0.8rem;
  margin-left: 6px;
  font-family: var(--ts-font-mono);
}
.pv-card-desc {
  margin: 6px 0 0;
  font-size: 0.9rem;
  line-height: 1.4;
}
.pv-card-meta {
  margin: 6px 0 0;
  font-size: 0.8rem;
  gap: 4px;
}

.pv-pill {
  font-size: 0.75rem;
  padding: 2px 8px;
  border-radius: var(--ts-radius-pill);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
.pv-pill-active { background: var(--ts-success-bg); color: var(--ts-success); }
.pv-pill-installed { background: var(--ts-bg-selected); color: var(--ts-text-muted); }
.pv-pill-disabled { background: var(--ts-bg-selected); color: var(--ts-text-dim); }
.pv-pill-error { background: var(--ts-error-bg); color: var(--ts-error); }

.pv-caps,
.pv-settings,
.pv-actions { margin-top: 10px; }
.pv-caps h4,
.pv-settings h4 {
  font-size: 0.85rem;
  margin-bottom: 6px;
  font-weight: 600;
}
.pv-cap-list {
  list-style: none;
  margin: 0;
  padding: 0;
  gap: 8px;
}
.pv-cap-item label {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px;
  background: var(--ts-bg-input);
  border-radius: var(--ts-radius-sm);
  font-size: 0.85rem;
  cursor: pointer;
}
.pv-cap-name,
.pv-setting-key,
code { font-family: var(--ts-font-mono); }
.pv-cap-warn { color: var(--ts-warning); }
.pv-cap-auto { color: var(--ts-text-muted); }

.pv-settings {
  padding-top: 10px;
  border-top: 1px dashed var(--ts-border);
}
.pv-setting-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.pv-setting-label {
  align-items: baseline;
  font-size: 0.85rem;
}
.pv-setting-name { font-weight: 600; }
.pv-setting-key { font-size: 0.75rem; }
.pv-setting-desc { margin: 0; font-size: 0.8rem; }
.pv-setting-item input[type='text'],
.pv-setting-item input[type='number'],
.pv-setting-item select { max-width: 280px; }

.pv-btn {
  padding: 6px 12px;
  border-radius: var(--ts-radius-sm);
  border: 1px solid var(--ts-border);
  background: var(--ts-bg-input);
  color: var(--ts-text-primary);
  cursor: pointer;
  font-size: 0.85rem;
}
.pv-btn:hover:not(:disabled) { background: var(--ts-bg-hover); }
.pv-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.pv-btn-primary { border-color: var(--ts-accent-blue); }
.pv-btn-danger {
  border-color: var(--ts-error);
  color: var(--ts-error);
}

code {
  font-size: 0.85em;
  padding: 1px 4px;
  background: var(--ts-bg-input);
  border-radius: var(--ts-radius-sm);
}
</style>
