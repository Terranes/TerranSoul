import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

/**
 * Multi-agent workflow plans store (Chunk 30.3).
 *
 * Surfaces YAML-backed workflow plans, per-agent LLM overrides, calendar
 * projection of scheduled occurrences (MS Teams-style recurrence), and
 * agent-role recommendations.
 *
 * All persistence happens through Tauri commands backed by
 * `<data_dir>/workflow_plans/*.yaml`.
 */

// ---------------------------------------------------------------------------
// Types (mirrors of Rust types in src-tauri/src/coding/multi_agent.rs)
// ---------------------------------------------------------------------------

export type AgentRole =
  | 'planner'
  | 'coder'
  | 'reviewer'
  | 'tester'
  | 'researcher'
  | 'orchestrator';

export type WorkflowKind = 'coding' | 'daily' | 'one_time';

export type WorkflowPlanStatus =
  | 'pending_review'
  | 'approved'
  | 'running'
  | 'paused'
  | 'completed'
  | 'failed'
  | 'cancelled';

export type StepStatus =
  | 'pending'
  | 'awaiting_approval'
  | 'running'
  | 'completed'
  | 'failed'
  | 'skipped';

export type StepOutputFormat =
  | 'prose'
  | 'code'
  | 'json'
  | 'plan'
  | 'test_results'
  | 'verdict';

export type Weekday =
  | 'sunday'
  | 'monday'
  | 'tuesday'
  | 'wednesday'
  | 'thursday'
  | 'friday'
  | 'saturday';

export type RecurrencePattern =
  | { kind: 'once' }
  | { kind: 'daily'; interval: number }
  | { kind: 'weekly'; interval: number; weekdays: Weekday[] }
  | { kind: 'monthly'; interval: number; day_of_month: number };

export interface WorkflowSchedule {
  start_at: number;
  end_at?: number;
  duration_minutes: number;
  recurrence: RecurrencePattern;
  timezone: string;
  last_fired_at?: number;
}

export interface AgentLlmConfig {
  model: string;
  provider: string;
  api_key?: string;
  base_url?: string;
}

export interface WorkflowStep {
  id: string;
  agent: AgentRole;
  llm_model: string;
  llm_provider: string;
  description: string;
  depends_on: string[];
  output_format: StepOutputFormat;
  status: StepStatus;
  output?: string;
  error?: string;
  duration_ms: number;
  requires_approval: boolean;
}

export interface WorkflowPlan {
  id: string;
  title: string;
  kind: WorkflowKind;
  status: WorkflowPlanStatus;
  user_request: string;
  steps: WorkflowStep[];
  agent_llm_overrides: Partial<Record<AgentRole, AgentLlmConfig>>;
  created_at: number;
  updated_at: number;
  tags: string[];
  schedule?: WorkflowSchedule;
}

export interface WorkflowPlanSummary {
  id: string;
  title: string;
  kind: WorkflowKind;
  status: WorkflowPlanStatus;
  step_count: number;
  completed_steps: number;
  created_at: number;
  updated_at: number;
  tags: string[];
  next_occurrence?: number;
  recurring: boolean;
}

export interface CalendarEvent {
  workflow_id: string;
  title: string;
  kind: WorkflowKind;
  start_at: number;
  end_at: number;
  recurring: boolean;
  status: WorkflowPlanStatus;
}

export interface AgentRecommendation {
  model: string;
  provider: string;
  tier: string;
  reason: string;
}

