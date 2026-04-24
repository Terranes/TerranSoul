/**
 * useCameraCapture — per-session camera consent composable.
 *
 * Enforces the "per-session only" rule: no on-disk "always on" flag.
 * Each chat session must explicitly grant camera access.
 *
 * Integrates with the persona store's `startCameraSession()` /
 * `stopCameraSession()` for bookkeeping.
 */

import { ref, onUnmounted, type Ref } from 'vue';
import { usePersonaStore } from '../stores/persona';
import { FaceMirror, type VrmExpressionWeights, zeroWeights } from '../renderer/face-mirror';

/** How long (ms) with no `update()` calls before we consider the user idle and auto-stop. */
const IDLE_TIMEOUT_MS = 5 * 60_000; // 5 minutes

export interface CameraCaptureReturn {
  /** Whether the camera is currently streaming. */
  active: Ref<boolean>;
  /** Whether MediaPipe is still initialising. */
  loading: Ref<boolean>;
  /** The internal <video> element (for PersonaTeacher preview). */
  videoEl: Ref<HTMLVideoElement | null>;
  /** Latest smoothed VRM expression weights. */
  weights: Ref<VrmExpressionWeights>;
  /** Request camera + initialise FaceLandmarker. Requires a chatId for session scoping. */
  start: (chatId: string) => Promise<void>;
  /** Stop camera + release resources. */
  stop: () => void;
  /** Call every frame to get updated expression weights. Returns current weights. */
  update: (dt: number) => VrmExpressionWeights;
}

/**
 * Per-session camera capture composable.
 *
 * Camera access is requested via `getUserMedia` each time `start()` is called.
 * No on-disk flag is ever written — camera consent lives only in memory.
 *
 * Auto-stops on:
 *   - Component unmount
 *   - Idle timeout (5 min with no `update()` calls)
 *   - Explicit `stop()` call
 */
export function useCameraCapture(): CameraCaptureReturn {
  const active = ref(false);
  const loading = ref(false);
  const videoEl = ref<HTMLVideoElement | null>(null);
  const weights = ref<VrmExpressionWeights>(zeroWeights());

  let stream: MediaStream | null = null;
  let mirror: FaceMirror | null = null;
  let idleTimer: ReturnType<typeof setTimeout> | null = null;

  const persona = usePersonaStore();

  function resetIdleTimer(): void {
    if (idleTimer) clearTimeout(idleTimer);
    idleTimer = setTimeout(() => {
      stop();
    }, IDLE_TIMEOUT_MS);
  }

  async function start(chatId: string): Promise<void> {
    if (active.value) return;
    loading.value = true;

    try {
      // Request camera permission (browser-level, per-session)
      stream = await navigator.mediaDevices.getUserMedia({
        video: { facingMode: 'user', width: { ideal: 640 }, height: { ideal: 480 } },
        audio: false,
      });

      // Create hidden video element
      const video = document.createElement('video');
      video.srcObject = stream;
      video.playsInline = true;
      video.muted = true;
      await video.play();
      videoEl.value = video;

      // Init FaceLandmarker (lazy-loads @mediapipe/tasks-vision)
      mirror = new FaceMirror();
      await mirror.init();

      // Record session in persona store
      persona.startCameraSession(chatId);

      active.value = true;
      resetIdleTimer();
    } catch (err) {
      // Clean up partial state
      cleanup();
      throw err;
    } finally {
      loading.value = false;
    }
  }

  function update(dt: number): VrmExpressionWeights {
    if (!active.value || !mirror || !videoEl.value) return weights.value;
    resetIdleTimer();
    weights.value = mirror.update(videoEl.value, dt);
    return weights.value;
  }

  function cleanup(): void {
    if (idleTimer) {
      clearTimeout(idleTimer);
      idleTimer = null;
    }
    if (mirror) {
      mirror.dispose();
      mirror = null;
    }
    if (videoEl.value) {
      videoEl.value.pause();
      videoEl.value.srcObject = null;
      videoEl.value = null;
    }
    if (stream) {
      for (const track of stream.getTracks()) track.stop();
      stream = null;
    }
  }

  function stop(): void {
    if (!active.value) return;
    cleanup();
    persona.stopCameraSession();
    active.value = false;
    weights.value = zeroWeights();
  }

  onUnmounted(stop);

  return { active, loading, videoEl, weights, start, stop, update };
}
