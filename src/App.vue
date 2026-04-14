<template>
  <div class="app-shell" :class="{ 'pet-mode': isPetMode }">
    <!-- Pet overlay mode: transparent character + floating chat -->
    <PetOverlayView v-if="isPetMode" />

    <!-- Normal mode: Brain onboarding or tabbed UI -->
    <template v-else>
      <!-- Brain onboarding: shown until a brain is configured -->
      <BrainSetupView v-if="!hasBrain && !skipSetup" @done="onBrainDone" />

      <template v-else>
        <!-- Side navigation -->
        <nav class="app-nav">
          <button
            v-for="tab in tabs"
            :key="tab.id"
            :class="['nav-btn', { active: activeTab === tab.id }]"
            :title="tab.label"
            @click="activeTab = tab.id"
          >{{ tab.icon }}</button>

          <!-- "No brain" warning pill -->
          <button
            v-if="!hasBrain"
            class="nav-btn nav-brain-warn"
            title="No brain — click to set up"
            @click="skipSetup = false"
          >⚠</button>

          <!-- Pet mode toggle (only when Tauri is available) -->
          <button
            v-if="tauriAvailable"
            class="nav-btn nav-pet-toggle"
            title="Switch to pet mode"
            @click="enterPetMode"
          >🐾</button>
        </nav>

        <!-- Main area -->
        <main class="app-main">
          <ChatView v-show="activeTab === 'chat'" />
          <MemoryView v-if="activeTab === 'memory'" />
          <MarketplaceView v-if="activeTab === 'marketplace'" />
          <VoiceSetupView v-if="activeTab === 'voice'" @done="activeTab = 'chat'" />
        </main>
      </template>
    </template>

  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue';
import { useBrainStore } from './stores/brain';
import { useWindowStore } from './stores/window';
import ChatView from './views/ChatView.vue';
import MemoryView from './views/MemoryView.vue';
import MarketplaceView from './views/MarketplaceView.vue';
import BrainSetupView from './views/BrainSetupView.vue';
import VoiceSetupView from './views/VoiceSetupView.vue';
import PetOverlayView from './views/PetOverlayView.vue';
import speedInsights from '@vercel/speed-insights';

const brain = useBrainStore();
const windowStore = useWindowStore();
const activeTab = ref<'chat' | 'memory' | 'marketplace' | 'voice'>('chat');
const skipSetup = ref(false);
const tauriAvailable = ref(false);

const hasBrain = computed(() => brain.hasBrain);
const isPetMode = computed(() => windowStore.mode === 'pet');

const tabs = [
  { id: 'chat' as const, icon: '💬', label: 'Chat' },
  { id: 'memory' as const, icon: '🧠', label: 'Memory' },
  { id: 'marketplace' as const, icon: '🏪', label: 'Marketplace' },
  { id: 'voice' as const, icon: '🎤', label: 'Voice' },
];

async function onBrainDone() {
  skipSetup.value = true;
}

async function enterPetMode() {
  await windowStore.setMode('pet');
}

// Watch for window mode changes (e.g. from tray icon toggle)
watch(
  () => windowStore.mode,
  (mode) => {
    if (mode === 'pet') {
      // Ensure transparent body background for pet mode
      document.body.style.background = 'transparent';
    } else {
      document.body.style.background = '#0f172a';
    }
  },
);

onMounted(async () => {
  // Initialize Vercel Speed Insights
  speedInsights.injectSpeedInsights();

  try {
    await brain.loadActiveBrain();
    tauriAvailable.value = true;

    // Load current window mode from Rust backend
    await windowStore.loadMode();
  } catch {
    // No Tauri backend available (dev server / E2E tests) — auto-configure free API.
    brain.autoConfigureFreeApi();
    skipSetup.value = true;
    return;
  }

  // Also try to load brain mode (three-tier config)
  try {
    await brain.loadBrainMode();
  } catch {
    // Ignore — will fall through to setup
  }

  // If brain is already set (either legacy or new mode), skip the onboarding.
  if (brain.hasBrain) {
    skipSetup.value = true;
  }
});
</script>

<style>
*, *::before, *::after { box-sizing: border-box; }
body { margin: 0; background: var(--ts-bg-base, #0f172a); color: var(--ts-text-primary, #f1f5f9); font-family: var(--ts-font-family, system-ui, sans-serif); }
</style>

<style scoped>
.app-shell { display: flex; height: 100vh; height: 100dvh; overflow: hidden; }
.app-shell.pet-mode { background: transparent; }
.app-nav {
  display: flex; flex-direction: column; align-items: center; gap: 0.5rem;
  padding: 0.75rem 0.5rem;
  background: var(--ts-bg-nav);
  border-right: 1px solid var(--ts-bg-surface);
  width: 52px; flex-shrink: 0;
  position: relative;
}
.nav-btn {
  width: 38px; height: 38px; border: none;
  border-radius: var(--ts-radius-md);
  background: transparent; cursor: pointer; font-size: 1.2rem;
  display: flex; align-items: center; justify-content: center;
  position: relative;
  transition: background var(--ts-transition-fast), transform var(--ts-transition-fast);
}
.nav-btn:hover { background: var(--ts-bg-surface); transform: scale(1.06); }
.nav-btn.active { background: var(--ts-bg-surface); }
.nav-btn.active::after {
  content: '';
  position: absolute;
  left: -0.5rem;
  top: 50%;
  transform: translateY(-50%);
  width: 3px;
  height: 20px;
  background: var(--ts-accent);
  border-radius: 0 var(--ts-radius-sm) var(--ts-radius-sm) 0;
}
.nav-brain-warn { color: var(--ts-warning); margin-top: auto; }
.nav-pet-toggle { color: var(--ts-accent-violet); margin-top: auto; }
.nav-pet-toggle:hover { background: rgba(139, 92, 246, 0.2); }
.app-main { flex: 1; overflow: hidden; display: flex; flex-direction: column; min-width: 0; }

/* Tooltip on hover for nav buttons */
.nav-btn::before {
  content: attr(title);
  position: absolute;
  left: calc(100% + 8px);
  top: 50%;
  transform: translateY(-50%);
  padding: 4px 10px;
  border-radius: var(--ts-radius-sm);
  background: var(--ts-bg-surface);
  color: var(--ts-text-primary);
  font-size: var(--ts-text-xs);
  white-space: nowrap;
  pointer-events: none;
  opacity: 0;
  transition: opacity var(--ts-transition-fast);
  z-index: 100;
  box-shadow: var(--ts-shadow-sm);
}
.nav-btn:hover::before {
  opacity: 1;
}

/* Mobile: collapse sidebar to horizontal bottom bar */
@media (max-width: 640px) {
  .app-shell { flex-direction: column; }
  .app-nav { flex-direction: row; width: 100%; order: 1; padding: 0.35rem 0.5rem; border-right: none; border-top: 1px solid var(--ts-bg-surface); justify-content: center; }
  .app-main { order: 0; }
  .nav-btn.active::after {
    left: 50%; top: auto; bottom: -0.35rem;
    transform: translateX(-50%);
    width: 20px; height: 3px;
    border-radius: var(--ts-radius-sm) var(--ts-radius-sm) 0 0;
  }
  .nav-btn::before { display: none; }
}
</style>
