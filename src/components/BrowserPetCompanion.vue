<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import CharacterViewport from './CharacterViewport.vue';
import { useAsrManager } from '../composables/useAsrManager';
import { useTtsPlayback } from '../composables/useTtsPlayback';
import { useCharacterStore } from '../stores/character';
import { useConversationStore } from '../stores/conversation';
import { useVoiceStore } from '../stores/voice';

const conversation = useConversationStore();
const voice = useVoiceStore();
const character = useCharacterStore();

const inputText = ref('');
const messagesRef = ref<HTMLElement | null>(null);
const liveVoice = ref(false);
const asrError = ref<string | null>(null);
const ttsStreamActive = ref(false);
let spokenSentenceCount = 0;
let liveRestartTimer: ReturnType<typeof setTimeout> | null = null;

const tts = useTtsPlayback({
  getBrowserPitch: () => 1.04,
  getBrowserRate: () => 1.02,
});

const asr = useAsrManager({
  onTranscript: (text) => {
    void handleVoiceTranscript(text);
  },
  onError: (message) => {
    asrError.value = message;
    liveVoice.value = false;
  },
});

const recentMessages = computed(() => conversation.messages.slice(-8));
const translatorMode = computed(() => conversation.translatorMode);
const translatorLabel = computed(() => {
  const mode = translatorMode.value;
  if (!mode?.active) return 'Chat companion';
  return `${mode.source.name} ↔ ${mode.target.name} live translator`;
});
const statusLabel = computed(() => {
  if (asr.isListening.value) return 'Listening…';
  if (conversation.isStreaming) return 'Streaming voice…';
  if (conversation.isThinking) return 'Thinking…';
  if (tts.isSpeaking.value) return 'Speaking…';
  return 'Ready';
});

function scrollToBottom() {
  const el = messagesRef.value;
  if (el) el.scrollTop = el.scrollHeight;
}

async function ensureBrowserVoice(): Promise<void> {
  if (!voice.config.asr_provider || !voice.config.tts_provider) {
    await voice.autoConfigureVoice();
  }
}

function beginStreamingTtsIfNeeded(): void {
  if (!voice.config.tts_provider) return;
  if (!ttsStreamActive.value) {
    tts.stop();
    ttsStreamActive.value = true;
  }
}

function handleSentenceEvent(event: Event): void {
  const sentence = (event as CustomEvent<{ sentence?: string }>).detail?.sentence?.trim();
  if (!sentence || !voice.config.tts_provider) return;
  beginStreamingTtsIfNeeded();
  spokenSentenceCount += 1;
  // `useTtsPlayback` emits on sentence terminators followed by whitespace.
  tts.feedChunk(`${sentence} `);
}

watch(
  () => conversation.isStreaming,
  (streaming) => {
    if (!streaming && ttsStreamActive.value) {
      tts.flush();
      ttsStreamActive.value = false;
    }
  },
);

watch(
  () => conversation.messages.length,
  () => nextTick(scrollToBottom),
);

watch(
  () => conversation.isThinking || conversation.isStreaming || tts.isSpeaking.value,
  (busy) => {
    character.setState(busy ? (tts.isSpeaking.value ? 'talking' : 'thinking') : 'idle');
  },
);

async function submitText(text: string): Promise<void> {
  const trimmed = text.trim();
  if (!trimmed || conversation.isThinking || conversation.isStreaming) return;

  await ensureBrowserVoice();
  asrError.value = null;
  const sentenceCountBefore = spokenSentenceCount;
  tts.stop();
  ttsStreamActive.value = false;
  await conversation.sendMessage(trimmed);

  const last = conversation.messages[conversation.messages.length - 1];
  if (
    voice.config.tts_provider &&
    last?.role === 'assistant' &&
    spokenSentenceCount === sentenceCountBefore
  ) {
    // Persona fallback / non-streaming path: still talk back.
    tts.feedChunk(`${last.content}\n`);
    tts.flush();
  }
}

async function handleSend(): Promise<void> {
  const text = inputText.value;
  inputText.value = '';
  await submitText(text);
}

function scheduleLiveRestart(delay = 500): void {
  if (liveRestartTimer) clearTimeout(liveRestartTimer);
  liveRestartTimer = setTimeout(() => {
    liveRestartTimer = null;
    if (!liveVoice.value || asr.isListening.value) return;
    if (conversation.isThinking || conversation.isStreaming || tts.isSpeaking.value) {
      scheduleLiveRestart(700);
      return;
    }
    void asr.startListening();
  }, delay);
}

async function handleVoiceTranscript(text: string): Promise<void> {
  await submitText(text);
  if (liveVoice.value) scheduleLiveRestart();
}

async function toggleMic(): Promise<void> {
  await ensureBrowserVoice();
  if (asr.isListening.value) {
    asr.stopListening();
  } else {
    liveVoice.value = false;
    await asr.startListening();
  }
}

