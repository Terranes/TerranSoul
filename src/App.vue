<template>
  <div class="app-shell" data-tauri-drag-region>
    <!-- Custom drag handle (no window decorations) -->
    <div class="drag-bar" data-tauri-drag-region>
      <span class="drag-title">TerranSoul</span>
    </div>

    <!-- Always show main chat view with 3D model -->
    <main class="app-main">
      <ChatView />
    </main>
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue';
import { useBrainStore } from './stores/brain';
import ChatView from './views/ChatView.vue';

const brain = useBrainStore();

onMounted(async () => {
  try {
    await brain.loadActiveBrain();
  } catch {
    // No Tauri backend available (dev server / E2E tests)
  }
});
</script>

<style>
*, *::before, *::after { box-sizing: border-box; }
body { margin: 0; background: transparent; color: #f1f5f9; font-family: system-ui, sans-serif; }
</style>

<style scoped>
.app-shell { display: flex; flex-direction: column; height: 100vh; overflow: hidden; background: rgba(15, 23, 42, 0.85); border-radius: 12px; border: 1px solid rgba(255,255,255,0.08); }
.app-main { flex: 1; overflow: hidden; display: flex; flex-direction: column; min-width: 0; }
.drag-bar { position: fixed; top: 0; left: 0; right: 0; height: 32px; display: flex; align-items: center; padding-left: 12px; z-index: 1000; -webkit-app-region: drag; cursor: grab; background: transparent; }
.drag-title { font-size: 0.75rem; font-weight: 600; color: rgba(255,255,255,0.5); pointer-events: none; }
</style>
