<template>
  <div
    class="chat-view"
    :class="{ 'chatbox-only': props.chatboxMode }"
    :style="{ '--keyboard-offset': keyboardHeight + 'px' }"
  >
    <!-- Full-screen character viewport — hidden in chatbox mode -->
    <div
      v-if="!props.chatboxMode"
      class="viewport-layer"
      :class="{ 'viewport-layer--settings-priority': viewportOverlayOpen }"
    >
      <CharacterViewport
        ref="viewportRef"
        :hide-settings-dialog="chatDrawerExpanded"
        @request-add-music="handleAddMusicRequest"
        @overlay-open="handleViewportOverlayOpen"
        @set-display-mode="(mode: 'desktop' | 'chatbox') => $emit('set-display-mode', mode)"
        @toggle-pet-mode="$emit('toggle-pet-mode')"
      />
    </div>

    <!-- ── Floating overlays on top of the character ── -->

    <!-- Brain setup card (shown when no brain is configured) -->
    <Transition name="fade-up">
      <div
        v-if="showBrowserLlmPrompt && !props.chatboxMode"
        class="brain-overlay browser-llm-overlay"
      >
        <BrowserAuthPanel
          compact
          @configured="showBrowserLlmConfig = false"
        />
      </div>
      <div
        v-else-if="!usesRemoteConversation && !brain.hasBrain && !props.chatboxMode"
        class="brain-overlay"
      >
        <div class="brain-card">
          <div class="brain-card-header">
            <span>🧠</span>
            <strong>Set up your Brain</strong>
          </div>
          <div class="brain-free-start">
            <p>Start with a free-tier cloud LLM provider:</p>
            <button
              class="brain-activate-btn"
              @click="activateFreeApi"
            >
              ☁️ Use Free Cloud API
            </button>
          </div>
          <div class="brain-local-section">
            <p
              v-if="brain.systemInfo"
              class="brain-hw"
            >
              {{ brain.systemInfo?.cpu_name || 'Unknown CPU' }} · {{ formatRam(brain.systemInfo?.total_ram_mb ?? 0) }} RAM
            </p>
            <p
              v-if="brain.topRecommendation"
              class="brain-rec"
            >
              Or run locally: <strong>{{ brain.topRecommendation.display_name }}</strong>
              <br><small>{{ brain.topRecommendation.description }}</small>
            </p>
            <div
              v-if="brain.recommendations.length"
              class="brain-models"
            >
              <button
                v-for="m in brain.recommendations"
                :key="m.model_tag"
                :class="['brain-model-btn', { selected: selectedBrain === m.model_tag, top: m.is_top_pick }]"
                @click="selectedBrain = m.model_tag"
              >
                <span>{{ m.display_name }}</span>
                <span
                  v-if="m.is_top_pick"
                  class="brain-star"
                >⭐</span>
              </button>
            </div>
            <div
              v-if="!brain.ollamaStatus.running && brain.recommendations.length"
              class="brain-warn"
            >
              ❌ Ollama not running — start it first (<code>ollama serve</code>)
              <button
                class="brain-retry-btn"
                @click="brain.checkOllamaStatus()"
              >
                🔄 Retry
              </button>
            </div>
            <div
              v-else-if="brain.isPulling"
              class="brain-pulling"
            >
              <div class="brain-spinner" /> Downloading…
            </div>
            <div
              v-else-if="brain.pullError"
              class="brain-warn"
            >
              ❌ {{ brain.pullError }}
            </div>
            <button
              v-if="brain.ollamaStatus.running && !brain.isPulling && selectedBrain"
              class="brain-local-btn"
              @click="activateBrain"
            >
              ⬇ Install & activate {{ selectedBrain }}
            </button>
          </div>
        </div>
      </div>
    </Transition>

    <!-- Floating emoji popup above character head -->
    <Transition name="emoji-pop">
      <div
        v-if="emojiPopupVisible && !props.chatboxMode"
        :key="emojiPopupKey"
        class="emoji-popup"
      >
        {{ emojiPopupText }}
      </div>
    </Transition>

    <!-- In 3D mode, top-row model + state bubbles are rendered once in
         CharacterViewport's corner cluster. Chatbox mode keeps its own inline
         provider/state pills below. -->

    <!-- ═══ CHATBOX-ONLY LAYOUT ═══ -->
    <!-- Clean full-height chat when 3D viewport is hidden -->
    <div
      v-if="props.chatboxMode"
      class="chatbox-layout"
    >
      <!-- Chatbox header bar -->
      <div class="chatbox-header">
        <div class="chatbox-header-left">
          <div
            v-if="!usesRemoteConversation && !brain.hasBrain && !browserRuntime"
            class="chatbox-brain-setup"
          >
            <span>🧠</span>
            <button
              class="chatbox-brain-btn"
              @click="activateFreeApi"
            >
              Set up Brain — Use Free Cloud API
            </button>
          </div>
          <div
            v-else
            class="chatbox-provider"
          >
            <span class="brain-pill-dot" />
            <span>{{ activeProviderName || 'Choose LLM provider' }}</span>
          </div>
        </div>
        <div class="chatbox-header-right">
          <button
            v-if="browserRuntime && !usesRemoteConversation"
            type="button"
            class="chatbox-reconfigure-btn"
            @click="showBrowserLlmConfig = !showBrowserLlmConfig"
          >
            {{ showBrowserLlmConfig || !brain.browserAuthSession ? 'Choose LLM' : 'Reconfigure LLM' }}
          </button>
          <div
            class="chatbox-state-pill"
            :class="characterStore.state"
          >
            <span class="ai-state-dot" />
            <span>{{ stateLabel }}</span>
          </div>
        </div>
      </div>

      <!-- Full-height message list -->
      <div class="chatbox-messages">
        <BrowserAuthPanel
          v-if="showBrowserLlmPrompt"
          compact
          class="chat-llm-auth"
          @configured="showBrowserLlmConfig = false"
        />
        <AgentThreadPicker
          :messages="conversationStore.messages"
          :current-agent="conversationStore.currentAgent"
          @pick="(id: string) => conversationStore.setAgent(id)"
        />
        <TaskProgressBar />
        <ChatMessageList
          :messages="conversationStore.agentMessages"
          :is-thinking="conversationStore.isThinking"
          :streaming-text="conversationStore.streamingText"
          :is-streaming="conversationStore.isStreaming"
          :streaming-thinking-text="streaming.thinkingText"
          :is-thinking-phase="streaming.isThinkingPhase"
          @suggest="handleSend"
          @start-quest="handleStartQuest"
          @navigate="(target: string) => emit('navigate', target)"
          @rate-charisma-turn="handleCharismaTurnRating"
        />
      </div>

      <!-- Input footer -->
      <div class="chatbox-footer">
        <QuestChoiceOverlay
          :choices="activeQuestChoices"
          :quest-id="activeQuestId"
          :question-text="activeQuestQuestion"
          @pick="handleQuestChoice"
          @dismiss="dismissHotseat"
        />
        <TaskControls
          :visible="showTaskControls"
          :queue-count="conversationStore.messageQueue.length"
          @stop="conversationStore.stopGeneration()"
          @stop-and-send="conversationStore.stopAndSend()"
          @add-to-queue="(msg: string) => conversationStore.addToQueue(msg)"
          @steer="(msg: string) => conversationStore.steerWithMessage(msg)"
        />
        <div class="input-top-left-controls">
          <select
            class="reasoning-effort-select"
            :value="reasoningEffortUiValue"
            :title="brain.hasBrain ? `Reasoning effort: ${reasoningEffortUiValue}` : 'Configure Brain to enable reasoning controls'"
            :disabled="!brain.hasBrain"
            @change="handleReasoningEffortChange"
          >
            <option value="off">
              💬 Instant
            </option>
            <option value="medium">
              ⚖ Balanced
            </option>
            <option value="high">
              🧠 Deep
            </option>
          </select>
        </div>
        <div class="input-row">
          <button
            v-if="voice.config.asr_provider"
            class="mic-btn"
            :class="{ listening: asr.isListening.value }"
            :aria-label="asr.isListening.value ? 'Stop listening' : 'Start voice input'"
            @click="toggleMic"
          >
            <svg
              v-if="!asr.isListening.value"
              width="18"
              height="18"
              viewBox="0 0 24 24"
              fill="currentColor"
            >
              <path d="M12 14c1.66 0 3-1.34 3-3V5c0-1.66-1.34-3-3-3S9 3.34 9 5v6c0 1.66 1.34 3 3 3zm5.91-3c-.49 0-.9.36-.98.85C16.52 14.2 14.47 16 12 16s-4.52-1.8-4.93-4.15c-.08-.49-.49-.85-.98-.85-.61 0-1.09.54-1 1.14.49 3 2.89 5.35 5.91 5.78V20c0 .55.45 1 1 1s1-.45 1-1v-2.08c3.02-.43 5.42-2.78 5.91-5.78.1-.6-.39-1.14-1-1.14z" />
            </svg>
            <svg
              v-else
              width="18"
              height="18"
              viewBox="0 0 24 24"
              fill="currentColor"
            >
              <rect
                x="6"
                y="6"
                width="12"
                height="12"
                rx="2"
              />
            </svg>
          </button>
          <ChatInput
            :disabled="conversationStore.isThinking"
            :thinking="conversationStore.isThinking || streaming.isThinkingPhase"
            @submit="handleSend"
            @focus="onInputFocused"
            @blur="onInputBlurred"
          />
        </div>
      </div>
    </div>
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

    <!-- Bottom chat panel — input always visible, history toggles via button (3D mode only) -->
    <div
      v-if="!props.chatboxMode"
      class="bottom-panel"
      :class="{ expanded: chatDrawerExpanded }"
    >
      <!-- Chat history (shown when expanded) -->
      <Transition name="chat-panel">
        <div
          v-if="chatDrawerExpanded"
          class="chat-history"
          @click.stop
        >
          <div class="chat-history-header">
            <div class="chat-history-controls">
              <button
                class="chat-drawer-toggle active"
                aria-label="Hide chat"
                @click="toggleChatDrawer()"
              >
                <svg
                  width="16"
                  height="16"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                >
                  <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
                </svg>
                <span class="toggle-label">Hide</span>
              </button>
              <select
                v-if="brain.hasBrain"
                class="reasoning-effort-select"
                :value="reasoningEffortUiValue"
                :title="`Reasoning effort: ${reasoningEffortUiValue}`"
                @change="handleReasoningEffortChange"
              >
                <option value="off">
                  💬 Instant
                </option>
                <option value="medium">
                  ⚖ Balanced
                </option>
                <option value="high">
                  🧠 Deep
                </option>
              </select>
              <span
                v-if="activeProviderName"
                class="chat-context-pill"
                :title="activeProviderName"
              >
                {{ activeProviderName }}
              </span>
            </div>
            <div class="chat-history-actions">
              <button
                class="chat-history-action-btn"
                aria-label="Copy chat history"
                @click="copyChatHistoryToClipboard"
              >
                Copy
              </button>
              <button
                class="chat-history-action-btn"
                aria-label="Paste clipboard as message"
                @click="pasteClipboardAsMessage"
              >
                Paste
              </button>
              <button
                v-if="canSkipDialog"
                class="chat-history-action-btn skip"
                aria-label="Skip dialog and TTS"
                @click="skipCurrentDialog"
              >
                Skip
              </button>
            </div>
          </div>
          <TaskProgressBar />
          <AgentThreadPicker
            v-if="!showBrowserLlmPrompt"
            :messages="conversationStore.messages"
            :current-agent="conversationStore.currentAgent"
            @pick="(id: string) => conversationStore.setAgent(id)"
          />
          <BrowserAuthPanel
            v-if="showBrowserLlmPrompt"
            compact
            class="chat-llm-auth"
            @configured="showBrowserLlmConfig = false"
          />
          <ChatMessageList
            :messages="conversationStore.agentMessages"
            :is-thinking="conversationStore.isThinking"
            :streaming-text="conversationStore.streamingText"
            :is-streaming="conversationStore.isStreaming"
            :streaming-thinking-text="streaming.thinkingText"
            :is-thinking-phase="streaming.isThinkingPhase"
            @suggest="handleSend"
            @start-quest="handleStartQuest"
            @navigate="(target: string) => emit('navigate', target)"
            @rate-charisma-turn="handleCharismaTurnRating"
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
        <!-- Long-running task controls — shown while AI is thinking/streaming -->
        <TaskControls
          :visible="showTaskControls"
          :queue-count="conversationStore.messageQueue.length"
          @stop="conversationStore.stopGeneration()"
          @stop-and-send="conversationStore.stopAndSend()"
          @add-to-queue="(msg: string) => conversationStore.addToQueue(msg)"
          @steer="(msg: string) => conversationStore.steerWithMessage(msg)"
        />
        <!-- Karaoke subtitle — inline above the chat toggle group, full width -->
        <Transition
          name="subtitle"
          mode="out-in"
        >
          <div
            v-if="karaokeDialogEnabled && subtitleVisible && !chatDrawerExpanded"
            :key="subtitleKey"
            class="subtitle-inline"
          >
            <div
              ref="subtitleRef"
              class="subtitle-text"
            >
              <span
                v-for="(segment, index) in subtitleSegments"
                :key="index"
                :class="segment.className"
              >{{ segment.text }}</span>
            </div>
          </div>
        </Transition>
        <div
          v-if="!chatDrawerExpanded"
          class="input-top-left-controls"
        >
          <button
            class="chat-drawer-toggle"
            :class="{ active: chatDrawerExpanded }"
            :aria-label="chatDrawerExpanded ? 'Hide chat' : 'Show chat'"
            @click="toggleChatDrawer()"
          >
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
            </svg>
            <span class="toggle-label">{{ chatDrawerExpanded ? 'Hide' : 'Chat' }}</span>
          </button>
          <select
            v-if="brain.hasBrain"
            class="reasoning-effort-select"
            :value="reasoningEffortUiValue"
            :title="`Reasoning effort: ${reasoningEffortUiValue}`"
            @change="handleReasoningEffortChange"
          >
            <option value="off">
              💬 Instant
            </option>
            <option value="medium">
              ⚖ Balanced
            </option>
            <option value="high">
              🧠 Deep
            </option>
          </select>
          <span
            v-if="activeProviderName"
            class="chat-context-pill"
            :title="activeProviderName"
          >
            {{ activeProviderName }}
          </span>
        </div>
        <div class="input-row">
          <button
            v-if="voice.config.asr_provider"
            class="mic-btn"
            :class="{ listening: asr.isListening.value }"
            :aria-label="asr.isListening.value ? 'Stop listening' : 'Start voice input'"
            @click="toggleMic"
          >
            <svg
              v-if="!asr.isListening.value"
              width="18"
              height="18"
              viewBox="0 0 24 24"
              fill="currentColor"
            >
              <path d="M12 14c1.66 0 3-1.34 3-3V5c0-1.66-1.34-3-3-3S9 3.34 9 5v6c0 1.66 1.34 3 3 3zm5.91-3c-.49 0-.9.36-.98.85C16.52 14.2 14.47 16 12 16s-4.52-1.8-4.93-4.15c-.08-.49-.49-.85-.98-.85-.61 0-1.09.54-1 1.14.49 3 2.89 5.35 5.91 5.78V20c0 .55.45 1 1 1s1-.45 1-1v-2.08c3.02-.43 5.42-2.78 5.91-5.78.1-.6-.39-1.14-1-1.14z" />
            </svg>
            <svg
              v-else
              width="18"
              height="18"
              viewBox="0 0 24 24"
              fill="currentColor"
            >
              <rect
                x="6"
                y="6"
                width="12"
                height="12"
                rx="2"
              />
            </svg>
          </button>
          <ChatInput
            :disabled="conversationStore.isThinking"
            :thinking="conversationStore.isThinking || streaming.isThinkingPhase"
            @submit="handleSend"
            @focus="onInputFocused"
            @blur="onInputBlurred"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import { computed } from 'vue';
