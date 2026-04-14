/**
 * Emotion, motion, and pose tag parser for LLM response text.
 *
 * Parses tags like `[happy]`, `[sad]`, `[motion:wave]`, and
 * `[pose:confident=0.6,attentive=0.3]` from LLM text, strips them from
 * the display text, and returns the extracted metadata.
 */
import type { EmotionTag, MotionTag, ParsedLlmChunk, PoseBlendInstruction } from '../types';

const EMOTION_TAGS: ReadonlySet<string> = new Set([
  'happy',
  'sad',
  'angry',
  'relaxed',
  'surprised',
  'neutral',
]);

/**
 * Parse a `pose:...` tag body into blend instructions.
 * Format: `pose:confident=0.6,attentive=0.3`
 * Returns null if the body is empty or malformed.
 */
function parsePoseTag(body: string): PoseBlendInstruction[] | null {
  // body = "confident=0.6,attentive=0.3"
  const pairs = body.split(',');
  const instructions: PoseBlendInstruction[] = [];
  for (const pair of pairs) {
    const eq = pair.indexOf('=');
    if (eq === -1) continue;
    const presetId = pair.slice(0, eq).trim();
    const weightStr = pair.slice(eq + 1).trim();
    const weight = Math.max(0, Math.min(1, parseFloat(weightStr)));
    if (presetId && Number.isFinite(weight)) {
      instructions.push({ presetId, weight });
    }
  }
  return instructions.length > 0 ? instructions : null;
}

/**
 * Parse emotion, motion, and pose tags from a text chunk.
 *
 * Recognized tags:
 * - Emotion: `[happy]`, `[sad]`, `[angry]`, `[relaxed]`, `[surprised]`, `[neutral]`
 * - Motion:  `[motion:wave]`, `[motion:nod]`, etc.
 * - Pose:    `[pose:confident=0.6,attentive=0.3]`
 *
 * Tags are stripped from the returned text. Only the first emotion, first
 * motion, and first pose tag per chunk are used (subsequent ones are stripped).
 */
export function parseTags(input: string): ParsedLlmChunk {
  let text = input;
  let emotion: EmotionTag | null = null;
  let motion: MotionTag | null = null;
  let poseBlend: PoseBlendInstruction[] | null = null;

  // pose: tags may contain = and , so use a broader bracket pattern
  // Match [word:rest-of-tag] where rest may include =, . and ,
  const tagRegex = /\[([^\]]+)\]\s*/g;
  text = text.replace(tagRegex, (_match, tagContent: string) => {
    if (tagContent.startsWith('pose:')) {
      if (poseBlend === null) {
        poseBlend = parsePoseTag(tagContent.slice('pose:'.length).trim());
      }
      return '';
    }

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
    poseBlend,
  };
}

/**
 * Strip all emotion/motion/pose tags from text, returning clean display text.
 */
export function stripTags(input: string): string {
  return parseTags(input).text;
}

