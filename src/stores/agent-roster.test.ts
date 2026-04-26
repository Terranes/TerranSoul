/**
 * Tests for the agent roster store.
 * Mocks the Tauri invoke surface so we can assert the store's IPC
 * contract + browser-fallback behaviour.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import type {
  AgentProfile,
  AgentRamCapReport,
  BrainBackend,
  WorkflowSummary,
} from './agent-roster';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

// Import after the mock is registered so the store's `invoke` binding
// goes through the mock.
import { useAgentRosterStore } from './agent-roster';

const sampleNative: AgentProfile = {
  id: 'default',
  display_name: 'TerranSoul',
  vrm_model_id: 'shinra',
  brain_backend: { kind: 'native', data: { mode: null } },
  working_folder: null,
  created_at: 1,
  last_active_at: 2,
};

const sampleCli: AgentProfile = {
  id: 'coder',
  display_name: 'Coder',
  vrm_model_id: 'komori',
  brain_backend: {
    kind: 'external_cli',
    data: { kind: 'codex', binary: 'codex', extra_args: ['--json'] },
  },
  working_folder: '/tmp/repo',
  created_at: 3,
  last_active_at: 4,
};

const sampleCap: AgentRamCapReport = {
  cap: {
    free_mb: 16000,
    reserve_mb: 1500,
    cap: 4,
    mean_per_agent_mb: 400,
    reasoning: '14500 MB usable / 400 MB per agent = 36 concurrent.',
  },
  footprints: [
    { agent_id: 'default', kind: 'native_api', estimated_mb: 200 },
    { agent_id: 'coder', kind: 'external_cli', estimated_mb: 600 },
  ],
};

const samplePending: WorkflowSummary[] = [
  {
    workflow_id: 'wf-1',
    name: 'cli_run',
    status: 'running',
    started_at: 10,
    last_event_at: 20,
    event_count: 3,
  },
  {
    workflow_id: 'wf-2',
    name: 'cli_run',
    status: 'resuming',
    started_at: 5,
    last_event_at: 6,
    event_count: 1,
  },
  {
    workflow_id: 'wf-3',
    name: 'cli_run',
    status: 'completed',
    started_at: 1,
    last_event_at: 2,
    event_count: 4,
  },
];

function mockTauriAvailable(available: boolean) {
  if (available) {
    (window as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {};
  } else {
    delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  }
}

describe('agent roster store', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockTauriAvailable(false);
  });

  it('browser fallback: refresh yields a single default agent', async () => {
    const store = useAgentRosterStore();
    await store.refresh();
    expect(store.agents).toHaveLength(1);
    expect(store.agents[0].id).toBe('default');
    expect(store.currentAgent?.id).toBe('default');
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it('Tauri refresh populates agents + ramCap + workflows', async () => {
    mockTauriAvailable(true);
    mockInvoke.mockImplementation(async (cmd: string) => {
      switch (cmd) {
        case 'roster_list':
          return [sampleNative, sampleCli];
        case 'roster_get_current':
          return 'coder';
        case 'roster_get_ram_cap':
          return sampleCap;
        case 'roster_list_workflows':
          return samplePending;
        default:
          throw new Error(`unexpected command: ${cmd}`);
      }
    });

    const store = useAgentRosterStore();
    await store.refresh();

    expect(store.agents).toHaveLength(2);
    expect(store.currentAgentId).toBe('coder');
    expect(store.currentAgent?.id).toBe('coder');
    expect(store.ramCap?.cap.cap).toBe(4);
    expect(store.workflows).toHaveLength(3);
    // activeWorkflowCount excludes terminal states
    expect(store.activeWorkflowCount).toBe(2);
  });

  it('atRamCap is false when there is headroom', async () => {
    mockTauriAvailable(true);
    mockInvoke.mockImplementation(async (cmd: string) => {
      switch (cmd) {
        case 'roster_list':
          return [sampleNative];
        case 'roster_get_current':
          return 'default';
        case 'roster_get_ram_cap':
          return sampleCap; // cap = 4
        case 'roster_list_workflows':
          return []; // 0 active
        default:
          return null;
      }
    });
    const store = useAgentRosterStore();
    await store.refresh();
    expect(store.atRamCap).toBe(false);
  });

  it('atRamCap is true when activeWorkflowCount >= cap', async () => {
    mockTauriAvailable(true);
    const fullCap: AgentRamCapReport = {
      ...sampleCap,
      cap: { ...sampleCap.cap, cap: 1 },
    };
    mockInvoke.mockImplementation(async (cmd: string) => {
      switch (cmd) {
        case 'roster_list':
          return [sampleNative];
        case 'roster_get_current':
          return 'default';
        case 'roster_get_ram_cap':
          return fullCap;
        case 'roster_list_workflows':
          return [samplePending[0]]; // 1 running
        default:
          return null;
      }
    });
    const store = useAgentRosterStore();
    await store.refresh();
    expect(store.atRamCap).toBe(true);
  });

  it('createAgent forwards the full request payload', async () => {
    mockTauriAvailable(true);
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'roster_create') return sampleCli;
      if (cmd === 'roster_list') return [sampleNative, sampleCli];
      if (cmd === 'roster_get_current') return 'coder';
      if (cmd === 'roster_get_ram_cap') return sampleCap;
      if (cmd === 'roster_list_workflows') return [];
      return null;
    });
    const store = useAgentRosterStore();
    const backend: BrainBackend = {
      kind: 'external_cli',
      data: { kind: 'codex', binary: 'codex', extra_args: ['--json'] },
    };
    const created = await store.createAgent('Coder', 'komori', backend, '/tmp/repo');
    expect(created?.id).toBe('coder');
    const createCall = mockInvoke.mock.calls.find((c) => c[0] === 'roster_create');
    expect(createCall?.[1]).toEqual({
      request: {
        display_name: 'Coder',
        vrm_model_id: 'komori',
        brain_backend: backend,
        working_folder: '/tmp/repo',
      },
    });
  });

  it('createAgent surfaces backend errors via lastError', async () => {
    mockTauriAvailable(true);
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'roster_create') throw 'agent roster is full (max 32)';
      return null;
    });
    const store = useAgentRosterStore();
    const res = await store.createAgent('X', 'y', { kind: 'native', data: { mode: null } });
    expect(res).toBeNull();
    expect(store.lastError).toContain('full');
  });

  it('startCliWorkflow returns the workflow id', async () => {
    mockTauriAvailable(true);
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'roster_start_cli_workflow') return { workflow_id: 'wf-42' };
      if (cmd === 'roster_list') return [sampleCli];
      if (cmd === 'roster_get_current') return 'coder';
      if (cmd === 'roster_get_ram_cap') return sampleCap;
      if (cmd === 'roster_list_workflows') return [];
      return null;
    });
    const store = useAgentRosterStore();
    const id = await store.startCliWorkflow('coder', 'explain this repo');
    expect(id).toBe('wf-42');
  });

  it('switchAgent in browser mode only flips the active id', async () => {
    const store = useAgentRosterStore();
    await store.refresh();
    const ok = await store.switchAgent('default');
    expect(ok).toBe(true);
    expect(store.currentAgentId).toBe('default');
  });

  it('deleteAgent fails cleanly in browser mode', async () => {
    const store = useAgentRosterStore();
    await store.refresh();
    const ok = await store.deleteAgent('default');
    expect(ok).toBe(false);
  });
});
