/**
 * Emotion tag parser for LLM response text.
 *
 * Parses tags like `[happy]`, `[sad]` from LLM text, strips them from
 * the display text, and returns the extracted metadata.
 */
import type { EmotionTag, ParsedLlmChunk } from '../types';

const EMOTION_TAGS: ReadonlySet<string> = new Set([
  'happy',
  'sad',
  'angry',
  'relaxed',
  'surprised',
  'neutral',
]);

/**
 * Parse emotion tags from a text chunk.
 *
 * Recognized tags:
 * - Emotion: `[happy]`, `[sad]`, `[angry]`, `[relaxed]`, `[surprised]`, `[neutral]`
 *
 * Tags are stripped from the returned text. Only the first emotion tag
 * per chunk is used (subsequent ones are stripped).
 */
export function parseTags(input: string): ParsedLlmChunk {
  let text = input;
  let emotion: EmotionTag | null = null;

  const tagRegex = /\[([^\]]+)\]\s*/g;
  text = text.replace(tagRegex, (_match, tagContent: string) => {
    const lower = tagContent.toLowerCase();
    if (EMOTION_TAGS.has(lower)) {
      if (emotion === null) {
        emotion = lower as EmotionTag;
      }
      return '';
    }

    // Not a recognized tag — leave it in place
    return `[${tagContent}] `;
  });

  return {
    text: text.trim(),
    emotion,
  };
}

/**
 * Strip all emotion tags from text, returning clean display text.
 */
export function stripTags(input: string): string {
  return parseTags(input).text;
}

