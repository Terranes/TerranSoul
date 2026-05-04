import { computed, ref } from 'vue';
import { defineStore } from 'pinia';
import type { Message } from '../types';
import { parseTags } from '../utils/emotion-parser';
import { shouldUseRemoteConversation } from '../utils/runtime-target';
import { useMobilePairingStore } from './mobile-pairing';
import {
  createGrpcWebRemoteHost,
  createLocalRemoteHost,
  detectRemotePhoneToolIntent,
  dispatchRemotePhoneTool,
  remoteBaseUrl,
  type RemoteHost,
} from '../transport';

export interface RemoteConversationAdapters {
  remoteHost: () => RemoteHost | null;
  capabilities: () => string[] | undefined;
  now: () => number;
  createId: () => string;
}

let adapterOverrides: Partial<RemoteConversationAdapters> = {};

export function configureRemoteConversationAdapters(
  overrides: Partial<RemoteConversationAdapters>,
): void {
  adapterOverrides = overrides;
}

export function resetRemoteConversationAdapters(): void {
  adapterOverrides = {};
}

export const useRemoteConversationStore = defineStore('remote-conversation', () => {
  const messages = ref<Message[]>([]);
  const currentAgent = ref('auto');
  const agentSwitchHistory = ref<{ agentId: string; timestamp: number }[]>([]);
  const isThinking = ref(false);
  const isStreaming = ref(false);
  const streamingText = ref('');
  const translatorMode = ref(null);
  const messageQueue = ref<string[]>([]);
  const generationActive = ref(false);
  const cancelRequested = ref(false);
  const partialCommitted = ref(false);

  const agentMessages = computed<Message[]>(() => {
    const agentId = activeAgentId();
    if (!agentId) return messages.value;
    return messages.value.filter((message) => !message.agentId || message.agentId === agentId);
  });

  function activeAgentId(): string | undefined {
    return currentAgent.value === 'auto' ? undefined : currentAgent.value;
  }

  function setAgent(agentId: string): void {
    const previous = currentAgent.value;
    currentAgent.value = agentId;
    if (previous !== agentId && agentId !== 'auto') {
      agentSwitchHistory.value.push({ agentId, timestamp: currentAdapters().now() });
    }
  }

  async function sendMessage(content: string): Promise<void> {
    const text = content.trim();
    if (!text) return;
    if (generationActive.value) {
      addToQueue(text);
      return;
    }

    generationActive.value = true;
    cancelRequested.value = false;
    partialCommitted.value = false;
    isThinking.value = true;
    isStreaming.value = false;
    streamingText.value = '';

    messages.value.push(stampAgent({
      id: currentAdapters().createId(),
      role: 'user',
      content: text,
      timestamp: currentAdapters().now(),
    }));

    const adapters = currentAdapters();
    const host = adapters.remoteHost();
    if (!host) {
      addAssistantMessage(
        'This phone is not connected to a desktop host yet. Pair it from Link, then come back to chat.',
        'neutral',
      );
      finishGeneration();
      await drainQueue();
      return;
    }

    const toolCall = detectRemotePhoneToolIntent(text);
    if (toolCall) {
      try {
        const result = await dispatchRemotePhoneTool(host, toolCall.name, toolCall.args, {
          capabilities: adapters.capabilities(),
          now: adapters.now,
        });
        addAssistantMessage(result.content, 'neutral');
      } catch (toolError) {
        addAssistantMessage(`Remote tool failed: ${errorMessage(toolError)}`, 'sad');
      } finally {
        finishGeneration();
        await drainQueue();
      }
      return;
    }

    let fullText = '';
    try {
      for await (const chunk of host.streamChatMessage(text)) {
        if (cancelRequested.value) break;
        if (chunk.done) break;
        if (!chunk.text) continue;
        fullText += chunk.text;
        streamingText.value += chunk.text;
        isThinking.value = false;
        isStreaming.value = true;
      }

      if (!fullText.trim() && !cancelRequested.value) {
        fullText = await host.sendChatMessage(text);
      }
      if (fullText.trim() && !partialCommitted.value && !cancelRequested.value) {
        addParsedAssistantMessage(fullText);
      }
    } catch (error) {
      if (!fullText.trim() && !cancelRequested.value) {
        try {
          fullText = await host.sendChatMessage(text);
          addParsedAssistantMessage(fullText);
        } catch (fallbackError) {
          addAssistantMessage(`Remote chat failed: ${errorMessage(fallbackError)}`, 'sad');
        }
      } else if (!cancelRequested.value) {
        addAssistantMessage(`Remote chat stream ended early: ${errorMessage(error)}`, 'sad');
      }
    } finally {
      finishGeneration();
      await drainQueue();
    }
  }

  async function getConversation(): Promise<void> {
    // Phone-control chat streams from the desktop host, but history sync lives
    // in the broader device-sync path. Keep local session history in this view.
  }

  async function addMessage(message: Message): Promise<void> {
    messages.value.push(stampAgent({ ...message }));
  }

  async function rateCharismaTurn(_messageId: string, _rating: number): Promise<boolean> {
    return false;
  }

  function pushProviderWarning(): void {
    addAssistantMessage(
      'Remote provider warning: the desktop host is rotating providers or waiting for a configured brain.',
      'neutral',
    );
  }

  function stopGeneration(): void {
    cancelRequested.value = true;
    isThinking.value = false;
    isStreaming.value = false;
    streamingText.value = '';
  }

  function stopAndSend(): void {
    const partial = streamingText.value.trim();
    if (partial) {
      addParsedAssistantMessage(partial);
      partialCommitted.value = true;
    }
    stopGeneration();
  }

  function addToQueue(content: string): void {
    const text = content.trim();
    if (text) messageQueue.value.push(text);
  }

  function steerWithMessage(content: string): void {
    const text = content.trim();
    if (text) messageQueue.value.unshift(text);
  }

  async function drainQueue(): Promise<void> {
    if (generationActive.value || messageQueue.value.length === 0) return;
    const next = messageQueue.value.shift();
    if (next) await sendMessage(next);
  }

  function addParsedAssistantMessage(rawText: string): void {
    const parsed = parseTags(rawText.replace(/<pose>[\s\S]*?<\/pose>\s*/g, ''));
    addAssistantMessage(parsed.text || rawText.trim(), parsed.emotion ?? 'neutral', parsed.motion ?? undefined, parsed.emoji ?? undefined);
  }

  function addAssistantMessage(
    content: string,
    sentiment: Message['sentiment'] = 'neutral',
    motion?: string,
    emoji?: string,
  ): void {
    messages.value.push(stampAgent({
      id: currentAdapters().createId(),
      role: 'assistant',
      content,
      agentName: 'TerranSoul',
      sentiment,
      motion,
      emoji,
      timestamp: currentAdapters().now(),
    }));
  }

  function stampAgent(message: Message): Message {
    const agentId = activeAgentId();
    if (agentId) message.agentId = agentId;
    return message;
  }

  function finishGeneration(): void {
    generationActive.value = false;
    isThinking.value = false;
    isStreaming.value = false;
    streamingText.value = '';
    cancelRequested.value = false;
  }

  return {
    messages,
    currentAgent,
    agentMessages,
    agentSwitchHistory,
    isThinking,
    isStreaming,
    streamingText,
    translatorMode,
    messageQueue,
    generationActive,
    setAgent,
    sendMessage,
    rateCharismaTurn,
    getConversation,
    addMessage,
    pushProviderWarning,
    stopGeneration,
    stopAndSend,
    addToQueue,
    steerWithMessage,
  };
});

function currentAdapters(): RemoteConversationAdapters {
  return {
    remoteHost: adapterOverrides.remoteHost ?? defaultRemoteHost,
    capabilities: adapterOverrides.capabilities ?? defaultCapabilities,
    now: adapterOverrides.now ?? Date.now,
    createId: adapterOverrides.createId ?? createId,
  };
}

function defaultRemoteHost(): RemoteHost | null {
  const pairing = useMobilePairingStore();
  const credentials = pairing.storedRecord?.credentials;
  if (credentials?.desktopHost && credentials.desktopPort) {
    return createGrpcWebRemoteHost({
      baseUrl: remoteBaseUrl(credentials.desktopHost, credentials.desktopPort),
    });
  }
  return shouldUseRemoteConversation() ? null : createLocalRemoteHost();
}

function defaultCapabilities(): string[] | undefined {
  const pairing = useMobilePairingStore();
  return pairing.storedRecord?.credentials.capabilities;
}

function createId(): string {
  return typeof crypto !== 'undefined' && 'randomUUID' in crypto
    ? crypto.randomUUID()
    : `remote-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function errorMessage(value: unknown): string {
  return value instanceof Error ? value.message : String(value);
}