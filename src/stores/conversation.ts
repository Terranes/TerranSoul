import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { Message } from '../types';
import { useBrainStore } from './brain';
import { useStreamingStore } from './streaming';
import { useProviderHealthStore } from './provider-health';
import { useSkillTreeStore } from './skill-tree';
import { streamChatCompletion, buildHistory, getSystemPrompt } from '../utils/free-api-client';
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

// ── Chat-based LLM switching ─────────────────────────────────────────────────

/** Known free-provider keywords mapped to provider IDs. */
const PROVIDER_KEYWORDS: Record<string, string> = {
  pollinations: 'pollinations',
  groq: 'groq',
  cerebras: 'cerebras',
  siliconflow: 'siliconflow',
  mistral: 'mistral',
  'github models': 'github-models',
  openrouter: 'openrouter',
  nvidia: 'nvidia-nim',
  gemini: 'gemini',
};

/**
 * Detect if the user message is an LLM switching command.
 * Recognises patterns like:
 *   "switch to groq", "use pollinations", "change brain to cerebras",
 *   "use my openai api key sk-..."
 * Returns a result object if a command was detected, or null otherwise.
 */
function detectLlmCommand(text: string): {
  type: 'switch_free';
  providerId: string;
  providerName: string;
} | {
  type: 'switch_paid';
  provider: string;
  apiKey: string;
  model: string;
} | null {
  const lower = text.toLowerCase().trim();

  // Free provider switching: "switch to groq", "use cerebras", "change to pollinations"
  // Pattern: (switch|change|use|set) [to|brain to|model to|provider to] <provider_name> [api|provider|model|brain]
  const switchPattern = /(?:switch|change|use|set)\s+(?:to\s+|brain\s+to\s+|model\s+to\s+|provider\s+to\s+)?(\w[\w\s]*?)(?:\s+(?:api|provider|model|brain))?$/i;
  const match = lower.match(switchPattern);
  if (match) {
    const keyword = match[1].trim();
    for (const [name, id] of Object.entries(PROVIDER_KEYWORDS)) {
      if (keyword.includes(name)) {
        return { type: 'switch_free', providerId: id, providerName: name };
      }
    }
  }

  // Paid API: "use my openai api key sk-..." or "set openai key sk-..."
  const paidPattern = /(?:use|set)\s+(?:my\s+)?(\w+)\s+(?:api\s+)?key\s+(sk-\S+)/i;
  const paidMatch = text.match(paidPattern);
  if (paidMatch) {
    const provider = paidMatch[1].toLowerCase();
    const apiKey = paidMatch[2];
    const model = provider === 'anthropic' ? 'claude-sonnet-4-20250514' : 'gpt-4o';
    const baseUrl = provider === 'anthropic' ? 'https://api.anthropic.com' : 'https://api.openai.com';
    return { type: 'switch_paid', provider: baseUrl, apiKey, model };
  }

  return null;
}

/**
 * Execute an LLM switching command and return a confirmation message.
 */
