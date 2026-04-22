<template>
  <div class="chat-view" :style="{ '--keyboard-offset': keyboardHeight + 'px' }">
    <!-- Full-screen character viewport — the star of the show -->
    <div class="viewport-layer">
      <CharacterViewport ref="viewportRef" @request-add-music="handleAddMusicRequest" />
      <!-- Teleport target for the music bar (left side, below settings button).
           Must live inside .viewport-layer so its z-index competes with the
           settings dropdown rather than sitting above the entire viewport. -->
      <div id="music-bar-portal" class="music-bar-portal" />
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

    <!-- Floating subtitle — shows AI response synced with TTS voice -->
    <Transition name="subtitle" mode="out-in">
      <div v-if="subtitleVisible" class="subtitle-overlay" :style="{ bottom: subtitleBottom }" :key="subtitleKey">
        <div class="subtitle-text" ref="subtitleRef" v-html="subtitleHtml"></div>
      </div>
    </Transition>

    <!-- Floating emoji popup above character head -->
    <Transition name="emoji-pop">
      <div v-if="emojiPopupVisible" class="emoji-popup" :key="emojiPopupKey">
        {{ emojiPopupText }}
      </div>
    </Transition>

    <!-- AI state indicator pill -->
    <div class="ai-state-pill" :class="characterStore.state">
      <span class="ai-state-dot" />
      <span class="ai-state-label">{{ stateLabel }}</span>
    </div>

    <!-- Brain status (shows active provider/model) -->
    <Transition name="fade">
      <div v-if="brain.hasBrain" class="brain-status-pill">
        <span class="brain-pill-dot" />
        <span>{{ activeProviderName }}</span>
      </div>
    </Transition>

    <!-- Game-dialog upgrade prompt -->
    <UpgradeDialog
      :visible="showUpgradeDialog"
      :current-model-name="currentUpgradeModel"
      :current-model-desc="currentUpgradeDesc"
      :recommended-name="recommendedUpgradeName"
      :recommended-desc="recommendedUpgradeDesc"
      :options="upgradeOptions"
      @accept="handleUpgradeAccept"
      @dismiss="showUpgradeDialog = false"
    />

    <!-- FF-style Knowledge Quest chain dialog -->
    <KnowledgeQuestDialog
      :visible="showKnowledgeQuest"
      :topic="knowledgeQuestTopic"
      @close="showKnowledgeQuest = false"
      @finish="handleKnowledgeQuestFinish"
    />

    <!-- Bottom chat panel — input always visible, history toggles via button -->
    <div class="bottom-panel" :class="{ expanded: chatDrawerExpanded }">
      <!-- Chat history (shown when expanded) -->
      <Transition name="chat-panel">
        <div v-if="chatDrawerExpanded" class="chat-history" @click.stop>
          <div class="chat-history-header">
            <span class="chat-history-title">Chat History</span>
            <button class="chat-history-close" @click="toggleChatDrawer()" aria-label="Close chat history">&times;</button>
          </div>
          <TaskProgressBar />
          <ChatMessageList
            :messages="conversationStore.messages"
            :is-thinking="conversationStore.isThinking"
            :streaming-text="conversationStore.streamingText"
            :is-streaming="conversationStore.isStreaming"
            @suggest="handleSend"
            @start-quest="handleStartQuest"
            @navigate="(target: string) => emit('navigate', target)"
          />
        </div>
      </Transition>
      <!-- Input footer — always visible -->
      <div class="input-footer">
        <!-- Quest choice strip — inline above the input row -->
        <QuestChoiceOverlay
          :choices="activeQuestChoices"
          :quest-id="activeQuestId"
          :question-text="activeQuestQuestion"
          @pick="handleQuestChoice"
          @dismiss="dismissHotseat"
        />
        <div class="input-row">
          <button
            class="chat-drawer-toggle"
            :class="{ active: chatDrawerExpanded }"
            @click="toggleChatDrawer()"
            :aria-label="chatDrawerExpanded ? 'Hide chat' : 'Show chat'"
          >
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/>
            </svg>
            <span class="toggle-label">{{ chatDrawerExpanded ? 'Hide' : 'Chat' }}</span>
          </button>
          <button
            v-if="voice.config.asr_provider"
            class="mic-btn"
            :class="{ listening: asr.isListening.value }"
            :aria-label="asr.isListening.value ? 'Stop listening' : 'Start voice input'"
            @click="toggleMic"
          >
            <svg v-if="!asr.isListening.value" width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
              <path d="M12 14c1.66 0 3-1.34 3-3V5c0-1.66-1.34-3-3-3S9 3.34 9 5v6c0 1.66 1.34 3 3 3zm5.91-3c-.49 0-.9.36-.98.85C16.52 14.2 14.47 16 12 16s-4.52-1.8-4.93-4.15c-.08-.49-.49-.85-.98-.85-.61 0-1.09.54-1 1.14.49 3 2.89 5.35 5.91 5.78V20c0 .55.45 1 1 1s1-.45 1-1v-2.08c3.02-.43 5.42-2.78 5.91-5.78.1-.6-.39-1.14-1-1.14z"/>
            </svg>
            <svg v-else width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
              <rect x="6" y="6" width="12" height="12" rx="2"/>
            </svg>
          </button>
          <ChatInput :disabled="conversationStore.isThinking" @submit="handleSend" @focus="onInputFocused" @blur="onInputBlurred" />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue';
