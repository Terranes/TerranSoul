<template>
  <div
    class="bp-shell settings-view"
    data-density="cozy"
    data-testid="settings-view"
  >
    <AppBreadcrumb
      here="SETTINGS"
      @navigate="emit('navigate', $event)"
    />

    <header class="sv-hero">
      <h1 class="sv-title">
        Settings
      </h1>
      <p class="sv-subtitle">
        App-wide preferences. Brain, Voice, and Memory have dedicated panels —
        use the side nav or the buttons below for deep configuration.
      </p>
    </header>

    <!-- ── View Mode ───────────────────────────────────────────────── -->
    <section
      class="sv-section ts-cockpit-card ts-cockpit-card--compact"
      data-testid="settings-section-view-mode"
    >
      <h2 class="sv-section-title">
        View Mode
      </h2>
      <p class="sv-section-help">
        Choose how the companion appears on your desktop.
      </p>
      <div class="sv-mode-row">
        <button
          type="button"
          class="sv-mode-btn"
          :class="{ active: !isPetMode && !isChatboxMode }"
          data-testid="settings-view-mode-desktop"
          @click="onSetDisplayMode('desktop')"
        >
          <span class="sv-mode-icon">🖥</span>
          <span class="sv-mode-name">3D Desktop</span>
          <span class="sv-mode-desc">Full 3D character viewport</span>
        </button>
        <button
          type="button"
          class="sv-mode-btn"
          :class="{ active: isChatboxMode && !isPetMode }"
          data-testid="settings-view-mode-chatbox"
          @click="onSetDisplayMode('chatbox')"
        >
          <span class="sv-mode-icon">💬</span>
          <span class="sv-mode-name">Chatbox</span>
          <span class="sv-mode-desc">Compact text-first layout</span>
        </button>
        <button
          type="button"
          class="sv-mode-btn"
          :class="{ active: isPetMode }"
          data-testid="settings-view-mode-pet"
          @click="onTogglePetMode"
        >
          <span class="sv-mode-icon">🐾</span>
          <span class="sv-mode-name">Pet Mode</span>
          <span class="sv-mode-desc">Floating desktop companion</span>
        </button>
      </div>
    </section>

    <!-- ── Appearance ───────────────────────────────────────────────── -->
    <section
      class="sv-section ts-cockpit-card ts-cockpit-card--compact"
      data-testid="settings-section-appearance"
    >
      <h2 class="sv-section-title">
        Appearance
      </h2>
      <p class="sv-section-help">
        Pick from the full built-in theme set — dark, light, and colourful
        variants. The choice is persisted to settings and applied app-wide.
      </p>
      <ThemePicker />
    </section>

    <!-- ── Character ───────────────────────────────────────────────── -->
    <section
      class="sv-section ts-cockpit-card ts-cockpit-card--compact"
      data-testid="settings-section-character"
    >
      <h2 class="sv-section-title">
        Character
      </h2>
      <p class="sv-section-help">
        Active VRM model used by the 3D viewport.
      </p>
      <div class="sv-row">
        <label class="sv-field">
          <span class="sv-field-label">Active model</span>
          <select
            class="sv-select"
            data-testid="settings-character-model"
            :value="characterStore.selectedModelId ?? ''"
            @change="onModelChange"
          >
            <optgroup label="Bundled">
              <option
                v-for="model in characterStore.defaultModels"
                :key="model.id"
                :value="model.id"
              >
                {{ characterStore.resolveModelProfile(model).name }}
              </option>
            </optgroup>
            <optgroup
              v-if="characterStore.userModels.length > 0"
              label="Imported"
            >
              <option
                v-for="model in characterStore.userModels"
                :key="model.id"
                :value="model.id"
              >
                {{ characterStore.resolveModelProfile(model).name }}
              </option>
            </optgroup>
          </select>
        </label>
      </div>
    </section>

    <!-- ── Persona (per active character/model) ───────────────────── -->
    <section
      class="sv-section ts-cockpit-card ts-cockpit-card--compact"
      data-testid="settings-section-persona"
    >
      <h2 class="sv-section-title">
        Persona — {{ activeCharacterName }}
      </h2>
      <p class="sv-section-help">
        Persona traits and voice design apply to the active character/model
        above. Switch characters to edit a different persona. These settings
        are not TerranSoul-wide.
      </p>
      <PersonaPanel />
    </section>

    <!-- ── Quick Links ───────────────────────────────────────────────── -->
    <section
      class="sv-section ts-cockpit-card ts-cockpit-card--compact"
      data-testid="settings-section-quick-links"
    >
      <h2 class="sv-section-title">
        Deep Configuration
      </h2>
      <p class="sv-section-help">
        Detailed settings live in dedicated panels.
      </p>
      <div class="sv-links">
        <button
          type="button"
          class="sv-link-btn"
          data-testid="settings-link-brain"
          @click="emit('navigate', 'brain')"
        >
          <span class="sv-link-icon">🧠</span>
          <span class="sv-link-body">
            <span class="sv-link-title">Brain &amp; Knowledge</span>
            <span class="sv-link-desc">LLM providers, models, retrieval</span>
          </span>
        </button>
        <button
          type="button"
          class="sv-link-btn"
          data-testid="settings-link-voice"
          @click="emit('navigate', 'voice')"
        >
          <span class="sv-link-icon">🎙</span>
          <span class="sv-link-body">
            <span class="sv-link-title">Voice</span>
            <span class="sv-link-desc">TTS / ASR configuration</span>
          </span>
        </button>
        <button
          type="button"
          class="sv-link-btn"
          data-testid="settings-link-memory"
          @click="emit('navigate', 'memory')"
        >
          <span class="sv-link-icon">📚</span>
          <span class="sv-link-body">
            <span class="sv-link-title">Knowledge Graphs</span>
            <span class="sv-link-desc">Persistent memory and RAG</span>
          </span>
        </button>
        <button
          type="button"
          class="sv-link-btn"
          data-testid="settings-link-mobile"
          @click="emit('navigate', 'mobile')"
        >
          <span class="sv-link-icon">📱</span>
          <span class="sv-link-body">
            <span class="sv-link-title">Link</span>
            <span class="sv-link-desc">Pair phones &amp; other devices</span>
          </span>
        </button>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import AppBreadcrumb from '../components/ui/AppBreadcrumb.vue';
