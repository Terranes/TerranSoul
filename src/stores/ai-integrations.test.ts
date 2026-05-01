/**
 * Tests for `useAiIntegrationsStore` (Chunk 15.4).
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';

import { useAiIntegrationsStore } from './ai-integrations';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (cmd: string, args?: unknown) => mockInvoke(cmd, args),
}));

describe('useAiIntegrationsStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it('refreshStatus stores the snapshot', async () => {
    mockInvoke.mockResolvedValueOnce({
      running: true,
      port: 7421,
      token: 'abcdefghijklmnop',
      is_dev: false,
    });
    const s = useAiIntegrationsStore();
    await s.refreshStatus();
    expect(s.isRunning).toBe(true);
    expect(s.port).toBe(7421);
    expect(s.tokenPreview).toBe('abcdefgh…');
  });

  it('tokenPreview shortens long tokens but returns short tokens whole', async () => {
    mockInvoke.mockResolvedValueOnce({
      running: true,
      port: 7421,
      token: 'short',
      is_dev: false,
    });
    const s = useAiIntegrationsStore();
    await s.refreshStatus();
    expect(s.tokenPreview).toBe('short');
  });

  it('refreshStatus surfaces errors', async () => {
    mockInvoke.mockRejectedValueOnce('boom');
    const s = useAiIntegrationsStore();
    await s.refreshStatus();
    expect(s.error).toBe('boom');
    expect(s.isRunning).toBe(false);
  });

  it('startServer calls mcp_server_start and stores the result', async () => {
    mockInvoke.mockResolvedValueOnce({
      running: true,
      port: 7422,
      token: 'tok',
      is_dev: true,
    });
    const s = useAiIntegrationsStore();
    await s.startServer();
    expect(mockInvoke).toHaveBeenCalledWith('mcp_server_start', undefined);
    expect(s.isRunning).toBe(true);
    expect(s.port).toBe(7422);
  });

  it('stopServer calls stop then refreshes status', async () => {
    mockInvoke
      .mockResolvedValueOnce(undefined) // mcp_server_stop
      .mockResolvedValueOnce({ running: false, port: null, token: null, is_dev: false });
    const s = useAiIntegrationsStore();
    await s.stopServer();
    expect(mockInvoke).toHaveBeenNthCalledWith(1, 'mcp_server_stop', undefined);
    expect(mockInvoke).toHaveBeenNthCalledWith(2, 'mcp_server_status', undefined);
    expect(s.isRunning).toBe(false);
  });

  it('regenerateToken updates the cached token in state', async () => {
    // Seed status with old token first.
    mockInvoke.mockResolvedValueOnce({
      running: true,
      port: 7421,
      token: 'old-token-12345678',
      is_dev: false,
    });
    const s = useAiIntegrationsStore();
    await s.refreshStatus();
    mockInvoke.mockResolvedValueOnce('new-token-87654321');
    await s.regenerateToken();
    expect(s.serverStatus?.token).toBe('new-token-87654321');
    expect(s.tokenPreview).toBe('new-toke…');
  });

  it('refreshClients calls list_mcp_clients with workspace root', async () => {
    mockInvoke.mockResolvedValueOnce([
      { client: 'VS Code / Copilot', configured: true, config_path: '/x' },
      { client: 'Claude Desktop', configured: false, config_path: '/y' },
    ]);
    const s = useAiIntegrationsStore();
    await s.refreshClients('/proj');
    expect(mockInvoke).toHaveBeenCalledWith('list_mcp_clients', {
      workspaceRoot: '/proj',
    });
    expect(s.clientStatuses.length).toBe(2);
  });

  it('setupClient picks the stdio command by default', async () => {
    mockInvoke
      .mockResolvedValueOnce({ message: 'ok' }) // setup_vscode_mcp_stdio
      .mockResolvedValueOnce([]); // refreshClients
    const s = useAiIntegrationsStore();
    await s.setupClient('vscode', '/proj');
    expect(mockInvoke).toHaveBeenNthCalledWith(1, 'setup_vscode_mcp_stdio', {
      workspaceRoot: '/proj',
    });
  });

  it('setupClient switches to http command when transport=http', async () => {
    mockInvoke
      .mockResolvedValueOnce({ message: 'ok' })
      .mockResolvedValueOnce([]);
    const s = useAiIntegrationsStore();
    await s.setupClient('claude', '/proj', 'http');
    expect(mockInvoke).toHaveBeenNthCalledWith(1, 'setup_claude_mcp', {});
  });

  it('setupClient does not pass workspaceRoot for non-vscode clients', async () => {
    mockInvoke
      .mockResolvedValueOnce({ message: 'ok' })
      .mockResolvedValueOnce([]);
    const s = useAiIntegrationsStore();
    await s.setupClient('codex', '/proj', 'stdio');
    expect(mockInvoke).toHaveBeenNthCalledWith(1, 'setup_codex_mcp_stdio', {});
  });

  it('removeClient routes to the matching remove_*_mcp command', async () => {
    mockInvoke
      .mockResolvedValueOnce({ message: 'removed' })
      .mockResolvedValueOnce([]);
    const s = useAiIntegrationsStore();
    await s.removeClient('vscode', '/proj');
    expect(mockInvoke).toHaveBeenNthCalledWith(1, 'remove_vscode_mcp', {
      workspaceRoot: '/proj',
    });
  });

  it('forgetWindow calls vscode_forget_window then refreshes', async () => {
    mockInvoke
      .mockResolvedValueOnce(undefined) // forget
      .mockResolvedValueOnce([]); // refreshVscodeWindows
    const s = useAiIntegrationsStore();
    await s.forgetWindow(12345);
    expect(mockInvoke).toHaveBeenNthCalledWith(1, 'vscode_forget_window', {
      pid: 12345,
    });
  });

  it('setupClient bubbles errors and does not crash', async () => {
    mockInvoke.mockRejectedValueOnce('write denied');
    const s = useAiIntegrationsStore();
    const result = await s.setupClient('vscode', '/proj');
    expect(result).toBeNull();
    expect(s.error).toBe('write denied');
  });

  it('setTransport mutates preference', () => {
    const s = useAiIntegrationsStore();
    expect(s.preferredTransport).toBe('stdio');
    s.setTransport('http');
    expect(s.preferredTransport).toBe('http');
  });
});
