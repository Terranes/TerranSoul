/**
 * Layered avatar state model and animation state machine.
 *
 * All fast-changing animation data lives in a plain mutable object — NOT in
 * Vue reactive state — so the render loop can read it every frame without
 * triggering reactivity overhead or Tauri IPC.
 *
 * Layers:
 *   body   — coarse locomotion/activity state (idle/listen/think/talk)
 *   emotion — facial emotion overlay (neutral/happy/sad/angry/relaxed/surprised)
 *   viseme — mouth shape weights for lip sync (aa/ih/ou/ee/oh, each 0–1)
 *   blink  — eyelid closure weight (0 = open, 1 = closed)
 *   lookAt — gaze direction offset ({x, y} in normalized screen coords)
 *
 * The state machine enforces valid body transitions and prevents impossible
 * combos while keeping emotion/viseme/blink/lookAt independently settable.
 */

// ── Types ────────────────────────────────────────────────────────────────────

export type BodyState = 'idle' | 'listen' | 'think' | 'talk';
export type EmotionState = 'neutral' | 'happy' | 'sad' | 'angry' | 'relaxed' | 'surprised';

export interface VisemeWeights {
  aa: number;
  ih: number;
  ou: number;
  ee: number;
  oh: number;
}

export interface LookAtTarget {
  x: number;
  y: number;
}

/**
 * The avatar's full animation state — a plain mutable object.
 * Read directly in the frame loop; written by coarse state events only.
 */
export interface AvatarState {
  body: BodyState;
  emotion: EmotionState;
  viseme: VisemeWeights;
  blink: number;
  lookAt: LookAtTarget;
  /** Set to true whenever any channel changes; the render loop can use this
   *  for on-demand rendering optimisation (Chunk 126). */
  needsRender: boolean;
}

// ── Valid transitions ────────────────────────────────────────────────────────

/**
 * Allowed body state transitions. Each state lists which states it can
 * transition TO. `idle` is the universal reset target.
 *
 *   idle → listen → think → talk → idle
 *         ↘ think (skip listen)
 *   any  → idle   (always allowed as a reset)
 *   talk → think  (re-think mid-conversation)
 */
const BODY_TRANSITIONS: Record<BodyState, readonly BodyState[]> = {
  idle:   ['listen', 'think', 'talk'],
  listen: ['think', 'idle'],
  think:  ['talk', 'idle'],
  talk:   ['idle', 'think'],
};

// ── Factory ──────────────────────────────────────────────────────────────────

/** Create a fresh AvatarState with all channels at rest. */
export function createAvatarState(): AvatarState {
  return {
    body: 'idle',
    emotion: 'neutral',
    viseme: { aa: 0, ih: 0, ou: 0, ee: 0, oh: 0 },
    blink: 0,
    lookAt: { x: 0, y: 0 },
    needsRender: true,
  };
}

// ── State Machine ────────────────────────────────────────────────────────────

/**
 * Animation state machine that manages the layered AvatarState.
 *
 * - Body transitions are enforced (invalid transitions are silently ignored).
 * - Emotion, viseme, blink, and lookAt are independent layers — always settable.
 * - Blink runs as a self-contained cycle; external code can override it.
 * - LookAt defaults to (0,0) = camera; external code can redirect gaze.
 */
export class AvatarStateMachine {
  readonly state: AvatarState;

  // Blink subsystem
  private blinkActive = false;
  private blinkTimer = 0;
  private nextBlinkAt: number;
  private blinkOverridden = false;

  private static readonly BLINK_DURATION = 0.15;
  private static readonly MIN_BLINK_INTERVAL = 2.0;
  private static readonly MAX_BLINK_INTERVAL = 6.0;

  constructor(initial?: Partial<AvatarState>) {
    this.state = createAvatarState();
    if (initial) {
      if (initial.body) this.state.body = initial.body;
      if (initial.emotion) this.state.emotion = initial.emotion;
      if (initial.viseme) Object.assign(this.state.viseme, initial.viseme);
      if (initial.blink !== undefined) this.state.blink = initial.blink;
      if (initial.lookAt) Object.assign(this.state.lookAt, initial.lookAt);
    }
    this.nextBlinkAt = AvatarStateMachine.randomBlinkInterval();
  }

  // ── Body layer ───────────────────────────────────────────────────────

  /**
   * Request a body state transition.  Returns true if the transition was
   * accepted, false if it was rejected (invalid transition).
   */
  setBody(target: BodyState): boolean {
    if (target === this.state.body) return true;

    // `idle` is always a valid target from any state (universal reset)
    if (target === 'idle') {
      this.state.body = 'idle';
      this.state.needsRender = true;
      return true;
    }

    const allowed = BODY_TRANSITIONS[this.state.body];
    if (allowed.includes(target)) {
      this.state.body = target;
      this.state.needsRender = true;
      return true;
    }

    return false;
  }

  /** Force a body state regardless of transition rules (for error recovery). */
  forceBody(target: BodyState): void {
    this.state.body = target;
    this.state.needsRender = true;
  }

