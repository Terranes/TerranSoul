<template>
  <section
    class="maw-panel"
    aria-labelledby="maw-panel-title"
  >
    <header class="maw-panel-header">
      <div class="maw-panel-title-row">
        <span
          class="maw-panel-icon"
          aria-hidden="true"
        >🤖</span>
        <h2
          id="maw-panel-title"
          class="maw-panel-title"
        >
          Multi-Agent Workflows
        </h2>
        <span
          class="maw-pill"
          :class="{ 'maw-pill-active': activeCount > 0 }"
        >
          {{ activeCount }} active
        </span>
      </div>
      <p class="maw-panel-sub">
        Coordinate Planner, Coder, Reviewer, Tester, Researcher, and Orchestrator
        agents through YAML workflow plans. Schedule recurring tasks like
        Microsoft Teams calendar.
      </p>

      <nav
        class="maw-tabs"
        role="tablist"
      >
        <button
          v-for="tab in tabs"
          :key="tab.id"
          role="tab"
          :aria-selected="activeTab === tab.id"
          :class="['maw-tab', { 'maw-tab-active': activeTab === tab.id }]"
          @click="activeTab = tab.id"
        >
          <span class="maw-tab-icon">{{ tab.icon }}</span>
          {{ tab.label }}
        </button>
      </nav>
    </header>

    <!-- WORKFLOWS TAB -->
    <div
      v-if="activeTab === 'workflows'"
      class="maw-tab-body"
    >
      <div class="maw-toolbar">
        <button
          class="maw-btn maw-btn-primary"
          @click="showNewPlanForm = !showNewPlanForm"
        >
          + New Workflow
        </button>
        <button
          class="maw-btn"
          :disabled="store.loading"
          @click="store.loadPlans()"
        >
          ↻ Refresh
        </button>
      </div>

      <!-- New plan form -->
      <div
        v-if="showNewPlanForm"
        class="maw-card maw-new-form"
      >
        <h3>Create new workflow</h3>
        <label class="maw-field">
          <span>What do you want done?</span>
          <textarea
            v-model="newPlanRequest"
            rows="3"
            placeholder="e.g. Refactor the auth module to use OAuth2"
          />
        </label>
        <label class="maw-field">
          <span>Workflow kind</span>
          <select v-model="newPlanKind">
            <option value="coding">Coding (writes code)</option>
            <option value="daily">Daily (recurring task)</option>
            <option value="one_time">One-time</option>
          </select>
        </label>
        <div class="maw-form-actions">
          <button
            class="maw-btn maw-btn-primary"
            :disabled="!newPlanRequest.trim()"
            @click="createNewPlan"
          >
            Create
          </button>
          <button
            class="maw-btn"
            @click="showNewPlanForm = false"
          >
            Cancel
          </button>
        </div>
      </div>

      <!-- Plans list -->
      <div
        v-if="store.plans.length === 0 && !store.loading"
        class="maw-empty"
      >
        No workflow plans yet. Create one to get started.
      </div>
      <ul
        v-else
        class="maw-plans-list"
      >
        <li
          v-for="plan in store.plans"
          :key="plan.id"
          :class="['maw-plan-card', { 'maw-plan-active': store.activePlan?.id === plan.id }]"
          @click="selectPlan(plan.id)"
        >
          <div class="maw-plan-header">
            <span
              class="maw-kind-badge"
              :class="`maw-kind-${plan.kind}`"
            >{{ kindLabel(plan.kind) }}</span>
            <span class="maw-plan-title">{{ plan.title }}</span>
            <span
              class="maw-status-dot"
              :style="{ background: statusColor(plan.status) }"
            />
          </div>
          <div class="maw-plan-meta">
            <span>{{ plan.completed_steps }}/{{ plan.step_count }} steps</span>
            <span
              v-if="plan.recurring"
              class="maw-recurring-badge"
            >↻ recurring</span>
            <span
              v-if="plan.next_occurrence"
              class="maw-next-occurrence"
            >
              Next: {{ formatDateTime(plan.next_occurrence) }}
            </span>
          </div>
          <div
            v-if="plan.tags.length > 0"
            class="maw-tags"
          >
            <span
              v-for="t in plan.tags"
              :key="t"
              class="maw-tag"
            >#{{ t }}</span>
          </div>
        </li>
      </ul>

      <!-- Active plan editor -->
      <div
        v-if="store.activePlan"
        class="maw-card maw-plan-editor"
      >
        <header class="maw-editor-header">
          <h3>{{ store.activePlan.title }}</h3>
          <div class="maw-editor-actions">
            <button
              class="maw-btn maw-btn-danger-outline"
              @click="onDeletePlan"
            >
              Delete
            </button>
          </div>
        </header>
        <p class="maw-plan-request">
          <em>"{{ store.activePlan.user_request }}"</em>
        </p>

        <h4>Steps</h4>
        <ol class="maw-steps">
          <li
            v-for="step in store.activePlan.steps"
            :key="step.id"
            class="maw-step"
          >
            <div class="maw-step-row">
              <span class="maw-agent-badge">
                {{ agentIcon(step.agent) }} {{ agentLabel(step.agent) }}
              </span>
              <span class="maw-step-id">{{ step.id }}</span>
              <span
                class="maw-status-pill"
                :style="{ background: statusColor(step.status) + '22', color: statusColor(step.status) }"
              >
                {{ stepStatusLabel(step.status) }}
              </span>
              <span
                v-if="step.requires_approval"
                class="maw-approval-flag"
              >⚠ approval</span>
            </div>
            <p class="maw-step-desc">
              {{ step.description }}
            </p>
            <div class="maw-step-llm">
              <label>
                <span>LLM:</span>
                <select
                  :value="`${step.llm_provider}/${step.llm_model}`"
                  @change="onChangeStepLlm(step, ($event.target as HTMLSelectElement).value)"
                >
                  <optgroup
                    v-for="rec in store.recommendations.find(r => r.role === step.agent)?.recommendations || []"
                    :key="`${step.agent}-${rec.model}`"
                    :label="rec.tier"
                  >
                    <option :value="`${rec.provider}/${rec.model}`">
                      {{ rec.model }} ({{ rec.provider }}) — {{ rec.reason }}
                    </option>
                  </optgroup>
                </select>
              </label>
            </div>
            <div
              v-if="step.depends_on.length > 0"
              class="maw-step-deps"
            >
              Depends on: <code
                v-for="d in step.depends_on"
                :key="d"
              >{{ d }}</code>
            </div>
            <details
              v-if="step.output"
              class="maw-step-output"
            >
              <summary>Output</summary>
              <pre>{{ step.output }}</pre>
            </details>
            <div
              v-if="step.error"
              class="maw-step-error"
            >
              {{ step.error }}
            </div>
          </li>
        </ol>

        <!-- Schedule editor -->
        <details class="maw-schedule-editor">
          <summary>{{ store.activePlan.schedule ? 'Edit schedule' : 'Add schedule (recurring)' }}</summary>
          <ScheduleEditor
            :plan="store.activePlan"
            @save="onSaveSchedule"
          />
        </details>
      </div>
    </div>

    <!-- CALENDAR TAB -->
    <div
      v-else-if="activeTab === 'calendar'"
      class="maw-tab-body"
    >
      <WorkflowCalendar />
    </div>

    <!-- AGENTS TAB -->
    <div
      v-else-if="activeTab === 'agents'"
      class="maw-tab-body"
    >
      <div class="maw-roles-grid">
        <article
          v-for="role in store.recommendations"
          :key="role.role"
          class="maw-role-card"
        >
          <header class="maw-role-header">
            <span class="maw-role-icon">{{ agentIcon(role.role) }}</span>
            <h3>{{ role.display_name }}</h3>
          </header>
          <ul class="maw-role-recs">
            <li
              v-for="rec in role.recommendations"
              :key="rec.model"
            >
              <span
                class="maw-tier-pill"
                :class="`maw-tier-${rec.tier}`"
              >{{ rec.tier }}</span>
              <strong>{{ rec.model }}</strong>
              <span class="maw-rec-provider">({{ rec.provider }})</span>
              <p class="maw-rec-reason">
                {{ rec.reason }}
              </p>
            </li>
          </ul>
        </article>
      </div>
    </div>

    <p
      v-if="store.error"
      class="maw-error"
    >
      {{ store.error }}
    </p>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import {
  type AgentRole,
  type StepStatus,
  type WorkflowKind,
  type WorkflowPlanStatus,
  type WorkflowSchedule,
  type WorkflowStep,
  agentRoleIcon,
  agentRoleLabel,
  statusBadgeColor,
  useWorkflowPlansStore,
} from '../stores/workflow-plans';
import WorkflowCalendar from './WorkflowCalendar.vue';
import ScheduleEditor from './ScheduleEditor.vue';