import { useConversationStore, detectSentiment } from '../stores/conversation';
import { useCharacterStore } from '../stores/character';
import { useBrainStore } from '../stores/brain';
import { useStreamingStore } from '../stores/streaming';
import { useVoiceStore } from '../stores/voice';
import { useSettingsStore } from '../stores/settings';
import { useKeyboardDetector } from '../composables/useKeyboardDetector';
import { useTtsPlayback } from '../composables/useTtsPlayback';
import { useAsrManager } from '../composables/useAsrManager';
import { useLipSyncBridge } from '../composables/useLipSyncBridge';
import { GENDER_VOICES } from '../config/default-models';
import type { CharacterState } from '../types';
import type { AvatarStateMachine } from '../renderer/avatar-state';
import { assessCapacity, resetCapacityTracking } from '../utils/capacity-detector';
import type { UpgradeOption } from '../components/UpgradeDialog.vue';
import { useSkillTreeStore } from '../stores/skill-tree';
import { useTaskStore } from '../stores/tasks';
import { useChatExpansion } from '../composables/useChatExpansion';
import CharacterViewport from '../components/CharacterViewport.vue';
import ChatMessageList from '../components/ChatMessageList.vue';
import ChatInput from '../components/ChatInput.vue';
import TaskProgressBar from '../components/TaskProgressBar.vue';
import UpgradeDialog from '../components/UpgradeDialog.vue';
import QuestChoiceOverlay from '../components/QuestChoiceOverlay.vue';
import KnowledgeQuestDialog from '../components/KnowledgeQuestDialog.vue';

const conversationStore = useConversationStore();
const characterStore = useCharacterStore();
const brain = useBrainStore();
const streaming = useStreamingStore();
const voice = useVoiceStore();
const settingsStore = useSettingsStore();
const skillTree = useSkillTreeStore();
const { chatDrawerExpanded, toggleChatDrawer, setChatDrawerExpanded } = useChatExpansion();
const tts = useTtsPlayback({
  getBrowserPitch: () => GENDER_VOICES[characterStore.currentGender()].browserPitch,
  getBrowserRate: () => GENDER_VOICES[characterStore.currentGender()].browserRate,
});
const asr = useAsrManager({
  onTranscript: (text: string) => handleSend(text),
});
const selectedBrain = ref('');
/** Pre-detected emotion from user input, used during streaming for immediate feedback. */
const pendingEmotion = ref<CharacterState>('idle');
let unlistenLlmChunk: (() => void) | null = null;
let unlistenLlmAnimation: (() => void) | null = null;
let unlistenProvidersExhausted: (() => void) | null = null;

const viewportRef = ref<InstanceType<typeof CharacterViewport> | null>(null);

/** Access the AvatarStateMachine from the viewport (null before mount). */
function getAsm(): AvatarStateMachine | null {
  return viewportRef.value?.avatarStateMachine ?? null;
}

// LipSync ↔ TTS bridge: feeds TTS audio into LipSync → AvatarState.viseme
const lipSyncBridge = useLipSyncBridge(tts, getAsm);

/**
 * Set coarse avatar state: updates the AvatarStateMachine (read by render loop)
 * AND the characterStore (for the UI pill label). This is the single bridge point.
 */
function setAvatarState(charState: CharacterState): void {
  // When a VRMA mood animation is actively playing (e.g. angry.vrma),
  // don't let transient states like 'talking' override the emotion or
  // trigger mood watcher state changes that would kill the animation.
  const vrmaActive = viewportRef.value?.isAnimationActive ?? false;
  if (vrmaActive && (charState === 'talking' || charState === 'idle' || charState === 'thinking')) {
    return;
  }
  characterStore.setState(charState);
  const asm = getAsm();
  if (!asm) return;
  switch (charState) {
    case 'idle':      asm.forceBody('idle');  asm.setEmotion('neutral');   break;
    case 'thinking':  asm.forceBody('think'); asm.setEmotion('neutral');   break;
    case 'talking':   asm.forceBody('talk');  asm.setEmotion('neutral');   break;
    case 'happy':     asm.setEmotion('happy');     break;
    case 'sad':       asm.setEmotion('sad');       break;
    case 'angry':     asm.setEmotion('angry');     break;
    case 'relaxed':   asm.setEmotion('relaxed');   break;
    case 'surprised': asm.setEmotion('surprised'); break;
  }
}

// ── Upgrade dialog state ──────────────────────────────────────────
const showUpgradeDialog = ref(false);
/** Track the user message that triggered the upgrade suggestion. */
let lastUserQuery = '';
/** Only suggest once per session unless dismissed. */
let upgradeAlreadySuggested = false;

const currentUpgradeModel = computed(() => {
  if (brain.brainMode?.mode === 'free_api') {
    const providerId = brain.brainMode.provider_id;
    const p = brain.freeProviders.find((fp) => fp.id === providerId);
    return p?.display_name ?? 'Free Cloud API';
  }
  return 'Free Cloud API';
});
const currentUpgradeDesc = computed(() => 'Rate-limited · Basic model');
const recommendedUpgradeName = computed(() => {
  if (brain.topRecommendation) return brain.topRecommendation.display_name;
  return 'Groq (Llama 3.3 70B)';
});
const recommendedUpgradeDesc = computed(() => {
  if (brain.topRecommendation) return brain.topRecommendation.description;
  return 'Faster · Smarter · Free with API key';
});
const upgradeOptions = computed<UpgradeOption[]>(() => {
  const opts: UpgradeOption[] = [
    {
      id: 'free_upgrade',
      icon: '☁️',
      label: 'Switch to Groq (free)',
      detail: 'Better model, free tier — requires API key from groq.com',
      primary: true,
    },
  ];
  if (brain.topRecommendation) {
    opts.push({
      id: 'local',
      icon: '🖥',
      label: `Install ${brain.topRecommendation.display_name} locally`,
      detail: `Run on your machine with Ollama — ${formatRam(brain.topRecommendation.required_ram_mb)} RAM needed`,
    });
  }
  opts.push({
    id: 'paid',
    icon: '💳',
    label: 'Use paid API (OpenAI, Anthropic)',
    detail: 'Best quality — bring your own API key',
  });
  return opts;
});