import { watch } from 'vue';
import { onMounted } from 'vue';
import { onUnmounted } from 'vue';
import { nextTick } from 'vue';
import { storeToRefs } from 'pinia';
import { invoke } from '@tauri-apps/api/core';
import { detectSentiment, handleLearnDocsChoice, handleModelUpdateChoice } from '../stores/conversation';
import { shouldUseRemoteChatStore, useChatConversationStore } from '../stores/chat-store-router';
import { loadBrowserLanHost } from '../utils/browser-lan';
import { useCharacterStore } from '../stores/character';
import { useBrainStore } from '../stores/brain';
import { useAiDecisionPolicyStore } from '../stores/ai-decision-policy';
import { useStreamingStore } from '../stores/streaming';
import { useVoiceStore } from '../stores/voice';
import { useSettingsStore } from '../stores/settings';
import { useAudioStore } from '../stores/audio';
import { useKeyboardDetector } from '../composables/useKeyboardDetector';
import { useTtsPlayback } from '../composables/useTtsPlayback';
import { useAsrManager } from '../composables/useAsrManager';
import { useLipSyncBridge } from '../composables/useLipSyncBridge';
import { GENDER_VOICES } from '../config/default-models';
import type { CharacterState } from '../types';
import type { MemoryEdge, MemoryEntry } from '../types';
import type { AvatarStateMachine } from '../renderer/avatar-state';
import { assessCapacity, resetCapacityTracking } from '../utils/capacity-detector';
import { copyChatHistory, readClipboardText } from '../utils/chat-history-clipboard';
import { BRAIN_WIKI_HELP_TEXT, parseBrainWikiSlashCommand } from '../utils/slash-commands';
import type { UpgradeOption } from '../components/UpgradeDialog.vue';
import { useSkillTreeStore } from '../stores/skill-tree';
import { useTaskStore } from '../stores/tasks';
import { useChatExpansion } from '../composables/useChatExpansion';
import { usePluginSlashDispatch } from '../composables/usePluginSlashDispatch';
import { usePromptCommandDispatch } from '../composables/usePromptCommandDispatch';
import { usePromptCommandsStore } from '../stores/prompt-commands';
import CharacterViewport from '../components/CharacterViewport.vue';
import ChatMessageList from '../components/ChatMessageList.vue';
import ChatInput from '../components/ChatInput.vue';
import TaskProgressBar from '../components/TaskProgressBar.vue';
import TaskControls from '../components/TaskControls.vue';
import AgentThreadPicker from '../components/AgentThreadPicker.vue';
import UpgradeDialog from '../components/UpgradeDialog.vue';
import QuestChoiceOverlay from '../components/QuestChoiceOverlay.vue';
import KnowledgeQuestDialog from '../components/KnowledgeQuestDialog.vue';
import BrowserAuthPanel from '../components/BrowserAuthPanel.vue';

