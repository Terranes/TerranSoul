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
 * Parse emotion/motion data from LLM text.
 *
 * Handles two formats:
 * - Schema: `<anim>{"emotion":"happy","motion":"wave"}</anim>` (primary)
 * - Legacy: `[happy]`, `[motion:wave]` bracket tags (backward-compat)
 *
 * Tags/blocks are stripped from the returned text. Only the first emotion
 * and first motion per call are used.
 */
export function parseTags(input: string): ParsedLlmChunk {
  let text = input;
  let emotion: EmotionTag | null = null;
  let motion: string | null = null;
  let emoji: string | null = null;

  // Primary: parse <anim>JSON</anim> blocks
  const animRegex = /<anim>([\s\S]*?)<\/anim>\s*/g;
  text = text.replace(animRegex, (_match, jsonStr: string) => {
    try {
      const cmd = JSON.parse(jsonStr);
      if (cmd.emotion && EMOTION_TAGS.has(cmd.emotion.toLowerCase()) && emotion === null) {
        emotion = cmd.emotion.toLowerCase() as EmotionTag;
      }
      if (cmd.motion && motion === null) {
        motion = String(cmd.motion).toLowerCase();
      }
    } catch {
      // Invalid JSON — strip the block anyway
    }
    return '';
  });

  // Legacy fallback: bracket tags [happy], [motion:wave]
  const tagRegex = /\[([^\]]+)\]\s*/g;
  text = text.replace(tagRegex, (_match, tagContent: string) => {
    const lower = tagContent.toLowerCase();
    if (EMOTION_TAGS.has(lower)) {
      if (emotion === null) {
        emotion = lower as EmotionTag;
      }
      return '';
    }

    // Motion tags like [motion:wave], [motion:nod]
    if (lower.startsWith('motion:')) {
      if (motion === null) {
        motion = lower.slice(7); // e.g. 'wave'
      }
      return '';
    }

    // Not a recognized tag — leave it in place
    return `[${tagContent}] `;
  });

  // JSON-wrapped response fallback: LLM sometimes returns bare JSON like
  // {"text":"Hello","emoji":"🎧"} — extract the text and emoji fields.
  const trimmed = text.trim();
  if (trimmed.startsWith('{') && trimmed.endsWith('}')) {
    try {
      const obj = JSON.parse(trimmed);
      if (typeof obj.text === 'string') {
        text = obj.text;
        if (typeof obj.emoji === 'string') {
          emoji = obj.emoji;
        }
        if (obj.emotion && EMOTION_TAGS.has(String(obj.emotion).toLowerCase()) && emotion === null) {
          emotion = String(obj.emotion).toLowerCase() as EmotionTag;
        }
      }
    } catch {
      // Not valid JSON — keep text as-is
    }
  }

  return {
    text: text.trim(),
    emotion,
    motion,
    emoji,
  };
}

/**
 * Strip all emotion tags from text, returning clean display text.
 */
export function stripTags(input: string): string {
  return parseTags(input).text;
}