// ── Keyboard detection ────────────────────────────────────────────
const { keyboardHeight, onInputFocused, onInputBlurred } = useKeyboardDetector();

// ── Knowledge Quest dialog ────────────────────────────────────────
const showKnowledgeQuest = ref(false);
const knowledgeQuestTopic = ref('');

// ── Millionaire Hot-Seat overlay — quest choices on-screen ────────
/** Whether the user explicitly dismissed the current set of choices. */
const hotseatDismissed = ref(false);
/** Track the message ID that the user last picked a choice from, so we don't
 *  immediately re-show the same set of choices when the skill-tree pushes
 *  follow-up messages in response. */
const lastPickedMessageId = ref<string | null>(null);

/** The most recent message that has quest choices (drives the overlay). */
const activeQuestMessage = computed(() => {
  if (hotseatDismissed.value) return null;
  const msgs = conversationStore.messages;
  for (let i = msgs.length - 1; i >= 0; i--) {
    if (msgs[i].questChoices?.length) return msgs[i];
  }
  return null;
});
const activeQuestChoices = computed(() => activeQuestMessage.value?.questChoices ?? []);
const activeQuestId = computed(() => activeQuestMessage.value?.questId ?? '');
const activeQuestQuestion = computed(() => {
  const msg = activeQuestMessage.value;
  if (!msg) return '';
  // Pull first line as a short question, or the whole text if short
  const first = stripMarkdownForSubtitle(msg.content).split(/[.\n]/)[0].trim();
  return first || 'What would you like to do?';
});

function dismissHotseat() {
  hotseatDismissed.value = true;
}

// Reset dismissed flag when a new quest message with choices arrives.
// Compare against the last-picked message ID so we don't re-show the exact
// same choices, but DO show follow-up choices from the same quest.
watch(() => conversationStore.messages.length, () => {
  const msgs = conversationStore.messages;
  for (let i = msgs.length - 1; i >= 0; i--) {
    if (msgs[i].questChoices?.length) {
      if (msgs[i].id !== lastPickedMessageId.value) {
        hotseatDismissed.value = false;
      }
      return;
    }
  }
});

// ── Emoji popup — floating emoji above character head ─────────────
const emojiPopupVisible = ref(false);
const emojiPopupText = ref('');
const emojiPopupKey = ref(0);
let emojiPopupTimer: ReturnType<typeof setTimeout> | null = null;
const EMOJI_POPUP_DURATION_MS = 3500;

function showEmojiPopup(emoji: string) {
  if (emojiPopupTimer) { clearTimeout(emojiPopupTimer); emojiPopupTimer = null; }
  emojiPopupText.value = emoji;
  emojiPopupVisible.value = true;
  emojiPopupKey.value++;
  emojiPopupTimer = setTimeout(() => {
    emojiPopupVisible.value = false;
    emojiPopupTimer = null;
  }, EMOJI_POPUP_DURATION_MS);
}

// ── Subtitle system — karaoke-style word highlight synced with TTS ───
const subtitleKey = ref(0);
const subtitleRef = ref<HTMLElement | null>(null);
/** Full text of the current AI response for subtitle display. */
const subtitleFullText = ref('');
/** Whether the subtitle overlay is visible. */
const subtitleVisible = ref(false);
let subtitleHideTimer: ReturnType<typeof setTimeout> | null = null;
/** Duration to keep the subtitle visible after TTS finishes. */
const SUBTITLE_LINGER_MS = 3000;

/** Dynamic bottom offset for subtitle — stays above the bottom panel. */
const subtitleBottom = computed(() => {
  // Base: input footer height (~60px) + gap
  let offset = 70;
  if (activeQuestChoices.value.length > 0) offset += 60; // quest choice strip
  if (chatDrawerExpanded.value) offset = Math.max(offset, 320); // chat history open
  return `${offset}px`;
});

/**
 * Build subtitle HTML with karaoke-style highlighting.
 * Spoken text is dimmed, the current sentence is bright/highlighted,
 * and upcoming text is at normal opacity.
 */
const subtitleHtml = computed(() => {
  const full = subtitleFullText.value;
  if (!full) return '';

  const spoken = tts.spokenText.value ?? '';
  const current = tts.currentSentence.value ?? '';

  if (!current && !spoken) {
    // Not speaking yet — show full text at normal opacity
    return escapeHtml(full);
  }

  // When spoken has text but current is empty, TTS just finished the
  // last sentence. Dim everything as "spoken" (done).
  if (!current && spoken) {
    return `<span class="subtitle-spoken">${escapeHtml(full)}</span>`;
  }

  // Find where the current sentence starts in the full text.
  // Search backwards from the end in case the sentence appears multiple times.
  let currentStart = -1;
  if (current) {
    // Try searching near the spoken-length position first
    const searchFrom = Math.max(0, spoken.length - current.length - 20);
    currentStart = full.indexOf(current, searchFrom);
    if (currentStart === -1) {
      // Fallback: search from beginning
      currentStart = full.indexOf(current);
    }
  }

  if (currentStart === -1 && current) {
    // Can't find exact match — just highlight what we can
    return escapeHtml(full);
  }

  const parts: string[] = [];

  // Spoken portion (before current sentence)
  if (currentStart > 0) {
    parts.push(`<span class="subtitle-spoken">${escapeHtml(full.slice(0, currentStart))}</span>`);
  }

  // Current sentence (highlighted)
  if (current && currentStart !== -1) {
    const currentEnd = currentStart + current.length;
    parts.push(`<span class="subtitle-active">${escapeHtml(full.slice(currentStart, currentEnd))}</span>`);

    // Upcoming text
    if (currentEnd < full.length) {
      parts.push(`<span class="subtitle-upcoming">${escapeHtml(full.slice(currentEnd))}</span>`);
    }
  }

  return parts.join('');
});