const usesRemoteConversation = shouldUseRemoteChatStore();
const conversationStore = useChatConversationStore();
const characterStore = useCharacterStore();
const brain = useBrainStore();
const aiDecisionPolicy = useAiDecisionPolicyStore().policy;
const streaming = useStreamingStore();
const voice = useVoiceStore();
const settingsStore = useSettingsStore();
const audioStore = useAudioStore();
const { muted: audioMuted } = storeToRefs(audioStore);
const skillTree = useSkillTreeStore();
const { chatDrawerExpanded, toggleChatDrawer, setChatDrawerExpanded } = useChatExpansion();
const { tryDispatchSlashCommand } = usePluginSlashDispatch();
const { tryDispatchPromptCommand } = usePromptCommandDispatch();
const promptCommandsStore = usePromptCommandsStore();
const tts = useTtsPlayback({
  getBrowserPitch: () => GENDER_VOICES[characterStore.currentGender()].browserPitch,
  getBrowserRate: () => GENDER_VOICES[characterStore.currentGender()].browserRate,
  mutedRef: audioMuted,
});
const asr = useAsrManager({
  onTranscript: (text: string) => handleSend(text),
});
const selectedBrain = ref('');
const browserRuntime = computed(() => typeof window !== 'undefined' && !('__TAURI_INTERNALS__' in window));
const showBrowserLlmConfig = ref(false);
const showBrowserLlmPrompt = computed(() =>
  browserRuntime.value && !usesRemoteConversation && (!brain.hasBrain || showBrowserLlmConfig.value),
);

interface SessionReflectionReport {
  facts_saved: number;
  summary: string;
  reflection_id: number;
  source_turn_count: number;
  derived_edge_count: number;
}

interface SourceDedupResult {
  kind: 'skipped' | 'ingested';
  existing_id?: number;
  entry_id?: number;
}

interface IngestStartResult {
  task_id: string;
  source: string;
  source_type: string;
}

interface BrainWikiAuditReport {
  open_conflicts: unknown[];
  orphan_ids: number[];
  stale_ids: number[];
  pending_embeddings: number;
  total_memories: number;
  total_edges: number;
  generated_at: number;
}

interface BrainWikiGodNode {
  entry: MemoryEntry;
  degree: number;
}

interface BrainWikiSurprisingConnection {
  edge: MemoryEdge;
  src: MemoryEntry;
  dst: MemoryEntry;
  label: string;
}

interface BrainWikiReviewItem {
  entry: MemoryEntry;
  gravity: number;
}
/** Pre-detected emotion from user input, used during streaming for immediate feedback. */
const pendingEmotion = ref<CharacterState>('idle');
let unlistenLlmChunk: (() => void) | null = null;
let unlistenLlmAnimation: (() => void) | null = null;
let unlistenProvidersExhausted: (() => void) | null = null;
let isStreamTtsActive = false;

const viewportRef = ref<InstanceType<typeof CharacterViewport> | null>(null);
const viewportOverlayOpen = ref(false);

function handleViewportOverlayOpen(open: boolean): void {
  viewportOverlayOpen.value = open;
}

/** Access the AvatarStateMachine from the viewport (null before mount). */
function getAsm(): AvatarStateMachine | null {
  return viewportRef.value?.avatarStateMachine ?? null;
}

// LipSync ↔ TTS bridge: feeds TTS audio into LipSync → AvatarState.viseme
const lipSyncBridge = useLipSyncBridge(tts, getAsm);

type BrowserSentenceDetail = { sentence?: string; language?: string };

function isBrowserSentenceEvent(event: Event): event is CustomEvent<BrowserSentenceDetail> {
  return event instanceof CustomEvent;
}

