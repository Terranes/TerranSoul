/**
 * learned-motion-player.ts — Play learned motion clips on the VRM avatar.
 *
 * Thin orchestrator that bakes a {@link LearnedMotion} into a
 * `THREE.AnimationClip` (via {@link bakeMotionToClip}) and delegates
 * playback to {@link VrmaManager.playClip}.
 *
 * Also provides a static helper to preview a {@link LearnedExpression}
 * on the VRM by setting expression manager weights directly.
 */

import type { VRM } from '@pixiv/three-vrm';
import { bakeMotionToClip } from './vrma-baker';
import type { VrmaManager } from './vrma-manager';
import type { LearnedExpression, LearnedMotion } from '../stores/persona-types';

// ── Motion playback ───────────────────────────────────────────────────────────

export class LearnedMotionPlayer {
  private manager: VrmaManager;

  constructor(vrmaManager: VrmaManager) {
    this.manager = vrmaManager;
  }

  /**
   * Bake a learned motion clip into an AnimationClip and play it.
   * Returns false if the motion has no valid frames.
   */
  play(motion: LearnedMotion, loop = false, fadeIn = 0.3): boolean {
    const clip = bakeMotionToClip(motion);
    if (!clip) return false;
    return this.manager.playClip(clip, loop, fadeIn);
  }

  /** Stop any playing animation and return to procedural control. */
  stop(fadeOut = 0.3): void {
    this.manager.stop(fadeOut);
  }

  /** Whether an animation is currently playing. */
  get isPlaying(): boolean {
    return this.manager.isPlaying;
  }
}

// ── Expression preview ────────────────────────────────────────────────────────

/**
 * Apply a learned expression's weights to a VRM model.
 * Sets expression manager values directly — caller should reset after
 * preview timeout (e.g. 3 seconds).
 */
export function applyLearnedExpression(vrm: VRM, expr: LearnedExpression): void {
  const mgr = vrm.expressionManager;
  if (!mgr) return;

  // Main emotion + viseme weights
  for (const [name, weight] of Object.entries(expr.weights)) {
    mgr.setValue(name, weight);
  }

  // Blink
  if (expr.blink !== undefined) {
    mgr.setValue('blink', expr.blink);
  }
}

/**
 * Reset all expression weights back to zero.
 * Called after a timed expression preview ends.
 */
export function clearExpressionPreview(vrm: VRM): void {
  const mgr = vrm.expressionManager;
  if (!mgr) return;

  const names = [
    'happy', 'sad', 'angry', 'relaxed', 'surprised', 'neutral',
    'aa', 'ih', 'ou', 'ee', 'oh', 'blink',
  ];
  for (const name of names) {
    mgr.setValue(name, 0);
  }
}
