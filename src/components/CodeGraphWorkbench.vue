<script setup lang="ts">
/**
 * CodeGraphWorkbench — Dense code-intelligence workbench (Chunk 37.11).
 *
 * Layout: 3-panel with repo switcher / status bar.
 * Left: Cluster tree + file tree
 * Center: Graph canvas (placeholder for future Cytoscape/Three.js)
 * Right: References panel + impact details
 */
import { onMounted, ref, watch } from 'vue';
import { useCodeIntelStore } from '../stores/code-intel';

const store = useCodeIntelStore();
const diffRef = ref('HEAD~1');
const impactSymbol = ref('');

onMounted(async () => {
  await store.fetchRepos();
  if (store.activeRepoId !== null) {
    await Promise.all([store.fetchClusters(), store.fetchProcesses()]);
  }
});

watch(
  () => store.activeRepoId,
  async (id) => {
    if (id !== null) {
      await Promise.all([store.fetchClusters(), store.fetchProcesses()]);
    }
  },
);

function handleClusterClick(id: number) {
  store.selectCluster(store.selectedClusterId === id ? null : id);
}

async function runImpact() {
  if (impactSymbol.value.trim()) {
    await store.analyzeImpact(impactSymbol.value.trim());
  }
}

async function runDiffImpact() {
  await store.analyzeDiffImpact(diffRef.value);
}

function riskClass(risk: string): string {
  return `cgw-risk--${risk.toLowerCase()}`;
}
</script>