function handleBrowserSentenceEvent(event: Event) {
  if (!isBrowserSentenceEvent(event)) return;
  const detail = event.detail;
  const sentence = detail?.sentence?.trim();
  if (!sentence || !voice.config.tts_provider) return;
  if (!isStreamTtsActive) {
    tts.stop();
    isStreamTtsActive = true;
  }
  // Add trailing whitespace so useTtsPlayback's sentence detector flushes
  // browser-direct sentence events immediately instead of waiting for done.
  tts.feedChunk(`${sentence} `, { language: detail?.language });
}

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
    case 'idle':      asm.forceBody('idle');  asm.setEmotion('neutral');                                    break;
    case 'thinking':  asm.forceBody('think'); asm.setEmotion('neutral');                                    break;
    case 'talking':   asm.forceBody('talk');  asm.setEmotion('neutral');                                    break;
    case 'happy':     asm.setEmotion('happy',     characterStore.emotionIntensity); break;
    case 'sad':       asm.setEmotion('sad',       characterStore.emotionIntensity); break;
    case 'angry':     asm.setEmotion('angry',     characterStore.emotionIntensity); break;
    case 'relaxed':   asm.setEmotion('relaxed',   characterStore.emotionIntensity); break;
    case 'surprised': asm.setEmotion('surprised', characterStore.emotionIntensity); break;
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
// Also trigger karaoke + TTS for quest messages pushed directly (not via LLM stream).
let prevMessageCount = 0;
watch(() => conversationStore.messages.length, (count) => {
  const msgs = conversationStore.messages;
  for (let i = msgs.length - 1; i >= 0; i--) {
    if (msgs[i].questChoices?.length) {
      if (msgs[i].id !== lastPickedMessageId.value) {
        hotseatDismissed.value = false;
      }
      break;
    }
  }

  // When a new assistant message arrives outside of LLM streaming (e.g. quest
  // flow pushes from startLearnDocsFlow, pushTeachScholarQuestForTopic),
  // trigger karaoke subtitle + TTS so the user hears and sees it.
  // Also fires when the background intent classifier aborts a stream and
  // pushes a quest message mid-generation — the quest text replaces whatever
  // partial LLM text was being spoken.
  if (count > prevMessageCount) {
    const last = msgs[msgs.length - 1];
    if (last?.role === 'assistant' && last.questChoices?.length) {
      showSubtitle(last.content);
      speakQuestText(last.content);
    }
  }
  prevMessageCount = count;
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

// ── TaskControls linger — keep stop/queue visible briefly after streaming ends ──
const taskControlsLinger = ref(false);
let taskControlsLingerTimer: ReturnType<typeof setTimeout> | null = null;
const TASK_CONTROLS_LINGER_MS = 2000;
const showTaskControls = computed(() =>
  conversationStore.isThinking || conversationStore.isStreaming || taskControlsLinger.value,
);
watch(
  () => conversationStore.isThinking || conversationStore.isStreaming,
  (active) => {
    if (taskControlsLingerTimer) { clearTimeout(taskControlsLingerTimer); taskControlsLingerTimer = null; }
    if (!active) {
      taskControlsLinger.value = true;
      taskControlsLingerTimer = setTimeout(() => { taskControlsLinger.value = false; }, TASK_CONTROLS_LINGER_MS);
    }
  },
);

// ── Subtitle system — karaoke-style word highlight synced with TTS ───
const subtitleKey = ref(0);
const subtitleRef = ref<HTMLElement | null>(null);
/** Full text of the current AI response for subtitle display. */
const subtitleFullText = ref('');
/** Whether the subtitle overlay is visible. */
const subtitleVisible = ref(false);
let subtitleHideTimer: ReturnType<typeof setTimeout> | null = null;
/** Duration to keep the subtitle visible after TTS finishes. */
const SUBTITLE_LINGER_MS = 8000;

const karaokeDialogEnabled = computed(() => settingsStore.settings.karaoke_dialog_enabled !== false);

type SubtitleSegment = { text: string; className?: string };

/** Build karaoke-style subtitle segments without injecting HTML. */
const subtitleSegments = computed<SubtitleSegment[]>(() => {
  const full = subtitleFullText.value;
  if (!full) return [];

  const spoken = tts.spokenText.value ?? '';
  const current = tts.currentSentence.value ?? '';

  if (!current && !spoken) {
    return [{ text: full }];
  }

  if (!current && spoken) {
    return [{ text: full, className: 'subtitle-spoken' }];
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
    return [{ text: full }];
  }

  const parts: SubtitleSegment[] = [];

  if (currentStart > 0) {
    parts.push({ text: full.slice(0, currentStart), className: 'subtitle-spoken' });
  }

  if (current && currentStart !== -1) {
    const currentEnd = currentStart + current.length;
    parts.push({ text: full.slice(currentStart, currentEnd), className: 'subtitle-active' });

    if (currentEnd < full.length) {
      parts.push({ text: full.slice(currentEnd), className: 'subtitle-upcoming' });
    }
  }

  return parts;
});

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
  if (!karaokeDialogEnabled.value) return;
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
  if (usesRemoteConversation) {
    const browserLanHost = loadBrowserLanHost();
    return browserLanHost ? `LAN Desktop · ${browserLanHost.host}:${browserLanHost.port}` : 'Remote Desktop';
  }
  if (browserRuntime.value && !brain.browserAuthSession) return '';
  const mode = brain.brainMode;
  if (!mode) return '';
  if (mode.mode === 'free_api') {
    const p = brain.freeProviders.find((fp) => fp.id === mode.provider_id);
    return p?.display_name ?? mode.provider_id ?? '';
  }
  if (mode.mode === 'local_ollama') {
    return `Ollama · ${mode.model}`;
  }
  if (mode.mode === 'local_lm_studio') {
    return `LM Studio · ${mode.model}`;
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

function generateMessageId(): string {
  return crypto.randomUUID();
}

function addUserChatMessage(content: string): void {
  conversationStore.addMessage({
    id: generateMessageId(),
    role: 'user',
    content,
    timestamp: Date.now(),
  });
}

function addTerranSoulChatMessage(content: string, sentiment: 'neutral' | 'sad' = 'neutral'): void {
  conversationStore.addMessage({
    id: generateMessageId(),
    role: 'assistant',
    content,
    agentName: 'TerranSoul',
    sentiment,
    timestamp: Date.now(),
  });
}

function compactMemory(entry: MemoryEntry, max = 120): string {
  const text = entry.content.replace(/\s+/g, ' ').trim();
  return text.length > max ? `${text.slice(0, max - 1)}...` : text;
}

function formatGodNodes(nodes: BrainWikiGodNode[]): string {
  if (nodes.length === 0) return 'No connected memories found yet. Run edge extraction or add linked memories first.';
  return [
    'Spotlight memories:',
    ...nodes.map((node, index) =>
      `${index + 1}. #${node.entry.id} (${node.degree} edge${node.degree === 1 ? '' : 's'}) ${compactMemory(node.entry)}`,
    ),
  ].join('\n');
}

function formatSurprises(items: BrainWikiSurprisingConnection[]): string {
  if (items.length === 0) return 'No cross-topic connections found yet. Detect communities after adding more linked memories.';
  return [
    'Serendipity links:',
    ...items.map((item, index) =>
      `${index + 1}. #${item.src.id} -> #${item.dst.id} (${item.edge.rel_type}, ${Math.round(item.edge.confidence * 100)}%, ${item.label})\n   ${compactMemory(item.src, 72)}\n   ${compactMemory(item.dst, 72)}`,
    ),
  ].join('\n');
}

function formatReviewQueue(items: BrainWikiReviewItem[]): string {
  if (items.length === 0) return 'No memories are ready for review right now.';
  return [
    'Revisit queue:',
    ...items.map((item, index) =>
      `${index + 1}. #${item.entry.id} (gravity ${item.gravity.toFixed(2)}) ${compactMemory(item.entry)}`,
    ),
  ].join('\n');
}

function looksLikeIngestSource(value: string): boolean {
  return /^(https?:\/\/|crawl:)/i.test(value) || /^[a-zA-Z]:[\\/]/.test(value) || value.startsWith('\\\\');
}

async function handleBrainWikiSlashCommand(message: string): Promise<boolean> {
  const command = parseBrainWikiSlashCommand(message);
  if (!command) return false;

  addUserChatMessage(message);

  try {
    switch (command.kind) {
      case 'digest': {
        if (!command.arg) {
          addTerranSoulChatMessage(`Usage:\n${BRAIN_WIKI_HELP_TEXT}`);
          return true;
        }
        if (looksLikeIngestSource(command.arg)) {
          const result = await invoke<IngestStartResult>('ingest_document', {
            source: command.arg,
            tags: 'wiki:digest,imported',
            importance: 4,
          });
          addTerranSoulChatMessage(
            `Digest started for ${result.source_type}: ${result.source}\nTask: ${result.task_id}`,
          );
          return true;
        }
        const result = await invoke<SourceDedupResult>('brain_wiki_digest_text', {
          content: command.arg,
          sourceUrl: null,
          tags: 'wiki:digest,chat',
          importance: 4,
        });
        const id = result.kind === 'skipped' ? result.existing_id : result.entry_id;
        addTerranSoulChatMessage(
          result.kind === 'skipped'
            ? `That source note is already remembered as #${id}.`
            : `Digest saved as memory #${id}.`,
        );
        return true;
      }
      case 'ponder': {
        const report = await invoke<BrainWikiAuditReport>('brain_wiki_audit', { limit: 50 });
        addTerranSoulChatMessage([
          'Brain wiki audit complete.',
          `Memories: ${report.total_memories}`,
          `Live edges: ${report.total_edges}`,
          `Open conflicts: ${report.open_conflicts.length}`,
          `Orphans: ${report.orphan_ids.length}`,
          `Stale review candidates: ${report.stale_ids.length}`,
          `Embedding queue: ${report.pending_embeddings}`,
        ].join('\n'));
        return true;
      }
      case 'spotlight': {
        const nodes = await invoke<BrainWikiGodNode[]>('brain_wiki_spotlight', { limit: 10 });
        addTerranSoulChatMessage(formatGodNodes(nodes));
        return true;
      }
      case 'serendipity': {
        const items = await invoke<BrainWikiSurprisingConnection[]>('brain_wiki_serendipity', { limit: 10 });
        addTerranSoulChatMessage(formatSurprises(items));
        return true;
      }
      case 'revisit': {
        const items = await invoke<BrainWikiReviewItem[]>('brain_wiki_revisit', { limit: 12 });
        addTerranSoulChatMessage(formatReviewQueue(items));
        return true;
      }
      case 'weave':
      case 'trace':
      case 'why':
        addTerranSoulChatMessage(
          `/${command.kind} is planned for the next wiki rollout. Available now:\n${BRAIN_WIKI_HELP_TEXT}`,
        );
        return true;
      default:
        return false;
    }
  } catch (error) {
    addTerranSoulChatMessage(`Brain wiki command failed: ${String(error)}`, 'sad');
    return true;
  }
}

function precomputePendingEmotionForStreaming(message: string): void {
  // Pre-compute user emotion before the async send starts so streaming UI
  // can use this value instead of a generic 'talking' state.
  const userSentiment = detectSentiment(message);
  pendingEmotion.value = sentimentToState(userSentiment);
}

const reasoningEffortUiValue = computed<'off' | 'medium' | 'high'>(() => {
  const value = settingsStore.settings.reasoning_effort ?? 'off';
  if (value === 'off' || value === 'medium' || value === 'high') return value;
  // Migrate legacy "low" to the new 3-level UI as "balanced".
  return 'medium';
});

function handleReasoningEffortChange(e: Event) {
  const value = (e.target as HTMLSelectElement).value as 'off' | 'medium' | 'high';
  settingsStore.saveSettings({ reasoning_effort: value });
}

async function handleSend(message: string) {
  if (browserRuntime.value && !brain.browserAuthSession && !brain.hasBrain) {
    showBrowserLlmConfig.value = true;
    return;
  }

  // Stop any ongoing TTS playback before sending a new message.
  tts.stop();

  // /commands — list all available slash commands (built-in + prompt files)
  if (message.trim().toLowerCase() === '/commands') {
    addUserChatMessage(message);
    const { getAvailableCommands } = usePromptCommandDispatch();
    const promptCmds = getAvailableCommands();
    const builtIn = ['reflect', 'commands'];
    let output = '**Available Commands:**\n\n';
    output += '**Built-in:**\n';
    for (const cmd of builtIn) {
      output += `- \`/${cmd}\`\n`;
    }
    if (promptCmds.length > 0) {
      output += '\n**Prompt Commands** (`.terransoul/prompts/`):\n';
      for (const cmd of promptCmds) {
        output += `- \`/${cmd.name}\` — ${cmd.description}\n`;
      }
    }
    addTerranSoulChatMessage(output);
    return;
  }

  if (message.trim().toLowerCase() === '/reflect') {
    addUserChatMessage(message);
    try {
      const report = await invoke<SessionReflectionReport>('reflect_on_session');
      addTerranSoulChatMessage(`Reflection saved.\n\nSummary: ${report.summary}\n\nSaved ${report.facts_saved} fact${report.facts_saved === 1 ? '' : 's'} and linked ${report.source_turn_count} source turn${report.source_turn_count === 1 ? '' : 's'} with ${report.derived_edge_count} provenance edge${report.derived_edge_count === 1 ? '' : 's'}.`);
    } catch (error) {
      addTerranSoulChatMessage(`Reflection failed: ${String(error)}`, 'sad');
    }
    return;
  }

  if (await handleBrainWikiSlashCommand(message)) {
    return;
  }

  // Extensible prompt commands: if the message matches a loaded
  // `.terransoul/prompts/*.md` file, inject its content as the prompt.
  const promptResult = tryDispatchPromptCommand(message);
  if (promptResult.handled) {
    addUserChatMessage(message);
    if (promptResult.error) {
      addTerranSoulChatMessage(`⚠️ Prompt command /${promptResult.name} failed: ${promptResult.error}`, 'sad');
      return;
    }
    setAvatarState('thinking');
    await conversationStore.sendMessage(promptResult.prompt!);
    const lastMsg = conversationStore.messages[conversationStore.messages.length - 1];
    const reactionState = lastMsg?.role === 'assistant'
      ? sentimentToState(lastMsg.sentiment)
      : pendingEmotion.value;
    setAvatarState(reactionState);
    pendingEmotion.value = 'idle';
    return;
  }

  // Plugin slash-command interception (Chunk 22.4): if the message
  // matches `/<name> ...` and an active plugin contributes that name,
  // route to the plugin host and surface the result as an assistant
  // chat message instead of going through the LLM.
  const dispatch = await tryDispatchSlashCommand(message);
  if (dispatch.handled) {
    addUserChatMessage(message);
    addTerranSoulChatMessage(
      dispatch.error
        ? `⚠️ Plugin command \`/${dispatch.name}\` failed: ${dispatch.error}`
        : (dispatch.output || `(plugin returned no output for /${dispatch.name})`),
      dispatch.error ? 'sad' : 'neutral',
    );
    return;
  }

  // Store user query for capacity detection.
  lastUserQuery = message;

  // Explicitly pre-compute emotion now for later streaming-phase UI usage.
  precomputePendingEmotionForStreaming(message);

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

    // Assess response quality — suggest upgrade if struggling. Respects the
    // "Capacity-detection auto-upgrade" toggle in the Brain panel; when off,
    // the user is never auto-prompted to upgrade based on phrasing heuristics.
    if (brain.isFreeApiMode && !upgradeAlreadySuggested && aiDecisionPolicy.capacityDetectionEnabled) {
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
      viewportRef.value?.stopMotion?.();
    }
  }, 6000);
}

async function handleCharismaTurnRating(messageId: string, rating: number): Promise<void> {
  await conversationStore.rateCharismaTurn(messageId, rating);
}

const canSkipDialog = computed(
  // Desktop chat has a subtitle overlay; include it so skip can dismiss subtitle-only dialog state.
  () => conversationStore.isThinking || conversationStore.isStreaming || tts.isSpeaking.value || subtitleVisible.value,
);

function skipCurrentDialog() {
  tts.stop();
  isStreamTtsActive = false;
  if (subtitleHideTimer) {
    clearTimeout(subtitleHideTimer);
    subtitleHideTimer = null;
  }
  subtitleVisible.value = false;
  subtitleFullText.value = '';
  emojiPopupVisible.value = false;
  if (emojiPopupTimer) {
    clearTimeout(emojiPopupTimer);
    emojiPopupTimer = null;
  }
  streaming.reset();
  conversationStore.isStreaming = false;
  conversationStore.streamingText = '';
  setAvatarState('idle');
  viewportRef.value?.stopMotion?.();
}

async function copyChatHistoryToClipboard() {
  try {
    await copyChatHistory(conversationStore.messages);
  } catch {
    // Clipboard unavailable
  }
}

async function pasteClipboardAsMessage() {
  if (conversationStore.isThinking) return;
  try {
    const text = (await readClipboardText()).trim();
    if (!text) return;
    await handleSend(text);
  } catch {
    // Clipboard unavailable
  }
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
        id: generateMessageId(),
        role: 'assistant',
        content: 'Brain upgraded! I switched to Groq for better responses. You can get a free API key at groq.com for even better performance!',
        agentName: 'TerranSoul',
        sentiment: 'happy',
        timestamp: Date.now(),
      });
    }
  } else if (optionId === 'local') {
    // Install local model via Ollama
    const recommendation = brain.topRecommendation;
    const model = recommendation?.model_tag;
    const displayName = recommendation?.display_name ?? model;
    if (model) {
      conversationStore.messages.push({
        id: generateMessageId(),
        role: 'assistant',
        content: `Great choice! I'm downloading ${displayName} now. This may take a few minutes...`,
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
          id: generateMessageId(),
          role: 'assistant',
          content: `${displayName} is installed and active! I'm much smarter now. Try asking me something complex!`,
          agentName: 'TerranSoul',
          sentiment: 'happy',
          timestamp: Date.now(),
        });
      } else {
        conversationStore.messages.push({
          id: generateMessageId(),
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
      id: generateMessageId(),
      role: 'assistant',
      content: 'To set up a paid API, open the Marketplace (🏪) and configure your API key in the LLM settings section.',
      agentName: 'TerranSoul',
      sentiment: 'happy',
      timestamp: Date.now(),
    });
  }
}

