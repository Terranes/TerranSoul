<template>
  <Teleport to="body">
    <Transition name="flw-backdrop">
      <div
        v-if="visible"
        class="flw-backdrop"
      >
        <Transition name="flw-dialog">
          <div
            v-if="visible"
            class="flw-dialog"
          >
            <!-- Step 1: Quest acceptance mode -->
            <template v-if="step === 'quest-mode'">
              <div class="flw-header">
                <span class="flw-logo">🌟</span>
                <h2 class="flw-title">
                  Welcome to TerranSoul
                </h2>
                <p class="flw-subtitle">
                  Your AI companion is ready. How would you like to discover features?
                </p>
              </div>

              <div class="flw-options">
                <button
                  class="flw-option flw-option--recommended"
                  @click="chooseAutoAll"
                >
                  <span class="flw-option-icon">✨</span>
                  <div class="flw-option-text">
                    <strong>Auto-Accept All</strong>
                    <span class="flw-option-desc">
                      Activate all available quests instantly — brain, voice, avatar, and more. Jump right in.
                    </span>
                  </div>
                  <span class="flw-badge">Recommended</span>
                </button>

                <button
                  class="flw-option"
                  @click="chooseOneByOne"
                >
                  <span class="flw-option-icon">📜</span>
                  <div class="flw-option-text">
                    <strong>Accept One by One</strong>
                    <span class="flw-option-desc">
                      Discover each quest at your own pace — you'll be guided through them step by step.
                    </span>
                  </div>
                </button>

                <button
                  class="flw-option"
                  @click="chooseManual"
                >
                  <span class="flw-option-icon">🔧</span>
                  <div class="flw-option-text">
                    <strong>Set Up From Scratch</strong>
                    <span class="flw-option-desc">
                      Skip auto-configuration — configure each feature manually through settings.
                    </span>
                  </div>
                </button>
              </div>
            </template>

            <!-- Step 3: Setting up (progress) -->
            <template v-if="step === 'confirm-disk'">
              <div class="flw-header">
                <span class="flw-logo">💾</span>
                <h2 class="flw-title">
                  Disk Space Check
                </h2>
                <p class="flw-subtitle">
                  A local AI model will be downloaded to your computer.
                </p>
              </div>

              <div class="flw-disk-info">
                <div class="flw-disk-row">
                  <span class="flw-disk-label">Model</span>
                  <span class="flw-disk-value">{{ modelDisplayName }}</span>
                </div>
                <div class="flw-disk-row">
                  <span class="flw-disk-label">Download size</span>
                  <span class="flw-disk-value flw-disk-size">
                    ~{{ (modelDownloadMb / 1024).toFixed(1) }} GB
                  </span>
                </div>
                <div class="flw-disk-row">
                  <span class="flw-disk-label">Storage location</span>
                  <span class="flw-disk-value flw-disk-path">{{ ollamaDir || 'Default' }}</span>
                </div>
                <div class="flw-disk-row">
                  <span class="flw-disk-label">Available space</span>
                  <span
                    class="flw-disk-value"
                    :class="diskInsufficient ? 'flw-disk-warn' : 'flw-disk-ok'"
                  >
                    {{ formatBytes(diskAvailable) }} / {{ formatBytes(diskTotal) }}
                    <span v-if="diskMountPoint"> on {{ diskMountPoint }}</span>
                  </span>
                </div>

                <!-- Disk space bar -->
                <div class="flw-disk-bar-container">
                  <div class="flw-disk-bar">
                    <div
                      class="flw-disk-bar-used"
                      :style="{ width: Math.min(100, ((diskTotal - diskAvailable) / diskTotal) * 100) + '%' }"
                    />
                    <div
                      class="flw-disk-bar-model"
                      :style="{ width: Math.min(100, (modelDownloadMb * 1024 * 1024 / diskTotal) * 100) + '%' }"
                    />
                  </div>
                  <div class="flw-disk-bar-legend">
                    <span><span class="flw-legend-dot flw-legend-used" /> Used</span>
                    <span><span class="flw-legend-dot flw-legend-model" /> Model</span>
                    <span><span class="flw-legend-dot flw-legend-free" /> Free</span>
                  </div>
                </div>

                <!-- Warning if insufficient -->
                <div
                  v-if="diskInsufficient"
                  class="flw-disk-warning"
                >
                  ⚠️ Not enough disk space! Free up at least
                  {{ (modelDownloadMb / 1024).toFixed(1) }} GB or use a different drive.
                </div>

                <!-- Other drives (if multiple) -->
                <div
                  v-if="allDrives.length > 1"
                  class="flw-drives-section"
                >
                  <p class="flw-drives-label">
                    Other drives available:
                  </p>
                  <div
                    v-for="drive in allDrives.filter(d => d.mount_point !== diskMountPoint)"
                    :key="drive.mount_point"
                    class="flw-drive-item"
                  >
                    <span class="flw-drive-name">
                      {{ drive.mount_point }} {{ drive.label ? `(${drive.label})` : '' }}
                    </span>
                    <span class="flw-drive-space">
                      {{ formatBytes(drive.available_bytes) }} free
                    </span>
                  </div>
                  <p class="flw-drives-hint">
                    To use a different drive, set the <code>OLLAMA_MODELS</code> environment
                    variable to your preferred path and restart Ollama.
                  </p>
                </div>
              </div>

              <div class="flw-disk-actions">
                <button
                  class="flw-done-btn"
                  :disabled="diskInsufficient"
                  @click="confirmDiskAndProceed"
                >
                  {{ diskInsufficient ? 'Insufficient Space' : 'Continue with Download →' }}
                </button>
                <button
                  class="flw-back"
                  @click="step = 'quest-mode'"
                >
                  ← Back
                </button>
              </div>
            </template>

            <!-- Step 4: Setting up (progress) -->
            <template v-if="step === 'setup'">
              <div class="flw-header">
                <span class="flw-logo flw-spin">⚙️</span>
                <h2 class="flw-title">
                  Setting Up...
                </h2>
                <p class="flw-subtitle">
                  {{ setupMessage }}
                </p>
              </div>
              <div class="flw-progress">
                <div
                  class="flw-progress-bar"
                  :style="{ width: setupProgress + '%' }"
                />
              </div>
              <p class="flw-percent">
                {{ setupProgress }}%
              </p>

              <!-- Debug Log Toggle -->
              <button
                class="flw-debug-toggle"
                @click="showDebugLog = !showDebugLog"
              >
                {{ showDebugLog ? '▼ Hide' : '▶ Show' }} Debug Log ({{ debugLog.length }} entries)
              </button>
              <div
                v-if="showDebugLog"
                ref="debugLogRef"
                class="flw-debug-log"
              >
                <div
                  v-for="(entry, i) in debugLog"
                  :key="i"
                  class="flw-debug-entry"
                >
                  <span class="flw-debug-time">{{ entry.time }}</span>
                  <span :class="['flw-debug-msg', entry.level]">{{ entry.message }}</span>
                </div>
                <div
                  v-if="debugLog.length === 0"
                  class="flw-debug-entry"
                >
                  Waiting for events...
                </div>
              </div>
            </template>

            <!-- Step 4: Done (success or failure) -->
            <template v-if="step === 'done'">
              <div class="flw-header">
                <span class="flw-logo">{{ setupFailed ? '⚠️' : '🎉' }}</span>
                <h2 class="flw-title">
                  {{ setupFailed ? 'Setup Completed with Issues' : 'All Set!' }}
                </h2>
                <p class="flw-subtitle">
                  {{ doneMessage }}
                </p>
              </div>

              <div class="flw-summary">
                <div
                  v-for="item in completedItems"
                  :key="item.label"
                  class="flw-summary-item"
                  :class="{ 'flw-summary-item--error': item.error }"
                >
                  <span class="flw-summary-icon">{{ item.icon }}</span>
                  <span>{{ item.label }}</span>
                </div>
              </div>

              <!-- Debug Log Toggle (visible on done screen) -->
              <button
                class="flw-debug-toggle"
                @click="showDebugLog = !showDebugLog"
              >
                {{ showDebugLog ? '▼ Hide' : '▶ Show' }} Debug Log ({{ debugLog.length }} entries)
              </button>
              <div
                v-if="showDebugLog"
                ref="debugLogRef"
                class="flw-debug-log"
              >
                <div
                  v-for="(entry, i) in debugLog"
                  :key="i"
                  class="flw-debug-entry"
                >
                  <span class="flw-debug-time">{{ entry.time }}</span>
                  <span :class="['flw-debug-msg', entry.level]">{{ entry.message }}</span>
                </div>
              </div>

              <button
                class="flw-done-btn"
                @click="$emit('done')"
              >
                {{ setupFailed ? 'Continue Anyway →' : 'Start Chatting →' }}
              </button>
            </template>
          </div>
        </Transition>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
