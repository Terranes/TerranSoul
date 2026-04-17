/**
 * Composable that wires TTS audio playback into the LipSync engine,
 * feeding viseme values into the AvatarStateMachine.
 *
 * Architecture:
 *   TTS audio → MediaElementAudioSourceNode → AnalyserNode → LipSync
 *   LipSync.getVisemeValues() → AvatarStateMachine.setViseme()
 *
 * A single AudioContext is shared across all TTS sentences.
 * MediaElementAudioSourceNode is cleaned up between sentences.
 */

import { LipSync } from '../renderer/lip-sync';
import type { AvatarStateMachine } from '../renderer/avatar-state';
import type { TtsPlaybackHandle } from './useTtsPlayback';

export interface LipSyncBridgeHandle {
  /** Start the per-frame viseme update loop. Call in onMounted. */
  start(): void;
  /** Stop the per-frame loop and clean up. Call in onUnmounted. */
  dispose(): void;
}

export function useLipSyncBridge(
  tts: TtsPlaybackHandle,
  getAsm: () => AvatarStateMachine | null,
): LipSyncBridgeHandle {
  const lipSync = new LipSync({ fftSize: 256, smoothingTimeConstant: 0.5 });

  let audioCtx: AudioContext | null = null;
  let analyser: AnalyserNode | null = null;
  let currentSource: MediaElementAudioSourceNode | null = null;
  let rafId = 0;
  let running = false;

  /** Ensure the shared AudioContext + AnalyserNode exist. */
  function ensureAudioGraph(): AnalyserNode {
    if (!audioCtx || audioCtx.state === 'closed') {
      audioCtx = new AudioContext();
      analyser = audioCtx.createAnalyser();
      analyser.fftSize = 256;
      analyser.smoothingTimeConstant = 0.5;
      analyser.connect(audioCtx.destination);
      lipSync.connectAnalyser(analyser);
    }
    return analyser!;
  }

  /** Clean up the current MediaElementAudioSourceNode between sentences. */
  function cleanupSource(): void {
    if (currentSource) {
      try { currentSource.disconnect(); } catch { /* already disconnected */ }
      currentSource = null;
    }
  }

  /** Per-frame loop: read visemes from LipSync and push into AvatarState. */
  function tick(): void {
    if (!running) return;
    rafId = requestAnimationFrame(tick);

    const asm = getAsm();
    if (!asm) return;

    if (lipSync.active) {
      const v = lipSync.getVisemeValues();
      asm.setViseme(v);
    }
  }

  // ── TTS callbacks ──────────────────────────────────────────────────────────

  tts.onAudioStart((audio: HTMLAudioElement) => {
    cleanupSource();
    const node = ensureAudioGraph();

    // Resume AudioContext on user gesture (browser autoplay policy)
    if (audioCtx && audioCtx.state === 'suspended') {
      audioCtx.resume();
    }

    currentSource = audioCtx!.createMediaElementSource(audio);
    currentSource.connect(node);
  });

  tts.onAudioEnd(() => {
    cleanupSource();
    // Visemes will damp to 0 naturally via lambda=18 in CharacterAnimator
    const asm = getAsm();
    if (asm) asm.zeroVisemes();
  });

  tts.onPlaybackStop(() => {
    cleanupSource();
    // Immediately zero visemes on hard stop
    const asm = getAsm();
    if (asm) asm.zeroVisemes();
  });

  // ── Public API ─────────────────────────────────────────────────────────────

  function start(): void {
    if (running) return;
    running = true;
    lipSync.enableWorker();
    tick();
  }

  function dispose(): void {
    running = false;
    cancelAnimationFrame(rafId);
    cleanupSource();
    lipSync.disconnect();
    if (audioCtx) {
      try { audioCtx.close(); } catch { /* already closed */ }
      audioCtx = null;
      analyser = null;
    }
  }

  return { start, dispose };
}
