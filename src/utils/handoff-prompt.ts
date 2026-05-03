/**
 * Handoff system-prompt block builder.
 *
 * Pure function that turns a recorded *handoff context* (the
 * conversation-window summary captured by `agent-roster.setAgent`)
 * into a `[HANDOFF FROM <prev>]` block to inject at the head of the
 * *next* assistant turn after an agent switch.
 *
 * Same precedence shape as `[PERSONA]` (`utils/persona-prompt.ts`)
 * and `[LONG-TERM MEMORY]` (the RAG layer):
 *
 *     [PERSONA]            ← stable identity
 *     [LONG-TERM MEMORY]   ← retrieved facts
 *     [HANDOFF FROM A]     ← one-shot situational briefing
 *     <user turn>
 *
 * The block is emitted **once** after a switch; the orchestrator is
 * responsible for clearing the recorded context after it has been
 * injected so the new agent then operates on its own thread without
 * re-receiving the briefing every turn (see Chunk 23.2 in
 * `rules/milestones.md` and the doc note on the agent-roster store).
 *
 * This file is intentionally dependency-free so it can be unit-tested
 * in isolation and reused from any consumer (chat streaming pipeline,
 * preview UI, or future programmatic orchestrator).
 */

/** Maximum characters rendered into the handoff body (keeps prompt cost bounded). */
export const HANDOFF_MAX_CHARS = 2000;

/** Maximum number of context lines preserved when truncating from the head. */
export const HANDOFF_MAX_LINES = 40;

/** Inputs to the handoff block. */
export interface HandoffBlockInput {
  /** Previous agent's display name (e.g. "Aria"). Required. */
  prevAgentName: string;
  /** Recorded conversation-window summary; usually multi-line. */
  context: string;
  /** Optional: the *new* agent's name, only used for symmetry — not rendered. */
  nextAgentName?: string;
}

/**
 * Build the `[HANDOFF FROM <prev-agent-name>]` block.
 *
 * Returns an empty string when:
 * - `prevAgentName` is blank after sanitisation, or
 * - `context` is blank / whitespace-only after sanitisation.
 *
 * Otherwise renders:
 *
 *     \n\n[HANDOFF FROM <prev>]\n<context lines>\n[/HANDOFF]
 *
 * The block is line-prefixed (no JSON / no nested markup) so even
 * small local LLMs can parse it reliably.
 *
 * Truncation rules (in order):
 * 1. Lines are clipped to {@link HANDOFF_MAX_LINES} from the **tail**
 *    (most-recent turns win — older context is dropped).
 * 2. Each line is sanitised: control characters stripped, trailing
 *    whitespace trimmed, and empty lines dropped.
 * 3. The joined body is hard-capped at {@link HANDOFF_MAX_CHARS}; if
 *    the cap is hit we keep the **tail** of the body and prefix a
 *    `…(truncated)\n` marker so the reader knows context was lost.
 */
export function buildHandoffBlock(input: HandoffBlockInput | null | undefined): string {
  if (!input) return '';

  const prev = sanitiseLine(input.prevAgentName);
  if (!prev) return '';

  const body = sanitiseContext(input.context);
  if (!body) return '';

  const capped = capCharacters(body);

  return `\n\n[HANDOFF FROM ${prev}]\n${capped}\n[/HANDOFF]`;
}

/**
 * Clean up a multi-line context string:
 *
 * - Replace CRLF with LF.
 * - Strip control chars (except \n / \t).
 * - Trim each line; drop empty ones.
 * - Keep only the last {@link HANDOFF_MAX_LINES} non-empty lines.
 */
function sanitiseContext(raw: unknown): string {
  if (typeof raw !== 'string') return '';

  const cleaned = raw.replace(/\r\n?/g, '\n').replace(/[\x00-\x08\x0B-\x1F\x7F]/g, '');
  const lines = cleaned
    .split('\n')
    .map((l) => l.trimEnd())
    .filter((l) => l.length > 0);
  if (lines.length === 0) return '';
  const tail = lines.slice(-HANDOFF_MAX_LINES);
  return tail.join('\n');
}

/** Hard cap the body, preserving the tail (most-recent context). */
function capCharacters(body: string): string {
  if (body.length <= HANDOFF_MAX_CHARS) return body;
  // Keep room for the truncation marker.
  const marker = '…(truncated)\n';
  const budget = HANDOFF_MAX_CHARS - marker.length;
  const tail = body.slice(body.length - budget);
  return marker + tail;
}

/** Strip control chars + collapse whitespace for a single-line field. */
function sanitiseLine(s: unknown): string {
  if (typeof s !== 'string') return '';

  return s.replace(/[\x00-\x1F\x7F]/g, ' ').replace(/\s+/g, ' ').trim();
}