function escapeHtml(text: string): string {
  return text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

/** Strip markdown so subtitle text matches TTS-stripped sentences. */
function stripMarkdownForSubtitle(text: string): string {
  return text
    .replace(/\*\*([^*]+)\*\*/g, '$1')
    .replace(/\*([^*]+)\*/g, '$1')
    .replace(/`([^`]+)`/g, '$1')
    .replace(/\[([^\]]+)\]\([^)]+\)/g, '$1')
    .replace(/#+\s/g, '')
    .replace(/^[-*+]\s/gm, '')
    .replace(/\n{2,}/g, '. ')
    .replace(/\n/g, ' ')
    .trim();
}

/** Show the subtitle with the full response text. */
function showSubtitle(text: string) {
  if (subtitleHideTimer) { clearTimeout(subtitleHideTimer); subtitleHideTimer = null; }
  subtitleFullText.value = stripMarkdownForSubtitle(text);
  subtitleVisible.value = true;
  subtitleKey.value++;
}

/** Hide the subtitle after a linger delay. */
function scheduleSubtitleHide() {
  if (subtitleHideTimer) clearTimeout(subtitleHideTimer);
  subtitleHideTimer = setTimeout(() => {
    subtitleVisible.value = false;
    subtitleFullText.value = '';
    subtitleHideTimer = null;
  }, SUBTITLE_LINGER_MS);
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
  if (!mode) return '';
  if (mode.mode === 'free_api') {
    const p = brain.freeProviders.find((fp) => fp.id === mode.provider_id);
    return p?.display_name ?? mode.provider_id ?? '';
  }
  if (mode.mode === 'local_ollama') {
    return `Ollama · ${mode.model}`;
  }
  if (mode.mode === 'paid_api') {
    return `${mode.provider} · ${mode.model}`;
  }
  return '';
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

/** Toggle the ASR microphone on/off. */
async function toggleMic() {
  if (asr.isListening.value) {
    asr.stopListening();
  } else {
    await asr.startListening();
  }
}

/** Handle "add music" request from the floating music bar. */
function handleAddMusicRequest() {
  setChatDrawerExpanded(true);
  handleSend('Can you suggest some good background music for me?');
}

async function handleSend(message: string) {
  // Stop any ongoing TTS playback before sending a new message.
  tts.stop();

  // Store user query for capacity detection.
  lastUserQuery = message;

  // Detect emotion from user input immediately for responsive UI feedback.
  // This is stored so the streaming watcher can show the correct emotion
  // instead of generic 'talking' while the API call is in progress.
  const userSentiment = detectSentiment(message);
  pendingEmotion.value = sentimentToState(userSentiment);

  setAvatarState('thinking');
  await conversationStore.sendMessage(message);

  const lastMsg = conversationStore.messages[conversationStore.messages.length - 1];
  const reactionState = lastMsg?.role === 'assistant'
    ? sentimentToState(lastMsg.sentiment)
    : pendingEmotion.value;

  setAvatarState(reactionState);
  pendingEmotion.value = 'idle';

  // Show the AI's response as a floating subtitle
  if (lastMsg?.role === 'assistant') {
    showSubtitle(lastMsg.content);

    // Trigger VRMA body animation from the LLM's motion tag (if any).
    // For the Tauri streaming path the motion is applied live via llm-animation
    // events, but for browser-side streaming/fallback it arrives here.
    if (lastMsg.motion) {
      viewportRef.value?.playMotion(lastMsg.motion);
    }

    // Speak quest messages via TTS (they bypass LLM streaming so feedChunk is never called)
    if (lastMsg.questChoices?.length) {
      speakQuestText(lastMsg.content);
    }

    // Show emoji popup above character if the response included one
    if (lastMsg.emoji) {
      showEmojiPopup(lastMsg.emoji);
    }

    // Assess response quality — suggest upgrade if struggling
    if (brain.isFreeApiMode && !upgradeAlreadySuggested) {
      const signal = assessCapacity(lastMsg.content, lastUserQuery);
      if (signal.shouldSuggestUpgrade) {
        showUpgradeDialog.value = true;
        upgradeAlreadySuggested = true;
      }
    }
  }

  setTimeout(() => {
    // Only reset to idle if TTS is not still playing — the TTS isSpeaking
    // watcher will handle the idle transition when playback finishes.
    if (!tts.isSpeaking.value) {
      setAvatarState('idle');
      viewportRef.value?.stopMotion();
    }
  }, 6000);
}

/** Handle user accepting an upgrade option from the game dialog. */
async function handleUpgradeAccept(optionId: string) {
  showUpgradeDialog.value = false;
  resetCapacityTracking();

  if (optionId === 'free_upgrade') {
    // Switch to a better free provider (Groq)
    const groq = brain.freeProviders.find((p) => p.id === 'groq');
    if (groq) {
      try {
        await brain.setBrainMode({ mode: 'free_api', provider_id: 'groq', api_key: null });
      } catch {
        brain.autoConfigureFreeApi();
      }
      // Notify user in chat
      conversationStore.messages.push({
        id: crypto.randomUUID(),
        role: 'assistant',
        content: 'Brain upgraded! I switched to Groq for better responses. You can get a free API key at groq.com for even better performance!',
        agentName: 'TerranSoul',
        sentiment: 'happy',
        timestamp: Date.now(),
      });
    }
  } else if (optionId === 'local') {
    // Install local model via Ollama
    const model = brain.topRecommendation?.model_tag;
    if (model) {
      conversationStore.messages.push({
        id: crypto.randomUUID(),
        role: 'assistant',
        content: `Great choice! I'm downloading ${brain.topRecommendation!.display_name} now. This may take a few minutes...`,
        agentName: 'TerranSoul',
        sentiment: 'happy',
        timestamp: Date.now(),
      });
      const ok = await brain.pullModel(model);
      if (ok) {
        await brain.setActiveBrain(model);
        try {
          await brain.setBrainMode({ mode: 'local_ollama', model });
        } catch {
          // Tauri unavailable
        }
        conversationStore.messages.push({
          id: crypto.randomUUID(),
          role: 'assistant',
          content: `${brain.topRecommendation!.display_name} is installed and active! I'm much smarter now. Try asking me something complex!`,
          agentName: 'TerranSoul',
          sentiment: 'happy',
          timestamp: Date.now(),
        });
      } else {
        conversationStore.messages.push({
          id: crypto.randomUUID(),
          role: 'assistant',
          content: `Download failed: ${brain.pullError ?? 'unknown error'}. Make sure Ollama is running (ollama serve).`,
          agentName: 'TerranSoul',
          sentiment: 'sad',
          timestamp: Date.now(),
        });
      }
    }
  } else if (optionId === 'paid') {
    // Navigate to brain setup for paid API configuration
    conversationStore.messages.push({
      id: crypto.randomUUID(),
      role: 'assistant',
      content: 'To set up a paid API, open the Marketplace (🏪) and configure your API key in the LLM settings section.',
      agentName: 'TerranSoul',
      sentiment: 'happy',
      timestamp: Date.now(),
    });
  }
}

