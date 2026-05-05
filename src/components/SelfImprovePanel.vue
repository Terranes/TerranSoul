<template>
  <section
    class="si-panel"
    aria-labelledby="si-panel-title"
  >
    <header class="si-panel-header">
      <div class="si-panel-title-row">
        <span
          class="si-panel-icon"
          aria-hidden="true"
        >🛠️</span>
        <h2
          id="si-panel-title"
          class="si-panel-title"
        >
          Self-Improve
        </h2>
        <span
          class="si-status-pill"
          :class="statusPillClass"
        >
          {{ statusPillLabel }}
        </span>
      </div>
      <p class="si-panel-sub">
        Autonomous coding loop that drives <code>rules/milestones.md</code>
        through your dedicated Coding LLM.
      </p>
    </header>

    <!-- Progress overview -->
    <div class="si-progress-card">
      <div class="si-progress-top">
        <span class="si-progress-label">
          {{ store.completedCount }} / {{ store.totalCount }} phases complete
        </span>
        <span class="si-progress-percent">{{ store.progressPercent }}%</span>
      </div>
      <div
        class="si-progress-track"
        role="progressbar"
        :aria-valuenow="store.progressPercent"
        aria-valuemin="0"
        aria-valuemax="100"
      >
        <div
          class="si-progress-fill"
          :style="{ width: `${store.progressPercent}%` }"
        />
      </div>
      <p
        v-if="store.nextPhase"
        class="si-next-line"
      >
        <strong>Next:</strong> {{ store.nextPhase.title }}
      </p>
      <p
        v-else
        class="si-next-line si-next-done"
      >
        ✓ All phases complete
      </p>
    </div>

    <!-- Self-improve queue dashboard -->
    <div
      class="si-queue"
      aria-label="Self-improve work tracker"
    >
      <article class="si-queue-lane si-queue-lane--done">
        <header class="si-queue-head">
          <span>Finished</span>
          <strong>{{ finishedPhases.length }}</strong>
        </header>
        <ul class="si-queue-list">
          <li
            v-for="phase in finishedPhases.slice(0, 4)"
            :key="phase.id"
          >
            ✓ {{ phase.title }}
          </li>
          <li
            v-if="finishedPhases.length === 0"
            class="si-queue-empty"
          >
            Nothing finished yet
          </li>
        </ul>
      </article>
      <article class="si-queue-lane si-queue-lane--active">
        <header class="si-queue-head">
          <span>Working on</span>
          <strong>{{ workingPhases.length }}</strong>
        </header>
        <ul class="si-queue-list">
          <li
            v-for="phase in workingPhases"
            :key="phase.id"
          >
            ◐ {{ phase.title }}
          </li>
          <li
            v-if="workingPhases.length === 0"
            class="si-queue-empty"
          >
            Waiting for the next chunk
          </li>
        </ul>
      </article>
      <article class="si-queue-lane si-queue-lane--backlog">
        <header class="si-queue-head">
          <span>Backlog</span>
          <strong>{{ backlogCount }}</strong>
        </header>
        <ul class="si-queue-list">
          <li
            v-for="phase in backlogPreview"
            :key="phase.id"
            class="si-queue-row"
          >
            <span class="si-queue-row-title">○ {{ phase.title }}</span>
            <button
              v-if="isPromotable(phase)"
              type="button"
              class="si-promote-btn"
              :title="`Promote to milestone chunk: ${phase.title}`"
              @click="onPromoteItem(phase.title, backlogItemGoal(phase))"
            >
              ↑ Promote
            </button>
          </li>
          <li
            v-if="backlogPreview.length === 0"
            class="si-queue-empty"
          >
            No queued phases
          </li>
        </ul>
      </article>
    </div>

    <!-- Coding workflow UX -->
    <div class="si-workflow">
      <h3 class="si-section-h">
        Coding workflow
      </h3>
      <ol class="si-workflow-list">
        <li
          v-for="step in workflowSteps"
          :key="step.label"
          class="si-workflow-step"
          :class="{ 'si-workflow-step--active': step.active }"
        >
          <span class="si-workflow-index">{{ step.index }}</span>
          <div>
            <strong>{{ step.label }}</strong>
            <p>{{ step.detail }}</p>
          </div>
        </li>
      </ol>
    </div>

    <!-- Phase roadmap -->
    <div class="si-roadmap">
      <h3 class="si-section-h">
        Roadmap
      </h3>
      <ul class="si-phase-list">
        <li
          v-for="(p, idx) in store.phases"
          :key="p.id"
          :class="['si-phase', `si-phase--${p.status}`]"
        >
          <span class="si-phase-num">{{ idx + 1 }}</span>
          <span
            class="si-phase-icon"
            :title="phaseStatusLabel(p.status)"
            :aria-label="phaseStatusLabel(p.status)"
          >
            {{ phaseStatusIcon(p.status) }}
          </span>
          <div class="si-phase-content">
            <div class="si-phase-title">
              {{ p.title }}
            </div>
            <div class="si-phase-desc">
              {{ p.description }}
            </div>
            <div
              v-if="p.blockedReason && p.status !== 'completed'"
              class="si-phase-blocked"
            >
              ⛔ {{ p.blockedReason }}
            </div>
          </div>
        </li>
      </ul>
    </div>

    <!-- Research-backed improvement chunks -->
    <div class="si-improvement">
      <h3 class="si-section-h">
        Improvement chunks
      </h3>
      <p class="si-improvement-desc">
        Runtime failures and new ideas become scoped chunks. The loop should
        verify bugs, research better current approaches online, and review
        Redis/model/tool/API options before implementation.
      </p>
      <ul class="si-improvement-list">
        <li
          v-for="chunk in store.improvementChunks"
          :key="chunk.id"
          class="si-improvement-item"
          :class="`si-improvement-item--${chunk.priority}`"
        >
          <div class="si-improvement-top">
            <span class="si-improvement-priority">{{ chunk.priority }}</span>
            <span class="si-improvement-status">{{ chunk.status }}</span>
          </div>
          <div class="si-improvement-title">
            {{ chunk.title }}
          </div>
          <div class="si-improvement-body">
            {{ chunk.description }}
          </div>
          <div class="si-improvement-trigger">
            Trigger: {{ chunk.trigger }}
          </div>
          <div class="si-improvement-actions">
            <button
              type="button"
              class="si-promote-btn"
              :title="`Promote to milestone chunk: ${chunk.title}`"
              @click="onPromoteItem(chunk.title, chunk.description)"
            >
              ↑ Promote to milestone
            </button>
          </div>
        </li>
      </ul>
    </div>

    <!-- Activity feed -->
    <div class="si-activity">
      <h3 class="si-section-h">
        Activity
      </h3>
      <div
        v-if="store.activity.length === 0"
        class="si-activity-empty"
      >
        No activity yet. Configure a Coding LLM to get started.
      </div>
      <ul
        v-else
        class="si-activity-list"
      >
        <li
          v-for="entry in store.activity.slice(0, 12)"
          :key="entry.id"
          :class="['si-activity-item', `si-activity-item--${entry.level}`]"
        >
          <span class="si-activity-time">{{ formatTime(entry.timestamp) }}</span>
          <span class="si-activity-msg">{{ entry.message }}</span>
        </li>
      </ul>
    </div>

    <!-- Live engine status (visible whenever the loop has reported anything) -->
    <div
      v-if="store.running || store.activePhase"
      class="si-live"
      :class="{ 'si-live--running': store.running }"
    >
      <div class="si-live-row">
        <span
          class="si-live-dot"
          :class="{ 'si-live-dot--on': store.running }"
        />
        <span class="si-live-phase">{{ store.activePhase ?? 'idle' }}</span>
        <span class="si-live-msg">{{ store.liveMessage }}</span>
        <span class="si-live-pct">{{ store.livePercent }}%</span>
      </div>
      <div class="si-live-track">
        <div
          class="si-live-fill"
          :style="{ width: `${store.livePercent}%` }"
        />
      </div>
    </div>

    <!-- Observability: success/fail rates, last error, run history -->
    <div class="si-obs">
      <div class="si-obs-header">
        <h3 class="si-section-h">
          Observability
        </h3>
        <button
          v-if="store.runs.length > 0"
          type="button"
          class="si-btn si-btn-tiny"
          title="Wipe the persisted run log"
          @click="onClearLog"
        >
          🗑 Clear log
        </button>
      </div>
      <div class="si-obs-stats">
        <div class="si-stat">
          <div class="si-stat-num">
            {{ store.metrics.total_runs }}
          </div>
          <div class="si-stat-label">
            Runs
          </div>
        </div>
        <div class="si-stat si-stat--ok">
          <div class="si-stat-num">
            {{ formatRate(store.metrics.success_rate) }}
          </div>
          <div class="si-stat-label">
            Success ({{ store.metrics.successes }})
          </div>
        </div>
        <div class="si-stat si-stat--err">
          <div class="si-stat-num">
            {{ formatRate(store.metrics.failure_rate) }}
          </div>
          <div class="si-stat-label">
            Failure ({{ store.metrics.failures }})
          </div>
        </div>
        <div class="si-stat">
          <div class="si-stat-num">
            {{ formatDuration(store.metrics.avg_duration_ms) }}
          </div>
          <div class="si-stat-label">
            Avg. latency
          </div>
        </div>
      </div>

      <!-- Cost / token telemetry (Chunk 28.5) -->
      <div class="si-cost-row">
        <div class="si-cost-card">
          <div class="si-cost-label">
            Total spend
          </div>
          <div class="si-cost-value">
            {{ formatUsd(store.metrics.total_cost_usd) }}
          </div>
          <div class="si-cost-sub">
            {{ formatTokens(store.metrics.total_prompt_tokens) }} prompt
            · {{ formatTokens(store.metrics.total_completion_tokens) }} out
          </div>
        </div>
        <div class="si-cost-card">
          <div class="si-cost-label">
            Last 7 days
          </div>
          <div class="si-cost-value">
            {{ formatUsd(store.metrics.rolling_7d_cost_usd) }}
          </div>
          <div class="si-cost-sub">
            {{ store.metrics.rolling_7d_runs }} runs
            · {{ formatTokens(store.metrics.rolling_7d_prompt_tokens) }} prompt
          </div>
        </div>
        <div
          v-if="providerCostEntries.length > 0"
          class="si-cost-card si-cost-card--breakdown"
        >
          <div class="si-cost-label">
            By provider
          </div>
          <ul class="si-cost-breakdown">
            <li
              v-for="[provider, cost] in providerCostEntries"
              :key="provider"
              class="si-cost-breakdown-item"
            >
              <code>{{ provider }}</code>
              <span>{{ formatUsd(cost) }}</span>
            </li>
          </ul>
        </div>
      </div>
      <div
        v-if="store.metrics.last_error"
        class="si-obs-error"
        :title="store.metrics.last_error ?? ''"
      >
        <span class="si-obs-error-tag">Last error</span>
        <span class="si-obs-error-chunk">[{{ store.metrics.last_error_chunk }}]</span>
        <span class="si-obs-error-msg">{{ store.metrics.last_error }}</span>
      </div>

      <!-- Persisted run log (newest first) -->
      <div class="si-runs">
        <div class="si-runs-header">
          <span>Recent runs</span>
          <span class="si-runs-count">{{ store.runs.length }}</span>
        </div>
        <ul
          v-if="store.runs.length > 0"
          class="si-runs-list"
        >
          <li
            v-for="r in store.runs.slice(0, 25)"
            :key="`${r.started_at_ms}-${r.chunk_id}-${r.outcome}`"
            class="si-run"
            :class="`si-run--${r.outcome}`"
            :title="r.error ?? ''"
          >
            <span class="si-run-icon">{{ runIcon(r.outcome) }}</span>
            <span class="si-run-time">{{ formatTime(r.finished_at_ms) }}</span>
            <span class="si-run-chunk">{{ r.chunk_id }}</span>
            <span class="si-run-title">{{ r.chunk_title }}</span>
            <span class="si-run-meta">
              <code>{{ r.provider }}/{{ r.model }}</code>
              · {{ formatDuration(r.duration_ms) }}
              <template v-if="r.outcome === 'success'"> · {{ r.plan_chars }}c</template>
              <template v-if="r.cost_usd !== null && r.cost_usd !== undefined">
                · <span
                  class="si-run-cost"
                  :title="`${r.prompt_tokens ?? 0} prompt + ${r.completion_tokens ?? 0} completion tokens`"
                >{{ formatUsd(r.cost_usd) }}</span>
              </template>
            </span>
          </li>
        </ul>
        <p
          v-else
          class="si-runs-empty"
        >
          No runs yet. Enable self-improve to see live planning runs here.
        </p>
      </div>
    </div>

    <!-- Action footer -->
    <footer class="si-footer">
      <button
        v-if="!store.isEnabled"
        type="button"
        class="si-btn si-btn-primary"
        :disabled="!store.canEnable"
        :title="store.canEnable ? '' : 'Configure a Coding LLM first'"
        @click="$emit('request-enable')"
      >
        ⚡ Enable self-improve
      </button>
      <button
        v-else
        type="button"
        class="si-btn si-btn-danger"
        @click="onDisable"
      >
        ⏹ Disable self-improve
      </button>
      <button
        type="button"
        class="si-btn si-btn-ghost"
        @click="$emit('configure-llm')"
      >
        🧠 Configure Coding LLM
      </button>
      <label
        class="si-autostart"
        :title="'Launch TerranSoul on Windows login (per-user, reversible)'"
      >
        <input
          type="checkbox"
          :checked="store.autostartEnabled"
          @change="onAutostartToggle(($event.target as HTMLInputElement).checked)"
        >
        <span>Auto-start on login</span>
      </label>
    </footer>

    <!-- GitHub PR + pull-from-main controls (Chunk 25.13) -->
    <div class="si-github">
      <h3 class="si-section-h">
        GitHub
      </h3>
      <p class="si-github-desc">
        When all chunks complete, the loop opens a Pull Request against
        <code>{{ store.githubConfig?.default_base ?? 'main' }}</code> and
        requests review from your admin reviewers.
      </p>
      <div class="si-github-device">
        <div class="si-github-device-copy">
          <strong>Browser authorization</strong>
          <span>Use GitHub Device Flow to authorize this machine without pasting a token manually.</span>
        </div>
        <button
          type="button"
          class="si-btn si-btn-tiny"
          :disabled="githubAuthBusy"
          @click="onStartGithubBrowserAuth"
        >
          {{ githubAuthBusy ? 'Waiting for authorization...' : 'Authorize with GitHub in browser' }}
        </button>
        <div
          v-if="store.githubDeviceCode"
          class="si-github-device-code"
        >
          <span>Enter this code:</span>
          <code>{{ store.githubDeviceCode.user_code }}</code>
          <button
            type="button"
            class="si-btn si-btn-tiny"
            @click="openExternalUrl(store.githubDeviceCode.verification_uri)"
          >
            Open authorization page
          </button>
        </div>
        <p
          v-if="githubAuthStatus"
          class="si-github-result"
          :class="githubAuthStatusKind === 'success' ? 'si-github-result--ok' : 'si-github-result--warn'"
        >
          {{ githubAuthStatus }}
        </p>
      </div>
      <div class="si-github-grid">
        <label class="si-field">
          <span class="si-field-label">Token</span>
          <input
            v-model="ghToken"
            type="password"
            class="si-input"
            placeholder="ghp_…"
            autocomplete="off"
          >
        </label>
        <label class="si-field">
          <span class="si-field-label">Owner / Repo</span>
          <input
            v-model="ghOwnerRepo"
            type="text"
            class="si-input"
            placeholder="owner/repo (auto-detected if empty)"
          >
        </label>
        <label class="si-field">
          <span class="si-field-label">Base branch</span>
          <input
            v-model="ghBase"
            type="text"
            class="si-input"
            placeholder="main"
          >
        </label>
        <label class="si-field">
          <span class="si-field-label">Reviewers (comma-separated)</span>
          <input
            v-model="ghReviewers"
            type="text"
            class="si-input"
            placeholder="alice, bob"
          >
        </label>
      </div>
      <div class="si-github-actions">
        <button
          type="button"
          class="si-btn si-btn-tiny"
          @click="onSaveGithub"
        >
          💾 Save GitHub config
        </button>
        <button
          type="button"
          class="si-btn si-btn-tiny"
          :disabled="!store.githubConfig"
          title="Open or update PR for the current feature branch"
          @click="onOpenPr"
        >
          🚀 Open PR now
        </button>
        <button
          type="button"
          class="si-btn si-btn-tiny"
          title="Pull origin/<base> and merge with LLM-assisted conflict resolution"
          @click="onPullMain"
        >
          ⬇ Pull from main
        </button>
      </div>
      <p
        v-if="store.lastPullRequest"
        class="si-github-result si-github-result--ok"
      >
        Last PR: <a
          :href="store.lastPullRequest.html_url"
          target="_blank"
          rel="noreferrer"
        >
          #{{ store.lastPullRequest.number }}
        </a>
        ({{ store.lastPullRequest.created ? 'opened' : 'updated' }})
      </p>
      <p
        v-if="store.lastPullResult"
        class="si-github-result"
        :class="store.lastPullResult.merged ? 'si-github-result--ok' : 'si-github-result--warn'"
      >
        Last pull: {{ store.lastPullResult.message }}
      </p>
    </div>

    <SelfImproveSessionsPanel />
  </section>