<template>
  <div
    class="code-graph-workbench"
    data-testid="code-graph-workbench"
  >
    <!-- Status Bar -->
    <header class="cgw-status-bar">
      <select
        v-if="store.repos.length > 0"
        :value="store.activeRepoId"
        class="cgw-repo-select"
        data-testid="repo-select"
        @change="store.setActiveRepo(Number(($event.target as HTMLSelectElement).value))"
      >
        <option
          v-for="repo in store.repos"
          :key="repo.id"
          :value="repo.id"
        >
          {{ repo.label }}
        </option>
      </select>
      <span
        v-else
        class="cgw-no-repo"
      >No repos indexed</span>

      <span class="cgw-stats">
        <span data-testid="cluster-count">{{ store.clusters.length }} clusters</span>
        <span>{{ store.processes.length }} processes</span>
      </span>

      <button
        class="cgw-btn cgw-resync-btn"
        :disabled="store.loading || store.activeRepoId === null"
        title="Re-index the active repository (full re-sync)"
        data-testid="resync-repo"
        @click="store.reIndexRepo()"
      >
        🔄 Re-sync
      </button>

      <span
        v-if="store.loading"
        class="cgw-loading"
      >⏳</span>
      <span
        v-if="store.error"
        class="cgw-error"
        :title="store.error"
      >⚠️</span>
    </header>

    <!-- Main 3-Panel Layout -->
    <div class="cgw-panels">
      <!-- Left Panel: Clusters + Processes -->
      <aside class="cgw-left">
        <section class="cgw-section">
          <h3 class="cgw-section-title">Clusters</h3>
          <ul class="cgw-cluster-list">
            <li
              v-for="cluster in store.clusters"
              :key="cluster.id"
              class="cgw-cluster-item"
              :class="{ 'cgw-cluster-item--active': store.selectedClusterId === cluster.id }"
              :data-testid="`cluster-${cluster.id}`"
              @click="handleClusterClick(cluster.id)"
            >
              <span class="cgw-cluster-label">{{ cluster.label }}</span>
              <span class="cgw-cluster-size">{{ cluster.size }}</span>
            </li>
          </ul>
        </section>

        <section class="cgw-section">
          <h3 class="cgw-section-title">Processes</h3>
          <ul class="cgw-process-list">
            <li
              v-for="(proc, idx) in store.processes.slice(0, 15)"
              :key="idx"
              class="cgw-process-item"
              :data-testid="`process-${idx}`"
            >
              <span class="cgw-process-entry">{{ proc.entry_point }}</span>
              <span class="cgw-process-steps">{{ proc.steps.length }} steps</span>
            </li>
          </ul>
        </section>
      </aside>

      <!-- Center Panel: Graph Canvas -->
      <main class="cgw-center">
        <div
          class="cgw-graph-canvas"
          data-testid="graph-canvas"
        >
          <div
            v-if="store.selectedCluster"
            class="cgw-graph-info"
          >
            <h3>{{ store.selectedCluster.label }}</h3>
            <p>{{ store.selectedCluster.size }} symbols in this cluster</p>
          </div>
          <div
            v-else
            class="cgw-graph-placeholder"
          >
            <p>Select a cluster to visualize</p>
            <p class="cgw-hint">Graph canvas — future Cytoscape/Three.js integration</p>
          </div>
        </div>
      </main>

      <!-- Right Panel: Impact + References -->
      <aside class="cgw-right">
        <section class="cgw-section">
          <h3 class="cgw-section-title">Impact Analysis</h3>
          <div class="cgw-impact-controls">
            <input
              v-model="impactSymbol"
              class="cgw-input"
              placeholder="Symbol name..."
              data-testid="impact-symbol-input"
              @keyup.enter="runImpact"
            />
            <button
              class="cgw-btn"
              data-testid="run-impact"
              @click="runImpact"
            >
              Analyze
            </button>
          </div>

          <div
            v-if="store.impactResult"
            class="cgw-impact-result"
            data-testid="impact-result"
          >
            <p>
              <strong>{{ store.impactResult.symbol }}</strong> —
              {{ store.impactResult.total_affected }} affected
            </p>
            <ul>
              <li
                v-for="group in store.impactResult.by_depth"
                :key="group.depth"
              >
                Depth {{ group.depth }}: {{ group.affected.length }} callers
              </li>
            </ul>
          </div>
        </section>

        <section class="cgw-section">
          <h3 class="cgw-section-title">Diff Impact</h3>
          <div class="cgw-impact-controls">
            <input
              v-model="diffRef"
              class="cgw-input"
              placeholder="HEAD~1"
              data-testid="diff-ref-input"
            />
            <button
              class="cgw-btn"
              data-testid="run-diff-impact"
              @click="runDiffImpact"
            >
              Analyze
            </button>
          </div>

          <div
            v-if="store.diffImpact"
            class="cgw-diff-result"
            data-testid="diff-impact-result"
          >
            <div class="cgw-risk-summary">
              <span class="cgw-risk--critical">{{ store.diffImpact.risk_summary.critical }} critical</span>
              <span class="cgw-risk--high">{{ store.diffImpact.risk_summary.high }} high</span>
              <span class="cgw-risk--moderate">{{ store.diffImpact.risk_summary.moderate }} moderate</span>
              <span class="cgw-risk--low">{{ store.diffImpact.risk_summary.low }} low</span>
            </div>
            <p>{{ store.diffImpact.symbols_changed }} symbols changed, {{ store.diffImpact.total_affected }} affected</p>

            <ul class="cgw-impact-list">
              <li
                v-for="(impact, idx) in store.diffImpact.impacts.slice(0, 10)"
                :key="idx"
                :class="riskClass(impact.risk)"
              >
                <span class="cgw-impact-symbol">{{ impact.symbol.name }}</span>
                <span class="cgw-impact-kind">{{ impact.symbol.kind }}</span>
                <span class="cgw-impact-count">{{ impact.affected_count }} affected</span>
              </li>
            </ul>
          </div>
        </section>
      </aside>
    </div>
  </div>
</template>

