<template>
  <!-- Panel-only window: opened from pet mode context menu -->
  <div
    v-if="panelOnly"
    class="app-shell panel-window"
  >
    <main class="app-main panel-main">
      <SkillTreeView
        v-if="panelOnly === 'skills'"
        @navigate="handleSkillNavigate"
      />
      <BrainView
        v-if="panelOnly === 'brain'"
        @navigate="handleSkillNavigate"
      />
      <MemoryView v-if="panelOnly === 'memory'" />
      <MarketplaceView v-if="panelOnly === 'marketplace'" />
      <VoiceSetupView
        v-if="panelOnly === 'voice'"
        @done="() => {}"
      />
    </main>
  </div>

  <template v-else>
  <!-- Loading splash shown during app initialization -->
  <Transition name="splash-fade">
    <SplashScreen v-if="appLoading" />
  </Transition>

  <!-- First-launch wizard (shown once, before the main app) -->
  <FirstLaunchWizard
    :visible="showFirstLaunchWizard"
    @done="onFirstLaunchDone"
  />

  <div
    v-show="!appLoading"
    class="app-shell"
    :class="{ 'pet-mode': isPetMode }"
  >

    <!-- Pet overlay mode: transparent character + floating chat -->
    <div v-if="isPetMode" class="pet-mode-wrapper">
      <!-- DEV badge — inline in the pet mode layout, top-left -->
      <FloatingBadge
        v-if="windowStore.isDevBuild"
        class="pet-dev-badge"
        tone="warning"
        readonly
        title="Development build — MCP on port 7422"
      >
        DEV
      </FloatingBadge>
      <PetOverlayView />
    </div>

    <!-- Normal mode: Brain onboarding or tabbed UI -->
    <template v-else>
      <!-- Brain onboarding: shown until a brain is configured -->
      <BrainSetupView
        v-if="!hasBrain && !skipSetup"
        @done="onBrainDone"
      />

      <template v-else>
        <!-- Desktop side navigation -->
        <nav class="app-nav desktop-nav">
          <div class="nav-logo">
            <img
              :src="appIconUrl"
              alt="TerranSoul"
              class="nav-logo-img"
            >
          </div>
          <button
            v-for="tab in tabs"
            :key="tab.id"
            :class="['nav-btn', { active: activeTab === tab.id }]"
            @click="activeTab = tab.id"
          >
            <span
              class="nav-icon"
              v-html="tab.svg"
            />
            <span class="nav-label">{{ tab.label }}</span>
          </button>

          <div class="nav-spacer" />

          <!-- "No brain" warning pill -->
          <button
            v-if="!hasBrain"
            class="nav-btn nav-brain-warn"
            @click="skipSetup = false"
          >
            <span class="nav-icon">⚠</span>
            <span class="nav-label">Brain</span>
          </button>

          <!-- DEV badge — inline in the sidebar, below spacer -->
          <FloatingBadge
            v-if="windowStore.isDevBuild"
            class="nav-dev-badge"
            tone="warning"
            readonly
            title="Development build — MCP on port 7422"
          >
            DEV
          </FloatingBadge>
        </nav>

        <!-- Mobile bottom tab bar (replaces hamburger menu) -->
        <nav class="mobile-bottom-nav">
          <!-- DEV indicator — sits as first item in the tab row -->
          <span
            v-if="windowStore.isDevBuild"
            class="mobile-dev-indicator"
            title="Development build"
          >DEV</span>
          <button
            v-for="tab in tabs"
            :key="tab.id"
            :class="['mobile-tab', { active: activeTab === tab.id }]"
            @click="activeTab = tab.id"
          >
            <span
              class="mobile-tab-icon"
              v-html="tab.svg"
            />
            <span class="mobile-tab-label">{{ tab.label }}</span>
          </button>
        </nav>

        <!-- Main area -->
        <main class="app-main">
          <!-- Mode toggle toolbar — sits in the layout flow above ChatView.
               Hidden on non-chat tabs and when quest constellation is open. -->
          <div
            v-if="activeTab === 'chat' && !questConstellationOpen"
            class="mode-toggle-toolbar"
          >
            <div class="mode-segmented">
              <button
                :class="['mode-seg-btn', { active: !isChatboxMode }]"
                title="3D character mode"
                @click="setDisplayMode('desktop')"
              >
                🖥 <span class="mode-seg-label">3D</span>
              </button>
              <button
                :class="['mode-seg-btn', { active: isChatboxMode }]"
                title="Chat-only mode (no 3D character)"
                @click="setDisplayMode('chatbox')"
              >
                💬 <span class="mode-seg-label">Chat</span>
              </button>
              <button
                class="mode-seg-btn"
                title="Switch to pet mode"
                @click="togglePetMode"
              >
                🐾 <span class="mode-seg-label">Pet</span>
              </button>
            </div>
          </div>

          <ChatView
            v-show="activeTab === 'chat'"
            :chatbox-mode="isChatboxMode"
            @navigate="handleSkillNavigate"
          />
          <SkillTreeView
            v-if="activeTab === 'skills'"
            @navigate="handleSkillNavigate"
          />
          <BrainView
            v-if="activeTab === 'brain'"
            @navigate="handleSkillNavigate"
          />
          <MemoryView v-if="activeTab === 'memory'" />
          <MarketplaceView v-if="activeTab === 'marketplace'" />
          <VoiceSetupView
            v-if="activeTab === 'voice'"
            @done="activeTab = 'chat'"
          />
        </main>

        <!-- Floating quest progress bubble — chat tab only so it doesn't
             overlap Memory, Marketplace, Voice, or Skill-tree pages. -->
        <QuestBubble
          v-if="activeTab === 'chat'"
          @trigger="handleQuestBubble"
          @navigate="handleSkillNavigate"
          @update:constellation-open="questConstellationOpen = $event"
        />

        <!-- Combo unlock notifications (Chunk 131) -->
        <ComboToast />

        <!-- Quest reward ceremony overlay (Chunk 132) -->
        <QuestRewardCeremony />
      </template>
    </template>
  </div>
  </template>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';
