import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  OllvClient,
  DEFAULT_OLLV_WS_URL,
  type OllvCallbacks,
  type OllvAudioMessage,
} from './ollv-client';

// ── Mock WebSocket ────────────────────────────────────────────────────────────

class MockWebSocket {
  static CONNECTING = 0;
  static OPEN = 1;
  static CLOSING = 2;
  static CLOSED = 3;

  url: string;
  readyState: number;
  onopen: ((ev: Event) => void) | null = null;
  onclose: ((ev: CloseEvent) => void) | null = null;
  onerror: ((ev: Event) => void) | null = null;
  onmessage: ((ev: MessageEvent) => void) | null = null;
  sentMessages: string[] = [];

  constructor(url: string) {
    this.url = url;
    this.readyState = MockWebSocket.CONNECTING;
  }

  send(data: string): void {
    this.sentMessages.push(data);
  }

  close(code?: number, _reason?: string): void {
    this.readyState = MockWebSocket.CLOSED;
    if (this.onclose) {
      this.onclose(new CloseEvent('close', { code: code ?? 1000, reason: _reason ?? '' }));
    }
  }

  // Helper to simulate connection open
  simulateOpen(): void {
    this.readyState = MockWebSocket.OPEN;
    this.onopen?.(new Event('open'));
  }

  // Helper to simulate incoming message
  simulateMessage(data: string): void {
    this.onmessage?.(new MessageEvent('message', { data }));
  }

  // Helper to simulate error
  simulateError(): void {
    this.onerror?.(new Event('error'));
  }
}

// Store the last created MockWebSocket instance
let lastWs: MockWebSocket | null = null;

// Install global mock
beforeEach(() => {
  lastWs = null;
  vi.stubGlobal('WebSocket', class extends MockWebSocket {
    constructor(url: string) {
      super(url);
      lastWs = this;
    }
    static override CONNECTING = 0;
    static override OPEN = 1;
    static override CLOSING = 2;
    static override CLOSED = 3;
  });
});

afterEach(() => {
  vi.restoreAllMocks();
  lastWs = null;
});