<style scoped>
.code-graph-workbench {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--ts-bg-surface, #1a1a2e);
  color: var(--ts-text-primary, #e0e0e0);
  font-family: var(--ts-font-mono, monospace);
  font-size: 0.85rem;
}

.cgw-status-bar {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 0.5rem 1rem;
  background: var(--ts-bg-elevated, #16213e);
  border-bottom: 1px solid var(--ts-border, #333);
}

.cgw-repo-select {
  background: var(--ts-bg-input, #0f3460);
  color: var(--ts-text-primary, #e0e0e0);
  border: 1px solid var(--ts-border, #444);
  border-radius: 4px;
  padding: 0.25rem 0.5rem;
}

.cgw-stats {
  display: flex;
  gap: 0.75rem;
  opacity: 0.7;
}

.cgw-loading {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.cgw-panels {
  display: grid;
  grid-template-columns: 240px 1fr 300px;
  flex: 1;
  overflow: hidden;
}

.cgw-left,
.cgw-right {
  overflow-y: auto;
  border-right: 1px solid var(--ts-border, #333);
  padding: 0.5rem;
}

.cgw-right {
  border-right: none;
  border-left: 1px solid var(--ts-border, #333);
}

.cgw-center {
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
}

.cgw-section {
  margin-bottom: 1rem;
}

.cgw-section-title {
  font-size: 0.75rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  opacity: 0.6;
  margin: 0 0 0.5rem;
  padding: 0 0.25rem;
}

.cgw-cluster-list,
.cgw-process-list,
.cgw-impact-list {
  list-style: none;
  margin: 0;
  padding: 0;
}

.cgw-cluster-item {
  display: flex;
  justify-content: space-between;
  padding: 0.3rem 0.5rem;
  border-radius: 4px;
  cursor: pointer;
  transition: background 0.15s;
}

.cgw-cluster-item:hover {
  background: var(--ts-bg-hover, rgba(255, 255, 255, 0.05));
}

.cgw-cluster-item--active {
  background: var(--ts-accent-muted, rgba(100, 180, 255, 0.15));
  border-left: 2px solid var(--ts-accent, #64b4ff);
}

.cgw-cluster-size {
  opacity: 0.5;
  font-size: 0.75rem;
}

.cgw-process-item {
  display: flex;
  justify-content: space-between;
  padding: 0.25rem 0.5rem;
  font-size: 0.8rem;
}

.cgw-process-entry {
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 150px;
}

.cgw-process-steps {
  opacity: 0.5;
  font-size: 0.7rem;
}

.cgw-graph-canvas {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--ts-bg-canvas, #0d1117);
  border-radius: 8px;
  margin: 0.5rem;
}

.cgw-graph-placeholder {
  text-align: center;
  opacity: 0.5;
}

.cgw-hint {
  font-size: 0.7rem;
  margin-top: 0.5rem;
}

.cgw-impact-controls {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 0.5rem;
}

.cgw-input {
  flex: 1;
  background: var(--ts-bg-input, #0f3460);
  color: var(--ts-text-primary, #e0e0e0);
  border: 1px solid var(--ts-border, #444);
  border-radius: 4px;
  padding: 0.25rem 0.5rem;
  font-size: 0.8rem;
}

.cgw-btn {
  background: var(--ts-accent, #64b4ff);
  color: var(--ts-bg-surface, #1a1a2e);
  border: none;
  border-radius: 4px;
  padding: 0.25rem 0.75rem;
  font-size: 0.8rem;
  cursor: pointer;
  font-weight: 600;
}

.cgw-btn:hover {
  opacity: 0.85;
}

.cgw-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.cgw-resync-btn {
  margin-left: auto;
}

.cgw-risk-summary {
  display: flex;
  gap: 0.5rem;
  flex-wrap: wrap;
  margin-bottom: 0.5rem;
}

.cgw-risk--critical { color: var(--ts-error); font-weight: bold; }
  .cgw-risk--high { color: var(--ts-warning); }
  .cgw-risk--moderate { color: var(--ts-warning); opacity: 0.85; }
  .cgw-risk--low { color: var(--ts-success); }

.cgw-impact-list li {
  display: flex;
  gap: 0.5rem;
  padding: 0.2rem 0;
  align-items: center;
}

.cgw-impact-symbol {
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 120px;
}

.cgw-impact-kind {
  opacity: 0.5;
  font-size: 0.7rem;
}

.cgw-impact-count {
  margin-left: auto;
  opacity: 0.7;
  font-size: 0.75rem;
}
</style>
