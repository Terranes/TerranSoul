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
            <!-- Step 1: Choose setup path -->
            <template v-if="step === 'choose'">
              <div class="flw-header">
                <span class="flw-logo">🌟</span>
                <h2 class="flw-title">
                  Welcome to TerranSoul
                </h2>
                <p class="flw-subtitle">
                  Your AI companion is ready. How would you like to get started?
                </p>
              </div>

              <div class="flw-options">
                <button
                  class="flw-option flw-option--recommended"
                  @click="step = 'quest-mode'"
                >
                  <span class="flw-option-icon">⚡</span>
                  <div class="flw-option-text">
                    <strong>Recommended Setup</strong>
                    <span class="flw-option-desc">
                      Auto-configure brain, voice, and avatar — ready to chat in seconds.
                    </span>
                  </div>
                  <span class="flw-badge">Recommended</span>
                </button>

                <button
                  class="flw-option"
                  @click="chooseManual"
                >
                  <span class="flw-option-icon">🔧</span>
                  <div class="flw-option-text">
                    <strong>Set Up From Scratch</strong>
                    <span class="flw-option-desc">
                      Explore on your own — configure each feature manually through quests.
                    </span>
                  </div>
                </button>
              </div>
            </template>

            <!-- Step 2: Quest acceptance mode -->
            <template v-if="step === 'quest-mode'">
              <div class="flw-header">
                <span class="flw-logo">⚔️</span>
                <h2 class="flw-title">
                  Quest Activation
                </h2>
                <p class="flw-subtitle">
                  TerranSoul uses a quest system to unlock features. How would you like to discover them?
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
                  <span class="flw-badge">Fastest</span>
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
              </div>

              <button
                class="flw-back"
                @click="step = 'choose'"
              >
                ← Back
              </button>
            </template>

            <!-- Step 3: Setting up (progress) -->
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
              <p class="flw-percent">{{ setupProgress }}%</p>

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

type Step = 'choose' | 'quest-mode' | 'setup' | 'done';
const step = ref<Step>('choose');
const setupMessage = ref('Preparing...');
const setupProgress = ref(0);
const setupFailed = ref(false);

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

// Foundation quest IDs to auto-accept on "recommended" path.
const FOUNDATION_QUESTS = ['free-brain', 'avatar', 'tts', 'bgm'];
// Advanced brain/memory quests that the recommended flow also activates.
const RECOMMENDED_QUESTS = ['memory', 'rag-knowledge', 'asr'];

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

function stopPullProgressListener() {
  if (listenCleanup) {
    listenCleanup();
    listenCleanup = null;
  }
}

onUnmounted(() => stopPullProgressListener());

async function chooseManual() {
  // Mark first launch as complete but don't auto-configure.
  await settingsStore.saveSettings({ first_launch_complete: true });
  emit('done');
}

async function chooseAutoAll() {
  step.value = 'setup';
  await runRecommendedSetup(true);
}

async function chooseOneByOne() {
  step.value = 'setup';
  await runRecommendedSetup(false);
}

async function runRecommendedSetup(autoAcceptAll: boolean) {
  const items: { icon: string; label: string; error?: boolean }[] = [];
  const autoConfigured: string[] = [];

  logDebug('Starting recommended setup');

  // Start listening for pull progress events.
  await startPullProgressListener();

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
          items.push({
            icon: '🧠',
            label: `Brain connected (Local — ${result.model}${pullNote})`,
          });
          logDebug(`Brain configured: local ${result.model}${pullNote}`);
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

  // Stop the pull progress listener (download phase is over).
  stopPullProgressListener();

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
    logDebug('Phase 4: Auto-accepting quests');

    const allQuests = [...FOUNDATION_QUESTS, ...RECOMMENDED_QUESTS];
    for (const questId of allQuests) {
      skillTree.markComplete(questId);
    }
    items.push({ icon: '⚔️', label: `${allQuests.length} quests auto-activated` });
    autoConfigured.push('quests');
    logDebug(`${allQuests.length} quests activated`);
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
