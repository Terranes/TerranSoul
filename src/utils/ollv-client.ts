/**
 * Open-LLM-VTuber WebSocket client.
 *
 * Connects to a running Open-LLM-VTuber server's WebSocket endpoint and
 * provides methods for sending text/audio, receiving audio responses with
 * lip-sync data, and handling interruptions.
 *
 * Protocol reference: Open-LLM-VTuber/Open-LLM-VTuber websocket_handler.py
 *   - Frontend sends: text-input, mic-audio-data, mic-audio-end, interrupt-signal
 *   - Server sends:   audio, full-text, user-input-transcription, control,
 *                      conversation-chain-start, conversation-chain-end, interrupt-signal
 */

/** Default WebSocket endpoint for Open-LLM-VTuber. */
export const DEFAULT_OLLV_WS_URL = 'ws://localhost:12393/client-ws';

// ── Message types ─────────────────────────────────────────────────────────────

/** Outgoing message: send text input to the server. */
export interface OllvTextInput {
  type: 'text-input';
  text: string;
}

/** Outgoing message: send a chunk of mic audio (Float32Array). */
export interface OllvMicAudioData {
  type: 'mic-audio-data';
  audio: number[];
}

/** Outgoing message: signal end of mic audio capture. */
export interface OllvMicAudioEnd {
  type: 'mic-audio-end';
  images: string[];
}

/** Outgoing message: user interrupted AI speech. */
export interface OllvInterruptSignal {
  type: 'interrupt-signal';
}

export type OllvOutgoingMessage =
  | OllvTextInput
  | OllvMicAudioData
  | OllvMicAudioEnd
  | OllvInterruptSignal;

/** Incoming: audio response with optional lip-sync volumes. */
export interface OllvAudioMessage {
  type: 'audio';
  audio: string;           // base64-encoded WAV
  volumes: number[];       // per-frame RMS volumes for lip sync
  slice_length: number;
  display_text: string | null;
  actions?: {
    expressions?: Record<string, number>;
  };
  forwarded?: boolean;
}

/** Incoming: ASR transcription of user's speech. */
export interface OllvTranscriptionMessage {
  type: 'user-input-transcription';
  text: string;
}

/** Incoming: full display text / subtitle. */
export interface OllvFullTextMessage {
  type: 'full-text';
  text: string;
}

/** Incoming: control signal (start-mic, stop-mic, etc.). */
export interface OllvControlMessage {
  type: 'control';
  action: string;
}

/** Incoming: conversation chain lifecycle. */
export interface OllvChainMessage {
  type: 'conversation-chain-start' | 'conversation-chain-end';
}

/** Incoming: server-side interrupt. */
export interface OllvServerInterrupt {
  type: 'interrupt-signal';
}

export type OllvIncomingMessage =
  | OllvAudioMessage
  | OllvTranscriptionMessage
  | OllvFullTextMessage
  | OllvControlMessage
  | OllvChainMessage
  | OllvServerInterrupt;

// ── Event Callbacks ───────────────────────────────────────────────────────────

export interface OllvCallbacks {
  /** Received audio with lip-sync data. */
  onAudio?: (msg: OllvAudioMessage) => void;
  /** ASR produced a transcription of user speech. */
  onTranscription?: (text: string) => void;
  /** Full text / subtitle from the AI response. */
  onFullText?: (text: string) => void;
  /** AI started generating a response chain. */
  onChainStart?: () => void;
  /** AI finished generating a response chain. */
  onChainEnd?: () => void;
  /** Server sent an interrupt signal. */
  onInterrupt?: () => void;
  /** Control message from server. */
  onControl?: (action: string) => void;
  /** Connection opened. */
  onOpen?: () => void;
  /** Connection closed. */
  onClose?: (code: number, reason: string) => void;
  /** Connection error. */
  onError?: (error: Event) => void;
}

// ── Client ────────────────────────────────────────────────────────────────────

