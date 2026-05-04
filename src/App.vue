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
      <div
        v-if="isPetMode"
        class="pet-mode-wrapper"
      >
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
              >
                <AppTabIcon :name="tab.id" />
              </span>
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
      </template>
    </div>
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
import BackgroundScene from './components/BackgroundScene.vue';
import AppTabIcon from './components/AppTabIcon.vue';

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
const skipSetup = ref(false);
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

  // (Escape-to-exit safety net is attached at the top of onMounted so it
  // works in the browser fallback too.)

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

<style scoped>
/* Splash fade-out transition */
.splash-fade-leave-active { transition: opacity 0.5s ease; }
.splash-fade-leave-to { opacity: 0; }

.app-shell { display: flex; height: 100vh; height: 100dvh; overflow: hidden; }
.app-shell.pet-mode { background: transparent; }
.app-shell.panel-window { display: block; }
.panel-main { height: 100vh; height: 100dvh; overflow-y: auto; }

.browser-app-window {
  position: fixed; inset: 0; margin: auto;
  z-index: var(--ts-z-modal, var(--ts-z-overlay));
  width: min(820px, calc(100vw - 2 * var(--ts-space-lg)));
  height: min(780px, calc(100dvh - 2 * var(--ts-space-lg)));
  display: flex; flex-direction: column; overflow: hidden;
  border: 1px solid var(--ts-border); border-radius: var(--ts-radius-xl);
  background: color-mix(in srgb, var(--ts-bg-panel) 96%, transparent);
  box-shadow: var(--ts-shadow-lg);
}
.browser-backdrop {
  position: fixed; inset: 0;
  z-index: calc(var(--ts-z-modal, var(--ts-z-overlay)) - 1);
  border: 0; padding: 0; margin: 0;
  background: color-mix(in srgb, #050313 62%, transparent);
  backdrop-filter: blur(8px); -webkit-backdrop-filter: blur(8px);
  cursor: pointer;
}
.browser-backdrop-enter-active, .browser-backdrop-leave-active { transition: opacity var(--ts-transition-normal); }
.browser-backdrop-enter-from, .browser-backdrop-leave-to { opacity: 0; }

@media (max-width: 720px) {
  .browser-app-window {
    inset:
      calc(var(--ts-space-md) + env(safe-area-inset-top)) var(--ts-space-sm)
      calc(var(--ts-space-md) + env(safe-area-inset-bottom)) var(--ts-space-sm);
    width: auto; height: auto; max-height: none;
    border-radius: var(--ts-radius-lg);
  }
  .browser-window-header { align-items: flex-start; flex-direction: column; }
  .browser-window-actions { width: 100%; }
  .browser-window-actions button { flex: 1; }
  .browser-window-actions .browser-window-close { flex: 0 0 auto; }
}

.browser-window-header {
  display: flex; align-items: center; justify-content: space-between;
  gap: var(--ts-space-md); padding: var(--ts-space-md);
  border-bottom: 1px solid var(--ts-border); background: var(--ts-bg-panel);
}
.browser-window-kicker, .browser-window-header h2 { margin: 0; }
.browser-window-kicker {
  color: var(--ts-accent); font-size: 0.68rem; font-weight: 900;
  letter-spacing: 0.14em; text-transform: uppercase;
}
.browser-window-header h2 { font-size: 1rem; }
.browser-window-actions { display: flex; gap: var(--ts-space-xs); }
.browser-window-actions button {
  border: 1px solid var(--ts-border); border-radius: var(--ts-radius-pill);
  padding: 0.48rem 0.7rem; color: var(--ts-text-secondary);
  background: var(--ts-bg-input); cursor: pointer; font-weight: 800;
}
.browser-window-actions button.active {
  color: var(--ts-text-on-accent); background: var(--ts-accent); border-color: transparent;
}
.browser-window-actions .browser-window-close {
  display: inline-flex; align-items: center; justify-content: center;
  width: 2.25rem; height: 2.25rem; padding: 0;
  margin-left: var(--ts-space-xs);
  font-size: 1.4rem; line-height: 1; font-weight: 400;
  border-radius: var(--ts-radius-full, 999px);
}
.browser-window-actions .browser-window-close:hover {
  color: var(--ts-text-primary);
  background: color-mix(in srgb, var(--ts-danger, #e34) 18%, var(--ts-bg-input));
  border-color: color-mix(in srgb, var(--ts-danger, #e34) 40%, var(--ts-border));
}
.visually-hidden {
  position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px;
  overflow: hidden; clip: rect(0, 0, 0, 0); white-space: nowrap; border: 0;
}
.browser-window-body { flex: 1; min-height: 0; overflow: hidden; }

.browser-window-enter-active,
.browser-window-leave-active {
  transition: opacity var(--ts-transition-normal), transform var(--ts-transition-normal);
}

.browser-window-enter-from,
.browser-window-leave-to {
  opacity: 0;
  transform: translateY(14px) scale(0.98);
}

/* ── Desktop sidebar navigation ── */
.app-nav {
  display: flex; flex-direction: column; align-items: center; gap: 2px;
  padding: 12px 6px;
  background: color-mix(in srgb, var(--ts-bg-nav) 82%, transparent);
  border-right: 1px solid var(--ts-border);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
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
  height: var(--ts-mobile-nav-total-height);
  background: color-mix(in srgb, var(--ts-bg-panel) 82%, transparent);
  backdrop-filter: blur(24px);
  -webkit-backdrop-filter: blur(24px);
  border-top: 1px solid var(--ts-border);
  padding: 0 max(4px, var(--ts-safe-area-right)) var(--ts-safe-area-bottom) max(4px, var(--ts-safe-area-left));
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
  .app-main { flex: 1; min-height: 0; padding-bottom: var(--ts-mobile-nav-total-height); }
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
