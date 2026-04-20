<template>
  <Analytics />
  <SpeedInsights />
  <!-- Loading splash shown during app initialization -->
  <Transition name="splash-fade">
    <SplashScreen v-if="appLoading" />
  </Transition>

  <div v-show="!appLoading" class="app-shell" :class="{ 'pet-mode': isPetMode }">
    <!-- Floating mode-toggle pill — visible in BOTH desktop and pet mode,
         and in browser/dev where the store falls back to a local flip so the
         overlay UI is still reachable.  This is the ONLY pet-mode toggle in
         the app; nothing lives in the sidebar/bottom-nav. -->
    <div
      v-if="!appLoading"
      class="mode-toggle-pill"
      :class="{ 'is-pet': isPetMode }"
    >
      <button
        class="mode-toggle-btn"
        role="switch"
        :aria-checked="isPetMode"
        :title="isPetMode ? 'Switch to desktop mode' : 'Switch to pet mode'"
        @click="togglePetMode"
      >
        <span class="mode-toggle-track">
          <span class="mode-toggle-thumb">{{ isPetMode ? '🐾' : '🖥' }}</span>
        </span>
        <span class="mode-toggle-label">{{ isPetMode ? 'Pet' : 'Desktop' }}</span>
      </button>
    </div>

    <!-- Pet overlay mode: transparent character + floating chat -->
    <PetOverlayView v-if="isPetMode" />

    <!-- Normal mode: Brain onboarding or tabbed UI -->
    <template v-else>
      <!-- Brain onboarding: shown until a brain is configured -->
      <BrainSetupView v-if="!hasBrain && !skipSetup" @done="onBrainDone" />

      <template v-else>
        <!-- Desktop side navigation -->
        <nav class="app-nav desktop-nav">
          <div class="nav-logo"><img src="/icon.png" alt="TerranSoul" class="nav-logo-img" /></div>
          <button
            v-for="tab in tabs"
            :key="tab.id"
            :class="['nav-btn', { active: activeTab === tab.id }]"
            @click="activeTab = tab.id"
          >
            <span class="nav-icon" v-html="tab.svg"></span>
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

        </nav>

        <!-- Mobile bottom tab bar (replaces hamburger menu) -->
        <nav class="mobile-bottom-nav">
          <button
            v-for="tab in tabs"
            :key="tab.id"
            :class="['mobile-tab', { active: activeTab === tab.id }]"
            @click="activeTab = tab.id"
          >
            <span class="mobile-tab-icon" v-html="tab.svg"></span>
            <span class="mobile-tab-label">{{ tab.label }}</span>
          </button>
        </nav>

        <!-- Main area -->
        <main class="app-main">
          <ChatView v-show="activeTab === 'chat'" @navigate="handleSkillNavigate" />
          <SkillTreeView v-if="activeTab === 'skills'" @navigate="handleSkillNavigate" />
          <MemoryView v-if="activeTab === 'memory'" />
          <MarketplaceView v-if="activeTab === 'marketplace'" />
          <VoiceSetupView v-if="activeTab === 'voice'" @done="activeTab = 'chat'" />
        </main>

        <!-- Floating quest progress bubble -->
        <QuestBubble @trigger="handleQuestBubble" @navigate="handleSkillNavigate" />
      </template>
    </template>

  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';
import { useBrainStore } from './stores/brain';
import { useVoiceStore } from './stores/voice';
import { useWindowStore } from './stores/window';
import { useSkillTreeStore } from './stores/skill-tree';
import ChatView from './views/ChatView.vue';
import MemoryView from './views/MemoryView.vue';
import MarketplaceView from './views/MarketplaceView.vue';
import BrainSetupView from './views/BrainSetupView.vue';
import VoiceSetupView from './views/VoiceSetupView.vue';
import SkillTreeView from './views/SkillTreeView.vue';
import PetOverlayView from './views/PetOverlayView.vue';
import QuestBubble from './components/QuestBubble.vue';
import SplashScreen from './components/SplashScreen.vue';
import { Analytics } from '@vercel/analytics/vue';
import { SpeedInsights } from '@vercel/speed-insights/vue';

const brain = useBrainStore();
const voice = useVoiceStore();
const windowStore = useWindowStore();
const skillTree = useSkillTreeStore();
const activeTab = ref<'chat' | 'memory' | 'marketplace' | 'voice' | 'skills'>('chat');
const appLoading = ref(true);
const skipSetup = ref(false);
const tauriAvailable = ref(false);


const hasBrain = computed(() => brain.hasBrain);
const isPetMode = computed(() => windowStore.mode === 'pet');

const tabs = [
  { id: 'chat' as const, label: 'Chat', svg: '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/></svg>' },
  { id: 'skills' as const, label: 'Quests', svg: '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/></svg>' },
  { id: 'memory' as const, label: 'Memory', svg: '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2a7 7 0 0 1 7 7c0 2.38-1.19 4.47-3 5.74V17a1 1 0 0 1-1 1H9a1 1 0 0 1-1-1v-2.26C6.19 13.47 5 11.38 5 9a7 7 0 0 1 7-7z"/><line x1="9" y1="21" x2="15" y2="21"/></svg>' },
  { id: 'marketplace' as const, label: 'Market', svg: '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><polyline points="9 22 9 12 15 12 15 22"/></svg>' },
  { id: 'voice' as const, label: 'Voice', svg: '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/><line x1="12" y1="19" x2="12" y2="23"/><line x1="8" y1="23" x2="16" y2="23"/></svg>' },
];

