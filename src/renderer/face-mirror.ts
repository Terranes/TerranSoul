/**
 * face-mirror.ts — MediaPipe FaceLandmarker → ARKit → VRM expression mapper.
 *
 * The pure mapper (`mapBlendshapesToVRM`) is the unit-tested seam.
 * The `FaceMirror` class wraps MediaPipe FaceLandmarker for real-time use.
 *
 * Mapping table: docs/persona-design.md § 6.1
 */

// ── Pure mapper (no MediaPipe dependency — unit-testable) ─────────────────

/** VRM expression weights output by the mapper. */
export interface VrmExpressionWeights {
  happy: number;
  sad: number;
  angry: number;
  relaxed: number;
  surprised: number;
  neutral: number;
  aa: number;
  ih: number;
  ou: number;
  ee: number;
  oh: number;
  blink: number;
  lookAtX: number;
  lookAtY: number;
}

/** Helper: mean of values, returning 0 for empty input. */
function mean(values: number[]): number {
  if (values.length === 0) return 0;
  let sum = 0;
  for (let i = 0; i < values.length; i++) sum += values[i];
  return sum / values.length;
}

/** Clamp a number to [0, 1]. */
function clamp01(v: number): number {
  return v < 0 ? 0 : v > 1 ? 1 : v;
}

/**
 * Read a blendshape coefficient from the ARKit scores map.
 * Returns 0 if the key is missing — MediaPipe may omit some shapes.
 */
function bs(scores: ReadonlyMap<string, number>, name: string): number {
  return scores.get(name) ?? 0;
}

/**
 * Map 52 ARKit blendshape coefficients (from MediaPipe FaceLandmarker) to
 * TerranSoul's 12+2 VRM expression channels.
 *
 * The mapping follows docs/persona-design.md § 6.1.
 * All weights are clamped to [0, 1].
 *
 * @param scores — Map of ARKit blendshape name → coefficient (0–1).
 *                 FaceLandmarker returns these as `FaceLandmarkerResult.faceBlendshapes[0].categories`.
 */
export function mapBlendshapesToVRM(scores: ReadonlyMap<string, number>): VrmExpressionWeights {
  // ── Emotion channels ────────────────────────────────────────────────
  const smile = mean([bs(scores, 'mouthSmileLeft'), bs(scores, 'mouthSmileRight')]);
  const cheekSquint = mean([bs(scores, 'cheekSquintLeft'), bs(scores, 'cheekSquintRight')]);
  const happy = clamp01(smile * 0.7 + cheekSquint * 0.3);

  const frown = mean([bs(scores, 'mouthFrownLeft'), bs(scores, 'mouthFrownRight')]);
  const browDown = mean([bs(scores, 'browDownLeft'), bs(scores, 'browDownRight')]);
  const browInnerUp = bs(scores, 'browInnerUp');
  const sad = clamp01(frown * 0.6 + browInnerUp * 0.4);

  const noseSneer = mean([bs(scores, 'noseSneerLeft'), bs(scores, 'noseSneerRight')]);
  const mouthPress = mean([bs(scores, 'mouthPressLeft'), bs(scores, 'mouthPressRight')]);
  const angry = clamp01(browDown * 0.5 + noseSneer * 0.3 + mouthPress * 0.2);

  const eyeWide = mean([bs(scores, 'eyeWideLeft'), bs(scores, 'eyeWideRight')]);
  const jawOpen = bs(scores, 'jawOpen');
  const browOuterUp = mean([bs(scores, 'browOuterUpLeft'), bs(scores, 'browOuterUpRight')]);
  const surprised = clamp01(eyeWide * 0.5 + jawOpen * 0.3 + browOuterUp * 0.2);

  // Relaxed = inverse arousal
  const relaxed = clamp01(1 - Math.max(angry, sad, surprised));

  // Neutral falls out of normalisation
  const emotionSum = happy + sad + angry + surprised;
  const neutral = clamp01(1 - emotionSum);

  // ── Viseme channels ─────────────────────────────────────────────────
  const mouthFunnel = bs(scores, 'mouthFunnel');
  const mouthPucker = bs(scores, 'mouthPucker');
  const mouthStretch = mean([bs(scores, 'mouthStretchLeft'), bs(scores, 'mouthStretchRight')]);

  const aa = clamp01(jawOpen * 0.7 + mouthFunnel * 0.3);
  const ih = clamp01(mean([bs(scores, 'mouthSmileLeft'), bs(scores, 'mouthSmileRight'),
    bs(scores, 'mouthStretchLeft'), bs(scores, 'mouthStretchRight')]));
  const ou = clamp01(mean([mouthPucker, mouthFunnel]));
  const ee = clamp01(mouthStretch);
  const oh = clamp01(mean([mouthFunnel, jawOpen * 0.5]));

  // ── Blink ───────────────────────────────────────────────────────────
  const blink = clamp01(Math.max(bs(scores, 'eyeBlinkLeft'), bs(scores, 'eyeBlinkRight')));

  // ── Gaze direction ──────────────────────────────────────────────────
  const lookAtX = clamp01(bs(scores, 'eyeLookOutRight')) - clamp01(bs(scores, 'eyeLookOutLeft'));
  const lookUp = mean([bs(scores, 'eyeLookUpLeft'), bs(scores, 'eyeLookUpRight')]);
  const lookDown = mean([bs(scores, 'eyeLookDownLeft'), bs(scores, 'eyeLookDownRight')]);
  const lookAtY = lookUp - lookDown;

  return {
    happy, sad, angry, relaxed, surprised, neutral,
    aa, ih, ou, ee, oh,
    blink,
    lookAtX, lookAtY,
  };
}

