<template>
  <div
    class="bp-shell memory-view"
    data-density="cozy"
  >
    <!-- ── Breadcrumb ──────────────────────────────────────────────────────── -->
    <AppBreadcrumb
      here="KNOWLEDGE GRAPHS"
      @navigate="emit('navigate', $event)"
    />

    <header class="mv-header">
      <h2>🧠 Memory</h2>
      <div class="mv-header-actions">
        <button
          class="bp-btn bp-btn--ghost bp-btn--sm"
          :disabled="isActing"
          @click="handleExtract"
        >
          {{ isActing ? 'Working…' : '⬇ Extract from session' }}
        </button>
        <button
          class="bp-btn bp-btn--ghost bp-btn--sm"
          :disabled="isActing"
          @click="handleSummarize"
        >
          📄 Summarize session
        </button>
        <button
          class="bp-btn bp-btn--ghost bp-btn--sm"
          :disabled="isActing"
          title="Apply time-decay to all memories"
          @click="handleDecay"
        >
          ⏳ Decay
        </button>
        <button
          class="bp-btn bp-btn--ghost bp-btn--sm"
          :disabled="isActing"
          title="Remove fully decayed memories"
          @click="handleGC"
        >
          🧹 GC
        </button>
        <button
          class="bp-btn bp-btn--primary bp-btn--sm"
          @click="showAdd = true"
        >
          ＋ Add memory
        </button>
        <button
          class="bp-btn bp-btn--ghost bp-btn--sm"
          data-testid="mv-obsidian-export"
          :disabled="isActing"
          @click="showObsidianExport = true"
        >
          📓 Export to Obsidian
        </button>
      </div>
    </header>

    <!-- ── Memory source picker (BRAIN-REPO-RAG-1a) ────────────────────────── -->
    <nav
      class="mv-source-picker"
      data-testid="mv-source-picker"
      aria-label="Knowledge source"
    >
      <button
        type="button"
        class="mv-source-pill"
        :class="{ 'is-active': sourcesStore.activeId === SELF_SOURCE_ID }"
        data-testid="mv-source-self"
        :title="'Built-in TerranSoul brain'"
        @click="sourcesStore.setActive(SELF_SOURCE_ID)"
      >
        🧠 TerranSoul
      </button>
      <button
        v-for="repo in sourcesStore.repoSources"
        :key="repo.id"
        type="button"
        class="mv-source-pill"
        :class="{ 'is-active': sourcesStore.activeId === repo.id }"
        :data-testid="`mv-source-${repo.id}`"
        :title="repo.repo_url ?? repo.label"
        @click="sourcesStore.setActive(repo.id)"
      >
        📦 {{ repo.label }}
      </button>
      <button
        type="button"
        class="mv-source-pill"
        :class="{ 'is-active': sourcesStore.isAllView }"
        data-testid="mv-source-all"
        title="Search across every source (BRAIN-REPO-RAG-1c)"
        @click="sourcesStore.setActive(ALL_SOURCES_ID)"
      >
        🌐 All sources
      </button>
      <button
        type="button"
        class="mv-source-pill mv-source-pill--add"
        data-testid="mv-source-add"
        title="Register a new repository or topic source"
        @click="showAddSource = true"
      >
        ＋ Add source
      </button>
      <button
        type="button"
        class="mv-source-pill mv-source-pill--add"
        data-testid="mv-source-oauth"
        title="Connect GitHub for private repositories"
        @click="showRepoOAuth = true"
      >
        🔐 GitHub auth
      </button>
      <span
        v-if="!sourcesStore.isAllView && sourcesStore.activeSource && sourcesStore.activeSource.kind === 'repo'"
        class="mv-source-hint"
        data-testid="mv-source-empty-hint"
      >
        Repo ingest lands in BRAIN-REPO-RAG-1b — this source is registered but not yet indexed.
      </span>
    </nav>

    <p
      v-if="feedback"
      class="mv-feedback"
    >
      {{ feedback }}
    </p>

    <!-- Stats dashboard -->
    <section class="bp-module">
      <header class="bp-module-head">
        <div class="bp-module-head-left">
          <div class="bp-module-eyebrow">
            <span class="ix">01</span> Memory Health
          </div>
        </div>
      </header>
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
          <span class="mv-stat-value">{{ (store.stats.avg_decay ?? 0).toFixed(2) }}</span>
          <span class="mv-stat-label">Avg Decay</span>
        </div>
      </div>
    </section>

    <section class="bp-module">
      <header class="bp-module-head">
        <div class="bp-module-head-left">
          <div class="bp-module-eyebrow">
            <span class="ix">02</span> Storage
          </div>
          <h2 class="bp-module-title">
            Cache & persistence
          </h2>
        </div>
      </header>
      <section class="mv-rag-config">
        <div class="mv-storage-summary">
          <strong>Memory configuration</strong>
          <span>Brain memory &amp; RAG in memory: {{ formatBytes(memoryCacheBytes) }} / {{ formatBytes(maxMemoryMb * 1024 * 1024) }}</span>
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
          <span>Brain memory &amp; RAG in storage: {{ formatBytes(memoryStorageBytes) }} / {{ formatBytes(maxMemoryGb * 1024 * 1024 * 1024) }}</span>
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
    </section>

    <!-- CLAIM-VERIFY-3 — Contradictions / claim verification panel -->
    <section
      v-if="store.openConflictCount > 0 || conflictList.length > 0"
      class="bp-module mv-conflicts"
    >
      <header class="bp-module-head">
        <div class="bp-module-head-left">
          <div class="bp-module-eyebrow">
            <span class="ix">⚖️</span> Contradictions
            <span
              v-if="store.openConflictCount > 0"
              class="mv-conflict-count"
            >{{ store.openConflictCount }} open</span>
          </div>
          <p class="bp-module-sub">
            New memories that closely resemble an existing one are
            checked for contradictions. Pick a winner — the loser is
            soft-closed (preserved for audit), never deleted.
          </p>
        </div>
        <div class="bp-module-head-right">
          <button
            type="button"
            class="mv-btn"
            :disabled="conflictScanBusy"
            @click="onScanRecentConflicts"
          >
            {{ conflictScanBusy ? 'Scanning…' : 'Scan recent (50)' }}
          </button>
          <button
            type="button"
            class="mv-btn"
            :disabled="conflictListBusy"
            @click="onRefreshConflicts"
          >
            Refresh
          </button>
        </div>
      </header>
      <p
        v-if="conflictFeedback"
        class="mv-feedback"
      >
        {{ conflictFeedback }}
      </p>
      <ul
        v-if="conflictList.length > 0"
        class="mv-conflict-list"
      >
        <li
          v-for="conflict in conflictList"
          :key="conflict.id"
          class="mv-conflict-item"
        >
          <div class="mv-conflict-reason">
            {{ conflict.reason || 'Contradiction detected by brain.' }}
          </div>
          <div class="mv-conflict-pair">
            <article class="mv-conflict-side">
              <header>Memory A · #{{ conflict.entry_a_id }}</header>
              <p>{{ conflictContent(conflict.entry_a_id) }}</p>
              <button
                type="button"
                class="mv-btn mv-btn-primary"
                :disabled="conflictResolveBusy === conflict.id"
                @click="onPickWinner(conflict, conflict.entry_a_id)"
              >
                Keep A
              </button>
            </article>
            <article class="mv-conflict-side">
              <header>Memory B · #{{ conflict.entry_b_id }}</header>
              <p>{{ conflictContent(conflict.entry_b_id) }}</p>
              <button
                type="button"
                class="mv-btn mv-btn-primary"
                :disabled="conflictResolveBusy === conflict.id"
                @click="onPickWinner(conflict, conflict.entry_b_id)"
              >
                Keep B
              </button>
            </article>
          </div>
          <footer class="mv-conflict-actions">
            <button
              type="button"
              class="mv-btn"
              :disabled="conflictResolveBusy === conflict.id"
              @click="onDismissConflict(conflict)"
            >
              Dismiss (not a real conflict)
            </button>
          </footer>
        </li>
      </ul>
      <p
        v-else-if="!conflictListBusy"
        class="mv-empty"
      >
        No open contradictions. New conflicts surface here automatically
        when `auto_detect_conflicts` is enabled in Settings.
      </p>
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
        <MemoryGraph
          :memories="graphMemories"
          :edges="store.edges"
          :edge-mode="edgeMode"
          @select="onNodeSelect"
          @keep-only-selection="handleKeepOnly"
        />
        <div
          v-if="store.isLoading"
          class="mv-graph-loading"
        >
          Loading {{ store.memories.length }} memories…
        </div>
      </div>
      <div
        v-if="selectedEntry"
        class="mv-graph-crud-shell"
      >
        <GraphNodeCrudPanel
          class="mv-graph-crud"
          :entry="selectedEntry"
          :edges="selectedEdges"
          :all-memories="store.memories"
          @close="selectedEntry = null"
          @navigate="onNodeSelect"
          @changed="onGraphChanged"
        />
      </div>
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
      class="mv-session-panel ts-cockpit-card ts-cockpit-card--compact"
    >
      <span
        class="ts-cockpit-label mv-panel-kicker"
        aria-hidden="true"
      >01 / Session Stream</span>
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
      class="mv-audit-panel ts-cockpit-card ts-cockpit-card--compact"
      data-testid="mv-audit-panel"
    >
      <span
        class="ts-cockpit-label mv-panel-kicker"
        aria-hidden="true"
      >02 / Provenance</span>
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
                  v-for="ver in auditHistorySorted"
                  :key="ver.id"
                  class="mv-audit-version"
                >
                  <div class="mv-audit-version-head">
                    <span class="mv-audit-version-num">v{{ ver.version_num }}</span>
                    <span class="mv-audit-version-date">{{ formatDate(ver.created_at) }}</span>
                    <span class="mv-audit-version-type">{{ ver.memory_type }}</span>
                  </div>
                  <p class="mv-audit-version-content">
                    {{ ver.content }}
                  </p>
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
                  <p class="mv-audit-version-content">
                    {{ auditSelected.content }}
                  </p>
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
                v-else-if="!auditProvenance"
                class="mv-status"
                data-testid="mv-audit-no-provenance"
              >
                Edge data is not available yet.
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

    <!-- ── Add memory-source dialog (BRAIN-REPO-RAG-1a) ──────────────────── -->
    <div
      v-if="showAddSource"
      class="mv-modal-backdrop"
      data-testid="mv-add-source-dialog"
      @click.self="showAddSource = false"
    >
      <div
        class="mv-modal"
        role="dialog"
        aria-labelledby="mv-add-source-title"
      >
        <h3 id="mv-add-source-title">
          ＋ Add memory source
        </h3>
        <p class="mv-desc">
          Register a repository or topic source. At this milestone we
          only persist the source metadata — automated clone + indexing
          ships in BRAIN-REPO-RAG-1b.
        </p>
        <label>Label
          <input
            v-model="newSourceLabel"
            placeholder="e.g. terransoul-docs"
            data-testid="mv-add-source-label"
            :disabled="newSourceSaving"
          >
        </label>
        <label>Repository URL <span class="mv-desc-inline">(optional)</span>
          <input
            v-model="newSourceRepoUrl"
            placeholder="https://github.com/owner/repo"
            data-testid="mv-add-source-url"
            :disabled="newSourceSaving"
          >
        </label>
        <label>Git ref <span class="mv-desc-inline">(branch / tag)</span>
          <input
            v-model="newSourceRepoRef"
            placeholder="main"
            data-testid="mv-add-source-ref"
            :disabled="newSourceSaving"
          >
        </label>
        <p
          v-if="newSourceError"
          class="mv-feedback mv-feedback--error"
          data-testid="mv-add-source-error"
        >
          {{ newSourceError }}
        </p>
        <div class="mv-modal-actions">
          <button
            class="bp-btn bp-btn--primary bp-btn--sm"
            data-testid="mv-add-source-save"
            :disabled="newSourceSaving || !newSourceLabel.trim()"
            @click="handleAddSource"
          >
            {{ newSourceSaving ? 'Saving…' : 'Save source' }}
          </button>
          <button
            class="bp-btn bp-btn--ghost bp-btn--sm"
            :disabled="newSourceSaving"
            @click="showAddSource = false; resetAddSourceForm()"
          >
            Cancel
          </button>
        </div>
      </div>
    </div>

    <!-- BRAIN-REPO-RAG-1e: GitHub OAuth device-flow dialog -->
    <RepoOAuthDialog
      :open="showRepoOAuth"
      @close="showRepoOAuth = false"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watchEffect } from 'vue';
