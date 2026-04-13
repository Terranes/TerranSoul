/**
 * Pinia store for agent-to-agent messaging (topic-based pub/sub).
 */
import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { AgentMessageInfo } from '../types';

export const useMessagingStore = defineStore('messaging', () => {
  const messages = ref<AgentMessageInfo[]>([]);
  const subscriptions = ref<string[]>([]);
  const error = ref<string | null>(null);
  const isLoading = ref(false);

  async function publish(
    sender: string,
    topic: string,
    payload: unknown,
  ): Promise<AgentMessageInfo | null> {
    error.value = null;
    try {
      const msg = await invoke<AgentMessageInfo>('publish_agent_message', {
        sender,
        topic,
        payload,
      });
      return msg;
    } catch (err) {
      error.value = String(err);
      return null;
    }
  }

  async function subscribe(agentName: string, topic: string): Promise<boolean> {
    error.value = null;
    try {
      await invoke('subscribe_agent_topic', { agentName, topic });
      return true;
    } catch (err) {
      error.value = String(err);
      return false;
    }
  }

  async function unsubscribe(agentName: string, topic: string): Promise<boolean> {
    error.value = null;
    try {
      await invoke('unsubscribe_agent_topic', { agentName, topic });
      return true;
    } catch (err) {
      error.value = String(err);
      return false;
    }
  }

  async function getMessages(agentName: string): Promise<AgentMessageInfo[]> {
    isLoading.value = true;
    error.value = null;
    try {
      const msgs = await invoke<AgentMessageInfo[]>('get_agent_messages', { agentName });
      messages.value = msgs;
      return msgs;
    } catch (err) {
      error.value = String(err);
      return [];
    } finally {
      isLoading.value = false;
    }
  }

  async function listSubscriptions(agentName: string): Promise<string[]> {
    error.value = null;
    try {
      const subs = await invoke<string[]>('list_agent_subscriptions', { agentName });
      subscriptions.value = subs;
      return subs;
    } catch (err) {
      error.value = String(err);
      return [];
    }
  }

  function clearError() {
    error.value = null;
  }

  return {
    messages,
    subscriptions,
    error,
    isLoading,
    publish,
    subscribe,
    unsubscribe,
    getMessages,
    listSubscriptions,
    clearError,
  };
});
