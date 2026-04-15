<template>
  <Analytics />
  <SpeedInsights />
  <div class="app-shell" :class="{ 'pet-mode': isPetMode }">
    <!-- Pet overlay mode: transparent character + floating chat -->
    <PetOverlayView v-if="isPetMode" />

    <!-- Normal mode: Brain onboarding or tabbed UI -->
    <template v-else>
      <!-- Brain onboarding: shown until a brain is configured -->
      <BrainSetupView v-if="!hasBrain && !skipSetup" @done="onBrainDone" />

      <template v-else>
        <!-- Desktop side navigation -->
        <nav class="app-nav desktop-nav">
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

        <!-- Mobile hamburger menu (top-left dropdown, collapsed by default) -->
        <div class="mobile-menu" ref="mobileMenuRef">
          <button
            class="mobile-menu-toggle"
            :class="{ open: mobileMenuOpen }"
            aria-label="Menu"
            @click.stop="mobileMenuOpen = !mobileMenuOpen"
          >
            <span class="hamburger-line" />
            <span class="hamburger-line" />
            <span class="hamburger-line" />
          </button>
          <Transition name="mobile-dropdown">
            <div v-if="mobileMenuOpen" class="mobile-menu-dropdown" @click.stop>
              <button
                v-for="tab in tabs"
                :key="tab.id"
                :class="['mobile-menu-item', { active: activeTab === tab.id }]"
                @click="activeTab = tab.id; mobileMenuOpen = false"
              >
                <span class="mobile-menu-icon">{{ tab.icon }}</span>
                <span class="mobile-menu-label">{{ tab.label }}</span>
              </button>

              <button
                v-if="!hasBrain"
                class="mobile-menu-item mobile-menu-warn"
                @click="skipSetup = false; mobileMenuOpen = false"
              >
                <span class="mobile-menu-icon">⚠</span>
                <span class="mobile-menu-label">Set up Brain</span>
              </button>

              <button
                v-if="tauriAvailable"
                class="mobile-menu-item mobile-menu-pet"
                @click="enterPetMode(); mobileMenuOpen = false"
              >
                <span class="mobile-menu-icon">🐾</span>
                <span class="mobile-menu-label">Pet Mode</span>
              </button>
            </div>
          </Transition>
        </div>

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
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';
import { useBrainStore } from './stores/brain';
import { useWindowStore } from './stores/window';
import ChatView from './views/ChatView.vue';
import MemoryView from './views/MemoryView.vue';
import MarketplaceView from './views/MarketplaceView.vue';
import BrainSetupView from './views/BrainSetupView.vue';
import VoiceSetupView from './views/VoiceSetupView.vue';
import PetOverlayView from './views/PetOverlayView.vue';
import { Analytics } from '@vercel/analytics/vue';
import { SpeedInsights } from '@vercel/speed-insights/vue';

const brain = useBrainStore();
const windowStore = useWindowStore();
const activeTab = ref<'chat' | 'memory' | 'marketplace' | 'voice'>('chat');
const skipSetup = ref(false);
const tauriAvailable = ref(false);
const mobileMenuOpen = ref(false);
const mobileMenuRef = ref<HTMLElement | null>(null);

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

function handleClickOutsideMenu(e: MouseEvent) {
  if (mobileMenuRef.value && e.target instanceof Node && !mobileMenuRef.value.contains(e.target)) {
    mobileMenuOpen.value = false;
  }
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

onMounted(async () => {
  document.addEventListener('click', handleClickOutsideMenu);

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

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutsideMenu);
});
</script>

<style>
*, *::before, *::after { box-sizing: border-box; }
body { margin: 0; background: var(--ts-bg-base, #0b1120); color: var(--ts-text-primary, #f0f2f8); font-family: var(--ts-font-family, system-ui, sans-serif); }
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

/* ── Mobile menu (hamburger dropdown) ── */
.mobile-menu {
  display: none;
  position: fixed;
  top: 10px;
  left: 10px;
  z-index: 50;
}

.mobile-menu-toggle {
  width: 36px;
  height: 36px;
  border: 1px solid rgba(255, 255, 255, 0.18);
  border-radius: var(--ts-radius-md);
  background: rgba(11, 17, 32, 0.82);
  backdrop-filter: blur(12px);
  cursor: pointer;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 4px;
  padding: 8px;
  transition: background var(--ts-transition-fast), transform var(--ts-transition-fast), box-shadow var(--ts-transition-fast);
  box-shadow: 0 2px 10px rgba(0, 0, 0, 0.35);
}
.mobile-menu-toggle:hover {
  background: rgba(124, 111, 255, 0.45);
  box-shadow: 0 4px 16px rgba(124, 111, 255, 0.25);
}
.mobile-menu-toggle.open {
  background: rgba(124, 111, 255, 0.6);
}
.hamburger-line {
  display: block;
  width: 16px;
  height: 2px;
  background: rgba(255, 255, 255, 0.85);
  border-radius: 1px;
  transition: transform 0.25s ease, opacity 0.25s ease;
}
.mobile-menu-toggle.open .hamburger-line:nth-child(1) {
  transform: translateY(6px) rotate(45deg);
}
.mobile-menu-toggle.open .hamburger-line:nth-child(2) {
  opacity: 0;
}
.mobile-menu-toggle.open .hamburger-line:nth-child(3) {
  transform: translateY(-6px) rotate(-45deg);
}

.mobile-menu-dropdown {
  position: absolute;
  top: 42px;
  left: 0;
  min-width: 180px;
  padding: 6px;
  border-radius: var(--ts-radius-lg);
  border: 1px solid rgba(255, 255, 255, 0.12);
  background: rgba(11, 17, 32, 0.95);
  backdrop-filter: blur(24px);
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.6);
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.mobile-menu-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 14px;
  border: none;
  border-radius: var(--ts-radius-md);
  background: transparent;
  color: var(--ts-text-secondary);
  font-size: 0.85rem;
  font-weight: 500;
  cursor: pointer;
  transition: background var(--ts-transition-fast), color var(--ts-transition-fast);
}
.mobile-menu-item:hover {
  background: var(--ts-bg-hover);
  color: var(--ts-text-primary);
}
.mobile-menu-item.active {
  background: rgba(124, 111, 255, 0.2);
  color: var(--ts-text-primary);
}
.mobile-menu-item.active .mobile-menu-icon {
  transform: scale(1.1);
}
.mobile-menu-icon {
  font-size: 1.1rem;
  flex-shrink: 0;
  transition: transform var(--ts-transition-fast);
}
.mobile-menu-label {
  letter-spacing: 0.02em;
}
.mobile-menu-warn { color: var(--ts-warning); }
.mobile-menu-pet { color: var(--ts-accent-violet); }

/* Mobile dropdown transition */
.mobile-dropdown-enter-active, .mobile-dropdown-leave-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
}
.mobile-dropdown-enter-from, .mobile-dropdown-leave-to {
  opacity: 0;
  transform: translateY(-8px) scale(0.95);
}

/* Mobile: hide sidebar, show hamburger menu */
@media (max-width: 640px) {
  .app-shell { flex-direction: column; }
  .desktop-nav { display: none; }
  .mobile-menu { display: block; }
  .app-main { flex: 1; min-height: 0; }
}
</style>
