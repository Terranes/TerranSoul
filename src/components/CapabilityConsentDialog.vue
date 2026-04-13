<template>
  <div class="consent-backdrop" @click.self="$emit('cancel')">
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
          <span v-if="isSensitive(cap)" class="consent-warn">⚠ sensitive</span>
        </li>
      </ul>

      <p v-if="hasSensitive" class="consent-warning">
        This agent requests sensitive capabilities that could access your files, clipboard,
        network, or spawn processes. Only grant these if you trust the agent.
      </p>

      <div class="consent-btns">
        <button class="btn-primary" @click="$emit('confirm')">Grant &amp; Install</button>
        <button class="btn-secondary" @click="$emit('cancel')">Cancel</button>
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
.consent-backdrop { position: fixed; inset: 0; background: rgba(0,0,0,0.6); display: flex; align-items: center; justify-content: center; z-index: 100; }
.consent-dialog { background: #1e293b; border-radius: 12px; padding: 1.5rem; width: min(480px, 90vw); display: flex; flex-direction: column; gap: 0.75rem; }
.consent-desc { margin: 0; color: #cbd5e1; font-size: 0.9rem; }
.consent-list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 0.4rem; }
.consent-item { display: flex; align-items: center; gap: 0.5rem; padding: 0.5rem 0.75rem; background: #0f172a; border-radius: 6px; font-size: 0.85rem; }
.consent-item.sensitive { border-left: 3px solid #f59e0b; }
.consent-icon { font-size: 1.1rem; }
.consent-label { flex: 1; color: #e2e8f0; }
.consent-warn { font-size: 0.7rem; color: #f59e0b; font-weight: 600; }
.consent-warning { margin: 0; padding: 0.5rem 0.75rem; background: #451a03; color: #fbbf24; border-radius: 6px; font-size: 0.8rem; }
.consent-btns { display: flex; gap: 0.5rem; justify-content: flex-end; }
.btn-primary { padding: 0.4rem 1rem; background: #3b82f6; color: #fff; border: none; border-radius: 6px; cursor: pointer; }
.btn-secondary { padding: 0.4rem 1rem; background: #334155; color: #f1f5f9; border: none; border-radius: 6px; cursor: pointer; }
</style>