import { useMemoryStore } from '../stores/memory';
import type { MemoryConflict } from '../stores/memory';
import { useSettingsStore } from '../stores/settings';
import {
  useMemorySourcesStore,
  SELF_SOURCE_ID,
  ALL_SOURCES_ID,
} from '../stores/memory-sources';
import MemoryGraph from '../components/MemoryGraph.vue';
import GraphNodeCrudPanel from '../components/GraphNodeCrudPanel.vue';
import AppBreadcrumb from '../components/ui/AppBreadcrumb.vue';
import RepoOAuthDialog from '../components/RepoOAuthDialog.vue';
import type { MemoryEntry, MemoryType, MemoryTier, MemoryProvenance } from '../types';

const emit = defineEmits<{
  navigate: [target: string];
}>();

const store = useMemoryStore();
const settingsStore = useSettingsStore();
const sourcesStore = useMemorySourcesStore();

// ── Add-source dialog state (BRAIN-REPO-RAG-1a) ──────────────────────────
const showAddSource = ref(false);
const showRepoOAuth = ref(false);
const newSourceLabel = ref('');
const newSourceRepoUrl = ref('');
const newSourceRepoRef = ref('main');
const newSourceError = ref<string | null>(null);
const newSourceSaving = ref(false);

function slugifySourceId(url: string, label: string): string {
  // Prefer the repo URL host+path; fall back to label for non-URL sources.
  const trimmed = url.trim();
  if (trimmed) {
    try {
      const parsed = new URL(trimmed);
      const path = parsed.pathname.replace(/\.git$/, '').replace(/^\/+|\/+$/g, '');
      if (parsed.host && path) return `repo:${parsed.host}/${path}`;
    } catch {
      /* not a URL — fall through to label slug */
    }
  }
  const slug = label.trim().toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '');
  return slug ? `repo:${slug}` : `repo:source-${Date.now()}`;
}