import PersonaPanel from '../components/PersonaPanel.vue';
import ThemePicker from '../components/ThemePicker.vue';
import { useSettingsStore } from '../stores/settings';
import { useWindowStore } from '../stores/window';
import { useCharacterStore } from '../stores/character';

const emit = defineEmits<{
  (e: 'navigate', target: string): void;
}>();

const settingsStore = useSettingsStore();
const windowStore = useWindowStore();
const characterStore = useCharacterStore();

const isPetMode = computed(() => windowStore.mode === 'pet');
const isChatboxMode = computed(
  () => settingsStore.settings.chatbox_mode === true,
);

/** Display name for the active character — surfaces in the per-character
 *  Persona section heading so the per-model scope is visible. */
const activeCharacterName = computed(() => {
  const id = characterStore.selectedModelId;
  const all = [...characterStore.defaultModels, ...characterStore.userModels];
  const match = all.find((m) => m.id === id);
  if (!match) return 'active character';
  return characterStore.resolveModelProfile(match).name;
});

// Theme selection is delegated to <ThemePicker /> which uses the shared
// useTheme() composable and the full theme registry in config/themes.ts.
// SettingsView no longer maintains its own 2-value theme state.

async function onSetDisplayMode(mode: 'desktop' | 'chatbox') {
  // Leaving pet mode if active.
  if (isPetMode.value) {
    await windowStore.setMode('window');
  }
  await settingsStore.setChatboxMode(mode === 'chatbox');
}

async function onTogglePetMode() {
  await windowStore.toggleMode();
}

function onModelChange(e: Event) {
  const id = (e.target as HTMLSelectElement).value;
  if (!id) return;
  void characterStore.selectModel(id);
}
</script>

<style scoped>
.settings-view { container-type: inline-size; }

