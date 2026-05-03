import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import type { MemoryEntry, Message, SystemInfo } from '../types';

export type RemoteHostKind = 'tauri-local' | 'grpc-web';
export type RemoteSearchMode = 'rrf' | 'hybrid' | 'hyde';

export interface RemoteSystemStatus {
  totalMemoryBytes: number;
  usedMemoryBytes: number;
  cpuUsagePct: number;
  brainProvider: string;
  brainModel: string;
  memoryEntryCount: number;
}

export interface RemoteVsCodeWorkspace {
  path: string;
  name: string;
  lastOpenedUnixMs: number;
}

export interface RemoteCopilotSessionStatus {
  found: boolean;
  workspaceFolder: string;
  sessionId: string;
  model: string;
  lastUserTurnTs: string;
  lastUserPreview: string;
  lastAssistantTurnTs: string;
  lastAssistantPreview: string;
  toolInvocationCount: number;
  eventCount: number;
}

export interface RemoteWorkflowRun {
  workflowId: string;
  name: string;
  status: string;
  startedAtUnixMs: number;
  lastEventAtUnixMs: number;
  eventCount: number;
}

export interface RemoteWorkflowProgress extends RemoteWorkflowRun {
  summary: string;
}

export interface RemoteContinueWorkflowResult {
  accepted: boolean;
  message: string;
}

export interface RemotePairedDevice {
  deviceId: string;
  displayName: string;
  pairedAtUnixMs: number;
  lastSeenAtUnixMs: number | null;
  capabilities: string[];
  certFingerprint?: string;
}

export interface RemoteBrainHealth {
  version: string;
  brainProvider: string;
  brainModel: string | null;
  ragQualityPct: number;
  memoryTotal: number;
}

export interface RemoteSearchHit {
  id: number;
  content: string;
  tags: string;
  importance: number;
  score: number;
  sourceUrl: string | null;
  tier: string;
}

export interface RemoteChatChunk {
  text: string;
  done: boolean;
}

export interface RemoteHost {
  kind: RemoteHostKind;
  getSystemStatus(): Promise<RemoteSystemStatus>;
  listVsCodeWorkspaces(): Promise<RemoteVsCodeWorkspace[]>;
  getCopilotSessionStatus(workspacePath?: string): Promise<RemoteCopilotSessionStatus>;
  listWorkflowRuns(includeFinished?: boolean): Promise<RemoteWorkflowRun[]>;
  getWorkflowProgress(workflowId: string): Promise<RemoteWorkflowProgress>;
  continueWorkflow(workflowId: string): Promise<RemoteContinueWorkflowResult>;
  sendChatMessage(message: string): Promise<string>;
  streamChatMessage(message: string): AsyncIterable<RemoteChatChunk>;
  listPairedDevices(): Promise<RemotePairedDevice[]>;
  brainHealth(): Promise<RemoteBrainHealth>;
  searchMemories(query: string, limit?: number, mode?: RemoteSearchMode): Promise<RemoteSearchHit[]>;
  streamSearchMemories(query: string, limit?: number, mode?: RemoteSearchMode): AsyncIterable<RemoteSearchHit>;
}

export type TauriInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

interface LocalBrainMode {
  mode?: string;
  model?: string;
  provider_id?: string;
}

interface LocalMemoryStats {
  total: number;
}

interface LocalWorkflowSummary {
  workflow_id: string;
  name: string;
  status: string;
  started_at: number;
  last_event_at: number;
  event_count: number;
}

interface LocalCopilotSummary {
  workspace_folder?: string | null;
  session_id?: string | null;
  model?: string | null;
  last_user_turn_ts?: string | null;
  last_user_preview?: string | null;
  last_assistant_turn_ts?: string | null;
  last_assistant_preview?: string | null;
  tool_invocation_count?: number;
  event_count?: number;
}

interface LocalPairedDevice {
  device_id: string;
  display_name: string;
  cert_fingerprint?: string;
  capabilities: string[];
  paired_at: number;
  last_seen_at: number | null;
}

export function createLocalRemoteHost(invoke: TauriInvoke = tauriInvoke): RemoteHost {
  return new LocalTauriRemoteHost(invoke);
}

export function toFiniteNumber(value: unknown, fallback = 0): number {
  if (typeof value === 'number' && Number.isFinite(value)) return value;
  if (typeof value === 'bigint') return Number(value);
  if (typeof value === 'string' && value.trim()) {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : fallback;
  }
  return fallback;
}

class LocalTauriRemoteHost implements RemoteHost {
  readonly kind = 'tauri-local' as const;

  constructor(private readonly invoke: TauriInvoke) {}