function resetAddSourceForm(): void {
  newSourceLabel.value = '';
  newSourceRepoUrl.value = '';
  newSourceRepoRef.value = 'main';
  newSourceError.value = null;
}

async function handleAddSource(): Promise<void> {
  newSourceError.value = null;
  const label = newSourceLabel.value.trim();
  if (!label) {
    newSourceError.value = 'Label is required.';
    return;
  }
  newSourceSaving.value = true;
  const created = await sourcesStore.createSource({
    id: slugifySourceId(newSourceRepoUrl.value, label),
    kind: 'repo',
    label,
    repo_url: newSourceRepoUrl.value.trim() || null,
    repo_ref: newSourceRepoRef.value.trim() || null,
  });
  newSourceSaving.value = false;
  if (created) {
    showAddSource.value = false;
    resetAddSourceForm();
  } else {
    newSourceError.value = sourcesStore.error ?? 'Failed to create source.';
  }
}

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
const auditHistorySorted = computed(() => auditHistory.value);
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
const maxMemoryMb = ref(256);
const memoryStorageBytes = computed(() => store.stats?.storage_bytes ?? 0);
// Use backend-provided stats to avoid O(n) client-side reductions on large memory sets.
const memoryCacheBytes = computed(() => store.stats?.cache_bytes ?? 0);

