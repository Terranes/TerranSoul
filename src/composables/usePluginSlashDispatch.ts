/**
 * Plugin slash-command dispatcher (Chunk 22.4).
 *
 * Intercepts user messages that begin with `/<name> ...` and routes
 * them to a contributed plugin slash-command if one matches.
 *
 * The dispatcher does NOT replace the host's built-in slash-commands
 * (those are handled by the conversation store). It only fires for
 * names that match an active plugin's
 * `manifest.contributes.slash_commands[].name`.
 *
 * The full execution path for plugin commands lands in Chunk 22.7.
 * For now `plugin_invoke_slash_command` returns a stub `CommandResult`
 * echoing the command title, and we surface that as a chat-side
 * status message.
 */

import { usePluginStore } from '../stores/plugins'

export interface SlashDispatchResult {
  /** True if a plugin claimed and executed (or attempted) this message. */
  handled: boolean
  /** Bare command name (without `/`) — set when handled. */
  name?: string
  /** Argument string after the command word (trimmed). */
  args?: string
  /** Plugin's stub output, when invocation succeeded. */
  output?: string
  /** Error string when invocation failed. */
  error?: string
}

/**
 * Parse a chat message and, if it is a slash-command contributed by an
 * active plugin, parse out the name and argument string.
 *
 * Returns `null` when the message is not a slash form, or when no
 * active plugin contributes a matching name (so the host's normal
 * dispatch can run).
 */
export function parseSlashIntent(
  message: string,
  knownNames: ReadonlySet<string>,
): { name: string; args: string } | null {
  const trimmed = message.trimStart()
  if (!trimmed.startsWith('/')) return null
  // Reject empty `/`, double slashes (URLs), or escape (`//`).
  if (trimmed.length < 2 || trimmed[1] === '/') return null
  // First whitespace splits name from args.
  const rest = trimmed.slice(1)
  const wsIdx = rest.search(/\s/)
  const name = wsIdx === -1 ? rest : rest.slice(0, wsIdx)
  const args = wsIdx === -1 ? '' : rest.slice(wsIdx + 1).trim()
  if (!name) return null
  if (!knownNames.has(name)) return null
  return { name, args }
}

/**
 * Composable wrapper that resolves the plugin store and exposes a
 * single `tryDispatchSlashCommand(message)` async function.
 */
export function usePluginSlashDispatch() {
  const store = usePluginStore()

  function knownSlashNames(): Set<string> {
    return new Set(store.slashCommands.map((s) => s.slash_command.name))
  }

  async function tryDispatchSlashCommand(message: string): Promise<SlashDispatchResult> {
    const intent = parseSlashIntent(message, knownSlashNames())
    if (!intent) return { handled: false }

    try {
      const result = await store.invokeSlashCommand(
        intent.name,
        intent.args ? { text: intent.args } : null,
      )
      if (result.success) {
        return {
          handled: true,
          name: intent.name,
          args: intent.args,
          output: result.output ?? '',
        }
      }
      return {
        handled: true,
        name: intent.name,
        args: intent.args,
        error: result.error ?? 'plugin returned failure',
      }
    } catch (e) {
      return {
        handled: true,
        name: intent.name,
        args: intent.args,
        error: String(e),
      }
    }
  }

  return { tryDispatchSlashCommand, parseSlashIntent }
}