</template>

<script setup lang="ts">
/* eslint-disable max-lines */
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { useSelfImproveStore, type SelfImprovePhase } from '../stores/self-improve';
import SelfImproveSessionsPanel from './SelfImproveSessionsPanel.vue';

const store = useSelfImproveStore();

// ── GitHub config bound to local input fields ──────────────────────────
const ghToken = ref('');
const ghOwnerRepo = ref('');
const ghBase = ref('main');
const ghReviewers = ref('');
const githubAuthBusy = ref(false);
const githubAuthStatus = ref('');
const githubAuthStatusKind = ref<'info' | 'success' | 'warn'>('info');
let githubAuthPollTimer: number | null = null;

onBeforeUnmount(() => {
  clearGithubAuthPoll();
});

watch(
  () => store.githubConfig,
  (cfg) => {
    if (!cfg) return;
    // Never echo the token back into the visible input; show a masked
    // placeholder by leaving it blank. Save logic preserves the existing
    // token if the user leaves it blank.
    ghOwnerRepo.value = cfg.owner && cfg.repo ? `${cfg.owner}/${cfg.repo}` : '';
    ghBase.value = cfg.default_base || 'main';
    ghReviewers.value = cfg.reviewers.join(', ');
  },
  { immediate: true },
);

async function onSaveGithub() {
  const [owner, repo] = ghOwnerRepo.value.includes('/')
    ? ghOwnerRepo.value.split('/').map((s) => s.trim())
    : ['', ''];
  const token = ghToken.value.trim() || store.githubConfig?.token || '';
  if (!token) {
    console.warn('[SelfImprove] GitHub token required to save');
    store.logActivity('warn', 'GitHub config: token required');
    return;
  }
  try {
    await store.setGithubConfig({
      token,
      owner,
      repo,
      default_base: ghBase.value.trim() || 'main',
      reviewers: ghReviewers.value
        .split(',')
        .map((s) => s.trim())
        .filter((s) => s.length > 0),
    });
    ghToken.value = ''; // never linger
  } catch (e) {
    console.warn('[SelfImprove] save github config failed:', e);
  }
}

