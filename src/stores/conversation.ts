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

/**
 * Patterns that indicate a provider/service warning in the response text.
 * When detected, the warning is stripped and converted into a quest suggestion.
 */
const WARNING_PATTERNS = [
  /⚠️?\s*\*{0,2}IMPORTANT NOTICE\*{0,2}[\s\S]*?(?:deprecated|shutting down|end of life|discontinued)[\s\S]*?(?:\n\n|$)/i,
  /⚠️?\s*\*{0,2}(?:Deprecation|Service|Migration)\s*(?:Notice|Warning|Alert)\*{0,2}[:\s][\s\S]*?(?:\n\n|$)/i,
  /(?:^|\n)\[?(?:WARNING|NOTICE)\]?:?\s*(?:This|The)\s+(?:API|service|endpoint|model)[\s\S]*?(?:deprecated|removed|sunset|discontinued)[\s\S]*?(?:\n\n|$)/i,
];

/** Detect and extract a provider warning from response text. Returns cleaned text and the warning if found. */
function extractWarning(text: string): { clean: string; warning: string | null } {
  for (const pattern of WARNING_PATTERNS) {
    const match = text.match(pattern);
    if (match) {
      const warning = match[0].trim();
      const clean = text.replace(pattern, '').trim();
      return { clean, warning };
    }
  }
  return { clean: text, warning: null };
}

