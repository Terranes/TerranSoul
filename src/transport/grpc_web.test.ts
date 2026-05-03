import { describe, expect, it } from 'vitest';
import { HealthResponse, SearchHit, SearchResponse } from './brain_pb';
import { createGrpcWebRemoteHost, normaliseBaseUrl, remoteBaseUrl } from './grpc_web';
import {
  ChatChunk,
  ChatResponse,
  ContinueWorkflowResponse,
  CopilotSessionResponse,
  ListDevicesResponse,
  ListWorkflowsResponse,
  ListWorkspacesResponse,
  PairedDeviceInfo,
  SystemStatusResponse,
  WorkflowProgressResponse,
  WorkflowRun,
} from './phone_control_pb';

describe('gRPC-Web RemoteHost adapter', () => {
  it('maps phone-control responses into RemoteHost DTOs', async () => {
    const host = createGrpcWebRemoteHost({
      baseUrl: 'http://192.168.1.42:7422/',
      phoneClient: {
        getSystemStatus: async () => new SystemStatusResponse({
          totalMemoryBytes: '16000000000',
          usedMemoryBytes: '6000000000',
          cpuUsagePct: 12.5,
          brainProvider: 'local_ollama',
          brainModel: 'gemma3:4b',
          memoryEntryCount: 42,
        }),
        listVsCodeWorkspaces: async () => new ListWorkspacesResponse(),
        getCopilotSessionStatus: async () => new CopilotSessionResponse({ found: true, sessionId: 's1' }),
        listWorkflowRuns: async () => new ListWorkflowsResponse({
          runs: [new WorkflowRun({
            workflowId: 'wf-1',
            name: 'Copilot loop',
            status: 'Running',
            startedAtUnixMs: '1000',
            lastEventAtUnixMs: '1500',
            eventCount: '3',
          })],
        }),
        getWorkflowProgress: async () => new WorkflowProgressResponse({
          workflowId: 'wf-1',
          name: 'Copilot loop',
          status: 'Running',
          startedAtUnixMs: '1000',
          lastEventAtUnixMs: '1500',
          eventCount: '3',
          summary: '3 events, still moving',
        }),
        continueWorkflow: async () => new ContinueWorkflowResponse({ accepted: true, message: 'heartbeat sent' }),
        sendChatMessage: async () => new ChatResponse({ reply: 'hello phone' }),
        streamChatMessage: async function* () {
          yield new ChatChunk({ text: 'hello ', done: false });
          yield new ChatChunk({ text: 'phone', done: false });
          yield new ChatChunk({ done: true });
        },
        listPairedDevices: async () => new ListDevicesResponse({
          devices: [new PairedDeviceInfo({
            deviceId: 'ios-1',
            displayName: 'Ari iPhone',
            pairedAtUnixMs: '2000',
            lastSeenAtUnixMs: '3000',
            capabilities: ['chat'],
          })],
        }),
      },
      brainClient: {
        health: async () => new HealthResponse({
          version: '0.1',
          brainProvider: 'local_ollama',
          brainModel: 'gemma3:4b',
          ragQualityPct: 88,
          memoryTotal: '42',
        }),
        search: async () => new SearchResponse(),
        streamSearch: async function* () {},
      },
    });

    await expect(host.getSystemStatus()).resolves.toMatchObject({
      totalMemoryBytes: 16000000000,
      brainProvider: 'local_ollama',
      memoryEntryCount: 42,
    });
    await expect(host.listWorkflowRuns(true)).resolves.toEqual([
      expect.objectContaining({ workflowId: 'wf-1', eventCount: 3 }),
    ]);
    await expect(host.getWorkflowProgress('wf-1')).resolves.toMatchObject({ summary: '3 events, still moving' });
    await expect(host.continueWorkflow('wf-1')).resolves.toEqual({ accepted: true, message: 'heartbeat sent' });
    await expect(host.sendChatMessage('hi')).resolves.toBe('hello phone');
    const chunks = [];
    for await (const chunk of host.streamChatMessage('hi')) chunks.push(chunk);
    expect(chunks).toEqual([
      { text: 'hello ', done: false },
      { text: 'phone', done: false },
      { text: '', done: true },
    ]);
    await expect(host.listPairedDevices()).resolves.toEqual([
      expect.objectContaining({ deviceId: 'ios-1', displayName: 'Ari iPhone', lastSeenAtUnixMs: 3000 }),
    ]);
  });

  it('supports server-streaming brain search responses', async () => {
    const host = createGrpcWebRemoteHost({
      baseUrl: 'http://localhost:7422',
      phoneClient: emptyPhoneClient(),
      brainClient: {
        health: async () => new HealthResponse({
          version: '0.1',
          brainProvider: 'stub',
          ragQualityPct: 0,
          memoryTotal: '0',
        }),
        search: async () => new SearchResponse(),
        streamSearch: async function* () {
          yield new SearchHit({ id: '7', content: 'local-first memory', importance: '4', score: 0.9, tags: 'project', tier: 'long' });
        },
      },
    });

    const hits = [];
    for await (const hit of host.streamSearchMemories('local-first', 3, 'rrf')) {
      hits.push(hit);
    }

    expect(hits).toEqual([
      expect.objectContaining({ id: 7, content: 'local-first memory', importance: 4, sourceUrl: null }),
    ]);
  });

  it('normalises endpoint helpers conservatively', () => {
    expect(normaliseBaseUrl(' http://host:7422/// ')).toBe('http://host:7422');
    expect(remoteBaseUrl('192.168.1.42', 7422)).toBe('http://192.168.1.42:7422');
    expect(remoteBaseUrl('terransoul.local', 7422, true)).toBe('https://terransoul.local:7422');
    expect(() => remoteBaseUrl('', 7422)).toThrow('hostname');
    expect(() => remoteBaseUrl('host', 70000)).toThrow('valid TCP port');
  });
});

function emptyPhoneClient() {
  return {
    getSystemStatus: async () => new SystemStatusResponse(),
    listVsCodeWorkspaces: async () => new ListWorkspacesResponse(),
    getCopilotSessionStatus: async () => new CopilotSessionResponse(),
    listWorkflowRuns: async () => new ListWorkflowsResponse(),
    getWorkflowProgress: async () => new WorkflowProgressResponse(),
    continueWorkflow: async () => new ContinueWorkflowResponse(),
    sendChatMessage: async () => new ChatResponse(),
    streamChatMessage: async function* () {},
    listPairedDevices: async () => new ListDevicesResponse(),
  };
}