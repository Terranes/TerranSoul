import type { AnimationPersona } from '../types';

export interface DefaultModel {
  id: string;
  name: string;
  path: string;
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
    id: 'model1',
    name: 'Model 1',
    path: '/models/default/Model1.vrm',
    rotationY: 0,
    persona: 'cool',
    skipBonePose: true,
  },
  {
    id: 'model2',
    name: 'Model 2',
    path: '/models/default/Model2.vrm',
    rotationY: Math.PI,
    persona: 'cute',
  },
];

export const DEFAULT_MODEL_ID = 'model2';
