<template>
  <div class="memory-view">
    <header class="mv-header">
      <h2>🧠 Memory</h2>
      <div class="mv-header-actions">
        <button
          class="btn-secondary"
          :disabled="isActing"
          @click="handleExtract"
        >
          {{ isActing ? 'Working…' : '⬇ Extract from session' }}
        </button>
        <button
          class="btn-secondary"
          :disabled="isActing"
          @click="handleSummarize"
        >
          📄 Summarize session
        </button>
        <button
          class="btn-secondary"
          :disabled="isActing"
          title="Apply time-decay to all memories"
          @click="handleDecay"
        >
          ⏳ Decay
        </button>
        <button
          class="btn-secondary"
          :disabled="isActing"
          title="Remove fully decayed memories"
          @click="handleGC"
        >
          🧹 GC
        </button>
        <button
          class="btn-primary"
          @click="showAdd = true"
        >
          ＋ Add memory
        </button>
        <button
          class="btn-secondary"
          data-testid="mv-obsidian-export"
          :disabled="isActing"
          @click="showObsidianExport = true"
        >
          📓 Export to Obsidian
        </button>
      </div>
    </header>

    <p
      v-if="feedback"
      class="mv-feedback"
    >
      {{ feedback }}
    </p>

    <!-- Stats dashboard -->
    <div
      v-if="store.stats"
      class="mv-stats"
    >
      <div class="mv-stat">
        <span class="mv-stat-value">{{ store.stats.total }}</span>
        <span class="mv-stat-label">Total</span>
      </div>
      <div class="mv-stat tier-short">
        <span class="mv-stat-value">{{ store.stats.short_count }}</span>
        <span class="mv-stat-label">Short</span>
      </div>
      <div class="mv-stat tier-working">
        <span class="mv-stat-value">{{ store.stats.working_count }}</span>
        <span class="mv-stat-label">Working</span>
      </div>
      <div class="mv-stat tier-long">
        <span class="mv-stat-value">{{ store.stats.long_count }}</span>
        <span class="mv-stat-label">Long</span>
      </div>
      <div class="mv-stat">
        <span class="mv-stat-value">{{ formatTokens(store.stats.total_tokens) }}</span>
        <span class="mv-stat-label">Tokens</span>
      </div>
      <div class="mv-stat">
        <span class="mv-stat-value">{{ store.stats.avg_decay.toFixed(2) }}</span>
        <span class="mv-stat-label">Avg Decay</span>
      </div>
    </div>

    <section class="mv-rag-config">
      <div class="mv-storage-summary">
        <strong>Memory configuration</strong>
        <span>Brain memory &amp; RAG in memory: {{ formatBytes(memoryCacheBytes) }} / {{ maxMemoryMb.toFixed(0) }} MB</span>
      </div>
      <label class="mv-storage-control">
        <span>Maximum in-memory RAG cache</span>
        <input
          v-model.number="maxMemoryMb"
          type="range"
          min="1"
          max="1024"
          step="1"
          @change="saveMemoryCacheCap"
        >
      </label>
      <label class="mv-storage-number">
        <input
          v-model.number="maxMemoryMb"
          type="number"
          min="1"
          max="1024"
          step="1"
          @change="saveMemoryCacheCap"
        >
        <span>MB</span>
      </label>
    </section>

    <section class="mv-rag-config">
      <div class="mv-storage-summary">
        <strong>Storage configuration</strong>
        <span>Brain memory &amp; RAG in storage: {{ formatBytes(memoryStorageBytes) }} / {{ maxMemoryGb.toFixed(1) }} GB</span>
      </div>
      <label class="mv-storage-control">
        <span>Maximum persistent RAG storage</span>
        <input
          v-model.number="maxMemoryGb"
          type="range"
          min="1"
          max="100"
          step="0.5"
          @change="saveMemoryCap"
        >
      </label>
      <label class="mv-storage-number">
        <input
          v-model.number="maxMemoryGb"
          type="number"
          min="1"
          max="100"
          step="0.5"
          @change="saveMemoryCap"
        >
        <span>GB</span>
      </label>
    </section>

    <!-- Tabs -->
    <nav class="mv-tabs">
      <button
        v-for="tab in tabs"
        :key="tab"
        :class="['mv-tab', { active: activeTab === tab }]"
        @click="activeTab = tab"
      >
        {{ tab }}
      </button>
    </nav>

    <!-- ── Graph tab ── -->
    <div
      v-if="activeTab === 'Graph'"
      class="mv-graph-panel"
    >
      <div class="mv-graph-main">
        <div class="mv-graph-toolbar">
          <label class="mv-graph-toggle">
            <span>Edges:</span>
            <select
              v-model="edgeMode"
              class="mv-edge-mode"
            >
              <option value="typed">Typed (knowledge graph)</option>
              <option value="tag">Tag co-occurrence</option>
              <option value="both">Both</option>
            </select>
          </label>
          <label class="mv-graph-toggle">
            <input
              v-model="graph3d"
              type="checkbox"
              data-testid="mv-graph-3d-toggle"
            >
            <span>3-D</span>
          </label>
          <button
            class="btn-secondary"
            :disabled="isActing || store.memories.length < 2"
            :title="store.memories.length < 2 ? 'Add at least 2 memories first' : 'Use the brain to propose edges'"
            @click="handleExtractEdges"
          >
            🔗 Extract edges
          </button>
          <span
            v-if="store.edgeStats"
            class="mv-edge-counter"
          >
            {{ store.edgeStats.total_edges }} edge{{ store.edgeStats.total_edges === 1 ? '' : 's' }}
            · {{ store.edgeStats.connected_memories }} connected
          </span>
        </div>
        <BrainGraphViewport
          v-if="graph3d"
          :memories="store.memories"
          :edges="store.edges"
          @select="onNodeSelect"
        />
        <MemoryGraph
          v-else
          :memories="store.memories"
          :edges="store.edges"
          :edge-mode="edgeMode"
          @select="onNodeSelect"
        />
      </div>
      <aside
        v-if="selectedEntry"
        class="mv-node-detail"
      >
        <h3>{{ selectedEntry.content }}</h3>
        <p><strong>Type:</strong> {{ selectedEntry.memory_type }}</p>
        <p><strong>Tier:</strong> <span :class="'mv-tier-badge tier-' + selectedEntry.tier">{{ selectedEntry.tier }}</span></p>
        <p><strong>Tags:</strong> {{ selectedEntry.tags || '—' }}</p>
        <p><strong>Importance:</strong> {{ '★'.repeat(selectedEntry.importance) }}</p>
        <p><strong>Decay:</strong> {{ (selectedEntry.decay_score * 100).toFixed(0) }}%</p>
        <p><strong>Accessed:</strong> {{ selectedEntry.access_count }}×</p>
        <div
          v-if="selectedEdges.length"
          class="mv-node-edges"
        >
          <strong>Edges ({{ selectedEdges.length }}):</strong>
          <ul>
            <li
              v-for="e in selectedEdges"
              :key="e.id"
              class="mv-node-edge"
            >
              <span class="mv-rel-pill">{{ e.rel_type }}</span>
              <span class="mv-edge-direction">
                {{ e.src_id === selectedEntry.id ? '→' : '←' }}
                #{{ e.src_id === selectedEntry.id ? e.dst_id : e.src_id }}
              </span>
              <button
                class="mv-edge-del"
                title="Delete edge"
                @click="handleDeleteEdge(e.id)"
              >
                ×
              </button>
            </li>
          </ul>
        </div>
        <div class="mv-node-btns">
          <button
            class="btn-secondary"
            @click="startEdit(selectedEntry)"
          >
            ✏ Edit
          </button>
          <button
            class="btn-danger"
            @click="confirmDelete(selectedEntry.id)"
          >
            🗑 Delete
          </button>
        </div>
      </aside>
    </div>

    <!-- ── List tab ── -->
    <div
      v-else-if="activeTab === 'List'"
      class="mv-list-panel"
    >
      <div class="mv-search-row">
        <input
          v-model="searchQuery"
          placeholder="Search memories…"
          class="mv-search"
          @keyup.enter="doSearch"
        >
        <button
          class="btn-secondary"
          @click="doSearch"
        >
          🔍 Search
        </button>
        <button
          class="btn-secondary"
          title="Brain-powered semantic search"
          @click="doSemanticSearch"
        >
          🤖 Semantic
        </button>
        <button
          class="btn-primary"
          title="6-signal hybrid search"
          @click="doHybridSearch"
        >
          ⚡ Hybrid
        </button>
      </div>

      <div class="mv-filter-row">
        <span class="mv-filter-label">Type:</span>
        <button
          v-for="t in allTypes"
          :key="t"
          :class="['mv-type-chip', { active: typeFilter === t }]"
          @click="typeFilter = typeFilter === t ? null : t"
        >
          {{ t }}
        </button>
        <span class="mv-filter-divider">|</span>
        <span class="mv-filter-label">Tier:</span>
        <button
          v-for="tier in allTiers"
          :key="tier"
          :class="['mv-tier-chip', 'tier-' + tier, { active: tierFilter === tier }]"
          @click="tierFilter = tierFilter === tier ? null : tier"
        >
          {{ tier }}
        </button>
      </div>

      <div
        v-if="tagPrefixCounts.size > 0"
        class="mv-filter-row"
        data-testid="mv-tag-prefix-filter"
      >
        <span class="mv-filter-label">Tag:</span>
        <button
          v-for="prefix in TAG_PREFIXES"
          :key="prefix"
          :class="['mv-tag-chip', { active: tagPrefixFilter === prefix }]"
          :disabled="!tagPrefixCounts.has(prefix)"
          @click="tagPrefixFilter = tagPrefixFilter === prefix ? null : prefix"
        >
          {{ prefix }} ({{ tagPrefixCounts.get(prefix) ?? 0 }})
        </button>
      </div>

      <p
        v-if="store.isLoading"
        class="mv-status"
      >
        Loading…
      </p>
      <p
        v-else-if="displayedMemories.length === 0"
        class="mv-status"
      >
        No memories yet.
      </p>

      <ul
        v-else
        class="mv-list"
      >
        <li
          v-for="m in displayedMemories"
          :key="m.id"
          :class="['mv-card', `type-${m.memory_type}`]"
        >
          <div class="mv-card-header">
            <span class="mv-chip">{{ m.memory_type }}</span>
            <span :class="'mv-tier-badge tier-' + m.tier">{{ m.tier }}</span>
            <span class="mv-stars">{{ '★'.repeat(m.importance) }}</span>
            <span
              class="mv-decay-bar"
              :title="'Decay: ' + (m.decay_score * 100).toFixed(0) + '%'"
            >
              <span
                class="mv-decay-fill"
                :style="{ width: (m.decay_score * 100) + '%' }"
              />
            </span>
          </div>
          <p class="mv-content">
            {{ m.content }}
          </p>
          <div
            v-if="m.tags"
            class="mv-tags"
          >
            <span
              v-for="tag in m.tags.split(',')"
              :key="tag"
              class="mv-tag"
            >{{ tag.trim() }}</span>
          </div>
          <div class="mv-card-footer">
            <span class="mv-ts">{{ formatDate(m.created_at) }}</span>
            <span
              v-if="m.token_count"
              class="mv-token-count"
              title="Token count"
            >{{ m.token_count }}t</span>
            <button
              v-if="m.tier !== 'long'"
              class="btn-icon"
              :title="'Promote to ' + promoteTier(m.tier)"
              @click="handlePromote(m.id, promoteTier(m.tier))"
            >
              ⬆
            </button>
            <button
              class="btn-icon"
              title="Edit"
              @click="startEdit(m)"
            >
              ✏
            </button>
            <button
              class="btn-icon danger"
              title="Delete"
              @click="confirmDelete(m.id)"
            >
              🗑
            </button>
          </div>
        </li>
      </ul>
    </div>

    <!-- ── Session tab ── -->
    <div
      v-else-if="activeTab === 'Session'"
      class="mv-session-panel"
    >
      <p class="mv-session-hint">
        Short-term memory — the last 20 messages of the current session that the brain reads
        before every reply.
      </p>
      <p
        v-if="shortTerm.length === 0"
        class="mv-status"
      >
        No conversation yet.
      </p>
      <ul
        v-else
        class="mv-session-list"
      >
        <li
          v-for="msg in shortTerm"
          :key="msg.id"
          :class="['mv-session-msg', msg.role]"
        >
          <strong>{{ msg.role === 'user' ? 'You' : '🤖' }}</strong>
          <span>{{ msg.content }}</span>
        </li>
      </ul>
    </div>

    <!-- ── Audit tab (chunk 33B.4) ── -->
    <div
      v-else-if="activeTab === 'Audit'"
      class="mv-audit-panel"
      data-testid="mv-audit-panel"
    >
      <p class="mv-audit-hint">
        Provenance view — pick a memory entry to see its full edit history and the typed edges
        that connect it to other memories. Edges are colour-coded by source: solid =
        user-curated, dashed = LLM-inferred, faded = auto-detected.
      </p>
      <div class="mv-audit-toolbar">
        <input
          v-model="auditSearch"
          class="mv-audit-search"
          type="search"
          placeholder="Filter memories…"
          data-testid="mv-audit-search"
        >
        <span class="mv-audit-count">{{ auditCandidates.length }} entries</span>
      </div>
      <div class="mv-audit-body">
        <ul
          class="mv-audit-list"
          data-testid="mv-audit-list"
        >
          <li
            v-if="auditCandidates.length === 0"
            class="mv-status"
          >
            No memories match.
          </li>
          <li
            v-for="entry in auditCandidates"
            :key="entry.id"
            :class="['mv-audit-list-item', { active: auditSelectedId === entry.id }]"
            data-testid="mv-audit-list-item"
            @click="selectAuditMemory(entry.id)"
          >
            <span class="mv-audit-list-title">{{ truncate(entry.content, 80) }}</span>
            <span class="mv-audit-list-meta">
              <span class="mv-audit-list-tier">{{ entry.tier }}</span>
              <span>·</span>
              <span>{{ formatDate(entry.created_at) }}</span>
            </span>
          </li>
        </ul>
        <section
          class="mv-audit-detail"
          data-testid="mv-audit-detail"
        >
          <p
            v-if="!auditSelected"
            class="mv-status"
          >
            Select a memory on the left to view its provenance.
          </p>
          <template v-else>
            <header class="mv-audit-header">
              <h3>Memory #{{ auditSelected.id }}</h3>
              <p class="mv-audit-current">
                {{ auditSelected.content }}
              </p>
              <ul class="mv-audit-meta">
                <li><strong>Type:</strong> {{ auditSelected.memory_type }}</li>
                <li><strong>Tier:</strong> {{ auditSelected.tier }}</li>
                <li><strong>Importance:</strong> {{ auditSelected.importance }}</li>
                <li><strong>Created:</strong> {{ formatDate(auditSelected.created_at) }}</li>
              </ul>
            </header>

            <section class="mv-audit-section">
              <h4>📜 Version history</h4>
              <p
                v-if="auditLoading"
                class="mv-status"
              >
                Loading…
              </p>
              <p
                v-else-if="auditHistory.length === 0"
                class="mv-status"
                data-testid="mv-audit-no-history"
              >
                No prior versions — this memory has not been edited.
              </p>
              <ol
                v-else
                class="mv-audit-timeline"
                data-testid="mv-audit-timeline"
              >
                <li
                  v-for="ver in auditHistoryReversed"
                  :key="ver.id"
                  class="mv-audit-version"
                >
                  <div class="mv-audit-version-head">
                    <span class="mv-audit-version-num">v{{ ver.version_num }}</span>
                    <span class="mv-audit-version-date">{{ formatDate(ver.created_at) }}</span>
                    <span class="mv-audit-version-type">{{ ver.memory_type }}</span>
                  </div>
                  <p class="mv-audit-version-content">{{ ver.content }}</p>
                  <p
                    v-if="ver.tags"
                    class="mv-audit-version-tags"
                  >
                    🏷 {{ ver.tags }}
                  </p>
                </li>
                <li class="mv-audit-version current">
                  <div class="mv-audit-version-head">
                    <span class="mv-audit-version-num">current</span>
                    <span class="mv-audit-version-date">{{ formatDate(auditSelected.created_at) }}</span>
                    <span class="mv-audit-version-type">{{ auditSelected.memory_type }}</span>
                  </div>
                  <p class="mv-audit-version-content">{{ auditSelected.content }}</p>
                  <p
                    v-if="auditSelected.tags"
                    class="mv-audit-version-tags"
                  >
                    🏷 {{ auditSelected.tags }}
                  </p>
                </li>
              </ol>
            </section>

            <section class="mv-audit-section">
              <h4>🕸 Edges ({{ auditEdges.length }})</h4>
              <p
                v-if="auditLoading"
                class="mv-status"
              >
                Loading…
              </p>
              <p
                v-else-if="auditEdges.length === 0"
                class="mv-status"
                data-testid="mv-audit-no-edges"
              >
                No edges connect this memory yet.
              </p>
              <ul
                v-else
                class="mv-audit-edges"
                data-testid="mv-audit-edges"
              >
                <li
                  v-for="edge in auditEdges"
                  :key="edge.edge.id"
                  :class="['mv-audit-edge', `source-${edge.edge.source}`]"
                >
                  <span class="mv-audit-edge-arrow">{{ edge.direction === 'outgoing' ? '→' : '←' }}</span>
                  <span class="mv-audit-edge-target">
                    #{{ edge.neighbor?.id ?? (edge.direction === 'outgoing' ? edge.edge.dst_id : edge.edge.src_id) }}
                  </span>
                  <span class="mv-audit-edge-rel">
                    <strong>{{ edge.edge.rel_type }}</strong>
                    <small v-if="edge.neighbor">{{ truncate(edge.neighbor.content, 96) }}</small>
                  </span>
                  <span class="mv-audit-edge-source">{{ edge.edge.source }}</span>
                  <span class="mv-audit-edge-conf">{{ (edge.edge.confidence * 100).toFixed(0) }}%</span>
                </li>
              </ul>
            </section>
          </template>
        </section>
      </div>
    </div>

    <!-- Add / Edit modal -->
    <div
      v-if="showAdd || editTarget"
      class="mv-modal-backdrop"
      @click.self="closeModal"
    >
      <div class="mv-modal">
        <h3>{{ editTarget ? 'Edit memory' : 'Add memory' }}</h3>
        <label>Content
          <textarea
            v-model="form.content"
            rows="3"
            placeholder="What should I remember?"
          />
        </label>
        <label>Tags (comma-separated)
          <input
            v-model="form.tags"
            placeholder="python, work, project"
          >
        </label>
        <label>Type
          <select v-model="form.memory_type">
            <option
              v-for="t in allTypes"
              :key="t"
              :value="t"
            >{{ t }}</option>
          </select>
        </label>
        <label>Importance (1–5)
          <input
            v-model.number="form.importance"
            type="range"
            min="1"
            max="5"
          >
          <span>{{ form.importance }}</span>
        </label>
        <div class="mv-modal-btns">
          <button
            class="btn-primary"
            @click="saveMemory"
          >
            Save
          </button>
          <button
            class="btn-secondary"
            @click="closeModal"
          >
            Cancel
          </button>
        </div>
      </div>
    </div>

    <!-- Obsidian export modal -->
    <div
      v-if="showObsidianExport"
      class="mv-modal-backdrop"
      @click.self="showObsidianExport = false"
    >
      <div
        class="mv-modal"
        data-testid="mv-obsidian-dialog"
      >
        <h3>📓 Export to Obsidian</h3>
        <p class="mv-desc">
          Export all long-tier memories as Markdown files with YAML frontmatter into your Obsidian vault.
        </p>
        <label>Vault directory
          <input
            v-model="obsidianVaultDir"
            placeholder="e.g. C:\Users\Me\Documents\MyVault"
            data-testid="mv-obsidian-path"
          >
        </label>
        <p
          v-if="obsidianResult"
          class="mv-feedback"
          data-testid="mv-obsidian-result"
        >
          {{ obsidianResult }}
        </p>
        <div class="mv-modal-btns">
          <button
            class="btn-primary"
            :disabled="!obsidianVaultDir.trim() || isActing"
            data-testid="mv-obsidian-run"
            @click="handleObsidianExport"
          >
            {{ isActing ? 'Exporting…' : 'Export' }}
          </button>
          <button
            class="btn-secondary"
            @click="showObsidianExport = false"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useMemoryStore } from '../stores/memory';