import { useBrainStore } from './stores/brain';
import { useVoiceStore } from './stores/voice';
import { useWindowStore } from './stores/window';
import { useSkillTreeStore } from './stores/skill-tree';
import { usePersonaStore } from './stores/persona';
import { useSettingsStore } from './stores/settings';
import { useTheme } from './composables/useTheme';
import ChatView from './views/ChatView.vue';
import MemoryView from './views/MemoryView.vue';
import MarketplaceView from './views/MarketplaceView.vue';
import BrainView from './views/BrainView.vue';
import BrainSetupView from './views/BrainSetupView.vue';
import VoiceSetupView from './views/VoiceSetupView.vue';
import SkillTreeView from './views/SkillTreeView.vue';
import PetOverlayView from './views/PetOverlayView.vue';
import QuestBubble from './components/QuestBubble.vue';
import ComboToast from './components/ComboToast.vue';
import QuestRewardCeremony from './components/QuestRewardCeremony.vue';
import SplashScreen from './components/SplashScreen.vue';
import FirstLaunchWizard from './components/FirstLaunchWizard.vue';
import FloatingBadge from './components/ui/FloatingBadge.vue';

const brain = useBrainStore();
const voice = useVoiceStore();
const windowStore = useWindowStore();
const skillTree = useSkillTreeStore();
const persona = usePersonaStore();
const settingsStore = useSettingsStore();
const { themeId } = useTheme();
const activeTab = ref<'chat' | 'memory' | 'marketplace' | 'voice' | 'skills' | 'brain'>('chat');
const appLoading = ref(true);
const skipSetup = ref(false);
const tauriAvailable = ref(false);
const questConstellationOpen = ref(false);
const showFirstLaunchWizard = ref(false);


const hasBrain = computed(() => brain.hasBrain);
const isPetMode = computed(() => windowStore.mode === 'pet');
const isChatboxMode = computed(() => settingsStore.settings.chatbox_mode === true);
const appIconUrl = '/icon.png';

