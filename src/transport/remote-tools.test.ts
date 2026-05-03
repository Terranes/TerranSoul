import { describe, expect, it, vi } from 'vitest';
import {
  detectRemotePhoneToolIntent,
  dispatchRemotePhoneTool,
  REMOTE_PHONE_TOOL_DEFINITIONS,
} from './remote-tools';
import type { RemoteHost } from './remote-host';

describe('remote phone tools', () => {
  it('exposes the expected MCP-style tool names', () => {
    expect(REMOTE_PHONE_TOOL_DEFINITIONS.map((tool) => tool.name)).toEqual([
      'describe_copilot_session',
      'describe_workflow_progress',
      'continue_workflow',
    ]);
  });

  it('describes a Copilot session through RemoteHost', async () => {
    const host = fakeRemoteHost({
      getCopilotSessionStatus: vi.fn(async () => ({
        found: true,
        workspaceFolder: 'D:/Git/TerranSoul',
        sessionId: 's1',
        model: 'gpt-5-mini',
        lastUserTurnTs: '10:00',
        lastUserPreview: 'Continue next chunks',
        lastAssistantTurnTs: '10:01',
        lastAssistantPreview: 'Running tests',
        toolInvocationCount: 2,
        eventCount: 8,
      })),
    });

    const result = await dispatchRemotePhoneTool(host, 'describe_copilot_session', {}, {
      capabilities: ['copilot:read'],
    });

    expect(result.content).toContain('Copilot Chat is active in D:/Git/TerranSoul');
    expect(result.content).toContain('Running tests');
    expect(result.content).toContain('2 tool invocations');
  });

  it('selects the most recent pending workflow for progress narration', async () => {
    const host = fakeRemoteHost({
      listWorkflowRuns: vi.fn(async () => [
        workflowRun('old', 'Older run', 1_000),
        workflowRun('new', 'Newer run', 2_000),
      ]),
      getWorkflowProgress: vi.fn(async (workflowId: string) => ({
        ...workflowRun(workflowId, 'Newer run', 2_000),
        summary: 'phase two is running',
      })),
    });

    const result = await dispatchRemotePhoneTool(host, 'describe_workflow_progress', {}, {
      capabilities: ['workflow:read'],
      now: () => 125_000,
    });

    expect(result.content).toContain('Workflow Newer run is running');
    expect(result.content).toContain('phase two is running');
  });

  it('continues the selected active workflow and narrates the result', async () => {
    const continueWorkflow = vi.fn(async () => ({ accepted: true, message: 'heartbeat sent' }));
    const host = fakeRemoteHost({
      listWorkflowRuns: vi.fn(async () => [workflowRun('wf-active', 'Copilot loop', 3_000)]),
      continueWorkflow,
      getWorkflowProgress: vi.fn(async () => ({
        ...workflowRun('wf-active', 'Copilot loop', 3_000),
        eventCount: 5,
        summary: 'resumed via phone',
      })),
    });

    const result = await dispatchRemotePhoneTool(host, 'continue_workflow', {}, {
      capabilities: ['workflow:continue'],
    });

    expect(continueWorkflow).toHaveBeenCalledWith('wf-active');
    expect(result.content).toContain('Continue request accepted');
    expect(result.content).toContain('resumed via phone');
  });

  it('denies tools outside the provided capability scope', async () => {
    await expect(dispatchRemotePhoneTool(fakeRemoteHost(), 'continue_workflow', {}, {
      capabilities: ['workflow:read'],
    })).rejects.toThrow('workflow:continue');
  });

  it('detects the headline phone prompts', () => {
    expect(detectRemotePhoneToolIntent("What's Copilot doing on my desktop?")?.name)
      .toBe('describe_copilot_session');
    expect(detectRemotePhoneToolIntent('continue the next chunk')?.name)
      .toBe('continue_workflow');
    expect(detectRemotePhoneToolIntent('workflow progress please')?.name)
      .toBe('describe_workflow_progress');
  });
});

function workflowRun(workflowId: string, name: string, lastEventAtUnixMs: number) {
  return {
    workflowId,
    name,
    status: 'Running',
    startedAtUnixMs: 500,
    lastEventAtUnixMs,
    eventCount: 3,
  };
}

function fakeRemoteHost(overrides: Partial<RemoteHost> = {}): RemoteHost {
  return {
    kind: 'grpc-web',
    getSystemStatus: async () => ({
      totalMemoryBytes: 0,
      usedMemoryBytes: 0,
      cpuUsagePct: 0,
      brainProvider: 'stub',
      brainModel: 'stub',
      memoryEntryCount: 0,
    }),
    listVsCodeWorkspaces: async () => [],
    getCopilotSessionStatus: async () => ({
      found: false,
      workspaceFolder: '',
      sessionId: '',
      model: '',
      lastUserTurnTs: '',
      lastUserPreview: '',
      lastAssistantTurnTs: '',
      lastAssistantPreview: '',
      toolInvocationCount: 0,
      eventCount: 0,
    }),
    listWorkflowRuns: async () => [],
    getWorkflowProgress: async () => ({
      ...workflowRun('wf', 'wf', 0),
      summary: '',
    }),
    continueWorkflow: async () => ({ accepted: false, message: '' }),
    sendChatMessage: async () => 'reply',
    streamChatMessage: async function* () {},
    listPairedDevices: async () => [],
    brainHealth: async () => ({
      version: 'test',
      brainProvider: 'stub',
      brainModel: null,
      ragQualityPct: 0,
      memoryTotal: 0,
    }),
    searchMemories: async () => [],
    streamSearchMemories: async function* () {},
    ...overrides,
  };
}