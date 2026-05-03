import { createPromiseClient, type PromiseClient, type Transport } from '@bufbuild/connect';
import { createGrpcWebTransport } from '@bufbuild/connect-web';
import { BrainService, SearchMode, type SearchHit as BrainSearchHit } from './brain_pb';
import { PhoneControlService, type PairedDeviceInfo, type WorkflowRun } from './phone_control_pb';
import {
  toFiniteNumber,
  type RemoteBrainHealth,
  type RemoteChatChunk,
  type RemoteContinueWorkflowResult,
  type RemoteCopilotSessionStatus,
  type RemoteHost,
  type RemotePairedDevice,
  type RemoteSearchHit,
  type RemoteSearchMode,
  type RemoteSystemStatus,
  type RemoteVsCodeWorkspace,
  type RemoteWorkflowProgress,
  type RemoteWorkflowRun,
} from './remote-host';

type PhoneControlClient = PromiseClient<typeof PhoneControlService>;
type BrainClient = PromiseClient<typeof BrainService>;

export interface GrpcWebRemoteHostOptions {
  baseUrl: string;
  fetch?: typeof globalThis.fetch;
  transport?: Transport;
  phoneClient?: PhoneControlClient;
  brainClient?: BrainClient;
}

export function createGrpcWebRemoteHost(options: GrpcWebRemoteHostOptions): RemoteHost {
  const transport = options.transport ?? createGrpcWebTransport({
    baseUrl: normaliseBaseUrl(options.baseUrl),
    fetch: options.fetch,
    useBinaryFormat: true,
  });
  return new GrpcWebRemoteHost(
    options.phoneClient ?? createPromiseClient(PhoneControlService, transport),
    options.brainClient ?? createPromiseClient(BrainService, transport),
  );
}

export function normaliseBaseUrl(baseUrl: string): string {
  const trimmed = baseUrl.trim();
  if (!trimmed) throw new Error('Remote host baseUrl is required.');
  return trimmed.replace(/\/+$/, '');
}

export function remoteBaseUrl(host: string, port: number, secure = false): string {
  if (!host.trim()) throw new Error('Remote host requires a hostname.');
  if (!Number.isInteger(port) || port <= 0 || port > 65535) {
    throw new Error('Remote host requires a valid TCP port.');
  }
  return `${secure ? 'https' : 'http'}://${host.trim()}:${port}`;
}

class GrpcWebRemoteHost implements RemoteHost {
  readonly kind = 'grpc-web' as const;

  constructor(
    private readonly phoneClient: PhoneControlClient,
    private readonly brainClient: BrainClient,
  ) {}

  async getSystemStatus(): Promise<RemoteSystemStatus> {
    const response = await this.phoneClient.getSystemStatus({});
    return {
      totalMemoryBytes: toFiniteNumber(response.totalMemoryBytes),
      usedMemoryBytes: toFiniteNumber(response.usedMemoryBytes),
      cpuUsagePct: response.cpuUsagePct,
      brainProvider: response.brainProvider,
      brainModel: response.brainModel,
      memoryEntryCount: response.memoryEntryCount,
    };
  }

  async listVsCodeWorkspaces(): Promise<RemoteVsCodeWorkspace[]> {
    const response = await this.phoneClient.listVsCodeWorkspaces({});
    return response.workspaces.map((workspace) => ({
      path: workspace.path,
      name: workspace.name,
      lastOpenedUnixMs: toFiniteNumber(workspace.lastOpenedUnixMs),
    }));
  }

  async getCopilotSessionStatus(workspacePath = ''): Promise<RemoteCopilotSessionStatus> {
    const response = await this.phoneClient.getCopilotSessionStatus({ workspacePath });
    return {
      found: response.found,
      workspaceFolder: response.workspaceFolder,
      sessionId: response.sessionId,
      model: response.model,
      lastUserTurnTs: response.lastUserTurnTs,
      lastUserPreview: response.lastUserPreview,
      lastAssistantTurnTs: response.lastAssistantTurnTs,
      lastAssistantPreview: response.lastAssistantPreview,
      toolInvocationCount: response.toolInvocationCount,
      eventCount: response.eventCount,
    };
  }

