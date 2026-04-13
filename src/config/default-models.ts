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
}

export const DEFAULT_MODELS: DefaultModel[] = [
  {
    id: 'annabelle',
    name: 'Annabelle the Sorcerer',
    path: '/models/default/Annabelle the Sorcerer.vrm',
    thumbnail: '/models/default/Annabelle the Sorcerer.png',
    persona: 'witch',
  },
  {
    id: 'm58',
    name: 'M58',
    path: '/models/default/M58.vrm',
    thumbnail: '/models/default/M58.png',
    persona: 'idol',
  },
  {
    id: 'genshin',
    name: 'GENSHIN',
    path: '/models/default/2250278607152806301.vrm',
    thumbnail: '/models/default/GENSHIN.png',
    persona: 'idol',
  },
];

export const DEFAULT_MODEL_ID = 'annabelle';