/* eslint-disable max-lines */
import { ref, computed, nextTick, onUnmounted } from 'vue';
import { useBrainStore } from '../stores/brain';
import { useVoiceStore } from '../stores/voice';
import { useSkillTreeStore } from '../stores/skill-tree';
import { useSettingsStore } from '../stores/settings';

let listenCleanup: (() => void) | null = null;

const emit = defineEmits<{ done: [] }>();
defineProps<{ visible: boolean }>();

const brain = useBrainStore();
const voice = useVoiceStore();
const skillTree = useSkillTreeStore();
const settingsStore = useSettingsStore();

type Step = 'quest-mode' | 'confirm-disk' | 'setup' | 'done';
const step = ref<Step>('quest-mode');
const setupMessage = ref('Preparing...');
const setupProgress = ref(0);
const setupFailed = ref(false);
const autoAcceptMode = ref(true);

// ── Disk check state ─────────────────────────────────────────────────
interface DriveInfo { mount_point: string; label: string; available_bytes: number; total_bytes: number }
const ollamaDir = ref('');
const diskAvailable = ref(0);
const diskTotal = ref(0);
const diskMountPoint = ref('');
const allDrives = ref<DriveInfo[]>([]);
const modelDownloadMb = ref(0);
const modelDisplayName = ref('');
const diskInsufficient = computed(
  () => modelDownloadMb.value > 0 && diskAvailable.value > 0
    && modelDownloadMb.value * 1024 * 1024 > diskAvailable.value,
);
const spaceFreedMb = ref(0);

