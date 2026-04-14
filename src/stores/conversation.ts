import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { Message } from '../types';
import { useBrainStore } from './brain';
import { useStreamingStore } from './streaming';
import { useProviderHealthStore } from './provider-health';
import { streamChatCompletion, buildHistory } from '../utils/free-api-client';
import { parseTags } from '../utils/emotion-parser';

/**
 * Keyword-based sentiment detection from text content.
 * Used as a fallback when the LLM response doesn't include emotion tags.
 * Checks both user input and assistant response for emotional cues.
 */
export function detectSentiment(text: string): 'happy' | 'sad' | 'angry' | 'relaxed' | 'surprised' | 'neutral' {
  const lower = text.toLowerCase();

  if (lower.includes('angry') || lower.includes('furious') || lower.includes('annoyed') || lower.includes('frustrat')) {
    return 'angry';
  }
  if (lower.includes('surprise') || lower.includes('wow') || lower.includes('unexpected') || lower.includes('amazing') || lower.includes('whoa') || lower.includes('omg')) {
    return 'surprised';
  }
  if (lower.includes('relax') || lower.includes('calm') || lower.includes('peaceful') || lower.includes('chill') || lower.includes('meditat')) {
    return 'relaxed';
  }
  if (lower.includes('sad') || lower.includes('bad') || lower.includes('hate') || lower.includes('sorry') || lower.includes('cry')) {
    return 'sad';
  }
  if (lower.includes('hello') || /\bhi\b/.test(lower) || lower.startsWith('hey') || lower.includes('happy') || lower.includes('great') || lower.includes('awesome') || lower.includes('love')) {
    return 'happy';
  }
  return 'neutral';
}

