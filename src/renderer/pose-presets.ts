/**
 * VRM Humanoid Pose Presets
 *
 * Each preset defines Euler angle offsets (radians) for key upper-body bones.
 * These offsets are applied *additively* on top of the AnimationMixer's output —
 * they do not replace the keyframe animation; they tilt and pose the character.
 *
 * Bone names follow the VRMHumanBoneName convention from @pixiv/three-vrm.
 * Only bones that actually change from neutral are listed (sparse representation).
 *
 * Coordinate convention (VRM / Three.js Y-up, right-hand rule):
 *   x > 0 → forward pitch (head bows, spine leans forward)
 *   x < 0 → backward pitch (head lifts, spine leans back)
 *   y     → yaw (turn left / right)
 *   z     → roll (tilt left / right)
 */

export interface PoseBoneRotation {
  x: number; // Euler X in radians
  y: number; // Euler Y in radians
  z: number; // Euler Z in radians
}

export interface PosePreset {
  id: string;
  label: string;
  /** Sparse map of bone name → Euler rotation offset (radians). */
  boneRotations: Partial<Record<string, PoseBoneRotation>>;
}

/** All recognized pose preset IDs. */
export type PosePresetId =
  | 'confident'
  | 'shy'
  | 'excited'
  | 'thoughtful'
  | 'relaxed'
  | 'defensive'
  | 'attentive'
  | 'playful'
  | 'bored'
  | 'empathetic';

// ── Pose definitions ──────────────────────────────────────────────────────────

const POSE_PRESETS: PosePreset[] = [
  {
    id: 'confident',
    label: 'Confident',
    boneRotations: {
      spine:         { x: -0.04, y: 0,    z: 0    }, // slight back lean
      chest:         { x: -0.05, y: 0,    z: 0    }, // chest out
      head:          { x: -0.06, y: 0,    z: 0    }, // head slightly raised
      leftUpperArm:  { x:  0,    y: 0.08, z: 0.22 }, // arms slightly away from body
      rightUpperArm: { x:  0,    y:-0.08, z:-0.22 },
    },
  },
  {
    id: 'shy',
    label: 'Shy',
    boneRotations: {
      spine:         { x:  0.10, y: 0,    z: 0    }, // hunched forward
      chest:         { x:  0.07, y: 0,    z: 0    },
      neck:          { x:  0.04, y: 0,    z: 0    },
      head:          { x:  0.12, y:-0.04, z:-0.06 }, // looking slightly down-left
      leftUpperArm:  { x:  0,    y:-0.12, z: 0.10 }, // arms pulled in
      rightUpperArm: { x:  0,    y: 0.12, z:-0.10 },
    },
  },
  {
    id: 'excited',
    label: 'Excited',
    boneRotations: {
      spine:         { x: -0.03, y: 0,    z: 0    },
      chest:         { x: -0.04, y: 0,    z: 0    },
      head:          { x: -0.04, y: 0,    z: 0    },
      leftUpperArm:  { x:  0,    y: 0.18, z: 0.38 }, // arms open/raised
      rightUpperArm: { x:  0,    y:-0.18, z:-0.38 },
      leftLowerArm:  { x:  0,    y: 0,    z:-0.18 }, // slight forearm tilt
      rightLowerArm: { x:  0,    y: 0,    z: 0.18 },
    },
  },
  {
    id: 'thoughtful',
    label: 'Thoughtful',
    boneRotations: {
      spine:         { x:  0.02, y: 0,    z: 0    },
      neck:          { x:  0,    y: 0,    z: 0.08 }, // head tilt to right
      head:          { x:  0.04, y: 0,    z: 0.06 }, // looking slightly down-right
      rightUpperArm: { x:  0.25, y: 0.1,  z:-0.10 }, // hand-to-chin pose
      rightLowerArm: { x:  0.50, y: 0,    z: 0    },
    },
  },
  {
    id: 'relaxed',
    label: 'Relaxed',
    boneRotations: {
      spine:         { x:  0.05, y: 0,    z: 0    }, // slight slouch
      chest:         { x:  0.03, y: 0,    z: 0    },
      head:          { x:  0.04, y: 0,    z: 0    }, // head slightly down
      leftUpperArm:  { x:  0.06, y: 0,    z: 0.14 }, // arms hanging loosely
      rightUpperArm: { x:  0.06, y: 0,    z:-0.14 },
    },
  },
  {
    id: 'defensive',
    label: 'Defensive',
    boneRotations: {
      spine:         { x:  0.06, y: 0,    z: 0    }, // closed-off lean
      chest:         { x:  0.04, y: 0,    z: 0    },
      head:          { x:  0.04, y:-0.05, z: 0    }, // slight look-away
      leftUpperArm:  { x:  0.22, y:-0.28, z: 0.18 }, // arms hugged in
      rightUpperArm: { x:  0.22, y: 0.28, z:-0.18 },
      leftLowerArm:  { x:  0.45, y: 0,    z: 0    }, // forearms up
      rightLowerArm: { x:  0.45, y: 0,    z: 0    },
    },
  },
  {
    id: 'attentive',
    label: 'Attentive',
    boneRotations: {
      hips:          { x: -0.04, y: 0,    z: 0    }, // subtle lean forward from hips
      spine:         { x: -0.05, y: 0,    z: 0    }, // leaning forward
      head:          { x: -0.04, y: 0,    z: 0    }, // engaged
    },
  },
  {
    id: 'playful',
    label: 'Playful',
    boneRotations: {
      spine:         { x: -0.02, y: 0.03, z: 0    },
      neck:          { x:  0,    y: 0,    z:-0.10 }, // head tilt to left
      head:          { x: -0.02, y: 0,    z:-0.07 },
      leftUpperArm:  { x:  0,    y: 0.14, z: 0.28 }, // left arm ready to gesture
      rightUpperArm: { x:  0,    y:-0.08, z:-0.18 },
    },
  },
  {
    id: 'bored',
    label: 'Bored',
    boneRotations: {
      spine:         { x:  0.12, y: 0,    z: 0    }, // slumped
      chest:         { x:  0.06, y: 0,    z: 0    },
      neck:          { x:  0.06, y: 0,    z: 0    },
      head:          { x:  0.14, y: 0,    z: 0    }, // head drooping
      leftUpperArm:  { x:  0.08, y: 0,    z: 0.10 }, // heavy hanging arms
      rightUpperArm: { x:  0.08, y: 0,    z:-0.10 },
    },
  },
  {
    id: 'empathetic',
    label: 'Empathetic',
    boneRotations: {
      hips:          { x: -0.05, y: 0,    z: 0    }, // lean into it
      spine:         { x: -0.07, y: 0,    z: 0    },
      chest:         { x: -0.04, y: 0,    z: 0    },
      head:          { x: -0.05, y: 0,    z: 0.04 }, // engaged, slight tilt
      leftUpperArm:  { x:  0,    y: 0.14, z: 0.32 }, // open arms
      rightUpperArm: { x:  0,    y:-0.14, z:-0.32 },
      leftLowerArm:  { x:  0,    y: 0,    z:-0.14 },
      rightLowerArm: { x:  0,    y: 0,    z: 0.14 },
    },
  },
];