async function onStartGithubBrowserAuth() {
  clearGithubAuthPoll();
  githubAuthBusy.value = true;
  githubAuthStatusKind.value = 'info';
  githubAuthStatus.value = 'Requesting a device code...';
  try {
    const response = await store.requestGitHubDeviceCode('repo');
    githubAuthStatus.value = 'Waiting for authorization in your browser.';
    await openExternalUrl(response.verification_uri);
    scheduleGithubAuthPoll(Math.max(1, response.interval));
  } catch (e) {
    githubAuthBusy.value = false;
    githubAuthStatusKind.value = 'warn';
    githubAuthStatus.value = `Authorization could not start: ${String(e)}`;
  }
}

function scheduleGithubAuthPoll(intervalSeconds: number): void {
  clearGithubAuthPoll();
  githubAuthPollTimer = window.setInterval(() => {
    void pollGithubAuthOnce();
  }, intervalSeconds * 1000);
  void pollGithubAuthOnce();
}

async function pollGithubAuthOnce(): Promise<void> {
  try {
    const result = await store.pollGitHubDeviceToken();
    if (result.status === 'pending') {
      githubAuthStatusKind.value = 'info';
      githubAuthStatus.value = 'Still waiting for browser authorization.';
      return;
    }
    clearGithubAuthPoll();
    githubAuthBusy.value = false;
    if (result.status === 'success') {
      githubAuthStatusKind.value = 'success';
      githubAuthStatus.value = 'GitHub authorization saved for self-improve.';
      ghToken.value = '';
      return;
    }
    githubAuthStatusKind.value = 'warn';
    githubAuthStatus.value = result.status === 'error'
      ? `GitHub authorization failed: ${result.message}`
      : `GitHub authorization ${result.status}.`;
  } catch (e) {
    clearGithubAuthPoll();
    githubAuthBusy.value = false;
    githubAuthStatusKind.value = 'warn';
    githubAuthStatus.value = `GitHub authorization failed: ${String(e)}`;
  }
}

