/**
 * Composable for dispatching extensible prompt commands.
 *
 * When a user types `/command-name [args]` in chat, this checks against
 * loaded prompt files from `.terransoul/prompts/` and injects the prompt
 * into the LLM conversation.
 *
 * Template variables supported in prompt files:
 *   {{input}}  — replaced with user-provided args after the command name
 *   {{date}}   — replaced with current date (YYYY-MM-DD)
 */

import { usePromptCommandsStore, type PromptCommand } from '../stores/prompt-commands';

export interface PromptDispatchResult {
  handled: boolean;
  name?: string;
  prompt?: string;
  error?: string;
}

/**
 * Parse a message as a potential prompt command.
 * Returns the resolved prompt content or null if not a prompt command.
 */
export function usePromptCommandDispatch() {
  const store = usePromptCommandsStore();

  /**
   * Try to dispatch a message as a prompt command.
   * Returns the result with the processed prompt content.
   */
  function tryDispatchPromptCommand(message: string): PromptDispatchResult {
    if (!message.startsWith('/')) return { handled: false };

    const rest = message.slice(1);
    const spaceIdx = rest.indexOf(' ');
    const name = spaceIdx === -1 ? rest : rest.slice(0, spaceIdx);
    const args = spaceIdx === -1 ? '' : rest.slice(spaceIdx + 1).trim();

    if (!name || !store.hasCommand(name)) {
      return { handled: false };
    }

    const rawContent = store.getPrompt(name);
    if (!rawContent) {
      return { handled: true, name, error: `Prompt file for '${name}' is empty.` };
    }

    // Process template variables
    const processed = processTemplate(rawContent, args);
    return { handled: true, name, prompt: processed };
  }

  /** Get all available prompt command names for autocomplete (mode-filtered). */
  function getAvailableCommands(): Array<{ name: string; description: string }> {
    return store.activeCommands.map((c: PromptCommand) => ({
      name: c.name,
      description: c.description,
    }));
  }

  return {
    tryDispatchPromptCommand,
    getAvailableCommands,
  };
}

/** Replace template variables in prompt content. */
function processTemplate(content: string, input: string): string {
  const now = new Date();
  const dateStr = now.toISOString().slice(0, 10);

  return content
    .replace(/\{\{input\}\}/g, input)
    .replace(/\{\{date\}\}/g, dateStr);
}
