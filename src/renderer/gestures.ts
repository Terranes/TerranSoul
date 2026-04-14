/**
 * Built-in gesture definitions and registry.
 *
 * Each gesture is a short animation sequence defined as keyframe data.
 * Keyframes specify Euler rotation offsets (in radians) relative to the
 * character's current bone rotation. At time 0 and at the end of the gesture
 * all offsets should return to zero so the gesture blends cleanly into the
 * idle/active pose.
 *
 * Coordinate convention: same as pose-presets.ts (Three.js Y-up).
 */

export interface GestureKeyframe {
  /** Time in seconds from the start of the gesture. */
  time: number;
  /** Bone name → Euler offset {x, y, z} in radians. */
  bones: Partial<Record<string, { x: number; y: number; z: number }>>;
}

export interface GestureDefinition {
  id: string;
  /** Total duration in seconds. */
  duration: number;
  keyframes: GestureKeyframe[];
}

// ── Gesture library ───────────────────────────────────────────────────────────

const GESTURES: GestureDefinition[] = [
  {
    id: 'nod',
    duration: 0.6,
    keyframes: [
      { time: 0.0,  bones: { head: { x:  0.00, y: 0, z: 0 } } },
      { time: 0.15, bones: { head: { x:  0.28, y: 0, z: 0 }, neck: { x: 0.10, y: 0, z: 0 } } },
      { time: 0.35, bones: { head: { x:  0.05, y: 0, z: 0 }, neck: { x: 0.02, y: 0, z: 0 } } },
      { time: 0.50, bones: { head: { x:  0.18, y: 0, z: 0 }, neck: { x: 0.06, y: 0, z: 0 } } },
      { time: 0.60, bones: { head: { x:  0.00, y: 0, z: 0 }, neck: { x: 0.00, y: 0, z: 0 } } },
    ],
  },
  {
    id: 'wave',
    duration: 1.2,
    keyframes: [
      { time: 0.00, bones: { rightUpperArm: { x: 0, y: 0, z:  0.00 }, rightLowerArm: { x: 0, y: 0, z:  0.00 } } },
      { time: 0.25, bones: { rightUpperArm: { x: 0, y: 0, z: -0.80 }, rightLowerArm: { x: 0.6, y: 0, z:  0.00 } } },
      { time: 0.45, bones: { rightUpperArm: { x: 0, y: 0, z: -0.80 }, rightLowerArm: { x: 0.6, y: 0.3, z:  0.00 } } },
      { time: 0.60, bones: { rightUpperArm: { x: 0, y: 0, z: -0.80 }, rightLowerArm: { x: 0.6, y:-0.3, z:  0.00 } } },
      { time: 0.75, bones: { rightUpperArm: { x: 0, y: 0, z: -0.80 }, rightLowerArm: { x: 0.6, y: 0.3, z:  0.00 } } },
      { time: 0.90, bones: { rightUpperArm: { x: 0, y: 0, z: -0.80 }, rightLowerArm: { x: 0.6, y:-0.2, z:  0.00 } } },
      { time: 1.20, bones: { rightUpperArm: { x: 0, y: 0, z:  0.00 }, rightLowerArm: { x: 0, y: 0, z:  0.00 } } },
    ],
  },
  {
    id: 'shrug',
    duration: 0.8,
    keyframes: [
      { time: 0.00, bones: { leftShoulder: { x: 0, y: 0, z:  0.00 }, rightShoulder: { x: 0, y: 0, z:  0.00 }, head: { x: 0.00, y: 0, z: 0 } } },
      { time: 0.25, bones: { leftShoulder: { x: 0, y: 0, z: -0.30 }, rightShoulder: { x: 0, y: 0, z:  0.30 }, head: { x: 0.06, y: 0, z: 0 } } },
      { time: 0.50, bones: { leftShoulder: { x: 0, y: 0, z: -0.35 }, rightShoulder: { x: 0, y: 0, z:  0.35 }, head: { x: 0.08, y: 0, z: 0 } } },
      { time: 0.80, bones: { leftShoulder: { x: 0, y: 0, z:  0.00 }, rightShoulder: { x: 0, y: 0, z:  0.00 }, head: { x: 0.00, y: 0, z: 0 } } },
    ],
  },
  {
    id: 'lean-in',
    duration: 1.0,
    keyframes: [
      { time: 0.00, bones: { hips: { x:  0.00, y: 0, z: 0 }, spine: { x:  0.00, y: 0, z: 0 }, head: { x:  0.00, y: 0, z: 0 } } },
      { time: 0.30, bones: { hips: { x: -0.08, y: 0, z: 0 }, spine: { x: -0.10, y: 0, z: 0 }, head: { x: -0.06, y: 0, z: 0 } } },
      { time: 0.60, bones: { hips: { x: -0.08, y: 0, z: 0 }, spine: { x: -0.10, y: 0, z: 0 }, head: { x: -0.06, y: 0, z: 0 } } },
      { time: 1.00, bones: { hips: { x:  0.00, y: 0, z: 0 }, spine: { x:  0.00, y: 0, z: 0 }, head: { x:  0.00, y: 0, z: 0 } } },
    ],
  },
  {
    id: 'head-tilt',
    duration: 0.6,
    keyframes: [
      { time: 0.00, bones: { neck: { x: 0, y: 0, z:  0.00 }, head: { x: 0, y: 0, z:  0.00 } } },
      { time: 0.20, bones: { neck: { x: 0, y: 0, z: -0.18 }, head: { x: 0, y: 0, z: -0.12 } } },
      { time: 0.45, bones: { neck: { x: 0, y: 0, z: -0.20 }, head: { x: 0, y: 0, z: -0.14 } } },
      { time: 0.60, bones: { neck: { x: 0, y: 0, z:  0.00 }, head: { x: 0, y: 0, z:  0.00 } } },
    ],
  },
  {
    id: 'reach-out',
    duration: 0.8,
    keyframes: [
      { time: 0.00, bones: { rightUpperArm: { x:  0.00, y: 0, z:  0.00 }, rightLowerArm: { x: 0.00, y: 0, z: 0 } } },
      { time: 0.30, bones: { rightUpperArm: { x: -0.50, y: 0, z: -0.20 }, rightLowerArm: { x: 0.30, y: 0, z: 0 } } },
      { time: 0.55, bones: { rightUpperArm: { x: -0.55, y: 0, z: -0.22 }, rightLowerArm: { x: 0.30, y: 0, z: 0 } } },
      { time: 0.80, bones: { rightUpperArm: { x:  0.00, y: 0, z:  0.00 }, rightLowerArm: { x: 0.00, y: 0, z: 0 } } },
    ],
  },
  {
    id: 'bow',
    duration: 1.2,
    keyframes: [
      { time: 0.00, bones: { hips: { x:  0.00, y: 0, z: 0 }, spine: { x:  0.00, y: 0, z: 0 }, chest: { x:  0.00, y: 0, z: 0 }, head: { x:  0.00, y: 0, z: 0 } } },
      { time: 0.35, bones: { hips: { x:  0.20, y: 0, z: 0 }, spine: { x:  0.30, y: 0, z: 0 }, chest: { x:  0.25, y: 0, z: 0 }, head: { x:  0.15, y: 0, z: 0 } } },
      { time: 0.65, bones: { hips: { x:  0.22, y: 0, z: 0 }, spine: { x:  0.32, y: 0, z: 0 }, chest: { x:  0.26, y: 0, z: 0 }, head: { x:  0.16, y: 0, z: 0 } } },
      { time: 1.20, bones: { hips: { x:  0.00, y: 0, z: 0 }, spine: { x:  0.00, y: 0, z: 0 }, chest: { x:  0.00, y: 0, z: 0 }, head: { x:  0.00, y: 0, z: 0 } } },
    ],
  },
  {
    id: 'nod-slow',
    duration: 1.2,
    keyframes: [
      { time: 0.00, bones: { head: { x:  0.00, y: 0, z: 0 }, neck: { x:  0.00, y: 0, z: 0 } } },
      { time: 0.35, bones: { head: { x:  0.22, y: 0, z: 0 }, neck: { x:  0.08, y: 0, z: 0 } } },
      { time: 0.70, bones: { head: { x: -0.04, y: 0, z: 0 }, neck: { x: -0.02, y: 0, z: 0 } } },
      { time: 1.00, bones: { head: { x:  0.14, y: 0, z: 0 }, neck: { x:  0.05, y: 0, z: 0 } } },
      { time: 1.20, bones: { head: { x:  0.00, y: 0, z: 0 }, neck: { x:  0.00, y: 0, z: 0 } } },
    ],
  },
  {
    id: 'shake-head',
    duration: 0.8,
    keyframes: [
      { time: 0.00, bones: { head: { x: 0, y:  0.00, z: 0 }, neck: { x: 0, y:  0.00, z: 0 } } },
      { time: 0.15, bones: { head: { x: 0, y: -0.22, z: 0 }, neck: { x: 0, y: -0.10, z: 0 } } },
      { time: 0.35, bones: { head: { x: 0, y:  0.22, z: 0 }, neck: { x: 0, y:  0.10, z: 0 } } },
      { time: 0.55, bones: { head: { x: 0, y: -0.15, z: 0 }, neck: { x: 0, y: -0.07, z: 0 } } },
      { time: 0.70, bones: { head: { x: 0, y:  0.08, z: 0 }, neck: { x: 0, y:  0.04, z: 0 } } },
      { time: 0.80, bones: { head: { x: 0, y:  0.00, z: 0 }, neck: { x: 0, y:  0.00, z: 0 } } },
    ],
  },
  {
    id: 'idle-fidget',
    duration: 1.6,
    keyframes: [
      { time: 0.00, bones: { spine: { x:  0.00, y:  0.00, z:  0.00 }, head: { x:  0.00, y:  0.00, z:  0.00 } } },
      { time: 0.40, bones: { spine: { x:  0.02, y:  0.02, z:  0.01 }, head: { x:  0.01, y:  0.03, z:  0.02 } } },
      { time: 0.80, bones: { spine: { x: -0.01, y: -0.01, z:  0.02 }, head: { x: -0.02, y: -0.02, z: -0.01 } } },
      { time: 1.20, bones: { spine: { x:  0.02, y:  0.01, z: -0.01 }, head: { x:  0.02, y:  0.01, z:  0.02 } } },
      { time: 1.60, bones: { spine: { x:  0.00, y:  0.00, z:  0.00 }, head: { x:  0.00, y:  0.00, z:  0.00 } } },
    ],
  },
];

// ── Registry ──────────────────────────────────────────────────────────────────

const GESTURE_INDEX: Map<string, GestureDefinition> = new Map(
  GESTURES.map(g => [g.id, g]),
);

/** Return all built-in gesture definitions. */
export function getAllGestures(): GestureDefinition[] {
  return GESTURES;
}

/** Return a gesture by id, or undefined if not found. */
export function getGesture(id: string): GestureDefinition | undefined {
  return GESTURE_INDEX.get(id);
}
