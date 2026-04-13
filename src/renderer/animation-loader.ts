import * as THREE from 'three';
import type { VRM, VRMHumanBoneName } from '@pixiv/three-vrm';
import type { AnimationPersona, CharacterState } from '../types';

// ── JSON animation data types ────────────────────────────────────────

/** A single bone track: Euler rotations and/or positions at specific times. */
interface BoneTrack {
  bone: string;
  times: number[];
  /** [x, y, z] Euler rotations (radians) per keyframe. */
  rotations?: [number, number, number][];
  /** [x, y, z] local-space positions per keyframe (e.g. hips vertical bob). */
  positions?: [number, number, number][];
}

interface ClipData {
  duration: number;
  tracks: BoneTrack[];
}

/** Per-persona JSON: one clip per CharacterState. */
interface PersonaAnimationData {
  idle: ClipData;
  thinking: ClipData;
  talking: ClipData;
  happy: ClipData;
  sad: ClipData;
}

// ── Static imports (bundled by Vite — no runtime fetch) ──────────────

import witchData from './animations/witch.json';
import idolData from './animations/idol.json';

const DATA_MAP: Record<AnimationPersona, PersonaAnimationData> = {
  witch: witchData as PersonaAnimationData,
  idol: idolData as PersonaAnimationData,
};

// ── Euler → Quaternion helper ────────────────────────────────────────

const _euler = new THREE.Euler();
const _quat = new THREE.Quaternion();

function eulerToQuatArray(rotations: [number, number, number][]): number[] {
  const out: number[] = [];
  for (const [x, y, z] of rotations) {
    _euler.set(x, y, z, 'XYZ');
    _quat.setFromEuler(_euler);
    out.push(_quat.x, _quat.y, _quat.z, _quat.w);
  }
  return out;
}

// ── Public API ───────────────────────────────────────────────────────

export type PersonaClips = Record<CharacterState, THREE.AnimationClip>;

/**
 * Build a set of AnimationClips for every CharacterState of a given persona.
 *
 * The clips target VRM *normalized* bone nodes so the mixer drives the
 * same skeleton layer that `vrm.humanoid.update()` copies to raw bones.
 *
 * @param vrm  – A loaded VRM whose humanoid is available.
 * @param persona – Which animation persona to build clips for.
 */
export function buildPersonaClips(vrm: VRM, persona: AnimationPersona): PersonaClips {
  const data = DATA_MAP[persona];
  const states: CharacterState[] = ['idle', 'thinking', 'talking', 'happy', 'sad'];
  const clips: Partial<PersonaClips> = {};

  for (const state of states) {
    clips[state] = buildClip(vrm, `${persona}-${state}`, data[state]);
  }

  return clips as PersonaClips;
}

function buildClip(vrm: VRM, name: string, clipData: ClipData): THREE.AnimationClip {
  const tracks: THREE.KeyframeTrack[] = [];

  for (const t of clipData.tracks) {
    const boneName = t.bone as VRMHumanBoneName;
    const node = vrm.humanoid?.getNormalizedBoneNode(boneName);
    if (!node) continue;

    // Rotation track (Euler → Quaternion)
    if (t.rotations && t.rotations.length > 0) {
      const quatValues = eulerToQuatArray(t.rotations);
      tracks.push(
        new THREE.QuaternionKeyframeTrack(
          `${node.name}.quaternion`,
          t.times,
          quatValues,
        ),
      );
    }

    // Position track (for hips vertical bob, etc.)
    if (t.positions && t.positions.length > 0) {
      const posValues: number[] = [];
      for (const [x, y, z] of t.positions) {
        posValues.push(x, y, z);
      }
      tracks.push(
        new THREE.VectorKeyframeTrack(
          `${node.name}.position`,
          t.times,
          posValues,
        ),
      );
    }
  }

  return new THREE.AnimationClip(name, clipData.duration, tracks);
}
