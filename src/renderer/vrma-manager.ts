/**
 * VRMA Animation Manager — loads and plays .vrma animation files on a VRM model.
 *
 * Uses @pixiv/three-vrm-animation to load VRMAnimation data from .vrma files,
 * then creates THREE.AnimationClips bound to the current VRM and plays them
 * through a THREE.AnimationMixer.
 *
 * When a VRMA animation is active, the mixer drives bone transforms and the
 * procedural CharacterAnimator should yield bone control.
 */

import * as THREE from 'three';
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';
import type { VRM } from '@pixiv/three-vrm';
import {
  VRMAnimationLoaderPlugin,
  createVRMAnimationClip,
} from '@pixiv/three-vrm-animation';
import type { CharacterState } from '../types';
import type { ModelGender } from '../config/default-models';

// ── Animation registry: maps keys to VRMA file paths ─────────────────────────

export interface VrmaAnimationEntry {
  /** Display name of the animation */
  label: string;
  /** URL path to the .vrma file (relative to public/) */
  path: string;
  /** Whether the animation should loop */
  loop: boolean;
  /** Optional mood association — auto-plays when this mood is set */
  mood?: CharacterState;
  /** Motion key the LLM can emit via <anim>{"motion":"key"}</anim> to trigger this animation */
  motionKey?: string;
}

/**
 * All available VRMA animations.
 * Mood-mapped animations auto-play when the corresponding mood is triggered.
 */
export const VRMA_ANIMATIONS: VrmaAnimationEntry[] = [
  // Idle/general
  { label: 'Idle',        path: '/animations/idle.vrma',        loop: true,  mood: undefined,     motionKey: 'idle' },
  { label: 'Ladylike',    path: '/animations/ladylike.vrma',    loop: true,                       motionKey: 'ladylike' },
  { label: 'Walk',        path: '/animations/walk.vrma',        loop: true,                       motionKey: 'walk' },
  { label: 'Greeting',    path: '/animations/greeting.vrma',    loop: false, mood: 'happy',       motionKey: 'greeting' },
  { label: 'Peace Sign',  path: '/animations/peace-sign.vrma',  loop: false,                      motionKey: 'peace' },
  { label: 'Spin',        path: '/animations/spin.vrma',        loop: false,                      motionKey: 'spin' },
  { label: 'Model Pose',  path: '/animations/model-pose.vrma',  loop: false, mood: 'relaxed',     motionKey: 'pose' },
  { label: 'Squat',       path: '/animations/squat.vrma',       loop: false,                      motionKey: 'squat' },
  // Emotion-mapped
  { label: 'Angry',       path: '/animations/angry.vrma',       loop: true,  mood: 'angry',       motionKey: 'angry' },
  { label: 'Sad',         path: '/animations/sad.vrma',         loop: true,  mood: 'sad',         motionKey: 'sad' },
  { label: 'Thinking',    path: '/animations/thinking.vrma',    loop: true,  mood: 'thinking',    motionKey: 'thinking' },
  { label: 'Surprised',   path: '/animations/surprised.vrma',   loop: false, mood: 'surprised',   motionKey: 'surprised' },
  { label: 'Relax',       path: '/animations/relax.vrma',       loop: true,  mood: 'relaxed',     motionKey: 'relax' },
  { label: 'Sleepy',      path: '/animations/sleepy.vrma',      loop: true,                       motionKey: 'sleepy' },
  { label: 'Clapping',    path: '/animations/clapping.vrma',    loop: false,                      motionKey: 'clapping' },
  { label: 'Jump',        path: '/animations/jump.vrma',        loop: false,                      motionKey: 'jump' },
  // Additional
  { label: 'Waiting',     path: '/animations/waiting.vrma',     loop: true,                       motionKey: 'waiting' },
  { label: 'Appearing',   path: '/animations/appearing.vrma',   loop: false,                      motionKey: 'appearing' },
  { label: 'Liked',       path: '/animations/liked.vrma',       loop: false,                      motionKey: 'liked' },
];

/** All valid motion keys the LLM can emit. Used for prompt and validation. */
export const VALID_MOTION_KEYS: readonly string[] = VRMA_ANIMATIONS
  .map(a => a.motionKey)
  .filter((k): k is string => !!k);

/** Paths of VRMA animations where the character is in a seated pose. */
export const SITTING_ANIMATION_PATHS = new Set([
  '/animations/relax.vrma',
]);

/**
 * Find a VRMA animation entry mapped to a given mood.
 * Returns the first matching entry, or undefined if none mapped.
 */
export function getAnimationForMood(mood: CharacterState): VrmaAnimationEntry | undefined {
  return VRMA_ANIMATIONS.find(a => a.mood === mood);
}

