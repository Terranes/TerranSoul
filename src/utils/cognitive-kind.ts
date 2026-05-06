/**
 * TS port of `src-tauri/src/memory/cognitive_kind.rs`.
 *
 * Pure-function classifier for the **episodic / semantic / procedural / judgment** axis
 * documented in `docs/brain-advanced-design.md` § 3.5. Used in the frontend so
 * the Brain hub can display kind-breakdowns without an IPC roundtrip per
 * memory.
 *
 * Resolution order (must match the Rust implementation):
 *   1. Explicit cognitive tag (`episodic`, `semantic`, `procedural`, `judgment`, optionally
 *      with `:detail` suffix). First recognised tag wins.
 *   2. Structural type defaults — `summary` → episodic, `preference` → semantic.
 *   3. Lightweight content heuristics — procedural verbs / numbered lists →
 *      procedural; explicit time anchors → episodic; otherwise → semantic.
 *
 * Keep this in lock-step with `cognitive_kind.rs`. When updating the verb /
 * hint lists, update both files and the unit tests in both languages.
 */

import type { MemoryType } from '../types';

export type CognitiveKind = 'episodic' | 'semantic' | 'procedural' | 'judgment';

const PROCEDURAL_VERBS: readonly string[] = [
  'how to', 'how-to', 'step ', 'steps to', 'first,', 'next,', 'finally,',
  'procedure', 'process for', 'workflow:', 'recipe', 'method:', 'algorithm:',
];

const EPISODIC_HINTS: readonly string[] = [
  'yesterday', 'today', 'this morning', 'this afternoon', 'this evening',
  'last night', 'last week', 'last month', 'earlier today', 'just now',
  'on monday', 'on tuesday', 'on wednesday', 'on thursday', 'on friday',
  'on saturday', 'on sunday',
  ' ago', 'happened', 'occurred', 'we met', 'i met', 'we visited',
  'i went', 'we went',
];

/**
 * Classify the cognitive kind of a memory from `(memory_type, tags, content)`.
 *
 * Pure function — no I/O, no side effects. Mirror of
 * `src-tauri/src/memory/cognitive_kind.rs::classify`.
 */
export function classifyCognitiveKind(
  memoryType: MemoryType,
  tags: string,
  content: string,
): CognitiveKind {
  const fromTag = classifyFromTags(tags);
  if (fromTag) return fromTag;

  const fromType = classifyFromType(memoryType);
  if (fromType) return fromType;

  return classifyFromContent(content);
}

function classifyFromTags(tags: string): CognitiveKind | null {
  // Mirror Rust split: comma, space, newline, tab.
  const tokens = tags.split(/[,\s]+/);
  for (const raw of tokens) {
    const token = raw.trim().toLowerCase();
    if (!token) continue;
    const head = token.split(':')[0];
    if (head === 'episodic') return 'episodic';
    if (head === 'semantic') return 'semantic';
    if (head === 'procedural') return 'procedural';
    if (head === 'judgment') return 'judgment';
  }
  return null;
}

function classifyFromType(memoryType: MemoryType): CognitiveKind | null {
  switch (memoryType) {
    case 'summary': return 'episodic';
    case 'preference': return 'semantic';
    case 'fact':
    case 'context':
      return null;
    default:
      return null;
  }
}

function classifyFromContent(content: string): CognitiveKind {
  const lower = content.toLowerCase();
  if (PROCEDURAL_VERBS.some((v) => lower.includes(v))) return 'procedural';
  if (hasNumberedList(lower)) return 'procedural';
  if (EPISODIC_HINTS.some((h) => lower.includes(h))) return 'episodic';
  return 'semantic';
}

/** Detect a numbered-list shape — at least two of "1.", "2.", "3."… */
function hasNumberedList(content: string): boolean {
  let count = 0;
  for (let n = 1; n <= 9; n++) {
    if (content.includes(`${n}.`)) {
      count++;
      if (count >= 2) return true;
    }
  }
  return false;
}

export interface CognitiveKindBreakdown {
  episodic: number;
  semantic: number;
  procedural: number;
  judgment: number;
  total: number;
}

/** Compute the {episodic, semantic, procedural, judgment} histogram for a list of memories. */
export function summariseCognitiveKinds(
  memories: ReadonlyArray<{ memory_type: MemoryType; tags: string; content: string }>,
): CognitiveKindBreakdown {
  const out: CognitiveKindBreakdown = { episodic: 0, semantic: 0, procedural: 0, judgment: 0, total: 0 };
  if (!memories) return out;
  for (const m of memories) {
    out[classifyCognitiveKind(m.memory_type, m.tags, m.content)]++;
    out.total++;
  }
  return out;
}
