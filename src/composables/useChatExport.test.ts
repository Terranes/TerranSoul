/**
 * Tests for the useChatExport composable.
 *
 * Covers export formatting, JSON serialization, filtering helpers,
 * and the browser download path.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { useChatExport } from './useChatExport';
import type { Message } from '../types';

function makeMessage(overrides: Partial<Message> = {}): Message {
  return {
    id: 'msg-1',
    role: 'user',
    content: 'Hello',
    timestamp: 1700000000000,
    ...overrides,
  };
}

describe('useChatExport', () => {
  let dateNowSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    dateNowSpy = vi.spyOn(Date, 'now').mockReturnValue(1700000099000);
  });

  afterEach(() => {
    dateNowSpy.mockRestore();
  });

  // ── formatExport ──────────────────────────────────────────────────────────

  it('returns correct metadata structure', () => {
    const { formatExport } = useChatExport();
    const messages: Message[] = [makeMessage()];
    const result = formatExport(messages);

    expect(result.metadata).toEqual({
      exported_at: 1700000099000,
      app_version: '0.1.0',
      message_count: 1,
      date_range: {
        first_message: 1700000000000,
        last_message: 1700000000000,
      },
      sentiment_summary: {},
    });
  });

  it('counts sentiments correctly', () => {
    const { formatExport } = useChatExport();
    const messages: Message[] = [
      makeMessage({ id: '1', sentiment: 'happy' }),
      makeMessage({ id: '2', sentiment: 'happy' }),
      makeMessage({ id: '3', sentiment: 'sad' }),
      makeMessage({ id: '4' }), // no sentiment
    ];
    const result = formatExport(messages);
    expect(result.metadata.sentiment_summary).toEqual({ happy: 2, sad: 1 });
  });

  it('handles empty messages array', () => {
    const { formatExport } = useChatExport();
    const result = formatExport([]);

    expect(result.metadata.message_count).toBe(0);
    expect(result.messages).toEqual([]);
    expect(result.metadata.sentiment_summary).toEqual({});
  });

  it('maps message fields correctly including timestamp_iso', () => {
    const { formatExport } = useChatExport();
    const msg = makeMessage({
      id: 'abc',
      role: 'assistant',
      content: 'Hi there',
      agentName: 'TerranSoul',
      sentiment: 'happy',
      timestamp: 1700000000000,
    });
    const result = formatExport([msg]);
    const exported = result.messages[0];

    expect(exported.id).toBe('abc');
    expect(exported.role).toBe('assistant');
    expect(exported.content).toBe('Hi there');
    expect(exported.agent_name).toBe('TerranSoul');
    expect(exported.sentiment).toBe('happy');
    expect(exported.timestamp).toBe(1700000000000);
    expect(exported.timestamp_iso).toBe(new Date(1700000000000).toISOString());
  });

  it('date_range is null for empty messages', () => {
    const { formatExport } = useChatExport();
    const result = formatExport([]);
    expect(result.metadata.date_range.first_message).toBeNull();
    expect(result.metadata.date_range.last_message).toBeNull();
  });

  it('date_range is correct for multiple messages', () => {
    const { formatExport } = useChatExport();
    const messages: Message[] = [
      makeMessage({ id: '1', timestamp: 1700000010000 }),
      makeMessage({ id: '2', timestamp: 1700000005000 }),
      makeMessage({ id: '3', timestamp: 1700000020000 }),
    ];
    const result = formatExport(messages);
    expect(result.metadata.date_range.first_message).toBe(1700000005000);
    expect(result.metadata.date_range.last_message).toBe(1700000020000);
  });

  it('maps missing agentName and sentiment to null', () => {
    const { formatExport } = useChatExport();
    const msg = makeMessage({ agentName: undefined, sentiment: undefined });
    const exported = formatExport([msg]).messages[0];
    expect(exported.agent_name).toBeNull();
    expect(exported.sentiment).toBeNull();
  });

  // ── toJson ────────────────────────────────────────────────────────────────

  it('returns valid JSON string', () => {
    const { toJson } = useChatExport();
    const json = toJson([makeMessage()]);
    const parsed = JSON.parse(json);
    expect(parsed.metadata).toBeDefined();
    expect(parsed.messages).toBeInstanceOf(Array);
    expect(parsed.messages.length).toBe(1);
  });

  it('with pretty=false returns compact JSON', () => {
    const { toJson } = useChatExport();
    const json = toJson([makeMessage()], false);
    // Compact JSON has no newlines
    expect(json).not.toContain('\n');
    // Still valid JSON
    expect(() => JSON.parse(json)).not.toThrow();
  });

  // ── filterByDateRange ─────────────────────────────────────────────────────

  it('filters by date range correctly', () => {
    const { filterByDateRange } = useChatExport();
    const messages: Message[] = [
      makeMessage({ id: '1', timestamp: 100 }),
      makeMessage({ id: '2', timestamp: 200 }),
      makeMessage({ id: '3', timestamp: 300 }),
      makeMessage({ id: '4', timestamp: 400 }),
    ];
    const filtered = filterByDateRange(messages, 150, 350);
    expect(filtered.map((m) => m.id)).toEqual(['2', '3']);
  });

  // ── filterByRole ──────────────────────────────────────────────────────────

  it('filters by user role', () => {
    const { filterByRole } = useChatExport();
    const messages: Message[] = [
      makeMessage({ id: '1', role: 'user' }),
      makeMessage({ id: '2', role: 'assistant' }),
      makeMessage({ id: '3', role: 'user' }),
    ];
    const users = filterByRole(messages, 'user');
    expect(users.map((m) => m.id)).toEqual(['1', '3']);

    const assistants = filterByRole(messages, 'assistant');
    expect(assistants.map((m) => m.id)).toEqual(['2']);
  });

  // ── filterBySentiment ─────────────────────────────────────────────────────

  it('filters by sentiment tag', () => {
    const { filterBySentiment } = useChatExport();
    const messages: Message[] = [
      makeMessage({ id: '1', sentiment: 'happy' }),
      makeMessage({ id: '2', sentiment: 'sad' }),
      makeMessage({ id: '3', sentiment: 'happy' }),
      makeMessage({ id: '4' }),
    ];
    const happy = filterBySentiment(messages, 'happy');
    expect(happy.map((m) => m.id)).toEqual(['1', '3']);
  });

  // ── downloadJson ──────────────────────────────────────────────────────────

  it('sets lastExportedAt on successful download', () => {
    const { downloadJson, lastExportedAt } = useChatExport();

    // Mock browser APIs
    const clickSpy = vi.fn();
    vi.spyOn(document, 'createElement').mockReturnValue({
      set href(_: string) { /* noop */ },
      set download(_: string) { /* noop */ },
      click: clickSpy,
    } as unknown as HTMLAnchorElement);
    vi.spyOn(URL, 'createObjectURL').mockReturnValue('blob:mock');
    vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => {});

    expect(lastExportedAt.value).toBeNull();
    downloadJson([makeMessage()]);
    expect(lastExportedAt.value).toBe(1700000099000);
    expect(clickSpy).toHaveBeenCalledOnce();
  });

  it('handles errors gracefully during download', () => {
    const { downloadJson, error, isExporting } = useChatExport();

    vi.spyOn(URL, 'createObjectURL').mockImplementation(() => {
      throw new Error('Blob creation failed');
    });

    downloadJson([makeMessage()]);
    expect(error.value).toContain('Blob creation failed');
    expect(isExporting.value).toBe(false);
  });
});
