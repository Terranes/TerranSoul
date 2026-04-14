<template>
  <div class="chat-view">
    <!-- Full-screen character viewport — the star of the show -->
    <div class="viewport-layer">
      <CharacterViewport />
    </div>

    <!-- ── Floating overlays on top of the character ── -->

    <!-- Brain setup card (shown when no brain is configured) -->
    <Transition name="fade-up">
      <div v-if="!brain.hasBrain" class="brain-overlay">
        <div class="brain-card">
          <div class="brain-card-header">
            <span>🧠</span>
            <strong>Set up your Brain</strong>
          </div>
          <div class="brain-free-start">
            <p>Start chatting instantly with a free cloud LLM:</p>
            <button class="brain-activate-btn" @click="activateFreeApi">
              ☁️ Use Free Cloud API (no setup)
            </button>
          </div>
          <div class="brain-local-section">
            <p class="brain-hw" v-if="brain.systemInfo">
              {{ brain.systemInfo.cpu_name }} · {{ formatRam(brain.systemInfo.total_ram_mb) }} RAM
            </p>
            <p v-if="brain.topRecommendation" class="brain-rec">
              Or run locally: <strong>{{ brain.topRecommendation.display_name }}</strong>
              <br><small>{{ brain.topRecommendation.description }}</small>
            </p>
            <div v-if="brain.recommendations.length" class="brain-models">
              <button
                v-for="m in brain.recommendations"
                :key="m.model_tag"
                :class="['brain-model-btn', { selected: selectedBrain === m.model_tag, top: m.is_top_pick }]"
                @click="selectedBrain = m.model_tag"
              >
                <span>{{ m.display_name }}</span>
                <span v-if="m.is_top_pick" class="brain-star">⭐</span>
              </button>
            </div>
            <div v-if="!brain.ollamaStatus.running && brain.recommendations.length" class="brain-warn">
              ❌ Ollama not running — start it first (<code>ollama serve</code>)
              <button class="brain-retry-btn" @click="brain.checkOllamaStatus()">🔄 Retry</button>
            </div>
            <div v-else-if="brain.isPulling" class="brain-pulling">
              <div class="brain-spinner" /> Downloading…
            </div>
            <div v-else-if="brain.pullError" class="brain-warn">❌ {{ brain.pullError }}</div>
            <button
              v-if="brain.ollamaStatus.running && !brain.isPulling && selectedBrain"
              class="brain-local-btn"
              @click="activateBrain"
            >
              ⬇ Install &amp; activate {{ selectedBrain }}
            </button>
          </div>
        </div>
      </div>
    </Transition>

    <!-- Floating subtitle — shows latest AI response on the canvas -->
    <Transition name="subtitle">
      <div v-if="subtitleText" class="subtitle-overlay" :key="subtitleKey">
        <p class="subtitle-text">{{ subtitleText }}</p>
      </div>
    </Transition>

    <!-- AI state indicator pill -->
    <div class="ai-state-pill" :class="characterStore.state">
      <span class="ai-state-dot" />
      <span class="ai-state-label">{{ stateLabel }}</span>
    </div>

    <!-- Brain status (when free API active) -->
    <Transition name="fade">
      <div v-if="brain.hasBrain && brain.isFreeApiMode" class="brain-status-pill">
        <span class="brain-pill-dot" />
        <span>{{ activeProviderName }}</span>
      </div>
    </Transition>

    <!-- Floating input footer -->
    <div class="input-footer" :class="{ collapsed: !showInput }">
      <button class="footer-toggle" @click="showInput = !showInput" aria-label="Toggle input">
        <svg :class="{ flipped: !showInput }" width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M7.41 8.59L12 13.17l4.59-4.58L18 10l-6 6-6-6z"/></svg>
      </button>
      <div v-if="showInput" class="footer-content">
        <ChatInput :disabled="conversationStore.isThinking" @submit="handleSend" />
      </div>
    </div>

    <!-- Chat history drawer toggle -->
    <button
      class="chat-drawer-toggle"
      :class="{ active: showDrawer }"
      @click="showDrawer = !showDrawer"
      aria-label="Toggle chat history"
    >💬</button>

    <!-- Chat history slide-over drawer -->
    <Transition name="drawer">
      <div v-if="showDrawer" class="chat-drawer" @click.stop>
        <div class="drawer-header">
          <span class="drawer-title">Chat</span>
          <button class="drawer-close" @click="showDrawer = false" aria-label="Close chat">✕</button>
        </div>
        <ChatMessageList
          :messages="conversationStore.messages"
          :is-thinking="conversationStore.isThinking"
          :streaming-text="conversationStore.streamingText"
          :is-streaming="conversationStore.isStreaming"
          @suggest="handleSend"
        />
      </div>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue';
