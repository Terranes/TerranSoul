import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { useLanShareStore } from './lan-share';

// Mock Tauri invoke
const invokeMock = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

describe('lan-share store', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invokeMock.mockReset();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe('initial state', () => {
    it('starts idle with no connections', () => {
      const store = useLanShareStore();
      expect(store.hosting).toBe(false);
      expect(store.discovering).toBe(false);
      expect(store.connections).toEqual([]);
      expect(store.discovered).toEqual([]);
      expect(store.mode).toBe('idle');
      expect(store.connectedCount).toBe(0);
    });
  });

  describe('startHosting', () => {
    it('calls lan_share_start and updates state on success', async () => {
      invokeMock.mockResolvedValue({
        hosting: true,
        brain_name: 'HR Rules',
        port: 7421,
        token: 'abc-token-123',
        auth_mode: 'token_required',
        token_required: true,
        connected_brains: 0,
        connections: [],
      });

      const store = useLanShareStore();
      await store.startHosting('HR Rules');

      expect(invokeMock).toHaveBeenCalledWith('lan_share_start', { brainName: 'HR Rules' });
      expect(store.hosting).toBe(true);
      expect(store.hostBrainName).toBe('HR Rules');
      expect(store.hostPort).toBe(7421);
      expect(store.hostToken).toBe('abc-token-123');
      expect(store.mode).toBe('hosting');
    });

    it('sets error on failure', async () => {
      invokeMock.mockRejectedValue('LAN mode not enabled');

      const store = useLanShareStore();
      await expect(store.startHosting('Test')).rejects.toThrow();
      expect(store.error).toBe('LAN mode not enabled');
      expect(store.hosting).toBe(false);
    });
  });

  describe('stopHosting', () => {
    it('calls lan_share_stop and resets state', async () => {
      invokeMock.mockResolvedValue(undefined);

      const store = useLanShareStore();
      store.hosting = true;
      store.hostBrainName = 'Test';
      store.hostPort = 7421;
      store.hostToken = 'token';

      await store.stopHosting();

      expect(invokeMock).toHaveBeenCalledWith('lan_share_stop');
      expect(store.hosting).toBe(false);
      expect(store.hostBrainName).toBe('');
      expect(store.hostPort).toBeNull();
      expect(store.hostToken).toBeNull();
    });
  });

  describe('startDiscovery', () => {
    it('calls lan_share_discover and populates discovered list', async () => {
      const brains = [
        {
          brain_name: 'HR Rules',
          host: '192.168.1.100',
          port: 7421,
          provider: 'free:pollinations',
          memory_count: 150,
          read_only: true,
          hostname: 'HR-PC',
          token_required: true,
        },
      ];
      invokeMock.mockResolvedValue(brains);

      const store = useLanShareStore();
      const result = await store.startDiscovery();

      expect(invokeMock).toHaveBeenCalledWith('lan_share_discover');
      expect(result).toEqual(brains);
      expect(store.discovered).toEqual(brains);
      expect(store.discovering).toBe(true);
    });

    it('sets error and returns empty on failure', async () => {
      invokeMock.mockRejectedValue('Network error');

      const store = useLanShareStore();
      const result = await store.startDiscovery();

      expect(result).toEqual([]);
      expect(store.error).toBe('Network error');
    });
  });

  describe('connect', () => {
    it('calls lan_share_connect and adds to connections', async () => {
      const conn = {
        id: 'conn-1',
        host: '192.168.1.100',
        port: 7421,
        token: 'token-123',
        token_required: true,
        brain_name: 'HR Rules',
        connected: true,
      };
      invokeMock.mockResolvedValue(conn);

      const store = useLanShareStore();
      const result = await store.connect('192.168.1.100', 7421, 'token-123', true, 'HR Rules');

      expect(invokeMock).toHaveBeenCalledWith('lan_share_connect', {
        host: '192.168.1.100',
        port: 7421,
        token: 'token-123',
        tokenRequired: true,
        brainName: 'HR Rules',
      });
      expect(result).toEqual(conn);
      expect(store.connections).toHaveLength(1);
      expect(store.connections[0].id).toBe('conn-1');
      expect(store.mode).toBe('connected');
      expect(store.connectedCount).toBe(1);
    });

    it('sets error and returns null on failure', async () => {
      invokeMock.mockRejectedValue('Connection refused');

      const store = useLanShareStore();
      const result = await store.connect('192.168.1.100', 7421, 'bad-token', true);

      expect(result).toBeNull();
      expect(store.error).toBe('Connection refused');
      expect(store.connections).toHaveLength(0);
    });
  });

  describe('disconnect', () => {
    it('removes connection from state', async () => {
      invokeMock.mockResolvedValue(undefined);

      const store = useLanShareStore();
      store.connections = [
        { id: 'conn-1', host: '192.168.1.100', port: 7421, token: 't', token_required: true, brain_name: 'HR', connected: true },
        { id: 'conn-2', host: '192.168.1.101', port: 7421, token: 't', token_required: true, brain_name: 'Legal', connected: true },
      ];

      await store.disconnect('conn-1');

      expect(invokeMock).toHaveBeenCalledWith('lan_share_disconnect', { id: 'conn-1' });
      expect(store.connections).toHaveLength(1);
      expect(store.connections[0].id).toBe('conn-2');
    });
  });

  describe('searchRemote', () => {
    it('calls lan_share_search and returns results', async () => {
      const results = [
        { id: 1, content: 'Company policy', tags: 'hr,policy', importance: 7, score: 0.9, source_url: null, tier: 'long' },
      ];
      invokeMock.mockResolvedValue(results);

      const store = useLanShareStore();
      const searchResults = await store.searchRemote('conn-1', 'vacation policy');

      expect(invokeMock).toHaveBeenCalledWith('lan_share_search', {
        connectionId: 'conn-1',
        query: 'vacation policy',
        limit: null,
      });
      expect(searchResults).toEqual(results);
    });
  });

  describe('searchAll', () => {
    it('calls lan_share_search_all and returns tagged results', async () => {
      const results = [
        {
          connection_id: 'conn-1',
          brain_name: 'HR Rules',
          result: { id: 1, content: 'Vacation is 20 days', tags: 'hr', importance: 5, score: 0.85, source_url: null, tier: 'long' },
        },
        {
          connection_id: 'conn-2',
          brain_name: 'Legal',
          result: { id: 5, content: 'Labor code section 3', tags: 'law', importance: 8, score: 0.75, source_url: null, tier: 'long' },
        },
      ];
      invokeMock.mockResolvedValue(results);

      const store = useLanShareStore();
      const taggedResults = await store.searchAll('vacation policy', 10);

      expect(invokeMock).toHaveBeenCalledWith('lan_share_search_all', {
        query: 'vacation policy',
        limit: 10,
      });
      expect(taggedResults).toHaveLength(2);
      expect(taggedResults[0].brain_name).toBe('HR Rules');
    });
  });

  describe('refreshStatus', () => {
    it('syncs state from backend', async () => {
      invokeMock.mockResolvedValue({
        hosting: true,
        brain_name: null,
        port: 7421,
        token: 'fresh-token',
        auth_mode: 'token_required',
        token_required: true,
        connected_brains: 1,
        connections: [
          { id: 'c1', host: '10.0.0.5', port: 7421, token: 'x', token_required: true, brain_name: 'Shared', connected: true },
        ],
      });

      const store = useLanShareStore();
      await store.refreshStatus();

      expect(store.hosting).toBe(true);
      expect(store.hostPort).toBe(7421);
      expect(store.hostToken).toBe('fresh-token');
      expect(store.connections).toHaveLength(1);
    });
  });

  describe('mode computed', () => {
    it('returns hosting when hosting', () => {
      const store = useLanShareStore();
      store.hosting = true;
      expect(store.mode).toBe('hosting');
    });

    it('returns connected when has connections', () => {
      const store = useLanShareStore();
      store.connections = [
        { id: 'c1', host: 'x', port: 1, token: 't', token_required: true, brain_name: 'B', connected: true },
      ];
      expect(store.mode).toBe('connected');
    });

    it('returns discovering when scanning', () => {
      const store = useLanShareStore();
      store.discovering = true;
      expect(store.mode).toBe('discovering');
    });
  });
});
