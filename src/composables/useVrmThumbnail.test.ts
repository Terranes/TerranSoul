import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useVrmThumbnail, preGenerateUserThumbnail, resetFailedThumbnails } from './useVrmThumbnail';

// Mock @tauri-apps/api/core
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Mock three — WebGL not available in test environment
vi.mock('three', async () => {
  const actual = await vi.importActual<typeof import('three')>('three');
  class MockWebGLRenderer {
    constructor() {
      throw new Error('WebGL not available in test');
    }
  }
  return {
    ...actual,
    WebGLRenderer: MockWebGLRenderer,
  };
});

describe('useVrmThumbnail', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resetFailedThumbnails();
  });

  it('returns null thumbnailUrl initially', () => {
    const { thumbnailUrl, isGenerating } = useVrmThumbnail('test-id', {
      modelPath: '/models/default/Shinra.vrm',
    });
    expect(thumbnailUrl.value).toBeNull();
    expect(isGenerating.value).toBe(false);
  });

  it('generate gracefully handles WebGL unavailability', async () => {
    const { thumbnailUrl, generate } = useVrmThumbnail('test-id', {
      modelPath: '/models/default/Shinra.vrm',
    });
    // Should not throw — error is caught internally
    await generate();
    // Thumbnail stays null since rendering failed
    expect(thumbnailUrl.value).toBeNull();
  });

  it('does not throw on concurrent generate calls for same key', async () => {
    const { generate } = useVrmThumbnail('dup-key', {
      modelPath: '/models/default/Shinra.vrm',
    });
    // Both calls should complete without throwing (errors caught internally)
    await Promise.all([generate(), generate()]);
  });

  it('skips generate if thumbnailUrl is already set', async () => {
    const { thumbnailUrl, generate } = useVrmThumbnail('skip-key', {
      modelPath: '/models/default/Shinra.vrm',
    });
    // Simulate cached value
    thumbnailUrl.value = 'data:image/png;base64,cached';
    await generate();
    // Should still be the cached value (not overwritten)
    expect(thumbnailUrl.value).toBe('data:image/png;base64,cached');
  });

  it('requires either modelPath or userModelId', () => {
    const { thumbnailUrl } = useVrmThumbnail('no-source', {});
    expect(thumbnailUrl.value).toBeNull();
  });
});

describe('preGenerateUserThumbnail', () => {
  it('does not throw when WebGL is unavailable', async () => {
    await expect(preGenerateUserThumbnail('u-1')).resolves.not.toThrow();
  });
});