import { useSettingsStore } from '../stores/settings';
import MemoryGraph from '../components/MemoryGraph.vue';
import BrainGraphViewport from '../components/BrainGraphViewport.vue';
import type { MemoryEntry, MemoryType, MemoryTier, MemoryProvenance } from '../types';

const store = useMemoryStore();
const settingsStore = useSettingsStore();

type MvTab = 'Graph' | 'List' | 'Session' | 'Audit';
const activeTab = ref<MvTab>('List');
const tabs: MvTab[] = ['List', 'Graph', 'Session', 'Audit'];
const allTypes: MemoryType[] = ['fact', 'preference', 'context', 'summary'];
const allTiers: MemoryTier[] = ['short', 'working', 'long'];

// ── Audit tab (chunk 33B.4) ──────────────────────────────────────────────
const auditSearch = ref('');
const auditSelectedId = ref<number | null>(null);
const auditProvenance = ref<MemoryProvenance | null>(null);
const auditLoading = ref(false);

const auditCandidates = computed(() => {
  const q = auditSearch.value.trim().toLowerCase();
  const list = q.length === 0
    ? store.memories
    : store.memories.filter((m) =>
        m.content.toLowerCase().includes(q) ||
        (m.tags ?? '').toLowerCase().includes(q),
      );
  // Most-recently-created first to surface fresh evidence quickly.
  return [...list].sort((a, b) => b.created_at - a.created_at).slice(0, 200);
});