const emit = defineEmits<{
  navigate: [target: string];
  'set-display-mode': [mode: 'desktop' | 'chatbox'];
  'toggle-pet-mode': [];
}>();

const props = defineProps<{
  /** When true, hide the 3D character and show a clean chat-only layout. */
  chatboxMode?: boolean;
}>();

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

/**
 * Matches common "learn X" phrasings and captures the topic in group 1.
 * Supported prompts include: "learn about ...", "teach me about ...",
 * "study ...", "deep dive into ...", and "learn ...".
 * Intentionally captures the remainder of the user message as topic text,
 * including multi-sentence input.
 */
const LEARNING_TOPIC_REGEX = /(?:learn about|teach me about|study|deep dive into|learn)\s+(.+\S)\s*$/;

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
        const match = lower.match(LEARNING_TOPIC_REGEX);
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
          await voice.setTtsProvider('web-speech');
          
          // Add confirmation message
          const speechMsg = `Perfect! I've enabled Web Speech (your browser's built-in voice synthesis - free, offline-capable, no third-party API). You'll now hear my responses spoken aloud. Try sending me a message to test it!`;
          await conversationStore.addMessage({
            id: generateMessageId(),
            role: 'assistant',
            content: speechMsg,
            agentName: 'TerranSoul',
            sentiment: 'happy',
            timestamp: Date.now(),
          });
          showSubtitle(speechMsg);
          speakQuestText(speechMsg);
          
        } catch (error) {
          console.warn('Auto-config failed:', error);
          // Show error message
          const errMsg = `I had trouble setting up voice automatically. You can configure it manually in the Voice tab.`;
          await conversationStore.addMessage({
            id: generateMessageId(),
            role: 'assistant',
            content: errMsg,
            agentName: 'TerranSoul',
            sentiment: 'sad',
            timestamp: Date.now(),
          });
          showSubtitle(errMsg);
          speakQuestText(errMsg);
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

  // Handle the "Learn X using my documents" install flow. These values are
  // produced by the new learn-docs prompts in the conversation store and
  // route to its quest-driven auto-install path.
  if (choiceValue.startsWith('learn-docs:')) {
    await handleLearnDocsChoice(choiceValue);
    return;
  }

  // Handle model-update upgrade/dismiss choices.
  if (choiceValue.startsWith('model-update:')) {
    await handleModelUpdateChoice(choiceValue);
    return;
  }

  // Handle "type this command" shortcuts — the button submits the literal
  // command text through sendMessage so the conversation store's command
  // detector fires exactly as if the user had typed it.
  if (choiceValue.startsWith('command:')) {
    const cmdText = choiceValue.slice('command:'.length);
    await conversationStore.sendMessage(cmdText);
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
  const content =
    `📚 **Scholar's Quest Complete!** I've finished learning about **${knowledgeQuestTopic.value}**.\n\n` +
    `Go ahead and ask me questions — my answers will now draw from the source materials you provided!`;
  conversationStore.addMessage({
    id: generateMessageId(),
    role: 'assistant',
    content,
    agentName: 'TerranSoul',
    sentiment: 'happy',
    timestamp: Date.now(),
  });
  showSubtitle(content);
  speakQuestText(content);
}

/** Set up Tauri event listeners for dual-stream LLM events. */
async function setupTauriEventListener() {
  if (usesRemoteConversation) return;
  try {
    const { listen } = await import('@tauri-apps/api/event');

    // Text stream — already clean (anim blocks stripped by Rust parser).
    // Thinking chunks (`thinking:true`) are reasoning traces and must NOT
    // be spoken or fed into TTS — only the answer chunks reach the voice
    // pipeline.
    const unlistenChunk = await listen<{ text: string; done: boolean; thinking?: boolean }>('llm-chunk', (event) => {
      streaming.handleChunk(event.payload);

      // Feed text directly into TTS — no tag stripping needed.
      if (voice.config.tts_provider) {
        if (event.payload.done) {
          tts.flush();
          isStreamTtsActive = false;
        } else if (event.payload.text && !event.payload.thinking) {
          if (!isStreamTtsActive) {
            // New AI response started: stop previous speech and only speak latest.
            tts.stop();
            isStreamTtsActive = true;
          }
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
    } else if (!tts.isSpeaking.value && streaming.currentEmotion) {
      // Stream done AND TTS not speaking — set final emotion.
      // If TTS is still speaking, the isSpeaking watcher handles
      // the transition when speech finishes.
      characterStore.setState(sentimentToState(streaming.currentEmotion), streaming.currentEmotionIntensity);
      const asm = getAsm();
      if (asm) asm.setEmotion(streaming.currentEmotion === 'neutral' ? 'neutral' : streaming.currentEmotion, streaming.currentEmotionIntensity);
    }
    // Note: tts.flush() is NOT called here — the llm-chunk done:true
    // handler already flushes the TTS buffer. Double-flushing caused
    // the last sentence to be spoken twice.
    if (!active) {
      isStreamTtsActive = false;
    }
  },
);

// Update subtitle text during streaming (don't re-key, just update content)
watch(
  () => conversationStore.streamingText,
  (text) => {
    if (text && karaokeDialogEnabled.value) {
      if (subtitleHideTimer) { clearTimeout(subtitleHideTimer); subtitleHideTimer = null; }
      subtitleFullText.value = stripMarkdownForSubtitle(text);
      subtitleVisible.value = true;
    }
  },
);

// TTS speaking state → body='talk', done → apply final emotion + schedule subtitle hide
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
    if (karaokeDialogEnabled.value) {
      if (subtitleHideTimer) { clearTimeout(subtitleHideTimer); subtitleHideTimer = null; }
      subtitleVisible.value = true;
    }
  } else {
    if (!vrmaActive) {
      asm.forceBody('idle');
      // Apply final emotion from stream instead of just going idle
      if (streaming.currentEmotion) {
        characterStore.setState(
          sentimentToState(streaming.currentEmotion),
          streaming.currentEmotionIntensity,
        );
        asm.setEmotion(
          streaming.currentEmotion === 'neutral' ? 'neutral' : streaming.currentEmotion,
          streaming.currentEmotionIntensity,
        );
      } else {
        characterStore.setState('idle');
      }
    }
    // TTS finished — schedule subtitle to fade away
    if (subtitleFullText.value) {
      scheduleSubtitleHide();
    }
  }
});

watch(karaokeDialogEnabled, (enabled) => {
  if (enabled) return;
  if (subtitleHideTimer) {
    clearTimeout(subtitleHideTimer);
    subtitleHideTimer = null;
  }
  subtitleVisible.value = false;
  subtitleFullText.value = '';
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
  window.addEventListener('ts:llm-sentence', handleBrowserSentenceEvent);
  await setupTauriEventListener();

  // Load extensible prompt commands from .terransoul/prompts/
  promptCommandsStore.loadCommands();

  // Initialise background task listener
  const taskStore = useTaskStore();
  await taskStore.init();

  if (browserRuntime.value) {
    brain.prepareBrowserProviderChoices();
    if (import.meta.env.VITE_E2E && !brain.hasBrain) {
      brain.autoConfigureFreeApi();
    }
  } else {
    try {
      await brain.initialise();
      // If on a free cloud API but Ollama is running locally,
      // auto-upgrade for dramatically lower latency (~400ms vs 5-25s).
      await brain.maybeUpgradeToLocalOllama();
      if (brain.topRecommendation) {
        selectedBrain.value = brain.topRecommendation.model_tag;
      }
      // Background model update check — once per day, non-blocking.
      brain.checkForModelUpdates();
    } catch {
      // No Tauri backend
    }
  }

  try {
    await voice.initialise();
  } catch {
    // No Tauri backend — voice stays in text-only mode
  }

  // Load persisted settings (model selection, camera state) and user-imported
  // VRMs. User models must be loaded before selectModel() so a previously
  // selected user model can be restored.
  try {
    await settingsStore.loadSettings();
    await characterStore.loadUserModels();
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
  window.removeEventListener('ts:llm-sentence', handleBrowserSentenceEvent);
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
  if (taskControlsLingerTimer) clearTimeout(taskControlsLingerTimer);
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

.viewport-layer--settings-priority {
  z-index: 16;
}

/* ── AI state pill colours — used by `.chatbox-state-pill` in chatbox header.
   The free-floating 3D-mode pill now lives in CharacterViewport's corner
   cluster (single flex stack alongside Settings — no overlap risk). ── */
.ai-state-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: currentColor;
  transition: background 0.4s ease;
}

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
  gap: 7px;
  padding: 5px 16px;
  border-radius: var(--ts-radius-pill);
  background: var(--ts-glass-bg, rgba(15, 23, 42, 0.72));
  backdrop-filter: blur(12px) saturate(1.3);
  -webkit-backdrop-filter: blur(12px) saturate(1.3);
  border: 1px solid rgba(34, 197, 94, 0.2);
  font-size: 0.7rem;
  color: var(--ts-success);
  pointer-events: auto;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.2);
  transition: transform var(--ts-transition-fast), box-shadow var(--ts-transition-fast);
}
.brain-status-pill:hover {
  transform: translateX(-50%) translateY(-1px);
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3);
}
.brain-pill-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--ts-success-dim);
  box-shadow: 0 0 6px rgba(34, 197, 94, 0.4);
  animation: pulse-dot 2s ease-in-out infinite;
}

