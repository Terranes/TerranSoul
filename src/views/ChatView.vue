<template>
  <div class="chat-view">
    <!-- 3D viewport fills the whole area; click it to toggle dialog -->
    <div class="viewport-section" @click="toggleDialog">
      <CharacterViewport />
      <!-- Toggle button overlay -->
      <button
        class="dialog-toggle-btn"
        :class="{ active: showDialog }"
        @click.stop="toggleDialog"
        aria-label="Toggle dialog"
      >💬</button>
      <ModelPanel v-if="showModelPanel" @close="showModelPanel = false" />
      <button class="model-panel-toggle" @click.stop="showModelPanel = !showModelPanel" aria-label="Toggle model panel">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
          <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 18c-4.41 0-8-3.59-8-8s3.59-8 8-8 8 3.59 8 8-3.59 8-8 8zm-1-13h2v6h-2zm0 8h2v2h-2z"/>
        </svg>
      </button>
    </div>

    <!-- Toggleable chat dialog -->
    <Transition name="slide">
      <div v-if="showDialog" class="chat-section" @click.stop>
        <!-- Inline brain setup card (shown when no brain is configured) -->
        <div v-if="!brain.hasBrain" class="brain-inline">
          <div class="brain-card">
            <div class="brain-card-header">
              <span>🧠</span>
              <strong>Set up your Brain</strong>
            </div>
            <p v-if="brain.systemInfo" class="brain-hw">
              {{ brain.systemInfo.cpu_name }} · {{ formatRam(brain.systemInfo.total_ram_mb) }} RAM
            </p>
            <p v-if="brain.topRecommendation" class="brain-rec">
              Recommended: <strong>{{ brain.topRecommendation.display_name }}</strong>
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
            <div v-if="!brain.ollamaStatus.running" class="brain-warn">
              ❌ Ollama not running — start it first (<code>ollama serve</code>)
              <button class="brain-retry-btn" @click="brain.checkOllamaStatus()">🔄 Retry</button>
            </div>
            <div v-else-if="brain.isPulling" class="brain-pulling">
              <div class="brain-spinner" /> Downloading…
            </div>
            <div v-else-if="brain.pullError" class="brain-warn">❌ {{ brain.pullError }}</div>
            <button
              v-if="brain.ollamaStatus.running && !brain.isPulling && selectedBrain"
              class="brain-activate-btn"
              @click="activateBrain"
            >
              ⬇ Install &amp; activate {{ selectedBrain }}
            </button>
          </div>
        </div>

        <ChatMessageList :messages="conversationStore.messages" :is-thinking="conversationStore.isThinking" />
        <ChatInput :disabled="conversationStore.isThinking" @submit="handleSend" />
      </div>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted } from 'vue';
import { useConversationStore } from '../stores/conversation';
import { useCharacterStore } from '../stores/character';
import { useBrainStore } from '../stores/brain';
import type { CharacterState } from '../types';
import CharacterViewport from '../components/CharacterViewport.vue';
import ChatMessageList from '../components/ChatMessageList.vue';
import ChatInput from '../components/ChatInput.vue';
import ModelPanel from '../components/ModelPanel.vue';

const conversationStore = useConversationStore();
const characterStore = useCharacterStore();
const brain = useBrainStore();
const showModelPanel = ref(false);
const showDialog = ref(true);
const selectedBrain = ref('');

function toggleDialog() {
  showDialog.value = !showDialog.value;
}

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

function sentimentToState(sentiment?: string): CharacterState {
  switch (sentiment) {
    case 'happy': return 'happy';
    case 'sad': return 'sad';
    default: return 'talking';
  }
}

async function handleSend(message: string) {
  characterStore.setState('thinking');
  await conversationStore.sendMessage(message);

  const lastMsg = conversationStore.messages[conversationStore.messages.length - 1];
  const reactionState = lastMsg?.role === 'assistant'
    ? sentimentToState(lastMsg.sentiment)
    : 'talking';

  characterStore.setState(reactionState);
  setTimeout(() => characterStore.setState('idle'), 3000);
}

watch(
  () => conversationStore.isThinking,
  (thinking) => {
    if (thinking) characterStore.setState('thinking');
  },
);

onMounted(async () => {
  try {
    await brain.initialise();
    if (brain.topRecommendation) {
      selectedBrain.value = brain.topRecommendation.model_tag;
    }
  } catch {
    // No Tauri backend
  }
});
</script>

