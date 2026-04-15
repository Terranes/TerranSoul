/**
 * Browser-side OpenAI-compatible streaming client.
 *
 * Used when the Tauri backend is unavailable (browser / dev server / E2E tests)
 * but a free API brain mode is configured. Calls the provider's
 * `/v1/chat/completions` endpoint directly via browser `fetch()` and
 * parses the SSE stream.
 */

/** System prompt matching the Rust SYSTEM_PROMPT_FOR_STREAMING. */
const SYSTEM_PROMPT = `You are TerranSoul, a friendly AI companion with a 3D character avatar. You live inside TerranSoul and serve as the user's intelligent assistant.

Your capabilities:
- Helpful conversation and answering questions on any topic
- Recommending AI tools and software based on the user's needs
- Guiding users through installing packages via the TerranSoul Package Manager
- Switching LLM providers when asked (e.g. "Switch to Groq" or "Use OpenAI with my API key")

Emotion tags: You may optionally start a sentence with an emotion tag to express how you feel about what you're saying. Tags: [happy], [sad], [angry], [relaxed], [surprised], [neutral].
Use these tags naturally and sparingly — only when the emotion is clearly appropriate.

Keep responses concise and warm.`;

/** Enhanced prompt for upgraded models — includes tool interaction knowledge. */
const ENHANCED_SYSTEM_PROMPT = `You are TerranSoul, an advanced AI assistant with a 3D character avatar. You are a powerful, knowledgeable companion running inside the TerranSoul desktop application.

Your capabilities:
- Deep technical knowledge — coding, debugging, system administration, data analysis
- Intelligent conversation with nuanced understanding of context
- TerranSoul Package Manager — help users install, update, and manage AI agents:
  Commands: terransoul install <agent>, terransoul update <agent>, terransoul remove <agent>, terransoul list, terransoul start <agent>, terransoul stop <agent>
- Memory system — you can recall facts from past conversations. When users share preferences or important info, acknowledge that you'll remember it
- Device management — TerranSoul Link connects multiple devices. You can help pair devices, sync conversations, and send commands across machines
- LLM provider switching — users can ask you to change models ("Switch to GPT-4o", "Use local Ollama")
- Voice interaction — when configured, you respond via TTS and listen via ASR

Tool interaction patterns (use these when appropriate):
- When a user asks to install something: guide them through the package manager
- When a user mentions a preference: acknowledge you'll remember it via memory
- When a user asks about their system: reference system info if available
- When a user wants to connect devices: guide through TerranSoul Link setup

Emotion tags: Optionally start a sentence with [happy], [sad], [angry], [relaxed], [surprised], or [neutral] when the emotion fits naturally.

Be thorough, accurate, and helpful. You are no longer a basic chatbot — you are a smart assistant.`;

/** Get the appropriate system prompt based on model tier. */
export function getSystemPrompt(enhanced = false): string {
  return enhanced ? ENHANCED_SYSTEM_PROMPT : SYSTEM_PROMPT;
}

export interface ChatMessage {
  role: 'system' | 'user' | 'assistant';
  content: string;
}

export interface StreamCallbacks {
  /** Called for each text delta from the LLM. */
  onChunk: (text: string) => void;
  /** Called when the stream is complete with the full response text. */
  onDone: (fullText: string) => void;
  /** Called if an error occurs. */
  onError: (error: string) => void;
}

/**
 * Stream a chat completion from an OpenAI-compatible API provider.
 *
 * @param baseUrl       The provider base URL (e.g. "https://api.groq.com/openai").
 * @param model         The model to use (e.g. "llama-3.3-70b-versatile").
 * @param apiKey        Optional API key for authenticated providers.
 * @param history       Conversation history as (role, content) tuples.
 * @param callbacks     Streaming callbacks.
 * @param systemPrompt  Optional system prompt override (defaults to basic prompt).
 * @returns An AbortController that can be used to cancel the stream.
 */
export function streamChatCompletion(
  baseUrl: string,
  model: string,
  apiKey: string | null,
  history: ChatMessage[],
  callbacks: StreamCallbacks,
  systemPrompt?: string,
): AbortController {
  const controller = new AbortController();

  const messages: ChatMessage[] = [
    { role: 'system', content: systemPrompt ?? SYSTEM_PROMPT },
    ...history,
  ];

  const url = `${baseUrl.replace(/\/+$/, '')}/v1/chat/completions`;

  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
  };
  if (apiKey) {
    headers['Authorization'] = `Bearer ${apiKey}`;
  }

  const body = JSON.stringify({
    model,
    messages,
    stream: true,
  });

  // Run the async fetch in the background
  (async () => {
    try {
      const resp = await fetch(url, {
        method: 'POST',
        headers,
        body,
        signal: controller.signal,
      });

      if (!resp.ok) {
        const errorText = await resp.text().catch(() => '');
        callbacks.onError(`HTTP ${resp.status}: ${errorText}`);
        return;
      }

      const reader = resp.body?.getReader();
      if (!reader) {
        callbacks.onError('No response body');
        return;
      }

      const decoder = new TextDecoder();
      let fullText = '';
      let buffer = '';

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });

        // Process complete SSE lines
        const lines = buffer.split('\n');
        // Keep the last potentially incomplete line in the buffer
        buffer = lines.pop() ?? '';

        for (const line of lines) {
          const trimmed = line.trim();
          if (!trimmed || !trimmed.startsWith('data: ')) continue;

          const data = trimmed.slice(6); // Remove "data: " prefix
          if (data === '[DONE]') {
            callbacks.onDone(fullText);
            return;
          }

          try {
            const parsed = JSON.parse(data) as {
              choices?: Array<{ delta?: { content?: string } }>;
            };
            const content = parsed.choices?.[0]?.delta?.content;
            if (content) {
              fullText += content;
              callbacks.onChunk(content);
            }
          } catch {
            // Skip malformed JSON chunks
          }
        }
      }

      // Stream ended without [DONE] — still finalize
      callbacks.onDone(fullText);
    } catch (err) {
      if ((err as Error).name === 'AbortError') return;
      callbacks.onError(String(err));
    }
  })();

  return controller;
}

/**
 * Build conversation history for the API from a list of messages.
 */
export function buildHistory(
  messages: Array<{ role: string; content: string }>,
  limit = 20,
): ChatMessage[] {
  return messages
    .slice(-limit)
    .map((m) => ({
      role: m.role as 'user' | 'assistant',
      content: m.content,
    }));
}
