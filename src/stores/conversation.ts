import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { Message } from '../types';

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
    } catch (err) {
      const errMsg: Message = {
        id: crypto.randomUUID(),
        role: 'assistant',
        content: `Error: ${String(err)}`,
        agentName: 'System',
        timestamp: Date.now(),
      };
      messages.value.push(errMsg);
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
