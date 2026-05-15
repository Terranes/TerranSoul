/**
 * Persona system prompt builder.
 *
 * Pure function that turns a {@link PersonaTraits} object into the
 * `[PERSONA]` block injected at the head of every chat's system prompt,
 * exactly the way `[LONG-TERM MEMORY]` is injected by the RAG layer
 * (see `docs/brain-advanced-design.md` § 4 RAG injection flow and
 * `docs/persona-design.md` § 9.1 system-prompt injection).
 *
 * This file is intentionally dependency-free so it can be unit-tested
 * in isolation and reused by both the browser-side streaming path
 * (`conversation.ts`) and the Rust streaming path (via a `set_persona_block`
 * command that pushes the rendered string to the backend).
 */

import {
  PERSONA_ENGLISH_ACCENT_OPTIONS,
  PERSONA_VOICE_AGE_OPTIONS,
  PERSONA_VOICE_GENDER_OPTIONS,
  PERSONA_VOICE_PITCH_OPTIONS,
  PERSONA_VOICE_STYLE_OPTIONS,
  migratePersonaVoiceProfile,
  type PersonaOption,
  type PersonaTraits,
  type PersonaVoiceProfile,
} from '../stores/persona-types';

/** Maximum characters rendered into the bio field (keeps prompt cost bounded). */
const BIO_MAX_CHARS = 500;

/** Maximum number of items rendered from each list field. */
const LIST_MAX_ITEMS = 8;

/** Maximum example dialogue entries rendered into the persona block. */
const EXAMPLE_DIALOGUE_MAX = 4;

/** A learned motion key the brain can emit; merged into the motion vocabulary. */
export interface LearnedMotionRef {
  /** Display name (for human-readable comments only). */
  name: string;
  /** Motion key the LLM can emit via `<anim>{"motion":"key"}</anim>`. */
  trigger: string;
}

/**
 * Build the `[PERSONA]` block for the system prompt.
 *
 * Returns an empty string when `traits.active` is false or all fields are
 * blank — in that case, nothing is injected and the brain falls back to
 * the bundled default system prompt unmodified.
 *
 * The block is intentionally short and structured: line-prefixed labels
 * mean even a small local LLM can parse it reliably without JSON.
 */
export function buildPersonaBlock(
  traits: PersonaTraits | null | undefined,
  learnedMotions: readonly LearnedMotionRef[] = [],
): string {
  if (!traits || !traits.active) return '';

  const lines: string[] = [];
  const name = sanitiseLine(traits.name);
  const role = sanitiseLine(traits.role);
  const bio = sanitiseMultiline(traits.bio).slice(0, BIO_MAX_CHARS);
  const tone = dedupTrim(traits.tone).slice(0, LIST_MAX_ITEMS);
  const quirks = dedupTrim(traits.quirks).slice(0, LIST_MAX_ITEMS);
  const avoid = dedupTrim(traits.avoid).slice(0, LIST_MAX_ITEMS);

  // Identity line — always present when the block is rendered at all.
  if (name || role) {
    lines.push(personaIdentityLine(name, role));
  }

  if (bio) {
    lines.push(`Background: ${bio}`);
  }
  if (tone.length > 0) {
    lines.push(`Tone: ${tone.join(', ')}.`);
  }
  if (quirks.length > 0) {
    lines.push(`Quirks: ${quirks.map(q => `"${q}"`).join('; ')}.`);
  }
  if (avoid.length > 0) {
    lines.push(`Never: ${avoid.join('; ')}.`);
  }

  const voiceDesign = buildVoiceDesignInstruction(traits.voiceProfile);
  if (voiceDesign) {
    lines.push(voiceDesign);
  }

  // Example dialogue — shows the LLM how this persona speaks.
  const examples = dedupTrim(traits.exampleDialogue).slice(0, EXAMPLE_DIALOGUE_MAX);
  if (examples.length > 0) {
    lines.push('Example dialogue:');
    for (const ex of examples) {
      lines.push(`- ${ex}`);
    }
  }

  // If the user has trained custom motions via the side chain (camera),
  // advertise them so the brain can pick them on its own initiative.
  // Per the design doc § 9.2, this is the same precedence shape as
  // user-preferences-shadow-defaults.
  const motionTriggers = dedupTrim(learnedMotions.map(m => m.trigger));
  if (motionTriggers.length > 0) {
    lines.push(
      `Personal motions you can trigger via <anim>{"motion":"key"}</anim>: ${motionTriggers.join(', ')}.`,
    );
  }

  // Bail if every field was empty.
  if (lines.length === 0) return '';

  return `\n\n[PERSONA]\n${lines.join('\n')}\n[/PERSONA]`;
}

/** Identity line; falls back gracefully when only one of name/role is set. */
function personaIdentityLine(name: string, role: string): string {
  if (name && role) return `You are ${name}, ${role}.`;
  if (name) return `You are ${name}.`;
  return `You are a ${role}.`;
}

export function buildVoiceDesignInstruction(profile: PersonaVoiceProfile | null | undefined): string {
  const p = migratePersonaVoiceProfile(profile);
  const parts = [
    `${labelFor(p.gender, PERSONA_VOICE_GENDER_OPTIONS).toLowerCase()} ${labelFor(p.age, PERSONA_VOICE_AGE_OPTIONS).toLowerCase()} voice`,
    `${labelFor(p.pitch, PERSONA_VOICE_PITCH_OPTIONS).toLowerCase()} pitch`,
    `${labelFor(p.style, PERSONA_VOICE_STYLE_OPTIONS).toLowerCase()} style`,
    `${labelFor(p.englishAccent, PERSONA_ENGLISH_ACCENT_OPTIONS)} English accent`,
  ];
  const voiceName = sanitiseLine(p.voiceName);
  if (voiceName) {
    parts.push(`preferred TTS voice ${voiceName}`);
  }
  return `Voice design: ${parts.join(', ')}.`;
}

function labelFor<T extends string>(value: T, options: readonly PersonaOption<T>[]): string {
  return options.find(option => option.value === value)?.label ?? value;
}

/** Strip control chars + collapse whitespace for a single-line field. */
function sanitiseLine(s: unknown): string {
  if (typeof s !== 'string') return '';
  return s
    .replace(/[\u0000-\u001F\u007F]/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();
}

/** Strip control chars but keep paragraph structure for multiline fields. */
function sanitiseMultiline(s: unknown): string {
  if (typeof s !== 'string') return '';
  return s
    // Strip control chars except newline
    .replace(/[\u0000-\u0009\u000B-\u001F\u007F]/g, ' ')
    .replace(/[ \t]+/g, ' ')
    .replace(/\n{3,}/g, '\n\n')
    .trim();
}

/** Trim each item, drop empties + duplicates, keep original order. */
function dedupTrim(items: readonly unknown[] | null | undefined): string[] {
  if (!items) return [];
  const seen = new Set<string>();
  const out: string[] = [];
  for (const item of items) {
    if (typeof item !== 'string') continue;
    const t = sanitiseLine(item);
    if (!t) continue;
    const key = t.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(t);
  }
  return out;
}
