<template>
  <div class="chat-view">
    <div class="viewport-section">
      <CharacterViewport />
      <button class="model-panel-toggle" @click="showModelPanel = !showModelPanel" aria-label="Toggle model panel">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
          <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 18c-4.41 0-8-3.59-8-8s3.59-8 8-8 8 3.59 8 8-3.59 8-8 8zm-1-13h2v6h-2zm0 8h2v2h-2z"/>
        </svg>
      </button>
      <ModelPanel v-if="showModelPanel" @close="showModelPanel = false" />
    </div>
    <div class="chat-section">
      <ChatMessageList :messages="conversationStore.messages" :is-thinking="conversationStore.isThinking" />
      <ChatInput :disabled="conversationStore.isThinking" @submit="handleSend" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue';
import { useConversationStore } from '../stores/conversation';
import { useCharacterStore } from '../stores/character';
import type { CharacterState } from '../types';
import CharacterViewport from '../components/CharacterViewport.vue';
import ChatMessageList from '../components/ChatMessageList.vue';
import ChatInput from '../components/ChatInput.vue';
import ModelPanel from '../components/ModelPanel.vue';

const conversationStore = useConversationStore();
const characterStore = useCharacterStore();
const showModelPanel = ref(false);

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

  // Use the sentiment from the last assistant response to drive character state
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
</script>

<style scoped>
.chat-view {
  display: flex;
  flex-direction: column;
  height: 100vh;
  width: 100%;
  overflow: hidden;
}

.viewport-section {
  flex: 0 0 60%;
  min-height: 0;
  position: relative;
}

.chat-section {
  flex: 0 0 40%;
  display: flex;
  flex-direction: column;
  min-height: 0;
  background: rgba(10, 10, 20, 0.85);
  border-top: 1px solid rgba(255, 255, 255, 0.1);
}

.model-panel-toggle {
  position: absolute;
  top: 12px;
  right: 60px;
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

.model-panel-toggle:hover {
  background: rgba(108, 99, 255, 0.4);
  color: #fff;
}
</style>