.brain-reconfigure-btn,
.chatbox-reconfigure-btn {
  border: 1px solid rgba(124, 111, 255, 0.25);
  border-radius: var(--ts-radius-pill);
  padding: 0.28rem 0.6rem;
  color: var(--ts-text-primary);
  background: var(--ts-bg-input, rgba(255, 255, 255, 0.04));
  font-size: 0.68rem;
  font-weight: 800;
  cursor: pointer;
  transition: all var(--ts-transition-fast);
}

.brain-reconfigure-btn:hover,
.chatbox-reconfigure-btn:hover {
  background: rgba(124, 111, 255, 0.12);
  border-color: rgba(124, 111, 255, 0.4);
}

/* ── Inline subtitle — karaoke-style word sync, full width above chat controls ── */
.subtitle-inline {
  width: 100%;
  margin-bottom: 4px;
}
.subtitle-text {
  margin: 0;
  padding: 10px 16px;
  background: var(--ts-glass-bg, rgba(15, 23, 42, 0.72));
  backdrop-filter: blur(20px) saturate(1.4);
  -webkit-backdrop-filter: blur(20px) saturate(1.4);
  border-radius: var(--ts-radius-lg);
  border: 1px solid var(--ts-border, rgba(255, 255, 255, 0.08));
  color: var(--ts-text-primary);
  font-size: 0.93rem;
  line-height: 1.6;
  text-align: center;
  max-height: 8em;
  overflow-y: auto;
  scroll-behavior: smooth;
  scrollbar-width: thin;
  scrollbar-color: var(--ts-text-dim) transparent;
}
.subtitle-text::-webkit-scrollbar { width: 4px; }
.subtitle-text::-webkit-scrollbar-track { background: transparent; }
.subtitle-text::-webkit-scrollbar-thumb { background: var(--ts-text-dim); border-radius: 2px; }
/* Spoken text — dimmed */
:deep(.subtitle-spoken) {
  color: var(--ts-text-dim);
  transition: color 0.3s ease;
}
/* Currently speaking sentence — bright highlight */
:deep(.subtitle-active) {
  color: var(--ts-text-on-accent);
  background: var(--ts-accent-glow);
  border-radius: 3px;
  padding: 1px 2px;
  transition: color 0.2s ease, background 0.2s ease;
}
/* Upcoming text — normal but slightly dimmed */
:deep(.subtitle-upcoming) {
  color: var(--ts-text-secondary);
  transition: color 0.3s ease;
}