function clearGithubAuthPoll(): void {
  if (githubAuthPollTimer !== null) {
    window.clearInterval(githubAuthPollTimer);
    githubAuthPollTimer = null;
  }
}

async function openExternalUrl(url: string): Promise<void> {
  try {
    const shell = await import('@tauri-apps/plugin-shell');
    await shell.open(url);
  } catch {
    window.open(url, '_blank', 'noopener,noreferrer');
  }
}

async function onOpenPr() {
  try {
    await store.openPullRequest();
  } catch (e) {
    console.warn('[SelfImprove] open PR failed:', e);
  }
}

async function onPullMain() {
  try {
    await store.pullFromMain();
  } catch (e) {
    console.warn('[SelfImprove] pull main failed:', e);
  }
}

/**
 * Eligible-to-promote check: backlog items sourced from the run log
 * (failed runs) or improvement-chunk seeds. Items already from
 * `rules/milestones.md` are skipped — they're already chunks.
 */
function isPromotable(item: unknown): boolean {
  if (typeof item !== 'object' || item === null) return false;
  const source = (item as { source?: unknown }).source;
  return typeof source === 'string' && source !== 'rules/milestones.md';
}

/**
 * Pull a goal/description string from a backlog item (workboard
 * `detail` field or fallback to the title). Used to seed the promote
 * confirm dialog.
 */
function backlogItemGoal(item: unknown): string {
  if (typeof item === 'object' && item !== null) {
    const detail = (item as { detail?: unknown }).detail;
    if (typeof detail === 'string' && detail.trim().length > 0) return detail;
    const title = (item as { title?: unknown }).title;
    if (typeof title === 'string') return title;
  }
  return '';
}

/**
 * Confirm-then-promote a backlog or improvement item to a milestone
 * chunk. Uses the browser's native confirm dialog so the user always
 * gets a final yes/no before the markdown file is written. The store
 * action refreshes the workboard on success.
 */
async function onPromoteItem(title: string, goal: string): Promise<void> {
  const trimmedTitle = title.trim();
  const trimmedGoal = goal.trim();
  if (!trimmedTitle || !trimmedGoal) return;
  const ok = window.confirm(
    `Promote to milestone chunk?\n\nTitle: ${trimmedTitle}\n\nGoal: ${trimmedGoal}\n\nThis appends a new "not-started" row to rules/milestones.md.`,
  );
  if (!ok) return;
  try {
    const result = await store.promoteToChunk(trimmedTitle, trimmedGoal);
    window.alert(`Created chunk ${result.chunk_id} — ${result.title}`);
  } catch (e) {
    console.warn('[SelfImprove] promote failed:', e);
    window.alert(`Promotion failed: ${String(e)}`);
  }
}

defineEmits<{
  'request-enable': [];
  'configure-llm': [];
}>();

const statusPillClass = computed(() => ({
  'si-status-pill--on': store.isEnabled,
  'si-status-pill--off': !store.isEnabled,
}));
const statusPillLabel = computed(() => (store.isEnabled ? 'ENABLED' : 'OFF'));

/**
 * Sorted per-provider cost entries (descending USD) so the UI shows
 * the biggest spenders first. Filters out zero-cost providers (local
 * Ollama) so the list stays short.
 */