/** Convert a warning into quest choices attached to the message. */
function applyWarningAsQuest(msg: Message, _warning: string): void {
  msg.questId = 'migrate-brain';
  msg.questChoices = [
    { label: 'Upgrade to Paid API', value: 'navigate:brain-setup', icon: '⚡' },
    { label: 'Use llmfit (Local AI)', value: 'navigate:brain-setup', icon: '🏰' },
    { label: 'Switch Provider', value: 'navigate:marketplace', icon: '🔄' },
    { label: 'Dismiss', value: 'dismiss', icon: '💤' },
  ];
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
 * After the LLM responds, check if the user should be shown a quest overlay.
 *
 * Uses a hybrid approach:
 * 1. Check if the LLM response mentions quests (brain-driven detection)
 * 2. Check if the user's original message expressed getting-started intent
 *
 * If either signal is present AND there are available quests, show the overlay.
 * This avoids hard-coded regex that blocks the LLM, while still ensuring
 * quest suggestions appear reliably for getting-started queries.
 */
function maybeShowQuestFromResponse(responseText: string, userInput?: string): void {
  const responseLower = responseText.toLowerCase();
  const inputLower = (userInput ?? '').toLowerCase();

  // Signal 1: LLM response specifically mentions quests (word-boundary match to avoid
  // false positives like "requests" containing "quest" as a substring)
  const questWordRe = /\bquest\b/i;
  const hasResponseSignal = questWordRe.test(responseText) || responseLower.includes('skill tree');

  // Signal 2: User's intent is getting-started (broad keyword matching, not strict regex)
  const gettingStartedWords = ['start', 'begin', 'first', 'should i do', 'can i do', 'what next', 'get started', 'where do i', 'how do i'];
  const hasInputSignal = gettingStartedWords.some(w => inputLower.includes(w));

  if (!hasResponseSignal && !hasInputSignal) return;

  try {
    const skillTree = useSkillTreeStore();
    const availableQuests = skillTree.nodes.filter(n => {
      try { return skillTree.getSkillStatus(n.id) === 'available'; }
      catch { return false; }
    });
    if (availableQuests.length > 0) {
      skillTree.triggerQuestEvent(availableQuests[0].id);
    }
  } catch {
    // Skill tree not ready — skip quest overlay
  }
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

        const TAURI_STREAM_TIMEOUT_MS = 60_000; // 60s timeout
        let sendOk = false;
        try {
          sendOk = await Promise.race([
            streaming.sendStreaming(content),
            new Promise<boolean>((_, reject) =>
              setTimeout(() => reject(new Error('Tauri streaming timeout')), TAURI_STREAM_TIMEOUT_MS),
            ),
          ]);
        } finally {
          clearInterval(syncInterval);
        }

        if (!sendOk) throw new Error(streaming.error ?? 'Streaming failed');

        // Grace period for any in-flight events after invoke resolves.
        if (streaming.isStreaming) {
          const graceWait = 1_500;
          const start = Date.now();
          while (streaming.isStreaming && Date.now() - start < graceWait) {
            streamingText.value = streaming.streamText;
            await new Promise((r) => setTimeout(r, 50));
          }
        }

        isStreaming.value = false;
        streamingText.value = '';

        // Create the final assistant message from accumulated text.
        // Text is already clean (anim blocks stripped by Rust parser),
        // but the LLM may still return JSON-wrapped text outside tags.
        const parsed = parseTags(streaming.streamText);
        const cleanText = parsed.text;
        if (cleanText) {
          // Emotion comes from the streaming store (set by llm-animation events).
          const sentiment = streaming.currentEmotion ?? parsed.emotion ?? detectSentiment(content);
          const { clean, warning } = extractWarning(cleanText);
          const assistantMsg: Message = {
            id: crypto.randomUUID(),
            role: 'assistant',
            content: clean || cleanText,
            agentName: 'TerranSoul',
            sentiment: sentiment as Message['sentiment'],
            timestamp: Date.now(),
            emoji: parsed.emoji ?? undefined,
          };
          if (warning) applyWarningAsQuest(assistantMsg, warning);
          messages.value.push(assistantMsg);
          maybeShowQuestFromResponse(clean || cleanText, content);
        } else {
          // Streaming completed but no text accumulated (events not received or
          // API returned empty) — fall back to non-streaming invoke which also
          // routes through brain_mode on the backend.
          throw new Error('Streaming produced no text');
        }
        streaming.reset();
      } catch {
        // Tauri streaming failed — fall back to non-streaming invoke with timeout
        try {
          const FALLBACK_TIMEOUT_MS = 30_000;
          const response = await Promise.race([
            invoke<Message>('send_message', {
              message: content,
              agentId: currentAgent.value === 'auto' ? null : currentAgent.value,
            }),
            new Promise<never>((_, reject) =>
              setTimeout(() => reject(new Error('Fallback invoke timeout')), FALLBACK_TIMEOUT_MS),
            ),
          ]);
          messages.value.push(response);
          maybeShowQuestFromResponse(response.content, content);
        } catch {
          messages.value.push(createPersonaResponse(content));
          pushNetworkOrProviderWarning();
        }
      } finally {
        isThinking.value = false;
        isStreaming.value = false;
        streamingText.value = '';
      }
      return;
    }

    // Path 2: No Tauri, but brain is configured with free API → browser-side streaming
    if (brain.hasBrain) {
      const provider = resolveFreeProvider(brain);
      if (provider) {
        try {
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

            const STREAM_TIMEOUT_MS = 30_000; // 30s timeout to prevent stuck thinking
            const fullText = await new Promise<string>((resolve, reject) => {
              let timeout: ReturnType<typeof setTimeout> | undefined;
              let settled = false;
              const abortController = streamChatCompletion(
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
                  onDone: (full) => { if (!settled) { settled = true; clearTimeout(timeout); resolve(full); } },
                  onError: (err) => { if (!settled) { settled = true; clearTimeout(timeout); reject(new Error(err)); } },
                },
                getSystemPrompt(useEnhanced),
              );
              timeout = setTimeout(() => {
                if (!settled) { settled = true; abortController.abort(); reject(new Error('Stream timeout: no response within 60s')); }
              }, STREAM_TIMEOUT_MS);
            });

            isStreaming.value = false;
            streamingText.value = '';

            const parsed = parseTags(fullText);
            // Use keyword detection on user input as fallback when LLM has no emotion tags
            const sentiment = parsed.emotion ?? detectSentiment(content);
            const { clean, warning } = extractWarning(parsed.text);
            const assistantMsg: Message = {
              id: crypto.randomUUID(),
              role: 'assistant',
              content: clean || parsed.text,
              agentName: 'TerranSoul',
              sentiment: sentiment as Message['sentiment'],
              timestamp: Date.now(),
              emoji: parsed.emoji ?? undefined,
            };
            if (warning) applyWarningAsQuest(assistantMsg, warning);
            messages.value.push(assistantMsg);
            maybeShowQuestFromResponse(clean || parsed.text, content);
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
            // Check for network / timeout errors
            if (errMsg.includes('Network error') || errMsg.includes('Network timeout') || errMsg.includes('Stream timeout') || errMsg.includes('Failed to fetch')) {
              // Network issue — try next provider
              continue;
            }
            // Other error — don't retry
            break;
          }
        }

        if (!succeeded) {
          messages.value.push(createPersonaResponse(content));
          pushNetworkOrProviderWarning();
        }
        } finally {
          isThinking.value = false;
          isStreaming.value = false;
          streamingText.value = '';
        }
        return;
      }
    }

    // Path 3: No brain configured — persona fallback
    await new Promise((r) => setTimeout(r, 500));
    messages.value.push(createPersonaResponse(content));
    isThinking.value = false;
  }

  /** Push a warning message with upgrade quest when all providers are exhausted. */
  function pushProviderWarning(): void {
    // Avoid duplicate warnings within the last few messages
    const recent = messages.value.slice(-5);
    if (recent.some(m => m.agentName === 'System' && m.content.includes('rate limit'))) return;

    messages.value.push({
      id: crypto.randomUUID(),
      role: 'assistant',
      content:
        '⚠️ The free AI providers are currently rate-limited. ' +
        'Responses may be slower or use a basic fallback until limits reset.\n\n' +
        'You can upgrade to a paid API or use llmfit to run a local model for unlimited, faster responses!',
      agentName: 'System',
      sentiment: 'neutral',
      timestamp: Date.now(),
      questChoices: [
        { label: 'Upgrade to Paid API', value: 'navigate:brain-setup', icon: '⚡' },
        { label: 'Use llmfit (Local AI)', value: 'navigate:brain-setup', icon: '🏰' },
        { label: 'I\'ll wait', value: 'dismiss', icon: '⏳' },
      ],
      questId: 'paid-brain',
    });
  }

  /**
   * Detect whether the failure was network-related and push the appropriate
   * warning. Falls back to the rate-limit warning when no network signal is
   * found.
   */
  function pushNetworkOrProviderWarning(): void {
    const recent = messages.value.slice(-5);
    if (recent.some(m => m.agentName === 'System' && (m.content.includes('network') || m.content.includes('rate limit')))) return;

    messages.value.push({
      id: crypto.randomUUID(),
      role: 'assistant',
      content:
        '⚠️ Could not reach the AI provider — this is likely a network issue or the service is temporarily down.\n\n' +
        'For a more reliable experience, consider installing a **local LLM** so TerranSoul can work offline without depending on external servers.',
      agentName: 'System',
      sentiment: 'neutral',
      timestamp: Date.now(),
      questChoices: [
        { label: 'Install Local LLM (Recommended)', value: 'navigate:brain-setup', icon: '🏰' },
        { label: 'Upgrade to Paid API', value: 'navigate:brain-setup', icon: '⚡' },
        { label: 'Retry', value: 'retry', icon: '🔄' },
      ],
      questId: 'local-brain',
    });
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
    pushProviderWarning,
  };
});