// ── Index ─────────────────────────────────────────────────────────────────────

const PRESET_INDEX: Map<string, PosePreset> = new Map(
  POSE_PRESETS.map(p => [p.id, p]),
);

/** Return all pose presets. */
export function getAllPosePresets(): PosePreset[] {
  return POSE_PRESETS;
}

/** Return a pose preset by id, or undefined if not found. */
export function getPosePreset(id: string): PosePreset | undefined {
  return PRESET_INDEX.get(id);
}

/** Default emotion → pose mapping used when no [pose:...] tag is present. */
export const EMOTION_TO_POSE: Record<string, Array<{ presetId: string; weight: number }>> = {
  idle:      [{ presetId: 'relaxed', weight: 0.5 }],
  thinking:  [{ presetId: 'thoughtful', weight: 0.75 }],
  talking:   [{ presetId: 'attentive', weight: 0.45 }],
  happy:     [{ presetId: 'excited', weight: 0.45 }, { presetId: 'playful', weight: 0.25 }],
  sad:       [{ presetId: 'shy', weight: 0.5 }, { presetId: 'bored', weight: 0.25 }],
  angry:     [{ presetId: 'defensive', weight: 0.65 }],
  relaxed:   [{ presetId: 'relaxed', weight: 0.75 }],
  surprised: [{ presetId: 'excited', weight: 0.45 }, { presetId: 'attentive', weight: 0.35 }],
  neutral:   [{ presetId: 'attentive', weight: 0.4 }],
};

/** All valid VRM humanoid bone names used by pose presets. */
export const VALID_POSE_BONES: ReadonlySet<string> = new Set([
  'hips', 'spine', 'chest', 'upperChest', 'neck', 'head',
  'leftShoulder', 'rightShoulder',
  'leftUpperArm', 'rightUpperArm',
  'leftLowerArm', 'rightLowerArm',
  'leftHand', 'rightHand',
  'leftUpperLeg', 'rightUpperLeg',
  'leftLowerLeg', 'rightLowerLeg',
  'leftFoot', 'rightFoot',
]);