const store = useWorkflowPlansStore();

type TabId = 'workflows' | 'calendar' | 'agents';
const tabs: { id: TabId; label: string; icon: string }[] = [
  { id: 'workflows', label: 'Workflows', icon: '📋' },
  { id: 'calendar', label: 'Calendar', icon: '📅' },
  { id: 'agents', label: 'Agents', icon: '🤖' },
];
const activeTab = ref<TabId>('workflows');

const showNewPlanForm = ref(false);
const newPlanRequest = ref('');
const newPlanKind = ref<WorkflowKind>('coding');

const activeCount = computed(
  () => store.plans.filter((p) => p.status === 'running' || p.status === 'paused').length,
);

onMounted(async () => {
  await Promise.all([store.loadPlans(), store.loadRecommendations()]);
});

async function selectPlan(planId: string): Promise<void> {
  await store.loadPlan(planId);
}

async function createNewPlan(): Promise<void> {
  const plan = await store.createBlank(newPlanRequest.value.trim(), newPlanKind.value);
  if (plan) {
    await store.savePlan(plan);
    showNewPlanForm.value = false;
    newPlanRequest.value = '';
  }
}

async function onDeletePlan(): Promise<void> {
  if (!store.activePlan) return;
  if (confirm(`Delete workflow "${store.activePlan.title}"?`)) {
    await store.deletePlan(store.activePlan.id);
  }
}