/** Panel-only window mode: when opened from pet mode context menu with ?panel=<id>. */
const panelParam = new URLSearchParams(window.location.search).get('panel');
const VALID_PANELS = ['brain', 'memory', 'skills', 'marketplace', 'voice'] as const;
type PanelId = typeof VALID_PANELS[number];
const panelOnly = VALID_PANELS.includes(panelParam as PanelId) ? (panelParam as PanelId) : null;

const tabs = [
  { id: 'chat' as const, label: 'Chat', svg: '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/></svg>' },
  { id: 'skills' as const, label: 'Quests', svg: '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/></svg>' },
  { id: 'brain' as const, label: 'Brain', svg: '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9.5 2A2.5 2.5 0 0 1 12 4.5v15a2.5 2.5 0 0 1-4.96.44 2.5 2.5 0 0 1-2.96-3.08 3 3 0 0 1-.34-5.58 2.5 2.5 0 0 1 1.32-4.24 2.5 2.5 0 0 1 1.98-3A2.5 2.5 0 0 1 9.5 2z"/><path d="M14.5 2a2.5 2.5 0 0 0-2.5 2.5v15a2.5 2.5 0 0 0 4.96.44 2.5 2.5 0 0 0 2.96-3.08 3 3 0 0 0 .34-5.58 2.5 2.5 0 0 0-1.32-4.24 2.5 2.5 0 0 0-1.98-3A2.5 2.5 0 0 0 14.5 2z"/></svg>' },
  { id: 'memory' as const, label: 'Memory', svg: '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2a7 7 0 0 1 7 7c0 2.38-1.19 4.47-3 5.74V17a1 1 0 0 1-1 1H9a1 1 0 0 1-1-1v-2.26C6.19 13.47 5 11.38 5 9a7 7 0 0 1 7-7z"/><line x1="9" y1="21" x2="15" y2="21"/></svg>' },
  { id: 'marketplace' as const, label: 'Market', svg: '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><polyline points="9 22 9 12 15 12 15 22"/></svg>' },
  { id: 'voice' as const, label: 'Voice', svg: '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/><line x1="12" y1="19" x2="12" y2="23"/><line x1="8" y1="23" x2="16" y2="23"/></svg>' },
];

async function onBrainDone() {
  skipSetup.value = true;
}

function onFirstLaunchDone() {
  showFirstLaunchWizard.value = false;
  // The wizard already configured brain + voice + quests.
  // If the manual path was chosen and brain is still not set,
  // show the BrainSetupView.
  if (!brain.hasBrain) {
    skipSetup.value = false;
  }
}

async function togglePetMode() {
  await windowStore.toggleMode();
}

async function setDisplayMode(mode: 'desktop' | 'chatbox') {
  await settingsStore.setChatboxMode(mode === 'chatbox');
}



function handleSkillNavigate(target: string) {
  const tabMap: Record<string, typeof activeTab.value> = {
    chat: 'chat',
    memory: 'memory',
    marketplace: 'marketplace',
    voice: 'voice',
    brain: 'brain',
    'brain-setup': 'brain', // brain setup wizard re-opens from the Brain hub
  };
  const tab = tabMap[target];
  if (tab) {
    activeTab.value = tab;
  }
  // 'brain-setup' target: re-open the brain wizard
  if (target === 'brain-setup') {
    skipSetup.value = false;
  }
}

function handleQuestBubble() {
  activeTab.value = 'chat';
}

function applyBodyBackground(mode: 'window' | 'pet') {
  if (typeof document === 'undefined') return;
  // In pet mode the body must be transparent so the desktop shows through.
  // In window mode, read the current theme's gradient token so every theme
  // gets its own thematic background without hardcoded hex values.
  if (mode === 'pet') {
    document.body.style.background = 'transparent';
  } else {
    const styles = getComputedStyle(document.documentElement);
    const gradient = styles.getPropertyValue('--ts-bg-gradient').trim();
    const base = styles.getPropertyValue('--ts-bg-base').trim();
    document.body.style.background = gradient || base || '#0f172a';
  }
}