// ── Debug log ────────────────────────────────────────────────────────
interface DebugEntry { time: string; message: string; level: 'info' | 'warn' | 'error' }
const debugLog = ref<DebugEntry[]>([]);
const showDebugLog = ref(false);
const debugLogRef = ref<HTMLElement | null>(null);

function logDebug(message: string, level: DebugEntry['level'] = 'info') {
  const now = new Date();
  const time = [now.getHours(), now.getMinutes(), now.getSeconds()]
    .map(n => String(n).padStart(2, '0')).join(':');
  debugLog.value.push({ time, message, level });
  // Auto-scroll when log is visible.
  if (showDebugLog.value) {
    nextTick(() => {
      debugLogRef.value?.scrollTo({ top: debugLogRef.value.scrollHeight });
    });
  }
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

// Which quests got auto-completed
const completedItems = ref<{ icon: string; label: string; error?: boolean }[]>([]);

const doneMessage = computed(() => {
  if (setupFailed.value) {
    return 'Some steps had issues. You can check the debug log for details or continue to configure manually.';
  }
  if (completedItems.value.length > 0) {
    return 'Your companion is fully configured and quests are activated. Say hi!';
  }
  return 'Your companion is ready to chat. Explore quests at your own pace!';
});

// ── Setup: progress event listener ────────────────────────────────────
/** Start listening to Ollama pull progress events from the Tauri backend. */
async function startPullProgressListener() {
  try {
    const { listen } = await import('@tauri-apps/api/event');
    const unlisten = await listen<{
      status: string;
      digest: string;
      total: number;
      completed: number;
      percent: number;
    }>('ollama-pull-progress', (event) => {
      const p = event.payload;
      // Update the progress bar with the real percentage from Ollama.
      // Map pull progress into the 5–30% range reserved for brain setup.
      const pullPercent = Math.min(p.percent, 100);
      setupProgress.value = 5 + Math.round(pullPercent * 0.25);

      // Update message
      if (p.total > 0) {
        const done = formatBytes(p.completed);
        const total = formatBytes(p.total);
        setupMessage.value = `Downloading model: ${done} / ${total} (${p.percent}%)`;
      } else if (p.status) {
        setupMessage.value = `${p.status}...`;
      }

      // Log interesting events
      if (p.status === 'pulling manifest') {
        logDebug('Pulling manifest from Ollama registry');
      } else if (p.status === 'verifying sha256 digest') {
        logDebug('Verifying layer integrity (SHA-256)');
      } else if (p.status === 'writing manifest') {
        logDebug('Writing model manifest');
      } else if (p.status === 'success') {
        logDebug('Model pull completed successfully');
      } else if (p.total > 0 && p.completed >= p.total) {
        const short = p.digest.slice(0, 16);
        logDebug(`Layer ${short}… done (${formatBytes(p.total)})`);
      }
    });
    listenCleanup = unlisten;
  } catch {
    // No Tauri backend (e.g. E2E tests) — ignore.
    logDebug('Tauri event listener unavailable (running without backend)', 'warn');
  }
}

let installListenCleanup: (() => void) | null = null;

/** Start listening to Ollama install progress events (download + setup phases). */
async function startInstallProgressListener() {
  try {
    const { listen } = await import('@tauri-apps/api/event');
    const unlisten = await listen<{ phase: string; percent: number }>('ollama-install-progress', (event) => {
      const p = event.payload;
      setupMessage.value = p.phase;
      // Map install progress into the 5–25% range reserved for Ollama install.
      setupProgress.value = 5 + Math.round((p.percent / 100) * 20);
      logDebug(`Install: ${p.phase} (${p.percent}%)`);
    });
    installListenCleanup = unlisten;
  } catch {
    // No Tauri backend — ignore.
  }
}

function stopInstallProgressListener() {
  if (installListenCleanup) {
    installListenCleanup();
    installListenCleanup = null;
  }
}

function stopPullProgressListener() {
  if (listenCleanup) {
    listenCleanup();
    listenCleanup = null;
  }
}

onUnmounted(() => {
  stopPullProgressListener();
  stopInstallProgressListener();
});

async function chooseManual() {
  // Mark first launch as complete but don't auto-configure.
  await settingsStore.saveSettings({ first_launch_complete: true });
  emit('done');
}

async function chooseAutoAll() {
  autoAcceptMode.value = true;
  await prepareDiskCheck();
}

async function chooseOneByOne() {
  autoAcceptMode.value = false;
  await prepareDiskCheck();
}

/** Gather disk info and the recommended model, then show confirmation. */
async function prepareDiskCheck() {
  try {
    // Refresh catalogue + recommendations + system info in parallel.
    await Promise.allSettled([
      brain.checkOllamaStatus(),
      brain.fetchSystemInfo(),
      brain.fetchRecommendations(),
      brain.fetchInstalledModels(),
    ]);

    // Determine which model would be pulled.
    const top = brain.topRecommendation;
    if (top) {
      modelDisplayName.value = top.display_name;
      modelDownloadMb.value = top.download_size_mb ?? 0;
    }

    // Only show disk check if Ollama is running and we need to pull.
    if (brain.ollamaStatus.running) {
      // Already installed? Skip disk check.
      const topTag = top?.model_tag;
      const installed = brain.installedModels;
      if (topTag && installed.some(m => m.name === topTag)) {
        step.value = 'setup';
        await runRecommendedSetup(autoAcceptMode.value);
        return;
      }

      // Get Ollama models dir + disk info.
      try {
        const [dir, drives] = await Promise.all([
          brain.getOllamaModelsDir(),
          brain.listDrives(),
        ]);
        ollamaDir.value = dir;
        allDrives.value = drives;
        const diskInfo = await brain.getDiskSpace(dir);
        diskAvailable.value = diskInfo.available_bytes;
        diskTotal.value = diskInfo.total_bytes;
        diskMountPoint.value = diskInfo.mount_point;
      } catch {
        // If disk queries fail, proceed without checking.
      }

      if (modelDownloadMb.value > 0) {
        step.value = 'confirm-disk';
        return;
      }
    }
  } catch {
    // If any detection fails, just proceed.
  }

  step.value = 'setup';
  await runRecommendedSetup(autoAcceptMode.value);
}

function confirmDiskAndProceed() {
  step.value = 'setup';
  runRecommendedSetup(autoAcceptMode.value);
}

async function runRecommendedSetup(autoAcceptAll: boolean) {
  const items: { icon: string; label: string; error?: boolean }[] = [];
  const autoConfigured: string[] = [];

  logDebug('Starting recommended setup');

  // Start listening for pull + install progress events.
  await startPullProgressListener();
  await startInstallProgressListener();

  // Suppress quest-unlock / combo-unlock notifications during batch setup
  // so the user isn't blasted with popups for every auto-detected feature.
  skillTree.suppressNotifications();

  // ── Phase 1: Brain (Local-First per rules/local-first-brain.md) ─────
  setupMessage.value = 'Detecting local AI runtime...';
  setupProgress.value = 5;
  logDebug('Phase 1: Brain configuration');

  try {
    if (!brain.hasBrain) {
      const preferLocal = settingsStore.settings.prefer_local_brain !== false;

      if (preferLocal) {
        logDebug('Prefer local brain — checking Ollama...');
        const result = await brain.autoConfigureLocalFirst({
          onProgress: (msg: string) => {
            setupMessage.value = msg;
            logDebug(msg);
          },
        });
        autoConfigured.push('brain');

        if (result.mode === 'local') {
          const pullNote = result.pulled ? ' — just downloaded' : '';
          const installNote = result.ollamaInstalled && !result.ollamaStarted
            ? ' (Ollama auto-installed)'
            : result.ollamaStarted ? ' (Ollama auto-started)' : '';
          items.push({
            icon: '🧠',
            label: `Brain connected (Local — ${result.model}${pullNote})${installNote}`,
          });
          logDebug(`Brain configured: local ${result.model}${pullNote}${installNote}`);
        } else if (result.pullFailed) {
          // Download was attempted but failed — show as error.
          logDebug(`Model download failed: ${result.pullFailed}`, 'error');
          items.push({
            icon: '⚠️',
            label: `Local model download failed — using cloud fallback. Error: ${result.pullFailed}`,
            error: true,
          });
          setupFailed.value = true;
        } else {
          items.push({
            icon: '🧠',
            label: 'Brain connected (Pollinations AI — free cloud fallback)',
          });
          logDebug('Brain configured: cloud fallback (Pollinations AI)');
        }
      } else {
        setupMessage.value = 'Configuring AI brain (cloud)...';
        logDebug('User preference: cloud brain');
        try {
          await brain.autoConfigureForDesktop();
        } catch {
          brain.autoConfigureFreeApi();
        }
        autoConfigured.push('brain');
        items.push({ icon: '🧠', label: 'Brain connected (Pollinations AI — free cloud)' });
        logDebug('Brain configured: Pollinations AI');
      }
    } else {
      const mode = brain.brainMode;
      if (mode?.mode === 'local_ollama') {
        items.push({ icon: '🧠', label: `Brain connected (Local — ${mode.model})` });
        logDebug(`Brain already configured: local ${mode.model}`);
      } else if (mode?.mode === 'free_api') {
        items.push({ icon: '🧠', label: `Brain connected (${mode.provider_id} — free cloud)` });
        logDebug(`Brain already configured: ${mode.provider_id}`);
      } else {
        items.push({ icon: '🧠', label: 'Brain connected' });
        logDebug('Brain already configured');
      }
    }
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    logDebug(`Brain setup failed: ${msg}`, 'error');
    items.push({ icon: '❌', label: `Brain setup failed: ${msg}`, error: true });
    setupFailed.value = true;
  }
  setupProgress.value = 30;

  // Stop the pull + install progress listeners (download phase is over).
  stopPullProgressListener();
  stopInstallProgressListener();

  // ── Cleanup: track space freed by removing old models ──────────────
  try {
    await brain.fetchInstalledModels();
    // Measure disk space freed (if we can reach the disk API).
    if (ollamaDir.value) {
      const afterDisk = await brain.getDiskSpace(ollamaDir.value);
      const freed = afterDisk.available_bytes - diskAvailable.value;
      if (freed > 1024 * 1024) {
        // Disk space increased → old models were removed.
        spaceFreedMb.value = Math.round(freed / (1024 * 1024));
        logDebug(`Disk cleanup: freed ${formatBytes(freed)}`);
      } else if (freed < -(1024 * 1024)) {
        // Disk space decreased → model was downloaded.
        logDebug(`Model downloaded: used ${formatBytes(-freed)} of disk space`);
      }
    }
  } catch {
    // Non-critical — skip cleanup reporting.
  }

  // ── Phase 2: Voice (TTS) ────────────────────────────────────────────
  setupMessage.value = 'Setting up voice...';
  setupProgress.value = 40;
  logDebug('Phase 2: Voice configuration');

  try {
    if (!voice.hasVoice) {
      await voice.autoConfigureVoice();
      autoConfigured.push('voice');
    }
    items.push({ icon: '🗣️', label: 'Voice enabled (Edge TTS neural voices)' });
    logDebug('Voice configured');
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    logDebug(`Voice setup failed: ${msg}`, 'error');
    items.push({ icon: '⚠️', label: `Voice setup issue: ${msg}`, error: true });
  }
  setupProgress.value = 60;

  // ── Phase 3: Skill tree initialization ──────────────────────────────
  setupMessage.value = 'Initializing quest system...';
  setupProgress.value = 70;
  logDebug('Phase 3: Skill tree initialization');

  try {
    await skillTree.initialise();
    items.push({ icon: '✨', label: 'Avatar loaded (3D VRM companion)' });
    logDebug('Skill tree initialised');
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    logDebug(`Skill tree init failed: ${msg}`, 'error');
    items.push({ icon: '⚠️', label: `Quest system issue: ${msg}`, error: true });
  }

  // ── Phase 4: Quest activation ───────────────────────────────────────
  if (autoAcceptAll) {
    setupMessage.value = 'Activating quests...';
    setupProgress.value = 80;
    logDebug('Phase 4: Quest system ready');

    // On first launch, do NOT mark any quests as completed or report them
    // as "auto-activated". The skill tree auto-detects active features via
    // checkActive() but we don't want to confuse users by showing
    // completion badges before they've even started using the app.
    // Quests will naturally show as "available" or "active" based on
    // real-time feature detection as the user explores the app.
    items.push({ icon: '⚔️', label: 'Quests ready — explore your skill tree!' });
    autoConfigured.push('quests');
    logDebug('Quests: ready for discovery (no pre-completion marks)');
  } else {
    items.push({ icon: '📜', label: 'Quests ready — accept them one by one from the Quest tab' });
    logDebug('Quests: manual mode');
  }
  setupProgress.value = 90;

  // Resume notifications — marks all current activations + combos as "seen"
  // so only future user-driven unlocks trigger ceremonies.
  skillTree.resumeNotifications();

  // ── Phase 5: Persist ────────────────────────────────────────────────
  setupMessage.value = 'Saving configuration...';
  logDebug('Phase 5: Persisting settings');
  await settingsStore.saveSettings({
    first_launch_complete: true,
    auto_configured: autoConfigured,
  });
  setupProgress.value = 100;
  logDebug(setupFailed.value ? 'Setup completed with issues' : 'Setup completed successfully');

  // Add disk space summary if relevant.
  if (spaceFreedMb.value > 0) {
    const gb = (spaceFreedMb.value / 1024).toFixed(1);
    items.push({ icon: '🧹', label: `Cleaned up old models — freed ${gb} GB` });
  }

  completedItems.value = items;

  // Brief pause so user sees 100%
  await new Promise(r => setTimeout(r, 600));
  step.value = 'done';
}
</script>

<style scoped>
.flw-backdrop {
  position: fixed;
  inset: 0;
  z-index: 300;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--ts-bg-backdrop);
  backdrop-filter: blur(8px);
}