import { useConversationStore, detectSentiment } from '../stores/conversation';
import { useCharacterStore } from '../stores/character';
import { useBrainStore } from '../stores/brain';
import { useStreamingStore } from '../stores/streaming';
import type { CharacterState } from '../types';
import CharacterViewport from '../components/CharacterViewport.vue';
import ChatMessageList from '../components/ChatMessageList.vue';
import ChatInput from '../components/ChatInput.vue';

const conversationStore = useConversationStore();
const characterStore = useCharacterStore();
const brain = useBrainStore();
const streaming = useStreamingStore();
const showDrawer = ref(false);
const showInput = ref(true);
const selectedBrain = ref('');
/** Pre-detected emotion from user input, used during streaming for immediate feedback. */
const pendingEmotion = ref<CharacterState>('idle');
let unlistenLlmChunk: (() => void) | null = null;

// ── Subtitle system ──────────────────────────────────────────────
const MAX_SUBTITLE_LENGTH = 150;
const SUBTITLE_DURATION_MS = 8000;
const subtitleText = ref('');
const subtitleKey = ref(0);
let subtitleTimer: ReturnType<typeof setTimeout> | null = null;

function showSubtitle(text: string) {
  subtitleText.value = text.length > MAX_SUBTITLE_LENGTH ? text.slice(0, MAX_SUBTITLE_LENGTH) + '…' : text;
  subtitleKey.value++;
  if (subtitleTimer) clearTimeout(subtitleTimer);
  subtitleTimer = setTimeout(() => { subtitleText.value = ''; }, SUBTITLE_DURATION_MS);
}

// ── State label ──────────────────────────────────────────────────
const STATE_LABELS: Record<CharacterState, string> = {
  idle: 'Idle',
  thinking: 'Thinking…',
  talking: 'Talking',
  happy: 'Happy',
  sad: 'Sad',
  angry: 'Angry',
  relaxed: 'Relaxed',
  surprised: 'Surprised',
};
const stateLabel = computed(() => STATE_LABELS[characterStore.state] ?? characterStore.state);

const activeProviderName = computed(() => {
  const mode = brain.brainMode;
  if (!mode || mode.mode !== 'free_api') return '';
  const p = brain.freeProviders.find((fp) => fp.id === mode.provider_id);
  return p?.display_name ?? mode.provider_id ?? '';
});

function formatRam(mb: number): string {
  return mb >= 1024 ? `${(mb / 1024).toFixed(0)} GB` : `${mb} MB`;
}

async function activateBrain() {
  const model = selectedBrain.value;
  if (!model) return;
  const installed = brain.installedModels.some((m) => m.name === model);
  if (!installed) {
    const ok = await brain.pullModel(model);
    if (!ok) return;
  }
  await brain.setActiveBrain(model);
}

async function activateFreeApi() {
  try {
    await brain.setBrainMode({
      mode: 'free_api',
      provider_id: 'pollinations',
      api_key: null,
    });
  } catch {
    // Tauri unavailable — set locally
    brain.autoConfigureFreeApi();
  }
}

function sentimentToState(sentiment?: string): CharacterState {
  switch (sentiment) {
    case 'happy': return 'happy';
    case 'sad': return 'sad';
    case 'angry': return 'angry';
    case 'relaxed': return 'relaxed';
    case 'surprised': return 'surprised';
    default: return 'talking';
  }
}

async function handleSend(message: string) {
  // Detect emotion from user input immediately for responsive UI feedback.
  // This is stored so the streaming watcher can show the correct emotion
  // instead of generic 'talking' while the API call is in progress.
  const userSentiment = detectSentiment(message);
  pendingEmotion.value = sentimentToState(userSentiment);

  characterStore.setState('thinking');
  await conversationStore.sendMessage(message);

  const lastMsg = conversationStore.messages[conversationStore.messages.length - 1];
  const reactionState = lastMsg?.role === 'assistant'
    ? sentimentToState(lastMsg.sentiment)
    : pendingEmotion.value;

  characterStore.setState(reactionState);
  pendingEmotion.value = 'idle';

  // Show the AI's response as a floating subtitle
  if (lastMsg?.role === 'assistant') {
    showSubtitle(lastMsg.content);
  }

  setTimeout(() => characterStore.setState('idle'), 6000);
}