/** Browser-side persona fallback when no brain is configured at all. */
function createPersonaResponse(content: string): Message {
  const sentiment = detectSentiment(content);
  let response: string;

  switch (sentiment) {
    case 'angry':
      response = "I can sense your frustration. Take a deep breath — I'm here to help. 🔥";
      break;
    case 'surprised':
      response = "Wow, that's surprising! Tell me more! 😮";
      break;
    case 'relaxed':
      response = "That sounds so peaceful. Let's take a moment to enjoy the calm. 🧘";
      break;
    case 'sad':
      response = "I understand you're going through something difficult. I'm here for you. 💙";
      break;
    case 'happy':
      response = "That's wonderful to hear! Your positive energy is contagious! ✨";
      break;
    default:
      response = `Hello! I'm TerranSoul. Please configure a brain (free cloud API or paid API) in the Marketplace so I can have a real conversation with you!`;
      break;
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

/** Detect if the Tauri IPC bridge is available (synchronous check). */
function isTauriAvailable(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

/**
 * Resolve the free provider details (base_url, model, api_key) from the brain store
 * for browser-side streaming.
 */
function resolveFreeProvider(brain: ReturnType<typeof useBrainStore>): {
  baseUrl: string;
  model: string;
  apiKey: string | null;
} | null {
  if (!brain.brainMode || brain.brainMode.mode !== 'free_api') return null;

  const providerId = brain.brainMode.provider_id;
  const apiKey = brain.brainMode.api_key ?? null;
  const provider = brain.freeProviders.find((p) => p.id === providerId);
  if (!provider) return null;

  return {
    baseUrl: provider.base_url,
    model: provider.model,
    apiKey,
  };
}

export const useConversationStore = defineStore('conversation', () => {
  const messages = ref<Message[]>([]);
  const currentAgent = ref<string>('auto');
  const isThinking = ref(false);
  /** Live streaming text shown in the UI while the LLM is generating. */
  const streamingText = ref('');
  /** Whether a streaming response is in progress. */
  const isStreaming = ref(false);

  async function sendMessage(content: string) {
    const userMsg: Message = {
      id: crypto.randomUUID(),
      role: 'user',
      content,
      timestamp: Date.now(),
    };
    messages.value.push(userMsg);
    isThinking.value = true;
    streamingText.value = '';
    isStreaming.value = false;

    const brain = useBrainStore();

    // Path 1: Tauri backend available → use streaming IPC command
    if (isTauriAvailable()) {
      try {
        const streaming = useStreamingStore();
        const ok = await streaming.sendStreaming(content);
        if (!ok) throw new Error(streaming.error ?? 'Streaming failed');

        // The actual chunk events come via Tauri events (llm-chunk).
        // ChatView wires up the event listener and calls streaming.handleChunk().
        // We wait for the stream to finish by polling streaming.isStreaming.
        // To avoid blocking forever, use a timeout.
        const maxWait = 120_000; // 2 minutes
        const start = Date.now();
        while (streaming.isStreaming && Date.now() - start < maxWait) {
          streamingText.value = streaming.streamText;
          isStreaming.value = true;
          await new Promise((r) => setTimeout(r, 100));
        }

        isStreaming.value = false;
        streamingText.value = '';

        // Create the final assistant message from accumulated text
        const finalText = streaming.streamText;
        if (finalText) {
          const parsed = parseTags(finalText);
          // Use keyword detection on user input as fallback when LLM has no emotion tags
          const sentiment = parsed.emotion ?? detectSentiment(content);
          const assistantMsg: Message = {
            id: crypto.randomUUID(),
            role: 'assistant',
            content: finalText,
            agentName: 'TerranSoul',
            sentiment: sentiment as Message['sentiment'],
            timestamp: Date.now(),
          };
          messages.value.push(assistantMsg);
        }
        streaming.reset();
      } catch {
        // Tauri streaming failed — fall back to non-streaming invoke
        try {
          const response = await invoke<Message>('send_message', {
            message: content,
            agentId: currentAgent.value === 'auto' ? null : currentAgent.value,
          });
          messages.value.push(response);
        } catch {
          messages.value.push(createPersonaResponse(content));
        }
      } finally {
        isThinking.value = false;
      }
      return;
    }

    // Path 2: No Tauri, but brain is configured with free API → browser-side streaming
    if (brain.hasBrain) {
      const provider = resolveFreeProvider(brain);
      if (provider) {
        const healthStore = useProviderHealthStore();
        const history = buildHistory(
          messages.value.map((m) => ({ role: m.role, content: m.content })),
        );

        // Try the primary provider, then rotate to next healthy on rate-limit
        const providersToTry = [provider];
        // Add fallback providers from the brain store
        const primaryProviderId = brain.brainMode?.mode === 'free_api' ? brain.brainMode.provider_id : '';
        for (const fp of brain.freeProviders) {
          if (fp.id !== primaryProviderId && !providersToTry.some((p) => p.baseUrl === fp.base_url)) {
            providersToTry.push({ baseUrl: fp.base_url, model: fp.model, apiKey: provider.apiKey });
          }
        }

        let succeeded = false;
        for (const prov of providersToTry) {
          const provId = brain.freeProviders.find((f) => f.base_url === prov.baseUrl)?.id ?? 'unknown';
          // Skip if already known to be rate-limited
          const healthInfo = healthStore.providers.find((p) => p.id === provId);
          if (healthInfo?.is_rate_limited) continue;

          try {
            isStreaming.value = true;
            streamingText.value = '';

            const fullText = await new Promise<string>((resolve, reject) => {
              streamChatCompletion(
                prov.baseUrl,
                prov.model,
                prov.apiKey,
                history,
                {
                  onChunk: (text) => {
                    streamingText.value += text;
                  },
                  onDone: (full) => resolve(full),
                  onError: (err) => reject(new Error(err)),
                },
              );
            });

            isStreaming.value = false;
            streamingText.value = '';

            const parsed = parseTags(fullText);
            // Use keyword detection on user input as fallback when LLM has no emotion tags
            const sentiment = parsed.emotion ?? detectSentiment(content);
            const assistantMsg: Message = {
              id: crypto.randomUUID(),
              role: 'assistant',
              content: parsed.text,
              agentName: 'TerranSoul',
              sentiment: sentiment as Message['sentiment'],
              timestamp: Date.now(),
            };
            messages.value.push(assistantMsg);
            succeeded = true;
            break;
          } catch (err) {
            isStreaming.value = false;
            streamingText.value = '';
            // Check if it's a rate-limit error — mark and try next provider
            const errMsg = String(err);
            if (errMsg.includes('429') || errMsg.toLowerCase().includes('rate limit')) {
              healthStore.markRateLimited(provId);
              continue; // Try next provider
            }
            // Other error — don't retry
            break;
          }
        }

        if (!succeeded) {
          messages.value.push(createPersonaResponse(content));
        }
        isThinking.value = false;
        return;
      }
    }

    // Path 3: No brain configured — persona fallback
    await new Promise((r) => setTimeout(r, 500));
    messages.value.push(createPersonaResponse(content));
    isThinking.value = false;
  }

  async function getConversation() {
    try {
      const history = await invoke<Message[]>('get_conversation');
      messages.value = history;
    } catch {
      // ignore
    }
  }

  return {
    messages,
    currentAgent,
    isThinking,
    streamingText,
    isStreaming,
    sendMessage,
    getConversation,
  };
});