const emit = defineEmits<{ navigate: [target: string] }>();

/**
 * Speak quest/non-streamed text via TTS.
 * Quest messages are injected directly (not via LLM streaming),
 * so the normal feedChunk pipeline is bypassed. This feeds the
 * full text and flushes immediately.
 */
function speakQuestText(text: string) {
  tts.stop();
  tts.feedChunk(text);
  tts.flush();
}

/** Trigger the first available quest from the welcome screen. */
function handleStartQuest() {
  setChatDrawerExpanded(true);
  const availableQuests = skillTree.nodes.filter(n => skillTree.getSkillStatus(n.id) === 'available');
  if (availableQuests.length > 0) {
    skillTree.triggerQuestEvent(availableQuests[0].id);
    // Speak the newly injected quest message
    const lastMsg = conversationStore.messages[conversationStore.messages.length - 1];
    if (lastMsg?.role === 'assistant') {
      showSubtitle(lastMsg.content);
      speakQuestText(lastMsg.content);
    }
  }
}

/** Handle quest choice button clicks from hot-seat overlay or ChatMessageList. */
async function handleQuestChoice(questId: string, choiceValue: string) {
  // Record which message we picked from BEFORE dismissing, because
  // activeQuestMessage is computed from hotseatDismissed and returns null
  // once dismissed. Without this order, the watcher would re-show the overlay.
  lastPickedMessageId.value = activeQuestMessage.value?.id ?? null;
  hotseatDismissed.value = true;

  // Handle Knowledge Quest start
  if (choiceValue === 'knowledge-quest-start') {
    // Extract topic from the most recent user message
    const msgs = conversationStore.messages;
    let topic = 'this topic';
    for (let i = msgs.length - 1; i >= 0; i--) {
      if (msgs[i].role === 'user') {
        const lower = msgs[i].content.toLowerCase();
        const match = lower.match(/(?:learn about|teach me about|study|deep dive into|learn)\s+(.+?)(?:\.|$)/);
        if (match) topic = match[1].trim();
        break;
      }
    }
    knowledgeQuestTopic.value = topic;
    showKnowledgeQuest.value = true;
    return;
  }

  // Handle auto-configuration choices
  if (choiceValue.startsWith('auto-config:')) {
    const questIdToConfig = choiceValue.slice('auto-config:'.length);
    const node = skillTree.nodes.find(n => n.id === questIdToConfig);
    
    if (node) {
      // Auto-configure based on quest type
      if (questIdToConfig === 'gift-of-speech') {
        // Auto-configure voice/TTS
        try {
          await voice.setTtsProvider('edge-tts');
          
          // Add confirmation message
          await conversationStore.addMessage({
            id: crypto.randomUUID(),
            role: 'assistant',
            content: `Perfect! I've configured Edge TTS (free Microsoft neural voices) for you. You'll now hear my responses spoken aloud. Try sending me a message to test it!`,
            agentName: 'TerranSoul',
            sentiment: 'happy',
            timestamp: Date.now(),
          });
          
        } catch (error) {
          console.warn('Auto-config failed:', error);
          // Show error message
          await conversationStore.addMessage({
            id: crypto.randomUUID(),
            role: 'assistant',
            content: `I had trouble setting up voice automatically. You can configure it manually in the Voice tab.`,
            agentName: 'TerranSoul',
            sentiment: 'sad',
            timestamp: Date.now(),
          });
        }
      } else if (questIdToConfig === 'superior-intellect') {
        // Guide to brain setup
        emit('navigate', 'marketplace');
      } else {
        // Generic quest acceptance
        await skillTree.handleQuestChoice(questId, 'accept');
      }
      
      // Mark quest as completed if auto-config succeeded
      skillTree.triggerQuestEvent(questIdToConfig);
    }
    return;
  }

  // Handle navigation choices — emit to App.vue for tab switching
  if (choiceValue.startsWith('navigate:')) {
    const target = choiceValue.slice('navigate:'.length);
    await skillTree.handleQuestChoice(questId, choiceValue);
    emit('navigate', target);
    return;
  }

  // Auto-enable BGM when user picks "Autoplay BGM"
  if (questId === 'bgm' && choiceValue === 'bgm-autoplay') {
    viewportRef.value?.enableBgm();
  }

  await skillTree.handleQuestChoice(questId, choiceValue);

  // Speak the follow-up quest response via TTS
  const lastMsg = conversationStore.messages[conversationStore.messages.length - 1];
  if (lastMsg?.role === 'assistant') {
    showSubtitle(lastMsg.content);
    speakQuestText(lastMsg.content);
  }
}

