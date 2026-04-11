<template>
  <div class="app-shell">
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
      </nav>

      <!-- Main area -->
      <main class="app-main">
        <ChatView v-show="activeTab === 'chat'" />
        <MemoryView v-if="activeTab === 'memory'" />
      </main>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useBrainStore } from './stores/brain';
import ChatView from './views/ChatView.vue';
import MemoryView from './views/MemoryView.vue';
import BrainSetupView from './views/BrainSetupView.vue';

const brain = useBrainStore();
const activeTab = ref<'chat' | 'memory'>('chat');
const skipSetup = ref(false);

const hasBrain = computed(() => brain.hasBrain);

const tabs = [
  { id: 'chat' as const, icon: '💬', label: 'Chat' },
  { id: 'memory' as const, icon: '🧠', label: 'Memory' },
];

async function onBrainDone() {
  skipSetup.value = true;
}

onMounted(async () => {
  try {
    await brain.loadActiveBrain();
  } catch {
    // No Tauri backend available (dev server / E2E tests) — skip the setup wizard.
    skipSetup.value = true;
    return;
  }
  // If brain is already set, skip the onboarding.
  if (brain.hasBrain) {
    skipSetup.value = true;
  }
});
</script>

<style>
*, *::before, *::after { box-sizing: border-box; }
body { margin: 0; background: #0f172a; color: #f1f5f9; font-family: system-ui, sans-serif; }
</style>

<style scoped>
.app-shell { display: flex; height: 100vh; overflow: hidden; }
.app-nav { display: flex; flex-direction: column; align-items: center; gap: 0.5rem; padding: 0.75rem 0.5rem; background: #0c1527; border-right: 1px solid #1e293b; width: 52px; }
.nav-btn { width: 38px; height: 38px; border: none; border-radius: 8px; background: transparent; cursor: pointer; font-size: 1.2rem; display: flex; align-items: center; justify-content: center; }
.nav-btn:hover, .nav-btn.active { background: #1e293b; }
.nav-brain-warn { color: #fbbf24; margin-top: auto; }
.app-main { flex: 1; overflow: hidden; display: flex; flex-direction: column; min-width: 0; }
</style>