export interface RoleRecommendations {
  role: AgentRole;
  display_name: string;
  recommendations: AgentRecommendation[];
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const useWorkflowPlansStore = defineStore('workflow-plans', () => {
  const plans = ref<WorkflowPlanSummary[]>([]);
  const activePlan = ref<WorkflowPlan | null>(null);
  const calendarEvents = ref<CalendarEvent[]>([]);
  const recommendations = ref<RoleRecommendations[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);

  /** Calendar viewport (ms epoch). Defaults to current week. */
  const calendarRangeStart = ref<number>(startOfWeek(Date.now()));
  const calendarRangeEnd = ref<number>(startOfWeek(Date.now()) + 7 * 86_400_000);

  // -------------------------------------------------------------------------
  // Computed views
  // -------------------------------------------------------------------------

  const plansByKind = computed(() => {
    const out: Record<WorkflowKind, WorkflowPlanSummary[]> = {
      coding: [],
      daily: [],
      one_time: [],
    };
    for (const p of plans.value) {
      out[p.kind].push(p);
    }
    return out;
  });

  const activePlans = computed(() =>
    plans.value.filter((p) => p.status === 'running' || p.status === 'paused'),
  );

  const recurringPlans = computed(() => plans.value.filter((p) => p.recurring));

  /** Calendar events grouped by day (YYYY-MM-DD key). */
  const eventsByDay = computed(() => {
    const out: Record<string, CalendarEvent[]> = {};
    for (const ev of calendarEvents.value) {
      const key = isoDayKey(ev.start_at);
      if (!out[key]) out[key] = [];
      out[key].push(ev);
    }
    return out;
  });

  // -------------------------------------------------------------------------
  // Actions
  // -------------------------------------------------------------------------

  async function loadPlans(): Promise<void> {
    loading.value = true;
    error.value = null;
    try {
      const result = await invoke<WorkflowPlanSummary[]>('workflow_plan_list');
      plans.value = Array.isArray(result) ? result : [];
    } catch (err) {
      error.value = String(err);
      plans.value = [];
    } finally {
      loading.value = false;
    }
  }

  async function loadPlan(planId: string): Promise<void> {
    loading.value = true;
    error.value = null;
    try {
      activePlan.value = await invoke<WorkflowPlan>('workflow_plan_load', { planId });
    } catch (err) {
      error.value = String(err);
      activePlan.value = null;
    } finally {
      loading.value = false;
    }
  }

  async function savePlan(plan: WorkflowPlan): Promise<WorkflowPlan | null> {
    error.value = null;
    try {
      const result = await invoke<WorkflowPlan>('workflow_plan_save', { plan });
      activePlan.value = result;
      await loadPlans();
      return result;
    } catch (err) {
      error.value = String(err);
      return null;
    }
  }

  async function deletePlan(planId: string): Promise<boolean> {
    error.value = null;
    try {
      await invoke('workflow_plan_delete', { planId });
      if (activePlan.value?.id === planId) {
        activePlan.value = null;
      }
      await loadPlans();
      return true;
    } catch (err) {
      error.value = String(err);
      return false;
    }
  }

  async function createBlank(
    userRequest: string,
    kind: WorkflowKind,
  ): Promise<WorkflowPlan | null> {
    error.value = null;
    try {
      const plan = await invoke<WorkflowPlan>('workflow_plan_create_blank', {
        args: { user_request: userRequest, kind },
      });
      activePlan.value = plan;
      return plan;
    } catch (err) {
      error.value = String(err);
      return null;
    }
  }

  async function validatePlan(plan: WorkflowPlan): Promise<string[]> {
    try {
      const errors = await invoke<string[]>('workflow_plan_validate', { plan });
      return Array.isArray(errors) ? errors : [];
    } catch (err) {
      return [String(err)];
    }
  }

  async function updateStep(
    planId: string,
    stepId: string,
    status: StepStatus,
    output?: string,
    err?: string,
  ): Promise<void> {
    error.value = null;
    try {
      const result = await invoke<WorkflowPlan>('workflow_plan_update_step', {
        planId,
        stepId,
        status,
        output: output ?? null,
        error: err ?? null,
      });
      activePlan.value = result;
    } catch (e) {
      error.value = String(e);
    }
  }

  async function overrideAgentLlm(
    planId: string,
    agent: AgentRole,
    config: AgentLlmConfig,
  ): Promise<void> {
    error.value = null;
    try {
      const result = await invoke<WorkflowPlan>('workflow_plan_override_llm', {
        planId,
        agent,
        config,
      });
      activePlan.value = result;
    } catch (e) {
      error.value = String(e);
    }
  }

  async function loadCalendarEvents(fromMs: number, toMs: number): Promise<void> {
    error.value = null;
    calendarRangeStart.value = fromMs;
    calendarRangeEnd.value = toMs;
    try {
      const result = await invoke<CalendarEvent[]>('workflow_calendar_events', {
        fromMs,
        toMs,
      });
      calendarEvents.value = Array.isArray(result) ? result : [];
    } catch (err) {
      error.value = String(err);
      calendarEvents.value = [];
    }
  }

  async function loadRecommendations(): Promise<void> {
    try {
      const result = await invoke<RoleRecommendations[]>('workflow_agent_recommendations');
      recommendations.value = Array.isArray(result) ? result : [];
    } catch (err) {
      error.value = String(err);
      recommendations.value = [];
    }
  }

  function shiftCalendarWeek(weeks: number): void {
    const offset = weeks * 7 * 86_400_000;
    void loadCalendarEvents(
      calendarRangeStart.value + offset,
      calendarRangeEnd.value + offset,
    );
  }

  function jumpCalendarToToday(): void {
    const start = startOfWeek(Date.now());
    void loadCalendarEvents(start, start + 7 * 86_400_000);
  }

  return {
    // state
    plans,
    activePlan,
    calendarEvents,
    recommendations,
    loading,
    error,
    calendarRangeStart,
    calendarRangeEnd,
    // computed
    plansByKind,
    activePlans,
    recurringPlans,
    eventsByDay,
    // actions
    loadPlans,
    loadPlan,
    savePlan,
    deletePlan,
    createBlank,
    validatePlan,
    updateStep,
    overrideAgentLlm,
    loadCalendarEvents,
    loadRecommendations,
    shiftCalendarWeek,
    jumpCalendarToToday,
  };
});

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Local-time start of the calendar week (Sunday 00:00) for a given epoch ms. */
export function startOfWeek(epochMs: number): number {
  const d = new Date(epochMs);
  d.setHours(0, 0, 0, 0);
  d.setDate(d.getDate() - d.getDay()); // back to Sunday
  return d.getTime();
}

/** YYYY-MM-DD key in local time. */
export function isoDayKey(epochMs: number): string {
  const d = new Date(epochMs);
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
}

/** Format a recurrence pattern for human-readable display. */
export function formatRecurrence(pattern: RecurrencePattern): string {
  switch (pattern.kind) {
    case 'once':
      return 'One time';
    case 'daily':
      return pattern.interval === 1 ? 'Every day' : `Every ${pattern.interval} days`;
    case 'weekly': {
      const days = pattern.weekdays.map(weekdayShort).join(', ');
      const prefix = pattern.interval === 1 ? 'Weekly' : `Every ${pattern.interval} weeks`;
      return `${prefix} on ${days || '—'}`;
    }
    case 'monthly':
      return pattern.interval === 1
        ? `Monthly on day ${pattern.day_of_month}`
        : `Every ${pattern.interval} months on day ${pattern.day_of_month}`;
  }
}

export function weekdayShort(w: Weekday): string {
  const map: Record<Weekday, string> = {
    sunday: 'Sun',
    monday: 'Mon',
    tuesday: 'Tue',
    wednesday: 'Wed',
    thursday: 'Thu',
    friday: 'Fri',
    saturday: 'Sat',
  };
  return map[w];
}

export function agentRoleLabel(role: AgentRole): string {
  const map: Record<AgentRole, string> = {
    planner: 'Planner',
    coder: 'Coder',
    reviewer: 'Reviewer',
    tester: 'Tester',
    researcher: 'Researcher',
    orchestrator: 'Orchestrator',
  };
  return map[role];
}

export function agentRoleIcon(role: AgentRole): string {
  const map: Record<AgentRole, string> = {
    planner: '🗺️',
    coder: '⌨️',
    reviewer: '🔍',
    tester: '🧪',
    researcher: '📚',
    orchestrator: '🎯',
  };
  return map[role];
}

export function statusBadgeColor(status: WorkflowPlanStatus | StepStatus): string {
  switch (status) {
    case 'pending':
    case 'pending_review':
      return 'var(--ts-text-muted)';
    case 'awaiting_approval':
    case 'paused':
      return 'var(--ts-warning)';
    case 'approved':
    case 'running':
      return 'var(--ts-info)';
    case 'completed':
      return 'var(--ts-success)';
    case 'failed':
      return 'var(--ts-error)';
    case 'cancelled':
    case 'skipped':
      return 'var(--ts-text-muted)';
    default:
      return 'var(--ts-text-muted)';
  }
}