  async listWorkflowRuns(includeFinished = false): Promise<RemoteWorkflowRun[]> {
    const response = await this.phoneClient.listWorkflowRuns({ includeFinished });
    return response.runs.map(workflowRunToRemote);
  }

  async getWorkflowProgress(workflowId: string): Promise<RemoteWorkflowProgress> {
    const response = await this.phoneClient.getWorkflowProgress({ workflowId });
    return {
      workflowId: response.workflowId,
      name: response.name,
      status: response.status,
      startedAtUnixMs: toFiniteNumber(response.startedAtUnixMs),
      lastEventAtUnixMs: toFiniteNumber(response.lastEventAtUnixMs),
      eventCount: toFiniteNumber(response.eventCount),
      summary: response.summary,
    };
  }

  async continueWorkflow(workflowId: string): Promise<RemoteContinueWorkflowResult> {
    const response = await this.phoneClient.continueWorkflow({ workflowId });
    return { accepted: response.accepted, message: response.message };
  }

  async sendChatMessage(message: string): Promise<string> {
    const response = await this.phoneClient.sendChatMessage({ message });
    return response.reply;
  }

  async *streamChatMessage(message: string): AsyncIterable<RemoteChatChunk> {
    for await (const chunk of this.phoneClient.streamChatMessage({ message })) {
      yield { text: chunk.text, done: chunk.done };
    }
  }

  async listPairedDevices(): Promise<RemotePairedDevice[]> {
    const response = await this.phoneClient.listPairedDevices({});
    return response.devices.map(pairedDeviceToRemote);
  }

  async brainHealth(): Promise<RemoteBrainHealth> {
    const response = await this.brainClient.health({});
    return {
      version: response.version,
      brainProvider: response.brainProvider,
      brainModel: response.brainModel ?? null,
      ragQualityPct: response.ragQualityPct,
      memoryTotal: toFiniteNumber(response.memoryTotal),
    };
  }

  async searchMemories(query: string, limit = 10, mode: RemoteSearchMode = 'rrf'): Promise<RemoteSearchHit[]> {
    const response = await this.brainClient.search({ query, limit, mode: searchModeToProto(mode) });
    return response.hits.map(searchHitToRemote);
  }

  async *streamSearchMemories(query: string, limit = 10, mode: RemoteSearchMode = 'rrf'): AsyncIterable<RemoteSearchHit> {
    for await (const hit of this.brainClient.streamSearch({ query, limit, mode: searchModeToProto(mode) })) {
      yield searchHitToRemote(hit);
    }
  }
}

function workflowRunToRemote(run: WorkflowRun): RemoteWorkflowRun {
  return {
    workflowId: run.workflowId,
    name: run.name,
    status: run.status,
    startedAtUnixMs: toFiniteNumber(run.startedAtUnixMs),
    lastEventAtUnixMs: toFiniteNumber(run.lastEventAtUnixMs),
    eventCount: toFiniteNumber(run.eventCount),
  };
}

function pairedDeviceToRemote(device: PairedDeviceInfo): RemotePairedDevice {
  return {
    deviceId: device.deviceId,
    displayName: device.displayName,
    pairedAtUnixMs: toFiniteNumber(device.pairedAtUnixMs),
    lastSeenAtUnixMs: toFiniteNumber(device.lastSeenAtUnixMs) || null,
    capabilities: [...device.capabilities],
  };
}

function searchHitToRemote(hit: BrainSearchHit): RemoteSearchHit {
  return {
    id: toFiniteNumber(hit.id),
    content: hit.content,
    tags: hit.tags,
    importance: toFiniteNumber(hit.importance),
    score: hit.score,
    sourceUrl: hit.sourceUrl ?? null,
    tier: hit.tier,
  };
}

function searchModeToProto(mode: RemoteSearchMode): SearchMode {
  switch (mode) {
    case 'hybrid':
      return SearchMode.Hybrid;
    case 'hyde':
      return SearchMode.Hyde;
    case 'rrf':
      return SearchMode.Rrf;
  }
}