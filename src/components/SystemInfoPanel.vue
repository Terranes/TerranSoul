<template>
  <PanelShell
    variant="overlay-fixed"
    title="📊 System Information"
    test-id="system-info-panel"
    :on-close="handleClose"
    @close="handleClose"
  >
    <!-- System Hardware -->
    <div class="info-section">
      <h4>💻 Hardware</h4>
      <div class="info-grid">
        <div class="info-row">
          <span class="info-label">OS:</span>
          <span class="info-value">{{ systemInfo?.os_name || 'Unknown' }} ({{ systemInfo?.arch || 'Unknown' }})</span>
        </div>
        <div class="info-row">
          <span class="info-label">CPU:</span>
          <span class="info-value">{{ systemInfo?.cpu_name || 'Unknown' }} ({{ systemInfo?.cpu_cores || 0 }} cores)</span>
        </div>
        <div class="info-row">
          <span class="info-label">RAM:</span>
          <span class="info-value">{{ formatRam(systemInfo?.total_ram_mb || 0) }} ({{ systemInfo?.ram_tier_label || 'Unknown' }})</span>
        </div>
        <div
          v-if="systemInfo?.gpu_name"
          class="info-row"
        >
          <span class="info-label">GPU:</span>
          <span class="info-value">{{ systemInfo.gpu_name }}</span>
        </div>
      </div>
    </div>

    <!-- AI Brain -->
    <div class="info-section">
      <h4>🧠 AI Brain</h4>
      <div class="info-grid">
        <div class="info-row">
          <span class="info-label">Status:</span>
          <span
            class="info-value"
            :class="{ 'status-active': brain.hasBrain, 'status-inactive': !brain.hasBrain }"
          >
            {{ brain.hasBrain ? 'Connected' : 'Not Configured' }}
          </span>
        </div>
        <div
          v-if="brain.brainMode"
          class="info-row"
        >
          <span class="info-label">Mode:</span>
          <span class="info-value">{{ getBrainModeDisplay() }}</span>
        </div>
        <div
          v-if="brain.brainMode?.mode === 'local_ollama' && brain.activeBrain"
          class="info-row"
        >
          <span class="info-label">Model:</span>
          <span class="info-value">{{ brain.activeBrain }}</span>
        </div>
        <div
          v-if="brain.brainMode?.mode === 'free_api'"
          class="info-row"
        >
          <span class="info-label">Provider:</span>
          <span class="info-value">{{ getProviderName() }}</span>
        </div>
      </div>
    </div>

    <!-- Audio System -->
    <div class="info-section">
      <h4>🎤 Audio System</h4>
      <div class="info-grid">
        <div class="info-row">
          <span class="info-label">ASR:</span>
          <span class="info-value">{{ voice.selectedAsrProvider?.display_name || 'None' }}</span>
        </div>
        <div class="info-row">
          <span class="info-label">TTS:</span>
          <span class="info-value">{{ voice.selectedTtsProvider?.display_name || 'None' }}</span>
        </div>
        <div class="info-row">
          <span class="info-label">System Volume:</span>
          <span class="info-value">{{ Math.round(systemVolume * 100) }}%</span>
        </div>
        <div class="info-row">
          <span class="info-label">BGM Volume:</span>
          <span class="info-value">{{ Math.round(bgmVolume * 100) }}%</span>
        </div>
      </div>
    </div>

    <!-- Renderer Info -->
    <div class="info-section">
      <h4>🎮 Renderer</h4>
      <div class="info-grid">
        <div class="info-row">
          <span class="info-label">Type:</span>
          <span class="info-value">{{ rendererInfo?.type || 'Unknown' }}</span>
        </div>
        <div class="info-row">
          <span class="info-label">Triangles:</span>
          <span class="info-value">{{ rendererInfo?.triangles?.toLocaleString() || '0' }}</span>
        </div>
        <div class="info-row">
          <span class="info-label">Draw Calls:</span>
          <span class="info-value">{{ rendererInfo?.calls || 0 }}</span>
        </div>
        <div class="info-row">
          <span class="info-label">Programs:</span>
          <span class="info-value">{{ rendererInfo?.programs || 0 }}</span>
        </div>
      </div>
    </div>

    <!-- Version Info -->
    <div class="info-section">
      <h4>ℹ️ Version</h4>
      <div class="info-grid">
        <div class="info-row">
          <span class="info-label">TerranSoul:</span>
          <span class="info-value">{{ version }}</span>
        </div>
        <div class="info-row">
          <span class="info-label">Build:</span>
          <span class="info-value">{{ new Date().toISOString().split('T')[0] }}</span>
        </div>
      </div>
    </div>
  </PanelShell>
