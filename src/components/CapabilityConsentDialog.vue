<template>
  <div
    class="consent-backdrop"
    @click.self="$emit('cancel')"
  >
    <div class="consent-dialog">
      <h3>🔒 Capability Consent</h3>
      <p class="consent-desc">
        <strong>{{ agentName }}</strong> requests the following capabilities:
      </p>

      <ul class="consent-list">
        <li
          v-for="cap in capabilities"
          :key="cap"
          :class="['consent-item', { sensitive: isSensitive(cap) }]"
        >
          <span class="consent-icon">{{ capabilityIcon(cap) }}</span>
          <span class="consent-label">{{ capabilityLabel(cap) }}</span>
          <span
            v-if="isSensitive(cap)"
            class="consent-warn"
          >⚠ sensitive</span>
        </li>
      </ul>

      <p
        v-if="hasSensitive"
        class="consent-warning"
      >
        This agent requests sensitive capabilities that could access your files, clipboard,
        network, or spawn processes. Only grant these if you trust the agent.
      </p>

      <div class="consent-btns">
        <button
          class="btn-primary"
          @click="$emit('confirm')"
        >
          Grant &amp; Install
        </button>
        <button
          class="btn-secondary"
          @click="$emit('cancel')"
        >
          Cancel
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';

const props = defineProps<{
  agentName: string;
  capabilities: string[];
  sensitiveCapabilities: string[];
}>();

defineEmits<{
  confirm: [];
  cancel: [];
}>();

const hasSensitive = computed(() => props.sensitiveCapabilities.length > 0);

function isSensitive(cap: string): boolean {
  return props.sensitiveCapabilities.includes(cap);
}

function capabilityIcon(cap: string): string {
  const icons: Record<string, string> = {
    chat: '💬',
    filesystem: '📁',
    network: '🌐',
    clipboard: '📋',
    process_spawn: '⚙',
  };
  return icons[cap] ?? '🔧';
}

function capabilityLabel(cap: string): string {
  const labels: Record<string, string> = {
    chat: 'Chat — send and receive messages',
    filesystem: 'File System — read/write files',
    network: 'Network — make HTTP requests',
    clipboard: 'Clipboard — read/write clipboard',
    process_spawn: 'Process — spawn system processes',
  };
  return labels[cap] ?? cap;
}
</script>

<style scoped>
.consent-backdrop { position: fixed; inset: 0; background: rgba(0,0,0,0.6); display: flex; align-items: center; justify-content: center; z-index: 100; backdrop-filter: blur(4px); }
.consent-dialog { background: var(--ts-bg-surface); border: 1px solid var(--ts-border); border-radius: 12px; padding: 1.5rem; width: min(480px, 90vw); display: flex; flex-direction: column; gap: 0.75rem; box-shadow: var(--ts-shadow-lg); }
.consent-desc { margin: 0; color: var(--ts-text-secondary); font-size: 0.9rem; }
.consent-list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 0.4rem; }
.consent-item { display: flex; align-items: center; gap: 0.5rem; padding: 0.5rem 0.75rem; background: var(--ts-bg-base); border-radius: 6px; font-size: 0.85rem; }
.consent-item.sensitive { border-left: 3px solid var(--ts-warning); }
.consent-icon { font-size: 1.1rem; }
.consent-label { flex: 1; color: var(--ts-text-primary); }
.consent-warn { font-size: 0.7rem; color: var(--ts-warning); font-weight: 600; }
.consent-warning { margin: 0; padding: 0.5rem 0.75rem; background: var(--ts-warning-bg); color: var(--ts-warning); border-radius: 6px; font-size: 0.8rem; }
.consent-btns { display: flex; gap: 0.5rem; justify-content: flex-end; }
.btn-primary { padding: 0.4rem 1rem; background: var(--ts-accent-blue-hover); color: var(--ts-text-on-accent); border: none; border-radius: 6px; cursor: pointer; transition: background var(--ts-transition-fast); }
.btn-primary:hover { background: var(--ts-accent-blue); }
.btn-secondary { padding: 0.4rem 1rem; background: var(--ts-bg-elevated); color: var(--ts-text-primary); border: none; border-radius: 6px; cursor: pointer; transition: background var(--ts-transition-fast); }
.btn-secondary:hover { background: var(--ts-bg-hover); }
</style>
