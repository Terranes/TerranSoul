<template>
  <section
    class="mcp-activity"
    :class="[
      `mcp-activity--${activity.snapshot.status}`,
      { 'mcp-activity--working': activity.isWorking, 'mcp-activity--collapsed': collapsed },
    ]"
    aria-live="polite"
    aria-label="MCP activity"
    data-testid="mcp-activity-panel"
  >
    <div class="mcp-activity__header">
      <span class="mcp-activity__dot" />
      <span
        v-if="!collapsed"
        class="mcp-activity__status"
      >{{ activity.statusLabel }}</span>
      <span class="mcp-activity__mode">MCP</span>
      <button
        class="mcp-activity__toggle"
        :aria-label="collapsed ? 'Expand MCP panel' : 'Collapse MCP panel'"
        :title="collapsed ? 'Expand' : 'Collapse'"
        data-testid="mcp-activity-toggle"
        @click.stop="collapsed = !collapsed"
      >
        {{ collapsed ? '▼' : '▲' }}
      </button>
    </div>
    <template v-if="!collapsed">
      <div class="mcp-activity__model">
        {{ activity.modelLabel }}
      </div>
      <div class="mcp-activity__message">
        {{ activity.speechText }}
      </div>
      <div
        v-if="activity.snapshot.toolName"
        class="mcp-activity__tool"
      >
        {{ activity.workLabel }}
      </div>
    </template>
  </section>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from 'vue';
import { storeToRefs } from 'pinia';
import { useAudioStore } from '../stores/audio';
import { useCharacterStore } from '../stores/character';
import { useMcpActivityStore } from '../stores/mcp-activity';
import { useTtsPlayback } from '../composables/useTtsPlayback';

const activity = useMcpActivityStore();
const character = useCharacterStore();
const audio = useAudioStore();
const { muted: audioMuted } = storeToRefs(audio);
const collapsed = ref(false);

const tts = useTtsPlayback({
  getBrowserPitch: () => character.currentBrowserPitch(),
  getBrowserRate: () => character.currentBrowserRate(),
  mutedRef: audioMuted,
});

let lastSpokenAt = 0;

onMounted(() => {
  void activity.initialise();
});

onUnmounted(() => {
  activity.dispose();
  tts.stop();
});

watch(
  () => activity.snapshot.updatedAtMs,
  () => {
    const snapshot = activity.snapshot;
    if (!snapshot.speak || snapshot.updatedAtMs === lastSpokenAt) return;
    const text = activity.speechText;
    if (!text) return;
    lastSpokenAt = snapshot.updatedAtMs;
    character.setState(snapshot.status === 'working' ? 'thinking' : 'talking');
    tts.stop();
    tts.feedChunk(text.endsWith('.') ? text : `${text}.`);
    tts.flush();
  },
);

watch(
  () => [activity.snapshot.status, tts.isSpeaking.value] as const,
  ([status, isSpeaking]) => {
    if (status === 'working') {
      character.setState('thinking');
      return;
    }
    if (!isSpeaking) {
      character.setState('idle');
    }
  },
);
</script>

<style scoped>
.mcp-activity {
  position: fixed;
  top: calc(var(--ts-space-md) + env(safe-area-inset-top));
  right: calc(var(--ts-space-md) + env(safe-area-inset-right));
  z-index: var(--ts-z-overlay);
  width: min(320px, calc(100vw - 2 * var(--ts-space-md)));
  padding: var(--ts-space-sm) var(--ts-space-md);
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-sm);
  background: color-mix(in srgb, var(--ts-bg-panel) 92%, transparent);
  box-shadow: var(--ts-shadow-md);
  backdrop-filter: blur(12px);
}

.mcp-activity__header {
  display: flex;
  align-items: center;
  gap: var(--ts-space-xs);
  min-width: 0;
}

.mcp-activity__dot {
  width: 0.55rem;
  height: 0.55rem;
  border-radius: 999px;
  background: var(--ts-info);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--ts-info) 20%, transparent);
  flex: 0 0 auto;
}

.mcp-activity--working .mcp-activity__dot {
  animation: mcp-pulse 1s ease-in-out infinite;
}

.mcp-activity--error .mcp-activity__dot {
  background: var(--ts-error);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--ts-error) 22%, transparent);
}

.mcp-activity--success .mcp-activity__dot {
  background: var(--ts-success);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--ts-success) 22%, transparent);
}

.mcp-activity__status,
.mcp-activity__mode {
  font-size: 0.72rem;
  font-weight: 700;
  line-height: 1.2;
  text-transform: uppercase;
  color: var(--ts-text-primary);
}

.mcp-activity__mode {
  margin-left: auto;
  color: var(--ts-info);
}

.mcp-activity__toggle {
  all: unset;
  cursor: pointer;
  margin-left: var(--ts-space-xs);
  font-size: 0.6rem;
  line-height: 1;
  color: var(--ts-text-muted);
  opacity: 0.7;
  transition: opacity 0.15s;
}

.mcp-activity__toggle:hover {
  opacity: 1;
}

.mcp-activity--collapsed {
  width: auto;
  min-width: 0;
}

.mcp-activity__model {
  margin-top: var(--ts-space-xs);
  font-size: 0.82rem;
  font-weight: 650;
  line-height: 1.25;
  color: var(--ts-text-primary);
  overflow-wrap: anywhere;
}

.mcp-activity__message {
  margin-top: 0.25rem;
  font-size: 0.78rem;
  line-height: 1.35;
  color: var(--ts-text-secondary);
  overflow-wrap: anywhere;
}

.mcp-activity__tool {
  margin-top: var(--ts-space-xs);
  font-size: 0.68rem;
  font-weight: 650;
  letter-spacing: 0;
  text-transform: uppercase;
  color: var(--ts-text-muted);
  overflow-wrap: anywhere;
}

@keyframes mcp-pulse {
  0%, 100% { opacity: 0.55; transform: scale(0.9); }
  50% { opacity: 1; transform: scale(1.12); }
}

@media (max-width: 640px) {
  .mcp-activity {
    top: auto;
    right: var(--ts-space-sm);
    bottom: calc(4.75rem + env(safe-area-inset-bottom));
    left: var(--ts-space-sm);
    width: auto;
  }
}
</style>