const auditSelected = computed<MemoryEntry | null>(() => {
  const id = auditSelectedId.value;
  if (id == null) return null;
  return store.memories.find((m) => m.id === id) ?? null;
});

/** Versions oldest → newest (Rust returns oldest-first; render in that order). */
const auditHistory = computed(() => auditProvenance.value?.versions ?? []);
const auditHistoryReversed = computed(() => auditHistory.value);
const auditEdges = computed(() => auditProvenance.value?.edges ?? []);

async function selectAuditMemory(id: number) {
  auditSelectedId.value = id;
  auditLoading.value = true;
  auditProvenance.value = null;
  try {
    auditProvenance.value = await store.getMemoryProvenance(id);
  } catch (e) {
    console.error('[audit] failed to load provenance:', e);
  } finally {
    auditLoading.value = false;
  }
}

const maxMemoryGb = ref(10);
const maxMemoryMb = ref(10);
const memoryStorageBytes = computed(() => store.stats?.storage_bytes ?? 0);
// Use backend-provided stats to avoid O(n) client-side reductions on large memory sets.
const memoryCacheBytes = computed(() => store.stats?.storage_bytes ?? 0);

// Search & filter
const searchQuery = ref('');
const typeFilter = ref<MemoryType | null>(null);
const tierFilter = ref<MemoryTier | null>(null);
const tagPrefixFilter = ref<string | null>(null);
const searchResults = ref<MemoryEntry[] | null>(null);

