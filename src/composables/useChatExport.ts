import { ref } from 'vue';
import type { Message } from '../types';

/** Metadata included in exported chat logs. */
export interface ChatExportMetadata {
  exported_at: number;
  app_version: string;
  message_count: number;
  date_range: {
    first_message: number | null;
    last_message: number | null;
  };
  sentiment_summary: Record<string, number>;
}

/** Full chat log export format. */
export interface ChatExport {
  metadata: ChatExportMetadata;
  messages: ExportedMessage[];
}

/** A single message in the export format. */
export interface ExportedMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  agent_name: string | null;
  sentiment: string | null;
  timestamp: number;
  timestamp_iso: string;
}

export function useChatExport() {
  const isExporting = ref(false);
  const error = ref<string | null>(null);
  const lastExportedAt = ref<number | null>(null);

  /** Convert messages to the export format. */
  function formatExport(messages: Message[]): ChatExport {
    const sentimentSummary: Record<string, number> = {};
    for (const msg of messages) {
      if (msg.sentiment) {
        sentimentSummary[msg.sentiment] = (sentimentSummary[msg.sentiment] ?? 0) + 1;
      }
    }

    const exportedMessages: ExportedMessage[] = messages.map((msg) => ({
      id: msg.id,
      role: msg.role,
      content: msg.content,
      agent_name: msg.agentName ?? null,
      sentiment: msg.sentiment ?? null,
      timestamp: msg.timestamp,
      timestamp_iso: new Date(msg.timestamp).toISOString(),
    }));

    const timestamps = messages.map((m) => m.timestamp).filter((t) => t > 0);

    return {
      metadata: {
        exported_at: Date.now(),
        app_version: '0.1.0',
        message_count: messages.length,
        date_range: {
          first_message: timestamps.length > 0 ? Math.min(...timestamps) : null,
          last_message: timestamps.length > 0 ? Math.max(...timestamps) : null,
        },
        sentiment_summary: sentimentSummary,
      },
      messages: exportedMessages,
    };
  }

  /** Export messages as a JSON string. */
  function toJson(messages: Message[], pretty = true): string {
    const exportData = formatExport(messages);
    return pretty ? JSON.stringify(exportData, null, 2) : JSON.stringify(exportData);
  }

  /** Download messages as a JSON file in the browser. */
  function downloadJson(messages: Message[], filename?: string): void {
    isExporting.value = true;
    error.value = null;
    try {
      const json = toJson(messages);
      const blob = new Blob([json], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const date = new Date().toISOString().slice(0, 10);
      const name = filename ?? `terransoul-chat-${date}.json`;

      const a = document.createElement('a');
      a.href = url;
      a.download = name;
      a.click();
      URL.revokeObjectURL(url);

      lastExportedAt.value = Date.now();
    } catch (e) {
      error.value = String(e);
    } finally {
      isExporting.value = false;
    }
  }

  /** Filter messages by date range (inclusive). */
  function filterByDateRange(messages: Message[], startMs: number, endMs: number): Message[] {
    return messages.filter((m) => m.timestamp >= startMs && m.timestamp <= endMs);
  }

  /** Filter messages by role. */
  function filterByRole(messages: Message[], role: 'user' | 'assistant'): Message[] {
    return messages.filter((m) => m.role === role);
  }

  /** Filter messages by sentiment. */
  function filterBySentiment(messages: Message[], sentiment: string): Message[] {
    return messages.filter((m) => m.sentiment === sentiment);
  }

  return {
    isExporting,
    error,
    lastExportedAt,
    formatExport,
    toJson,
    downloadJson,
    filterByDateRange,
    filterByRole,
    filterBySentiment,
  };
}
