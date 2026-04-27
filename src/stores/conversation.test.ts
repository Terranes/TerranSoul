/**
 * Integration tests for the conversation store.
 *
 * The conversation store now has three paths:
 *  1. Tauri backend available (window.__TAURI_INTERNALS__) → streaming IPC
 *  2. No Tauri but brain configured → browser-side free API
 *  3. No brain → persona fallback
 *
 * In jsdom tests, __TAURI_INTERNALS__ is absent unless explicitly set,
 * so tests exercise paths 2 and 3 by default.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { useConversationStore } from './conversation';
import { useBrainStore } from './brain';
import { useAiDecisionPolicyStore } from './ai-decision-policy';
import type { Message } from '../types';

// Mock the Tauri invoke API
const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

// Mock the free-api-client to avoid real HTTP calls
const mockStreamChat = vi.fn();
vi.mock('../utils/free-api-client', () => ({
  streamChatCompletion: (...args: unknown[]) => mockStreamChat(...args),
  buildHistory: (msgs: Array<{ role: string; content: string }>, limit = 20) =>
    msgs.slice(-limit).map((m: { role: string; content: string }) => ({
      role: m.role,
      content: m.content,
    })),
  getSystemPrompt: () => 'You are TerranSoul.',
}));

describe('conversation store — no brain (persona fallback)', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockStreamChat.mockReset();
  });

  it('sendMessage uses persona fallback when no brain is configured', async () => {
    const store = useConversationStore();
    await store.sendMessage('hello');

    expect(store.isThinking).toBe(false);
    expect(store.messages).toHaveLength(2);
    expect(store.messages[0].role).toBe('user');
    expect(store.messages[0].content).toBe('hello');
    expect(store.messages[1].role).toBe('assistant');
    expect(store.messages[1].agentName).toBe('TerranSoul');
    expect(store.messages[1].sentiment).toBe('happy'); // "hello" triggers happy
  });

  it('persona fallback detects sadness', async () => {
    const store = useConversationStore();
    await store.sendMessage('I am sad today');

    expect(store.messages[1].sentiment).toBe('sad');
  });

  it('persona fallback default message no longer echoes input', async () => {
    const store = useConversationStore();
    await store.sendMessage('How are you?');

    expect(store.messages[1].content).not.toContain('You said:');
    expect(store.messages[1].content).toContain('configure a brain');
  });

  it('multiple messages accumulate in correct order', async () => {
    const store = useConversationStore();
    await store.sendMessage('hello');
    await store.sendMessage('I feel sad');

    expect(store.messages).toHaveLength(4);
    expect(store.messages[0].content).toBe('hello');
    expect(store.messages[0].role).toBe('user');
    expect(store.messages[1].role).toBe('assistant');
    expect(store.messages[1].sentiment).toBe('happy');
    expect(store.messages[2].content).toBe('I feel sad');
    expect(store.messages[2].role).toBe('user');
    expect(store.messages[3].role).toBe('assistant');
    expect(store.messages[3].sentiment).toBe('sad');
  });

  it('isThinking is set and cleared during persona fallback', async () => {
    const store = useConversationStore();
    const promise = store.sendMessage('hello');
    expect(store.isThinking).toBe(true);
    await promise;
    expect(store.isThinking).toBe(false);
  });
});

describe('conversation store — brain configured (browser-side free API)', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockStreamChat.mockReset();
  });

  it('calls free API when brain is configured', async () => {
    const brain = useBrainStore();
    brain.autoConfigureFreeApi();

    // Mock streamChatCompletion to call onDone immediately
    mockStreamChat.mockImplementation(
      (_baseUrl: string, _model: string, _apiKey: string | null, _history: unknown[], callbacks: { onDone: (text: string) => void }) => {
        callbacks.onDone('[happy] Hello! Great to see you!');
        return new AbortController();
      },
    );

    const store = useConversationStore();
    await store.sendMessage('hello');

    expect(mockStreamChat).toHaveBeenCalled();
    expect(store.messages).toHaveLength(2);
    expect(store.messages[1].role).toBe('assistant');
    expect(store.messages[1].content).toBe('Hello! Great to see you!'); // tags stripped
    expect(store.messages[1].sentiment).toBe('happy');
  });

  it('streams chunks to streamingText during generation', async () => {
    const brain = useBrainStore();
    brain.autoConfigureFreeApi();

    let capturedCallbacks: { onChunk: (t: string) => void; onDone: (t: string) => void } | null = null;
    mockStreamChat.mockImplementation(
      (_baseUrl: string, _model: string, _apiKey: string | null, _history: unknown[], callbacks: { onChunk: (t: string) => void; onDone: (t: string) => void }) => {
        capturedCallbacks = callbacks;
        // Simulate delayed chunks
        setTimeout(() => {
          callbacks.onChunk('Hello ');
          callbacks.onChunk('world!');
          callbacks.onDone('Hello world!');
        }, 10);
        return new AbortController();
      },
    );

    const store = useConversationStore();
    await store.sendMessage('hi');

    expect(capturedCallbacks).not.toBeNull();
    expect(store.messages).toHaveLength(2);
    expect(store.messages[1].content).toBe('Hello world!');
  });

  it('falls back to persona on free API error', async () => {
    const brain = useBrainStore();
    brain.autoConfigureFreeApi();

    mockStreamChat.mockImplementation(
      (_baseUrl: string, _model: string, _apiKey: string | null, _history: unknown[], callbacks: { onError: (err: string) => void }) => {
        callbacks.onError('HTTP 429: Rate limited');
        return new AbortController();
      },
    );

    const store = useConversationStore();
    await store.sendMessage('hello');

    // Should fall back to persona + show provider warning
    expect(store.messages).toHaveLength(3);
    expect(store.messages[1].role).toBe('assistant');
    expect(store.messages[1].agentName).toBe('TerranSoul');
    expect(store.messages[2].agentName).toBe('System');
    expect(store.messages[2].content).toContain('Could not reach the AI provider');
    expect(store.isThinking).toBe(false);
  });
});

describe('conversation store — Tauri backend available', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockStreamChat.mockReset();
    // Simulate Tauri environment
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
  });

  afterEach(() => {
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  });

  it('uses streaming IPC when Tauri is available', async () => {
    // Simulate chunks arriving during the invoke call via the streaming store.
    // In the real app, Tauri events fire during the invoke; here we inject
    // chunks inside the mocked invoke so they arrive before it resolves.
    const { useStreamingStore } = await import('./streaming');
    const streaming = useStreamingStore();

    mockInvoke.mockImplementation(async () => {
      // Simulate chunks arriving during the invoke
      streaming.handleChunk({ text: 'Hi there!', done: false });
      streaming.handleChunk({ text: '', done: true });
    });

    const store = useConversationStore();
    await store.sendMessage('hello');

    expect(mockInvoke).toHaveBeenCalledWith('send_message_stream', { message: 'hello' });
    expect(store.messages).toHaveLength(2);
    expect(store.messages[1].role).toBe('assistant');
    expect(store.messages[1].content).toBe('Hi there!');
    expect(store.isThinking).toBe(false);
    expect(store.isStreaming).toBe(false);
  });

  it('falls back to send_message on streaming failure', async () => {
    // First call (send_message_stream) rejects
    mockInvoke.mockRejectedValueOnce(new Error('stream not supported'));
    // Second call (send_message) succeeds
    const serverResponse: Message = {
      id: 'resp-fb',
      role: 'assistant',
      content: 'Hello via fallback!',
      agentName: 'TerranSoul',
      sentiment: 'happy',
      timestamp: Date.now(),
    };
    mockInvoke.mockResolvedValueOnce(serverResponse);

    const store = useConversationStore();
    await store.sendMessage('hello');

    expect(store.messages).toHaveLength(2);
    expect(store.messages[1]).toEqual(serverResponse);
  });

  it('falls back to persona when both streaming and invoke fail', async () => {
    mockInvoke.mockRejectedValue(new Error('all failed'));

    const store = useConversationStore();
    await store.sendMessage('hello');

    expect(store.messages).toHaveLength(3);
    expect(store.messages[1].role).toBe('assistant');
    expect(store.messages[1].agentName).toBe('TerranSoul');
    expect(store.messages[2].agentName).toBe('System');
    expect(store.messages[2].content).toContain('Could not reach the AI provider');
    expect(store.isThinking).toBe(false);
  });

  it('falls back to non-streaming invoke when streaming produces no text', async () => {
    // First invoke (send_message_stream) resolves OK but no chunks arrive
    mockInvoke.mockResolvedValueOnce(undefined);
    // Second invoke (send_message) returns a proper response
    const fallbackResponse: Message = {
      id: 'fb-1',
      role: 'assistant',
      content: 'Hello from non-streaming!',
      agentName: 'TerranSoul',
      sentiment: 'happy',
      timestamp: Date.now(),
    };
    mockInvoke.mockResolvedValueOnce(fallbackResponse);

    const store = useConversationStore();
    const promise = store.sendMessage('Hi');

    await new Promise((r) => setTimeout(r, 150));

    // Simulate the streaming ending with zero content (empty done chunk)
    const { useStreamingStore } = await import('./streaming');
    const streaming = useStreamingStore();
    streaming.handleChunk({ text: '', done: true });

    await promise;

    // Should fall back to non-streaming invoke and use its response
    expect(store.messages).toHaveLength(2);
    expect(store.messages[0].content).toBe('Hi');
    expect(store.messages[1]).toEqual(fallbackResponse);
    expect(store.isThinking).toBe(false);
  });
});

describe('conversation store — getConversation', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it('populates store from backend history', async () => {
    const history: Message[] = [
      { id: 'h1', role: 'user', content: 'first', timestamp: 1000 },
      { id: 'h2', role: 'assistant', content: 'response 1', agentName: 'TerranSoul', timestamp: 1001 },
      { id: 'h3', role: 'user', content: 'second', timestamp: 2000 },
      { id: 'h4', role: 'assistant', content: 'response 2', agentName: 'TerranSoul', timestamp: 2001 },
    ];
    mockInvoke.mockResolvedValueOnce(history);

    const store = useConversationStore();
    await store.getConversation();

    expect(mockInvoke).toHaveBeenCalledWith('get_conversation');
    expect(store.messages).toEqual(history);
    expect(store.messages).toHaveLength(4);
  });

  it('silently ignores errors', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('connection lost'));

    const store = useConversationStore();
    await store.getConversation();

    expect(store.messages).toHaveLength(0);
  });
});

describe('detectSentiment — keyword-based fallback', () => {
  it('detects happy from greetings', async () => {
    const { detectSentiment } = await import('./conversation');
    expect(detectSentiment('Hello!')).toBe('happy');
    expect(detectSentiment('Hey there')).toBe('happy');
    expect(detectSentiment('hi')).toBe('happy');
  });

  it('detects happy from positive keywords', async () => {
    const { detectSentiment } = await import('./conversation');
    expect(detectSentiment('I feel happy today')).toBe('happy');
    expect(detectSentiment('That was awesome')).toBe('happy');
    expect(detectSentiment('I love this')).toBe('happy');
  });

  it('detects sad from negative keywords', async () => {
    const { detectSentiment } = await import('./conversation');
    expect(detectSentiment('I feel so sad today')).toBe('sad');
    expect(detectSentiment('This is bad')).toBe('sad');
  });

  it('detects angry from frustration keywords', async () => {
    const { detectSentiment } = await import('./conversation');
    expect(detectSentiment('I am so angry!')).toBe('angry');
    expect(detectSentiment('This is frustrating')).toBe('angry');
  });

  it('detects relaxed from calm keywords', async () => {
    const { detectSentiment } = await import('./conversation');
    expect(detectSentiment('I want to relax')).toBe('relaxed');
    expect(detectSentiment('So calm and peaceful')).toBe('relaxed');
  });

  it('detects surprised from exclamation keywords', async () => {
    const { detectSentiment } = await import('./conversation');
    expect(detectSentiment('Wow that is so surprising!')).toBe('surprised');
    expect(detectSentiment('That was amazing')).toBe('surprised');
  });

  it('returns neutral for unknown content', async () => {
    const { detectSentiment } = await import('./conversation');
    expect(detectSentiment('What is the weather like?')).toBe('neutral');
  });
});

describe('detectLlmCommand — chat-based LLM switching', () => {
  it('detects "switch to groq"', async () => {
    const { detectLlmCommand } = await import('./conversation');
    const cmd = detectLlmCommand('switch to groq');
    expect(cmd).not.toBeNull();
    expect(cmd!.type).toBe('switch_free');
    if (cmd!.type === 'switch_free') {
      expect(cmd!.providerId).toBe('groq');
    }
  });

  it('detects "use pollinations"', async () => {
    const { detectLlmCommand } = await import('./conversation');
    const cmd = detectLlmCommand('use pollinations');
    expect(cmd).not.toBeNull();
    expect(cmd!.type).toBe('switch_free');
    if (cmd!.type === 'switch_free') {
      expect(cmd!.providerId).toBe('pollinations');
    }
  });

  it('detects "change to cerebras"', async () => {
    const { detectLlmCommand } = await import('./conversation');
    const cmd = detectLlmCommand('change to cerebras');
    expect(cmd).not.toBeNull();
    expect(cmd!.type).toBe('switch_free');
    if (cmd!.type === 'switch_free') {
      expect(cmd!.providerId).toBe('cerebras');
    }
  });

  it('detects "switch to mistral"', async () => {
    const { detectLlmCommand } = await import('./conversation');
    const cmd = detectLlmCommand('switch to mistral');
    expect(cmd).not.toBeNull();
    expect(cmd!.type).toBe('switch_free');
    if (cmd!.type === 'switch_free') {
      expect(cmd!.providerId).toBe('mistral');
    }
  });

  it('detects paid API key command', async () => {
    const { detectLlmCommand } = await import('./conversation');
    const cmd = detectLlmCommand('use my openai api key sk-abc123def456');
    expect(cmd).not.toBeNull();
    expect(cmd!.type).toBe('switch_paid');
    if (cmd!.type === 'switch_paid') {
      expect(cmd!.apiKey).toBe('sk-abc123def456');
    }
  });

  it('returns null for normal messages', async () => {
    const { detectLlmCommand } = await import('./conversation');
    expect(detectLlmCommand('What is the weather today?')).toBeNull();
    expect(detectLlmCommand('Tell me a joke')).toBeNull();
    expect(detectLlmCommand('How does machine learning work?')).toBeNull();
  });
});

describe('detectTeachIntent — explicit teach-the-AI detection', () => {
  it('triggers on "remember the following law: …"', async () => {
    const { detectTeachIntent } = await import('./conversation');
    const res = detectTeachIntent('Remember the following law: Article 429 says claims expire after 3 years.');
    expect(res).not.toBeNull();
    expect(res!.topic).toMatch(/article 429/i);
  });

  it('triggers on "please learn the following rule:"', async () => {
    const { detectTeachIntent } = await import('./conversation');
    const res = detectTeachIntent('please learn the following rule: no liability in force majeure');
    expect(res).not.toBeNull();
  });

  it('triggers on "memorize this law: …"', async () => {
    const { detectTeachIntent } = await import('./conversation');
    const res = detectTeachIntent('memorize this law: Article 351 on civil liability');
    expect(res).not.toBeNull();
    expect(res!.topic).toMatch(/article 351/i);
  });

  it('triggers on "ingest this document: URL"', async () => {
    const { detectTeachIntent } = await import('./conversation');
    const res = detectTeachIntent('ingest this document: https://example.com/law.pdf');
    expect(res).not.toBeNull();
  });

  it('triggers on "provide my own context"', async () => {
    const { detectTeachIntent } = await import('./conversation');
    const res = detectTeachIntent('provide my own context');
    expect(res).not.toBeNull();
  });

  it('does NOT trigger on a plain "I want to learn about X" question', async () => {
    const { detectTeachIntent } = await import('./conversation');
    // This is the critical regression: a question about a topic should
    // never be read as an instruction to ingest training material.
    expect(detectTeachIntent('I want to learn about Vietnamese civil law')).toBeNull();
    expect(detectTeachIntent('Teach me about contract liability')).toBeNull();
    expect(detectTeachIntent('Can you study Vietnamese law with me?')).toBeNull();
    expect(detectTeachIntent('What is the statute of limitations?')).toBeNull();
    expect(detectTeachIntent('Tell me about Article 429')).toBeNull();
  });
});

describe('detectDontKnow — uncertainty detection', () => {
  it('detects "I don\'t know"', async () => {
    const { detectDontKnow } = await import('./conversation');
    expect(detectDontKnow("I don't know the answer to that.")).toBe(true);
    expect(detectDontKnow('I do not know.')).toBe(true);
  });

  it('detects "I\'m not sure"', async () => {
    const { detectDontKnow } = await import('./conversation');
    expect(detectDontKnow("I'm not sure about this specific statute.")).toBe(true);
    expect(detectDontKnow('I am not certain of the answer.')).toBe(true);
  });

  it('detects "I cannot confirm"', async () => {
    const { detectDontKnow } = await import('./conversation');
    expect(detectDontKnow('I cannot confirm the current Vietnamese law.')).toBe(true);
    expect(detectDontKnow("I can't say for certain.")).toBe(true);
  });

  it('detects "no specific information"', async () => {
    const { detectDontKnow } = await import('./conversation');
    expect(detectDontKnow('I have no specific information about that article.')).toBe(true);
    expect(detectDontKnow('There is no reliable information available.')).toBe(true);
  });

  it('detects "my training data is limited"', async () => {
    const { detectDontKnow } = await import('./conversation');
    expect(detectDontKnow('My training data is limited and may not include this.')).toBe(true);
    expect(detectDontKnow("My knowledge doesn't cover this specifically.")).toBe(true);
  });

  it('detects "beyond my knowledge"', async () => {
    const { detectDontKnow } = await import('./conversation');
    expect(detectDontKnow('That is beyond my training cutoff.')).toBe(true);
  });

  it('does NOT trigger on a confident answer', async () => {
    const { detectDontKnow } = await import('./conversation');
    expect(detectDontKnow('The statute of limitations is 3 years under Article 429.')).toBe(false);
    expect(detectDontKnow('Article 351 governs civil liability for breach of contract.')).toBe(false);
  });
});

describe('detectGatedSetupCommand — user confirmation commands', () => {
  it('detects "upgrade to Gemini model"', async () => {
    const { detectGatedSetupCommand } = await import('./conversation');
    const res = detectGatedSetupCommand('upgrade to Gemini model');
    expect(res).not.toBeNull();
    expect(res!.type).toBe('upgrade_gemini');
  });

  it('detects "upgrade to Gemini" without "model"', async () => {
    const { detectGatedSetupCommand } = await import('./conversation');
    const res = detectGatedSetupCommand('upgrade to Gemini');
    expect(res).not.toBeNull();
    expect(res!.type).toBe('upgrade_gemini');
  });

  it('tolerates the "Gemni" typo', async () => {
    const { detectGatedSetupCommand } = await import('./conversation');
    const res = detectGatedSetupCommand('upgrade to Gemni model');
    expect(res).not.toBeNull();
    expect(res!.type).toBe('upgrade_gemini');
  });

  it('detects "provide your own context"', async () => {
    const { detectGatedSetupCommand } = await import('./conversation');
    const res = detectGatedSetupCommand('provide your own context');
    expect(res).not.toBeNull();
    expect(res!.type).toBe('provide_context');
  });

  it('detects "provide my own context"', async () => {
    const { detectGatedSetupCommand } = await import('./conversation');
    const res = detectGatedSetupCommand('provide my own context');
    expect(res).not.toBeNull();
    expect(res!.type).toBe('provide_context');
  });

  it('tolerates the "provde" typo', async () => {
    const { detectGatedSetupCommand } = await import('./conversation');
    const res = detectGatedSetupCommand('provde your own context');
    expect(res).not.toBeNull();
    expect(res!.type).toBe('provide_context');
  });

  it('returns null on unrelated text', async () => {
    const { detectGatedSetupCommand } = await import('./conversation');
    expect(detectGatedSetupCommand('What is the weather?')).toBeNull();
    expect(detectGatedSetupCommand('I want to learn about X')).toBeNull();
    expect(detectGatedSetupCommand('upgrade my computer')).toBeNull();
  });
});

describe('conversation store — new quest trigger behavior', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockStreamChat.mockReset();
  });

  it('does NOT auto-trigger Scholar\'s Quest when user asks a law question', async () => {
    const brain = useBrainStore();
    brain.autoConfigureFreeApi();
    mockStreamChat.mockImplementation(
      (_url: string, _m: string, _k: string | null, _h: unknown[], cb: { onDone: (t: string) => void }) => {
        cb.onDone('The statute of limitations is 3 years under Article 429.');
        return new AbortController();
      },
    );

    const store = useConversationStore();
    await store.sendMessage('I want to learn about Vietnamese civil law on contract liability');

    // Only: user + assistant answer.  No auto Scholar's Quest message.
    expect(store.messages).toHaveLength(2);
    const hasScholarQuest = store.messages.some((m) => m.questId === 'scholar-quest');
    expect(hasScholarQuest).toBe(false);
  });

  it('shows the don\'t-know prompt when the LLM answer signals uncertainty', async () => {
    const brain = useBrainStore();
    brain.autoConfigureFreeApi();
    mockStreamChat.mockImplementation(
      (_url: string, _m: string, _k: string | null, _h: unknown[], cb: { onDone: (t: string) => void }) => {
        cb.onDone("I don't know the specific statute of limitations — my training data is limited on Vietnamese law.");
        return new AbortController();
      },
    );

    const store = useConversationStore();
    await store.sendMessage('What is the statute of limitations under Vietnamese law?');

    // user + assistant-answer + system-dont-know
    expect(store.messages).toHaveLength(3);
    const dontKnow = store.messages.find((m) => m.questId === 'dont-know');
    expect(dontKnow).toBeDefined();
    expect(dontKnow!.agentName).toBe('System');
    expect(dontKnow!.content).toMatch(/upgrade to Gemini model/i);
    expect(dontKnow!.content).toMatch(/provide your own context/i);
    const values = dontKnow!.questChoices!.map((c) => c.value);
    expect(values).toContain('command:upgrade to Gemini model');
    expect(values).toContain('command:provide your own context');
  });

  it('pushes Scholar\'s Quest when the user types "provide your own context"', async () => {
    // Configure a brain so the LLM intent classifier runs, then mock its
    // decision via the `classify_intent` Tauri command.
    const brain = useBrainStore();
    brain.autoConfigureFreeApi();
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'classify_intent') {
        return { kind: 'gated_setup', setup: 'provide_context' };
      }
      return undefined;
    });

    const store = useConversationStore();
    await store.sendMessage('provide your own context');

    expect(store.messages).toHaveLength(2);
    expect(store.messages[0].content).toBe('provide your own context');
    expect(store.messages[1].questId).toBe('scholar-quest');
    const labels = store.messages[1].questChoices!.map((c) => c.label);
    expect(labels).toContain('Start Knowledge Quest');
  });

  it('offers the Gemini marketplace path when the user types "upgrade to Gemini model"', async () => {
    const brain = useBrainStore();
    brain.autoConfigureFreeApi();
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'classify_intent') {
        return { kind: 'gated_setup', setup: 'upgrade_gemini' };
      }
      return undefined;
    });

    const store = useConversationStore();
    await store.sendMessage('upgrade to Gemini model');

    expect(store.messages).toHaveLength(2);
    expect(store.messages[1].questId).toBe('upgrade-gemini');
    const values = store.messages[1].questChoices!.map((c) => c.value);
    expect(values).toContain('navigate:marketplace');
  });

  it('pushes Scholar\'s Quest when the user explicitly says "remember the following law:"', async () => {
    const brain = useBrainStore();
    brain.autoConfigureFreeApi();
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'classify_intent') {
        return { kind: 'teach_ingest', topic: 'Article 429 — claims expire after 3 years' };
      }
      return undefined;
    });

    const store = useConversationStore();
    await store.sendMessage('Remember the following law: Article 429 — claims expire after 3 years.');

    expect(store.messages).toHaveLength(2);
    expect(store.messages[1].questId).toBe('scholar-quest');
    expect(store.messages[1].content).toMatch(/article 429/i);
  });

  it('falls through to streaming chat when the classifier returns Unknown', async () => {
    // When the classifier can't decide (timeout, no brain, malformed JSON),
    // the safe default is to proceed with normal streaming chat — never
    // assume the user wants to learn from documents.
    const brain = useBrainStore();
    brain.autoConfigureFreeApi();
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'classify_intent') return { kind: 'unknown' };
      return undefined;
    });
    mockStreamChat.mockImplementation(
      (_baseUrl: string, _model: string, _apiKey: string | null, _history: unknown[], callbacks: { onDone: (text: string) => void }) => {
        callbacks.onDone('Hello! How can I help?');
        return new AbortController();
      },
    );

    const store = useConversationStore();
    await store.sendMessage('hello there');

    // No install-all overlay should be pushed — message proceeds normally.
    expect(store.messages.find((m) => m.questId === 'learn-docs-missing')).toBeUndefined();
    expect(mockStreamChat).toHaveBeenCalled();
  });
});

describe('conversation store — Learn-with-docs flow', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockStreamChat.mockReset();
  });

  it('detectLearnWithDocsIntent matches the canonical phrase', async () => {
    const { detectLearnWithDocsIntent } = await import('./conversation');
    expect(detectLearnWithDocsIntent('Learn Vietnamese laws using my provided documents'))
      .toEqual({ topic: 'Vietnamese laws' });
    expect(detectLearnWithDocsIntent('Study quantum physics with my files'))
      .toEqual({ topic: 'quantum physics' });
    expect(detectLearnWithDocsIntent('learn about contract law from my notes'))
      .toEqual({ topic: 'contract law' });
    // Plain question should NOT match — it's a chat, not an instruction.
    expect(detectLearnWithDocsIntent('What is Vietnamese law?')).toBeNull();
    // Bare "learn about X" without "documents" reference must not match.
    expect(detectLearnWithDocsIntent('learn about Vietnamese laws')).toBeNull();
  });

  it('pushes the missing-components prompt with three choices', async () => {
    const brain = useBrainStore();
    brain.autoConfigureFreeApi();
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'classify_intent') {
        return { kind: 'learn_with_docs', topic: 'Vietnamese laws' };
      }
      return undefined;
    });

    const store = useConversationStore();
    await store.sendMessage('Learn Vietnamese laws using my provided documents');

    expect(store.messages).toHaveLength(2);
    const prompt = store.messages[1];
    expect(prompt.questId).toBe('learn-docs-missing');
    expect(prompt.content).toMatch(/Vietnamese laws/);
    const values = prompt.questChoices!.map((c) => c.value);
    expect(values).toHaveLength(3);
    expect(values[0]).toMatch(/^learn-docs:install-all:/);
    expect(values[1]).toMatch(/^learn-docs:install-each:/);
    expect(values[2]).toBe('dismiss');
  });

  it('install-all runs auto-install and reports installed quests', async () => {
    const { handleLearnDocsChoice } = await import('./conversation');
    const { useSkillTreeStore } = await import('./skill-tree');
    const skillTree = useSkillTreeStore();
    const store = useConversationStore();

    // markComplete is called for scholar-quest
    const markSpy = vi.spyOn(skillTree, 'markComplete').mockImplementation(() => {});

    await handleLearnDocsChoice(`learn-docs:install-all:${encodeURIComponent('Vietnamese laws')}`);

    // Should have auto-install messages including installed list
    expect(store.messages.length).toBeGreaterThanOrEqual(2);
    // The auto-install message
    expect(store.messages[0].content).toMatch(/Auto-installing/);
    // Should have an installed summary or followup
    const last = store.messages[store.messages.length - 1];
    expect(last.questId === 'scholar-quest' || last.questId === 'learn-docs-followup' || last.content.includes('Installed')).toBe(true);

    markSpy.mockRestore();
  });

  it('install-auto legacy route still works', async () => {
    const { handleLearnDocsChoice } = await import('./conversation');
    const { useSkillTreeStore } = await import('./skill-tree');
    const skillTree = useSkillTreeStore();
    const store = useConversationStore();

    const markSpy = vi.spyOn(skillTree, 'markComplete').mockImplementation(() => {});

    await handleLearnDocsChoice(`learn-docs:install-auto:${encodeURIComponent('Vietnamese laws')}`);

    // Should produce auto-install messages
    expect(store.messages.length).toBeGreaterThanOrEqual(1);
    const last = store.messages[store.messages.length - 1];
    expect(last.questId === 'scholar-quest' || last.questId === 'learn-docs-followup' || last.content.includes('Installed')).toBe(true);

    markSpy.mockRestore();
  });

  it('install-each (one by one) renders one button per missing quest', async () => {
    const { handleLearnDocsChoice } = await import('./conversation');
    const store = useConversationStore();
    await handleLearnDocsChoice(`learn-docs:install-each:${encodeURIComponent('Vietnamese laws')}`);

    expect(store.messages).toHaveLength(1);
    const prompt = store.messages[0];
    expect(prompt.questId).toBe('learn-docs-install-each');
    // Every choice (except Cancel) should be a per-quest install action.
    const installs = prompt.questChoices!.filter((c) => c.value.startsWith('learn-docs:install-quest:'));
    expect(installs.length).toBeGreaterThan(0);
    expect(prompt.questChoices!.some((c) => c.value === 'dismiss')).toBe(true);
  });

  it('install-back reopens the original three-choice prompt', async () => {
    const { handleLearnDocsChoice } = await import('./conversation');
    const store = useConversationStore();
    await handleLearnDocsChoice(`learn-docs:install-back:${encodeURIComponent('Vietnamese laws')}`);

    const last = store.messages[store.messages.length - 1];
    expect(last.questId).toBe('learn-docs-missing');
  });
});

describe('conversation store — chat-based LLM switching integration', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockStreamChat.mockReset();
  });

  it('switches to pollinations via chat command', async () => {
    const brain = useBrainStore();
    brain.autoConfigureFreeApi();
    mockInvoke.mockResolvedValue(undefined);

    const store = useConversationStore();
    await store.sendMessage('switch to pollinations');

    expect(store.messages).toHaveLength(2);
    expect(store.messages[0].content).toBe('switch to pollinations');
    expect(store.messages[1].role).toBe('assistant');
    expect(store.messages[1].content).toContain('Pollinations');
    expect(store.isThinking).toBe(false);
  });

  it('warns about API key requirement for groq', async () => {
    const brain = useBrainStore();
    brain.autoConfigureFreeApi();

    const store = useConversationStore();
    await store.sendMessage('switch to groq');

    expect(store.messages).toHaveLength(2);
    expect(store.messages[1].content).toContain('API key');
    expect(store.messages[1].content).toContain('Marketplace');
  });

  it('normal messages are NOT treated as LLM commands', async () => {
    const brain = useBrainStore();
    brain.autoConfigureFreeApi();

    mockStreamChat.mockImplementation(
      (_baseUrl: string, _model: string, _apiKey: string | null, _history: unknown[], callbacks: { onDone: (text: string) => void }) => {
        callbacks.onDone('42 is the answer');
        return new AbortController();
      },
    );

    const store = useConversationStore();
    await store.sendMessage('What is the meaning of life?');

    // Should go through normal chat path, not command path
    expect(mockStreamChat).toHaveBeenCalled();
    expect(store.messages[1].content).toBe('42 is the answer');
  });
});

describe('conversation store — long-running task controls', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockStreamChat.mockReset();
  });

  it('addToQueue accumulates messages', () => {
    const store = useConversationStore();
    store.addToQueue('first');
    store.addToQueue('second');
    expect(store.messageQueue).toEqual(['first', 'second']);
  });

  it('addToQueue ignores blank messages', () => {
    const store = useConversationStore();
    store.addToQueue('');
    store.addToQueue('   ');
    expect(store.messageQueue).toHaveLength(0);
  });

  it('stopGeneration is callable without error when idle', () => {
    const store = useConversationStore();
    // Should not throw even when nothing is streaming
    expect(() => store.stopGeneration()).not.toThrow();
  });

  it('stopAndSend is callable without error when idle', () => {
    const store = useConversationStore();
    expect(() => store.stopAndSend()).not.toThrow();
  });

  it('steerWithMessage queues the steering message at front', () => {
    const store = useConversationStore();
    store.addToQueue('queued-first');
    store.steerWithMessage('steer-me');
    // Steer should be at the front (unshift)
    expect(store.messageQueue[0]).toBe('steer-me');
    expect(store.messageQueue[1]).toBe('queued-first');
  });

  it('steerWithMessage ignores blank messages', () => {
    const store = useConversationStore();
    store.steerWithMessage('');
    expect(store.messageQueue).toHaveLength(0);
  });

  it('drains queue after persona fallback completes', async () => {
    const store = useConversationStore();
    store.addToQueue('follow-up');
    await store.sendMessage('hello');
    // The first sendMessage completes and drains, triggering the queued message.
    // Give the async drainQueue a tick to run.
    await new Promise((r) => setTimeout(r, 600));
    // Both the original and the queued message should have been processed
    // (user + assistant × 2 = 4 messages minimum)
    expect(store.messages.length).toBeGreaterThanOrEqual(4);
    const userMsgs = store.messages.filter(m => m.role === 'user');
    expect(userMsgs.some(m => m.content === 'follow-up')).toBe(true);
  });

  it('stopGeneration during browser streaming aborts and discards', async () => {
    const brain = useBrainStore();
    brain.autoConfigureFreeApi();
    mockStreamChat.mockImplementation(
      (_baseUrl: string, _model: string, _apiKey: string | null, _history: unknown[], callbacks: { onChunk: (text: string) => void; onDone: (text: string) => void; onError: (err: string) => void }) => {
        callbacks.onChunk('partial ');
        // Don't call onDone yet — simulate long-running
        const ctrl = new AbortController();
        // When abort is called, fire onError
        ctrl.signal.addEventListener('abort', () => {
          callbacks.onError('AbortError');
        });
        return ctrl;
      },
    );

    const store = useConversationStore();
    const sendPromise = store.sendMessage('hello');

    // Wait a tick for streaming to start
    await new Promise((r) => setTimeout(r, 50));

    // Stop generation (discard)
    store.stopGeneration();

    // Wait for sendMessage to settle
    await sendPromise;

    // Should have only the user message — partial output discarded
    expect(store.isThinking).toBe(false);
  });
});

describe('conversation store — stream queue concurrency', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockStreamChat.mockReset();
  });

  it('auto-queues messages sent while a generation is active', async () => {
    const store = useConversationStore();

    // Start first message (persona fallback — takes 500ms)
    const first = store.sendMessage('first');

    // While first is still running, send a second message
    // generationActive should be true, so this gets queued
    await store.sendMessage('second');

    // The second call should have returned immediately (queued)
    expect(store.messageQueue).toContain('second');

    // Wait for first to complete + drain
    await first;
    await new Promise((r) => setTimeout(r, 600));

    // Both messages should have been processed sequentially
    const userMsgs = store.messages.filter(m => m.role === 'user');
    expect(userMsgs.some(m => m.content === 'first')).toBe(true);
    expect(userMsgs.some(m => m.content === 'second')).toBe(true);
  });

  it('generationActive is false after sendMessage completes', async () => {
    const store = useConversationStore();
    await store.sendMessage('hello');
    expect(store.generationActive).toBe(false);
  });

  it('generationActive is true during generation', async () => {
    const store = useConversationStore();
    const promise = store.sendMessage('hello');
    expect(store.generationActive).toBe(true);
    await promise;
    expect(store.generationActive).toBe(false);
  });

  it('rapid-fire messages are serialized, not interleaved', async () => {
    const store = useConversationStore();

    // Fire 3 messages rapidly
    const p1 = store.sendMessage('msg-1');
    store.sendMessage('msg-2');
    store.sendMessage('msg-3');

    // msg-2 and msg-3 should be queued
    expect(store.messageQueue).toEqual(['msg-2', 'msg-3']);

    // Wait for all to drain
    await p1;
    await new Promise((r) => setTimeout(r, 2000));

    // All 3 user messages should appear in order
    const userMsgs = store.messages
      .filter(m => m.role === 'user')
      .map(m => m.content);
    expect(userMsgs).toEqual(['msg-1', 'msg-2', 'msg-3']);

    // Each user message should be followed by an assistant reply
    for (let i = 0; i < store.messages.length - 1; i += 2) {
      expect(store.messages[i].role).toBe('user');
      expect(store.messages[i + 1].role).toBe('assistant');
    }
  });

  it('queue drains correctly after browser-side streaming completes', async () => {
    const brain = useBrainStore();
    brain.autoConfigureFreeApi();

    mockStreamChat.mockImplementation(
      (_baseUrl: string, _model: string, _apiKey: string | null, _history: unknown[], callbacks: { onDone: (text: string) => void }) => {
        callbacks.onDone('Response!');
        return new AbortController();
      },
    );

    const store = useConversationStore();
    const p1 = store.sendMessage('first');

    // Queue a second while first is active
    store.sendMessage('second');

    await p1;
    await new Promise((r) => setTimeout(r, 100));

    const userMsgs = store.messages
      .filter(m => m.role === 'user')
      .map(m => m.content);
    expect(userMsgs).toContain('first');
    expect(userMsgs).toContain('second');
    expect(store.generationActive).toBe(false);
    expect(store.messageQueue).toHaveLength(0);
  });

  it('generationActive resets on early-return paths (LLM commands)', async () => {
    const brain = useBrainStore();
    brain.autoConfigureFreeApi();
    mockInvoke.mockResolvedValue(undefined);

    const store = useConversationStore();
    await store.sendMessage('switch to pollinations');

    // LLM command path should still reset generationActive
    expect(store.generationActive).toBe(false);
  });
});

describe('conversation store — AI decision-making policy gates', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockStreamChat.mockReset();
    localStorage.clear();
    // Default: any time the chat path falls through to streaming, resolve
    // immediately so a falling-through gate test doesn't hang.
    mockStreamChat.mockImplementation(
      (_b: string, _m: string, _k: string | null, _h: unknown[], cb: { onDone: (t: string) => void }) => {
        cb.onDone('ok');
        return new AbortController();
      },
    );
  });

  it('skips the classifier IPC entirely when intentClassifierEnabled=false', async () => {
    const brain = useBrainStore();
    brain.autoConfigureFreeApi();
    const policy = useAiDecisionPolicyStore();
    policy.policy.intentClassifierEnabled = false;
    // If the classifier ran, this mock would short-circuit into the install
    // overlay. Instead the call must never happen and the message must reach
    // the streaming chat path.
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'classify_intent') return { kind: 'learn_with_docs', topic: 'X' };
      return undefined;
    });

    const store = useConversationStore();
    await store.sendMessage('Learn quantum physics with my files');

    const callTypes = mockInvoke.mock.calls.map((c) => c[0]);
    expect(callTypes).not.toContain('classify_intent');
    // No learn-docs overlay was pushed; the message reached the streaming path.
    expect(store.messages.find((m) => m.questId === 'learn-docs-missing')).toBeUndefined();
    expect(mockStreamChat).toHaveBeenCalled();
  });

  it('chat-based LLM switch is ignored when chatBasedLlmSwitchEnabled=false', async () => {
    const brain = useBrainStore();
    brain.autoConfigureFreeApi();
    const policy = useAiDecisionPolicyStore();
    policy.policy.chatBasedLlmSwitchEnabled = false;
    policy.policy.intentClassifierEnabled = false;
    mockInvoke.mockResolvedValue(undefined);

    const store = useConversationStore();
    await store.sendMessage('switch to pollinations');

    // The "switch to pollinations" message must be treated as plain chat;
    // there is no follow-up confirmation message about the brain switch and
    // the streaming path was invoked instead.
    const switchedMsg = store.messages.find(
      (m) => m.role === 'assistant' && /switched to/i.test(m.content),
    );
    expect(switchedMsg).toBeUndefined();
    expect(mockStreamChat).toHaveBeenCalled();
  });
});