.sv-hero {
  display: flex;
  flex-direction: column;
  gap: var(--ts-space-xs, 4px);
}

.sv-title {
  font-family: var(--ts-font-display, var(--ts-font-sans));
  font-size: 1.6rem;
  font-weight: 700;
  color: var(--ts-text-bright);
  letter-spacing: 0.02em;
  margin: 0;
}

.sv-subtitle {
  color: var(--ts-text-secondary);
  font-size: 0.85rem;
  max-width: 60ch;
  margin: 0;
}

.sv-section {
  display: flex;
  flex-direction: column;
  gap: var(--ts-space-sm, 8px);
  padding: var(--ts-space-lg, 16px);
}

/* Cockpit utility provides border/background/shadow when present; this
   fallback only applies if the .ts-cockpit-card class is absent. */
.sv-section:not(.ts-cockpit-card) {
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-md, 10px);
  background: var(--ts-bg-card);
}

.sv-section-title {
  font-size: 1rem;
  font-weight: 600;
  color: var(--ts-text-bright);
  margin: 0;
}

.sv-section-help {
  color: var(--ts-text-muted);
  font-size: 0.78rem;
  margin: 0;
}

/* ── View mode buttons ─────────────────────────────────────────────── */
.sv-mode-row {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: var(--ts-space-sm, 8px);
}

.sv-mode-btn {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: var(--ts-space-xs, 4px);
  padding: var(--ts-space-md, 12px);
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-sm, 8px);
  background: var(--ts-bg-elevated);
  color: var(--ts-text-primary);
  cursor: pointer;
  text-align: left;
  transition: border-color 0.15s ease, background 0.15s ease,
    transform 0.15s ease;
}

.sv-mode-btn:hover {
  border-color: var(--ts-accent);
  transform: translateY(-1px);
}

.sv-mode-btn.active {
  border-color: var(--ts-accent);
  background: var(--ts-bg-hover);
  box-shadow: inset 0 0 0 1px var(--ts-accent);
}

.sv-mode-icon {
  font-size: 1.4rem;
  line-height: 1;
}

.sv-mode-name {
  font-weight: 600;
  color: var(--ts-text-bright);
}

.sv-mode-desc {
  color: var(--ts-text-muted);
  font-size: 0.75rem;
}

/* Theme picker styling is owned by ThemePicker.vue (scoped). */

/* ── Character ─────────────────────────────────────────────────────── */
.sv-row { display: flex; gap: var(--ts-space-md, 12px); flex-wrap: wrap; }

.sv-field {
  display: flex;
  flex-direction: column;
  gap: var(--ts-space-xs, 4px);
  min-width: 240px;
  flex: 1;
}

.sv-field-label {
  color: var(--ts-text-muted);
  font-size: 0.72rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.sv-select {
  background: var(--ts-bg-input);
  color: var(--ts-text-primary);
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-sm, 6px);
  padding: var(--ts-space-sm, 8px) var(--ts-space-md, 12px);
  font-size: 0.85rem;
}

.sv-select:focus-visible {
  outline: 2px solid var(--ts-accent);
  outline-offset: 1px;
}

/* ── Quick Links ───────────────────────────────────────────────────── */
.sv-links {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: var(--ts-space-sm, 8px);
}

.sv-link-btn {
  display: flex;
  align-items: center;
  gap: var(--ts-space-md, 12px);
  padding: var(--ts-space-md, 12px);
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-sm, 8px);
  background: var(--ts-bg-elevated);
  color: var(--ts-text-primary);
  cursor: pointer;
  text-align: left;
  transition: border-color 0.15s ease, background 0.15s ease,
    transform 0.15s ease;
}

.sv-link-btn:hover {
  border-color: var(--ts-accent);
  background: var(--ts-bg-hover);
  transform: translateY(-1px);
}

.sv-link-icon {
  font-size: 1.4rem;
  line-height: 1;
}

.sv-link-body {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.sv-link-title {
  font-weight: 600;
  color: var(--ts-text-bright);
}

.sv-link-desc {
  font-size: 0.75rem;
  color: var(--ts-text-muted);
}
</style>
