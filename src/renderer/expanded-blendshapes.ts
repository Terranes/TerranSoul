/**
 * expanded-blendshapes.ts — opt-in per-ARKit-blendshape passthrough
 * for advanced VRM rigs (Chunk 27.3).
 *
 * The 6-preset baseline (`mapBlendshapesToVRM` in `face-mirror.ts`) is
 * the **default** path because every VRM model on disk is guaranteed to
 * ship those six channels. Some advanced rigs additionally publish raw
 * 52-channel ARKit shapes (`mouthSmileLeft`, `browInnerUp`, …) on the
 * VRM `expressionManager`. When the user opts in via
 * `AppSettings.expanded_blendshapes`, the live mirror calls this helper
 * to write each ARKit score to the rig **by name**; rigs that don't
 * ship a particular shape silently no-op because we probe for the
 * expression first.
 *
 * Design contract:
 * - **Pure & rig-aware:** never throws; works on any VRM regardless of
 *   how many ARKit shapes it ships.
 * - **Probe before set:** uses `expressionManager.getExpression(name)`
 *   to skip missing channels, so calling this on a stock 6-preset rig
 *   has zero effect.
 * - **Does not interfere with the 6-preset baseline:** the existing
 *   `mapBlendshapesToVRM` → `setValue('happy' | 'sad' | …)` path keeps
 *   running underneath; this helper only fans the *extra* channels.
 *
 * See `docs/persona-design.md` § 6.3.
 */

import type { VRM } from '@pixiv/three-vrm';

/** Canonical ARKit 52 blendshape names. */
export const ARKIT_BLENDSHAPE_NAMES = [
  // Brows (5)
  'browDownLeft', 'browDownRight', 'browInnerUp',
  'browOuterUpLeft', 'browOuterUpRight',
  // Cheeks (3)
  'cheekPuff', 'cheekSquintLeft', 'cheekSquintRight',
  // Eyes (12)
  'eyeBlinkLeft', 'eyeBlinkRight',
  'eyeLookDownLeft', 'eyeLookDownRight',
  'eyeLookInLeft', 'eyeLookInRight',
  'eyeLookOutLeft', 'eyeLookOutRight',
  'eyeLookUpLeft', 'eyeLookUpRight',
  'eyeSquintLeft', 'eyeSquintRight',
  'eyeWideLeft', 'eyeWideRight',
  // Jaw (4)
  'jawForward', 'jawLeft', 'jawRight', 'jawOpen',
  // Mouth (23)
  'mouthClose', 'mouthFunnel', 'mouthPucker',
  'mouthLeft', 'mouthRight',
  'mouthSmileLeft', 'mouthSmileRight',
  'mouthFrownLeft', 'mouthFrownRight',
  'mouthDimpleLeft', 'mouthDimpleRight',
  'mouthStretchLeft', 'mouthStretchRight',
  'mouthRollLower', 'mouthRollUpper',
  'mouthShrugLower', 'mouthShrugUpper',
  'mouthPressLeft', 'mouthPressRight',
  'mouthLowerDownLeft', 'mouthLowerDownRight',
  'mouthUpperUpLeft', 'mouthUpperUpRight',
  // Nose (2)
  'noseSneerLeft', 'noseSneerRight',
  // Tongue (1)
  'tongueOut',
] as const;

export type ArkitBlendshapeName = typeof ARKIT_BLENDSHAPE_NAMES[number];

/**
 * Names already covered by the 6-preset baseline. Skipped here so we
 * don't double-write `eyeBlinkLeft` (the baseline collapses both eyes
 * to a single `blink` channel). Advanced rigs that genuinely want
 * per-eye blink can list those names in `extraNames` instead.
 */
const BASELINE_OVERLAP: ReadonlySet<string> = new Set([
  'eyeBlinkLeft', 'eyeBlinkRight',
]);

function clamp01(v: number): number {
  return v < 0 ? 0 : v > 1 ? 1 : v;
}

/**
 * Fan a raw ARKit blendshape map onto every matching VRM expression
 * channel by name. Channels that don't exist on the rig are silently
 * skipped, so this is safe to call on any VRM regardless of authoring.
 *
 * @param vrm     The VRM to write into. `vrm.expressionManager` may be
 *                `null` (stock rigs without an expression manager) — in
 *                that case the call is a no-op.
 * @param scores  Map of ARKit name → coefficient (0–1). MediaPipe
 *                `FaceLandmarkerResult.faceBlendshapes[0].categories`
 *                can be flattened into this shape; missing keys are
 *                treated as 0 and not written.
 */
export function applyExpandedBlendshapes(
  vrm: VRM,
  scores: ReadonlyMap<string, number>,
): void {
  const mgr = vrm.expressionManager;
  if (!mgr) return;
  for (const name of ARKIT_BLENDSHAPE_NAMES) {
    if (BASELINE_OVERLAP.has(name)) continue;
    const score = scores.get(name);
    if (score === undefined) continue;
    // Probe first so missing rig channels stay silent (most VRMs).
    if (!mgr.getExpression(name)) continue;
    mgr.setValue(name, clamp01(score));
  }
}

/**
 * Companion reset — zeroes every ARKit channel the rig actually ships.
 * Symmetric with `clearExpressionPreview` in `learned-motion-player.ts`
 * but only touches the *extra* channels written by
 * `applyExpandedBlendshapes`. Safe to call on stock rigs (no-op).
 */
export function clearExpandedBlendshapes(vrm: VRM): void {
  const mgr = vrm.expressionManager;
  if (!mgr) return;
  for (const name of ARKIT_BLENDSHAPE_NAMES) {
    if (BASELINE_OVERLAP.has(name)) continue;
    if (!mgr.getExpression(name)) continue;
    mgr.setValue(name, 0);
  }
}
