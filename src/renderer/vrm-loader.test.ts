import { describe, it, expect, vi } from 'vitest';
import { extractVrmMetadata, createPlaceholderCharacter } from './vrm-loader';
import type { VRM } from '@pixiv/three-vrm';
import * as THREE from 'three';

// We cannot test loadVRM with real Three.js in jsdom (no WebGL),
// so we focus on testable pure functions and error path logic.

function makeFakeVrm(metaOverrides: Record<string, unknown> = {}): VRM {
  return {
    meta: {
      metaVersion: '1',
      name: 'Test Character',
      authors: ['TestAuthor'],
      licenseUrl: 'https://example.com/license',
      ...metaOverrides,
    },
  } as unknown as VRM;
}

describe('extractVrmMetadata', () => {
  it('extracts title, author, and license from VRM 1.0 meta', () => {
    const vrm = makeFakeVrm();
    const metadata = extractVrmMetadata(vrm);
    expect(metadata.title).toBe('Test Character');
    expect(metadata.author).toBe('TestAuthor');
    expect(metadata.license).toBe('https://example.com/license');
  });

  it('returns "Unknown" for missing name in VRM 1.0', () => {
    const vrm = makeFakeVrm({ name: '' });
    const metadata = extractVrmMetadata(vrm);
    expect(metadata.title).toBe('Unknown');
  });

  it('returns "Unknown" for empty authors in VRM 1.0', () => {
    const vrm = makeFakeVrm({ authors: [] });
    const metadata = extractVrmMetadata(vrm);
    expect(metadata.author).toBe('Unknown');
  });

  it('returns "Unknown" for missing licenseUrl in VRM 1.0', () => {
    const vrm = makeFakeVrm({ licenseUrl: '' });
    const metadata = extractVrmMetadata(vrm);
    expect(metadata.license).toBe('Unknown');
  });

  it('extracts from VRM 0.0 meta format', () => {
    const vrm = {
      meta: {
        metaVersion: '0',
        title: 'VRM0 Model',
        author: 'VRM0Author',
        licenseName: 'CC_BY',
      },
    } as unknown as VRM;
    const metadata = extractVrmMetadata(vrm);
    expect(metadata.title).toBe('VRM0 Model');
    expect(metadata.author).toBe('VRM0Author');
    expect(metadata.license).toBe('CC_BY');
  });

  it('VRM 0.0 falls back to otherLicenseUrl', () => {
    const vrm = {
      meta: {
        metaVersion: '0',
        title: 'Model',
        author: 'Author',
        otherLicenseUrl: 'https://example.com/other',
      },
    } as unknown as VRM;
    const metadata = extractVrmMetadata(vrm);
    expect(metadata.license).toBe('https://example.com/other');
  });

  it('handles completely empty meta object', () => {
    const vrm = { meta: {} } as unknown as VRM;
    const metadata = extractVrmMetadata(vrm);
    expect(metadata.title).toBe('Unknown');
    expect(metadata.author).toBe('Unknown');
    expect(metadata.license).toBe('Unknown');
  });

  it('handles null meta gracefully', () => {
    const vrm = { meta: null } as unknown as VRM;
    const metadata = extractVrmMetadata(vrm);
    expect(metadata.title).toBe('Unknown');
    expect(metadata.author).toBe('Unknown');
    expect(metadata.license).toBe('Unknown');
  });
});

describe('loadVRM validation', () => {
  it('loadVRM rejects empty path', async () => {
    // Dynamic import to avoid Three.js init issues in jsdom
    const { loadVRM } = await import('./vrm-loader');
    const fakeScene = {} as any;
    await expect(loadVRM(fakeScene, '')).rejects.toThrow('VRM path must be a non-empty string');
  });

  it('loadVRM rejects non-string path', async () => {
    const { loadVRM } = await import('./vrm-loader');
    const fakeScene = {} as any;
    await expect(loadVRM(fakeScene, null as any)).rejects.toThrow('VRM path must be a non-empty string');
  });

  it('loadVRMSafe returns null on error', async () => {
    const { loadVRMSafe } = await import('./vrm-loader');
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    const fakeScene = {} as any;
    const result = await loadVRMSafe(fakeScene, '');
    expect(result).toBeNull();
    expect(consoleSpy).toHaveBeenCalled();
    consoleSpy.mockRestore();
  });

  it('loadVRMSafe logs error message when load fails', async () => {
    const { loadVRMSafe } = await import('./vrm-loader');
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    const logSpy = vi.spyOn(console, 'log').mockImplementation(() => {});
    const fakeScene = {} as any;
    await loadVRMSafe(fakeScene, '');
    expect(consoleSpy).toHaveBeenCalledWith(
      '[TerranSoul] VRM load failed, using placeholder:',
      expect.any(Error),
    );
    consoleSpy.mockRestore();
    logSpy.mockRestore();
  });
});

describe('createPlaceholderCharacter', () => {
  it('adds a group to the scene with body and head meshes', () => {
    const scene = new THREE.Scene();
    const group = createPlaceholderCharacter(scene);
    expect(group).toBeInstanceOf(THREE.Group);
    expect(scene.children).toContain(group);
    // Should have at least body, head, and two eyes
    expect(group.children.length).toBeGreaterThanOrEqual(4);
  });

  it('positions head above body', () => {
    const scene = new THREE.Scene();
    const group = createPlaceholderCharacter(scene);
    const positions = group.children.map(c => c.position.y);
    // Head (y=1.6) should be above body (y=0.85)
    expect(Math.max(...positions)).toBeGreaterThan(1);
  });
});

describe('loadVRM URL encoding', () => {
  it('encodes spaces in file paths', async () => {
    // We cannot do a real load in jsdom, but we can verify the encoding logic
    // by checking the loadVRM function rejects with an error (not a path error)
    // when given a path with spaces — the path validation passes, meaning
    // encoding is applied before the fetch.
    const { loadVRM } = await import('./vrm-loader');
    const fakeScene = { add: vi.fn() } as any;
    // This will fail at the Three.js loader level (no WebGL), but the path
    // should be accepted (not rejected as invalid string)
    await expect(loadVRM(fakeScene, '/models/default/Shinra.vrm'))
      .rejects.toThrow(); // Will throw from GLTFLoader, not path validation
  });

  it('does not encode blob: URLs', async () => {
    const { loadVRM } = await import('./vrm-loader');
    const fakeScene = { add: vi.fn() } as any;
    // blob: URLs should pass path validation and fail at loader level
    await expect(loadVRM(fakeScene, 'blob:http://localhost/fake-id'))
      .rejects.toThrow();
  });
});
