import { describe, it, expect } from 'vitest';
import { DEFAULT_MODELS, DEFAULT_MODEL_ID, GENDER_VOICES, type ModelGender } from './default-models';

describe('default-models — gender configuration', () => {
  it('every model has a gender field', () => {
    for (const model of DEFAULT_MODELS) {
      expect(['female', 'male']).toContain(model.gender);
    }
  });

  it('annabelle is female', () => {
    const model = DEFAULT_MODELS.find(m => m.id === 'annabelle');
    expect(model?.gender).toBe('female');
  });

  it('m58 is male', () => {
    const model = DEFAULT_MODELS.find(m => m.id === 'm58');
    expect(model?.gender).toBe('male');
  });

  it('genshin is female', () => {
    const model = DEFAULT_MODELS.find(m => m.id === 'genshin');
    expect(model?.gender).toBe('female');
  });

  it('GENDER_VOICES has entries for both genders', () => {
    const genders: ModelGender[] = ['female', 'male'];
    for (const g of genders) {
      expect(GENDER_VOICES[g]).toBeDefined();
      expect(GENDER_VOICES[g].edgeVoice).toBeTruthy();
      expect(typeof GENDER_VOICES[g].edgePitch).toBe('number');
      expect(typeof GENDER_VOICES[g].edgeRate).toBe('number');
      expect(typeof GENDER_VOICES[g].browserPitch).toBe('number');
      expect(typeof GENDER_VOICES[g].browserRate).toBe('number');
    }
  });

  it('female voice is en-US-AnaNeural with cute anime prosody', () => {
    expect(GENDER_VOICES.female.edgeVoice).toBe('en-US-AnaNeural');
    expect(GENDER_VOICES.female.edgePitch).toBeGreaterThan(0);
    expect(GENDER_VOICES.female.edgeRate).toBeGreaterThan(0);
    expect(GENDER_VOICES.female.browserPitch).toBeGreaterThan(1.0);
    expect(GENDER_VOICES.female.browserRate).toBeGreaterThan(1.0);
  });

  it('male voice is en-US-AndrewNeural with low pitch', () => {
    expect(GENDER_VOICES.male.edgeVoice).toBe('en-US-AndrewNeural');
    expect(GENDER_VOICES.male.edgePitch).toBeLessThan(0);
    expect(GENDER_VOICES.male.browserPitch).toBeLessThan(1.0);
  });

  it('DEFAULT_MODEL_ID refers to an existing model', () => {
    const model = DEFAULT_MODELS.find(m => m.id === DEFAULT_MODEL_ID);
    expect(model).toBeDefined();
  });
});