const providerCostEntries = computed<[string, number][]>(() => {
  const map = store.metrics.cost_by_provider ?? {};
  return Object.entries(map)
    .filter(([, v]) => v > 0)
    .sort((a, b) => b[1] - a[1]);
});

const finishedPhases = computed(() =>
  store.workboard.finished.length > 0
    ? store.workboard.finished
    : store.phases.filter((p) => p.status === 'completed'),
);
const workingPhases = computed(() =>
  store.workboard.working.length > 0
    ? store.workboard.working
    : store.phases.filter((p) => p.status === 'in-progress'),
);
const backlogPhases = computed(() =>
  store.workboard.backlog.length > 0
    ? store.workboard.backlog
    : store.phases.filter((p) => p.status === 'not-started' || p.status === 'blocked'),
);
const backlogPreview = computed(() => backlogPhases.value.slice(0, 4));
const backlogCount = computed(() => backlogPhases.value.length + store.improvementChunks.length);

const workflowSteps = computed(() => [
  {
    index: 1,
    label: 'Select chunk',
    detail: store.nextPhase
      ? `Next queue item: ${store.nextPhase.title}`
      : 'All roadmap phases are complete.',
    active: !store.running,
  },
  {
    index: 2,
    label: 'Plan with brain context',
    detail: 'Load MCP/rules context, define validation gates, and keep durable lessons searchable.',
    active: store.activePhase === 'plan',
  },
  {
    index: 3,
    label: 'Code in isolated workflow',
    detail: store.running
      ? `${store.activePhase ?? 'engine'} · ${store.liveMessage}`
      : 'Use the configured Coding LLM, feature branch, and session transcript.',
    active: store.running,
  },
  {
    index: 4,
    label: 'Validate, archive, and PR',
    detail: 'Run checks, update milestones/completion log, and open or refresh the review PR.',
    active: store.activePhase === 'complete',
  },
]);

function phaseStatusIcon(status: SelfImprovePhase['status']): string {
  switch (status) {
    case 'completed': return '✓';
    case 'in-progress': return '◐';
    case 'blocked': return '⛔';
    default: return '○';
  }
}
function phaseStatusLabel(status: SelfImprovePhase['status']): string {
  switch (status) {
    case 'completed': return 'Completed';
    case 'in-progress': return 'In progress';
    case 'blocked': return 'Blocked';
    default: return 'Not started';
  }
}

function formatTime(ts: number): string {
  const d = new Date(ts);
  return d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit', second: '2-digit' });
}

function formatRate(r: number): string {
  if (!Number.isFinite(r) || r < 0) return '—';
  return `${Math.round(r * 100)}%`;
}

function formatDuration(ms: number): string {
  if (!Number.isFinite(ms) || ms <= 0) return '—';
  if (ms < 1000) return `${Math.round(ms)}ms`;
  if (ms < 60_000) return `${(ms / 1000).toFixed(1)}s`;
  const m = Math.floor(ms / 60_000);
  const s = Math.round((ms % 60_000) / 1000);
  return `${m}m${s.toString().padStart(2, '0')}s`;
}

function runIcon(outcome: 'running' | 'success' | 'failure'): string {
  switch (outcome) {
    case 'success': return '✓';
    case 'failure': return '✗';
    default: return '◐';
  }
}

/**
 * Format a USD cost. Renders `—` when undefined, `Free` for $0, and
 * up to 4 decimal places for small spends so a $0.0002 run doesn't
 * round to `$0.00` and look like a free local run.
 */
function formatUsd(usd: number | null | undefined): string {
  if (usd === null || usd === undefined || !Number.isFinite(usd)) return '—';
  if (usd === 0) return 'Free';
  if (usd < 0.01) return `$${usd.toFixed(4)}`;
  if (usd < 1) return `$${usd.toFixed(3)}`;
  return `$${usd.toFixed(2)}`;
}

/** Compact integer formatter — `1234` -> `1.2k`, `1_500_000` -> `1.5M`. */
function formatTokens(n: number | null | undefined): string {
  if (n === null || n === undefined || !Number.isFinite(n)) return '—';
  if (n < 1000) return n.toString();
  if (n < 1_000_000) return `${(n / 1000).toFixed(1)}k`;
  return `${(n / 1_000_000).toFixed(2)}M`;
}

async function onDisable() {
  try {
    await store.disable();
  } catch (e) {
    console.warn('[SelfImprove] disable failed:', e);
  }
}

async function onAutostartToggle(enabled: boolean) {
  try {
    await store.setAutostart(enabled);
  } catch (e) {
    console.warn('[SelfImprove] autostart toggle failed:', e);
  }
}

async function onClearLog() {
  try {
    await store.clearRunLog();
  } catch (e) {
    console.warn('[SelfImprove] clear log failed:', e);
  }
}
</script>