  // ── Emotion layer ────────────────────────────────────────────────────

  /** Set the facial emotion. Always accepted — emotions overlay any body state. */
  setEmotion(emotion: EmotionState): void {
    if (this.state.emotion !== emotion) {
      this.state.emotion = emotion;
      this.state.needsRender = true;
    }
  }

  // ── Viseme layer ─────────────────────────────────────────────────────

  /**
   * Update viseme weights. Only applied when body is `talk`; otherwise
   * visemes are zeroed so the mouth stays closed.
   */
  setViseme(weights: Partial<VisemeWeights>): void {
    if (this.state.body !== 'talk') {
      this.zeroVisemes();
      return;
    }
    const v = this.state.viseme;
    if (weights.aa !== undefined) v.aa = clamp01(weights.aa);
    if (weights.ih !== undefined) v.ih = clamp01(weights.ih);
    if (weights.ou !== undefined) v.ou = clamp01(weights.ou);
    if (weights.ee !== undefined) v.ee = clamp01(weights.ee);
    if (weights.oh !== undefined) v.oh = clamp01(weights.oh);
    this.state.needsRender = true;
  }

  /** Zero all viseme weights (close mouth). */
  zeroVisemes(): void {
    const v = this.state.viseme;
    if (v.aa === 0 && v.ih === 0 && v.ou === 0 && v.ee === 0 && v.oh === 0) return;
    v.aa = v.ih = v.ou = v.ee = v.oh = 0;
    this.state.needsRender = true;
  }

  // ── Blink layer ──────────────────────────────────────────────────────

  /**
   * Tick the automatic blink cycle.  Call once per frame with delta seconds.
   * When blink is overridden (e.g. by a "surprised" wide-eye), the automatic
   * cycle pauses until `releaseBlinkOverride()` is called.
   */
  tickBlink(delta: number): void {
    if (this.blinkOverridden) return;

    if (!this.blinkActive) {
      this.nextBlinkAt -= delta;
      if (this.nextBlinkAt <= 0) {
        this.blinkActive = true;
        this.blinkTimer = 0;
      }
    }

    if (this.blinkActive) {
      this.blinkTimer += delta;
      const half = AvatarStateMachine.BLINK_DURATION / 2;
      if (this.blinkTimer < half) {
        this.state.blink = this.blinkTimer / half;
      } else if (this.blinkTimer < AvatarStateMachine.BLINK_DURATION) {
        this.state.blink = 1.0 - (this.blinkTimer - half) / half;
      } else {
        this.state.blink = 0;
        this.blinkActive = false;
        this.nextBlinkAt = AvatarStateMachine.randomBlinkInterval();
      }
      this.state.needsRender = true;
    }
  }

  /** Override blink to a specific value (e.g. 0 for wide-eyed surprise). */
  overrideBlink(value: number): void {
    this.blinkOverridden = true;
    this.state.blink = clamp01(value);
    this.blinkActive = false;
    this.state.needsRender = true;
  }

  /** Release blink override — automatic cycle resumes. */
  releaseBlinkOverride(): void {
    this.blinkOverridden = false;
    this.nextBlinkAt = AvatarStateMachine.randomBlinkInterval();
  }

  // ── LookAt layer ─────────────────────────────────────────────────────

  /** Set gaze direction (normalised coords, 0,0 = camera center). */
  setLookAt(x: number, y: number): void {
    if (this.state.lookAt.x !== x || this.state.lookAt.y !== y) {
      this.state.lookAt.x = x;
      this.state.lookAt.y = y;
      this.state.needsRender = true;
    }
  }

  // ── Convenience ──────────────────────────────────────────────────────

  /** Reset all channels to resting state. */
  reset(): void {
    this.state.body = 'idle';
    this.state.emotion = 'neutral';
    this.zeroVisemes();
    this.state.blink = 0;
    this.state.lookAt.x = 0;
    this.state.lookAt.y = 0;
    this.blinkOverridden = false;
    this.blinkActive = false;
    this.nextBlinkAt = AvatarStateMachine.randomBlinkInterval();
    this.state.needsRender = true;
  }

  /**
   * Check if all damped values have settled (within epsilon of targets).
   * Useful for on-demand rendering — when settled, render rate can drop.
   */
  isSettled(): boolean {
    if (this.blinkActive) return false;
    if (this.state.body !== 'idle') return false;
    const v = this.state.viseme;
    if (v.aa > 0.001 || v.ih > 0.001 || v.ou > 0.001 || v.ee > 0.001 || v.oh > 0.001) {
      return false;
    }
    return true;
  }

  // ── Internal ─────────────────────────────────────────────────────────

  private static randomBlinkInterval(): number {
    return AvatarStateMachine.MIN_BLINK_INTERVAL +
      Math.random() * (AvatarStateMachine.MAX_BLINK_INTERVAL - AvatarStateMachine.MIN_BLINK_INTERVAL);
  }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

function clamp01(v: number): number {
  return v < 0 ? 0 : v > 1 ? 1 : v;
}