/** Curated tag prefixes — must match Rust `CURATED_PREFIXES`. */
const TAG_PREFIXES = ['personal', 'domain', 'project', 'tool', 'code', 'external', 'session', 'quest'] as const;
const TAG_PREFIX_SET = new Set<string>(TAG_PREFIXES as readonly string[]);

/** Count memories per curated tag prefix. */
const tagPrefixCounts = computed(() => {
  const source = searchResults.value ?? store.memories;
  const counts = new Map<string, number>();
  for (const m of source) {
    if (!m.tags) continue;
    const seen = new Set<string>();
    for (const tag of m.tags.split(',')) {
      const trimmed = tag.trim();
      const colonIdx = trimmed.indexOf(':');
      if (colonIdx <= 0) continue;
      const prefix = trimmed.slice(0, colonIdx).toLowerCase();
      if (!seen.has(prefix) && TAG_PREFIX_SET.has(prefix)) {
        seen.add(prefix);
        counts.set(prefix, (counts.get(prefix) ?? 0) + 1);
      }
    }
  }
  return counts;
});

const displayedMemories = computed(() => {
  const source = searchResults.value ?? store.memories;
  return source.filter((m) => {
    if (typeFilter.value && m.memory_type !== typeFilter.value) return false;
    if (tierFilter.value && m.tier !== tierFilter.value) return false;
    if (tagPrefixFilter.value) {
      if (!m.tags) return false;
      const prefix = tagPrefixFilter.value;
      const hasPrefix = m.tags.split(',').some((t) => {
        const trimmed = t.trim().toLowerCase();
        return trimmed.startsWith(prefix + ':');
      });
      if (!hasPrefix) return false;
    }
    return true;
  });
});