async function toggleLiveVoice(): Promise<void> {
  await ensureBrowserVoice();
  liveVoice.value = !liveVoice.value;
  if (liveVoice.value) {
    if (!asr.isListening.value) await asr.startListening();
  } else {
    asr.stopListening();
  }
}

async function startTranslatorDemo(): Promise<void> {
  liveVoice.value = false;
  await submitText('Become a translator to help me translate between English and Vietnamese.');
  liveVoice.value = true;
  scheduleLiveRestart();
}

async function stopTranslator(): Promise<void> {
  liveVoice.value = false;
  asr.stopListening();
  await submitText('stop translator mode');
}

onMounted(() => {
  window.addEventListener('ts:llm-sentence', handleSentenceEvent);
});

onBeforeUnmount(() => {
  window.removeEventListener('ts:llm-sentence', handleSentenceEvent);
  if (liveRestartTimer) clearTimeout(liveRestartTimer);
  asr.stopListening();
  tts.stop();
});
</script>

<template>
  <div class="browser-pet-companion">
    <div class="pet-frame">
      <CharacterViewport force-pet />
    </div>

    <section
      class="pet-console"
      aria-label="Talk with TerranSoul"
    >
      <header class="pet-console-head">
        <div>
          <p class="pet-console-kicker">
            {{ statusLabel }}
          </p>
          <h3>{{ translatorLabel }}</h3>
        </div>
        <button
          type="button"
          class="pet-chip"
          :class="{ active: liveVoice }"
          :aria-pressed="liveVoice"
          @click="toggleLiveVoice"
        >
          {{ liveVoice ? 'Live on' : 'Live voice' }}
        </button>
      </header>

      <div
        ref="messagesRef"
        class="pet-chat-log"
      >
        <p
          v-if="recentMessages.length === 0"
          class="pet-empty"
        >
          Type, tap mic, or start translator mode. I can answer and speak back sentence by sentence.
        </p>
        <div
          v-for="message in recentMessages"
          :key="message.id"
          :class="['pet-line', message.role]"
        >
          <span>{{ message.content }}</span>
        </div>
        <div
          v-if="conversation.isStreaming && conversation.streamingText"
          class="pet-line assistant streaming"
        >
          <span>{{ conversation.streamingText }}</span>
        </div>
      </div>

      <p
        v-if="asrError"
        class="pet-error"
      >
        {{ asrError }}
      </p>

      <div class="pet-tool-row">
        <button
          type="button"
          class="pet-tool"
          :class="{ active: asr.isListening.value }"
          @click="toggleMic"
        >
          {{ asr.isListening.value ? 'Stop mic' : 'Mic' }}
        </button>
        <button
          type="button"
          class="pet-tool"
          :class="{ active: translatorMode?.active }"
          @click="translatorMode?.active ? stopTranslator() : startTranslatorDemo()"
        >
          {{ translatorMode?.active ? 'Stop translator' : 'Translator demo' }}
        </button>
      </div>

      <form
        class="pet-input"
        @submit.prevent="handleSend"
      >
        <input
          v-model="inputText"
          type="text"
          placeholder="Talk with the pet…"
          :disabled="conversation.isThinking || conversation.isStreaming"
        >
        <button
          type="submit"
          :disabled="!inputText.trim() || conversation.isThinking || conversation.isStreaming"
        >
          Send
        </button>
      </form>
    </section>

    <p class="pet-caption">
      <span
        class="live-dot"
        aria-hidden="true"
      />
      Live VRM pet · drag to rotate · voice + translator ready
    </p>
  </div>
</template>

<style scoped>
.browser-pet-companion {
  display: grid;
  gap: var(--ts-space-md);
  justify-items: center;
  width: min(100%, 440px);
}

.pet-frame {
  position: relative;
  width: 100%;
  max-width: 380px;
  aspect-ratio: 3 / 4;
  border: 1px solid color-mix(in srgb, var(--ts-accent) 30%, var(--ts-border));
  border-radius: var(--ts-radius-xl);
  overflow: hidden;
  cursor: grab;
  background:
    radial-gradient(circle at 50% 20%, color-mix(in srgb, var(--ts-accent) 28%, transparent), transparent 65%),
    color-mix(in srgb, var(--ts-bg-panel) 75%, transparent);
  box-shadow:
    0 30px 60px -25px color-mix(in srgb, var(--ts-accent) 45%, transparent),
    0 0 0 1px color-mix(in srgb, var(--ts-accent) 20%, transparent) inset;
  backdrop-filter: blur(18px);
  -webkit-backdrop-filter: blur(18px);
}

.pet-frame:active { cursor: grabbing; }

.pet-frame :deep(canvas),
.pet-frame :deep(.viewport-wrapper) {
  width: 100% !important;
  height: 100% !important;
}

