import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  CodingLlmConfig,
  CodingLlmRecommendation,
  SelfImproveMetrics,
  SelfImproveRun,
  SelfImproveSettings,
} from '../types';

/**
 * A single phase of the self-improve roadmap. Surfaced in the progress UI.
 *
 * `status` is derived from runtime state where possible (e.g. coding-llm
 * config presence) and falls back to the static `not-started` default
 * for phases that need future chunks to land first.
 */
export interface SelfImprovePhase {
  id: string;
  title: string;
  description: string;
  status: 'not-started' | 'in-progress' | 'completed' | 'blocked';
  blockedReason?: string;
}

/**
 * A live activity entry shown in the progress feed (most-recent first).
 * Populated by the autonomous loop in future chunks; for now only
 * "user toggled X" entries appear.
 */
export interface SelfImproveActivity {
  id: string;
  timestamp: number;
  level: 'info' | 'warn' | 'error' | 'success';
  message: string;
}

export interface SelfImproveChunk {
  id: string;
  title: string;
  description: string;
  trigger: string;
  priority: 'high' | 'medium' | 'low';
  status: 'queued' | 'ready' | 'blocked';
}

/**
 * Phase 25 — Self-Improve foundation store.
 *
 * Tracks the dedicated "coding LLM" configuration and the self-improve
 * toggle. The toggle is persisted via Tauri but the autonomous loop is
 * intentionally NOT implemented yet — see `rules/milestones.md` Phase 25
 * for the full roadmap.
 *
 * The store exposes two key flows used by the pet-mode context menu:
 *  - {@link enable} — guards against missing coding-LLM config and surfaces
 *    a typed error so the UI can route the user to setup.
 *  - {@link disable} — always succeeds; the only way to stop self-improve.
 */
