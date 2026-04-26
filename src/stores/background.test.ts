import { describe, it, expect, beforeEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { useBackgroundStore } from './background';

describe('BackgroundStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('has exactly one built-in option: auto', () => {
    const store = useBackgroundStore();
    expect(store.allBackgrounds).toHaveLength(1);
    expect(store.allBackgrounds[0].id).toBe('auto');
    expect(store.allBackgrounds[0].kind).toBe('auto');
  });

  it('defaults to auto background', () => {
    const store = useBackgroundStore();
    expect(store.currentBackground.id).toBe('auto');
    expect(store.currentBackground.url).toBe('');
  });

  it('selects auto by id', () => {
    const store = useBackgroundStore();
    store.selectBackground('auto');
    expect(store.selectedBackgroundId).toBe('auto');
    expect(store.currentBackground.id).toBe('auto');
  });

  it('falls back to auto when invalid id is selected', () => {
    const store = useBackgroundStore();
    store.selectBackground('nonexistent');
    store.ensureValidSelection();
    expect(store.currentBackground.id).toBe('auto');
  });

  it('allBackgrounds includes auto and local backgrounds', () => {
    const store = useBackgroundStore();
    expect(store.allBackgrounds.length).toBe(1);
  });

  it('imports a local background', async () => {
    const store = useBackgroundStore();
    const file = new File([''], 'test.png', { type: 'image/png' });
    const ok = await store.importLocalBackground(file);
    expect(ok).toBe(true);
    expect(store.allBackgrounds.length).toBe(2);
    expect(store.currentBackground.kind).toBe('local');
  });

  it('rejects non-image files', async () => {
    const store = useBackgroundStore();
    const file = new File([''], 'test.txt', { type: 'text/plain' });
    const ok = await store.importLocalBackground(file);
    expect(ok).toBe(false);
    expect(store.importError).toBeTruthy();
  });
});

