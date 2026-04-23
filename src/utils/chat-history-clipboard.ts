import type { Message } from '../types';

function formatRole(role: Message['role']): string {
  return role === 'assistant' ? 'AI' : 'You';
}

export function formatChatHistory(messages: Message[]): string {
  return messages
    .map((msg) => {
      const time = new Date(msg.timestamp).toLocaleString();
      return `[${time}] ${formatRole(msg.role)}: ${msg.content}`;
    })
    .join('\n');
}

/**
 * Copy chat history to clipboard.
 * Returns true on success; false if clipboard is unavailable or write fails.
 */
export async function copyChatHistory(messages: Message[]): Promise<boolean> {
  if (typeof navigator === 'undefined' || !navigator.clipboard) return false;
  const payload = formatChatHistory(messages);
  try {
    await navigator.clipboard.writeText(payload);
    return true;
  } catch {
    return false;
  }
}

export async function readClipboardText(): Promise<string> {
  if (typeof navigator === 'undefined' || !navigator.clipboard) return '';
  return navigator.clipboard.readText();
}
