<template>
  <template v-if="browserMode">
    <BrowserLandingView @open-app-window="openBrowserAppWindow('desktop')" />
    <Transition name="browser-window">
      <section
        v-if="browserAppWindowOpen"
        ref="browserAppWindowRef"
        class="browser-app-window"
        role="dialog"
        aria-modal="false"
        aria-label="TerranSoul browser app window"
        tabindex="-1"
        @keydown.escape.stop="closeBrowserAppWindow"
      >
        <header class="browser-window-header">
          <div>
            <p class="browser-window-kicker">
              Browser mode
            </p>
            <h2>TerranSoul live app</h2>
          </div>
          <div
            class="browser-window-actions"
            role="toolbar"
            aria-label="Browser app display mode"
          >
            <button
              type="button"
              :class="{ active: browserDisplayMode === 'desktop' }"
              :aria-pressed="browserDisplayMode === 'desktop'"
              aria-label="Switch browser app window to 3D view"
              @click="setBrowserDisplayMode('desktop')"
            >
              3D
            </button>
            <button
              type="button"
              :class="{ active: browserDisplayMode === 'chatbox' }"
              :aria-pressed="browserDisplayMode === 'chatbox'"
              aria-label="Switch browser app window to chat view"
              @click="setBrowserDisplayMode('chatbox')"
            >
              Chat
            </button>
            <button
              type="button"
              class="browser-window-close"
              aria-label="Close app window and return to pet preview"
              title="Close (Esc)"
              @click="closeBrowserAppWindow"
            >
              <span aria-hidden="true">×</span>
              <span class="visually-hidden">Close</span>
            </button>
          </div>
        </header>
        <div class="browser-window-body">
          <ChatView
            :chatbox-mode="browserDisplayMode === 'chatbox'"
            @navigate="handleSkillNavigate"
          />
        </div>
      </section>
    </Transition>
    <!-- Click-outside backdrop so the modal is always dismissible even if
         the close button is obscured. Rendered behind the dialog. -->
    <Transition name="browser-backdrop">
      <button
        v-if="browserAppWindowOpen"
        type="button"
        class="browser-backdrop"
        aria-label="Close app window"
        @click="closeBrowserAppWindow"
      />
    </Transition>
  </template>

  <!-- Panel-only window: opened from pet mode context menu -->
  <div
    v-else-if="panelOnly"
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
      <MobilePairingView v-if="panelOnly === 'mobile'" />
      <VoiceSetupView
        v-if="panelOnly === 'voice'"
        @done="() => {}"
      />
    </main>
  </div>

  <template v-else>
    <!-- Animated 3D background scene — themed via html[data-theme]; auto-hidden in pet mode -->
    <BackgroundScene v-if="!isPetMode" />

    <!-- Loading splash shown during app initialization -->
    <Transition name="splash-fade">
      <SplashScreen v-if="appLoading && !(isPetMode && !windowStore.isMcpMode)" />
    </Transition>

    <!-- First-launch wizard (shown once, before the main app) -->
    <FirstLaunchWizard
      :visible="showFirstLaunchWizard"
      @done="onFirstLaunchDone"
    />

    <div
      v-show="!appLoading"
      class="app-shell"
      :class="{ 'pet-mode': isPetMode && !windowStore.isMcpMode }"
    >
      <!-- Pet overlay mode: transparent character + floating chat
           MCP mode overrides pet mode to show full tabbed UI -->
      <div
        v-if="isPetMode && !windowStore.isMcpMode"
        class="pet-mode-wrapper"
      >
        <PetOverlayView />
      </div>

      <!-- Normal mode (or MCP mode): tabbed UI -->
      <template v-else>
        <!-- Desktop side navigation -->
        <nav class="app-nav desktop-nav">
          <button
            type="button"
            class="nav-logo nav-logo-btn"
            :class="{ active: activeTab === 'chat' }"
            aria-label="Go to Chat"
            title="Chat"
            @click="activeTab = 'chat'"
          >
            <img
              :src="appIconUrl"
              alt="TerranSoul"
              class="nav-logo-img"
            >
          </button>
          <button
            v-for="tab in tabs"
            :key="tab.id"
            :class="['nav-btn', { active: activeTab === tab.id }]"
            @click="activeTab = tab.id"
          >
            <span
              class="nav-icon"
            >
              <AppTabIcon :name="tab.id" />
            </span>
            <span class="nav-label">{{ tab.label }}</span>
          </button>

          <div class="nav-spacer" />

          <!-- "No brain" warning pill -->
          <button
            v-if="!hasBrain"
            class="nav-btn nav-brain-warn"
            @click="activeTab = 'brain'"
          >
            <span class="nav-icon">⚠</span>
            <span class="nav-label">Brain</span>
          </button>

          <!-- Build-mode badge — inline in the sidebar, below spacer.
                 MCP mode (npm run mcp) takes priority over DEV. -->
          <FloatingBadge
            v-if="windowStore.isMcpMode"
            class="nav-mcp-badge"
            tone="info"
            readonly
            title="MCP mode — brain available on port 7423 (data: <repo>/mcp-data/)"
          >
            MCP
          </FloatingBadge>
          <FloatingBadge
            v-else-if="windowStore.isDevBuild"
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
          <!-- Build-mode indicator — first item in the tab row.
                 MCP mode takes priority over DEV. -->
          <span
            v-if="windowStore.isMcpMode"
            class="mobile-mcp-indicator"
            title="MCP mode"
          >MCP</span>
          <span
            v-else-if="windowStore.isDevBuild"
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
            >
              <AppTabIcon :name="tab.id" />
            </span>
            <span class="mobile-tab-label">{{ tab.label }}</span>
          </button>
        </nav>

        <!-- Main area -->
        <main
          class="app-main"
          :class="{ 'app-main--chat-viewport': activeTab === 'chat' && !isChatboxMode }"
        >
          <ChatView
            v-show="activeTab === 'chat'"
            :chatbox-mode="isChatboxMode"
            @navigate="handleSkillNavigate"
            @set-display-mode="setDisplayMode"
            @toggle-pet-mode="togglePetMode"
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
          <MobilePairingView v-if="activeTab === 'mobile'" />
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
    </div>

    <McpActivityPanel v-if="windowStore.isMcpMode && !appLoading" />
  </template>