/** Called when the Knowledge Quest chain completes — close dialog and notify. */
function handleKnowledgeQuestFinish() {
  showKnowledgeQuest.value = false;
  setChatDrawerExpanded(true);
  conversationStore.addMessage({
    id: crypto.randomUUID(),
    role: 'assistant',
    content:
      `📚 **Scholar's Quest Complete!** I've finished learning about **${knowledgeQuestTopic.value}**.\n\n` +
      `Go ahead and ask me questions — my answers will now draw from the source materials you provided!`,
    agentName: 'TerranSoul',
    sentiment: 'happy',
    timestamp: Date.now(),
  });
}

/** Set up Tauri event listeners for dual-stream LLM events. */
async function setupTauriEventListener() {
  try {
    const { listen } = await import('@tauri-apps/api/event');

    // Text stream — already clean (anim blocks stripped by Rust parser).
    const unlistenChunk = await listen<{ text: string; done: boolean }>('llm-chunk', (event) => {
      streaming.handleChunk(event.payload);

      // Feed text directly into TTS — no tag stripping needed.
      if (voice.config.tts_provider) {
        if (event.payload.done) {
          tts.flush();
        } else if (event.payload.text) {
          tts.feedChunk(event.payload.text);
        }
      }
    });
    unlistenLlmChunk = unlistenChunk;

    // Animation stream — structured JSON from Rust parser.
    const unlistenAnim = await listen<{ emotion?: string; motion?: string }>('llm-animation', (event) => {
      streaming.handleAnimation(event.payload);

      // Apply emotion to avatar immediately during streaming.
      if (event.payload.emotion) {
        const state = sentimentToState(event.payload.emotion);
        if (state !== 'idle') {
          setAvatarState(state);
        }
      }

      // Trigger VRMA body animation if the LLM specified a motion.
      if (event.payload.motion) {
        viewportRef.value?.playMotion(event.payload.motion);
      }
    });
    unlistenLlmAnimation = unlistenAnim;

    // Provider exhaustion — show upgrade quest
    const unlistenExhausted = await listen('providers-exhausted', () => {
      conversationStore.pushProviderWarning();
    });
    unlistenProvidersExhausted = unlistenExhausted;
  } catch {
    // Tauri event API not available (browser mode) — streaming handled via fetch
  }
}

watch(
  () => conversationStore.isThinking,
  (thinking) => {
    if (thinking) setAvatarState('thinking');
  },
);

// Show 'talking' animation during streaming; emotions applied from <anim> tags only
watch(
  () => conversationStore.isStreaming,
  (active) => {
    if (active) {
      // Always show 'talking' while streaming — real emotions arrive via
      // llm-animation events which call setAvatarState directly.
      setAvatarState('talking');
    } else if (streaming.currentEmotion) {
      // Stream done — set final emotion from parsed tags (once, not per-chunk)
      setAvatarState(sentimentToState(streaming.currentEmotion));
    }
  },
);

// Update subtitle text during streaming (don't re-key, just update content)
watch(
  () => conversationStore.streamingText,
  (text) => {
    if (text) {
      if (subtitleHideTimer) { clearTimeout(subtitleHideTimer); subtitleHideTimer = null; }
      subtitleFullText.value = stripMarkdownForSubtitle(text);
      subtitleVisible.value = true;
    }
  },
);

// TTS speaking state → body='talk', done → body='idle' + schedule subtitle hide
watch(tts.isSpeaking, (speaking) => {
  // Don't override state when a VRMA mood animation is active
  const vrmaActive = viewportRef.value?.isAnimationActive ?? false;
  const asm = getAsm();
  if (!asm) return;
  if (speaking) {
    if (!vrmaActive) {
      asm.forceBody('talk');
      characterStore.setState('talking');
    }
    // Keep subtitle visible while speaking
    if (subtitleHideTimer) { clearTimeout(subtitleHideTimer); subtitleHideTimer = null; }
    subtitleVisible.value = true;
  } else {
    if (!vrmaActive) {
      asm.forceBody('idle');
      characterStore.setState('idle');
    }
    // TTS finished — schedule subtitle to fade away
    if (subtitleFullText.value) {
      scheduleSubtitleHide();
    }
  }
});

// Auto-scroll subtitle to keep the highlighted sentence visible
watch(
  [() => tts.currentSentence.value, () => tts.spokenText.value],
  () => {
    nextTick(() => {
      const el = subtitleRef.value;
      if (!el) return;
      const active = el.querySelector('.subtitle-active') as HTMLElement | null;
      if (active) {
        active.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
      } else {
        // No active element — scroll to bottom (end of spoken text)
        el.scrollTop = el.scrollHeight;
      }
    });
  },
);

onMounted(async () => {
  await setupTauriEventListener();

  // Initialise background task listener
  const taskStore = useTaskStore();
  await taskStore.init();

  try {
    await brain.initialise();
    if (brain.topRecommendation) {
      selectedBrain.value = brain.topRecommendation.model_tag;
    }
  } catch {
    // No Tauri backend
  }

  try {
    await voice.initialise();
  } catch {
    // No Tauri backend — voice stays in text-only mode
  }

  // Load persisted settings (model selection, camera state).
  try {
    await settingsStore.loadSettings();
    const savedModelId = settingsStore.settings.selected_model_id;
    const defaultId = characterStore.selectedModelId;
    if (savedModelId && savedModelId !== defaultId) {
      await characterStore.selectModel(savedModelId);
    }
  } catch {
    // Settings unavailable — proceed with defaults
  }

  // Start the LipSync ↔ TTS bridge (per-frame viseme updates)
  lipSyncBridge.start();
});

