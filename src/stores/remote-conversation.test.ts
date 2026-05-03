import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import {
  configureRemoteConversationAdapters,
  resetRemoteConversationAdapters,
  useRemoteConversationStore,
} from './remote-conversation';
import type { RemoteHost } from '../transport';

describe('remote conversation store', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  afterEach(() => {
    resetRemoteConversationAdapters();
  });

  it('streams chat through RemoteHost and stores the final assistant message', async () => {
    const streamChatMessage = vi.fn(async function* () {
      yield { text: '[happy]Hello ', done: false };
      yield { text: 'from desktop', done: false };
      yield { text: '', done: true };
    });
    configureRemoteConversationAdapters({
      remoteHost: () => fakeRemoteHost({ streamChatMessage }),
      now: () => 1234,
      createId: idSequence('msg'),
    });

    const store = useRemoteConversationStore();
    await store.sendMessage('hello');

    expect(streamChatMessage).toHaveBeenCalledWith('hello');
    expect(store.messages).toHaveLength(2);
    expect(store.messages[0]).toMatchObject({ id: 'msg-1', role: 'user', content: 'hello' });
    expect(store.messages[1]).toMatchObject({
      id: 'msg-2',
      role: 'assistant',
      content: 'Hello from desktop',
      sentiment: 'happy',
    });
    expect(store.isThinking).toBe(false);
    expect(store.isStreaming).toBe(false);
    expect(store.streamingText).toBe('');
  });

  it('falls back to unary chat when a stream returns no text', async () => {
    configureRemoteConversationAdapters({
      remoteHost: () => fakeRemoteHost({
        sendChatMessage: vi.fn(async () => 'unary reply'),
        streamChatMessage: vi.fn(async function* () {
          yield { text: '', done: true };
        }),
      }),
      now: () => 2000,
      createId: idSequence('fb'),
    });

    const store = useRemoteConversationStore();
    await store.sendMessage('fallback please');

    expect(store.messages[1]).toMatchObject({ content: 'unary reply', agentName: 'TerranSoul' });
  });

  it('shows a pairing prompt when no remote host is available', async () => {
    configureRemoteConversationAdapters({
      remoteHost: () => null,
      now: () => 3000,
      createId: idSequence('pair'),
    });

    const store = useRemoteConversationStore();
    await store.sendMessage('hello?');

    expect(store.messages[1].content).toContain('not connected to a desktop host');
  });

  it('routes Copilot progress questions through the remote tool surface', async () => {
    const streamChatMessage = vi.fn(async function* () {
      yield { text: 'generic chat should not run', done: false };
    });
    configureRemoteConversationAdapters({
      remoteHost: () => fakeRemoteHost({
        streamChatMessage,
        getCopilotSessionStatus: async () => ({
          found: true,
          workspaceFolder: 'D:/Git/TerranSoul',
          sessionId: 's1',
          model: 'gpt-5-mini',
          lastUserTurnTs: '10:00',
          lastUserPreview: 'Continue next chunks',
          lastAssistantTurnTs: '10:01',
          lastAssistantPreview: 'Running focused tests',
          toolInvocationCount: 1,
          eventCount: 4,
        }),
      }),
      capabilities: () => ['copilot:read'],
      now: () => 4000,
      createId: idSequence('tool'),
    });

    const store = useRemoteConversationStore();
    await store.sendMessage("What's Copilot doing on my desktop?");

    expect(streamChatMessage).not.toHaveBeenCalled();
    expect(store.messages[1].content).toContain('Copilot Chat is active in D:/Git/TerranSoul');
    expect(store.messages[1].content).toContain('Running focused tests');
  });

  it('routes continue-next-step prompts through the workflow tool', async () => {
    const continueWorkflow = vi.fn(async () => ({ accepted: true, message: 'workflow wf-1 heartbeat sent' }));
    configureRemoteConversationAdapters({
      remoteHost: () => fakeRemoteHost({
        listWorkflowRuns: async () => [{
          workflowId: 'wf-1',
          name: 'Copilot loop',
          status: 'Running',
          startedAtUnixMs: 1000,
          lastEventAtUnixMs: 2000,
          eventCount: 3,
        }],
        continueWorkflow,
        getWorkflowProgress: async () => ({
          workflowId: 'wf-1',
          name: 'Copilot loop',
          status: 'Running',
          startedAtUnixMs: 1000,
          lastEventAtUnixMs: 2000,
          eventCount: 4,
          summary: 'resumed via phone',
        }),
      }),
      capabilities: () => ['workflow:continue'],
      now: () => 5000,
      createId: idSequence('cont'),
    });

    const store = useRemoteConversationStore();
    await store.sendMessage('continue the next chunk');

    expect(continueWorkflow).toHaveBeenCalledWith('wf-1');
    expect(store.messages[1].content).toContain('Continue request accepted');
    expect(store.messages[1].content).toContain('resumed via phone');
  });
});

function idSequence(prefix: string): () => string {
  let next = 0;
  return () => `${prefix}-${++next}`;
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
      workflowId: 'wf',
      name: 'wf',
      status: 'Done',
      startedAtUnixMs: 0,
      lastEventAtUnixMs: 0,
      eventCount: 0,
      summary: '',
    }),
    continueWorkflow: async () => ({ accepted: false, message: '' }),
    sendChatMessage: async () => 'reply',
    streamChatMessage: async function* () {
      yield { text: 'reply', done: false };
      yield { text: '', done: true };
    },
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