// Watch for window mode changes (e.g. from tray icon toggle)
watch(
  () => windowStore.mode,
  (mode) => {
    applyBodyBackground(mode);
  },
  { immediate: true },
);

// Re-apply body background when the theme changes (so the base color updates).
watch(themeId, () => {
  // Give the DOM one tick so the CSS variables are already written.
  requestAnimationFrame(() => applyBodyBackground(windowStore.mode));
});

// Safety escape hatch: pressing Escape while in pet mode returns to desktop
// mode.  Guards against any scenario where the toggle pill might be
// unreachable (e.g. covered by another app on top).
function onKeyDown(e: KeyboardEvent) {
  if (e.key === 'Escape' && isPetMode.value) {
    windowStore.setMode('window');
  }
}

onMounted(async () => {
  // Panel-only windows (opened from pet mode) skip the full init sequence.
  // They share the same Pinia stores via the same origin, and the main
  // window already initialised the brain/voice/settings.
  if (panelOnly) {
    try {
      await brain.loadActiveBrain();
    } catch {
      brain.autoConfigureFreeApi();
    }
    try {
      await settingsStore.loadSettings();
    } catch { /* best-effort */ }
    await skillTree.initialise();
    return;
  }

  // Register the Escape-to-exit-pet-mode safety net first so the listener
  // is attached whether we take the Tauri path or the browser fallback.
  window.addEventListener('keydown', onKeyDown);

  try {
    await brain.loadActiveBrain();
    tauriAvailable.value = true;

    // Load current window mode from Rust backend
    await windowStore.loadMode();
    // Load dev/release build flag for DEV badge
    await windowStore.loadDevBuildFlag();
  } catch {
    // No Tauri backend available (dev server / E2E tests) — auto-configure free API.
    brain.autoConfigureFreeApi();
    skipSetup.value = true;
    // Also auto-configure voice so it works out of the box
    await voice.autoConfigureVoice();
    await skillTree.initialise();
    appLoading.value = false;
    return;
  }

  // Also try to load brain mode (three-tier config)
  try {
    await brain.loadBrainMode();
  } catch {
    // Ignore — will fall through to auto-configure
  }

  // Load voice config from backend
  try {
    await voice.initialise();
  } catch {
    // Voice unavailable — will auto-configure below
  }

  // Load persona traits + learned libraries (best-effort; falls back to
  // localStorage in browser-only contexts). Lets soul-mirror activate
  // without first opening the Brain tab.
  try {
    await persona.load();
  } catch {
    // Persona unavailable — keep default in-memory traits.
  }

  // Load settings to check first-launch flag
  await settingsStore.loadSettings();

  // If brain is already set (either legacy or new mode), skip the onboarding.
  if (brain.hasBrain) {
    skipSetup.value = true;
  } else if (settingsStore.settings.first_launch_complete) {
    // Settings say we completed first launch before but brain is gone —
    // re-configure silently (the user already chose their path).
    await brain.autoConfigureForDesktop();
    skipSetup.value = true;
  } else {
    // True first launch: show the wizard and let the user choose.
    showFirstLaunchWizard.value = true;
    skipSetup.value = true; // hide BrainSetupView while wizard is open
  }

  // If voice is not configured, auto-enable Web Speech API + Edge TTS
  // (skipped when the wizard is showing — the wizard handles this)
  if (!voice.hasVoice && !showFirstLaunchWizard.value) {
    await voice.autoConfigureVoice();
  }

  // Initialise skill tree (load quest tracker, refresh daily suggestions)
  await skillTree.initialise();

  // Listen for the tray-driven 'window-mode-changed' event so the frontend
  // state stays in sync when the user toggles via the system-tray menu.
  try {
    const { listen } = await import('@tauri-apps/api/event');
    await listen<string>('window-mode-changed', (e) => {
      const m = e.payload as 'window' | 'pet';
      if (m === 'window' || m === 'pet') {
        windowStore.$patch({ mode: m });
      }
    });
  } catch {
    // Not in Tauri — ignore
  }

  // (Escape-to-exit safety net is attached at the top of onMounted so it
  // works in the browser fallback too.)

  appLoading.value = false;
});

