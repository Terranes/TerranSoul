<template>
  <div class="chat-view" :style="{ '--keyboard-offset': keyboardHeight + 'px' }">
    <!-- Full-screen character viewport — the star of the show -->
    <div class="viewport-layer">
      <CharacterViewport ref="viewportRef" />
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
    <Transition name="subtitle">
      <div v-if="subtitleVisible" class="subtitle-overlay" :key="subtitleKey">
        <div class="subtitle-text" ref="subtitleRef" v-html="subtitleHtml"></div>
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

    <!-- Bottom chat panel — input always visible, history toggles via 💬 button -->
    <div class="bottom-panel" :class="{ expanded: chatDrawerExpanded }">
      <!-- Chat history (shown when expanded) -->
      <Transition name="chat-panel">
        <div v-if="chatDrawerExpanded" class="chat-history" @click.stop>
          <ChatMessageList
            :messages="conversationStore.messages"
            :is-thinking="conversationStore.isThinking"
            :streaming-text="conversationStore.streamingText"
            :is-streaming="conversationStore.isStreaming"
            @suggest="handleSend"
            @quest-choice="handleQuestChoice"
          />
        </div>
      </Transition>
      <!-- Input footer — always visible -->
      <div class="input-footer">
        <div class="input-row">
          <button
            class="chat-drawer-toggle"
            :class="{ active: chatDrawerExpanded }"
            @click="toggleChatDrawer()"
            aria-label="Toggle chat history"
          >💬</button>
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
import { useChatExpansion } from '../composables/useChatExpansion';
import CharacterViewport from '../components/CharacterViewport.vue';
import ChatMessageList from '../components/ChatMessageList.vue';
import ChatInput from '../components/ChatInput.vue';
import UpgradeDialog from '../components/UpgradeDialog.vue';

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

  // Find where the current sentence starts in the full text
  const spokenLen = spoken.length;
  // The spoken text may not align exactly character-by-character with fullText
  // because fullText comes from streaming while spoken is from sentence splits.
  // Use the current sentence to find a reliable anchor.
  let currentStart = -1;
  if (current) {
    // Search for the current sentence starting from after spoken portion
    const searchFrom = Math.max(0, spokenLen - 20); // small overlap for safety
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
  const dimEnd = currentStart !== -1 ? currentStart : full.length;
  if (dimEnd > 0) {
    parts.push(`<span class="subtitle-spoken">${escapeHtml(full.slice(0, dimEnd))}</span>`);
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

/** Show the subtitle with the full response text. */
function showSubtitle(text: string) {
  if (subtitleHideTimer) { clearTimeout(subtitleHideTimer); subtitleHideTimer = null; }
  subtitleFullText.value = text;
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

/** Toggle the ASR microphone on/off. */
async function toggleMic() {
  if (asr.isListening.value) {
    asr.stopListening();
  } else {
    await asr.startListening();
  }
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

/** Handle quest choice button clicks from ChatMessageList. */
async function handleQuestChoice(questId: string, choiceValue: string) {
  // Open the drawer so user sees the conversation
  setChatDrawerExpanded(true);

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
          await voice.setTtsProvider(null); // Use default voice
          
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

  await skillTree.handleQuestChoice(questId, choiceValue);
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
    });
    unlistenLlmAnimation = unlistenAnim;
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

// Show detected emotion (or talking) animation during streaming
watch(
  () => conversationStore.isStreaming,
  (active) => {
    if (active) {
      // Use pre-detected emotion from user input if available,
      // otherwise fall back to generic 'talking' animation.
      setAvatarState(pendingEmotion.value !== 'idle' ? pendingEmotion.value : 'talking');
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
      subtitleFullText.value = text;
      subtitleVisible.value = true;
    }
  },
);

// TTS speaking state → body='talk', done → body='idle' + schedule subtitle hide
watch(tts.isSpeaking, (speaking) => {
  const asm = getAsm();
  if (!asm) return;
  if (speaking) {
    asm.forceBody('talk');
    characterStore.setState('talking');
    // Keep subtitle visible while speaking
    if (subtitleHideTimer) { clearTimeout(subtitleHideTimer); subtitleHideTimer = null; }
    subtitleVisible.value = true;
  } else {
    asm.forceBody('idle');
    characterStore.setState('idle');
    // TTS finished — schedule subtitle to fade away
    if (subtitleFullText.value) {
      scheduleSubtitleHide();
    }
  }
});

// Auto-scroll subtitle to keep the highlighted sentence visible
watch(
  () => tts.currentSentence.value,
  () => {
    nextTick(() => {
      const el = subtitleRef.value;
      if (!el) return;
      const active = el.querySelector('.subtitle-active') as HTMLElement | null;
      if (active) {
        active.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
      }
    });
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
  /* Use 100% to fill the parent .app-main flex container exactly.
     100vh/100dvh would overflow on mobile where .app-main is shorter
     than the viewport (viewport − bottom nav bar height). */
  height: 100%;
  overflow: hidden;
}

/* The 3D viewport is always full-size and never shifts.
   overflow:hidden on .chat-view clips any keyboard-driven translate. */
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

/* ── Floating subtitle overlay — karaoke-style word sync ── */
.subtitle-overlay {
  position: absolute;
  bottom: 90px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 12;
  width: 75%;
  max-width: 620px;
  pointer-events: none;
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
  max-height: 5.6em;
  overflow-y: auto;
  scroll-behavior: smooth;
  scrollbar-width: none;
}
.subtitle-text::-webkit-scrollbar { display: none; }
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

/* ── Bottom panel — input + expandable chat history ── */
.bottom-panel {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  z-index: 15;
  display: flex;
  flex-direction: column;
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

/* ── Chat toggle button (💬) — inline in the input row ── */
.chat-drawer-toggle {
  width: 40px;
  height: 40px;
  border-radius: 50%;
  border: 1px solid rgba(255, 255, 255, 0.18);
  background: rgba(11, 17, 32, 0.72);
  backdrop-filter: blur(10px);
  color: #fff;
  font-size: 1.2rem;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: background 0.2s ease, transform 0.2s ease, box-shadow 0.2s ease;
  box-shadow: 0 2px 10px rgba(0, 0, 0, 0.25);
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

/* ── Mic button — voice input toggle ── */
.mic-btn {
  width: 40px;
  height: 40px;
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
  .ai-state-pill { right: 10px; top: 8px; padding: 3px 10px; font-size: 0.65rem; }
  .brain-overlay { width: 92vw; }
  /* Shift brain status pill left to avoid collision with AI state pill */
  .brain-status-pill { left: 40%; font-size: 0.62rem; padding: 3px 10px; }
  /* Compact the input footer */
  .input-footer { padding: 6px 8px 8px; }
  .chat-drawer-toggle { width: 34px; height: 34px; font-size: 1rem; }
}
</style>