</template>

<script setup lang="ts">
import { ref, computed, nextTick, onMounted, onUnmounted, watch } from 'vue';
import { useBrainStore } from './stores/brain';
import { useVoiceStore } from './stores/voice';
import { useWindowStore } from './stores/window';
import { useSkillTreeStore } from './stores/skill-tree';
import { usePersonaStore } from './stores/persona';
import { useSettingsStore } from './stores/settings';
import { useMobileNotificationsStore } from './stores/mobile-notifications';
import { useTheme } from './composables/useTheme';
import ChatView from './views/ChatView.vue';
import BrowserLandingView from './views/BrowserLandingView.vue';
import MemoryView from './views/MemoryView.vue';
import MarketplaceView from './views/MarketplaceView.vue';
import MobilePairingView from './views/MobilePairingView.vue';
import BrainView from './views/BrainView.vue';
// BrainSetupView removed — FirstLaunchWizard handles initial brain config
import VoiceSetupView from './views/VoiceSetupView.vue';
import SkillTreeView from './views/SkillTreeView.vue';
import PetOverlayView from './views/PetOverlayView.vue';
import QuestBubble from './components/QuestBubble.vue';
import ComboToast from './components/ComboToast.vue';
import QuestRewardCeremony from './components/QuestRewardCeremony.vue';
import SplashScreen from './components/SplashScreen.vue';
import FirstLaunchWizard from './components/FirstLaunchWizard.vue';
import FloatingBadge from './components/ui/FloatingBadge.vue';
import BackgroundScene from './components/BackgroundScene.vue';
import AppTabIcon from './components/AppTabIcon.vue';
import McpActivityPanel from './components/McpActivityPanel.vue';