export const useSelfImproveStore = defineStore('self-improve', () => {
  const settings = ref<SelfImproveSettings>({
    enabled: false,
    updated_at: 0,
    last_acknowledged_at: 0,
    last_provider: '',
  });
  const codingLlm = ref<CodingLlmConfig | null>(null);
  const recommendations = ref<CodingLlmRecommendation[]>([]);
  const lastError = ref<string | null>(null);
  const isLoading = ref(false);
  const activity = ref<SelfImproveActivity[]>([]);
  /** Live runtime status: whether the Rust loop is currently running. */
  const running = ref(false);
  /** Cached "OS auto-launch on login" status (Windows-only effect). */
  const autostartEnabled = ref(false);
  /** Currently-active phase id reported by the engine, if any. */
  const activePhase = ref<string | null>(null);
  /** Latest progress percent reported by the engine (0-100). */
  const livePercent = ref(0);
  /** Latest live message from the engine, used as a status pill. */
  const liveMessage = ref<string>('Idle');
  /** Reachability test result for the BrainView "Test connection" button. */
  const reachability = ref<{
    ok: boolean;
    summary: string;
    detail?: string | null;
  } | null>(null);
  /** Aggregate observability stats (success/fail rates, last error). */
  const metrics = ref<SelfImproveMetrics>({
    total_runs: 0,
    successes: 0,
    failures: 0,
    success_rate: 0,
    failure_rate: 0,
    avg_duration_ms: 0,
    last_error: null,
    last_error_chunk: null,
    last_error_at_ms: 0,
    total_prompt_tokens: 0,
    total_completion_tokens: 0,
    total_cost_usd: 0,
    rolling_7d_runs: 0,
    rolling_7d_prompt_tokens: 0,
    rolling_7d_completion_tokens: 0,
    rolling_7d_cost_usd: 0,
    cost_by_provider: {},
  });
  /** Most-recent persisted run records (newest first). */
  const runs = ref<SelfImproveRun[]>([]);
  let unlistenProgress: UnlistenFn | null = null;

  /**
   * Static roadmap, mirrored from `rules/milestones.md` Phase 25.
   * Each phase exposes a `status` that reactive computeds may flip when
   * runtime conditions are met (e.g. `coding-llm` flips to `completed`
   * once a coding LLM is configured).
   */
  const ROADMAP: SelfImprovePhase[] = [
    {
      id: 'foundation',
      title: 'Foundation: toggle, types, persistence',
      description: 'Pet-mode menu item, confirm dialog, persisted settings, coding-LLM picker.',
      status: 'completed',
    },
    {
      id: 'coding-llm',
      title: 'Configure dedicated coding LLM',
      description: 'Pick Claude / OpenAI / DeepSeek and validate API key reachability.',
      status: 'not-started',
    },
    {
      id: 'progress-ui',
      title: 'Live progress UI',
      description: 'Real-time phase tracker, activity feed, and live progress bar.',
      status: 'in-progress',
    },
    {
      id: 'github-bind',
      title: 'GitHub repo binding',
      description: 'Detect/clone TerranSoul repo, ensure feature branch, OAuth device flow.',
      status: 'not-started',
    },
    {
      id: 'autonomy-loop',
      title: 'Autonomous coding loop',
      description: 'Read milestones.md, drive coding LLM, write changes to feature branch, open PR.',
      status: 'not-started',
      blockedReason: 'Foundation + GitHub binding required first',
    },
    {
      id: 'mcp-self-host',
      title: 'Self-host & self-improve MCP server',
      description: 'Auto-spawn local MCP server, allow loop to extend its own tools.',
      status: 'not-started',
    },
    {
      id: 'brain-data-migrate',
      title: 'Brain data migration & optimisation',
      description: 'Per `docs/brain-advanced-design.md` — auto-run schema migrations + ANN rebuilds.',
      status: 'not-started',
    },
    {
      id: 'service-tray',
      title: 'System tray + Windows auto-start',
      description: 'Tray icon, login auto-start (registry Run key, reversible).',
      status: 'not-started',
    },
    {
      id: 'resilience',
      title: 'Resilience: resume on app/computer restart',
      description: 'Persist task queue in SQLite; auto-resume on launch; only "untick" stops it.',
      status: 'not-started',
    },
    {
      id: 'failure-triage-radar',
      title: 'Failure triage & improvement radar',
      description: 'When a self-improve run hits a bug, convert the failure into a scoped repair chunk with evidence and verification gates.',
      status: 'not-started',
    },
    {
      id: 'online-tool-model-radar',
      title: 'Online tool/model/API research radar',
      description: 'Regularly compare better current approaches, model APIs, and developer tools before planning self-improve work.',
      status: 'not-started',
    },
    {
      id: 'redis-code-memory',
      title: 'Redis-backed code memory option',
      description: 'Evaluate Redis hybrid/vector search as an optional codebase memory index for faster self-improve retrieval.',
      status: 'not-started',
    },
  ];
  const phases = ref<SelfImprovePhase[]>(ROADMAP.map((p) => ({ ...p })));

  /** Phases reactively derived from real state (overrides ROADMAP defaults). */
  const livePhases = computed<SelfImprovePhase[]>(() =>
    phases.value.map((p) => {
      if (p.id === 'coding-llm') {
        return {
          ...p,
          status: codingLlm.value ? 'completed' : 'not-started',
        };
      }
      if (p.id === 'progress-ui') {
        // This panel itself == the deliverable for this phase, so it's done
        // the moment self-improve is configured.
        return {
          ...p,
          status: codingLlm.value ? 'completed' : 'in-progress',
        };
      }
      return p;
    }),
  );

  const completedCount = computed(
    () => livePhases.value.filter((p) => p.status === 'completed').length,
  );
  const totalCount = computed(() => livePhases.value.length);
  const progressPercent = computed(() =>
    totalCount.value === 0 ? 0 : Math.round((completedCount.value / totalCount.value) * 100),
  );
  const nextPhase = computed<SelfImprovePhase | null>(
    () => livePhases.value.find((p) => p.status === 'not-started' || p.status === 'in-progress') ?? null,
  );

  const improvementChunks = computed<SelfImproveChunk[]>(() => {
    const failedRuns = runs.value.filter((run) => run.outcome === 'failure');
    const chunks: SelfImproveChunk[] = [];
    if (metrics.value.last_error || failedRuns.length > 0) {
      const chunk = metrics.value.last_error_chunk ?? failedRuns[0]?.chunk_id ?? 'unknown';
      chunks.push({
        id: 'bug-triage-from-run',
        title: 'Bug triage from latest self-improve run',
        description: metrics.value.last_error
          ? `Capture the failure from ${chunk}, find a safer implementation path, and add a verification gate before retry.`
          : `Review ${failedRuns.length} failed run(s), group recurring errors, and promote the smallest repair chunk.`,
        trigger: 'Runtime/test failure observed during self-improve',
        priority: 'high',
        status: 'ready',
      });
    }
    chunks.push(
      {
        id: 'research-better-approach',
        title: 'Research better implementation approach online',
        description: 'Before non-trivial self-improve work, compare official docs and recent engineering writeups; record the chosen approach in the chunk plan.',
        trigger: 'Any new feature, bug fix, tool integration, or API decision',
        priority: 'medium',
        status: 'queued',
      },
      {
        id: 'redis-vector-memory-scout',
        title: 'Scout Redis hybrid/vector search for code memory',
        description: 'Evaluate Redis as an optional semantic code/documentation index for self-improve retrieval, with SQLite/usearch kept as the default path.',
        trigger: 'Large-codebase retrieval, repeated context misses, or RAG performance bottlenecks',
        priority: 'medium',
        status: 'queued',
      },
      {
        id: 'model-tool-news-radar',
        title: 'Refresh model, tool, and API recommendations',
        description: 'Check current model/API/tool news and benchmarks so the coding-LLM picker and autonomous loop can prefer stronger, cheaper, or safer options.',
        trigger: 'Provider errors, high cost, stale recommendations, or monthly self-improve review',
        priority: 'low',
        status: 'queued',
      },
    );
    return chunks;
  });

  function logActivity(level: SelfImproveActivity['level'], message: string): void {
    activity.value.unshift({
      id: `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      timestamp: Date.now(),
      level,
      message,
    });
    // Bound the in-memory feed; persisted history will land in a future chunk.
    if (activity.value.length > 100) activity.value.length = 100;
  }

  const isEnabled = computed(() => settings.value.enabled);
  const isConfigured = computed(() => codingLlm.value !== null);
  const canEnable = computed(() => isConfigured.value);

  async function loadSettings(): Promise<void> {
    try {
      settings.value = await invoke<SelfImproveSettings>('get_self_improve_settings');
    } catch (e) {
      lastError.value = String(e);
    }
  }

  async function loadCodingLlm(): Promise<void> {
    try {
      codingLlm.value = await invoke<CodingLlmConfig | null>('get_coding_llm_config');
    } catch (e) {
      lastError.value = String(e);
    }
  }

  async function loadRecommendations(): Promise<void> {
    try {
      const recs = await invoke<CodingLlmRecommendation[] | null>(
        'list_coding_llm_recommendations',
      );
      // Defensive: when the Tauri backend is unavailable (or a test mock
      // returns null), keep the default empty array so downstream
      // `.find` / `.length` calls remain safe.
      if (Array.isArray(recs)) recommendations.value = recs;
    } catch (e) {
      lastError.value = String(e);
    }
  }

  async function initialise(): Promise<void> {
    isLoading.value = true;
    try {
      await Promise.allSettled([
        loadSettings(),
        loadCodingLlm(),
        loadRecommendations(),
        loadStatus(),
        loadMetrics(),
        loadRuns(),
        loadGithubConfig(),
      ]);
      await subscribeToProgress();
    } finally {
      isLoading.value = false;
    }
  }

  /** Refresh aggregate stats from the persisted JSONL log. */
  async function loadMetrics(): Promise<void> {
    try {
      const m = await invoke<SelfImproveMetrics | null>('get_self_improve_metrics');
      if (m) metrics.value = m;
    } catch (e) {
      console.warn('[self-improve] metrics load failed:', e);
    }
  }

  /** Refresh the recent-runs list (newest first). */
  async function loadRuns(limit = 100): Promise<void> {
    try {
      const r = await invoke<SelfImproveRun[] | null>('get_self_improve_runs', { limit });
      if (Array.isArray(r)) runs.value = r;
    } catch (e) {
      console.warn('[self-improve] runs load failed:', e);
    }
  }

  /** Wipe the persisted run log. UI calls this from a "Clear log" button. */
  async function clearRunLog(): Promise<void> {
    try {
      const m = await invoke<SelfImproveMetrics | null>('clear_self_improve_log');
      if (m) metrics.value = m;
      runs.value = [];
      logActivity('info', 'Run log cleared');
    } catch (e) {
      lastError.value = String(e);
      logActivity('error', `Clear log failed: ${String(e)}`);
    }
  }

  /**
   * Status snapshot from Rust — running flag, autostart state, etc.
   * Polled on focus by the SelfImprovePanel and refreshed after every
   * state-changing command so the UI never lies about "running".
   */
  async function loadStatus(): Promise<void> {
    try {
      const s = await invoke<{
        running: boolean;
        enabled: boolean;
        has_coding_llm: boolean;
        autostart_enabled: boolean;
      } | null>('get_self_improve_status');
      if (s) {
        running.value = s.running;
        autostartEnabled.value = s.autostart_enabled;
      }
    } catch (e) {
      // Status is informational; do not surface as a hard error.
      console.warn('[self-improve] status load failed:', e);
    }
  }

  /**
   * Subscribe to `self-improve-progress` Tauri events. Idempotent — on
   * repeat calls the previous listener is unhooked first.
   */
  async function subscribeToProgress(): Promise<void> {
    if (unlistenProgress) {
      unlistenProgress();
      unlistenProgress = null;
    }
    try {
      unlistenProgress = await listen<{
        phase: string;
        message: string;
        progress: number;
        chunk_id: string | null;
        level: 'info' | 'success' | 'warn' | 'error';
      }>('self-improve-progress', (evt) => {
        const p = evt.payload;
        activePhase.value = p.phase;
        if (typeof p.progress === 'number') livePercent.value = p.progress;
        liveMessage.value = p.message;
        const decorated = p.chunk_id ? `[${p.chunk_id}] ${p.message}` : p.message;
        logActivity(p.level ?? 'info', decorated);
        // Mirror the engine's terminal phases into running flag.
        if (p.phase === 'stopped' || p.phase === 'exit') running.value = false;
        if (p.phase === 'startup') running.value = true;
        // Refresh metrics + run list whenever a planning round completes
        // (success or failure) so observability stays in sync with the
        // append-only log.
        if (p.phase === 'complete' || (p.phase === 'plan' && p.level === 'error')) {
          void loadMetrics();
          void loadRuns();
        }
      });
    } catch (e) {
      console.warn('[self-improve] event subscribe failed:', e);
    }
  }

  /** Probe the configured coding LLM — populates `reachability`. */
  async function testCodingLlmConnection(): Promise<void> {
    reachability.value = null;
    try {
      reachability.value = await invoke<{
        ok: boolean;
        summary: string;
        detail?: string | null;
      }>('test_coding_llm_connection');
      logActivity(
        reachability.value.ok ? 'success' : 'warn',
        `Coding LLM probe: ${reachability.value.summary}`,
      );
    } catch (e) {
      reachability.value = { ok: false, summary: 'Probe failed', detail: String(e) };
      logActivity('error', `Coding LLM probe failed: ${String(e)}`);
    }
  }

  /** Start the autonomous loop. The toggle must already be enabled. */
  async function startEngine(): Promise<void> {
    try {
      await invoke('start_self_improve');
      running.value = true;
      logActivity('success', 'Self-improve loop started');
    } catch (e) {
      lastError.value = String(e);
      logActivity('error', `Start failed: ${String(e)}`);
      throw e;
    }
  }

  /** Stop the autonomous loop. Idempotent. */
  async function stopEngine(): Promise<void> {
    try {
      await invoke('stop_self_improve');
      running.value = false;
      logActivity('info', 'Self-improve loop stop requested');
    } catch (e) {
      lastError.value = String(e);
    }
  }

  /** Toggle Windows launch-on-login. Returns the resulting effective state. */
  async function setAutostart(enabled: boolean): Promise<boolean> {
    try {
      const result = await invoke<boolean>('set_self_improve_autostart', { enabled });
      autostartEnabled.value = result;
      logActivity('info', `Autostart-on-login: ${result ? 'enabled' : 'disabled'}`);
      return result;
    } catch (e) {
      lastError.value = String(e);
      logActivity('error', `Autostart toggle failed: ${String(e)}`);
      throw e;
    }
  }

  async function setCodingLlm(config: CodingLlmConfig | null): Promise<void> {
    await invoke('set_coding_llm_config', { config });
    codingLlm.value = config;
    logActivity(
      config ? 'success' : 'info',
      config
        ? `Coding LLM set to ${config.provider} · ${config.model}`
        : 'Coding LLM configuration cleared',
    );
  }

  /**
   * Probe `127.0.0.1:11434/api/tags` and return the names of locally
   * installed Ollama models. Empty array when Ollama is not running.
   * Used by the BrainView "Local Ollama" recommendation card to populate
   * the model dropdown without making the user type a model name.
   */
  async function loadLocalCodingModels(baseUrl?: string): Promise<string[]> {
    try {
      const models = await invoke<string[]>('list_local_coding_models', {
        baseUrl: baseUrl ?? null,
      });
      return Array.isArray(models) ? models : [];
    } catch (e) {
      lastError.value = String(e);
      return [];
    }
  }

  /**
   * Enable self-improve. Returns the updated settings.
   *
   * Throws (rejects) when no coding LLM is configured — callers should
   * catch and route the user to the Brain → Coding LLM panel.
   */
  async function enable(): Promise<SelfImproveSettings> {
    lastError.value = null;
    try {
      const next = await invoke<SelfImproveSettings>('set_self_improve_enabled', {
        enabled: true,
      });
      settings.value = next;
      logActivity('success', `Self-improve enabled (provider: ${next.last_provider || 'n/a'})`);
      // Best-effort start the autonomous loop. A failure here is logged
      // but does not roll back the toggle — the user may have intentionally
      // enabled the toggle without an immediately-reachable LLM.
      try {
        await startEngine();
      } catch (e) {
        console.warn('[self-improve] enable: start engine failed:', e);
      }
      return next;
    } catch (e) {
      lastError.value = String(e);
      logActivity('error', `Failed to enable self-improve: ${String(e)}`);
      throw e;
    }
  }

  /** Disable self-improve. Always succeeds; also stops the running loop. */
  async function disable(): Promise<SelfImproveSettings> {
    const next = await invoke<SelfImproveSettings>('set_self_improve_enabled', {
      enabled: false,
    });
    settings.value = next;
    // Stop the engine first so the "stop requested" entry is older than
    // the final "disabled" announcement (most-recent-first feed).
    try {
      await stopEngine();
    } catch (e) {
      console.warn('[self-improve] disable: stop engine failed:', e);
    }
    logActivity('info', 'Self-improve disabled');
    return next;
  }

  // ── GitHub PR automation (Chunk 25.13) ────────────────────────────────
  /** Persisted GitHub binding (token, owner/repo, base branch, reviewers). */
  const githubConfig = ref<{
    token: string;
    owner: string;
    repo: string;
    default_base: string;
    reviewers: string[];
  } | null>(null);
  /** Last opened/found Pull Request, populated by `openPullRequest`. */
  const lastPullRequest = ref<{
    number: number;
    html_url: string;
    state: string;
    head_branch: string;
    base_branch: string;
    created: boolean;
  } | null>(null);
  /** Last `pull_main` outcome, populated by `pullFromMain`. */
  const lastPullResult = ref<{
    merged: boolean;
    fast_forward: boolean;
    already_up_to_date: boolean;
    resolved_conflicts: string[];
    unresolved_conflicts: string[];
    message: string;
  } | null>(null);

  async function loadGithubConfig(): Promise<void> {
    try {
      githubConfig.value = await invoke('get_github_config');
    } catch (e) {
      console.warn('[self-improve] github config load failed:', e);
    }
  }

  async function setGithubConfig(
    cfg: {
      token: string;
      owner: string;
      repo: string;
      default_base: string;
      reviewers: string[];
    } | null,
  ): Promise<void> {
    try {
      const stored = await invoke<typeof githubConfig.value>('set_github_config', {
        config: cfg,
      });
      githubConfig.value = stored;
      logActivity(
        cfg ? 'success' : 'info',
        cfg ? `GitHub config saved (${stored?.owner}/${stored?.repo})` : 'GitHub config cleared',
      );
    } catch (e) {
      lastError.value = String(e);
      logActivity('error', `GitHub config save failed: ${String(e)}`);
      throw e;
    }
  }

  async function openPullRequest(): Promise<void> {
    try {
      lastPullRequest.value = await invoke('open_self_improve_pr');
      const pr = lastPullRequest.value!;
      logActivity('success', `PR #${pr.number} ${pr.created ? 'opened' : 'already open'}: ${pr.html_url}`);
    } catch (e) {
      lastError.value = String(e);
      logActivity('error', `Open PR failed: ${String(e)}`);
      throw e;
    }
  }

  async function pullFromMain(): Promise<void> {
    try {
      lastPullResult.value = await invoke('pull_main_for_self_improve');
      const r = lastPullResult.value!;
      logActivity(r.merged ? 'success' : 'warn', `Pull main: ${r.message}`);
    } catch (e) {
      lastError.value = String(e);
      logActivity('error', `Pull main failed: ${String(e)}`);
      throw e;
    }
  }

  return {
    settings,
    codingLlm,
    recommendations,
    lastError,
    isLoading,
    activity,
    running,
    autostartEnabled,
    activePhase,
    livePercent,
    liveMessage,
    reachability,
    metrics,
    runs,
    phases: livePhases,
    completedCount,
    totalCount,
    progressPercent,
    nextPhase,
    improvementChunks,
    isEnabled,
    isConfigured,
    canEnable,
    githubConfig,
    lastPullRequest,
    lastPullResult,
    loadSettings,
    loadCodingLlm,
    loadRecommendations,
    loadStatus,
    initialise,
    setCodingLlm,
    loadLocalCodingModels,
    enable,
    disable,
    logActivity,
    subscribeToProgress,
    testCodingLlmConnection,
    startEngine,
    stopEngine,
    setAutostart,
    loadMetrics,
    loadRuns,
    clearRunLog,
    loadGithubConfig,
    setGithubConfig,
    openPullRequest,
    pullFromMain,
  };
});