async function onBrainDone() {
  skipSetup.value = true;
}

async function togglePetMode() {
  await windowStore.toggleMode();
}



function handleSkillNavigate(target: string) {
  const tabMap: Record<string, typeof activeTab.value> = {
    chat: 'chat',
    memory: 'memory',
    marketplace: 'marketplace',
    voice: 'voice',
    'brain-setup': 'chat', // brain setup opens from chat context
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

// Watch for window mode changes (e.g. from tray icon toggle)
watch(
  () => windowStore.mode,
  (mode) => {
    if (mode === 'pet') {
      // Ensure transparent body background for pet mode
      document.body.style.background = 'transparent';
    } else {
      document.body.style.background = '#0b1120';
    }
  },
);

// Safety escape hatch: pressing Escape while in pet mode returns to desktop
// mode.  Guards against any scenario where the toggle pill might be
// unreachable (e.g. covered by another app on top).
function onKeyDown(e: KeyboardEvent) {
  if (e.key === 'Escape' && isPetMode.value) {
    windowStore.setMode('window');
  }
}

onMounted(async () => {
  // Register the Escape-to-exit-pet-mode safety net first so the listener
  // is attached whether we take the Tauri path or the browser fallback.
  window.addEventListener('keydown', onKeyDown);

  try {
    await brain.loadActiveBrain();
    tauriAvailable.value = true;

    // Load current window mode from Rust backend
    await windowStore.loadMode();
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

  // If brain is already set (either legacy or new mode), skip the onboarding.
  if (brain.hasBrain) {
    skipSetup.value = true;
  } else {
    // Desktop first launch: auto-configure free API (same as browser) and persist
    // to the Tauri backend so `send_message_stream` knows the brain mode.
    await brain.autoConfigureForDesktop();
    skipSetup.value = true;
  }

  // If voice is not configured, auto-enable Web Speech API + Edge TTS
  if (!voice.hasVoice) {
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

/* ── Desktop sidebar navigation ── */
.app-nav {
  display: flex; flex-direction: column; align-items: center; gap: 2px;
  padding: 12px 6px;
  background: var(--ts-bg-nav);
  border-right: 1px solid rgba(255, 255, 255, 0.08);
  width: 72px; flex-shrink: 0;
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
.nav-btn:hover { background: rgba(255, 255, 255, 0.08); color: var(--ts-text-secondary); }
.nav-btn.active {
  background: rgba(124, 111, 255, 0.15);
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
/* ── Floating Desktop/Pet mode-toggle pill ──
 * Always-visible, fixed-position toggle — lives outside the sidebar so it
 * remains reachable in pet mode (where the sidebar disappears).
 *
 * Position is mode-aware so the pill never covers the character's status:
 *  - Desktop mode: anchored just right of the sidebar at the top so it sits
 *    clear of the IDLE state pill and other chat-header chrome.
 *  - Pet mode: top-right corner (the only decoration there). */
.mode-toggle-pill {
  position: fixed;
  top: 12px;
  left: 82px;
  z-index: 1000;
  pointer-events: auto;
}
.mode-toggle-pill.is-pet {
  left: auto;
  right: 14px;
}
@media (max-width: 640px) {
  /* No sidebar on mobile; pin to top-left with a small gutter. */
  .mode-toggle-pill {
    left: 12px;
  }
}
.mode-toggle-btn {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px 6px 6px;
  border-radius: 20px;
  border: 1px solid rgba(108, 99, 255, 0.35);
  background: rgba(15, 23, 42, 0.82);
  color: #e2e8f0;
  cursor: pointer;
  font-size: 0.78rem;
  font-weight: 600;
  letter-spacing: 0.04em;
  backdrop-filter: blur(10px);
  box-shadow: 0 6px 18px rgba(0, 0, 0, 0.35);
  transition: background 0.15s, transform 0.15s, border-color 0.15s;
}
.mode-toggle-btn:hover {
  background: rgba(108, 99, 255, 0.25);
  transform: translateY(-1px);
}
.mode-toggle-pill.is-pet .mode-toggle-btn {
  border-color: rgba(108, 99, 255, 0.7);
  background: rgba(40, 30, 80, 0.8);
}
.mode-toggle-track {
  position: relative;
  width: 36px;
  height: 18px;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.1);
  border: 1px solid rgba(255, 255, 255, 0.12);
  display: inline-flex;
  flex-shrink: 0;
  transition: background 0.15s, border-color 0.15s;
}
.mode-toggle-pill.is-pet .mode-toggle-track {
  background: rgba(108, 99, 255, 0.45);
  border-color: rgba(108, 99, 255, 0.6);
}
.mode-toggle-thumb {
  position: absolute;
  top: 50%;
  left: 2px;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: #0b1120;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 0.55rem;
  line-height: 1;
  transform: translateY(-50%);
  transition: left 0.18s ease, background 0.15s;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.4);
}
.mode-toggle-pill.is-pet .mode-toggle-thumb {
  left: calc(100% - 16px);
  background: #fff;
}
.mode-toggle-label { white-space: nowrap; }
.app-main { flex: 1; overflow: hidden; display: flex; flex-direction: column; min-width: 0; min-height: 0; }

/* ── Mobile bottom tab bar ── */
.mobile-bottom-nav {
  display: none;
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  z-index: 50;
  height: 56px;
  background: rgba(9, 14, 28, 0.95);
  backdrop-filter: blur(20px);
  border-top: 1px solid rgba(255, 255, 255, 0.08);
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
  .app-main { flex: 1; min-height: 0; padding-bottom: 56px; }
}
</style>