async function executeLlmCommand(
  cmd: NonNullable<ReturnType<typeof detectLlmCommand>>,
  brain: ReturnType<typeof useBrainStore>,
): Promise<Message> {
  if (cmd.type === 'switch_free') {
    const provider = brain.freeProviders.find((p) => p.id === cmd.providerId);
    if (provider?.requires_api_key) {
      return {
        id: crypto.randomUUID(),
        role: 'assistant',
        content: `I'd love to switch to ${provider.display_name}, but it requires an API key. You can configure it in the Marketplace (🏪) under "Configure LLM".`,
        agentName: 'TerranSoul',
        sentiment: 'neutral',
        timestamp: Date.now(),
      };
    }
    const mode = { mode: 'free_api' as const, provider_id: cmd.providerId, api_key: null };
    try {
      await brain.setBrainMode(mode);
    } catch {
      brain.brainMode = mode;
    }
    const displayName = provider?.display_name ?? cmd.providerName;
    return {
      id: crypto.randomUUID(),
      role: 'assistant',
      content: `Done! I've switched to ${displayName}. Let's chat!`,
      agentName: 'TerranSoul',
      sentiment: 'happy',
      timestamp: Date.now(),
    };
  }

  // Paid API
  const mode = {
    mode: 'paid_api' as const,
    provider: cmd.provider,
    api_key: cmd.apiKey,
    model: cmd.model,
    base_url: cmd.provider,
  };
  try {
    await brain.setBrainMode(mode);
  } catch {
    brain.brainMode = mode;
  }
  return {
    id: crypto.randomUUID(),
    role: 'assistant',
    content: `Done! Paid API configured with model ${cmd.model}. Your API key is saved. Let's chat!`,
    agentName: 'TerranSoul',
    sentiment: 'happy',
    timestamp: Date.now(),
  };
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

// Re-export detectLlmCommand for tests
export { detectLlmCommand };

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

    // ── Quest system integration ──────────────────────────────────
    // Detect quest-related queries and trigger appropriate responses
    if (content.toLowerCase().includes('suggest') && content.toLowerCase().includes('where to start')) {
      const skillTree = useSkillTreeStore();
      const availableQuests = skillTree.nodes.filter(n => skillTree.getSkillStatus(n.id) === 'available');
      
      if (availableQuests.length > 0) {
        // Get the top recommended quest (you might want to implement AI-based recommendation here)
        const recommendedQuest = availableQuests[0]; // Simplified - use first available quest
        
        const questResponse: Message = {
          id: crypto.randomUUID(),
          role: 'assistant',
          content: `I'd recommend starting with **${recommendedQuest.name}**: ${recommendedQuest.description}\n\nThis would be a great foundation for your TerranSoul experience. Would you like me to help you set it up?`,
          agentName: 'TerranSoul',
          sentiment: 'happy',
          timestamp: Date.now(),
          questChoices: [
            { label: 'Yes, set it up for me', value: `auto-config:${recommendedQuest.id}`, icon: '⚔️' },
            { label: 'Not right now', value: 'dismiss', icon: '💤' },
            { label: 'Show all quests', value: 'navigate:skills', icon: '🗺️' },
          ],
          questId: recommendedQuest.id,
        };
        
        messages.value.push(questResponse);
        isThinking.value = false;
        return;
      }
    }

    // ── Quest analysis (silent background processing) ──────────────────
    // Check if the message might be quest-related and trigger background analysis
    const questKeywords = ['goal', 'quest', 'mission', 'task', 'adventure', 'journey', 'story', 'explore', 'build', 'create', 'learn'];
    const hasQuestContent = questKeywords.some(keyword => content.toLowerCase().includes(keyword));
    
    if (hasQuestContent) {
      // Trigger silent quest analysis in the background
      brain.processPromptSilently(`Analyze this user message for potential quest opportunities: "${content}". Provide brief insights on what quests or adventures might be relevant.`)
        .then((analysis) => {
          if (analysis) {
            // Add silent analysis result to conversation history for context
            const analysisMessage: Message = {
              id: crypto.randomUUID(),
              role: 'assistant',
              content: `*Background analysis: ${analysis}*`,
              agentName: 'QuestAnalyzer',
              sentiment: 'neutral',
              timestamp: Date.now(),
              system: true, // Mark as system message to hide from main chat UI
            };
            addMessage(analysisMessage);
          }
        })
        .catch(() => {
          // Silent failure - don't disrupt the main conversation
        });
    }

    const llmCmd = detectLlmCommand(content);
    if (llmCmd) {
      const response = await executeLlmCommand(llmCmd, brain);
      messages.value.push(response);
      isThinking.value = false;
      return;
    }

    // Path 1: Tauri backend available → use streaming IPC command
    if (isTauriAvailable()) {
      try {
        const streaming = useStreamingStore();

        // Don't set isStreaming immediately - wait for first chunk
        // Keep character in thinking state until text actually arrives

        // While `invoke` blocks, mirror `streaming.streamText` into this
        // store's `streamingText` at ~50ms intervals so reactive UI stays live.
        const syncInterval = setInterval(() => {
          streamingText.value = streaming.streamText;
          // Sync isStreaming state with the streaming store
          if (streaming.isStreaming && !isStreaming.value) {
            isStreaming.value = true;
          } else if (!streaming.isStreaming && isStreaming.value) {
            isStreaming.value = false;
          }
        }, 50);

        let sendOk = false;
        try {
          sendOk = await streaming.sendStreaming(content);
        } finally {
          clearInterval(syncInterval);
        }

        if (!sendOk) throw new Error(streaming.error ?? 'Streaming failed');

        // Grace period for any in-flight events after invoke resolves.
        if (streaming.isStreaming) {
          const graceWait = 3_000;
          const start = Date.now();
          while (streaming.isStreaming && Date.now() - start < graceWait) {
            streamingText.value = streaming.streamText;
            await new Promise((r) => setTimeout(r, 50));
          }
        }

        isStreaming.value = false;
        streamingText.value = '';

        // Create the final assistant message from accumulated text.
        // Text is already clean (anim blocks stripped by Rust parser).
        const cleanText = streaming.streamText;
        if (cleanText) {
          // Emotion comes from the streaming store (set by llm-animation events).
          const sentiment = streaming.currentEmotion ?? detectSentiment(content);
          const assistantMsg: Message = {
            id: crypto.randomUUID(),
            role: 'assistant',
            content: cleanText,
            agentName: 'TerranSoul',
            sentiment: sentiment as Message['sentiment'],
            timestamp: Date.now(),
          };
          messages.value.push(assistantMsg);
        } else {
          // Streaming completed but no text accumulated (events not received or
          // API returned empty) — fall back to non-streaming invoke which also
          // routes through brain_mode on the backend.
          throw new Error('Streaming produced no text');
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
            // Don't set isStreaming immediately - wait for first chunk
            streamingText.value = '';

            // Use enhanced prompt for non-Pollinations providers (upgraded models)
            const useEnhanced = provId !== 'pollinations';

            const fullText = await new Promise<string>((resolve, reject) => {
              streamChatCompletion(
                prov.baseUrl,
                prov.model,
                prov.apiKey,
                history,
                {
                  onChunk: (text) => {
                    // Set isStreaming only when first chunk arrives
                    if (!isStreaming.value && text) {
                      isStreaming.value = true;
                    }
                    streamingText.value += text;
                  },
                  onDone: (full) => resolve(full),
                  onError: (err) => reject(new Error(err)),
                },
                getSystemPrompt(useEnhanced),
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

  /** Add a message directly to the conversation without AI processing. */
  function addMessage(message: Message): void {
    messages.value.push(message);
  }

  return {
    messages,
    currentAgent,
    isThinking,
    streamingText,
    isStreaming,
    sendMessage,
    getConversation,
    addMessage,
  };
});