export class OllvClient {
  private ws: WebSocket | null = null;
  private callbacks: OllvCallbacks;
  private _url: string;

  constructor(url: string = DEFAULT_OLLV_WS_URL, callbacks: OllvCallbacks = {}) {
    this._url = url;
    this.callbacks = callbacks;
  }

  get url(): string {
    return this._url;
  }

  get connected(): boolean {
    return this.ws !== null && this.ws.readyState === WebSocket.OPEN;
  }

  get readyState(): number {
    return this.ws?.readyState ?? WebSocket.CLOSED;
  }

  /** Connect to the Open-LLM-VTuber WebSocket server. */
  connect(): void {
    if (this.ws && this.ws.readyState !== WebSocket.CLOSED) {
      return; // Already connected or connecting
    }

    this.ws = new WebSocket(this._url);

    this.ws.onopen = () => {
      this.callbacks.onOpen?.();
    };

    this.ws.onclose = (ev) => {
      this.callbacks.onClose?.(ev.code, ev.reason);
    };

    this.ws.onerror = (ev) => {
      this.callbacks.onError?.(ev);
    };

    this.ws.onmessage = (ev) => {
      this.handleMessage(ev.data);
    };
  }

  /** Disconnect from the server. */
  disconnect(): void {
    if (this.ws) {
      this.ws.close(1000, 'Client disconnected');
      this.ws = null;
    }
  }

  /** Send a text message to the Open-LLM-VTuber LLM. */
  sendText(text: string): void {
    this.send({ type: 'text-input', text });
  }

  /** Send a chunk of mic audio (Float32Array converted to number[]). */
  sendAudioChunk(audio: Float32Array): void {
    this.send({ type: 'mic-audio-data', audio: Array.from(audio) });
  }

  /** Signal end of mic audio capture. */
  sendAudioEnd(images: string[] = []): void {
    this.send({ type: 'mic-audio-end', images });
  }

  /** Send an interrupt signal to stop AI speech. */
  sendInterrupt(): void {
    this.send({ type: 'interrupt-signal' });
  }

  /** Health check: attempt WebSocket connection and verify open within timeout. */
  static async healthCheck(url: string = DEFAULT_OLLV_WS_URL, timeoutMs: number = 3000): Promise<boolean> {
    return new Promise<boolean>((resolve) => {
      let ws: WebSocket;
      try {
        ws = new WebSocket(url);
      } catch {
        resolve(false);
        return;
      }

      const timer = setTimeout(() => {
        ws.close();
        resolve(false);
      }, timeoutMs);

      ws.onopen = () => {
        clearTimeout(timer);
        ws.close(1000, 'health-check');
        resolve(true);
      };

      ws.onerror = () => {
        clearTimeout(timer);
        ws.close();
        resolve(false);
      };
    });
  }

  // ── Private ─────────────────────────────────────────────────────────────────

  private send(msg: OllvOutgoingMessage): void {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      return;
    }
    this.ws.send(JSON.stringify(msg));
  }

  private handleMessage(data: string): void {
    let msg: OllvIncomingMessage;
    try {
      msg = JSON.parse(data) as OllvIncomingMessage;
    } catch {
      return; // Ignore non-JSON messages
    }

    switch (msg.type) {
      case 'audio':
        this.callbacks.onAudio?.(msg as OllvAudioMessage);
        break;
      case 'user-input-transcription':
        this.callbacks.onTranscription?.((msg as OllvTranscriptionMessage).text);
        break;
      case 'full-text':
        this.callbacks.onFullText?.((msg as OllvFullTextMessage).text);
        break;
      case 'conversation-chain-start':
        this.callbacks.onChainStart?.();
        break;
      case 'conversation-chain-end':
        this.callbacks.onChainEnd?.();
        break;
      case 'interrupt-signal':
        this.callbacks.onInterrupt?.();
        break;
      case 'control':
        this.callbacks.onControl?.((msg as OllvControlMessage).action);
        break;
    }
  }
}
