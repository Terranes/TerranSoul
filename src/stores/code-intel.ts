/**
 * Code Intelligence store (Chunk 37.11).
 *
 * Pinia store that communicates with the TerranSoul code-intelligence backend
 * via Tauri IPC commands. Provides reactive state for the Code Workbench UI:
 * repos, clusters, processes, symbols, and impact analysis.
 */
import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';

// ─── Types ──────────────────────────────────────────────────────────────────

export interface CodeRepo {
  id: number;
  path: string;
  label: string;
  indexed_at: string;
}

export interface CodeCluster {
  id: number;
  label: string;
  size: number;
  symbol_ids: number[];
}

export interface ProcessStep {
  symbol_id: number;
  name: string;
  file: string;
  line: number;
  depth: number;
}

export interface CodeProcess {
  entry_point: string;
  entry_file: string;
  entry_line: number;
  steps: ProcessStep[];
}

export interface CodeSymbol {
  id: number;
  name: string;
  kind: string;
  file: string;
  line: number;
  end_line?: number;
  parent?: string;
  exported: boolean;
  cluster_id?: number;
}

export interface ImpactResult {
  symbol: string;
  total_affected: number;
  by_depth: { depth: number; affected: { symbol: string; file: string; line: number }[] }[];
}

export interface DiffImpactReport {
  diff_ref: string;
  files_changed: number;
  symbols_changed: number;
  total_affected: number;
  risk_summary: { critical: number; high: number; moderate: number; low: number };
  impacts: {
    symbol: { name: string; kind: string; file: string; line: number };
    risk: string;
    affected_count: number;
    affected: { name: string; file: string; line: number; depth: number }[];
  }[];
}

// ─── Store ──────────────────────────────────────────────────────────────────

export const useCodeIntelStore = defineStore('code-intel', () => {
  // State
  const repos = ref<CodeRepo[]>([]);
  const activeRepoId = ref<number | null>(null);
  const clusters = ref<CodeCluster[]>([]);
  const processes = ref<CodeProcess[]>([]);
  const selectedClusterId = ref<number | null>(null);
  const selectedSymbol = ref<string | null>(null);
  const impactResult = ref<ImpactResult | null>(null);
  const diffImpact = ref<DiffImpactReport | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);

  // Computed
  const activeRepo = computed(() =>
    repos.value.find((r) => r.id === activeRepoId.value) ?? null,
  );

  const selectedCluster = computed(() =>
    clusters.value.find((c) => c.id === selectedClusterId.value) ?? null,
  );

  // Actions
  async function fetchRepos() {
    try {
      loading.value = true;
      error.value = null;
      repos.value = await invoke<CodeRepo[]>('code_list_repos');
      if (repos.value.length > 0 && activeRepoId.value === null) {
        activeRepoId.value = repos.value[0].id;
      }
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function fetchClusters() {
    if (activeRepoId.value === null) return;
    try {
      loading.value = true;
      clusters.value = await invoke<CodeCluster[]>('code_list_clusters', {
        repoId: activeRepoId.value,
      });
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function fetchProcesses() {
    if (activeRepoId.value === null) return;
    try {
      loading.value = true;
      processes.value = await invoke<CodeProcess[]>('code_list_processes', {
        repoId: activeRepoId.value,
      });
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function analyzeImpact(symbol: string, depth = 5) {
    try {
      loading.value = true;
      impactResult.value = await invoke<ImpactResult>('code_analyze_impact', {
        symbol,
        depth,
        repoId: activeRepoId.value,
      });
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function analyzeDiffImpact(diffRef = 'HEAD~1') {
    try {
      loading.value = true;
      diffImpact.value = await invoke<DiffImpactReport>('code_analyze_diff_impact', {
        diffRef,
        repoId: activeRepoId.value,
      });
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  function selectCluster(id: number | null) {
    selectedClusterId.value = id;
  }

  function selectSymbol(name: string | null) {
    selectedSymbol.value = name;
  }

  function setActiveRepo(id: number) {
    activeRepoId.value = id;
    clusters.value = [];
    processes.value = [];
    impactResult.value = null;
    diffImpact.value = null;
  }

  /** Re-index the currently active repo (full re-sync of files + edges). */
  async function reIndexRepo() {
    const repo = activeRepo.value;
    if (!repo) return;
    try {
      loading.value = true;
      error.value = null;
      await invoke('code_index_repo', { repoPath: repo.path });
      await invoke('code_resolve_edges', { repoPath: repo.path });
      await Promise.all([fetchRepos(), fetchClusters(), fetchProcesses()]);
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  return {
    // State
    repos,
    activeRepoId,
    clusters,
    processes,
    selectedClusterId,
    selectedSymbol,
    impactResult,
    diffImpact,
    loading,
    error,
    // Computed
    activeRepo,
    selectedCluster,
    // Actions
    fetchRepos,
    fetchClusters,
    fetchProcesses,
    analyzeImpact,
    analyzeDiffImpact,
    selectCluster,
    selectSymbol,
    setActiveRepo,
    reIndexRepo,
  };
});