/**
 * Pick the idle loop animation based on model gender.
 * Female models prefer `ladylike.vrma` most of the time; male models default
 * to the standard `idle.vrma` loop.
 *
 * When `excludeSitting` is true, any idle animation that is also in
 * `SITTING_ANIMATION_PATHS` is skipped — used by pet mode / the floating
 * preview where a chair prop would visibly float in mid-air.
 */
export function getIdleAnimationForGender(
  gender: ModelGender,
  random: () => number = Math.random,
  excludeSitting = false,
): VrmaAnimationEntry | undefined {
  const idle = VRMA_ANIMATIONS.find(a => a.motionKey === 'idle');
  const ladylike = VRMA_ANIMATIONS.find(a => a.motionKey === 'ladylike');
  const ladylikeAvailable = ladylike && !(excludeSitting && SITTING_ANIMATION_PATHS.has(ladylike.path));

  if (gender === 'female' && ladylikeAvailable) {
    // 75% chance ladylike, 25% standard idle for variety
    if (random() < 0.75) return ladylike;
  }
  return idle ?? (ladylikeAvailable ? ladylike : undefined);
}

/**
 * Find a non-sitting alternative animation for a mood. Used by the floating
 * pet preview where seated animations would spawn a chair that floats in
 * mid-air. Falls back to `undefined` when there is no standing equivalent.
 */
export function getStandingAnimationForMood(
  mood: CharacterState,
): VrmaAnimationEntry | undefined {
  return VRMA_ANIMATIONS.find(
    a => a.mood === mood && !SITTING_ANIMATION_PATHS.has(a.path),
  );
}

/**
 * Aliases so the LLM can use natural words instead of exact motion keys.
 * Maps common synonyms → canonical motionKey used in VRMA_ANIMATIONS.
 */
const MOTION_ALIASES: Record<string, string> = {
  lady:      'ladylike',
  clap:      'clapping',
  applause:  'clapping',
  applaud:   'clapping',
  wave:      'greeting',
  hello:     'greeting',
  hi:        'greeting',
  bye:       'greeting',
  goodbye:   'greeting',
  nod:       'greeting',
  dance:     'spin',
  twirl:     'spin',
  mad:       'angry',
  furious:   'angry',
  cry:       'sad',
  sigh:      'sad',
  wonder:    'thinking',
  think:     'thinking',
  ponder:    'thinking',
  shock:     'surprised',
  gasp:      'surprised',
  chill:     'relax',
  rest:      'relax',
  sleep:     'sleepy',
  yawn:      'sleepy',
  hop:       'jump',
  leap:      'jump',
  model:     'pose',
  strike:    'pose',
  crouch:    'squat',
  victory:   'peace',
  stroll:    'walk',
  wait:      'waiting',
  patient:   'waiting',
  appear:    'appearing',
  entrance:  'appearing',
  arrive:    'appearing',
  like:      'liked',
  love:      'liked',
  heart:     'liked',
  adore:     'liked',
};

/**
 * Find a VRMA animation entry by its motion key (as emitted by the LLM).
 * Supports aliases so the LLM can say "clap" instead of "clapping".
 * Returns the matching entry, or undefined if the key is unrecognised.
 */
export function getAnimationForMotion(motionKey: string): VrmaAnimationEntry | undefined {
  const lower = motionKey.toLowerCase();
  const canonical = MOTION_ALIASES[lower] ?? lower;
  return VRMA_ANIMATIONS.find(a => a.motionKey === canonical);
}

// ── VrmaManager class ────────────────────────────────────────────────────────

export class VrmaManager {
  private loader: GLTFLoader;
  private mixer: THREE.AnimationMixer | null = null;
  private currentAction: THREE.AnimationAction | null = null;
  private currentVrm: VRM | null = null;
  private clipCache = new Map<string, THREE.AnimationClip>();

  /** True when a VRMA animation is actively playing (mixer drives bones). */
  private _isPlaying = false;

  /** Path of the currently playing animation (null when stopped). */
  private _currentPath: string | null = null;

  /**
   * When true, the mood watcher in CharacterViewport should not auto-play
   * mood-mapped animations.  Set by `playMotion()` and cleared by `stop()`.
   * This prevents the mood watcher from overriding an explicit LLM motion.
   */
  private _suppressMoodAnimation = false;

  /** Fires when VRMA playback starts/stops so the CharacterAnimator can
   *  yield/reclaim bone control. */
  private _onPlaybackChange: ((playing: boolean) => void) | null = null;

  constructor() {
    this.loader = new GLTFLoader();
    this.loader.register((parser) => new VRMAnimationLoaderPlugin(parser));
  }

  /** Whether a VRMA animation is currently playing. */
  get isPlaying(): boolean {
    return this._isPlaying;
  }

  /** Path of the currently playing animation, or null. */
  get currentPath(): string | null {
    return this._currentPath;
  }

  /** Whether mood-mapped auto-play should be suppressed (explicit motion active). */
  get isMoodSuppressed(): boolean {
    return this._suppressMoodAnimation;
  }