onUnmounted(() => {
  if (unlistenLlmChunk) {
    unlistenLlmChunk();
    unlistenLlmChunk = null;
  }
  if (unlistenLlmAnimation) {
    unlistenLlmAnimation();
    unlistenLlmAnimation = null;
  }
  if (unlistenProvidersExhausted) {
    unlistenProvidersExhausted();
    unlistenProvidersExhausted = null;
  }
  useTaskStore().cleanup();
  if (subtitleHideTimer) clearTimeout(subtitleHideTimer);
  tts.stop();
  lipSyncBridge.dispose();
});
</script>

<style scoped>
/* ── Full-screen layout: character fills viewport, UI overlays on top ── */
.chat-view {
  position: relative;
  width: 100%;
  /* Fill the parent .app-main flex column and ensure absolute children
     (viewport, bottom-panel) reference this element's full height. */
  height: 100%;
  flex: 1 1 0%;
  min-height: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

/* The 3D viewport is always full-size and never shifts.
   overflow:hidden on .chat-view clips any keyboard-driven translate. */
.viewport-layer {
  position: absolute;
  inset: 0;
  z-index: 0;
}

/* Portal for the music bar — top-left, directly below the settings button
   (which was shifted right to clear the floating mode-toggle pill). */
.music-bar-portal {
  position: absolute;
  top: 56px;
  left: 150px;
  z-index: 16;
  pointer-events: none;
}
.music-bar-portal > * {
  pointer-events: auto;
}

/* ── AI State Indicator — animated pill ── */
.ai-state-pill {
  position: absolute;
  top: 14px;
  right: 16px;
  z-index: 20;
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 6px 16px;
  border-radius: var(--ts-radius-pill);
  font-size: 0.74rem;
  font-weight: 700;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  background: rgba(11, 17, 32, 0.78);
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

/* ── Floating subtitle overlay — karaoke-style word sync ── */
.subtitle-overlay {
  position: absolute;
  bottom: 70px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 20;
  width: 75%;
  max-width: 620px;
  pointer-events: none;
  transition: bottom 0.3s ease;
}
.subtitle-text {
  margin: 0;
  padding: 12px 20px;
  background: rgba(0, 0, 0, 0.7);
  backdrop-filter: blur(12px);
  border-radius: var(--ts-radius-lg);
  border: 1px solid rgba(255, 255, 255, 0.08);
  color: rgba(255, 255, 255, 0.92);
  font-size: 0.92rem;
  line-height: 1.6;
  text-align: center;
  text-shadow: 0 1px 3px rgba(0, 0, 0, 0.5);
  max-height: 8em;
  overflow-y: auto;
  scroll-behavior: smooth;
  pointer-events: auto;
  scrollbar-width: thin;
  scrollbar-color: rgba(255, 255, 255, 0.25) transparent;
}
.subtitle-text::-webkit-scrollbar { width: 4px; }
.subtitle-text::-webkit-scrollbar-track { background: transparent; }
.subtitle-text::-webkit-scrollbar-thumb { background: rgba(255, 255, 255, 0.25); border-radius: 2px; }
/* Spoken text — dimmed */
:deep(.subtitle-spoken) {
  color: rgba(255, 255, 255, 0.4);
  transition: color 0.3s ease;
}
/* Currently speaking sentence — bright highlight */
:deep(.subtitle-active) {
  color: #fff;
  background: rgba(124, 111, 255, 0.25);
  border-radius: 3px;
  padding: 1px 2px;
  transition: color 0.2s ease, background 0.2s ease;
}
/* Upcoming text — normal but slightly dimmed */
:deep(.subtitle-upcoming) {
  color: rgba(255, 255, 255, 0.55);
  transition: color 0.3s ease;
}

/* Subtitle transition */
.subtitle-enter-active { transition: opacity 0.3s ease, transform 0.3s ease; }
.subtitle-leave-active { transition: opacity 0.25s ease, transform 0.25s ease; }
.subtitle-enter-from { opacity: 0; transform: translateX(-50%) translateY(8px); }
.subtitle-leave-to { opacity: 0; transform: translateX(-50%) translateY(-4px); }

/* ── Floating emoji popup above character head ── */
.emoji-popup {
  position: absolute;
  top: 18%;
  left: 50%;
  transform: translateX(-50%);
  z-index: 25;
  font-size: 2.4rem;
  padding: 8px 14px;
  border-radius: 20px;
  background: rgba(11, 17, 32, 0.7);
  backdrop-filter: blur(10px);
  border: 1px solid rgba(255, 255, 255, 0.15);
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.4);
  pointer-events: none;
  line-height: 1;
  animation: emoji-float 3.5s ease-in-out;
}
@keyframes emoji-float {
  0% { transform: translateX(-50%) translateY(0) scale(0.5); opacity: 0; }
  12% { transform: translateX(-50%) translateY(-8px) scale(1.15); opacity: 1; }
  20% { transform: translateX(-50%) translateY(-4px) scale(1); opacity: 1; }
  80% { transform: translateX(-50%) translateY(-4px) scale(1); opacity: 1; }
  100% { transform: translateX(-50%) translateY(-20px) scale(0.8); opacity: 0; }
}
.emoji-pop-enter-active { transition: opacity 0.25s ease, transform 0.25s ease; }
.emoji-pop-leave-active { transition: opacity 0.3s ease, transform 0.3s ease; }
.emoji-pop-enter-from { opacity: 0; transform: translateX(-50%) scale(0.5); }
.emoji-pop-leave-to { opacity: 0; transform: translateX(-50%) translateY(-20px) scale(0.8); }

/* ── Bottom panel — input + expandable chat history ── */
.bottom-panel {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  z-index: 15;
  display: flex;
  flex-direction: column;
  justify-content: flex-end;
  max-height: 65vh;
  pointer-events: none;
  /* Slide the panel up by the keyboard height when the virtual keyboard
     is open — only the input floats up, the 3D viewport stays fixed. */
  transform: translateY(calc(-1 * var(--keyboard-offset, 0px)));
  transition: transform 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}
.bottom-panel > * { pointer-events: auto; }

/* Chat history — slides up above the input */
.chat-history {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  background: rgba(11, 17, 32, 0.92);
  backdrop-filter: blur(20px);
  border-top: 1px solid rgba(255, 255, 255, 0.10);
  scrollbar-width: thin;
  scrollbar-color: rgba(255,255,255,0.15) transparent;
  display: flex;
  flex-direction: column;
}
.chat-history-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 14px 8px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  flex-shrink: 0;
}
.chat-history-title {
  font-size: 0.72rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--ts-text-muted);
}
.chat-history-close {
  background: none;
  border: none;
  color: var(--ts-text-dim);
  font-size: 1.3rem;
  cursor: pointer;
  padding: 2px 6px;
  line-height: 1;
  border-radius: var(--ts-radius-sm);
  transition: color 0.15s, background 0.15s;
}
.chat-history-close:hover {
  color: var(--ts-text-primary);
  background: rgba(255, 255, 255, 0.1);
}

