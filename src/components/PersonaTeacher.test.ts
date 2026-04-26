import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia, defineStore } from 'pinia';
import PersonaTeacher from './PersonaTeacher.vue';

// ── Mocks ─────────────────────────────────────────────────────────────────────

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('../stores/persona', () => ({
  usePersonaStore: defineStore('persona', () => ({
    learnedExpressions: [] as unknown[],
    learnedMotions: [] as unknown[],
    startCameraSession: vi.fn(),
    stopCameraSession: vi.fn(),
    cameraSession: { active: false, startedAt: null, chatId: null },
  })),
}));

// Mock useCameraCapture — avoid real MediaPipe / getUserMedia
const mockStart = vi.fn().mockResolvedValue(undefined);
const mockStop = vi.fn();
const mockUpdate = vi.fn().mockReturnValue({
  happy: 0, sad: 0, angry: 0, relaxed: 0, surprised: 0, neutral: 1,
  aa: 0, ih: 0, ou: 0, ee: 0, oh: 0, blink: 0, lookAtX: 0, lookAtY: 0,
});

vi.mock('../composables/useCameraCapture', () => ({
  useCameraCapture: () => ({
    active: { value: false },
    loading: { value: false },
    videoEl: { value: null },
    weights: { value: {
      happy: 0, sad: 0, angry: 0, relaxed: 0, surprised: 0, neutral: 1,
      aa: 0, ih: 0, ou: 0, ee: 0, oh: 0, blink: 0, lookAtX: 0, lookAtY: 0,
    }},
    start: mockStart,
    stop: mockStop,
    update: mockUpdate,
  }),
}));

// ── Tests ─────────────────────────────────────────────────────────────────────

describe('PersonaTeacher', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
  });

  afterEach(() => {
    document.body.querySelectorAll('.pt-root').forEach(el => el.remove());
  });

  it('shows "Start Camera" button when visible and camera off', () => {
    const wrapper = mount(PersonaTeacher, {
      props: { visible: true },
    });
    expect(wrapper.text()).toContain('Expression');
    expect(wrapper.text()).toContain('Motion');
    expect(wrapper.text()).toContain('Start Camera');
  });

  it('is empty when visible is false', () => {
    const wrapper = mount(PersonaTeacher, {
      props: { visible: false },
    });
    expect(wrapper.find('.pt-root').exists()).toBe(false);
  });

  it('shows consent dialog when Start Camera is clicked', async () => {
    const wrapper = mount(PersonaTeacher, {
      props: { visible: true },
    });
    await wrapper.find('.pt-btn--primary').trigger('click');
    expect(wrapper.text()).toContain('camera access');
    expect(wrapper.text()).toContain('Allow This Session');
    expect(wrapper.text()).toContain('Cancel');
  });

  it('hides consent dialog on Cancel', async () => {
    const wrapper = mount(PersonaTeacher, {
      props: { visible: true },
    });
    await wrapper.find('.pt-btn--primary').trigger('click');
    // Click Cancel (ghost button)
    const cancelBtn = wrapper.findAll('.pt-btn--ghost').find(b => b.text() === 'Cancel');
    expect(cancelBtn).toBeTruthy();
    await cancelBtn!.trigger('click');
    // Should be back to start
    expect(wrapper.text()).toContain('Start Camera');
  });

  it('shows saved expressions when they exist', async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    // Pre-populate the mocked store
    const { usePersonaStore } = await import('../stores/persona');
    const store = usePersonaStore();
    store.learnedExpressions = [
      { id: '1', kind: 'expression', name: 'Smirk', trigger: 'smirk', weights: {}, learnedAt: 1 },
    ] as typeof store.learnedExpressions;

    const wrapper = mount(PersonaTeacher, {
      props: { visible: true },
      global: { plugins: [pinia] },
    });
    expect(wrapper.text()).toContain('Saved Expressions (1)');
    expect(wrapper.text()).toContain('Smirk');
    expect(wrapper.text()).toContain('smirk');
  });
});