// Search & filter
const searchQuery = ref('');
const typeFilter = ref<MemoryType | null>(null);
const tierFilter = ref<MemoryTier | null>(null);
const tagPrefixFilter = ref<string | null>(null);
const searchResults = ref<MemoryEntry[] | null>(null);

/** Curated tag prefixes — must match Rust `CURATED_PREFIXES`. */
const TAG_PREFIXES = ['personal', 'domain', 'project', 'tool', 'code', 'external', 'session', 'quest'] as const;
const TAG_PREFIX_SET = new Set<string>(TAG_PREFIXES as readonly string[]);

function normalizePrefixList(prefixes: readonly string[]): string[] {
  return [...prefixes].map((p) => p.trim().toLowerCase()).sort();
}

// Runtime guard for cross-stack drift: frontend TAG_PREFIXES vs backend CURATED_PREFIXES.
// This check is best-effort and only runs when backend exposes curated_tag_prefixes in stats.
watchEffect(() => {
  const backendPrefixes = (store.stats as { curated_tag_prefixes?: string[] } | undefined)?.curated_tag_prefixes;
  if (!Array.isArray(backendPrefixes) || backendPrefixes.length === 0) return;

  const frontend = normalizePrefixList(TAG_PREFIXES as readonly string[]);
  const backend = normalizePrefixList(backendPrefixes);
  if (frontend.length !== backend.length || frontend.some((p, i) => p !== backend[i])) {
    console.warn(
      '[memory] Curated tag prefix mismatch between frontend TAG_PREFIXES and backend CURATED_PREFIXES.',
      { frontend, backend },
    );
  }
});

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