/** Set up Tauri event listener for llm-chunk events (streaming LLM). */
async function setupTauriEventListener() {
  try {
    const { listen } = await import('@tauri-apps/api/event');
    const unlisten = await listen<{ text: string; done: boolean }>('llm-chunk', (event) => {
      streaming.handleChunk(event.payload);
    });
    unlistenLlmChunk = unlisten;
  } catch {
    // Tauri event API not available (browser mode) — streaming handled via fetch
  }
}

watch(
  () => conversationStore.isThinking,
  (thinking) => {
    if (thinking) characterStore.setState('thinking');
  },
);

// Show detected emotion (or talking) animation during streaming
watch(
  () => conversationStore.isStreaming,
  (active) => {
    if (active) {
      // Use pre-detected emotion from user input if available,
      // otherwise fall back to generic 'talking' animation.
      characterStore.setState(pendingEmotion.value !== 'idle' ? pendingEmotion.value : 'talking');
    }
  },
);

// Update subtitle during streaming
watch(
  () => conversationStore.streamingText,
  (text) => {
    if (text) {
      showSubtitle(text);
    }
  },
);

onMounted(async () => {
  await setupTauriEventListener();

  try {
    await brain.initialise();
    if (brain.topRecommendation) {
      selectedBrain.value = brain.topRecommendation.model_tag;
    }
  } catch {
    // No Tauri backend
  }
});

onUnmounted(() => {
  if (unlistenLlmChunk) {
    unlistenLlmChunk();
    unlistenLlmChunk = null;
  }
  if (subtitleTimer) clearTimeout(subtitleTimer);
});
</script>

<style scoped>
/* ── Full-screen layout: character fills viewport, UI overlays on top ── */
.chat-view {
  position: relative;
  width: 100%;
  height: 100vh;
  height: 100dvh;
  overflow: hidden;
}

.viewport-layer {
  position: absolute;
  inset: 0;
  z-index: 0;
}

