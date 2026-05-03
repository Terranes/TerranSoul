import type {
  RemoteCopilotSessionStatus,
  RemoteContinueWorkflowResult,
  RemoteHost,
  RemoteWorkflowProgress,
  RemoteWorkflowRun,
} from './remote-host';

export type RemotePhoneToolName =
  | 'describe_copilot_session'
  | 'describe_workflow_progress'
  | 'continue_workflow';

export interface RemotePhoneToolDefinition {
  name: RemotePhoneToolName;
  description: string;
  inputSchema: Record<string, unknown>;
  requiredCapabilities: string[];
}

export interface RemotePhoneToolCall {
  name: RemotePhoneToolName;
  args?: Record<string, unknown>;
}

export interface RemotePhoneToolOptions {
  capabilities?: string[];
  now?: () => number;
}

export interface RemotePhoneToolResult {
  name: RemotePhoneToolName;
  content: string;
  data: unknown;
}

const READ_COPILOT_CAPS = ['copilot:read', 'copilot', 'desktop:read', '*'];
const READ_WORKFLOW_CAPS = ['workflow:read', 'workflow', 'desktop:read', '*'];
const CONTINUE_WORKFLOW_CAPS = ['workflow:continue', 'workflow', 'desktop:write', '*'];

export const REMOTE_PHONE_TOOL_DEFINITIONS: RemotePhoneToolDefinition[] = [
  {
    name: 'describe_copilot_session',
    description: 'Describe the latest VS Code Copilot Chat session visible on the paired desktop.',
    inputSchema: {
      type: 'object',
      properties: {
        workspacePath: { type: 'string', description: 'Optional VS Code workspace path filter.' },
      },
    },
    requiredCapabilities: READ_COPILOT_CAPS,
  },
  {
    name: 'describe_workflow_progress',
    description: 'Describe progress for a workflow, or the most recent active workflow when no id is supplied.',
    inputSchema: {
      type: 'object',
      properties: {
        workflowId: { type: 'string', description: 'Optional workflow id. Empty selects the active run.' },
      },
    },
    requiredCapabilities: READ_WORKFLOW_CAPS,
  },
  {
    name: 'continue_workflow',
    description: 'Ask the paired desktop to continue or heartbeat an active workflow, then narrate its updated progress.',
    inputSchema: {
      type: 'object',
      properties: {
        workflowId: { type: 'string', description: 'Optional workflow id. Empty selects the active run.' },
      },
    },
    requiredCapabilities: CONTINUE_WORKFLOW_CAPS,
  },
];

export async function dispatchRemotePhoneTool(
  host: RemoteHost,
  name: RemotePhoneToolName,
  args: Record<string, unknown> = {},
  options: RemotePhoneToolOptions = {},
): Promise<RemotePhoneToolResult> {
  assertToolCapability(name, options.capabilities);
  switch (name) {
    case 'describe_copilot_session': {
      const status = await host.getCopilotSessionStatus(asOptionalString(args.workspacePath));
      return {
        name,
        content: describeCopilotSession(status),
        data: status,
      };
    }
    case 'describe_workflow_progress': {
      const progress = await resolveWorkflowProgress(host, asOptionalString(args.workflowId));
      return {
        name,
        content: describeWorkflowProgress(progress, options.now),
        data: progress,
      };
    }
    case 'continue_workflow': {
      const selected = await selectWorkflow(host, asOptionalString(args.workflowId), false);
      if (!selected) {
        return {
          name,
          content: 'No active desktop workflow is available to continue right now.',
          data: { accepted: false, message: 'no active workflow' },
        };
      }
      const continued = await host.continueWorkflow(selected.workflowId);
      const progress = await host.getWorkflowProgress(selected.workflowId).catch(() => workflowRunToProgress(selected));
      return {
        name,
        content: describeContinueWorkflow(continued, progress, options.now),
        data: { continued, progress },
      };
    }
  }
}