// ── BRAIN-REPO-RAG-2a: cross-source knowledge-graph projection ──────────
// When the "All" pseudo-source is active, fan-out into every connected repo
// source and merge their recent chunks into the graph as additional nodes.
const crossSourceNodes = ref<MemoryEntry[]>([]);

function projectRepoNodeToMemoryEntry(
  node: import('../types').CrossSourceGraphNode,
): MemoryEntry {
  return {
    id: node.graphId,
    content: node.content,
    tags: '',
    importance: 1,
    memory_type: 'fact',
    created_at: node.createdAt,
    last_accessed: null,
    access_count: 0,
    tier: 'long' as MemoryTier,
    decay_score: 0,
    session_id: null,
    parent_id: null,
    token_count: 0,
    confidence: 1,
    source_id: node.sourceId,
    source_label: node.sourceLabel,
    file_path: node.filePath ?? undefined,
    parent_symbol: node.parentSymbol ?? undefined,
  };
}

watchEffect(async () => {
  if (sourcesStore.isAllView) {
    try {
      const graph = await store.fetchCrossSourceGraph();
      crossSourceNodes.value = graph.nodes.map(projectRepoNodeToMemoryEntry);
    } catch {
      crossSourceNodes.value = [];
    }
  } else {
    crossSourceNodes.value = [];
  }
});

