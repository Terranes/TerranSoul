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
            </template>

            <!-- Step 4: Done -->
            <template v-if="step === 'done'">
              <div class="flw-header">
                <span class="flw-logo">🎉</span>
                <h2 class="flw-title">
                  All Set!
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
                >
                  <span class="flw-summary-icon">{{ item.icon }}</span>
                  <span>{{ item.label }}</span>
                </div>
              </div>

              <button
                class="flw-done-btn"
                @click="$emit('done')"
              >
                Start Chatting →
              </button>
            </template>
          </div>
        </Transition>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue';
import { useBrainStore } from '../stores/brain';
import { useVoiceStore } from '../stores/voice';
import { useSkillTreeStore } from '../stores/skill-tree';
import { useSettingsStore } from '../stores/settings';

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

// Which quests got auto-completed
const completedItems = ref<{ icon: string; label: string }[]>([]);

const doneMessage = computed(() => {
  if (completedItems.value.length > 0) {
    return 'Your companion is fully configured and quests are activated. Say hi!';
  }
  return 'Your companion is ready to chat. Explore quests at your own pace!';
});

// Foundation quest IDs to auto-accept on "recommended" path.
const FOUNDATION_QUESTS = ['free-brain', 'avatar', 'tts', 'bgm'];
// Advanced brain/memory quests that the recommended flow also activates.
const RECOMMENDED_QUESTS = ['memory', 'rag-knowledge', 'asr'];

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
  const items: { icon: string; label: string }[] = [];

  // Suppress quest-unlock / combo-unlock notifications during batch setup
  // so the user isn't blasted with popups for every auto-detected feature.
  skillTree.suppressNotifications();

  // ── Phase 1: Brain (Free API) ───────────────────────────────────────
  setupMessage.value = 'Configuring AI brain (Pollinations AI)...';
  setupProgress.value = 10;

  if (!brain.hasBrain) {
    try {
      await brain.autoConfigureForDesktop();
    } catch {
      brain.autoConfigureFreeApi();
    }
  }
  items.push({ icon: '🧠', label: 'Brain connected (Pollinations AI — free, no key needed)' });
  setupProgress.value = 30;

  // ── Phase 2: Voice (TTS) ────────────────────────────────────────────
  setupMessage.value = 'Setting up voice...';
  setupProgress.value = 40;

  if (!voice.hasVoice) {
    await voice.autoConfigureVoice();
  }
  items.push({ icon: '🗣️', label: 'Voice enabled (Edge TTS neural voices)' });
  setupProgress.value = 60;

  // ── Phase 3: Skill tree initialization ──────────────────────────────
  setupMessage.value = 'Initializing quest system...';
  setupProgress.value = 70;

  await skillTree.initialise();
  items.push({ icon: '✨', label: 'Avatar loaded (3D VRM companion)' });

  // ── Phase 4: Quest activation ───────────────────────────────────────
  if (autoAcceptAll) {
    setupMessage.value = 'Activating quests...';
    setupProgress.value = 80;

    const allQuests = [...FOUNDATION_QUESTS, ...RECOMMENDED_QUESTS];
    for (const questId of allQuests) {
      skillTree.markComplete(questId);
    }
    items.push({ icon: '⚔️', label: `${allQuests.length} quests auto-activated` });
  } else {
    items.push({ icon: '📜', label: 'Quests ready — accept them one by one from the Quest tab' });
  }
  setupProgress.value = 90;

  // Resume notifications — marks all current activations + combos as "seen"
  // so only future user-driven unlocks trigger ceremonies.
  skillTree.resumeNotifications();

  // ── Phase 5: Persist ────────────────────────────────────────────────
  setupMessage.value = 'Saving configuration...';
  await settingsStore.saveSettings({ first_launch_complete: true });
  setupProgress.value = 100;

  completedItems.value = items;

  // Brief pause so user sees 100%
  await new Promise(r => setTimeout(r, 400));
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
  background: rgba(0, 0, 0, 0.7);
  backdrop-filter: blur(8px);
}

.flw-dialog {
  width: min(520px, 92vw);
  max-height: 90vh;
  overflow-y: auto;
  background: var(--ts-bg-surface, #1a1d2e);
  border: 1px solid var(--ts-border, rgba(255, 255, 255, 0.1));
  border-radius: 16px;
  padding: 2rem;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5), 0 0 40px rgba(100, 120, 255, 0.1);
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
  color: var(--ts-text-primary, #e8eaf0);
  margin: 0 0 0.5rem;
}

.flw-subtitle {
  font-size: 0.95rem;
  color: var(--ts-text-secondary, #8b8fa8);
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
  background: var(--ts-bg-card, rgba(255, 255, 255, 0.04));
  border: 1px solid var(--ts-border, rgba(255, 255, 255, 0.08));
  border-radius: 12px;
  cursor: pointer;
  text-align: left;
  color: var(--ts-text-primary, #e8eaf0);
  transition: all 0.2s ease;
  position: relative;
}

.flw-option:hover {
  background: rgba(100, 120, 255, 0.08);
  border-color: rgba(100, 120, 255, 0.3);
  transform: translateY(-1px);
}

.flw-option--recommended {
  border-color: rgba(100, 200, 120, 0.3);
  background: rgba(100, 200, 120, 0.06);
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
  color: var(--ts-text-secondary, #8b8fa8);
  line-height: 1.4;
}

.flw-badge {
  font-size: 0.7rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  padding: 0.2rem 0.5rem;
  border-radius: 6px;
  background: rgba(100, 200, 120, 0.2);
  color: #7ddf8e;
  flex-shrink: 0;
}

.flw-back {
  display: inline-block;
  margin-top: 1rem;
  padding: 0.4rem 0.8rem;
  background: none;
  border: none;
  color: var(--ts-text-secondary, #8b8fa8);
  cursor: pointer;
  font-size: 0.85rem;
}

.flw-back:hover {
  color: var(--ts-text-primary, #e8eaf0);
}

.flw-progress {
  height: 6px;
  background: var(--ts-bg-card, rgba(255, 255, 255, 0.06));
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
  background: rgba(100, 200, 120, 0.06);
  border-radius: 8px;
  font-size: 0.9rem;
  color: var(--ts-text-primary, #e8eaf0);
}

.flw-summary-icon {
  font-size: 1.25rem;
  flex-shrink: 0;
}

.flw-done-btn {
  display: block;
  width: 100%;
  padding: 0.9rem;
  background: linear-gradient(135deg, #6478ff, #7ddf8e);
  border: none;
  border-radius: 10px;
  color: #fff;
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