<style scoped>
.si-panel {
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 16px;
  background: linear-gradient(135deg, rgba(15, 20, 35, 0.6), rgba(20, 25, 40, 0.4));
  border: 1px solid rgba(124, 111, 255, 0.18);
  border-radius: 12px;
  color: var(--ts-text-primary, #eaecf4);
}

.si-panel-header { display: flex; flex-direction: column; gap: 4px; }
.si-panel-title-row { display: flex; align-items: center; gap: 10px; }
.si-panel-icon { font-size: 1.4rem; }
.si-panel-title {
  margin: 0;
  font-size: 1.05rem;
  font-weight: 700;
  flex: 1;
}
.si-panel-sub {
  margin: 0;
  font-size: 0.82rem;
  color: var(--ts-text-muted, #94a3b8);
  line-height: 1.4;
}
.si-panel-sub code {
  background: rgba(255, 255, 255, 0.08);
  padding: 1px 5px;
  border-radius: 4px;
  font-size: 0.78rem;
}

.si-status-pill {
  font-size: 0.7rem;
  font-weight: 800;
  letter-spacing: 0.06em;
  padding: 3px 8px;
  border-radius: 999px;
  border: 1px solid transparent;
}
.si-status-pill--off {
  background: rgba(148, 163, 184, 0.15);
  color: #cbd5e1;
  border-color: rgba(148, 163, 184, 0.3);
}
.si-status-pill--on {
  background: rgba(34, 197, 94, 0.18);
  color: #86efac;
  border-color: rgba(34, 197, 94, 0.45);
  animation: si-pulse 2.4s ease-in-out infinite;
}

@keyframes si-pulse {
  0%, 100% { box-shadow: 0 0 0 0 rgba(34, 197, 94, 0.35); }
  50%      { box-shadow: 0 0 0 6px rgba(34, 197, 94, 0); }
}

/* Progress bar */
.si-progress-card {
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-radius: 10px;
  padding: 12px 14px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.si-progress-top {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  font-size: 0.85rem;
}
.si-progress-label { color: var(--ts-text-muted, #94a3b8); }
.si-progress-percent {
  font-weight: 700;
  font-size: 1rem;
  background: linear-gradient(135deg, #a78bfa, #7c6fff);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}
.si-progress-track {
  height: 8px;
  background: rgba(255, 255, 255, 0.06);
  border-radius: 999px;
  overflow: hidden;
  position: relative;
}
.si-progress-fill {
  height: 100%;
  background: linear-gradient(90deg, #7c6fff, #a78bfa, #ec4899);
  background-size: 200% 100%;
  border-radius: 999px;
  transition: width 0.5s cubic-bezier(0.22, 1, 0.36, 1);
  animation: si-shimmer 3s linear infinite;
}
@keyframes si-shimmer {
  0%   { background-position: 0% 0; }
  100% { background-position: 200% 0; }
}
.si-next-line { margin: 0; font-size: 0.8rem; color: var(--ts-text-muted, #94a3b8); }
.si-next-done { color: #86efac; }

/* Finished / working / backlog dashboard */
.si-queue {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 10px;
}
.si-queue-lane {
  padding: 10px;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.07);
}
.si-queue-lane--done { border-color: rgba(34, 197, 94, 0.28); }
.si-queue-lane--active { border-color: rgba(124, 111, 255, 0.38); }
.si-queue-lane--backlog { border-color: rgba(251, 191, 36, 0.28); }
.si-queue-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
  font-size: 0.76rem;
  font-weight: 800;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--ts-text-muted, #94a3b8);
}
.si-queue-head strong {
  color: var(--ts-text-primary, #eaecf4);
  font-size: 0.9rem;
}
.si-queue-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.si-queue-list li {
  font-size: 0.76rem;
  line-height: 1.35;
  color: var(--ts-text-secondary, #cbd5e1);
}
.si-queue-empty {
  color: var(--ts-text-muted, #94a3b8);
  font-style: italic;
}
.si-queue-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.si-queue-row-title {
  flex: 1 1 auto;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.si-promote-btn {
  flex: 0 0 auto;
  font-size: 0.7rem;
  padding: 2px 8px;
  border-radius: 6px;
  border: 1px solid var(--ts-accent, #7c6fff);
  background: rgba(124, 111, 255, 0.12);
  color: var(--ts-text-primary, #e2e8f0);
  cursor: pointer;
  transition: background 0.15s ease, border-color 0.15s ease;
}
.si-promote-btn:hover {
  background: rgba(124, 111, 255, 0.24);
  border-color: var(--ts-accent-bright, #a594ff);
}
.si-improvement-actions {
  margin-top: 6px;
}

/* Coding workflow */
.si-workflow {
  padding: 12px;
  border-radius: 10px;
  background: rgba(124, 111, 255, 0.06);
  border: 1px solid rgba(124, 111, 255, 0.16);
}
.si-workflow-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: 8px;
}
.si-workflow-step {
  display: grid;
  grid-template-columns: 24px 1fr;
  gap: 9px;
  padding: 8px;
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.025);
  border: 1px solid rgba(255, 255, 255, 0.06);
}
.si-workflow-step--active {
  border-color: rgba(94, 234, 212, 0.35);
  background: rgba(94, 234, 212, 0.07);
}
.si-workflow-index {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.08);
  font-size: 0.72rem;
  font-weight: 800;
}
.si-workflow-step strong {
  display: block;
  font-size: 0.82rem;
}
.si-workflow-step p {
  margin: 2px 0 0;
  color: var(--ts-text-muted, #94a3b8);
  font-size: 0.76rem;
  line-height: 1.4;
}

/* Roadmap */
.si-section-h {
  margin: 0 0 8px;
  font-size: 0.78rem;
  font-weight: 700;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: var(--ts-text-muted, #94a3b8);
}

.si-phase-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.si-phase {
  display: grid;
  grid-template-columns: 22px 22px 1fr;
  gap: 10px;
  padding: 8px 10px;
  background: rgba(255, 255, 255, 0.02);
  border: 1px solid rgba(255, 255, 255, 0.05);
  border-radius: 8px;
  transition: background 0.15s, border-color 0.15s;
}
.si-phase--completed {
  border-color: rgba(34, 197, 94, 0.35);
  background: rgba(34, 197, 94, 0.06);
}
.si-phase--in-progress {
  border-color: rgba(124, 111, 255, 0.45);
  background: rgba(124, 111, 255, 0.08);
  box-shadow: 0 0 0 1px rgba(124, 111, 255, 0.15);
}
.si-phase--blocked {
  border-color: rgba(248, 113, 113, 0.4);
  background: rgba(248, 113, 113, 0.06);
}
.si-phase-num {
  font-size: 0.72rem;
  font-weight: 700;
  color: var(--ts-text-muted, #94a3b8);
  text-align: center;
  align-self: center;
}
.si-phase-icon {
  font-size: 1rem;
  text-align: center;
  align-self: center;
}
.si-phase--completed .si-phase-icon { color: #86efac; }
.si-phase--in-progress .si-phase-icon {
  color: #c4b5fd;
  animation: si-spin 2s linear infinite;
  display: inline-block;
}
@keyframes si-spin {
  from { transform: rotate(0); }
  to   { transform: rotate(360deg); }
}
.si-phase--blocked .si-phase-icon { color: #fca5a5; }

.si-phase-title { font-size: 0.88rem; font-weight: 600; }
.si-phase-desc { font-size: 0.78rem; color: var(--ts-text-muted, #94a3b8); margin-top: 2px; line-height: 1.4; }
.si-phase-blocked { font-size: 0.74rem; color: #fca5a5; margin-top: 4px; }

/* Improvement chunks */
.si-improvement {
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(125, 211, 252, 0.15);
  border-radius: 10px;
  padding: 12px;
}
.si-improvement-desc {
  margin: 0 0 10px;
  color: var(--ts-text-muted, #94a3b8);
  font-size: 0.78rem;
  line-height: 1.45;
}
.si-improvement-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: 8px;
}
.si-improvement-item {
  padding: 10px;
  border-radius: 9px;
  background: rgba(255, 255, 255, 0.025);
  border: 1px solid rgba(255, 255, 255, 0.07);
}
.si-improvement-item--high { border-color: rgba(248, 113, 113, 0.45); }
.si-improvement-item--medium { border-color: rgba(125, 211, 252, 0.35); }
.si-improvement-item--low { border-color: rgba(148, 163, 184, 0.24); }
.si-improvement-top {
  display: flex;
  justify-content: space-between;
  gap: 8px;
  margin-bottom: 4px;
}
.si-improvement-priority,
.si-improvement-status {
  font-size: 0.66rem;
  font-weight: 800;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: var(--ts-text-muted, #94a3b8);
}
.si-improvement-title {
  font-weight: 700;
  font-size: 0.86rem;
}
.si-improvement-body,
.si-improvement-trigger {
  margin-top: 4px;
  color: var(--ts-text-muted, #94a3b8);
  font-size: 0.76rem;
  line-height: 1.4;
}

/* Activity feed */
.si-activity-empty {
  font-size: 0.82rem;
  color: var(--ts-text-muted, #94a3b8);
  font-style: italic;
  padding: 10px 0;
}
.si-activity-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
  max-height: 220px;
  overflow-y: auto;
}
.si-activity-item {
  display: grid;
  grid-template-columns: 70px 1fr;
  gap: 8px;
  font-size: 0.78rem;
  padding: 4px 8px;
  border-radius: 6px;
  border-left: 3px solid rgba(255, 255, 255, 0.1);
  background: rgba(255, 255, 255, 0.02);
}
.si-activity-item--success { border-left-color: #34d399; }
.si-activity-item--warn    { border-left-color: #fbbf24; }
.si-activity-item--error   { border-left-color: #f87171; }
.si-activity-item--info    { border-left-color: #60a5fa; }
.si-activity-time {
  color: var(--ts-text-muted, #94a3b8);
  font-variant-numeric: tabular-nums;
  font-size: 0.72rem;
}

/* Footer actions */
.si-footer {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  border-top: 1px solid rgba(255, 255, 255, 0.06);
  padding-top: 14px;
}
.si-btn {
  border: 1px solid transparent;
  border-radius: 8px;
  padding: 9px 16px;
  font-size: 0.86rem;
  font-weight: 600;
  cursor: pointer;
  transition: transform 0.12s, box-shadow 0.12s, background 0.12s;
}
.si-btn:focus-visible { outline: 2px solid var(--ts-accent, #7c6fff); outline-offset: 2px; }
.si-btn:disabled { opacity: 0.5; cursor: not-allowed; }

.si-btn-primary {
  background: linear-gradient(135deg, #7c6fff, #a78bfa);
  color: white;
  box-shadow: 0 4px 14px rgba(124, 111, 255, 0.35);
}
.si-btn-primary:hover:not(:disabled) { transform: translateY(-1px); box-shadow: 0 6px 18px rgba(124, 111, 255, 0.5); }

.si-btn-danger {
  background: linear-gradient(135deg, #ef4444, #b91c1c);
  color: white;
}
.si-btn-danger:hover { transform: translateY(-1px); }

.si-btn-ghost {
  background: rgba(255, 255, 255, 0.06);
  color: var(--ts-text-primary, #eaecf4);
  border-color: rgba(255, 255, 255, 0.1);
}
.si-btn-ghost:hover { background: rgba(255, 255, 255, 0.12); }

/* Live engine status banner */
.si-live {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 10px 12px;
  border-radius: 8px;
  background: rgba(124, 111, 255, 0.08);
  border: 1px solid rgba(124, 111, 255, 0.18);
}
.si-live--running { border-color: rgba(94, 234, 212, 0.4); background: rgba(94, 234, 212, 0.08); }
.si-live-row { display: flex; align-items: center; gap: 8px; font-size: 0.82rem; }
.si-live-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.3);
  flex-shrink: 0;
}
.si-live-dot--on {
  background: #5eead4;
  box-shadow: 0 0 8px #5eead4;
  animation: si-pulse 1.4s ease-in-out infinite;
}
@keyframes si-pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.5; } }
.si-live-phase { font-weight: 700; text-transform: uppercase; letter-spacing: 0.04em; font-size: 0.72rem; color: #5eead4; }
.si-live-msg { flex: 1; color: var(--ts-text-muted, #94a3b8); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.si-live-pct { font-variant-numeric: tabular-nums; font-weight: 600; }
.si-live-track { height: 4px; background: rgba(255, 255, 255, 0.08); border-radius: 2px; overflow: hidden; }
.si-live-fill { height: 100%; background: linear-gradient(90deg, #7c6fff, #5eead4); transition: width 0.3s ease; }

/* Autostart toggle */
.si-autostart {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 0.82rem;
  color: var(--ts-text-muted, #94a3b8);
  cursor: pointer;
  user-select: none;
  margin-left: auto;
}
.si-autostart input[type="checkbox"] { accent-color: #7c6fff; cursor: pointer; }

/* Observability section */
.si-obs { display: flex; flex-direction: column; gap: 10px; }
.si-obs-header { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
.si-obs-stats {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 8px;
}
.si-stat {
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  padding: 10px 8px;
  text-align: center;
}
.si-stat-num {
  font-size: 1.2rem;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
  line-height: 1.2;
}
.si-stat-label {
  font-size: 0.68rem;
  color: var(--ts-text-muted, #94a3b8);
  text-transform: uppercase;
  letter-spacing: 0.04em;
  margin-top: 2px;
}
.si-stat--ok { border-color: rgba(94, 234, 212, 0.3); }
.si-stat--ok .si-stat-num { color: #5eead4; }
.si-stat--err { border-color: rgba(252, 165, 165, 0.3); }
.si-stat--err .si-stat-num { color: #fca5a5; }

/* Cost telemetry (Chunk 28.5) */
.si-cost-row {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
  gap: 8px;
}
.si-cost-card {
  background: rgba(124, 111, 255, 0.06);
  border: 1px solid rgba(124, 111, 255, 0.2);
  border-radius: 8px;
  padding: 10px 12px;
}
.si-cost-card--breakdown {
  grid-column: 1 / -1;
}
.si-cost-label {
  font-size: 0.68rem;
  color: var(--ts-text-muted, #94a3b8);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
.si-cost-value {
  font-size: 1.15rem;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
  margin-top: 2px;
  color: #c7b9ff;
}
.si-cost-sub {
  font-size: 0.7rem;
  color: var(--ts-text-muted, #94a3b8);
  margin-top: 2px;
}
.si-cost-breakdown {
  list-style: none;
  margin: 6px 0 0;
  padding: 0;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
  gap: 4px 12px;
}
.si-cost-breakdown-item {
  display: flex;
  justify-content: space-between;
  font-size: 0.78rem;
  font-variant-numeric: tabular-nums;
}
.si-cost-breakdown-item code {
  background: transparent;
  padding: 0;
  color: var(--ts-text-muted, #94a3b8);
}
.si-run-cost {
  color: #c7b9ff;
  font-variant-numeric: tabular-nums;
}

.si-obs-error {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 10px;
  background: rgba(252, 165, 165, 0.08);
  border: 1px solid rgba(252, 165, 165, 0.25);
  border-radius: 6px;
  font-size: 0.78rem;
}
.si-obs-error-tag {
  font-weight: 700;
  color: #fca5a5;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  font-size: 0.68rem;
  flex-shrink: 0;
}
.si-obs-error-chunk { font-family: monospace; color: #fca5a5; flex-shrink: 0; }
.si-obs-error-msg {
  color: var(--ts-text-muted, #94a3b8);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.si-runs { display: flex; flex-direction: column; gap: 6px; }
.si-runs-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 0.78rem;
  color: var(--ts-text-muted, #94a3b8);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
.si-runs-count {
  background: rgba(255, 255, 255, 0.08);
  padding: 1px 8px;
  border-radius: 10px;
  font-size: 0.7rem;
}
.si-runs-list {
  list-style: none;
  margin: 0;
  padding: 0;
  max-height: 240px;
  overflow-y: auto;
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-radius: 6px;
}
.si-runs-empty {
  margin: 0;
  padding: 12px;
  text-align: center;
  font-size: 0.78rem;
  color: var(--ts-text-muted, #94a3b8);
  background: rgba(255, 255, 255, 0.02);
  border-radius: 6px;
}
.si-run {
  display: grid;
  grid-template-columns: 18px 70px 50px 1fr auto;
  gap: 8px;
  align-items: center;
  padding: 6px 10px;
  font-size: 0.78rem;
  border-bottom: 1px solid rgba(255, 255, 255, 0.04);
}
.si-run:last-child { border-bottom: none; }
.si-run-icon { font-weight: 700; text-align: center; }
.si-run--success .si-run-icon { color: #5eead4; }
.si-run--failure .si-run-icon { color: #fca5a5; }
.si-run--running .si-run-icon { color: #fcd34d; }
.si-run-time { color: var(--ts-text-muted, #94a3b8); font-variant-numeric: tabular-nums; font-size: 0.72rem; }
.si-run-chunk { font-family: monospace; color: #c4b5fd; }
.si-run-title { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.si-run-meta { color: var(--ts-text-muted, #94a3b8); font-size: 0.7rem; white-space: nowrap; }
.si-run-meta code {
  background: rgba(255, 255, 255, 0.06);
  padding: 1px 4px;
  border-radius: 3px;
  font-size: 0.68rem;
}

.si-btn-tiny {
  padding: 3px 8px;
  font-size: 0.7rem;
  background: rgba(255, 255, 255, 0.05);
  color: var(--ts-text-muted, #94a3b8);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 4px;
  cursor: pointer;
}
.si-btn-tiny:hover { background: rgba(255, 255, 255, 0.1); color: var(--ts-text-primary, #eaecf4); }

/* GitHub PR + pull-from-main controls (Chunk 25.13) */
.si-github {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
}
.si-github-desc {
  margin: 0;
  font-size: 0.8rem;
  color: var(--ts-text-muted, #94a3b8);
}
.si-github-device {
  display: grid;
  gap: 8px;
  padding: 10px;
  border: 1px solid var(--ts-border-soft);
  border-radius: 8px;
  background: var(--ts-surface-1);
}
.si-github-device-copy {
  display: grid;
  gap: 2px;
}
.si-github-device-copy strong {
  color: var(--ts-text-primary);
  font-size: 0.84rem;
}
.si-github-device-copy span {
  color: var(--ts-text-muted);
  font-size: 0.78rem;
  line-height: 1.35;
}
.si-github-device-code {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  color: var(--ts-text-muted);
  font-size: 0.78rem;
}
.si-github-device-code code {
  padding: 3px 8px;
  border-radius: 4px;
  color: var(--ts-text-primary);
  background: var(--ts-bg-input);
  font-size: 0.86rem;
  letter-spacing: 0.08em;
}
.si-github-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 8px;
}
.si-field { display: flex; flex-direction: column; gap: 3px; }
.si-field-label { font-size: 0.7rem; color: var(--ts-text-muted, #94a3b8); }
.si-input {
  padding: 5px 8px;
  background: rgba(0, 0, 0, 0.25);
  color: var(--ts-text-primary, #eaecf4);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 4px;
  font-size: 0.8rem;
}
.si-input:focus { outline: 1px solid var(--ts-accent, #7c6fff); }
.si-github-actions { display: flex; gap: 6px; flex-wrap: wrap; }
.si-github-result {
  margin: 0;
  font-size: 0.78rem;
  padding: 6px 8px;
  border-radius: 4px;
}
.si-github-result--ok {
  background: rgba(34, 197, 94, 0.12);
  color: #4ade80;
}
.si-github-result--warn {
  background: rgba(234, 179, 8, 0.12);
  color: #fbbf24;
}
.si-github-result a { color: inherit; text-decoration: underline; }
</style>