/* Subtitle transition */
.subtitle-enter-active { transition: opacity 0.3s ease, transform 0.3s ease; }
.subtitle-leave-active { transition: opacity 0.25s ease, transform 0.25s ease; }
.subtitle-enter-from { opacity: 0; transform: translateY(8px); }
.subtitle-leave-to { opacity: 0; transform: translateY(-4px); }

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
  background: var(--ts-bg-panel);
  backdrop-filter: blur(10px);
  border: 1px solid var(--ts-border);
  box-shadow: var(--ts-shadow-md);
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
  bottom: var(--ts-chat-bottom-nav-offset, 0px);
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
  background: var(--ts-glass-bg, rgba(15, 23, 42, 0.72));
  backdrop-filter: blur(24px) saturate(1.4);
  -webkit-backdrop-filter: blur(24px) saturate(1.4);
  border-top: 1px solid var(--ts-glass-border, rgba(255, 255, 255, 0.08));
  box-shadow: 0 -8px 32px rgba(0, 0, 0, 0.3);
  scrollbar-width: thin;
  scrollbar-color: var(--ts-text-dim) transparent;
  display: flex;
  flex-direction: column;
}
.chat-history-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 12px 6px;
  border-bottom: 1px solid var(--ts-border-subtle, rgba(255, 255, 255, 0.05));
  flex-shrink: 0;
}
.chat-history-actions {
  display: flex;
  align-items: center;
  gap: 4px;
}
.chat-history-action-btn {
  border: 1px solid var(--ts-border-subtle, rgba(255, 255, 255, 0.08));
  background: var(--ts-bg-input, rgba(255, 255, 255, 0.04));
  color: var(--ts-text-primary);
  font-size: 0.6rem;
  font-weight: 700;
  border-radius: var(--ts-radius-sm);
  padding: 3px 7px;
  cursor: pointer;
  transition: all var(--ts-transition-fast);
}
.chat-history-action-btn:hover {
  background: var(--ts-bg-hover);
  border-color: var(--ts-border);
}
.chat-history-action-btn.skip {
  border-color: rgba(239, 68, 68, 0.35);
  color: var(--ts-error);
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
  background: var(--ts-bg-hover);
}

/* Chat history slide transition */
.chat-panel-enter-active { transition: max-height 0.35s cubic-bezier(0.4,0,0.2,1), opacity 0.25s ease; }
.chat-panel-leave-active { transition: max-height 0.3s cubic-bezier(0.4,0,0.2,1), opacity 0.2s ease; }
.chat-panel-enter-from, .chat-panel-leave-to { max-height: 0; opacity: 0; overflow: hidden; }
.chat-panel-enter-to, .chat-panel-leave-from { max-height: 50vh; opacity: 1; }

/* Input footer — always visible at the very bottom */
.input-footer {
  background: var(--ts-glass-bg, rgba(15, 23, 42, 0.72));
  backdrop-filter: blur(24px) saturate(1.3);
  -webkit-backdrop-filter: blur(24px) saturate(1.3);
  border-top: 1px solid var(--ts-glass-border, rgba(255, 255, 255, 0.08));
  padding: 6px 12px var(--ts-chat-footer-bottom-padding, 12px);
  box-shadow: 0 -4px 20px rgba(0, 0, 0, 0.15);
}

/* Collapsed footer should not paint a full-width glass strip. */
.bottom-panel:not(.expanded) .input-footer {
  background: transparent;
  border-top-color: transparent;
  box-shadow: none;
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
}
.input-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.input-top-left-controls {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  margin-bottom: 4px;
  align-self: flex-start;
  width: fit-content;
  padding: 2px;
  border-radius: 999px;
  border: 1px solid var(--ts-glass-border, rgba(255, 255, 255, 0.08));
  background: color-mix(in srgb, var(--ts-glass-bg, rgba(15, 23, 42, 0.72)) 92%, transparent);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
}

.input-row :deep(.chat-input-bar) {
  flex: 1;
  min-width: 0;
}

.chat-history-controls {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  min-width: 0;
  width: fit-content;
  padding: 2px;
  border-radius: 999px;
  border: 1px solid var(--ts-glass-border, rgba(255, 255, 255, 0.08));
  background: color-mix(in srgb, var(--ts-glass-bg, rgba(15, 23, 42, 0.72)) 92%, transparent);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
}

/* Compact sizing for the top-left control rows only (not global pills). */
.input-top-left-controls .chat-drawer-toggle,
.chat-history-controls .chat-drawer-toggle {
  height: 26px;
  padding: 0 8px;
  font-size: 0.62rem;
  gap: 4px;
  border: 0;
  background: transparent;
  box-shadow: none;
}

.input-top-left-controls .reasoning-effort-select,
.chat-history-controls .reasoning-effort-select,
.input-top-left-controls .chat-context-pill,
.chat-history-controls .chat-context-pill {
  height: 26px;
  padding: 0 6px;
  font-size: 0.64rem;
  border: 0;
  background: transparent;
  max-width: 8rem;
}

.input-top-left-controls .chat-drawer-toggle + .reasoning-effort-select,
.chat-history-controls .chat-drawer-toggle + .reasoning-effort-select,
.input-top-left-controls .reasoning-effort-select + .chat-context-pill,
.chat-history-controls .reasoning-effort-select + .chat-context-pill {
  border-left: 1px solid var(--ts-glass-border, rgba(255, 255, 255, 0.12));
  border-radius: 0;
  padding-left: 6px;
}