/** Memories passed to the graph view: personal + cross-source repo chunks. */
const graphMemories = computed<MemoryEntry[]>(() => {
  if (!sourcesStore.isAllView || crossSourceNodes.value.length === 0) {
    return displayedMemories.value;
  }
  return [...displayedMemories.value, ...crossSourceNodes.value];
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

const selectedEdges = computed(() => {
  if (!selectedEntry.value) return [];
  const id = selectedEntry.value.id;
  return store.edges.filter((e) => e.src_id === id || e.dst_id === id);
});

function onNodeSelect(id: number) {
  selectedEntry.value = store.memories.find((m) => m.id === id) ?? null;
}

/** Called by GraphNodeCrudPanel after any node/edge mutation. */
async function onGraphChanged() {
  await store.fetchAll();
  await store.fetchEdges();
  await store.getEdgeStats();
  if (selectedEntry.value) {
    selectedEntry.value =
      store.memories.find((m) => m.id === selectedEntry.value!.id) ?? null;
  }
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

/**
 * Destructive "Keep only" action from `SelectedNodesPanel`. Deletes every
 * memory that is NOT in the supplied selection. Guarded by a native
 * `confirm()` because there is no undo.
 */
async function handleKeepOnly(keepIds: number[]) {
  const keepSet = new Set(keepIds);
  const toDelete = store.memories.filter((m) => !keepSet.has(m.id));
  if (toDelete.length === 0) {
    alert('All memories are in your selection — nothing to delete.');
    return;
  }
  const msg =
    `Delete ${toDelete.length} memor${toDelete.length === 1 ? 'y' : 'ies'} ` +
    `NOT in your selection of ${keepIds.length}? This cannot be undone.`;
  if (!confirm(msg)) return;
  for (const m of toDelete) {
    try {
      await store.deleteMemory(m.id);
    } catch (err) {
      console.error('[MemoryView] keep-only delete failed for', m.id, err);
    }
  }
  // Clear inspector if the previously selected entry was just deleted.
  if (selectedEntry.value && !keepSet.has(selectedEntry.value.id)) {
    selectedEntry.value = null;
  }
}

// Brain actions
const isActing = ref(false);
const feedback = ref('');

// CLAIM-VERIFY-3 — contradictions panel state.
const conflictList = ref<MemoryConflict[]>([]);
const conflictListBusy = ref(false);
const conflictScanBusy = ref(false);
const conflictResolveBusy = ref<number | null>(null);
const conflictFeedback = ref('');

function conflictContent(memoryId: number): string {
  const m = store.memories.find((row) => row.id === memoryId);
  return m?.content ?? '(memory not loaded — try Refresh)';
}

async function onRefreshConflicts(): Promise<void> {
  conflictListBusy.value = true;
  try {
    conflictList.value = await store.listConflicts('open');
    await store.refreshOpenConflictCount();
  } catch (e) {
    conflictFeedback.value = `Failed to load conflicts: ${String(e)}`;
  } finally {
    conflictListBusy.value = false;
  }
}

async function onScanRecentConflicts(): Promise<void> {
  conflictScanBusy.value = true;
  conflictFeedback.value = '';
  try {
    const opened = await store.scanRecentForConflicts(50);
    conflictFeedback.value = opened > 0
      ? `Scan complete — ${opened} new conflict(s) opened.`
      : 'Scan complete — no new contradictions found.';
    await onRefreshConflicts();
  } catch (e) {
    conflictFeedback.value = `Scan failed: ${String(e)}`;
  } finally {
    conflictScanBusy.value = false;
  }
}

async function onPickWinner(conflict: MemoryConflict, winnerId: number): Promise<void> {
  conflictResolveBusy.value = conflict.id;
  try {
    await store.resolveConflict(conflict.id, winnerId);
    conflictFeedback.value = `Resolved conflict #${conflict.id} — kept memory #${winnerId}.`;
    await onRefreshConflicts();
  } catch (e) {
    conflictFeedback.value = `Resolve failed: ${String(e)}`;
  } finally {
    conflictResolveBusy.value = null;
  }
}

async function onDismissConflict(conflict: MemoryConflict): Promise<void> {
  conflictResolveBusy.value = conflict.id;
  try {
    await store.dismissConflict(conflict.id);
    conflictFeedback.value = `Dismissed conflict #${conflict.id}.`;
    await onRefreshConflicts();
  } catch (e) {
    conflictFeedback.value = `Dismiss failed: ${String(e)}`;
  } finally {
    conflictResolveBusy.value = null;
  }
}

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
  // Fire all fetches in parallel so the graph renders as soon as memories
  // arrive, without waiting for edges/stats to finish first.
  await Promise.all([
    store.fetchAll(),
    store.fetchEdges(),
    loadShortTerm(),
    store.getStats(),
    store.getEdgeStats(),
    sourcesStore.fetchAll(),
  ]);
  // CLAIM-VERIFY-3 — load open conflicts so the panel populates on mount.
  void onRefreshConflicts();
});
</script>

<style scoped src="./MemoryView.css"></style>