async function doSearch() {
  if (!searchQuery.value.trim()) {
    searchResults.value = null;
    return;
  }
  searchResults.value = await store.search(searchQuery.value);
}

async function doSemanticSearch() {
  if (!searchQuery.value.trim()) return;
  searchResults.value = await store.semanticSearch(searchQuery.value);
}

async function doHybridSearch() {
  if (!searchQuery.value.trim()) return;
  searchResults.value = await store.hybridSearch(searchQuery.value);
}

// Session tab — uses Message type (chat messages), not MemoryEntry
const shortTerm = ref<import('../types').Message[]>([]);
async function loadShortTerm() {
  shortTerm.value = await store.getShortTermMemory(20);
}

// Graph node selection
const selectedEntry = ref<MemoryEntry | null>(null);
const edgeMode = ref<'typed' | 'tag' | 'both'>('typed');
const graph3d = ref(false);

const selectedEdges = computed(() => {
  if (!selectedEntry.value) return [];
  const id = selectedEntry.value.id;
  return store.edges.filter((e) => e.src_id === id || e.dst_id === id);
});

function onNodeSelect(id: number) {
  selectedEntry.value = store.memories.find((m) => m.id === id) ?? null;
}

async function handleExtractEdges() {
  isActing.value = true;
  feedback.value = '';
  store.error = null;
  const count = await store.extractEdgesViaBrain();
  if (count > 0) {
    feedback.value = `🔗 ${count} new edge${count === 1 ? '' : 's'} extracted by the brain.`;
  } else if (store.error) {
    feedback.value = `❌ Edge extraction failed: ${store.error}`;
  } else {
    feedback.value = '🤔 The brain found no new edges to propose.';
  }
  await store.getEdgeStats();
  isActing.value = false;
  setTimeout(() => (feedback.value = ''), 4000);
}