/* Chat history slide transition */
.chat-panel-enter-active { transition: max-height 0.35s cubic-bezier(0.4,0,0.2,1), opacity 0.25s ease; }
.chat-panel-leave-active { transition: max-height 0.3s cubic-bezier(0.4,0,0.2,1), opacity 0.2s ease; }
.chat-panel-enter-from, .chat-panel-leave-to { max-height: 0; opacity: 0; overflow: hidden; }
.chat-panel-enter-to, .chat-panel-leave-from { max-height: 50vh; opacity: 1; }

/* Input footer — always visible at the very bottom */
.input-footer {
  background: rgba(11, 17, 32, 0.75);
  backdrop-filter: blur(20px);
  border-top: 1px solid rgba(255, 255, 255, 0.08);
  padding: 8px 12px 10px;
}
.input-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

/* ── Chat toggle button — pill with icon + label ── */
.chat-drawer-toggle {
  height: 40px;
  padding: 0 14px;
  border-radius: var(--ts-radius-pill);
  border: 1px solid rgba(255, 255, 255, 0.15);
  background: rgba(11, 17, 32, 0.72);
  backdrop-filter: blur(10px);
  color: rgba(255, 255, 255, 0.8);
  font-size: 0.72rem;
  font-weight: 600;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
  transition: background 0.2s ease, transform 0.2s ease, box-shadow 0.2s ease, color 0.2s ease;
  box-shadow: 0 2px 10px rgba(0, 0, 0, 0.25);
}
.toggle-label {
  letter-spacing: 0.03em;
}
.chat-drawer-toggle:hover {
  background: rgba(124, 111, 255, 0.45);
  color: #fff;
  box-shadow: 0 4px 16px rgba(124, 111, 255, 0.25);
}
.chat-drawer-toggle.active {
  background: rgba(124, 111, 255, 0.65);
  border-color: rgba(124, 111, 255, 0.4);
  color: #fff;
}

/* ── Mic button — voice input toggle ── */
.mic-btn {
  width: 44px;
  height: 44px;
  border-radius: 50%;
  border: 1px solid rgba(255, 255, 255, 0.18);
  background: rgba(11, 17, 32, 0.72);
  backdrop-filter: blur(10px);
  color: #fff;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: background var(--ts-transition-normal), border-color var(--ts-transition-normal), box-shadow var(--ts-transition-fast);
}
.mic-btn:hover {
  background: rgba(255, 255, 255, 0.12);
}
.mic-btn.listening {
  background: rgba(230, 60, 80, 0.75);
  border-color: rgba(230, 60, 80, 0.5);
  box-shadow: 0 0 10px rgba(230, 60, 80, 0.4);
  animation: mic-pulse 1.5s ease-in-out infinite;
}
@keyframes mic-pulse {
  0%, 100% { box-shadow: 0 0 10px rgba(230, 60, 80, 0.4); }
  50% { box-shadow: 0 0 18px rgba(230, 60, 80, 0.7); }
}

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
  .bottom-panel { max-height: 50vh; }
  .subtitle-overlay { width: 90%; bottom: 75px; font-size: 0.82rem; }
  .subtitle-text { padding: 8px 14px; font-size: 0.82rem; }
  /* AI state pill: compact, tucked below the top-right settings gear */
  .ai-state-pill {
    top: 44px;
    right: 10px;
    padding: 2px 8px;
    font-size: 0.58rem;
    gap: 3px;
  }
  .ai-state-dot { width: 4px; height: 4px; }
  /* Brain status: below mode-toggle pill on the left */
  .brain-status-pill {
    left: 10px;
    top: 44px;
    transform: none;
    font-size: 0.58rem;
    padding: 2px 8px;
  }
  /* Music bar: below brain status */
  .music-bar-portal { top: 66px; left: 10px; }
  .brain-overlay { width: 92vw; }
  /* Compact input footer */
  .input-footer { padding: 6px 8px 8px; }
  .chat-drawer-toggle { height: 34px; padding: 0 10px; }
  .toggle-label { display: none; }
}
</style>