  async getSystemStatus(): Promise<RemoteSystemStatus> {
    const [systemInfo, brainMode, memoryStats] = await Promise.all([
      this.invoke<SystemInfo>('get_system_info'),
      this.invoke<LocalBrainMode | null>('get_brain_mode').catch(() => null),
      this.invoke<LocalMemoryStats>('get_memory_stats').catch(() => ({ total: 0 })),
    ]);
    return {
      totalMemoryBytes: systemInfo.total_ram_mb * 1024 * 1024,
      usedMemoryBytes: 0,
      cpuUsagePct: 0,
      brainProvider: brainMode?.mode ?? 'none',
      brainModel: brainMode?.model ?? brainMode?.provider_id ?? '',
      memoryEntryCount: memoryStats.total,
    };
  }

  async listVsCodeWorkspaces(): Promise<RemoteVsCodeWorkspace[]> {
    return [];
  }

  async getCopilotSessionStatus(): Promise<RemoteCopilotSessionStatus> {
    const summary = await this.invoke<LocalCopilotSummary | null>('get_copilot_session_status');
    return {
      found: Boolean(summary),
      workspaceFolder: summary?.workspace_folder ?? '',
      sessionId: summary?.session_id ?? '',
      model: summary?.model ?? '',
      lastUserTurnTs: summary?.last_user_turn_ts ?? '',
      lastUserPreview: summary?.last_user_preview ?? '',
      lastAssistantTurnTs: summary?.last_assistant_turn_ts ?? '',
      lastAssistantPreview: summary?.last_assistant_preview ?? '',
      toolInvocationCount: summary?.tool_invocation_count ?? 0,
      eventCount: summary?.event_count ?? 0,
    };
  }

  async listWorkflowRuns(includeFinished = false): Promise<RemoteWorkflowRun[]> {
    const command = includeFinished ? 'roster_list_workflows' : 'roster_list_pending_workflows';
    const workflows = await this.invoke<LocalWorkflowSummary[]>(command);
    return workflows.map(localWorkflowToRemote);
  }

  async getWorkflowProgress(workflowId: string): Promise<RemoteWorkflowProgress> {
    const workflow = (await this.listWorkflowRuns(true)).find((item) => item.workflowId === workflowId);
    if (!workflow) throw new Error(`workflow ${workflowId} not found`);
    return { ...workflow, summary: `${workflow.eventCount} events recorded locally` };
  }

  async continueWorkflow(workflowId: string): Promise<RemoteContinueWorkflowResult> {
    return {
      accepted: false,
      message: `Workflow ${workflowId} cannot be continued through local IPC yet.`,
    };
  }

  async sendChatMessage(message: string): Promise<string> {
    const reply = await this.invoke<Message>('send_message', { message, agentId: null });
    return reply.content;
  }

  async *streamChatMessage(message: string): AsyncIterable<RemoteChatChunk> {
    const text = await this.sendChatMessage(message);
    if (text) {
      yield { text, done: false };
    }
    yield { text: '', done: true };
  }

  async listPairedDevices(): Promise<RemotePairedDevice[]> {
    const devices = await this.invoke<LocalPairedDevice[]>('list_paired_devices');
    return devices.map((device) => ({
      deviceId: device.device_id,
      displayName: device.display_name,
      pairedAtUnixMs: device.paired_at,
      lastSeenAtUnixMs: device.last_seen_at,
      capabilities: device.capabilities,
      certFingerprint: device.cert_fingerprint,
    }));
  }

  async brainHealth(): Promise<RemoteBrainHealth> {
    const status = await this.getSystemStatus();
    return {
      version: 'local',
      brainProvider: status.brainProvider,
      brainModel: status.brainModel || null,
      ragQualityPct: 0,
      memoryTotal: status.memoryEntryCount,
    };
  }

  async searchMemories(query: string, limit = 10): Promise<RemoteSearchHit[]> {
    const memories = await this.invoke<MemoryEntry[]>('hybrid_search_memories_rrf', { query, limit });
    return memories.map((entry, index) => memoryEntryToSearchHit(entry, index));
  }

  async *streamSearchMemories(query: string, limit = 10): AsyncIterable<RemoteSearchHit> {
    for (const hit of await this.searchMemories(query, limit)) {
      yield hit;
    }
  }
}

function localWorkflowToRemote(workflow: LocalWorkflowSummary): RemoteWorkflowRun {
  return {
    workflowId: workflow.workflow_id,
    name: workflow.name,
    status: workflow.status,
    startedAtUnixMs: workflow.started_at,
    lastEventAtUnixMs: workflow.last_event_at,
    eventCount: workflow.event_count,
  };
}

function memoryEntryToSearchHit(entry: MemoryEntry, index: number): RemoteSearchHit {
  return {
    id: entry.id,
    content: entry.content,
    tags: entry.tags,
    importance: entry.importance,
    score: 1 / (index + 1),
    sourceUrl: null,
    tier: entry.tier,
  };
}