.pet-console {
  width: 100%;
  border: 1px solid color-mix(in srgb, var(--ts-accent) 28%, var(--ts-border));
  border-radius: var(--ts-radius-xl);
  padding: var(--ts-space-md);
  background:
    linear-gradient(135deg, color-mix(in srgb, var(--ts-bg-panel) 92%, transparent), color-mix(in srgb, var(--ts-bg-card) 72%, transparent));
  box-shadow:
    0 20px 50px -30px color-mix(in srgb, var(--ts-accent) 60%, transparent),
    inset 0 1px 0 color-mix(in srgb, var(--ts-text-primary) 12%, transparent);
  backdrop-filter: blur(22px) saturate(145%);
  -webkit-backdrop-filter: blur(22px) saturate(145%);
}

.pet-console-head,
.pet-tool-row,
.pet-input {
  display: flex;
  align-items: center;
  gap: var(--ts-space-sm);
}

.pet-console-head { justify-content: space-between; margin-bottom: var(--ts-space-sm); }
.pet-console-head h3 { margin: 0; font-size: 1rem; }

.pet-console-kicker {
  margin: 0 0 0.15rem;
  color: var(--ts-accent);
  font-size: 0.68rem;
  font-weight: 900;
  letter-spacing: 0.14em;
  text-transform: uppercase;
}

.pet-chat-log {
  display: grid;
  gap: 0.45rem;
  max-height: 190px;
  min-height: 104px;
  overflow-y: auto;
  padding: 0.35rem;
  border-radius: var(--ts-radius-lg);
  background: color-mix(in srgb, var(--ts-bg-input) 65%, transparent);
}

.pet-empty,
.pet-error {
  margin: 0;
  color: var(--ts-text-secondary);
  font-size: 0.85rem;
  line-height: 1.45;
}

.pet-error { color: var(--ts-warning-text); }

.pet-line {
  width: fit-content;
  max-width: 88%;
  padding: 0.55rem 0.7rem;
  border-radius: var(--ts-radius-lg);
  font-size: 0.88rem;
  line-height: 1.45;
  color: var(--ts-text-primary);
  background: color-mix(in srgb, var(--ts-bg-panel) 82%, transparent);
}

.pet-line.user {
  justify-self: end;
  color: var(--ts-text-on-accent);
  background: linear-gradient(135deg, var(--ts-accent), var(--ts-accent-violet, var(--ts-accent)));
}

.pet-line.assistant {
  justify-self: start;
  border: 1px solid color-mix(in srgb, var(--ts-accent) 20%, transparent);
}

.pet-line.streaming { box-shadow: 0 0 16px color-mix(in srgb, var(--ts-accent) 18%, transparent); }

.pet-tool-row {
  flex-wrap: wrap;
  margin: var(--ts-space-sm) 0;
}

.pet-chip,
.pet-tool,
.pet-input button {
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-pill);
  padding: 0.48rem 0.75rem;
  color: var(--ts-text-secondary);
  background: var(--ts-bg-input);
  cursor: pointer;
  font-weight: 800;
}

.pet-chip.active,
.pet-tool.active {
  color: var(--ts-text-on-accent);
  border-color: transparent;
  background: var(--ts-accent);
}

.pet-input input {
  min-width: 0;
  flex: 1;
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-pill);
  padding: 0.65rem 0.85rem;
  color: var(--ts-text-primary);
  background: var(--ts-bg-input);
}

.pet-input button:disabled,
.pet-input input:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.pet-caption {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  margin: 0;
  padding: 0.4rem 0.85rem;
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-pill);
  background: color-mix(in srgb, var(--ts-bg-panel) 80%, transparent);
  color: var(--ts-text-secondary);
  font-size: 0.78rem;
  font-weight: 700;
}

.live-dot {
  width: 0.45rem;
  height: 0.45rem;
  border-radius: 50%;
  background: var(--ts-success, #34d399);
  box-shadow: 0 0 8px var(--ts-success, #34d399);
  animation: pulse-dot 1.8s ease-in-out infinite;
}

@keyframes pulse-dot {
  0%, 100% { opacity: 0.5; transform: scale(1); }
  50% { opacity: 1; transform: scale(1.25); }
}

/* ── Mobile responsive ─────────────────────────────────────────────── */
@media (max-width: 640px) {
  .browser-pet-companion {
    width: 100%;
  }

  .pet-frame {
    max-width: min(280px, calc(100vw - 2rem));
    /* Keep aspect-ratio but cap height so the model is visible above the fold */
    max-height: 55vw;
    aspect-ratio: auto;
    height: min(calc((100vw - 2rem) * 4 / 3), 55vw);
  }

  .pet-chat-log {
    max-height: 140px;
    min-height: 80px;
  }

  .pet-console-head {
    flex-direction: column;
    align-items: flex-start;
    gap: 0.3rem;
  }
}

@media (max-width: 380px) {
  .pet-frame {
    max-width: calc(100vw - 2rem);
    height: 48vw;
  }

  .pet-chip,
  .pet-tool {
    font-size: 0.78rem;
    padding: 0.38rem 0.55rem;
  }
}
</style>
