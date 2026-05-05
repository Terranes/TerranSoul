//! Pure parser for self-improve session slash commands.
//!
//! Mirrors the Claude Code / claw-code interactive shell where the user
//! drives session management directly from the input bar. Keeping this
//! a pure function makes it cheap to unit-test without mounting the
//! Vue component.

/**
 * The set of slash commands recognised by the self-improve chat input.
 *
 * `chat` is the fall-through for plain prose so the caller can branch
 * on a single discriminator.
 */
export type SlashCommandKind =
  | 'chat'
  | 'clear'
  | 'rename'
  | 'fork'
  | 'resume'
  | 'help'
  | 'list'
  | 'unknown';

export interface ParsedSlashCommand {
  kind: SlashCommandKind;
  /** The bare argument after the command (single string). */
  arg: string;
  /** Original raw input (trimmed). */
  raw: string;
  /** Only set for `unknown` — the command token without the slash. */
  command?: string;
}

/** Static help text shown by `/help`. Exposed for test reuse. */
export const SLASH_HELP_TEXT = [
  '/clear            — clear the active session transcript',
  '/rename <name>    — rename the active session',
  '/fork [<name>]    — copy the active session to a new id',
  '/resume <id|name> — switch to and load another session',
  '/list             — list saved sessions',
  '/help             — show this help',
].join('\n');

const KNOWN_COMMANDS = new Set<SlashCommandKind>([
  'clear',
  'rename',
  'fork',
  'resume',
  'help',
  'list',
]);

/**
 * Parse a single line of self-improve chat input.
 *
 * Lines that do not start with `/` (after trimming) are returned as
 * `chat`. Commands are case-insensitive. Unknown slash tokens are
 * returned as `unknown` so the caller can show a friendly error
 * instead of silently treating them as chat.
 */
export function parseSlashCommand(input: string): ParsedSlashCommand {
  const raw = input.trim();
  if (!raw.startsWith('/')) {
    return { kind: 'chat', arg: raw, raw };
  }

  // Strip the leading slash and split on the first run of whitespace.
  const body = raw.slice(1).trim();
  if (body.length === 0) {
    // A bare `/` is treated as `help` so the user discovers the palette.
    return { kind: 'help', arg: '', raw };
  }

  const firstSpace = body.search(/\s/);
  const command =
    firstSpace === -1 ? body.toLowerCase() : body.slice(0, firstSpace).toLowerCase();
  const arg = firstSpace === -1 ? '' : body.slice(firstSpace + 1).trim();

  if (KNOWN_COMMANDS.has(command as SlashCommandKind)) {
    return { kind: command as SlashCommandKind, arg, raw };
  }
  return { kind: 'unknown', arg, raw, command };
}