/* ── AI State Indicator — animated pill ── */
.ai-state-pill {
  position: absolute;
  top: 14px;
  right: 16px;
  z-index: 20;
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 5px 14px;
  border-radius: var(--ts-radius-pill);
  font-size: 0.72rem;
  font-weight: 700;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  background: rgba(11, 17, 32, 0.72);
  backdrop-filter: blur(12px);
  border: 1px solid rgba(255, 255, 255, 0.14);
  color: rgba(255, 255, 255, 0.88);
  transition: background 0.4s ease, color 0.4s ease, border-color 0.4s ease, box-shadow 0.4s ease;
  pointer-events: none;
  box-shadow: 0 2px 10px rgba(0, 0, 0, 0.3);
}
.ai-state-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: currentColor;
  transition: background 0.4s ease;
}
.ai-state-pill.idle { background: rgba(37, 99, 235, 0.25); color: #93c5fd; border-color: rgba(147, 197, 253, 0.3); }
.ai-state-pill.idle .ai-state-dot { background: #3b82f6; }
.ai-state-pill.thinking { background: rgba(245, 158, 11, 0.3); color: #fcd34d; border-color: rgba(253, 230, 138, 0.35); }
.ai-state-pill.thinking .ai-state-dot { background: #f59e0b; animation: pulse-dot 1.2s ease-in-out infinite; }
.ai-state-pill.talking { background: rgba(22, 163, 74, 0.25); color: #86efac; border-color: rgba(134, 239, 172, 0.3); }
.ai-state-pill.talking .ai-state-dot { background: #22c55e; }
.ai-state-pill.happy { background: rgba(8, 145, 178, 0.25); color: #67e8f9; border-color: rgba(103, 232, 249, 0.3); }
.ai-state-pill.happy .ai-state-dot { background: #06b6d4; }
.ai-state-pill.sad { background: rgba(126, 34, 206, 0.25); color: #d8b4fe; border-color: rgba(216, 180, 254, 0.3); }
.ai-state-pill.sad .ai-state-dot { background: #a855f7; }
.ai-state-pill.angry { background: rgba(239, 68, 68, 0.25); color: #fca5a5; border-color: rgba(252, 165, 165, 0.3); }
.ai-state-pill.angry .ai-state-dot { background: #ef4444; }
.ai-state-pill.relaxed { background: rgba(45, 212, 191, 0.2); color: #5eead4; border-color: rgba(94, 234, 212, 0.25); }
.ai-state-pill.relaxed .ai-state-dot { background: #14b8a6; }
.ai-state-pill.surprised { background: rgba(251, 191, 36, 0.25); color: #fde68a; border-color: rgba(253, 230, 138, 0.3); }
.ai-state-pill.surprised .ai-state-dot { background: #f59e0b; }

@keyframes pulse-dot {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.4; transform: scale(0.85); }
}

/* ── Brain status pill ── */
.brain-status-pill {
  position: absolute;
  top: 14px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 15;
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 14px;
  border-radius: var(--ts-radius-pill);
  background: rgba(22, 163, 74, 0.15);
  backdrop-filter: blur(8px);
  border: 1px solid rgba(34, 197, 94, 0.2);
  font-size: 0.7rem;
  color: #86efac;
  pointer-events: none;
}
.brain-pill-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #22c55e;
  animation: pulse-dot 2s ease-in-out infinite;
}

/* ── Floating subtitle overlay ── */
.subtitle-overlay {
  position: absolute;
  bottom: 90px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 12;
  width: 65%;
  max-width: 560px;
  pointer-events: none;
}
.subtitle-text {
  margin: 0;
  padding: 10px 20px;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(10px);
  border-radius: var(--ts-radius-lg);
  border: 1px solid rgba(255, 255, 255, 0.08);
  color: rgba(255, 255, 255, 0.92);
  font-size: 0.9rem;
  line-height: 1.55;
  text-align: center;
  text-shadow: 0 1px 3px rgba(0, 0, 0, 0.5);
}

/* Subtitle transition */
.subtitle-enter-active { transition: opacity 0.3s ease, transform 0.3s ease; }
.subtitle-leave-active { transition: opacity 0.25s ease, transform 0.25s ease; }
.subtitle-enter-from { opacity: 0; transform: translateX(-50%) translateY(8px); }
.subtitle-leave-to { opacity: 0; transform: translateX(-50%) translateY(-4px); }

/* ── Floating input footer ── */
.input-footer {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  z-index: 15;
  background: rgba(11, 17, 32, 0.82);
  backdrop-filter: blur(16px);
  border-top: 1px solid rgba(255, 255, 255, 0.10);
  transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}
.input-footer.collapsed {
  transform: translateY(calc(100% - 28px));
}
.footer-toggle {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: 28px;
  border: none;
  background: transparent;
  color: rgba(255, 255, 255, 0.5);
  cursor: pointer;
  transition: color var(--ts-transition-fast);
}
.footer-toggle:hover { color: rgba(255, 255, 255, 0.8); }
.footer-toggle svg { transition: transform 0.3s ease; }
.footer-toggle svg.flipped { transform: rotate(180deg); }
.footer-content {
  padding: 0 8px 8px;
}

/* ── Chat drawer toggle button ── */
.chat-drawer-toggle {
  position: absolute;
  bottom: 90px;
  right: 16px;
  z-index: 20;
  width: 48px;
  height: 48px;
  border-radius: 50%;
  border: 1px solid rgba(255, 255, 255, 0.18);
  background: rgba(11, 17, 32, 0.72);
  backdrop-filter: blur(10px);
  color: #fff;
  font-size: 1.3rem;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.2s ease, transform 0.2s ease, box-shadow 0.2s ease;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.35);
}
.chat-drawer-toggle:hover {
  background: rgba(124, 111, 255, 0.55);
  transform: scale(1.08);
  box-shadow: 0 4px 24px rgba(124, 111, 255, 0.3);
}
.chat-drawer-toggle.active {
  background: rgba(124, 111, 255, 0.70);
  border-color: rgba(124, 111, 255, 0.5);
}

/* ── Chat history slide-over drawer ── */
.chat-drawer {
  position: absolute;
  top: 0;
  right: 0;
  bottom: 0;
  width: 380px;
  max-width: 85vw;
  z-index: 25;
  display: flex;
  flex-direction: column;
  background: rgba(11, 17, 32, 0.94);
  backdrop-filter: blur(24px);
  border-left: 1px solid rgba(255, 255, 255, 0.10);
  box-shadow: -8px 0 48px rgba(0, 0, 0, 0.5);
}
.drawer-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 16px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  flex-shrink: 0;
}
.drawer-title {
  font-size: 0.9rem;
  font-weight: 700;
  letter-spacing: 0.03em;
  color: var(--ts-text-primary);
}
.drawer-close {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  border: none;
  background: rgba(255, 255, 255, 0.08);
  color: var(--ts-text-secondary);
  font-size: 0.75rem;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background var(--ts-transition-fast), color var(--ts-transition-fast);
}
.drawer-close:hover {
  background: rgba(255, 255, 255, 0.15);
  color: var(--ts-text-primary);
}

/* Drawer transition */
.drawer-enter-active, .drawer-leave-active { transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1), opacity 0.3s ease; }
.drawer-enter-from, .drawer-leave-to { transform: translateX(100%); opacity: 0.5; }

/* ── Fade transitions ── */
.fade-enter-active, .fade-leave-active { transition: opacity 0.3s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; }
.fade-up-enter-active, .fade-up-leave-active { transition: opacity 0.3s ease, transform 0.3s ease; }
.fade-up-enter-from { opacity: 0; transform: translateY(12px); }
.fade-up-leave-to { opacity: 0; transform: translateY(-8px); }

/* ── Brain setup overlay (centered on screen) ── */
.brain-overlay {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  z-index: 30;
  width: 340px;
  max-width: 90vw;
}
.brain-card { background: rgba(11, 17, 32, 0.94); backdrop-filter: blur(20px); border-radius: var(--ts-radius-lg); padding: 18px 20px; display: flex; flex-direction: column; gap: 10px; border: 1px solid rgba(75, 142, 255, 0.3); box-shadow: 0 12px 48px rgba(0, 0, 0, 0.6); }
.brain-card-header { display: flex; align-items: center; gap: 6px; font-size: var(--ts-text-base); }
.brain-hw { font-size: var(--ts-text-sm); color: var(--ts-text-secondary); margin: 0; }
.brain-rec { font-size: 0.8rem; color: #cbd5e1; margin: 0; line-height: 1.4; }
.brain-rec small { color: var(--ts-text-muted); }
.brain-models { display: flex; flex-wrap: wrap; gap: 4px; }
.brain-model-btn { padding: 4px 10px; border-radius: var(--ts-radius-sm); border: 1px solid var(--ts-border); background: rgba(15, 23, 42, 0.8); color: var(--ts-text-secondary); font-size: 0.75rem; cursor: pointer; display: flex; align-items: center; gap: 4px; transition: all var(--ts-transition-fast); }
.brain-model-btn.top { border-color: rgba(59, 130, 246, 0.4); }
.brain-model-btn.selected { border-color: var(--ts-success); background: rgba(26, 46, 26, 0.8); color: #86efac; }
.brain-model-btn:hover { background: rgba(30, 41, 59, 0.8); }
.brain-star { font-size: 0.7rem; }
.brain-warn { font-size: var(--ts-text-sm); color: var(--ts-warning-text); background: var(--ts-error-bg); padding: 6px 10px; border-radius: var(--ts-radius-sm); display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
.brain-warn code { background: rgba(30, 41, 59, 0.8); padding: 1px 4px; border-radius: 3px; font-size: 0.72rem; }
.brain-retry-btn { padding: 2px 8px; border: none; background: rgba(59, 130, 246, 0.3); color: #93c5fd; border-radius: 4px; cursor: pointer; font-size: 0.72rem; }
.brain-pulling { display: flex; align-items: center; gap: 6px; font-size: var(--ts-text-sm); color: var(--ts-text-secondary); }
.brain-spinner { width: 14px; height: 14px; border: 2px solid #334155; border-top-color: var(--ts-accent-blue); border-radius: 50%; animation: spin 0.8s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
.brain-activate-btn { padding: 6px 14px; border: none; background: #16a34a; color: #fff; border-radius: var(--ts-radius-sm); cursor: pointer; font-size: 0.82rem; font-weight: 500; align-self: flex-start; transition: background var(--ts-transition-fast); }
.brain-activate-btn:hover { background: #15803d; }
.brain-local-btn { padding: 6px 14px; border: none; background: var(--ts-accent-blue); color: #fff; border-radius: var(--ts-radius-sm); cursor: pointer; font-size: 0.82rem; font-weight: 500; align-self: flex-start; transition: background var(--ts-transition-fast); }
.brain-local-btn:hover { background: var(--ts-accent-blue-hover); }
.brain-free-start { display: flex; flex-direction: column; gap: 4px; }
.brain-free-start p { margin: 0; font-size: var(--ts-text-sm); color: var(--ts-text-secondary); }
.brain-local-section { border-top: 1px solid var(--ts-border-subtle); padding-top: 6px; margin-top: 2px; }

/* ── Mobile adjustments ── */
@media (max-width: 640px) {
  .chat-drawer { width: 100%; max-width: 100%; }
  .subtitle-overlay { width: 85%; bottom: 85px; }
  .ai-state-pill { right: 16px; top: 10px; }
  .brain-overlay { width: 90vw; }
}
</style>