</template>

<script setup lang="ts">
import { onMounted, computed, ref } from 'vue';
import { useBrainStore } from '../stores/brain';
import { useVoiceStore } from '../stores/voice';
import PanelShell from './ui/PanelShell.vue';

// Self-contained dialog: pulls live system/brain info from the brain store
// rather than props, so the parent doesn't need to keep sync state in sync.
const emit = defineEmits<{
  close: [];
}>();

function handleClose() {
  emit('close');
}

const brain = useBrainStore();
const voice = useVoiceStore();
const systemInfo = computed(() => brain.systemInfo);
const rendererInfo = ref({ type: 'Unknown', triangles: 0, calls: 0, programs: 0 });
const systemVolume = ref(100);
const bgmVolume = ref(75);
const version = '0.1.0';

function formatRam(mb: number): string {
  if (mb >= 1024) {
    return `${(mb / 1024).toFixed(1)} GB`;
  }
  return `${mb} MB`;
}

function getBrainModeDisplay(): string {
  if (!brain.brainMode) return 'Unknown';

  switch (brain.brainMode.mode) {
    case 'free_api':
      return 'Free Cloud API';
    case 'paid_api':
      return 'Paid API';
    case 'local_ollama':
      return 'Local Ollama';
    case 'local_lm_studio':
      return 'Local LM Studio';
    default:
      return 'Unknown';
  }
}

function getProviderName(): string {
  if (!brain.brainMode || brain.brainMode.mode !== 'free_api') return 'N/A';

  const freeApiMode = brain.brainMode as { mode: 'free_api'; provider_id: string; api_key: string | null };
  const provider = brain.freeProviders.find((p) => p.id === freeApiMode.provider_id);
  return provider?.display_name || freeApiMode.provider_id || 'Unknown';
}

onMounted(async () => {
  if (!brain.systemInfo) {
    await brain.fetchSystemInfo();
  }
  if (!voice.selectedAsrProvider && !voice.selectedTtsProvider) {
    await voice.initialise();
  }
});
</script>

<style scoped>
.info-section {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.info-section h4 {
  margin: 0;
  font-size: 0.9rem;
  font-weight: 600;
  color: var(--ts-accent-violet);
  border-bottom: 1px solid var(--ts-border);
  padding-bottom: 4px;
}

.info-grid {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.info-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 4px 0;
}

.info-label {
  font-size: 0.8rem;
  color: var(--ts-text-secondary);
  font-weight: 500;
  min-width: 100px;
}

.info-value {
  font-size: 0.8rem;
  color: var(--ts-text-primary);
  font-family: 'SF Mono', 'Monaco', 'Cascadia Code', 'Roboto Mono', Consolas, 'Courier New', monospace;
  text-align: right;
  flex: 1;
  word-break: break-word;
}

.status-active {
  color: var(--ts-success) !important;
  font-weight: 600;
}

.status-inactive {
  color: var(--ts-warning) !important;
  font-weight: 600;
}

@media (max-width: 640px) {
  .info-row {
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
  }

  .info-label {
    min-width: auto;
  }

  .info-value {
    text-align: left;
  }
}
</style>