<style scoped>
.chat-view {
  display: flex;
  flex-direction: column;
  height: 100vh;
  width: 100%;
  overflow: hidden;
  position: relative;
}

.viewport-section {
  flex: 1;
  min-height: 0;
  position: relative;
  cursor: pointer;
}

.chat-section {
  flex: 0 0 45%;
  max-height: 45%;
  display: flex;
  flex-direction: column;
  min-height: 0;
  background: rgba(10, 10, 20, 0.9);
  border-top: 1px solid rgba(255, 255, 255, 0.1);
  backdrop-filter: blur(8px);
}

/* Toggle button */
.dialog-toggle-btn {
  position: absolute;
  bottom: 16px;
  right: 16px;
  z-index: 10;
  width: 44px;
  height: 44px;
  border-radius: 50%;
  border: 1px solid rgba(255, 255, 255, 0.25);
  background: rgba(0, 0, 0, 0.5);
  color: #fff;
  font-size: 1.2rem;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  backdrop-filter: blur(4px);
  transition: background 0.2s, transform 0.2s;
}
.dialog-toggle-btn:hover { background: rgba(59, 130, 246, 0.5); transform: scale(1.1); }
.dialog-toggle-btn.active { background: rgba(59, 130, 246, 0.6); }

/* Slide transition */
.slide-enter-active, .slide-leave-active { transition: transform 0.25s ease, opacity 0.25s ease; }
.slide-enter-from, .slide-leave-to { transform: translateY(100%); opacity: 0; }

.model-panel-toggle {
  position: absolute;
  top: 40px;
  right: 16px;
  z-index: 10;
  width: 32px;
  height: 32px;
  border-radius: 50%;
  border: 1px solid rgba(255, 255, 255, 0.2);
  background: rgba(0, 0, 0, 0.4);
  color: rgba(255, 255, 255, 0.7);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  backdrop-filter: blur(4px);
  transition: background 0.2s, color 0.2s;
}
.model-panel-toggle:hover { background: rgba(108, 99, 255, 0.4); color: #fff; }

/* ── Inline brain setup ── */
.brain-inline { padding: 10px 12px 4px; flex-shrink: 0; }
.brain-card { background: rgba(30, 41, 59, 0.9); border-radius: 10px; padding: 12px 14px; display: flex; flex-direction: column; gap: 6px; border: 1px solid rgba(59, 130, 246, 0.3); }
.brain-card-header { display: flex; align-items: center; gap: 6px; font-size: 0.9rem; }
.brain-hw { font-size: 0.78rem; color: #94a3b8; margin: 0; }
.brain-rec { font-size: 0.8rem; color: #cbd5e1; margin: 0; line-height: 1.4; }
.brain-rec small { color: #64748b; }
.brain-models { display: flex; flex-wrap: wrap; gap: 4px; }
.brain-model-btn { padding: 4px 10px; border-radius: 6px; border: 1px solid rgba(255,255,255,0.1); background: rgba(15, 23, 42, 0.8); color: #94a3b8; font-size: 0.75rem; cursor: pointer; display: flex; align-items: center; gap: 4px; transition: all 0.15s; }
.brain-model-btn.top { border-color: rgba(59, 130, 246, 0.4); }
.brain-model-btn.selected { border-color: #22c55e; background: rgba(26, 46, 26, 0.8); color: #86efac; }
.brain-model-btn:hover { background: rgba(30, 41, 59, 0.8); }
.brain-star { font-size: 0.7rem; }
.brain-warn { font-size: 0.78rem; color: #fca5a5; background: rgba(45, 28, 28, 0.7); padding: 6px 10px; border-radius: 6px; display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
.brain-warn code { background: rgba(30, 41, 59, 0.8); padding: 1px 4px; border-radius: 3px; font-size: 0.72rem; }
.brain-retry-btn { padding: 2px 8px; border: none; background: rgba(59, 130, 246, 0.3); color: #93c5fd; border-radius: 4px; cursor: pointer; font-size: 0.72rem; }
.brain-pulling { display: flex; align-items: center; gap: 6px; font-size: 0.78rem; color: #94a3b8; }
.brain-spinner { width: 14px; height: 14px; border: 2px solid #334155; border-top-color: #3b82f6; border-radius: 50%; animation: spin 0.8s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
.brain-activate-btn { padding: 6px 14px; border: none; background: #3b82f6; color: #fff; border-radius: 6px; cursor: pointer; font-size: 0.82rem; font-weight: 500; align-self: flex-start; transition: background 0.15s; }
.brain-activate-btn:hover { background: #2563eb; }
</style>
