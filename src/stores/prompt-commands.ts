/**
 * Extensible prompt commands store.
 *
 * Loads `.md` prompt files from `.terransoul/prompts/` in the user's
 * workspace (or data directory) and makes them available as slash
 * commands in chat. Similar to Copilot's `.github/prompts/*.prompt.md`
 * and Claude Code's `.claude/commands/*.md`.
 *
 * Convention:
 *   .terransoul/prompts/setup-prerequisites.md → /setup-prerequisites
 *   .terransoul/prompts/explain-code.md        → /explain-code
 *
 * The file content becomes the prompt injected into the LLM conversation.
 * First line starting with `#` is used as the description (optional).
 */

import { defineStore } from 'pinia';
import { ref } from 'vue';

export interface PromptCommand {
  /** Slash command name (without `/`). Derived from filename. */
  name: string;
  /** Human-readable description (first `# heading` or first line). */
  description: string;
  /** Full prompt content to inject into the LLM conversation. */
  content: string;
  /** Source file path (for display/debugging). */
  source: string;
}

export const usePromptCommandsStore = defineStore('prompt-commands', () => {
  const commands = ref<PromptCommand[]>([]);
  const loaded = ref(false);

  /** Load prompt commands from all known directories. */
  async function loadCommands() {
    const newCommands: PromptCommand[] = [];

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const promptFiles = await invoke<Array<{ name: string; content: string; path: string }>>(
        'list_prompt_commands',
      );
      for (const file of promptFiles) {
        const name = file.name.replace(/\.md$/i, '');
        const description = extractDescription(file.content);
        newCommands.push({
          name,
          description,
          content: file.content,
          source: file.path,
        });
      }
    } catch {
      // Tauri not available or command not registered — ignore.
    }

    commands.value = newCommands;
    loaded.value = true;
  }

  /** Check if a slash command name matches a prompt command. */
  function hasCommand(name: string): boolean {
    return commands.value.some((c) => c.name === name);
  }

  /** Get the prompt content for a given command name. */
  function getPrompt(name: string): string | null {
    const cmd = commands.value.find((c) => c.name === name);
    return cmd?.content ?? null;
  }

  /** Get all available command names (for autocomplete/help). */
  function availableNames(): string[] {
    return commands.value.map((c) => c.name);
  }

  /** Save (create or update) a prompt command. */
  async function saveCommand(name: string, content: string): Promise<void> {
    const { invoke } = await import('@tauri-apps/api/core');
    const file = await invoke<{ name: string; content: string; path: string }>(
      'save_prompt_command',
      { name, content },
    );
    const description = extractDescription(file.content);
    const idx = commands.value.findIndex((c) => c.name === file.name);
    const entry: PromptCommand = {
      name: file.name,
      description,
      content: file.content,
      source: file.path,
    };
    if (idx >= 0) {
      commands.value[idx] = entry;
    } else {
      commands.value.push(entry);
      commands.value.sort((a, b) => a.name.localeCompare(b.name));
    }
  }

  /** Delete a prompt command by name. */
  async function deleteCommand(name: string): Promise<void> {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('delete_prompt_command', { name });
    commands.value = commands.value.filter((c) => c.name !== name);
  }

  return {
    commands,
    loaded,
    loadCommands,
    hasCommand,
    getPrompt,
    availableNames,
    saveCommand,
    deleteCommand,
  };
});

/** Extract description from prompt content (first heading or first line). */
function extractDescription(content: string): string {
  const lines = content.split('\n');
  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed.startsWith('#')) {
      return trimmed.replace(/^#+\s*/, '');
    }
    if (trimmed.length > 0) {
      return trimmed.slice(0, 80);
    }
  }
  return '';
}
