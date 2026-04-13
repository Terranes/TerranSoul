/**
 * Emotion and motion tag parser for LLM response text.
 *
 * Parses tags like `[happy]`, `[sad]`, `[motion:wave]` from LLM text,
 * strips them from the display text, and returns the extracted metadata.
 */
import type { EmotionTag, MotionTag, ParsedLlmChunk } from '../types';

const EMOTION_TAGS: ReadonlySet<string> = new Set([
  'happy',
  'sad',
  'angry',
  'relaxed',
  'surprised',
  'neutral',
]);

/**
 * Parse emotion and motion tags from a text chunk.
 *
 * Recognized tags:
 * - Emotion: `[happy]`, `[sad]`, `[angry]`, `[relaxed]`, `[surprised]`, `[neutral]`
 * - Motion:  `[motion:wave]`, `[motion:nod]`, etc.
 *
 * Tags are stripped from the returned text. Only the first emotion and first
 * motion tag per chunk are used (subsequent ones are still stripped).
 */
export function parseTags(input: string): ParsedLlmChunk {
  let text = input;
  let emotion: EmotionTag | null = null;
  let motion: MotionTag | null = null;

  // Process all bracketed tags
  const tagRegex = /\[([\w:]+)\]\s*/g;
  text = text.replace(tagRegex, (_match, tagContent: string) => {
    if (tagContent.startsWith('motion:')) {
      const motionName = tagContent.slice('motion:'.length).trim();
      if (motion === null && motionName) {
        motion = motionName;
      }
      return '';
    }

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
    motion,
  };
}

/**
 * Strip all emotion/motion tags from text, returning clean display text.
 */
export function stripTags(input: string): string {
  return parseTags(input).text;
}