onUnmounted(() => {
  window.removeEventListener('keydown', onKeyDown);
});


</script>

<style>
*, *::before, *::after { box-sizing: border-box; }
body { margin: 0; background: var(--ts-bg-base, #0b1120); color: var(--ts-text-primary, #f0f2f8); font-family: var(--ts-font-family, system-ui, sans-serif); }
</style>

<style scoped>
/* Splash fade-out transition */
.splash-fade-leave-active { transition: opacity 0.5s ease; }
.splash-fade-leave-to { opacity: 0; }

.app-shell { display: flex; height: 100vh; height: 100dvh; overflow: hidden; }
.app-shell.pet-mode { background: transparent; }
.app-shell.panel-window { display: block; }
.panel-main { height: 100vh; height: 100dvh; overflow-y: auto; }

/* ── Desktop sidebar navigation ── */
.app-nav {
  display: flex; flex-direction: column; align-items: center; gap: 2px;
  padding: 12px 6px;
  background: var(--ts-bg-nav);
  border-right: 1px solid var(--ts-border);
  width: var(--ts-nav-width); flex-shrink: 0;
  position: relative;
}
.nav-logo {
  width: 36px; height: 36px;
  border-radius: var(--ts-radius-md);
  display: flex; align-items: center; justify-content: center;
  margin-bottom: 12px;
  flex-shrink: 0;
  overflow: hidden;
}
.nav-logo-img {
  width: 100%; height: 100%;
  object-fit: contain;
  border-radius: var(--ts-radius-md);
}
.nav-btn {
  width: 60px; height: 52px; border: none;
  border-radius: var(--ts-radius-md);
  background: transparent; cursor: pointer;
  display: flex; flex-direction: column; align-items: center; justify-content: center;
  gap: 3px;
  position: relative;
  transition: background var(--ts-transition-fast), transform var(--ts-transition-fast);
  color: var(--ts-text-muted);
}
.nav-icon {
  display: flex; align-items: center; justify-content: center;
  width: 22px; height: 22px;
  transition: color var(--ts-transition-fast);
}
.nav-icon :deep(svg) { width: 20px; height: 20px; }
.nav-label {
  font-size: 0.6rem;
  font-weight: 600;
  letter-spacing: 0.03em;
  line-height: 1;
  text-align: center;
}
.nav-btn:hover { background: var(--ts-bg-hover); color: var(--ts-text-secondary); }
.nav-btn.active {
  background: var(--ts-accent-glow);
  color: var(--ts-accent);
}
.nav-btn.active::before {
  content: '';
  position: absolute;
  left: -6px;
  top: 50%;
  transform: translateY(-50%);
  width: 3px;
  height: 24px;
  background: var(--ts-accent);
  border-radius: 0 3px 3px 0;
}
.nav-spacer { flex: 1; }
.nav-brain-warn { color: var(--ts-warning); }
/* ── Mode toggle toolbar (in-flow, inside .app-main) ── */
.mode-toggle-toolbar {
  position: absolute;
  top: var(--ts-space-sm);
  left: var(--ts-space-sm);
  z-index: var(--ts-z-sticky);
  pointer-events: auto;
}

/* ── 3-way segmented mode toggle ── */
.mode-segmented {
  display: inline-flex;
  border-radius: 20px;
  border: 1px solid var(--ts-accent-glow);
  background: var(--ts-bg-panel);
  backdrop-filter: blur(10px);
  box-shadow: var(--ts-shadow-md);
  overflow: hidden;
}
.mode-seg-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 6px 12px;
  border: none;
  background: transparent;
  color: var(--ts-text-dim);
  font-size: 0.74rem;
  font-weight: 600;
  letter-spacing: 0.03em;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
  white-space: nowrap;
}
.mode-seg-btn:hover {
  background: var(--ts-accent-glow);
  color: var(--ts-text-bright, var(--ts-text-primary));
}
.mode-seg-btn.active {
  background: var(--ts-accent);
  color: var(--ts-text-on-accent);
}
.mode-seg-btn + .mode-seg-btn {
  border-left: 1px solid var(--ts-border);
}
.mode-seg-label {
  display: inline;
}
.app-main { flex: 1; overflow: hidden; display: flex; flex-direction: column; min-width: 0; min-height: 0; position: relative; }

/* ── Mobile bottom tab bar ── */
.mobile-bottom-nav {
  display: none;
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  z-index: var(--ts-z-dropdown);
  height: var(--ts-mobile-nav-height);
  background: var(--ts-bg-panel);
  backdrop-filter: blur(20px);
  border-top: 1px solid var(--ts-border);
  padding: 0 4px;
  flex-direction: row;
  align-items: center;
  justify-content: space-around;
}
.mobile-tab {
  display: flex; flex-direction: column; align-items: center; justify-content: center;
  gap: 2px;
  border: none; background: transparent; cursor: pointer;
  color: var(--ts-text-muted);
  padding: 6px 8px;
  border-radius: var(--ts-radius-md);
  transition: color var(--ts-transition-fast), background var(--ts-transition-fast);
  min-width: 52px;
  position: relative;
}
.mobile-tab-icon {
  display: flex; align-items: center; justify-content: center;
  width: 22px; height: 22px;
}
.mobile-tab-icon :deep(svg) { width: 20px; height: 20px; }
.mobile-tab-label {
  font-size: 0.58rem;
  font-weight: 600;
  letter-spacing: 0.02em;
  line-height: 1;
}
.mobile-tab:hover { color: var(--ts-text-secondary); }
.mobile-tab.active {
  color: var(--ts-accent);
}
.mobile-tab.active::after {
  content: '';
  position: absolute;
  top: 0;
  left: 50%;
  transform: translateX(-50%);
  width: 20px;
  height: 2px;
  background: var(--ts-accent);
  border-radius: 0 0 2px 2px;
}

/* Mobile: hide sidebar, show bottom nav, add bottom padding for the tab bar */
@media (max-width: 640px) {
  .app-shell { flex-direction: column; }
  .desktop-nav { display: none; }
  .mobile-bottom-nav { display: flex; }
  .app-main { flex: 1; min-height: 0; padding-bottom: var(--ts-mobile-nav-height); }
}

/* ── DEV badge (layout-aware, no fixed positioning) ── */

/* Desktop sidebar: sits at the bottom of the flex column, after nav-spacer */
.nav-dev-badge {
  margin-top: 4px;
  margin-bottom: 4px;
  font-size: 0.6rem;
  flex-shrink: 0;
}

/* Mobile bottom bar: inline indicator as the first flex item */
.mobile-dev-indicator {
  display: none; /* hidden on desktop, shown via media query */
  align-items: center;
  justify-content: center;
  font-size: 0.55rem;
  font-weight: 700;
  letter-spacing: 0.05em;
  color: var(--ts-bg-base);
  background: var(--ts-warning);
  border-radius: 4px;
  padding: 2px 6px;
  line-height: 1;
  flex-shrink: 0;
  opacity: 0.8;
}
@media (max-width: 640px) {
  .nav-dev-badge { display: none; } /* sidebar hidden on mobile */
  .mobile-dev-indicator { display: flex; }
}

/* Pet mode: inline top-left inside the pet-mode-wrapper */
.pet-mode-wrapper {
  position: relative;
  width: 100%;
  height: 100%;
}
.pet-dev-badge {
  position: absolute;
  top: 4px;
  left: 4px;
  z-index: 10;
  font-size: 0.6rem;
  background: var(--ts-warning);
  backdrop-filter: blur(4px);
  border: 1px solid var(--ts-border);
}
</style>