  /** The currently bound VRM model (null until setVRM is called). */
  get vrm(): VRM | null {
    return this.currentVrm;
  }

  /** Mark that an explicit motion is playing — mood watcher should not override. */
  suppressMoodAnimation() {
    this._suppressMoodAnimation = true;
  }

  /** Clear the mood suppression flag. */
  clearMoodSuppression() {
    this._suppressMoodAnimation = false;
  }

  /** Register a callback for playback state changes. */
  onPlaybackChange(cb: (playing: boolean) => void) {
    this._onPlaybackChange = cb;
  }

  /** Bind to a VRM model. Must be called after loading a new model. */
  setVRM(vrm: VRM) {
    this.stop();
    this.currentVrm = vrm;
    this.mixer = new THREE.AnimationMixer(vrm.scene);
    this.clipCache.clear();

    // Listen for animation finished events to clean up fully
    this.mixer.addEventListener('finished', () => {
      if (this.currentAction) {
        this.currentAction.stop();
        this.currentAction = null;
      }
      this._currentPath = null;
      this._suppressMoodAnimation = false;
      this.setPlaybackState(false);
    });
  }

  /** Load a VRMA file and return the generated AnimationClip for the current VRM. */
  async loadClip(path: string): Promise<THREE.AnimationClip | null> {
    if (!this.currentVrm) return null;

    // Check cache
    if (this.clipCache.has(path)) {
      return this.clipCache.get(path)!;
    }

    try {
      const gltf = await this.loader.loadAsync(path);
      const vrmAnimation = gltf.userData.vrmAnimations?.[0];
      if (!vrmAnimation) {
        console.warn('[VrmaManager] No VRM animation found in:', path);
        return null;
      }

      // Cast through unknown to bypass private property incompatibility between
      // @pixiv/three-vrm and @pixiv/three-vrm-animation's internal VRMCore types.
      const clip = createVRMAnimationClip(vrmAnimation, this.currentVrm as unknown as Parameters<typeof createVRMAnimationClip>[1]);
      if (clip) {
        this.clipCache.set(path, clip);
      }
      return clip;
    } catch (error) {
      console.error('[VrmaManager] Failed to load VRMA:', path, error);
      return null;
    }
  }

  /**
   * Play a VRMA animation by path. Stops any current animation first.
   * @param path URL to the .vrma file
   * @param loop Whether to loop the animation
   * @param fadeIn Crossfade duration in seconds (default 0.3s)
   */
  async play(path: string, loop = true, fadeIn = 0.3): Promise<boolean> {
    if (!this.mixer || !this.currentVrm) return false;

    const clip = await this.loadClip(path);
    if (!clip) return false;

    this._currentPath = path;
    return this.playClip(clip, loop, fadeIn);
  }

  /**
   * Play a pre-built AnimationClip (e.g. from vrma-baker.ts bakeMotionToClip).
   * Stops any current animation first with crossfade.
   * @param clip The AnimationClip to play.
   * @param loop Whether to loop the animation.
   * @param fadeIn Crossfade duration in seconds (default 0.3s).
   */
  playClip(clip: THREE.AnimationClip, loop = false, fadeIn = 0.3): boolean {
    if (!this.mixer) return false;

    // Stop previous action with fadeout
    if (this.currentAction) {
      this.currentAction.fadeOut(fadeIn);
    }

    const action = this.mixer.clipAction(clip);
    action.setLoop(loop ? THREE.LoopRepeat : THREE.LoopOnce, loop ? Infinity : 1);
    action.clampWhenFinished = !loop;
    action.reset();
    action.fadeIn(fadeIn);
    action.play();

    this.currentAction = action;
    this.setPlaybackState(true);
    return true;
  }

  /** Stop the current animation and return to procedural control. */
  stop(fadeOut = 0.3) {
    if (this.currentAction) {
      this.currentAction.fadeOut(fadeOut);
      // After fadeout, stop completely
      setTimeout(() => {
        if (this.currentAction) {
          this.currentAction.stop();
          this.currentAction = null;
        }
      }, fadeOut * 1000);
    }
    this._currentPath = null;
    this._suppressMoodAnimation = false;
    this.setPlaybackState(false);
  }

  /** Pause / unpause the current animation. */
  togglePause() {
    if (this.currentAction) {
      this.currentAction.paused = !this.currentAction.paused;
    }
  }

  /** Call every frame to advance the animation mixer. */
  update(delta: number) {
    if (this.mixer && this._isPlaying) {
      this.mixer.update(delta);
    }
  }

  /** Clean up when changing models. */
  dispose() {
    this.stop(0);
    this._currentPath = null;
    this.mixer = null;
    this.currentVrm = null;
    this.clipCache.clear();
  }

  private setPlaybackState(playing: boolean) {
    if (this._isPlaying !== playing) {
      this._isPlaying = playing;
      this._onPlaybackChange?.(playing);
    }
  }
}