describe('OllvClient', () => {
  it('has correct default URL', () => {
    expect(DEFAULT_OLLV_WS_URL).toBe('ws://localhost:12393/client-ws');
  });

  it('creates with default URL', () => {
    const client = new OllvClient();
    expect(client.url).toBe(DEFAULT_OLLV_WS_URL);
    expect(client.connected).toBe(false);
  });

  it('creates with custom URL', () => {
    const client = new OllvClient('ws://custom:8000/ws');
    expect(client.url).toBe('ws://custom:8000/ws');
  });

  it('connect creates WebSocket and invokes onOpen', () => {
    const onOpen = vi.fn();
    const client = new OllvClient(DEFAULT_OLLV_WS_URL, { onOpen });
    client.connect();

    expect(lastWs).not.toBeNull();
    expect(lastWs!.url).toBe(DEFAULT_OLLV_WS_URL);

    lastWs!.simulateOpen();
    expect(onOpen).toHaveBeenCalledOnce();
    expect(client.connected).toBe(true);
  });

  it('disconnect closes WebSocket', () => {
    const onClose = vi.fn();
    const client = new OllvClient(DEFAULT_OLLV_WS_URL, { onClose });
    client.connect();
    lastWs!.simulateOpen();

    client.disconnect();
    expect(onClose).toHaveBeenCalledOnce();
    expect(client.connected).toBe(false);
  });

  it('sendText sends text-input message', () => {
    const client = new OllvClient();
    client.connect();
    lastWs!.simulateOpen();

    client.sendText('Hello');
    expect(lastWs!.sentMessages).toHaveLength(1);
    const msg = JSON.parse(lastWs!.sentMessages[0]);
    expect(msg).toEqual({ type: 'text-input', text: 'Hello' });
  });

  it('sendAudioChunk sends mic-audio-data message', () => {
    const client = new OllvClient();
    client.connect();
    lastWs!.simulateOpen();

    const audio = new Float32Array([0.1, 0.2, 0.3]);
    client.sendAudioChunk(audio);
    const msg = JSON.parse(lastWs!.sentMessages[0]);
    expect(msg.type).toBe('mic-audio-data');
    expect(msg.audio).toHaveLength(3);
    expect(msg.audio[0]).toBeCloseTo(0.1, 1);
    expect(msg.audio[1]).toBeCloseTo(0.2, 1);
    expect(msg.audio[2]).toBeCloseTo(0.3, 1);
  });

  it('sendAudioEnd sends mic-audio-end message', () => {
    const client = new OllvClient();
    client.connect();
    lastWs!.simulateOpen();

    client.sendAudioEnd();
    const msg = JSON.parse(lastWs!.sentMessages[0]);
    expect(msg).toEqual({ type: 'mic-audio-end', images: [] });
  });

  it('sendInterrupt sends interrupt-signal message', () => {
    const client = new OllvClient();
    client.connect();
    lastWs!.simulateOpen();

    client.sendInterrupt();
    const msg = JSON.parse(lastWs!.sentMessages[0]);
    expect(msg).toEqual({ type: 'interrupt-signal' });
  });

  it('does not send when not connected', () => {
    const client = new OllvClient();
    client.sendText('Hello');
    // No WebSocket created, no error thrown
    expect(lastWs).toBeNull();
  });

  it('handles audio message from server', () => {
    const onAudio = vi.fn();
    const client = new OllvClient(DEFAULT_OLLV_WS_URL, { onAudio });
    client.connect();
    lastWs!.simulateOpen();

    const audioMsg: OllvAudioMessage = {
      type: 'audio',
      audio: 'base64data==',
      volumes: [0.1, 0.5, 0.3],
      slice_length: 1024,
      display_text: 'Hello!',
    };
    lastWs!.simulateMessage(JSON.stringify(audioMsg));

    expect(onAudio).toHaveBeenCalledOnce();
    expect(onAudio).toHaveBeenCalledWith(audioMsg);
  });

  it('handles transcription message from server', () => {
    const onTranscription = vi.fn();
    const client = new OllvClient(DEFAULT_OLLV_WS_URL, { onTranscription });
    client.connect();
    lastWs!.simulateOpen();

    lastWs!.simulateMessage(JSON.stringify({
      type: 'user-input-transcription',
      text: 'Hello world',
    }));

    expect(onTranscription).toHaveBeenCalledWith('Hello world');
  });

  it('handles full-text message from server', () => {
    const onFullText = vi.fn();
    const client = new OllvClient(DEFAULT_OLLV_WS_URL, { onFullText });
    client.connect();
    lastWs!.simulateOpen();

    lastWs!.simulateMessage(JSON.stringify({ type: 'full-text', text: 'AI response' }));
    expect(onFullText).toHaveBeenCalledWith('AI response');
  });

  it('handles chain start/end messages', () => {
    const onChainStart = vi.fn();
    const onChainEnd = vi.fn();
    const client = new OllvClient(DEFAULT_OLLV_WS_URL, { onChainStart, onChainEnd });
    client.connect();
    lastWs!.simulateOpen();

    lastWs!.simulateMessage(JSON.stringify({ type: 'conversation-chain-start' }));
    expect(onChainStart).toHaveBeenCalledOnce();

    lastWs!.simulateMessage(JSON.stringify({ type: 'conversation-chain-end' }));
    expect(onChainEnd).toHaveBeenCalledOnce();
  });

  it('handles interrupt message from server', () => {
    const onInterrupt = vi.fn();
    const client = new OllvClient(DEFAULT_OLLV_WS_URL, { onInterrupt });
    client.connect();
    lastWs!.simulateOpen();

    lastWs!.simulateMessage(JSON.stringify({ type: 'interrupt-signal' }));
    expect(onInterrupt).toHaveBeenCalledOnce();
  });

  it('handles control message from server', () => {
    const onControl = vi.fn();
    const client = new OllvClient(DEFAULT_OLLV_WS_URL, { onControl });
    client.connect();
    lastWs!.simulateOpen();

    lastWs!.simulateMessage(JSON.stringify({ type: 'control', action: 'start-mic' }));
    expect(onControl).toHaveBeenCalledWith('start-mic');
  });

  it('ignores invalid JSON messages', () => {
    const callbacks: OllvCallbacks = {
      onAudio: vi.fn(),
      onTranscription: vi.fn(),
    };
    const client = new OllvClient(DEFAULT_OLLV_WS_URL, callbacks);
    client.connect();
    lastWs!.simulateOpen();

    lastWs!.simulateMessage('not valid json');
    expect(callbacks.onAudio).not.toHaveBeenCalled();
    expect(callbacks.onTranscription).not.toHaveBeenCalled();
  });

  it('does not create duplicate connections', () => {
    const client = new OllvClient();
    client.connect();
    const firstWs = lastWs;
    client.connect(); // Should not create a new one
    expect(lastWs).toBe(firstWs);
  });

  it('readyState returns CLOSED when not connected', () => {
    const client = new OllvClient();
    expect(client.readyState).toBe(WebSocket.CLOSED);
  });
});
