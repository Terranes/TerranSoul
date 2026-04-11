export interface DefaultModel {
  id: string;
  name: string;
  path: string;
}

export const DEFAULT_MODELS: DefaultModel[] = [
  {
    id: 'model1',
    name: 'Model 1',
    path: '/models/default/Model1.vrm',
  },
  {
    id: 'model2',
    name: 'Model 2',
    path: '/models/default/Model2.vrm',
  },
];

export const DEFAULT_MODEL_ID = 'model1';
