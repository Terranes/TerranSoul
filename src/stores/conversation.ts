import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { Message } from '../types';

/** Browser-side persona fallback when Tauri backend is unavailable. */
function createPersonaResponse(content: string): Message {
  const lower = content.toLowerCase();
  let response: string;
  let sentiment: 'happy' | 'sad' | 'angry' | 'relaxed' | 'surprised' | 'neutral';

  if (lower.includes('hello') || /\bhi\b/.test(lower) || lower.startsWith('hey')) {
    response = "Hello! I'm TerranSoul, your AI companion. How can I help you today? 😊";
    sentiment = 'happy';
  } else if (lower.includes('angry') || lower.includes('furious') || lower.includes('annoyed') || lower.includes('frustrated')) {
    response = "I can sense your frustration. Take a deep breath — I'm here to help. 🔥";
    sentiment = 'angry';
  } else if (lower.includes('surprise') || lower.includes('wow') || lower.includes('unexpected') || lower.includes('amazing') || lower.includes('whoa') || lower.includes('omg')) {
    response = "Wow, that's surprising! Tell me more! 😮";
    sentiment = 'surprised';
  } else if (lower.includes('relax') || lower.includes('calm') || lower.includes('peaceful') || lower.includes('chill') || lower.includes('meditat')) {
    response = "That sounds so peaceful. Let's take a moment to enjoy the calm. 🧘";
    sentiment = 'relaxed';
  } else if (lower.includes('sad') || lower.includes('bad') || lower.includes('hate') || lower.includes('sorry') || lower.includes('cry')) {
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