async function handleDeleteEdge(edgeId: number) {
  await store.deleteEdge(edgeId);
  await store.getEdgeStats();
}

// Add / Edit modal
const showAdd = ref(false);
const editTarget = ref<MemoryEntry | null>(null);
const form = ref({ content: '', tags: '', importance: 3, memory_type: 'fact' as MemoryType });

function startEdit(m: MemoryEntry) {
  editTarget.value = m;
  form.value = { content: m.content, tags: m.tags, importance: m.importance, memory_type: m.memory_type };
}

function closeModal() {
  showAdd.value = false;
  editTarget.value = null;
  form.value = { content: '', tags: '', importance: 3, memory_type: 'fact' };
}

function promoteTier(current: MemoryTier): MemoryTier {
  return current === 'short' ? 'working' : 'long';
}

async function handlePromote(id: number, tier: MemoryTier) {
  await store.promoteMemory(id, tier);
  await store.getStats();
}

async function saveMemory() {
  if (!form.value.content.trim()) return;
  if (editTarget.value) {
    await store.updateMemory(editTarget.value.id, { ...form.value });
  } else {
    await store.addMemory({ ...form.value });
  }
  closeModal();
}

async function confirmDelete(id: number) {
  if (confirm('Delete this memory?')) {
    await store.deleteMemory(id);
    selectedEntry.value = null;
  }
}

