import { describe, it, expect, beforeEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { useBackgroundStore } from './background';

describe('BackgroundStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('includes all 7 preset backgrounds', () => {
    const store = useBackgroundStore();
    expect(store.presetBackgrounds).toHaveLength(7);
  });

  it('includes new backgrounds (cyberpunk, forest, ocean, nebula)', () => {
    const store = useBackgroundStore();
    const ids = store.presetBackgrounds.map(bg => bg.id);
    expect(ids).toContain('cyberpunk-city');
    expect(ids).toContain('enchanted-forest');
    expect(ids).toContain('deep-ocean');
    expect(ids).toContain('cosmic-nebula');
  });

  it('defaults to the first preset background', () => {
    const store = useBackgroundStore();
    expect(store.currentBackground.id).toBe('studio-soft');
  });

  it('selects a background by id', () => {
    const store = useBackgroundStore();
    store.selectBackground('cosmic-nebula');
    expect(store.selectedBackgroundId).toBe('cosmic-nebula');
    expect(store.currentBackground.id).toBe('cosmic-nebula');
  });

  it('falls back to first preset when invalid id is selected', () => {
    const store = useBackgroundStore();
    store.selectBackground('nonexistent');
    store.ensureValidSelection();
    expect(store.currentBackground.id).toBe('studio-soft');
  });

  it('allBackgrounds includes presets and local backgrounds', () => {
    const store = useBackgroundStore();
    expect(store.allBackgrounds.length).toBe(7);
  });

  it('imports a local background', async () => {
    const store = useBackgroundStore();
    const file = new File([''], 'test.png', { type: 'image/png' });
    const ok = await store.importLocalBackground(file);
    expect(ok).toBe(true);
    expect(store.allBackgrounds.length).toBe(8);
    expect(store.currentBackground.kind).toBe('local');
  });

  it('rejects non-image files', async () => {
    const store = useBackgroundStore();
    const file = new File([''], 'test.txt', { type: 'text/plain' });
    const ok = await store.importLocalBackground(file);
    expect(ok).toBe(false);
    expect(store.importError).toBeTruthy();
  });

  it('each preset has valid url path', () => {
    const store = useBackgroundStore();
    for (const bg of store.presetBackgrounds) {
      expect(bg.url).toMatch(/^\/backgrounds\/.*\.svg$/);
      expect(bg.kind).toBe('preset');
    }
  });
});
