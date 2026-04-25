export type ModelGender = 'female' | 'male';

export interface DefaultModel {
  id: string;
  name: string;
  path: string;
  /** Character gender — determines TTS voice. Defaults to 'female'. */
  gender: ModelGender;
  /** Y-axis rotation to face camera. VRM standard faces +Z; models authored
   *  facing -Z need Math.PI to turn toward the camera at +Z. Default: 0. */
  rotationY?: number;
}

/** Edge TTS voice names and prosody mapped by gender. */
export const GENDER_VOICES: Record<ModelGender, {
  edgeVoice: string;
  /** Pitch offset in Hz for Edge TTS (higher = cuter). */
  edgePitch: number;
  /** Rate offset in % for Edge TTS (higher = faster/more energetic). */
  edgeRate: number;
  /** Browser Speech API pitch multiplier (0.1–2.0). */
  browserPitch: number;
  /** Browser Speech API rate multiplier (0.1–10.0). */
  browserRate: number;
}> = {
  female: { edgeVoice: 'en-US-AnaNeural', edgePitch: 50, edgeRate: 15, browserPitch: 1.5, browserRate: 1.15 },
  male:   { edgeVoice: 'en-US-AndrewNeural', edgePitch: -10, edgeRate: 0, browserPitch: 0.8, browserRate: 1.0 },
};

export const DEFAULT_MODELS: DefaultModel[] = [
  {
    id: 'shinra',
    name: 'Shinra',
    path: '/models/default/Shinra.vrm',
    gender: 'female',
  },
  {
    id: 'komori',
    name: 'Komori',
    path: '/models/default/Komori.vrm',
    gender: 'female',
  },
];

export const DEFAULT_MODEL_ID = 'shinra';
