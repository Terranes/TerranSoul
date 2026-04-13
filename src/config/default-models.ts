import type { AnimationPersona } from '../types';

export interface DefaultModel {
  id: string;
  name: string;
  path: string;
  /** Optional thumbnail image path shown in the model selector dropdown. */
  thumbnail?: string;
  /** Y-axis rotation to face camera. VRM standard faces +Z; models authored
   *  facing -Z need Math.PI to turn toward the camera at +Z. Default: 0. */
  rotationY?: number;
  /** Animation personality — affects movement style and expressions. */
  persona?: AnimationPersona;
  /** Skip normalized bone pose manipulation (for models with non-standard skeletons). */
  skipBonePose?: boolean;
}

export const DEFAULT_MODELS: DefaultModel[] = [
  {
    id: 'annabelle',
    name: 'Annabelle the Sorcerer',
    path: '/models/default/Annabelle the Sorcerer.vrm',
    thumbnail: '/models/default/Annabelle the Sorcerer.png',
    rotationY: Math.PI,
    persona: 'cool',
  },
  {
    id: 'm58',
    name: 'M58',
    path: '/models/default/M58.vrm',
    thumbnail: '/models/default/M58.png',
    rotationY: Math.PI,
    persona: 'cool',
  },
  {
    id: 'miyoura-toshie',
    name: 'Miyoura Toshie',
    path: '/models/default/Miyoura Toshie.vrm',
    thumbnail: '/models/default/Miyoura Toshie.png',
    rotationY: Math.PI,
    persona: 'cute',
  },
  {
    id: 'nogami-juto',
    name: 'Nogami Juto',
    path: '/models/default/Nogami Juto.vrm',
    thumbnail: '/models/default/Nogami Juto.png',
    rotationY: 0,
    persona: 'cool',
  },
];

export const DEFAULT_MODEL_ID = 'annabelle';
