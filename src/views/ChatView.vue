<template>
  <div class="chat-view">
    <div class="viewport-section">
      <CharacterViewport />
    </div>
    <div class="chat-section">
      <ChatMessageList :messages="conversationStore.messages" :is-thinking="conversationStore.isThinking" />
      <ChatInput :disabled="conversationStore.isThinking" @submit="handleSend" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { watch } from 'vue';
import { useConversationStore } from '../stores/conversation';
import { useCharacterStore } from '../stores/character';
import CharacterViewport from '../components/CharacterViewport.vue';
import ChatMessageList from '../components/ChatMessageList.vue';
import ChatInput from '../components/ChatInput.vue';

const conversationStore = useConversationStore();
const characterStore = useCharacterStore();

async function handleSend(message: string) {
  characterStore.setState('thinking');
  await conversationStore.sendMessage(message);
  characterStore.setState('talking');
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
}

.chat-section {
  flex: 0 0 40%;
  display: flex;
  flex-direction: column;
  min-height: 0;
  background: rgba(10, 10, 20, 0.85);
  border-top: 1px solid rgba(255, 255, 255, 0.1);
}
</style>
