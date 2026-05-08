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
 *
 * ## Mode gating
 *
 * Commands can specify a `mode` in YAML frontmatter to restrict when
 * they appear in the command picker:
 *
 * ```yaml
 * ---
 * mode: coding
 * ---
 * ```
 *
 * Supported modes:
 * - `all` (default) — always visible
 * - `coding` — only visible when coding workflow / self-improve is active
 * - `companion` — only visible in normal companion chat mode
 */

import { defineStore } from 'pinia';
import { ref, computed } from 'vue';

/** Mode restriction for prompt commands. */
export type PromptMode = 'all' | 'coding' | 'companion';

export interface PromptCommand {
  /** Slash command name (without `/`). Derived from filename. */
  name: string;
  /** Human-readable description (first `# heading` or first line). */
  description: string;
  /** Full prompt content to inject into the LLM conversation. */
  content: string;
  /** Source file path (for display/debugging). */
  source: string;
  /** Mode restriction — controls when this command appears. */
  mode: PromptMode;
}

export const usePromptCommandsStore = defineStore('prompt-commands', () => {
  const commands = ref<PromptCommand[]>([]);
  const loaded = ref(false);
  /** Whether coding workflow mode is currently active. */
  const codingModeActive = ref(false);

  /** Commands filtered by the current app mode. */
  const activeCommands = computed<PromptCommand[]>(() => {
    return commands.value.filter((c) => {
      if (c.mode === 'all') return true;
      if (c.mode === 'coding') return codingModeActive.value;
      if (c.mode === 'companion') return !codingModeActive.value;
      return true;
    });
  });

  /** Load prompt commands from all known directories. */
  async function loadCommands() {
    const newCommands: PromptCommand[] = [];

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const promptFiles = await invoke<Array<{ name: string; content: string; path: string; mode: PromptMode }>>(
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
          mode: file.mode ?? 'all',
        });
      }

      // Sync coding mode state from self-improve settings.
      try {
        const settings = await invoke<{ enabled: boolean } | null>('get_self_improve_settings');
        codingModeActive.value = settings?.enabled ?? false;
      } catch {
        // Command unavailable — keep current state.
      }
    } catch {
      // Tauri not available or command not registered — ignore.
    }

    commands.value = newCommands;
    loaded.value = true;
  }

  /** Update coding mode state (call when self-improve or coding workflow toggles). */
  function setCodingMode(active: boolean) {
    codingModeActive.value = active;
  }

  /** Check if a slash command name matches an active prompt command. */
  function hasCommand(name: string): boolean {
    return activeCommands.value.some((c) => c.name === name);
  }

  /** Get the prompt content for a given command name (only if active). */
  function getPrompt(name: string): string | null {
    const cmd = activeCommands.value.find((c) => c.name === name);
    return cmd?.content ?? null;
  }

  /** Get all available command names for the current mode (for autocomplete/help). */
  function availableNames(): string[] {
    return activeCommands.value.map((c) => c.name);
  }

  /** Save (create or update) a prompt command. */
  async function saveCommand(name: string, content: string): Promise<void> {
    const { invoke } = await import('@tauri-apps/api/core');
    const file = await invoke<{ name: string; content: string; path: string; mode: PromptMode }>(
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
      mode: file.mode ?? 'all',
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
    activeCommands,
    codingModeActive,
    loaded,
    loadCommands,
    setCodingMode,
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