export function detectRemotePhoneToolIntent(input: string): RemotePhoneToolCall | null {
  const text = input.trim();
  const lower = text.toLowerCase();
  const workflowId = extractWorkflowId(text);

  if (/\bcontinue\b/.test(lower) && /\b(workflow|chunk|step|task|copilot|work)\b/.test(lower)) {
    return { name: 'continue_workflow', args: workflowId ? { workflowId } : {} };
  }
  if (/\bcopilot\b/.test(lower) && /\b(doing|status|progress|session|active|working|what'?s)\b/.test(lower)) {
    return { name: 'describe_copilot_session' };
  }
  if (/\b(workflow|task|run|chunk)\b/.test(lower) && /\b(progress|status|doing|where|active)\b/.test(lower)) {
    return { name: 'describe_workflow_progress', args: workflowId ? { workflowId } : {} };
  }
  return null;
}

export function describeCopilotSession(status: RemoteCopilotSessionStatus): string {
  if (!status.found) {
    return 'I do not see an active VS Code Copilot Chat session on the paired desktop.';
  }

  const parts = [
    `Copilot Chat is active${status.workspaceFolder ? ` in ${status.workspaceFolder}` : ''}.`,
  ];
  if (status.model) parts.push(`Model: ${status.model}.`);
  if (status.lastUserPreview) parts.push(`Last user turn: ${status.lastUserPreview}`);
  if (status.lastAssistantPreview) parts.push(`Last assistant turn: ${status.lastAssistantPreview}`);
  parts.push(`${status.eventCount} log events recorded, including ${status.toolInvocationCount} tool invocation${status.toolInvocationCount === 1 ? '' : 's'}.`);
  return parts.join(' ');
}

export function describeWorkflowProgress(
  progress: RemoteWorkflowProgress | null,
  now: () => number = Date.now,
): string {
  if (!progress) {
    return 'No desktop workflow runs are available yet.';
  }
  const age = relativeAge(progress.lastEventAtUnixMs, now());
  const bits = [
    `Workflow ${progress.name || progress.workflowId} is ${normaliseStatus(progress.status)}.`,
    `${progress.eventCount} event${progress.eventCount === 1 ? '' : 's'} recorded.`,
  ];
  if (age) bits.push(age === 'just now' ? 'Last activity just now.' : `Last activity ${age} ago.`);
  if (progress.summary) bits.push(progress.summary);
  return bits.join(' ');
}

function describeContinueWorkflow(
  continued: RemoteContinueWorkflowResult,
  progress: RemoteWorkflowProgress,
  now: () => number = Date.now,
): string {
  const prefix = continued.accepted
    ? `Continue request accepted: ${continued.message || 'desktop workflow heartbeat sent'}.`
    : `Continue request was not accepted: ${continued.message || 'desktop declined the request'}.`;
  return `${prefix} ${describeWorkflowProgress(progress, now)}`;
}

function assertToolCapability(name: RemotePhoneToolName, capabilities?: string[]): void {
  if (!capabilities) return;
  const definition = REMOTE_PHONE_TOOL_DEFINITIONS.find((tool) => tool.name === name);
  const required = definition?.requiredCapabilities ?? [];
  if (required.some((capability) => capabilities.includes(capability))) return;
  throw new Error(`Remote phone tool '${name}' requires one of: ${required.join(', ')}`);
}

async function resolveWorkflowProgress(
  host: RemoteHost,
  workflowId?: string,
): Promise<RemoteWorkflowProgress | null> {
  const selected = await selectWorkflow(host, workflowId, true);
  if (!selected) return null;
  return host.getWorkflowProgress(selected.workflowId).catch(() => workflowRunToProgress(selected));
}

async function selectWorkflow(
  host: RemoteHost,
  workflowId: string | undefined,
  includeFinishedFallback: boolean,
): Promise<RemoteWorkflowRun | null> {
  if (workflowId) {
    const progress = await host.getWorkflowProgress(workflowId);
    return progress;
  }
  const pending = await host.listWorkflowRuns(false);
  const active = mostRecentWorkflow(pending);
  if (active || !includeFinishedFallback) return active;
  return mostRecentWorkflow(await host.listWorkflowRuns(true));
}

function mostRecentWorkflow(runs: RemoteWorkflowRun[]): RemoteWorkflowRun | null {
  if (runs.length === 0) return null;
  return [...runs].sort((a, b) => b.lastEventAtUnixMs - a.lastEventAtUnixMs)[0];
}

function workflowRunToProgress(run: RemoteWorkflowRun): RemoteWorkflowProgress {
  return { ...run, summary: `${run.eventCount} workflow event${run.eventCount === 1 ? '' : 's'} recorded.` };
}

function asOptionalString(value: unknown): string | undefined {
  return typeof value === 'string' && value.trim() ? value.trim() : undefined;
}

function extractWorkflowId(input: string): string | null {
  const explicit = input.match(/\bworkflow\s+([a-z0-9_-]{6,64})\b/i)?.[1];
  if (explicit) return explicit;
  return input.match(/\b[a-f0-9]{32}\b/i)?.[0] ?? null;
}

function normaliseStatus(status: string): string {
  return status.replace(/_/g, ' ').toLowerCase();
}

function relativeAge(timestamp: number, nowMs: number): string {
  const eventMs = timestamp > 0 && timestamp < 10_000_000_000 ? timestamp * 1000 : timestamp;
  if (!eventMs) return '';
  const delta = Math.max(0, nowMs - eventMs);
  if (delta < 1_000) return 'just now';
  const seconds = Math.floor(delta / 1_000);
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  return `${hours}h`;
}