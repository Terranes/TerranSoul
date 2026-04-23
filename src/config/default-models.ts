export type ModelGender = 'female' | 'male';

export interface DefaultModel {
  id: string;
  name: string;
  path: string;
  /** Character gender — determines TTS voice. Defaults to 'female'. */
  gender: ModelGender;
  /** Optional thumbnail image path shown in the model selector dropdown. */
  thumbnail?: string;
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
    id: 'annabelle',
    name: 'Annabelle the Sorcerer',
    path: '/models/default/Annabelle the Sorcerer.vrm',
    thumbnail: '/models/default/Annabelle the Sorcerer.png',
    gender: 'female',
  },
  {
    id: 'm58',
    name: 'M58',
    path: '/models/default/M58.vrm',
    thumbnail: '/models/default/M58.png',
    gender: 'male',
  },
];

export const DEFAULT_MODEL_ID = 'annabelle';