.flw-dialog {
  width: min(520px, 92vw);
  max-height: 90vh;
  overflow-y: auto;
  background: var(--ts-bg-surface);
  border: 1px solid var(--ts-border);
  border-radius: 16px;
  padding: 2rem;
  box-shadow: var(--ts-shadow-lg);
}

.flw-header {
  text-align: center;
  margin-bottom: 1.5rem;
}

.flw-logo {
  display: inline-block;
  font-size: 3rem;
  margin-bottom: 0.5rem;
}

.flw-spin {
  animation: flw-rotate 2s linear infinite;
}

@keyframes flw-rotate {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.flw-title {
  font-size: 1.5rem;
  font-weight: 700;
  color: var(--ts-text-primary);
  margin: 0 0 0.5rem;
}

.flw-subtitle {
  font-size: 0.95rem;
  color: var(--ts-text-secondary);
  margin: 0;
  line-height: 1.5;
}

.flw-options {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.flw-option {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 1rem 1.25rem;
  background: var(--ts-bg-card);
  border: 1px solid var(--ts-border);
  border-radius: 12px;
  cursor: pointer;
  text-align: left;
  color: var(--ts-text-primary);
  transition: all 0.2s ease;
  position: relative;
}

.flw-option:hover {
  background: var(--ts-accent-glow);
  border-color: var(--ts-accent);
  transform: translateY(-1px);
}

.flw-option--recommended {
  border-color: var(--ts-success-bg);
  background: var(--ts-success-bg);
}

.flw-option--recommended:hover {
  background: rgba(100, 200, 120, 0.12);
  border-color: rgba(100, 200, 120, 0.5);
}

.flw-option-icon {
  font-size: 1.75rem;
  flex-shrink: 0;
}

.flw-option-text {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  flex: 1;
}

.flw-option-text strong {
  font-size: 1rem;
}

.flw-option-desc {
  font-size: 0.85rem;
  color: var(--ts-text-secondary);
  line-height: 1.4;
}

.flw-badge {
  font-size: 0.7rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  padding: 0.2rem 0.5rem;
  border-radius: 6px;
  background: var(--ts-success-bg);
  color: var(--ts-success);
  flex-shrink: 0;
}

.flw-back {
  display: inline-block;
  margin-top: 1rem;
  padding: 0.4rem 0.8rem;
  background: none;
  border: none;
  color: var(--ts-text-secondary);
  cursor: pointer;
  font-size: 0.85rem;
}

.flw-back:hover {
  color: var(--ts-text-primary);
}

/* ── Disk check step ─────────────────────────────────────────────── */
.flw-disk-info {
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
  margin-bottom: 1.25rem;
}

.flw-disk-row {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  gap: 1rem;
  font-size: 0.88rem;
}

.flw-disk-label {
  color: var(--ts-text-secondary);
  flex-shrink: 0;
}

.flw-disk-value {
  text-align: right;
  font-weight: 500;
  word-break: break-all;
}

.flw-disk-size {
  color: var(--ts-accent, #6478ff);
  font-size: 1rem;
  font-weight: 600;
}

.flw-disk-path {
  font-family: 'Fira Code', 'Cascadia Code', monospace;
  font-size: 0.8rem;
  opacity: 0.8;
  max-width: 220px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.flw-disk-ok { color: var(--ts-success, #4caf50); }
.flw-disk-warn { color: var(--ts-danger, #e84040); font-weight: 600; }

.flw-disk-bar-container {
  margin-top: 0.25rem;
}

.flw-disk-bar {
  display: flex;
  height: 10px;
  border-radius: 5px;
  overflow: hidden;
  background: var(--ts-bg-card);
}

.flw-disk-bar-used {
  background: var(--ts-text-secondary);
  opacity: 0.4;
  flex-shrink: 0;
}

.flw-disk-bar-model {
  background: var(--ts-accent, #6478ff);
  opacity: 0.7;
  flex-shrink: 0;
}

.flw-disk-bar-legend {
  display: flex;
  gap: 1rem;
  justify-content: center;
  margin-top: 0.35rem;
  font-size: 0.72rem;
  color: var(--ts-text-secondary);
}

.flw-legend-dot {
  display: inline-block;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  margin-right: 3px;
  vertical-align: middle;
}

.flw-legend-used { background: var(--ts-text-secondary); opacity: 0.4; }
.flw-legend-model { background: var(--ts-accent, #6478ff); opacity: 0.7; }
.flw-legend-free { background: var(--ts-bg-card); border: 1px solid var(--ts-border); }

.flw-disk-warning {
  padding: 0.65rem 0.85rem;
  background: rgba(232, 64, 64, 0.08);
  border: 1px solid rgba(232, 64, 64, 0.2);
  border-radius: 8px;
  color: var(--ts-danger, #e84040);
  font-size: 0.85rem;
  text-align: center;
}

.flw-drives-section {
  margin-top: 0.5rem;
  padding-top: 0.5rem;
  border-top: 1px solid var(--ts-border);
}

.flw-drives-label {
  font-size: 0.82rem;
  color: var(--ts-text-secondary);
  margin-bottom: 0.35rem;
}

.flw-drive-item {
  display: flex;
  justify-content: space-between;
  font-size: 0.82rem;
  padding: 0.25rem 0;
}

.flw-drive-name { color: var(--ts-text-primary); }
.flw-drive-space { color: var(--ts-text-secondary); }

.flw-drives-hint {
  font-size: 0.75rem;
  color: var(--ts-text-secondary);
  margin-top: 0.5rem;
  line-height: 1.4;
}

.flw-drives-hint code {
  background: var(--ts-bg-card);
  padding: 1px 4px;
  border-radius: 3px;
  font-size: 0.72rem;
}

.flw-disk-actions {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  align-items: center;
}

.flw-disk-actions .flw-done-btn {
  width: 100%;
}

.flw-disk-actions .flw-done-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  filter: grayscale(0.5);
}

.flw-progress {
  height: 6px;
  background: var(--ts-bg-card);
  border-radius: 3px;
  overflow: hidden;
  margin-top: 1.5rem;
}

.flw-progress-bar {
  height: 100%;
  background: linear-gradient(90deg, #6478ff, #7ddf8e);
  border-radius: 3px;
  transition: width 0.4s ease;
}

.flw-percent {
  text-align: center;
  font-size: 0.85rem;
  color: var(--ts-text-secondary);
  margin: 0.5rem 0 0;
}

.flw-debug-toggle {
  display: block;
  margin: 1rem auto 0;
  padding: 0.35rem 0.75rem;
  background: none;
  border: 1px solid var(--ts-border);
  border-radius: 6px;
  color: var(--ts-text-secondary);
  font-size: 0.78rem;
  cursor: pointer;
  transition: all 0.15s;
}

.flw-debug-toggle:hover {
  color: var(--ts-text-primary);
  border-color: var(--ts-accent);
}

.flw-debug-log {
  margin-top: 0.5rem;
  max-height: 180px;
  overflow-y: auto;
  background: var(--ts-bg-card);
  border: 1px solid var(--ts-border);
  border-radius: 8px;
  padding: 0.5rem;
  font-family: 'Fira Code', 'Cascadia Code', monospace;
  font-size: 0.72rem;
  line-height: 1.5;
}

.flw-debug-entry {
  display: flex;
  gap: 0.5rem;
  color: var(--ts-text-secondary);
}

.flw-debug-time {
  flex-shrink: 0;
  opacity: 0.5;
}

.flw-debug-msg.info { color: var(--ts-text-secondary); }
.flw-debug-msg.warn { color: var(--ts-warning, #e8a838); }
.flw-debug-msg.error { color: var(--ts-danger, #e84040); }

.flw-summary {
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
  margin-bottom: 1.5rem;
}

.flw-summary-item {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.6rem 0.8rem;
  background: var(--ts-success-bg);
  border-radius: 8px;
  font-size: 0.9rem;
  color: var(--ts-text-primary);
}

.flw-summary-icon {
  font-size: 1.25rem;
  flex-shrink: 0;
}

.flw-summary-item--error {
  background: rgba(232, 64, 64, 0.08);
  border: 1px solid rgba(232, 64, 64, 0.2);
}

.flw-done-btn {
  display: block;
  width: 100%;
  padding: 0.9rem;
  background: linear-gradient(135deg, #6478ff, #7ddf8e);
  border: none;
  border-radius: 10px;
  color: var(--ts-text-on-accent);
  font-size: 1rem;
  font-weight: 600;
  cursor: pointer;
  transition: opacity 0.2s;
}

.flw-done-btn:hover {
  opacity: 0.9;
}

/* ── Transitions ─────────────────────────────────────────────────────────── */

.flw-backdrop-enter-active,
.flw-backdrop-leave-active {
  transition: opacity 0.3s ease;
}

.flw-backdrop-enter-from,
.flw-backdrop-leave-to {
  opacity: 0;
}

.flw-dialog-enter-active {
  transition: all 0.35s cubic-bezier(0.16, 1, 0.3, 1);
}

.flw-dialog-leave-active {
  transition: all 0.2s ease;
}

.flw-dialog-enter-from {
  opacity: 0;
  transform: scale(0.9) translateY(20px);
}

.flw-dialog-leave-to {
  opacity: 0;
  transform: scale(0.95);
}
</style>