.chat-context-pill {
  display: inline-flex;
  align-items: center;
  min-width: 0;
  color: var(--ts-text-secondary, rgba(226, 232, 240, 0.82));
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.reasoning-effort-select {
  height: 26px;
  padding: 0 4px;
  border-radius: var(--ts-radius-sm, 6px);
  border: 1px solid var(--ts-glass-border, rgba(255, 255, 255, 0.08));
  background: var(--ts-glass-bg, rgba(15, 23, 42, 0.72));
  color: var(--ts-text-primary, #e2e8f0);
  font-size: 0.68rem;
  font-weight: 500;
  cursor: pointer;
  outline: none;
  flex-shrink: 0;
  transition: border-color 0.2s, background 0.2s;
  /* Inherit from <html> color-scheme so the native popup adopts the
     active theme's light/dark scheme automatically. */
  color-scheme: inherit;
  accent-color: var(--ts-accent, #7c3aed);
}

.reasoning-effort-select:hover,
.reasoning-effort-select:focus {
  border-color: var(--ts-accent, #7c3aed);
  background: var(--ts-glass-bg-focus, var(--ts-glass-bg, rgba(15, 23, 42, 0.85)));
}

.reasoning-effort-select option {
  background: var(--ts-bg-surface, var(--ts-bg-base, #0f172a));
  color: var(--ts-text-primary, #e2e8f0);
}

/* ── Chat toggle button — pill with icon + label ── */
.chat-drawer-toggle {
  height: 28px;
  padding: 0 10px;
  border-radius: var(--ts-radius-pill);
  border: 1px solid var(--ts-glass-border, rgba(255, 255, 255, 0.08));
  background: var(--ts-glass-bg, rgba(15, 23, 42, 0.72));
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  color: var(--ts-text-primary);
  font-size: 0.64rem;
  font-weight: 600;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
  transition: all 0.25s cubic-bezier(0.34, 1.56, 0.64, 1);
  box-shadow: var(--ts-shadow-sm);
}
.toggle-label {
  letter-spacing: 0.03em;
}
.chat-drawer-toggle:hover {
  background: var(--ts-accent);
  color: var(--ts-text-on-accent);
  border-color: transparent;
  transform: scale(1.04);
  box-shadow: 0 4px 20px rgba(124, 111, 255, 0.3);
}
.chat-drawer-toggle.active {
  background: var(--ts-accent);
  border-color: var(--ts-accent-hover);
  color: var(--ts-text-on-accent);
}

/* ── Mic button — voice input toggle ── */
.mic-btn {
  width: 44px;
  height: 44px;
  border-radius: 50%;
  border: 1px solid var(--ts-border-strong);
  background: var(--ts-bg-panel);
  backdrop-filter: blur(10px);
  color: var(--ts-text-on-accent);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: background var(--ts-transition-normal), border-color var(--ts-transition-normal), box-shadow var(--ts-transition-fast);
}
.mic-btn:hover {
  background: var(--ts-bg-hover);
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
  width: 360px;
  max-width: 90vw;
}
.browser-llm-overlay {
  width: min(620px, 92vw);
}
.brain-card {
  background: var(--ts-glass-bg, rgba(15, 23, 42, 0.72));
  backdrop-filter: blur(24px) saturate(1.4);
  -webkit-backdrop-filter: blur(24px) saturate(1.4);
  border-radius: var(--ts-radius-lg);
  padding: 22px 24px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  border: 1px solid var(--ts-glass-border, rgba(255, 255, 255, 0.08));
  box-shadow: var(--ts-shadow-lg), inset 0 1px 0 rgba(255, 255, 255, 0.05);
}
.brain-card-header { display: flex; align-items: center; gap: 8px; font-size: var(--ts-text-base); font-weight: 600; }
.brain-hw { font-size: var(--ts-text-sm); color: var(--ts-text-secondary); margin: 0; }
.brain-rec { font-size: 0.8rem; color: var(--ts-text-bright, var(--ts-text-primary)); margin: 0; line-height: 1.45; }
.brain-rec small { color: var(--ts-text-muted); }
.brain-models { display: flex; flex-wrap: wrap; gap: 6px; }
.brain-model-btn {
  padding: 6px 12px;
  border-radius: var(--ts-radius-sm);
  border: 1px solid var(--ts-glass-border, rgba(255, 255, 255, 0.08));
  background: var(--ts-bg-input, rgba(255, 255, 255, 0.03));
  color: var(--ts-text-secondary);
  font-size: 0.75rem;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 5px;
  transition: all 0.2s cubic-bezier(0.34, 1.56, 0.64, 1);
}
.brain-model-btn.top { border-color: rgba(59, 130, 246, 0.35); }
.brain-model-btn.selected { border-color: var(--ts-success); background: rgba(34, 197, 94, 0.08); color: var(--ts-success); box-shadow: 0 0 10px rgba(34, 197, 94, 0.15); }
.brain-model-btn:hover { background: var(--ts-bg-hover); transform: translateY(-1px); }
.brain-star { font-size: 0.7rem; }
.brain-warn { font-size: var(--ts-text-sm); color: var(--ts-warning-text); background: rgba(239, 68, 68, 0.08); padding: 8px 12px; border-radius: var(--ts-radius-sm); border: 1px solid rgba(239, 68, 68, 0.15); display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
.brain-warn code { background: var(--ts-bg-input, rgba(0, 0, 0, 0.3)); padding: 2px 6px; border-radius: 3px; font-size: 0.72rem; }
.brain-retry-btn { padding: 3px 10px; border: none; background: rgba(124, 111, 255, 0.12); color: var(--ts-accent-blue); border-radius: 4px; cursor: pointer; font-size: 0.72rem; transition: all var(--ts-transition-fast); }
.brain-retry-btn:hover { background: rgba(124, 111, 255, 0.2); }
.brain-pulling { display: flex; align-items: center; gap: 8px; font-size: var(--ts-text-sm); color: var(--ts-text-secondary); }
.brain-spinner { width: 14px; height: 14px; border: 2px solid var(--ts-border, rgba(255, 255, 255, 0.1)); border-top-color: var(--ts-accent-blue); border-radius: 50%; animation: spin 0.8s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
.brain-activate-btn {
  padding: 8px 18px;
  border: none;
  background: linear-gradient(135deg, var(--ts-success-dim), var(--ts-success));
  color: var(--ts-text-on-accent);
  border-radius: var(--ts-radius-sm);
  cursor: pointer;
  font-size: 0.82rem;
  font-weight: 600;
  align-self: flex-start;
  transition: all 0.2s cubic-bezier(0.34, 1.56, 0.64, 1);
  box-shadow: 0 2px 10px rgba(34, 197, 94, 0.2);
}
.brain-activate-btn:hover { transform: translateY(-1px); box-shadow: 0 4px 16px rgba(34, 197, 94, 0.3); }
.brain-local-btn {
  padding: 8px 18px;
  border: none;
  background: linear-gradient(135deg, var(--ts-accent-blue), var(--ts-accent-blue-hover));
  color: var(--ts-text-on-accent);
  border-radius: var(--ts-radius-sm);
  cursor: pointer;
  font-size: 0.82rem;
  font-weight: 600;
  align-self: flex-start;
  transition: all 0.2s cubic-bezier(0.34, 1.56, 0.64, 1);
  box-shadow: 0 2px 10px rgba(96, 165, 250, 0.2);
}
.brain-local-btn:hover { transform: translateY(-1px); box-shadow: 0 4px 16px rgba(96, 165, 250, 0.3); }
.brain-free-start { display: flex; flex-direction: column; gap: 6px; }
.brain-free-start p { margin: 0; font-size: var(--ts-text-sm); color: var(--ts-text-secondary); }
.brain-local-section { border-top: 1px solid var(--ts-border-subtle, rgba(255, 255, 255, 0.05)); padding-top: 8px; margin-top: 4px; }

/* ── Mobile adjustments ── */
/* ═══ CHATBOX-ONLY MODE ═══
 * When chatbox_mode is active, the entire layout changes from the
 * 3D-overlay approach to a clean, traditional chat interface:
 * - No Three.js canvas (zero GPU cost)
 * - Full-height message list with dark-themed background
 * - Compact header bar with brain status and AI state
 * - Input pinned to the bottom
 * Designed for users who prefer a text-focused experience. */

.chat-view.chatbox-only {
  background: transparent;
}

.chatbox-layout {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

/* ── Chatbox header bar ── */
.chatbox-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 18px;
  background: var(--ts-glass-bg, rgba(15, 23, 42, 0.72));
  backdrop-filter: blur(20px) saturate(1.3);
  -webkit-backdrop-filter: blur(20px) saturate(1.3);
  border-bottom: 1px solid var(--ts-glass-border, rgba(255, 255, 255, 0.08));
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.15);
  flex-shrink: 0;
}
.chatbox-header-left {
  display: flex;
  align-items: center;
  gap: 10px;
}
.chatbox-header-right {
  display: flex;
  align-items: center;
  gap: 10px;
}
.chatbox-provider {
  display: flex;
  align-items: center;
  gap: 7px;
  font-size: 0.76rem;
  color: var(--ts-success);
  font-weight: 600;
}
.chatbox-brain-setup {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 0.78rem;
}
.chatbox-brain-btn {
  border: 1px solid rgba(124, 111, 255, 0.3);
  background: rgba(124, 111, 255, 0.1);
  color: var(--ts-accent);
  font-size: 0.72rem;
  font-weight: 700;
  padding: 6px 16px;
  border-radius: var(--ts-radius-pill);
  cursor: pointer;
  transition: all 0.25s cubic-bezier(0.34, 1.56, 0.64, 1);
}
.chatbox-brain-btn:hover {
  background: var(--ts-accent);
  color: var(--ts-text-on-accent);
  border-color: transparent;
  transform: translateY(-1px) scale(1.02);
  box-shadow: 0 4px 16px rgba(124, 111, 255, 0.3);
}

/* AI state pill in chatbox header — smaller inline variant */
.chatbox-state-pill {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 5px 14px;
  border-radius: var(--ts-radius-pill, 999px);
  font-size: 0.68rem;
  font-weight: 700;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  background: rgba(37, 99, 235, 0.15);
  color: #93c5fd;
  border: 1px solid rgba(147, 197, 253, 0.15);
  backdrop-filter: blur(8px);
  transition: all 0.3s ease;
}
.chatbox-state-pill.thinking { background: rgba(245, 158, 11, 0.18); color: var(--ts-warning-text); border-color: rgba(253, 230, 138, 0.2); }
.chatbox-state-pill.talking  { background: rgba(22, 163, 74, 0.15); color: var(--ts-success); border-color: rgba(134, 239, 172, 0.2); }
.chatbox-state-pill.happy    { background: rgba(8, 145, 178, 0.15); color: var(--ts-info); border-color: rgba(103, 232, 249, 0.2); }
.chatbox-state-pill.sad      { background: rgba(126, 34, 206, 0.15); color: var(--ts-accent-violet); border-color: rgba(216, 180, 254, 0.2); }
.chatbox-state-pill.angry    { background: rgba(239, 68, 68, 0.2); color: var(--ts-error); border-color: rgba(252, 165, 165, 0.25); }
.chatbox-state-pill.thinking .ai-state-dot { animation: pulse-dot 1.2s ease-in-out infinite; }

/* ── Full-height message area ── */
.chatbox-messages {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 0;
  scrollbar-width: thin;
  scrollbar-color: rgba(255, 255, 255, 0.15) transparent;
}

.chat-llm-auth {
  margin: 14px;
}

/* ── Chatbox input footer ── */
.chatbox-footer {
  flex-shrink: 0;
  background: var(--ts-glass-bg, rgba(15, 23, 42, 0.72));
  backdrop-filter: blur(20px) saturate(1.3);
  -webkit-backdrop-filter: blur(20px) saturate(1.3);
  border-top: 1px solid var(--ts-glass-border, rgba(255, 255, 255, 0.08));
  padding: 12px 18px 14px;
  box-shadow: 0 -2px 12px rgba(0, 0, 0, 0.12);
}
.chatbox-footer .input-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.chatbox-footer .input-top-left-controls {
  margin-bottom: 12px;
}

/* ── Mobile responsive for chatbox mode ── */
@media (max-width: 640px) {
  .chatbox-header { padding: 8px 10px; }
  .chatbox-footer { padding: 8px 10px 10px; }
  .chatbox-state-pill { padding: 2px 8px; font-size: 0.6rem; }
}

@media (max-width: 640px) {
  .bottom-panel { max-height: 50vh; }
  .subtitle-text { padding: 8px 14px; font-size: 0.82rem; }
  /* AI state pill mobile sizing now lives in CharacterViewport.vue alongside
     the corner cluster — keep brain-status / music-bar overrides here. */
  .ai-state-dot { width: 4px; height: 4px; }
  /* Brain status: below mode-toggle pill on the left */
  .brain-status-pill {
    left: 10px;
    top: 44px;
    transform: none;
    font-size: 0.58rem;
    padding: 2px 8px;
  }
  .brain-status-pill:hover {
    transform: translateY(-1px);
  }
  .brain-overlay { width: 92vw; }
  /* Compact input footer */
  .input-footer { padding: 6px 8px var(--ts-chat-footer-bottom-padding, 12px); }
  .input-top-left-controls { margin-bottom: 4px; }
  .chatbox-footer .input-top-left-controls { margin-bottom: 4px; }
  .chat-drawer-toggle { height: 34px; padding: 0 10px; }
  .toggle-label { display: none; }
}
</style>
