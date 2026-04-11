import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { Message } from '../types';

/** Browser-side persona fallback when Tauri backend is unavailable. */
function createPersonaResponse(content: string): Message {
  const lower = content.toLowerCase();
  let response: string;
  let sentiment: 'happy' | 'sad' | 'neutral';

  if (lower.includes('hello') || lower.includes('hi') || lower.startsWith('hey')) {
    response = "Hello! I'm TerranSoul, your AI companion. How can I help you today? 😊";
    sentiment = 'happy';
  } else if (lower.includes('sad') || lower.includes('bad') || lower.includes('hate') || lower.includes('sorry')) {
    response = "I understand you're going through something difficult. I'm here for you. 💙";
    sentiment = 'sad';
  } else if (lower.includes('happy') || lower.includes('great') || lower.includes('awesome') || lower.includes('love')) {
    response = "That's wonderful to hear! Your positive energy is contagious! ✨";
    sentiment = 'happy';
  } else {
    response = `I hear you! You said: "${content}". I'm still learning, but I'm always here to listen and help!`;
    sentiment = 'neutral';
  }

  return {
    id: crypto.randomUUID(),
    role: 'assistant',
    content: response,
    agentName: 'TerranSoul',
    sentiment,
    timestamp: Date.now(),
  };
}

export const useConversationStore = defineStore('conversation', () => {
  const messages = ref<Message[]>([]);
  const currentAgent = ref<string>('auto');
  const isThinking = ref(false);

  async function sendMessage(content: string) {
    const userMsg: Message = {
      id: crypto.randomUUID(),
      role: 'user',
      content,
      timestamp: Date.now(),
    };
    messages.value.push(userMsg);
    isThinking.value = true;

    try {
      const response = await invoke<Message>('send_message', {
        message: content,
        agentId: currentAgent.value === 'auto' ? null : currentAgent.value,
      });
      messages.value.push(response);
    } catch {
      // No Tauri backend — use browser-side persona with emotion
      await new Promise(r => setTimeout(r, 500));
      messages.value.push(createPersonaResponse(content));
    } finally {
      isThinking.value = false;
    }
  }

  async function getConversation() {
    try {
      const history = await invoke<Message[]>('get_conversation');
      messages.value = history;
    } catch {
      // ignore
    }
  }

  return { messages, currentAgent, isThinking, sendMessage, getConversation };
});
