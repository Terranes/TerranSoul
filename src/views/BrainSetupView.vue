<template>
  <div class="brain-setup">
    <!-- Step indicator -->
    <div class="bs-steps">
      <div
        v-for="(label, i) in stepLabels"
        :key="i"
        :class="['bs-step', { active: step === i, done: step > i }]"
      >
        <span class="bs-step-dot">{{ step > i ? '✓' : i + 1 }}</span>
        <span class="bs-step-label">{{ label }}</span>
      </div>
    </div>

    <!-- ── Step 0: Welcome + Hardware analysis ── -->
    <div v-if="step === 0" class="bs-card">
      <h2>🧠 Add a Brain to TerranSoul</h2>
      <p class="bs-desc">
        TerranSoul needs a local AI brain to become truly intelligent. We'll analyse your
        hardware and recommend the best model for your machine — all running privately on
        your device.
      </p>
      <div v-if="brain.systemInfo" class="bs-hw">
        <div class="bs-hw-row">
          <span>💾 RAM</span>
          <strong>{{ formatRam(brain.systemInfo.total_ram_mb) }} ({{ brain.systemInfo.ram_tier_label }})</strong>
        </div>
        <div class="bs-hw-row">
          <span>🖥 CPU</span>
          <strong>{{ brain.systemInfo.cpu_name }} · {{ brain.systemInfo.cpu_cores }} cores</strong>
        </div>
        <div class="bs-hw-row">
          <span>🗂 OS</span>
          <strong>{{ brain.systemInfo.os_name }} ({{ brain.systemInfo.arch }})</strong>
        </div>
      </div>
      <p v-else class="bs-loading">Analysing hardware…</p>
      <button class="btn-primary" :disabled="!brain.systemInfo" @click="step = 1">
        Next →
      </button>
    </div>

    <!-- ── Step 1: Choose brain model ── -->
    <div v-else-if="step === 1" class="bs-card">
      <h2>Choose your Brain</h2>
      <p class="bs-desc">Based on your {{ formatRam(brain.systemInfo?.total_ram_mb ?? 0) }} of RAM, we recommend:</p>
      <ul class="bs-models">
        <li
          v-for="m in brain.recommendations"
          :key="m.model_tag"
          :class="['bs-model', { top: m.is_top_pick, selected: selectedModel === m.model_tag }]"
          @click="selectedModel = m.model_tag"
        >
          <div class="bs-model-header">
            <strong>{{ m.display_name }}</strong>
            <span v-if="m.is_top_pick" class="bs-badge">⭐ Recommended</span>
          </div>
          <p>{{ m.description }}</p>
          <small>Requires {{ formatRam(m.required_ram_mb) }} RAM · tag: <code>{{ m.model_tag }}</code></small>
        </li>
      </ul>
      <div class="bs-nav">
        <button class="btn-secondary" @click="step = 0">← Back</button>
        <button class="btn-primary" :disabled="!selectedModel" @click="step = 2">Next →</button>
      </div>
    </div>

    <!-- ── Step 2: Check / install Ollama ── -->
    <div v-else-if="step === 2" class="bs-card">
      <h2>Check Ollama</h2>
      <p class="bs-desc">
        TerranSoul uses <a href="https://ollama.ai" target="_blank">Ollama</a> to run models
        locally. It must be running before we can download your brain.
      </p>
      <div :class="['bs-status-indicator', brain.ollamaStatus.running ? 'ok' : 'error']">
        {{ brain.ollamaStatus.running ? '✅ Ollama is running' : '❌ Ollama is not running' }}
      </div>
      <div v-if="!brain.ollamaStatus.running" class="bs-install-hint">
        <p>Install and start Ollama:</p>
        <ol>
          <li>Download from <a href="https://ollama.ai" target="_blank">ollama.ai</a></li>
          <li>Run <code>ollama serve</code> in a terminal</li>
          <li>Click Retry below</li>
        </ol>
      </div>
      <div class="bs-nav">
        <button class="btn-secondary" @click="step = 1">← Back</button>
        <button class="btn-secondary" @click="brain.checkOllamaStatus()">🔄 Retry</button>
        <button class="btn-primary" :disabled="!brain.ollamaStatus.running" @click="step = 3">
          Next →
        </button>
      </div>
    </div>

    <!-- ── Step 3: Download model ── -->
    <div v-else-if="step === 3" class="bs-card">
      <h2>Download {{ selectedModel }}</h2>
      <p class="bs-desc">
        This will download the model via Ollama. It may take several minutes depending on
        your connection speed.
      </p>

      <div v-if="modelAlreadyInstalled" class="bs-status-indicator ok">
        ✅ Model already installed locally
      </div>
      <div v-else-if="brain.isPulling" class="bs-pulling">
        <div class="bs-spinner" />
        <span>Downloading… this may take a few minutes.</span>
      </div>
      <div v-else-if="brain.pullError" class="bs-status-indicator error">
        ❌ {{ brain.pullError }}
      </div>

      <div class="bs-nav">
        <button class="btn-secondary" @click="step = 2">← Back</button>
        <button
          v-if="!modelAlreadyInstalled && !brain.isPulling"
          class="btn-primary"
          @click="doPull"
        >
          ⬇ Download model
        </button>
        <button
          v-if="modelAlreadyInstalled || pullDone"
          class="btn-primary"
          @click="step = 4"
        >
          Next →
        </button>
      </div>
    </div>

    <!-- ── Step 4: Done ── -->
    <div v-else-if="step === 4" class="bs-card bs-done">
      <div class="bs-done-icon">🎉</div>
      <h2>Brain connected!</h2>
      <p>
        <strong>{{ selectedModel }}</strong> is now your brain.
        TerranSoul will use it for all future conversations, memory extraction, and smart recall.
      </p>
      <button class="btn-primary" @click="activate">Start chatting →</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useBrainStore } from '../stores/brain';