const brain = useBrainStore();
const voice = useVoiceStore();
const windowStore = useWindowStore();
const skillTree = useSkillTreeStore();
const persona = usePersonaStore();
const settingsStore = useSettingsStore();
const mobileNotifications = useMobileNotificationsStore();
useTheme(); // applies saved theme to html[data-theme] at startup
const activeTab = ref<'chat' | 'memory' | 'marketplace' | 'voice' | 'skills' | 'brain' | 'mobile'>('chat');
const appLoading = ref(true);
const tauriAvailable = ref(false);
const browserMode = ref(false);
const browserAppWindowOpen = ref(false);
const browserAppWindowRef = ref<HTMLElement | null>(null);
const browserDisplayMode = ref<'desktop' | 'chatbox'>('desktop');
const questConstellationOpen = ref(false);
const showFirstLaunchWizard = ref(false);


const hasBrain = computed(() => brain.hasBrain);
const isPetMode = computed(() => windowStore.mode === 'pet');
const isChatboxMode = computed(() => settingsStore.settings.chatbox_mode === true);
const appIconUrl = '/icon.png';

/** Panel-only window mode: when opened from pet mode context menu with ?panel=<id>. */
const panelParam = new URLSearchParams(window.location.search).get('panel');
const VALID_PANELS = ['brain', 'memory', 'skills', 'marketplace', 'voice', 'mobile'] as const;
type PanelId = typeof VALID_PANELS[number];
const panelOnly = VALID_PANELS.includes(panelParam as PanelId) ? (panelParam as PanelId) : null;

const tabs = [
  { id: 'chat' as const, label: 'Chat' },
  { id: 'skills' as const, label: 'Quests' },
  { id: 'brain' as const, label: 'Brain' },
  { id: 'memory' as const, label: 'Memory' },
  { id: 'marketplace' as const, label: 'Market' },
  { id: 'mobile' as const, label: 'Link' },
  { id: 'voice' as const, label: 'Voice' },
];

function onFirstLaunchDone() {
  showFirstLaunchWizard.value = false;
}

async function togglePetMode() {
  await windowStore.toggleMode();
}

async function setDisplayMode(mode: 'desktop' | 'chatbox') {
  await settingsStore.setChatboxMode(mode === 'chatbox');
}

function openBrowserAppWindow(mode: 'desktop' | 'chatbox') {
  browserDisplayMode.value = mode;
  browserAppWindowOpen.value = true;
}

function closeBrowserAppWindow() { browserAppWindowOpen.value = false; }

function setBrowserDisplayMode(mode: 'desktop' | 'chatbox') {
  browserDisplayMode.value = mode;
  void focusBrowserAppWindow();
}

async function focusBrowserAppWindow() {
  await nextTick();
  browserAppWindowRef.value?.focus();
}
function handleSkillNavigate(target: string) {
  if (target === 'pet-mode') { if (browserMode.value) browserAppWindowOpen.value = false; else void windowStore.setMode('pet'); return; }
  const tabMap: Record<string, typeof activeTab.value> = {
    chat: 'chat',
    memory: 'memory',
    marketplace: 'marketplace',
    voice: 'voice',
    brain: 'brain',
    mobile: 'mobile',
    'brain-setup': 'brain',
  };
  const tab = tabMap[target];
  if (tab) {
    activeTab.value = tab;
  }
}

function handleQuestBubble() {
  activeTab.value = 'chat';
}

function applyBodyBackground(mode: 'window' | 'pet') {
  if (typeof document === 'undefined') return;
  // Pet mode: body must be transparent so the desktop shows through the
  // Tauri transparent window. Window mode: clear the inline style so the
  // CSS-driven var(--ts-bg-gradient) on body takes over automatically.
  // data-ts-mode is read by CSS to suppress the animated aura orbs in pet mode.
  document.body.style.background = mode === 'pet' ? 'transparent' : '';
  document.body.dataset.tsMode = mode;
}

// Watch for window mode changes (e.g. from tray icon toggle)
watch(
  () => windowStore.mode,
  (mode) => {
    applyBodyBackground(mode);
  },
  { immediate: true },
);

// Safety escape hatch: pressing Escape while in pet mode returns to desktop
// mode.  Guards against any scenario where the toggle pill might be
// unreachable (e.g. covered by another app on top).
function onKeyDown(e: KeyboardEvent) {
  if (e.key === 'Escape' && browserMode.value && browserAppWindowOpen.value) {
    browserAppWindowOpen.value = false;
    return;
  }
  if (e.key === 'Escape' && isPetMode.value) {
    windowStore.setMode('window');
  }
}