// ── EMA smoothing ─────────────────────────────────────────────────────────

/** Smooth a weight with exponential damping (frame-rate independent). */
export function dampWeight(current: number, target: number, lambda: number, dt: number): number {
  return current + (target - current) * (1 - Math.exp(-lambda * dt));
}

/**
 * Smooth all channels of a VrmExpressionWeights using EMA.
 * Mutates `smoothed` in-place and returns it.
 */
export function smoothWeights(
  smoothed: VrmExpressionWeights,
  raw: VrmExpressionWeights,
  lambda: number,
  dt: number,
): VrmExpressionWeights {
  const keys = Object.keys(smoothed) as (keyof VrmExpressionWeights)[];
  for (const k of keys) {
    smoothed[k] = dampWeight(smoothed[k], raw[k], lambda, dt);
  }
  return smoothed;
}

/** Create a zeroed VrmExpressionWeights. */
export function zeroWeights(): VrmExpressionWeights {
  return {
    happy: 0, sad: 0, angry: 0, relaxed: 0, surprised: 0, neutral: 0,
    aa: 0, ih: 0, ou: 0, ee: 0, oh: 0,
    blink: 0, lookAtX: 0, lookAtY: 0,
  };
}

// ── FaceMirror class (wraps MediaPipe) ────────────────────────────────────

/** Default EMA lambda — higher = faster tracking, lower = more smoothing. */
const DEFAULT_LAMBDA = 12;

/**
 * Real-time face → VRM expression mirror.
 *
 * Lazy-loads `@mediapipe/tasks-vision` to avoid bundle bloat until the
 * `expressions-pack` quest is unlocked.
 *
 * Usage:
 *   const mirror = new FaceMirror();
 *   await mirror.init(videoElement);
 *   // In rAF loop:
 *   const weights = mirror.update(deltaSeconds);
 *   // When done:
 *   mirror.dispose();
 */
export class FaceMirror {
  private landmarker: import('@mediapipe/tasks-vision').FaceLandmarker | null = null;
  private smoothed = zeroWeights();
  private _running = false;
  private _lastTimestamp = -1;
  /**
   * Most-recent raw ARKit scores (52-channel) seen on the last
   * processed video frame. Empty until `update()` has produced at
   * least one detection. Exposed for the opt-in expanded-blendshape
   * passthrough (Chunk 27.3) — consumers feed this into
   * `applyExpandedBlendshapes` when `AppSettings.expanded_blendshapes`
   * is on.
   */
  private _lastRawScores: Map<string, number> = new Map();

  /** Read-only snapshot of the latest ARKit blendshape coefficients. */
  get lastRawScores(): ReadonlyMap<string, number> {
    return this._lastRawScores;
  }

  get running(): boolean { return this._running; }

  /**
   * Initialise the FaceLandmarker.
   * Lazy-imports @mediapipe/tasks-vision on first call.
   */
  async init(): Promise<void> {
    const { FaceLandmarker, FilesetResolver } = await import('@mediapipe/tasks-vision');

    const vision = await FilesetResolver.forVisionTasks(
      'https://cdn.jsdelivr.net/npm/@mediapipe/tasks-vision@latest/wasm',
    );

    this.landmarker = await FaceLandmarker.createFromOptions(vision, {
      baseOptions: {
        modelAssetPath: 'https://storage.googleapis.com/mediapipe-models/face_landmarker/face_landmarker/float16/1/face_landmarker.task',
        delegate: 'GPU',
      },
      runningMode: 'VIDEO',
      outputFaceBlendshapes: true,
      outputFacialTransformationMatrixes: false,
      numFaces: 1,
    });

    this.smoothed = zeroWeights();
    this._running = true;
  }

  /**
   * Process a single video frame and return smoothed VRM expression weights.
   *
   * @param video — The HTMLVideoElement with the camera feed.
   * @param dt — Delta time in seconds since last frame.
   * @param lambda — EMA smoothing factor (default 12).
   */
  update(video: HTMLVideoElement, dt: number, lambda = DEFAULT_LAMBDA): VrmExpressionWeights {
    if (!this.landmarker || !this._running) return this.smoothed;

    // Avoid re-processing the same frame
    const timestamp = video.currentTime * 1000;
    if (timestamp === this._lastTimestamp) return this.smoothed;
    this._lastTimestamp = timestamp;

    const result = this.landmarker.detectForVideo(video, performance.now());

    if (result.faceBlendshapes && result.faceBlendshapes.length > 0) {
      const categories = result.faceBlendshapes[0].categories;
      const scores = new Map<string, number>();
      for (const cat of categories) {
        scores.set(cat.categoryName, cat.score);
      }
      this._lastRawScores = scores;
      const raw = mapBlendshapesToVRM(scores);
      smoothWeights(this.smoothed, raw, lambda, dt);
    }

    return this.smoothed;
  }

  /** Tear down the FaceLandmarker and release WASM resources. */
  dispose(): void {
    this._running = false;
    if (this.landmarker) {
      this.landmarker.close();
      this.landmarker = null;
    }
    this.smoothed = zeroWeights();
    this._lastRawScores = new Map();
  }
}
