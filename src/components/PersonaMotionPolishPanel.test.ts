import { describe, it, expect, vi, beforeEach } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import PersonaMotionPolishPanel from './PersonaMotionPolishPanel.vue';
import type { LearnedMotion, MotionPolishPreview } from '../stores/persona-types';

const mockStore = vi.hoisted(() => ({
  learnedMotions: [] as unknown[],
  polishLearnedMotion: vi.fn(),
  saveLearnedMotion: vi.fn(),
  requestMotionPreview: vi.fn(),
}));

vi.mock('../stores/persona', () => ({
  usePersonaStore: () => mockStore,
}));

function makeMotion(overrides: Partial<LearnedMotion> = {}): LearnedMotion {
  return {
    id: 'motion-source',
    kind: 'motion',
    name: 'Camera Wave',
    trigger: 'wave',
    fps: 30,
    duration_s: 1,
    learnedAt: 1000,
    provenance: 'camera',
    frames: [
      { t: 0, bones: { head: [0, 0, 0] } },
      { t: 0.5, bones: { head: [0.4, 0, 0] } },
      { t: 1, bones: { head: [0, 0, 0] } },
    ],
    ...overrides,
  };
}

function makePreview(): MotionPolishPreview {
  const candidate = makeMotion({
    id: 'motion-polished-1',
    name: 'Camera Wave (polished)',
    trigger: 'wave-polished',
    learnedAt: 2000,
    polish: {
      sourceMotionId: 'motion-source',
      backend: 'gaussian-v1',
      createdAt: 2000,
      meanDisplacement: 0.05,
      maxDisplacement: 0.2,
      acceptedByUser: false,
      preset: 'medium',
      sigma: 2,
      radius: null,
      pinEndpoints: true,
    },
  });
  return {
    originalMotionId: 'motion-source',
    candidateId: 'motion-polished-1',
    candidateMotion: candidate,
    meanDisplacementByBone: { head: 0.05, spine: 0.02 },
    maxDisplacement: 0.2,
    warnings: [],
    appliedConfig: {
      preset: 'medium',
      sigma: 2,
      radius: null,
      pinEndpoints: true,
    },
  };
}

describe('PersonaMotionPolishPanel', () => {
  beforeEach(() => {
    mockStore.learnedMotions = [makeMotion()];
    mockStore.polishLearnedMotion.mockReset();
    mockStore.saveLearnedMotion.mockReset();
    mockStore.requestMotionPreview.mockReset();
  });

  it('builds a medium smoothing preview without saving', async () => {
    const preview = makePreview();
    mockStore.polishLearnedMotion.mockResolvedValueOnce(preview);

    const wrapper = mount(PersonaMotionPolishPanel);
    await wrapper.find('[data-testid="pmp-build"]').trigger('click');
    await flushPromises();

    expect(mockStore.polishLearnedMotion).toHaveBeenCalledWith('motion-source', {
      preset: 'medium',
      pinEndpoints: true,
    });
    expect(mockStore.saveLearnedMotion).not.toHaveBeenCalled();
    expect(mockStore.requestMotionPreview).toHaveBeenCalledWith(preview.candidateMotion);
    expect(wrapper.find('[data-testid="pmp-preview"]').text()).toContain('Camera Wave (polished)');
    expect(wrapper.find('[data-testid="pmp-mean-displacement"]').text()).toContain('0.050 rad');
  });

  it('toggles playback between original and polished candidates', async () => {
    const preview = makePreview();
    mockStore.polishLearnedMotion.mockResolvedValueOnce(preview);

    const wrapper = mount(PersonaMotionPolishPanel);
    await wrapper.find('[data-testid="pmp-build"]').trigger('click');
    await flushPromises();
    await wrapper.find('[data-testid="pmp-play-original"]').trigger('click');
    await wrapper.find('[data-testid="pmp-play-polished"]').trigger('click');

    expect(mockStore.requestMotionPreview).toHaveBeenNthCalledWith(2, mockStore.learnedMotions[0]);
    expect(mockStore.requestMotionPreview).toHaveBeenNthCalledWith(3, preview.candidateMotion);
  });

  it('saves an accepted polished candidate as a new clip', async () => {
    const preview = makePreview();
    mockStore.polishLearnedMotion.mockResolvedValueOnce(preview);
    mockStore.saveLearnedMotion.mockResolvedValueOnce(undefined);

    const wrapper = mount(PersonaMotionPolishPanel);
    await wrapper.find('[data-testid="pmp-build"]').trigger('click');
    await flushPromises();
    await wrapper.find('[data-testid="pmp-save"]').trigger('click');
    await flushPromises();

    expect(mockStore.saveLearnedMotion).toHaveBeenCalledWith(expect.objectContaining({
      id: 'motion-polished-1',
      polish: expect.objectContaining({ acceptedByUser: true }),
    }));
    expect(mockStore.saveLearnedMotion.mock.calls[0][0].id).not.toBe('motion-source');
  });
});

	