watch(browserAppWindowOpen, (open) => {
  if (open) void focusBrowserAppWindow();
});

onMounted(async () => {
  // Panel-only windows (opened from pet mode) skip the full init sequence.
  // They share the same Pinia stores via the same origin, and the main
  // window already initialised the brain/voice/settings.
  if (panelOnly) {
    try {
      await brain.loadActiveBrain();
    } catch {
      brain.prepareBrowserProviderChoices();
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
    // Load MCP-mode flag (replaces DEV badge with MCP when running
    // as `npm run mcp` / `--mcp-app`).
    await windowStore.loadMcpModeFlag();
  } catch {
    // No Tauri backend available (dev server / E2E tests) — prepare browser-safe provider choices.
    // Only activate the browser landing page when NOT running under Playwright E2E,
    // so the test suite continues to see the normal app shell and chat view.
    if (!import.meta.env.VITE_E2E) {
      browserMode.value = true;
      // Opt the landing page out of the global overflow:hidden lock.
      document.documentElement.dataset.tsMode = document.body.dataset.tsMode = 'browser';
    }
    brain.prepareBrowserProviderChoices();
    if (import.meta.env.VITE_E2E) {
      brain.autoConfigureFreeApi();
    }
    // Simulate MCP mode flag in browser/E2E when VITE_MCP_MODE is set
    await windowStore.loadMcpModeFlag();
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

  // If the user is on Local Ollama, verify the saved model is actually
  // installed. A stale config (e.g. gemma4:31b after a catalogue fix that
  // now picks gemma3:4b) causes warmup to fail and the first message to
  // hit a cold-start. When the model is missing, re-pick the best fit.
  if (brain.brainMode?.mode === 'local_ollama') {
    try {
      await brain.checkOllamaStatus();
      if (brain.ollamaStatus.running) {
        await brain.fetchInstalledModels();
        const savedModel = brain.brainMode.model;
        const isInstalled = brain.installedModels.some(
          (m: { name: string }) => m.name === savedModel,
        );
        if (!isInstalled && brain.installedModels.length > 0) {
          // Saved model is gone — upgrade to best available
          await brain.maybeUpgradeToLocalOllama();
        }
      }
    } catch {
      // Best-effort — warmup below still fires with whatever model is saved
    }
  }

  // If the user is on Local Ollama, pre-warm the chat model into VRAM
  // immediately on launch so the first user reply is fast (<1s) instead
  // of paying a 10–20s cold-load on consumer GPUs. Fire-and-forget.
  if (brain.brainMode?.mode === 'local_ollama') {
    void brain.warmupLocalOllama(brain.brainMode.model);
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

  // If brain is already set, proceed normally.
  if (!brain.hasBrain) {
    if (settingsStore.settings.first_launch_complete) {
      // Settings say we completed first launch before but brain is gone —
      // re-configure silently (the user already chose their path).
      await brain.autoConfigureForDesktop();
    } else {
      // True first launch: show the wizard and let the user choose.
      // Dismiss the splash immediately — the wizard has its own UI.
      showFirstLaunchWizard.value = true;
      appLoading.value = false;
    }
  }

  // If voice is not configured, auto-enable Web Speech API + Edge TTS
  // (skipped when the wizard is showing — the wizard handles this)
  if (!voice.hasVoice && !showFirstLaunchWizard.value) {
    await voice.autoConfigureVoice();
  }

  // Initialise skill tree (load quest tracker, refresh daily suggestions)
  await skillTree.initialise();

  await mobileNotifications.start();

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

  appLoading.value = false;
});

onUnmounted(() => {
  window.removeEventListener('keydown', onKeyDown);
  mobileNotifications.stop();
});


</script>

<style>
*, *::before, *::after { box-sizing: border-box; }
body { margin: 0; color: var(--ts-text-primary, #f0f2f8); font-family: var(--ts-font-family, system-ui, sans-serif); }
</style>

<style src="./App.css" scoped />
