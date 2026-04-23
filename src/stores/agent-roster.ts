/**
 * Multi-agent roster store — Chunk 1.5.
 *
 * Tracks the user's agent roster, the currently active agent, and any
 * in-flight Temporal-style durable workflows started by external-CLI
 * agents (Codex / Claude / Gemini).
 *
 * Browser fallback: when the Tauri backend is unavailable the store
 * degrades to a single in-memory "default" agent so the UI remains
 * functional for the web preview. Persistence is skipped in that mode.
 */

import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';

function isTauriAvailable(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

// ── Types (mirror of the Rust structs) ────────────────────────────────

export type CliKind = 'codex' | 'claude' | 'gemini' | 'custom';

export type BrainBackend =
  | { kind: 'native'; data: { mode: null | Record<string, unknown> } }
  | {
      kind: 'external_cli';
      data: { kind: CliKind; binary: string; extra_args: string[] };
    };

export interface AgentProfile {
  id: string;
  display_name: string;
  vrm_model_id: string;
  brain_backend: BrainBackend;
  working_folder: string | null;
  created_at: number;
  last_active_at: number;
}

export type AgentBackendKind = 'native_api' | 'local_ollama' | 'external_cli';

export interface AgentFootprint {
  agent_id: string;
  kind: AgentBackendKind;
  estimated_mb: number;
}

export interface RamCap {
  free_mb: number;
  reserve_mb: number;
  cap: number;
  mean_per_agent_mb: number;
  reasoning: string;
}

export interface AgentRamCapReport {
  cap: RamCap;
  footprints: AgentFootprint[];
}

export type WorkflowStatus =
  | 'running'
  | 'completed'
  | 'failed'
  | 'cancelled'
  | 'resuming';

export interface WorkflowSummary {
  workflow_id: string;
  name: string;
  status: WorkflowStatus;
  started_at: number;
  last_event_at: number;
  event_count: number;
}

// ── Store ────────────────────────────────────────────────────────────

export const useAgentRosterStore = defineStore('agent-roster', () => {
  const agents = ref<AgentProfile[]>([]);
  const currentAgentId = ref<string | null>(null);
  const ramCap = ref<AgentRamCapReport | null>(null);
  const workflows = ref<WorkflowSummary[]>([]);
  const loading = ref(false);
  const lastError = ref<string | null>(null);

  const currentAgent = computed<AgentProfile | null>(() => {
    if (!currentAgentId.value) {
      return agents.value[0] ?? null;
    }
    return agents.value.find((a) => a.id === currentAgentId.value) ?? null;
  });

  /** Number of workflows currently in a non-terminal state. */
  const activeWorkflowCount = computed(
    () =>
      workflows.value.filter(
        (w) => w.status === 'running' || w.status === 'resuming',
      ).length,
  );

  /** True when activating another agent would breach the RAM cap. */
  const atRamCap = computed(
    () => ramCap.value !== null && activeWorkflowCount.value >= ramCap.value.cap.cap,
  );

  async function refresh() {
    if (!isTauriAvailable()) {
      // Browser fallback: single default agent, no RAM cap, no workflows.
      if (agents.value.length === 0) {
        agents.value = [
          {
            id: 'default',
            display_name: 'TerranSoul',
            vrm_model_id: 'annabelle',
            brain_backend: { kind: 'native', data: { mode: null } },
            working_folder: null,
            created_at: 0,
            last_active_at: 0,
          },
        ];
        currentAgentId.value = 'default';
      }
      return;
    }
    loading.value = true;
    lastError.value = null;
    try {
      const [list, current, cap, pending] = await Promise.all([
        invoke<AgentProfile[]>('roster_list'),
        invoke<string | null>('roster_get_current'),
        invoke<AgentRamCapReport>('roster_get_ram_cap'),
        invoke<WorkflowSummary[]>('roster_list_workflows'),
      ]);
      agents.value = list;
      currentAgentId.value = current ?? list[0]?.id ?? null;
      ramCap.value = cap;
      workflows.value = pending;
    } catch (e) {
      lastError.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function createAgent(
    displayName: string,
    vrmModelId: string,
    brainBackend: BrainBackend,
    workingFolder: string | null = null,
  ): Promise<AgentProfile | null> {
    if (!isTauriAvailable()) {
      lastError.value = 'Agent creation requires the desktop app';
      return null;
    }
    lastError.value = null;
    try {
      const agent = await invoke<AgentProfile>('roster_create', {
        request: {
          display_name: displayName,
          vrm_model_id: vrmModelId,
          brain_backend: brainBackend,
          working_folder: workingFolder,
        },
      });
      await refresh();
      return agent;
    } catch (e) {
      lastError.value = String(e);
      return null;
    }
  }

  async function deleteAgent(id: string): Promise<boolean> {
    if (!isTauriAvailable()) return false;
    try {
      await invoke('roster_delete', { id });
      await refresh();
      return true;
    } catch (e) {
      lastError.value = String(e);
      return false;
    }
  }

  async function switchAgent(id: string): Promise<boolean> {
    if (!isTauriAvailable()) {
      currentAgentId.value = id;
      return true;
    }
    try {
      const agent = await invoke<AgentProfile>('roster_switch', { id });
      currentAgentId.value = agent.id;
      await refresh();
      return true;
    } catch (e) {
      lastError.value = String(e);
      return false;
    }
  }

  async function setWorkingFolder(id: string, folder: string): Promise<boolean> {
    if (!isTauriAvailable()) return false;
    try {
      await invoke('roster_set_working_folder', { id, folder });
      await refresh();
      return true;
    } catch (e) {
      lastError.value = String(e);
      return false;
    }
  }

  async function startCliWorkflow(
    agentId: string,
    prompt: string,
  ): Promise<string | null> {
    if (!isTauriAvailable()) return null;
    lastError.value = null;
    try {
      const res = await invoke<{ workflow_id: string }>(
        'roster_start_cli_workflow',
        {
          request: { agent_id: agentId, prompt },
        },
      );
      await refresh();
      return res.workflow_id;
    } catch (e) {
      lastError.value = String(e);
      return null;
    }
  }

  async function cancelWorkflow(workflowId: string, reason?: string): Promise<boolean> {
    if (!isTauriAvailable()) return false;
    try {
      await invoke('roster_cancel_workflow', { workflowId, reason });
      await refresh();
      return true;
    } catch (e) {
      lastError.value = String(e);
      return false;
    }
  }

  async function refreshRamCap() {
    if (!isTauriAvailable()) return;
    try {
      ramCap.value = await invoke<AgentRamCapReport>('roster_get_ram_cap');
    } catch (e) {
      lastError.value = String(e);
    }
  }

  return {
    // state
    agents,
    currentAgentId,
    currentAgent,
    ramCap,
    workflows,
    loading,
    lastError,
    // derived
    activeWorkflowCount,
    atRamCap,
    // actions
    refresh,
    refreshRamCap,
    createAgent,
    deleteAgent,
    switchAgent,
    setWorkingFolder,
    startCliWorkflow,
    cancelWorkflow,
  };
});