async function onChangeStepLlm(step: WorkflowStep, value: string): Promise<void> {
  if (!store.activePlan) return;
  const [provider, ...modelParts] = value.split('/');
  const model = modelParts.join('/');
  await store.overrideAgentLlm(store.activePlan.id, step.agent, {
    provider,
    model,
  });
}

async function onSaveSchedule(schedule: WorkflowSchedule | null): Promise<void> {
  if (!store.activePlan) return;
  const updated = { ...store.activePlan, schedule: schedule ?? undefined };
  await store.savePlan(updated);
}

function kindLabel(kind: WorkflowKind): string {
  return { coding: 'Coding', daily: 'Daily', one_time: 'One-time' }[kind];
}

function agentLabel(role: AgentRole): string {
  return agentRoleLabel(role);
}

function agentIcon(role: AgentRole): string {
  return agentRoleIcon(role);
}

function statusColor(status: WorkflowPlanStatus | StepStatus): string {
  return statusBadgeColor(status);
}

function stepStatusLabel(status: StepStatus): string {
  return status.replace(/_/g, ' ');
}

function formatDateTime(epochMs: number): string {
  const d = new Date(epochMs);
  return d.toLocaleString(undefined, {
    month: 'short',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
  });
}
</script>

<style scoped>
.maw-panel {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  padding: 1rem;
  background: var(--ts-surface, #1a1a1a);
  color: var(--ts-text, #e5e7eb);
  border-radius: 0.75rem;
  min-height: 600px;
}
.maw-panel-header {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  border-bottom: 1px solid var(--ts-border, #333);
  padding-bottom: 0.75rem;
}
.maw-panel-title-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}
.maw-panel-icon { font-size: 1.5rem; }
.maw-panel-title { margin: 0; font-size: 1.25rem; }
.maw-panel-sub {
  margin: 0;
  font-size: 0.875rem;
  color: var(--ts-text-muted);
}
.maw-pill {
  margin-left: auto;
  padding: 0.125rem 0.5rem;
  border-radius: 999px;
  background: var(--ts-text-muted);
  color: var(--ts-surface);
  font-size: 0.75rem;
}
.maw-pill-active { background: var(--ts-success); }

.maw-tabs {
  display: flex;
  gap: 0.25rem;
}
.maw-tab {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.5rem 0.875rem;
  background: transparent;
  border: 1px solid var(--ts-border, #333);
  border-radius: 0.375rem;
  color: var(--ts-text-muted);
  cursor: pointer;
  font-size: 0.875rem;
}
.maw-tab:hover { color: var(--ts-text); }
.maw-tab-active {
  background: var(--ts-accent-glow);
  color: var(--ts-accent);
  border-color: var(--ts-accent);
}

.maw-tab-body { display: flex; flex-direction: column; gap: 0.75rem; }
.maw-toolbar { display: flex; gap: 0.5rem; }
.maw-btn {
  padding: 0.375rem 0.75rem;
  background: transparent;
  border: 1px solid var(--ts-border, #333);
  border-radius: 0.375rem;
  color: var(--ts-text);
  cursor: pointer;
  font-size: 0.875rem;
}
.maw-btn:hover:not(:disabled) { background: var(--ts-surface-hover, #222); }
.maw-btn:disabled { opacity: 0.5; cursor: not-allowed; }
.maw-btn-primary {
  background: var(--ts-accent);
  border-color: var(--ts-accent);
  color: white;
}
.maw-btn-primary:hover:not(:disabled) { background: var(--ts-accent-hover); }
.maw-btn-danger-outline {
  border-color: var(--ts-error);
  color: var(--ts-error);
}

.maw-card {
  background: var(--ts-surface-2, #222);
  border: 1px solid var(--ts-border, #333);
  border-radius: 0.5rem;
  padding: 0.875rem;
}
.maw-new-form { display: flex; flex-direction: column; gap: 0.5rem; }
.maw-field { display: flex; flex-direction: column; gap: 0.25rem; font-size: 0.875rem; }
.maw-field textarea, .maw-field select, .maw-field input {
  padding: 0.375rem 0.5rem;
  background: var(--ts-surface, #1a1a1a);
  border: 1px solid var(--ts-border, #333);
  border-radius: 0.25rem;
  color: var(--ts-text);
  font-family: inherit;
}
.maw-form-actions { display: flex; gap: 0.5rem; }

.maw-empty {
  padding: 2rem;
  text-align: center;
  color: var(--ts-text-muted);
  font-style: italic;
}

.maw-plans-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}
.maw-plan-card {
  padding: 0.75rem;
  background: var(--ts-surface-2, #222);
  border: 1px solid var(--ts-border, #333);
  border-radius: 0.5rem;
  cursor: pointer;
  transition: border-color 0.15s;
}
.maw-plan-card:hover { border-color: var(--ts-accent); }
.maw-plan-active { border-color: var(--ts-accent); background: var(--ts-accent-glow); }
.maw-plan-header { display: flex; align-items: center; gap: 0.5rem; }
.maw-plan-title { font-weight: 600; flex: 1; }
.maw-status-dot { width: 0.625rem; height: 0.625rem; border-radius: 50%; }
.maw-plan-meta {
  display: flex;
  gap: 0.75rem;
  margin-top: 0.25rem;
  font-size: 0.75rem;
  color: var(--ts-text-muted);
}
.maw-recurring-badge { color: var(--ts-info); }
.maw-tags { margin-top: 0.25rem; display: flex; gap: 0.25rem; flex-wrap: wrap; }
.maw-tag {
  font-size: 0.75rem;
  padding: 0.0625rem 0.375rem;
  background: var(--ts-accent-glow);
  border-radius: 0.25rem;
}

.maw-kind-badge {
  font-size: 0.6875rem;
  padding: 0.125rem 0.375rem;
  border-radius: 0.25rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  font-weight: 600;
}
.maw-kind-coding { background: rgba(96, 165, 250, 0.2); color: var(--ts-accent-blue); }
.maw-kind-daily { background: rgba(52, 211, 153, 0.2); color: var(--ts-success); }
.maw-kind-one_time { background: rgba(167, 139, 250, 0.2); color: var(--ts-accent-violet); }

.maw-plan-editor { margin-top: 0.5rem; }
.maw-editor-header { display: flex; justify-content: space-between; align-items: center; }
.maw-plan-request {
  color: var(--ts-text-muted);
  font-size: 0.875rem;
  margin: 0.25rem 0 0.75rem;
}

.maw-steps { padding-left: 1.5rem; display: flex; flex-direction: column; gap: 0.625rem; }
.maw-step {
  background: var(--ts-surface, #1a1a1a);
  border: 1px solid var(--ts-border, #333);
  border-radius: 0.375rem;
  padding: 0.625rem;
}
.maw-step-row { display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; }
.maw-agent-badge {
  font-size: 0.8125rem;
  font-weight: 600;
  padding: 0.125rem 0.5rem;
  border-radius: 0.25rem;
  background: var(--ts-accent-glow);
  color: var(--ts-accent);
}
.maw-step-id {
  font-family: monospace;
  font-size: 0.75rem;
  color: var(--ts-text-muted);
}
.maw-status-pill {
  font-size: 0.6875rem;
  padding: 0.0625rem 0.375rem;
  border-radius: 0.25rem;
  font-weight: 600;
  text-transform: uppercase;
}
.maw-approval-flag {
  font-size: 0.6875rem;
  color: var(--ts-warning);
}
.maw-step-desc { margin: 0.375rem 0; font-size: 0.875rem; }
.maw-step-llm { font-size: 0.8125rem; }
.maw-step-llm select { padding: 0.25rem; }
.maw-step-deps {
  font-size: 0.75rem;
  color: var(--ts-text-muted);
  margin-top: 0.25rem;
}
.maw-step-deps code {
  margin-right: 0.25rem;
  padding: 0 0.25rem;
  background: var(--ts-surface-2, #222);
  border-radius: 0.125rem;
}
.maw-step-output { margin-top: 0.5rem; font-size: 0.8125rem; }
.maw-step-output pre {
  max-height: 200px;
  overflow: auto;
  padding: 0.5rem;
  background: var(--ts-surface-2);
  border-radius: 0.25rem;
  font-size: 0.75rem;
}
.maw-step-error {
  margin-top: 0.5rem;
  padding: 0.375rem;
  background: var(--ts-error-bg);
  color: var(--ts-error);
  border-radius: 0.25rem;
  font-size: 0.8125rem;
}

.maw-schedule-editor {
  margin-top: 1rem;
  padding: 0.5rem;
  background: var(--ts-surface, #1a1a1a);
  border-radius: 0.375rem;
}
.maw-schedule-editor summary {
  cursor: pointer;
  font-weight: 600;
  color: var(--ts-accent);
}

.maw-roles-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 0.75rem;
}
.maw-role-card {
  background: var(--ts-surface-2, #222);
  border: 1px solid var(--ts-border, #333);
  border-radius: 0.5rem;
  padding: 0.875rem;
}
.maw-role-header { display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.5rem; }
.maw-role-icon { font-size: 1.5rem; }
.maw-role-card h3 { margin: 0; font-size: 1rem; }
.maw-role-recs {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}
.maw-role-recs li {
  padding: 0.5rem;
  background: var(--ts-surface, #1a1a1a);
  border-radius: 0.25rem;
}
.maw-tier-pill {
  display: inline-block;
  font-size: 0.6875rem;
  padding: 0.0625rem 0.375rem;
  border-radius: 0.25rem;
  font-weight: 600;
  text-transform: uppercase;
  margin-right: 0.375rem;
}
.maw-tier-fast { background: rgba(96, 165, 250, 0.2); color: var(--ts-accent-blue); }
.maw-tier-balanced { background: rgba(167, 139, 250, 0.2); color: var(--ts-accent-violet); }
.maw-tier-premium { background: rgba(251, 191, 36, 0.2); color: var(--ts-warning); }
.maw-rec-provider {
  font-size: 0.75rem;
  color: var(--ts-text-muted);
  margin-left: 0.25rem;
}
.maw-rec-reason {
  margin: 0.25rem 0 0;
  font-size: 0.75rem;
  color: var(--ts-text-muted);
}

.maw-error {
  padding: 0.5rem;
  background: var(--ts-error-bg);
  color: var(--ts-error);
  border-radius: 0.375rem;
  font-size: 0.875rem;
}
</style>