// Brain actions
const isActing = ref(false);
const feedback = ref('');

async function handleExtract() {
  isActing.value = true;
  feedback.value = '';
  const count = await store.extractFromSession();
  feedback.value = count > 0 ? `✅ ${count} new memories extracted.` : '🤔 Nothing new to remember.';
  isActing.value = false;
  setTimeout(() => (feedback.value = ''), 4000);
}

async function handleSummarize() {
  isActing.value = true;
  feedback.value = '';
  const summary = await store.summarizeSession();
  feedback.value = summary ? `✅ Session summarized and saved.` : '❌ Could not summarize. Is the brain active?';
  isActing.value = false;
  setTimeout(() => (feedback.value = ''), 4000);
}

async function handleDecay() {
  isActing.value = true;
  feedback.value = '';
  const count = await store.applyDecay();
  feedback.value = count > 0 ? `⏳ Decay applied to ${count} memories.` : '✅ All memories already decayed.';
  isActing.value = false;
  await store.getStats();
  setTimeout(() => (feedback.value = ''), 4000);
}

async function handleGC() {
  isActing.value = true;
  feedback.value = '';
  const count = await store.gcMemories();
  feedback.value = count > 0 ? `🧹 ${count} decayed memories removed.` : '✅ Nothing to clean up.';
  isActing.value = false;
  await store.getStats();
  setTimeout(() => (feedback.value = ''), 4000);
}