const emit = defineEmits<{ (e: 'done'): void }>();

const brain = useBrainStore();
const step = ref(0);
const selectedModel = ref('');
const pullDone = ref(false);

const stepLabels = ['Hardware', 'Choose model', 'Ollama', 'Download', 'Done'];

const modelAlreadyInstalled = computed(() =>
  brain.installedModels.some((m) => m.name === selectedModel.value),
);

function formatRam(mb: number): string {
  return mb >= 1024 ? `${(mb / 1024).toFixed(0)} GB` : `${mb} MB`;
}

async function doPull() {
  const ok = await brain.pullModel(selectedModel.value);
  if (ok) {
    pullDone.value = true;
    step.value = 4;
  }
}

async function activate() {
  await brain.setActiveBrain(selectedModel.value);
  emit('done');
}

onMounted(async () => {
  await brain.initialise();
  if (brain.recommendations.length > 0) {
    selectedModel.value = brain.topRecommendation?.model_tag ?? '';
  }
});
</script>

<style scoped>
.brain-setup { display: flex; flex-direction: column; align-items: center; gap: 1.5rem; padding: 2rem; min-height: 100%; background: #0f172a; }
.bs-steps { display: flex; gap: 0.5rem; align-items: center; }
.bs-step { display: flex; align-items: center; gap: 0.4rem; font-size: 0.8rem; color: #475569; }
.bs-step.active .bs-step-dot { background: #3b82f6; color: #fff; }
.bs-step.done .bs-step-dot { background: #22c55e; color: #fff; }
.bs-step-dot { width: 24px; height: 24px; border-radius: 50%; background: #1e293b; display: flex; align-items: center; justify-content: center; font-size: 0.75rem; }
.bs-step-label { display: none; }
@media (min-width: 480px) { .bs-step-label { display: inline; } }
.bs-card { background: #1e293b; border-radius: 12px; padding: 2rem; width: min(560px, 100%); display: flex; flex-direction: column; gap: 1rem; }
.bs-card h2 { margin: 0; font-size: 1.3rem; }
.bs-desc { color: #94a3b8; margin: 0; line-height: 1.5; }
.bs-loading { color: #64748b; }
.bs-hw { display: flex; flex-direction: column; gap: 0.4rem; background: #0f172a; border-radius: 8px; padding: 0.75rem 1rem; }
.bs-hw-row { display: flex; justify-content: space-between; font-size: 0.9rem; }
.bs-models { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 0.5rem; }
.bs-model { padding: 0.75rem 1rem; background: #0f172a; border-radius: 8px; border: 2px solid transparent; cursor: pointer; }
.bs-model.top { border-color: #3b82f6; }
.bs-model.selected { border-color: #22c55e; background: #1a2e1a; }
.bs-model-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.25rem; }
.bs-badge { font-size: 0.75rem; background: #1d4ed8; color: #bfdbfe; padding: 0.1rem 0.5rem; border-radius: 999px; }
.bs-model p { margin: 0 0 0.25rem; font-size: 0.85rem; color: #94a3b8; }
.bs-model small { color: #475569; font-size: 0.75rem; }
.bs-model code { background: #1e293b; padding: 0.1rem 0.3rem; border-radius: 3px; }
.bs-status-indicator { padding: 0.75rem 1rem; border-radius: 8px; font-weight: 500; }
.bs-status-indicator.ok { background: #1a2e1a; color: #86efac; }
.bs-status-indicator.error { background: #2d1c1c; color: #fca5a5; }
.bs-install-hint { background: #0f172a; border-radius: 8px; padding: 0.75rem 1rem; font-size: 0.85rem; color: #94a3b8; }
.bs-install-hint ol { margin: 0.5rem 0 0 1.25rem; line-height: 1.8; }
.bs-pulling { display: flex; align-items: center; gap: 0.75rem; color: #94a3b8; }
.bs-spinner { width: 20px; height: 20px; border: 3px solid #334155; border-top-color: #3b82f6; border-radius: 50%; animation: spin 0.8s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
.bs-nav { display: flex; gap: 0.5rem; justify-content: flex-end; margin-top: 0.5rem; }
.bs-done { align-items: center; text-align: center; }
.bs-done-icon { font-size: 3rem; }
.btn-primary { padding: 0.5rem 1.25rem; background: #3b82f6; color: #fff; border: none; border-radius: 8px; cursor: pointer; font-size: 0.9rem; }
.btn-primary:disabled { opacity: 0.4; cursor: not-allowed; }
.btn-secondary { padding: 0.5rem 1.25rem; background: #334155; color: #f1f5f9; border: none; border-radius: 8px; cursor: pointer; font-size: 0.9rem; }
a { color: #60a5fa; }
</style>