function formatDate(ts: number) {
  return new Date(ts).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
}

function truncate(text: string, max: number): string {
  if (!text) return '';
  return text.length <= max ? text : text.slice(0, max - 1) + '…';
}

function formatTokens(n: number) {
  return n >= 1000 ? (n / 1000).toFixed(1) + 'k' : String(n);
}

function formatBytes(bytes: number) {
  if (bytes >= 1024 ** 3) return (bytes / 1024 ** 3).toFixed(2) + ' GB';
  if (bytes >= 1024 ** 2) return (bytes / 1024 ** 2).toFixed(1) + ' MB';
  if (bytes >= 1024) return (bytes / 1024).toFixed(1) + ' KB';
  return bytes + ' B';
}

async function saveMemoryCap() {
  const gb = Math.min(100, Math.max(1, Number(maxMemoryGb.value) || 10));
  maxMemoryGb.value = gb;
  await settingsStore.saveMaxMemoryGb(gb);
  const report = await store.enforceStorageLimit();
  if (report && report.deleted > 0) {
    feedback.value = `🧹 Memory cap saved. Removed ${report.deleted} older low-utility memories.`;
    setTimeout(() => (feedback.value = ''), 4000);
  }
}

async function saveMemoryCacheCap() {
  const mb = Math.min(1024, Math.max(1, Number(maxMemoryMb.value) || 10));
  maxMemoryMb.value = mb;
  await settingsStore.saveMaxMemoryMb(mb);
  await store.fetchAll();
}

// Obsidian export
const showObsidianExport = ref(false);
const obsidianVaultDir = ref('');
const obsidianResult = ref('');

async function handleObsidianExport() {
  isActing.value = true;
  obsidianResult.value = '';
  try {
    const report = await store.exportToObsidian(obsidianVaultDir.value.trim());
    obsidianResult.value = `✅ Exported ${report.written} file${report.written === 1 ? '' : 's'}, skipped ${report.skipped} unchanged (${report.total} long-tier total).`;
  } catch (e) {
    obsidianResult.value = `❌ Export failed: ${String(e)}`;
  }
  isActing.value = false;
}

onMounted(async () => {
  await settingsStore.loadSettings();
  maxMemoryGb.value = settingsStore.settings.max_memory_gb ?? 10;
  maxMemoryMb.value = settingsStore.settings.max_memory_mb ?? 10;
  await store.fetchAll();
  await Promise.all([loadShortTerm(), store.getStats(), store.fetchEdges(), store.getEdgeStats()]);
});
</script>

<style scoped src="./MemoryView.css"></style>
