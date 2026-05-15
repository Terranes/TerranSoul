<template>
  <div
    class="bp-shell brain-view"
    data-density="cozy"
    data-testid="brain-view"
  >
    <!-- ── Breadcrumb ──────────────────────────────────────────────────────── -->
    <AppBreadcrumb
      here="BRAIN PANEL"
      @navigate="emit('navigate', $event)"
    />

    <!-- ── Cockpit hero (neon brain orb + status + ACTIVE BRAIN card) ──── -->
    <section class="bp-cockpit">
      <div
        class="bp-hero-avatar"
        data-testid="brain-avatar"
        :class="`mood-${moodKey}`"
      >
        <BrainOrb
          :lighting="brain.brainMode ? 'bright' : 'dim'"
          :state="brain.brainMode ? 'healthy' : 'offline'"
        />
      </div>
      <div class="bp-hero-body">
        <div class="bp-hero-eyebrow">
          COMPANION COCKPIT · <span>{{ moodPillLabel }}</span>
        </div>
        <h1 class="bp-hero-name">
          {{ heroTitle }}
        </h1>
        <p class="bp-hero-sub">
          {{ heroSubtitle }}
        </p>
        <div class="bp-hero-pills">
          <span
            class="bp-pill"
            data-tone="ok"
          >
            <span class="dot" />
            <strong>{{ memoryCount.toLocaleString() }}</strong> memories
          </span>
          <span
            v-if="edgeCount > 0"
            class="bp-pill"
            data-tone="violet"
          >
            <span class="dot" />
            <strong>{{ edgeCount.toLocaleString() }}</strong> connections
          </span>
          <span
            v-if="brain.ollamaStatus.running || brain.lmStudioStatus?.running"
            class="bp-pill"
            data-tone="pink"
          >
            <span class="dot" />
            {{ brain.ollamaStatus.running && brain.lmStudioStatus?.running ? 'Ollama + LM Studio' : brain.ollamaStatus.running ? 'Ollama running' : 'LM Studio running' }}
          </span>
          <span
            v-if="!brain.brainMode"
            class="bp-pill"
            data-tone="warn"
          >
            <span class="dot" />
            Setup required
          </span>
        </div>
      </div>
      <aside class="bp-now">
        <div class="bp-now-head">
          <span class="bp-now-eyebrow">ACTIVE BRAIN</span>
          <span
            class="bp-now-badge"
            :data-tone="brain.brainMode ? 'ok' : 'warn'"
          >{{ brain.brainMode ? 'ONLINE' : 'OFFLINE' }}</span>
        </div>
        <div class="bp-now-model">
          <strong>{{ configRows.model || 'No model' }}</strong>
        </div>
        <div class="bp-now-meta">
          <span>provider</span>
          <b>{{ configRows.provider }}</b>
        </div>
        <div
          v-if="configRows.endpoint"
          class="bp-now-meta"
        >
          <span>endpoint</span>
          <b>{{ shortUrl(configRows.endpoint) }}</b>
        </div>
        <div class="bp-now-meta">
          <span>mode</span>
          <b>{{ configRows.mode }}</b>
        </div>
        <div class="bp-now-actions">
          <button
            class="bp-btn bp-btn--primary bp-btn--sm"
            @click="$emit('navigate', 'brain-setup')"
          >
            Brain setup
          </button>
          <button
            class="bp-btn bp-btn--ghost bp-btn--sm"
            :disabled="isRefreshing"
            @click="refresh"
          >
            {{ isRefreshing ? '⟳' : '↻' }} Refresh
          </button>
        </div>
      </aside>
    </section>

    <!-- ── 01 · Provider & Model ───────────────────────────────────────── -->
    <section
      class="bp-module bp-module--feature"
      data-testid="bv-mode-switcher"
    >
      <header class="bp-module-head">
        <div class="bp-module-head-left">
          <div class="bp-module-eyebrow">
            <span class="ix">01</span> Provider & Model
          </div>
          <h2 class="bp-module-title">
            Where your brain thinks
          </h2>
          <p class="bp-module-sub">
            Pick a cloud or local LLM. Local Ollama is required for vector
            search (40 % of retrieval quality).
          </p>
        </div>
        <div class="bp-module-head-right">
          <button
            class="bp-btn bp-btn--ghost bp-btn--sm"
            @click="$emit('navigate', 'marketplace')"
          >
            Switch model
          </button>
        </div>
      </header>
      <div class="bp-providers">
        <button
          v-for="opt in modeOptions"
          :key="opt.key"
          type="button"
          class="bp-prov bv-mode-card"
          :class="{ active: opt.key === moodKey }"
          :data-active="opt.key === moodKey ? 'true' : 'false'"
          :disabled="opt.disabled"
          :title="opt.disabled ? opt.disabledReason : opt.description"
          @click="opt.disabled ? null : opt.action()"
        >
          <div class="bp-prov-head">
            <div class="bp-prov-icon">
              <span style="font-size: 18px;">{{ opt.emoji }}</span>
            </div>
            <span class="bp-prov-radio" />
          </div>
          <div class="bp-prov-name">
            {{ opt.label }}
          </div>
          <p class="bp-prov-desc">
            {{ opt.detail }}
          </p>
          <div class="bp-prov-foot">
            <span
              class="bp-prov-tag"
              :data-tone="opt.key === 'free' ? 'ok' : opt.key === 'paid' ? 'violet' : 'pink'"
            >{{ opt.key === 'free' ? 'FREE' : opt.key === 'paid' ? 'PAID' : 'LOCAL' }}</span>
            <span class="price">{{ opt.description }}</span>
          </div>
        </button>
      </div>
    </section>

    <!-- ── 02 · Coding LLM (Phase 25 — Self-Improve) ─────────────────────── -->
    <section
      class="bp-module"
      data-testid="bv-coding-llm"
    >
      <header class="bp-module-head">
        <div class="bp-module-head-left">
          <div class="bp-module-eyebrow">
            <span class="ix">02</span> Coding LLM
          </div>
          <h2 class="bp-module-title">
            Self-Improve engine
          </h2>
          <p class="bp-module-sub">
            Pick a dedicated provider for autonomous coding. Separate from the chat brain above.
          </p>
        </div>
      </header>

      <div class="bv-coding-llm-grid">
        <button
          v-for="rec in selfImprove.recommendations"
          :key="rec.provider + '::' + rec.display_name"
          type="button"
          :class="[
            'bv-coding-card',
            { active: selectedCodingRecKey === rec.provider + '::' + rec.display_name, top: rec.is_top_pick },
          ]"
          @click="selectedCodingRecKey = rec.provider + '::' + rec.display_name"
        >
          <div class="bv-coding-card-head">
            <strong>{{ rec.display_name }}</strong>
            <span
              v-if="rec.is_top_pick"
              class="bv-coding-badge"
            >⭐ Top pick</span>
          </div>
          <p class="bv-coding-notes">
            {{ rec.notes }}
          </p>
          <small>Default model: <code>{{ rec.default_model || 'custom' }}</code></small>
        </button>
      </div>

      <div
        v-if="selectedCodingRec"
        class="bv-coding-form"
      >
        <label>Model</label>
        <div
          v-if="isLocalOllamaSelection"
          class="bv-local-model-row"
        >
          <select
            v-if="localCodingModels.length > 0"
            v-model="codingModelInput"
            class="bv-input"
            data-testid="bv-local-model-select"
          >
            <option
              v-for="m in localCodingModels"
              :key="m"
              :value="m"
            >
              {{ m }}
            </option>
          </select>
          <input
            v-else
            v-model="codingModelInput"
            type="text"
            :placeholder="selectedCodingRec.default_model || 'gemma3:4b'"
            class="bv-input"
          >
          <button
            type="button"
            class="bv-btn bv-btn-ghost"
            :disabled="loadingLocalModels"
            data-testid="bv-refresh-local-models"
            @click="refreshLocalCodingModels"
          >
            {{ loadingLocalModels ? '…' : '↻' }}
          </button>
        </div>
        <input
          v-else
          v-model="codingModelInput"
          type="text"
          :placeholder="selectedCodingRec.default_model || 'gpt-4o'"
          class="bv-input"
        >
        <p
          v-if="isLocalOllamaSelection && !loadingLocalModels && localCodingModels.length === 0"
          class="bv-coding-hint"
        >
          ⚠ No Ollama models detected. Install one first:
          <code>ollama pull {{ selectedCodingRec.default_model || 'gemma3:4b' }}</code>
        </p>
        <label>Base URL</label>
        <input
          v-model="codingBaseUrlInput"
          type="url"
          :placeholder="selectedCodingRec.base_url || 'https://api.example.com/v1'"
          class="bv-input"
        >
        <template v-if="selectedCodingRec.requires_api_key">
          <label>API Key</label>
          <input
            v-model="codingApiKeyInput"
            type="password"
            placeholder="sk-…"
            class="bv-input"
          >
        </template>
        <p
          v-else
          class="bv-coding-hint"
        >
          🔒 No API key needed — this provider runs locally.
        </p>
        <div class="bv-coding-actions">
          <button
            type="button"
            class="bv-btn bv-btn-primary"
            :disabled="
              !(codingModelInput || selectedCodingRec.default_model) ||
                (selectedCodingRec.requires_api_key && !codingApiKeyInput)
            "
            @click="saveCodingLlm"
          >
            Save Coding LLM
          </button>
          <button
            v-if="selfImprove.codingLlm"
            type="button"
            class="bv-btn bv-btn-ghost"
            @click="clearCodingLlm"
          >
            Clear
          </button>
          <button
            v-if="selfImprove.codingLlm"
            type="button"
            class="bv-btn bv-btn-ghost"
            :disabled="codingTestInFlight"
            data-testid="bv-coding-test"
            @click="testCodingLlm"
          >
            {{ codingTestInFlight ? 'Testing…' : '🔌 Test connection' }}
          </button>
        </div>
        <p
          v-if="selfImprove.reachability"
          class="bv-coding-status"
          :class="selfImprove.reachability.ok ? 'bv-coding-status--ok' : 'bv-coding-status--err'"
        >
          {{ selfImprove.reachability.summary }}
          <span
            v-if="selfImprove.reachability.detail"
            class="bv-coding-detail"
          >— {{ selfImprove.reachability.detail }}</span>
        </p>
        <p
          v-if="selfImprove.codingLlm"
          class="bv-coding-status bv-coding-status--ok"
        >
          ✓ Configured: {{ selfImprove.codingLlm.provider }} ·
          <code>{{ selfImprove.codingLlm.model }}</code>
        </p>
      </div>
    </section>

    <!-- ── 03 · Coding Workflow Context (Chunk 25.16) ─────────────────────── -->
    <section
      class="bp-module"
      data-testid="bv-coding-workflow"
    >
      <header class="bp-module-head">
        <div class="bp-module-head-left">
          <div class="bp-module-eyebrow">
            <span class="ix">03</span> Coding Workflow
          </div>
          <h2 class="bp-module-title">
            Context & integration
          </h2>
        </div>
      </header>
      <CodingWorkflowConfigPanel />
    </section>

    <!-- ── Background task progress ────────────────────────────────────────── -->
    <TaskProgressBar />

    <!-- ── Self-healing embedding queue strip (Chunk 38.2) ─────────────────── -->
    <section
      v-if="embedQueueStatus && embedQueueStatus.pending > 0"
      class="bv-embed-queue"
      data-testid="bv-embed-queue"
      role="status"
      aria-live="polite"
      :class="{ 'bv-embed-queue--open': embedQueueDebugOpen }"
    >
      <div class="bv-embed-queue__row">
        <span
          class="bv-embed-queue__icon"
          aria-hidden="true"
        >🔄</span>
        <span class="bv-embed-queue__text">
          Self-healing embeddings:
          <strong>{{ embedQueueStatus.pending }}</strong> pending
          <template v-if="embedQueueStatus.failing > 0">
            (<span class="bv-embed-queue__warn">{{ embedQueueStatus.failing }}</span> retrying)
          </template>
        </span>
        <span class="bv-embed-queue__hint">Auto-retries every 10 s — no action needed.</span>
        <button
          type="button"
          class="bv-embed-queue__toggle"
          data-testid="bv-embed-queue-toggle"
          :aria-expanded="embedQueueDebugOpen"
          aria-controls="bv-embed-queue-debug"
          @click="toggleEmbedQueueDebug"
        >
          <span aria-hidden="true">{{ embedQueueDebugOpen ? '▾' : '▸' }}</span>
          {{ embedQueueDebugOpen ? 'Hide debug log' : 'Show debug log' }}
        </button>
      </div>

      <div
        v-if="embedQueueDebugOpen"
        id="bv-embed-queue-debug"
        class="bv-embed-queue__debug"
        data-testid="bv-embed-queue-debug"
      >
        <p class="bv-embed-queue__reason">
          {{ embedQueueDiagnostics?.reason ?? 'Loading diagnostics…' }}
        </p>

        <dl
          v-if="embedQueueDiagnostics"
          class="bv-embed-queue__facts"
        >
          <div>
            <dt>Brain</dt>
            <dd>{{ embedQueueDiagnostics.brain_mode_label ?? 'Not configured' }}</dd>
          </div>
          <div>
            <dt>Worker</dt>
            <dd>
              <template v-if="embedQueueDiagnostics.worker.rate_limited">
                Paused ({{ embedQueueDiagnostics.worker.pause_remaining_secs }}s left)
              </template>
              <template v-else-if="embedQueueDiagnostics.ollama_chat_skip_active">
                Throttled by chat activity
              </template>
              <template v-else>
                Active
              </template>
            </dd>
          </div>
          <div>
            <dt>Embedded</dt>
            <dd>{{ embedQueueDiagnostics.worker.total_embedded }} since boot</dd>
          </div>
          <div>
            <dt>Hard failures</dt>
            <dd>{{ embedQueueDiagnostics.worker.hard_failures }}</dd>
          </div>
          <div>
            <dt>Rate-limit pauses</dt>
            <dd>{{ embedQueueDiagnostics.worker.rate_limit_pauses }}</dd>
          </div>
          <div v-if="embedQueueDiagnostics.status.next_retry_at">
            <dt>Next retry</dt>
            <dd>{{ formatRetryEta(embedQueueDiagnostics.status.next_retry_at, embedQueueDiagnostics.now_ms) }}</dd>
          </div>
        </dl>

        <div
          v-if="embedQueueDiagnostics && embedQueueDiagnostics.recent_failures.length > 0"
          class="bv-embed-queue__failures"
        >
          <h4 class="bv-embed-queue__failures-title">
            Recent failures ({{ embedQueueDiagnostics.recent_failures.length }})
          </h4>
          <ul class="bv-embed-queue__failure-list">
            <li
              v-for="row in embedQueueDiagnostics.recent_failures"
              :key="row.memory_id"
              class="bv-embed-queue__failure"
            >
              <div class="bv-embed-queue__failure-head">
                <span class="bv-embed-queue__failure-id">#{{ row.memory_id }}</span>
                <span class="bv-embed-queue__failure-attempts">{{ row.attempts }} attempts</span>
                <span class="bv-embed-queue__failure-eta">{{ formatRetryEta(row.next_retry_at, embedQueueDiagnostics?.now_ms ?? Date.now()) }}</span>
              </div>
              <div
                v-if="row.last_error"
                class="bv-embed-queue__failure-error"
              >
                {{ row.last_error }}
              </div>
              <div class="bv-embed-queue__failure-preview">
                {{ row.content_preview }}
              </div>
            </li>
          </ul>
        </div>
        <p
          v-else-if="embedQueueDiagnostics"
          class="bv-embed-queue__no-failures"
        >
          No per-row failures recorded yet — entries are waiting for the next worker tick.
        </p>
      </div>
    </section>

    <!-- ── 04 · Brain Capacity & Storage (Chunk 38.5) ─────────────────────── -->
    <section class="bp-module">
      <header class="bp-module-head">
        <div class="bp-module-head-left">
          <div class="bp-module-eyebrow">
            <span class="ix">04</span> Brain Capacity
          </div>
          <h2 class="bp-module-title">
            Storage & embeddings
          </h2>
        </div>
      </header>
      <BrainCapacityPanel />
    </section>

    <!-- ── 05 · Context Folders — user-defined knowledge directories ─────── -->
    <section class="bp-module">
      <header class="bp-module-head">
        <div class="bp-module-head-left">
          <div class="bp-module-eyebrow">
            <span class="ix">05</span> Knowledge Sources
          </div>
          <h2 class="bp-module-title">
            Context folders
          </h2>
          <p class="bp-module-sub">
            ⚠️ Not recommended for large directories. Scanning is brute-force
            and can be slow for folders with thousands of files.
          </p>
        </div>
        <div class="bp-module-head-right">
          <button
            class="bp-btn bp-btn--ghost bp-btn--sm"
            data-testid="bv-context-folder-sync"
            :disabled="(contextFolders?.length ?? 0) === 0 || isSyncing"
            @click="syncAllContextFolders"
          >
            {{ isSyncing ? 'Syncing…' : '↻ Sync All' }}
          </button>
        </div>
      </header>

      <div class="bv-context-add">
        <input
          v-model="contextFolderInput"
          type="text"
          class="bv-context-input"
          placeholder="Paste folder path (e.g. D:\Docs\Notes)"
          data-testid="bv-context-folder-input"
          @keydown.enter="addContextFolder"
        >
        <button
          class="bv-btn bv-btn--accent"
          data-testid="bv-context-folder-add"
          :disabled="!contextFolderInput.trim()"
          @click="addContextFolder"
        >
          Add Folder
        </button>
        <button
          class="bv-btn bv-btn--secondary"
          data-testid="bv-context-folder-sync"
          :disabled="(contextFolders?.length ?? 0) === 0 || isSyncing"
          @click="syncAllContextFolders"
        >
          {{ isSyncing ? 'Syncing…' : '🔄 Sync All' }}
        </button>
      </div>

      <div
        v-if="!contextFolders || contextFolders.length === 0"
        class="bv-context-empty"
      >
        No context folders configured. Add a folder path above to start
        ingesting local documents into the brain.
      </div>

      <ul
        v-else
        class="bv-context-list"
      >
        <li
          v-for="folder in contextFolders"
          :key="folder.path"
          class="bv-context-item"
          :class="{ 'bv-context-item--disabled': !folder.enabled }"
        >
          <label class="bv-context-toggle">
            <input
              type="checkbox"
              :checked="folder.enabled"
              @change="toggleFolder(folder.path, !folder.enabled)"
            >
          </label>
          <div class="bv-context-info">
            <span class="bv-context-label">{{ folder.label }}</span>
            <span class="bv-context-path">{{ folder.path }}</span>
            <span
              v-if="folder.last_synced_at > 0"
              class="bv-context-meta"
            >
              Last synced: {{ formatDate(folder.last_synced_at) }}
              · {{ folder.last_file_count }} files
            </span>
          </div>
          <button
            class="bv-btn bv-btn--danger-sm"
            title="Remove folder"
            @click="removeFolder(folder.path)"
          >
            ✕
          </button>
        </li>
      </ul>

      <div
        v-if="syncResult"
        class="bv-context-result"
      >
        Synced {{ syncResult.folders_synced }} folder(s),
        {{ syncResult.files_ingested }} file(s) ingested.
        <span
          v-if="syncResult.errors.length > 0"
          class="bv-context-errors"
        >
          {{ syncResult.errors.length }} error(s).
        </span>
      </div>

      <!-- Conversion tools -->
      <div class="bv-context-conversion">
        <h4 class="bv-context-conversion-title">
          🔄 Context ↔ Knowledge Conversion
        </h4>
        <p class="bv-context-conversion-hint">
          Convert raw context-folder chunks into consolidated knowledge entries
          for better RAG retrieval, or export knowledge back to portable files.
        </p>

        <div
          v-if="contextMemoryInfo"
          class="bv-context-stats"
        >
          {{ contextMemoryInfo.total_memories }} context memories
          ({{ Math.round(contextMemoryInfo.total_tokens / 1000) }}k tokens)
          across {{ contextMemoryInfo.by_folder.length }} folder(s).
        </div>

        <div class="bv-context-conversion-actions">
          <button
            class="bv-btn bv-btn--secondary"
            data-testid="bv-context-convert"
            :disabled="isConverting || !contextMemoryInfo || contextMemoryInfo.total_memories === 0"
            @click="convertToKnowledge"
          >
            {{ isConverting ? 'Converting…' : '📚 Convert to Knowledge' }}
          </button>

          <div class="bv-context-export-row">
            <input
              v-model="exportFolderInput"
              type="text"
              class="bv-context-input bv-context-export-input"
              placeholder="Export path (e.g. D:\Export\Knowledge)"
              data-testid="bv-context-export-input"
            >
            <button
              class="bv-btn bv-btn--secondary"
              data-testid="bv-context-export"
              :disabled="!exportFolderInput.trim() || isExporting"
              @click="exportKnowledge"
            >
              {{ isExporting ? 'Exporting…' : '💾 Export to Files' }}
            </button>
          </div>
        </div>

        <div
          v-if="conversionResult"
          class="bv-context-result"
        >
          {{ conversionResult.summary }}
        </div>
        <div
          v-if="exportResult"
          class="bv-context-result"
        >
          Exported {{ exportResult.files_written }} file(s) to {{ exportResult.output_dir }}.
        </div>

        <!-- Knowledge Graph ↔ Files -->
        <div class="bv-kg-section">
          <h4 class="bv-context-conversion-title">
            🕸️ Knowledge Graph ↔ Files
          </h4>
          <p class="bv-context-conversion-hint">
            Import a text file as a structured KG subgraph (root → chunks with edges),
            or export a KG subtree starting from memory IDs.
          </p>

          <!-- File → KG import -->
          <div class="bv-kg-import-row">
            <input
              v-model="kgImportFilePath"
              type="text"
              class="bv-context-input"
              placeholder="File path to import (e.g. D:\Docs\notes.md)"
              data-testid="bv-kg-import-input"
            >
            <button
              class="bv-btn bv-btn--secondary"
              data-testid="bv-kg-import-btn"
              :disabled="!kgImportFilePath.trim() || isKgImporting"
              @click="importFileToKg"
            >
              {{ isKgImporting ? 'Importing…' : '📥 Import to KG' }}
            </button>
          </div>
          <div
            v-if="kgImportResult"
            class="bv-context-result"
          >
            {{ kgImportResult.summary }}
          </div>

          <!-- KG → files export -->
          <div class="bv-kg-export-row">
            <input
              v-model="kgExportRootIds"
              type="text"
              class="bv-context-input bv-kg-ids-input"
              placeholder="Root memory IDs (comma-separated, e.g. 42,78)"
              data-testid="bv-kg-export-ids"
            >
            <input
              v-model="kgExportDir"
              type="text"
              class="bv-context-input"
              placeholder="Output directory"
              data-testid="bv-kg-export-dir"
            >
            <button
              class="bv-btn bv-btn--secondary"
              data-testid="bv-kg-export-btn"
              :disabled="!kgExportRootIds.trim() || !kgExportDir.trim() || isKgExporting"
              @click="exportKgSubtree"
            >
              {{ isKgExporting ? 'Exporting…' : '📤 Export KG Subtree' }}
            </button>
          </div>
          <div
            v-if="kgExportResult"
            class="bv-context-result"
          >
            Exported {{ kgExportResult.nodes_exported }} nodes,
            {{ kgExportResult.edges_exported }} edges,
            {{ kgExportResult.files_written }} files
            to {{ kgExportResult.output_dir }}.
          </div>
        </div>
      </div>
    </section>

    <!-- ── 06 · Knowledge Wiki (graph + LLM-wiki pattern) ──────────────── -->
    <section class="bp-module">
      <header class="bp-module-head">
        <div class="bp-module-head-left">
          <div class="bp-module-eyebrow">
            <span class="ix">06</span> Knowledge Wiki
          </div>
          <h2 class="bp-module-title">
            Graph & LLM-wiki patterns
          </h2>
        </div>
      </header>
      <WikiPanel />
    </section>

    <!-- ── 07 · System Overview ───────────────────────────────────────────── -->
    <section class="bp-module">
      <header class="bp-module-head">
        <div class="bp-module-head-left">
          <div class="bp-module-eyebrow">
            <span class="ix">07</span> System Overview
          </div>
          <h2 class="bp-module-title">
            Configuration · Hardware · Memory health
          </h2>
        </div>
      </header>
      <div class="bp-grid bp-grid-3">
        <!-- Brain config card -->
        <article
          class="bv-card"
          data-testid="bv-card-config"
        >
          <header class="bv-card-header">
            <h3>🧬 Configuration</h3>
          </header>
          <dl class="bv-dl">
            <div class="bv-dl-row">
              <dt>Mode</dt>
              <dd>{{ configRows.mode }}</dd>
            </div>
            <div class="bv-dl-row">
              <dt>Provider</dt>
              <dd>{{ configRows.provider }}</dd>
            </div>
            <div class="bv-dl-row">
              <dt>Model</dt>
              <dd class="bv-model">
                <code>{{ configRows.model }}</code>
              </dd>
            </div>
            <div
              v-if="configRows.endpoint"
              class="bv-dl-row"
            >
              <dt>Endpoint</dt>
              <dd
                class="bv-endpoint"
                :title="configRows.endpoint"
              >
                {{ shortUrl(configRows.endpoint) }}
              </dd>
            </div>
            <div
              v-if="configRows.embeddingModel"
              class="bv-dl-row"
            >
              <dt>Embedding</dt>
              <dd class="bv-model">
                <code>{{ configRows.embeddingModel }}</code>
              </dd>
            </div>
          </dl>
        </article>

        <!-- Hardware card -->
        <article
          class="bv-card"
          data-testid="bv-card-hardware"
        >
          <header class="bv-card-header">
            <h3>💻 Hardware</h3>
          </header>
          <dl class="bv-dl">
            <div class="bv-dl-row">
              <dt>OS</dt>
              <dd>{{ hardwareRows.os }}</dd>
            </div>
            <div class="bv-dl-row">
              <dt>CPU</dt>
              <dd>{{ hardwareRows.cpu }}</dd>
            </div>
            <div class="bv-dl-row">
              <dt>RAM</dt>
              <dd>{{ hardwareRows.ram }}</dd>
            </div>
            <div
              v-if="hardwareRows.gpu"
              class="bv-dl-row"
            >
              <dt>GPU</dt>
              <dd>{{ hardwareRows.gpu }}</dd>
            </div>
          </dl>
          <div
            v-if="ramTier"
            class="bv-ram-bar"
            :title="`RAM tier: ${ramTier.label}`"
          >
            <div
              class="bv-ram-fill"
              :style="{ width: ramTier.percent + '%', background: ramTier.color }"
            />
          </div>
        </article>

        <!-- Memory health card -->
        <article
          class="bv-card"
          data-testid="bv-card-memory"
        >
          <header class="bv-card-header">
            <h3>🧠 Memory health</h3>
            <button
              class="bv-card-link"
              @click="emitNavigate('memory')"
            >
              Open explorer →
            </button>
          </header>
          <div class="bv-memory-tiers">
            <div
              class="bv-mem-tier tier-short"
              :title="`Short-term: ${memoryStats.short_count}`"
            >
              <span class="bv-mem-num">{{ memoryStats.short_count }}</span>
              <span class="bv-mem-label">short</span>
            </div>
            <div
              class="bv-mem-tier tier-working"
              :title="`Working: ${memoryStats.working_count}`"
            >
              <span class="bv-mem-num">{{ memoryStats.working_count }}</span>
              <span class="bv-mem-label">working</span>
            </div>
            <div
              class="bv-mem-tier tier-long"
              :title="`Long-term: ${memoryStats.long_count}`"
            >
              <span class="bv-mem-num">{{ memoryStats.long_count }}</span>
              <span class="bv-mem-label">long</span>
            </div>
          </div>
          <dl class="bv-dl">
            <div class="bv-dl-row">
              <dt>Total memories</dt>
              <dd>{{ memoryStats.total }}</dd>
            </div>
            <div class="bv-dl-row">
              <dt>Connections</dt>
              <dd>{{ edgeCount }} edge{{ edgeCount === 1 ? '' : 's' }}</dd>
            </div>
            <div class="bv-dl-row">
              <dt>Avg freshness</dt>
              <dd>
                <span class="bv-decay-bar">
                  <span
                    class="bv-decay-fill"
                    :style="{ width: (memoryStats.avg_decay * 100) + '%' }"
                  />
                </span>
                <span class="bv-decay-num">{{ Math.round(memoryStats.avg_decay * 100) }}%</span>
              </dd>
            </div>
          </dl>
        </article>
      </div>
    </section>

    <!-- ── 08 · Retrieval Quality ─────────────────────────────────────────── -->
    <section
      class="bp-module"
      data-testid="bv-cognitive-breakdown"
    >
      <header class="bp-module-head">
        <div class="bp-module-head-left">
          <div class="bp-module-eyebrow">
            <span class="ix">08</span> Retrieval Quality
          </div>
          <h2 class="bp-module-title">
            Cognitive kinds & RAG signals
          </h2>
          <p class="bp-module-sub">
            Episodic / Semantic / Procedural — derived from tags + content
          </p>
        </div>
      </header>
      <div
        v-if="cognitiveKinds.total === 0"
        class="bv-cognitive-empty"
      >
        No memories yet — once you add some, they'll be classified here.
      </div>
      <div
        v-else
        class="bv-cognitive-bars"
      >
        <div
          v-for="row in cognitiveRows"
          :key="row.key"
          class="bv-cog-row"
          :class="`bv-cog-${row.key}`"
          :data-testid="`bv-cog-${row.key}`"
        >
          <div class="bv-cog-head">
            <span class="bv-cog-emoji">{{ row.emoji }}</span>
            <span class="bv-cog-name">{{ row.label }}</span>
            <span class="bv-cog-count">{{ row.count }} <small>({{ row.percent }}%)</small></span>
          </div>
          <div class="bv-cog-bar">
            <div
              class="bv-cog-bar-fill"
              :style="{ width: row.percent + '%' }"
            />
          </div>
          <div class="bv-cog-desc">
            {{ row.description }}
          </div>
        </div>
      </div>
    </section>

    <!-- ── RAG capability strip (docs §4 / §10) ────────────────────────────── -->
    <section
      class="bp-module"
      data-testid="bv-rag-capability"
    >
      <header class="bp-module-head">
        <div class="bp-module-head-left">
          <div class="bp-module-eyebrow">
            <span class="ix">09</span> RAG Signals
          </div>
          <h2 class="bp-module-title">
            6-signal hybrid scoring
          </h2>
          <p class="bp-module-sub">
            Vector search needs a local embedding model
          </p>
        </div>
      </header>
      <div class="bp-rag-grid">
        <div
          v-for="sig in ragSignals"
          :key="sig.key"
          class="bp-rag-cell bv-rag-cell"
          :class="{ 'is-on': sig.available, 'is-off': !sig.available }"
          :data-on="sig.available ? 'true' : 'false'"
          :title="sig.available ? `${sig.label} active` : sig.unavailableReason"
        >
          <span class="bp-rag-check">{{ sig.available ? '✓' : '✗' }}</span>
          <span class="bp-rag-name">{{ sig.label }}</span>
          <span class="bp-rag-weight">{{ sig.weight }}</span>
        </div>
      </div>
      <div class="bp-rag-quality">
        <div class="bp-quality-ring">
          <svg viewBox="0 0 64 64">
            <circle
              class="bg"
              cx="32"
              cy="32"
              r="28"
            />
            <circle
              class="fg"
              cx="32"
              cy="32"
              r="28"
              :stroke-dasharray="`${(ragQuality.effective / 100) * 175.9} 175.9`"
            />
          </svg>
          <span class="bp-quality-num">{{ ragQuality.effective }}<small>%</small></span>
        </div>
        <div class="bp-quality-text">
          <strong>Effective quality</strong>
          <small>{{ ragQuality.note }}</small>
        </div>
      </div>
    </section>


    <!-- ── Active selection (docs §20) ─────────────────────────────────────── -->
    <section
      class="bp-module"
      data-testid="bv-active-selection"
    >
      <header class="bp-module-head">
        <div class="bp-module-head-left">
          <div class="bp-module-eyebrow">
            <span class="ix">10</span> Active Selection
          </div>
          <h2 class="bp-module-title">
            How the brain picks each component
          </h2>
        </div>
      </header>
      <div
        v-if="!brainSelection"
        class="bv-cog-desc"
      >
        Loading…
      </div>
      <dl
        v-else
        class="bv-config-list"
      >
        <dt>Provider</dt><dd>{{ selectionProviderLine }}</dd>
        <dt>Embedding</dt><dd>{{ selectionEmbeddingLine }}</dd>
        <dt>Search</dt><dd>{{ selectionSearchLine }}</dd>
        <dt>Storage</dt><dd>{{ selectionStorageLine }}</dd>
        <dt>Agents</dt><dd>{{ selectionAgentsLine }}</dd>
        <dt>RAG quality</dt><dd>{{ brainSelection.rag_quality_percent }}% — {{ brainSelection.rag_quality_note }}</dd>
      </dl>
    </section>

    <!-- ── Daily learning (docs §21) ───────────────────────────────────────── -->
    <section
      class="bp-module"
      data-testid="bv-daily-learning"
    >
      <header class="bp-module-head">
        <div class="bp-module-head-left">
          <div class="bp-module-eyebrow">
            <span class="ix">11</span> Daily Learning
          </div>
          <h2 class="bp-module-title">
            Conversation → long-term memory
          </h2>
        </div>
      </header>
      <div
        v-if="!autoLearnPolicy"
        class="bv-cog-desc"
      >
        Loading…
      </div>
      <template v-else>
        <label
          class="bv-config-list"
          style="display:flex;align-items:center;gap:8px;"
        >
          <input
            type="checkbox"
            :checked="autoLearnPolicy.enabled"
            data-testid="bv-autolearn-toggle"
            @change="onToggleAutoLearn(($event.target as HTMLInputElement).checked)"
          >
          <span>Enable auto-learn from conversation</span>
        </label>
        <dl class="bv-config-list">
          <dt>Cadence</dt>
          <dd>Fire every {{ autoLearnPolicy.every_n_turns }} turns (cooldown {{ autoLearnPolicy.min_cooldown_turns }})</dd>
          <dt>This session</dt>
          <dd>{{ autoLearnSessionLine }}</dd>
          <dt>Status</dt>
          <dd>{{ autoLearnStatusLine }}</dd>
        </dl>
        <div style="display:flex;gap:8px;margin-top:8px;flex-wrap:wrap;">
          <button
            class="bv-link"
            data-testid="bv-autolearn-force"
            @click="forceExtractNow"
          >
            Extract now →
          </button>
          <button
            class="bv-link"
            data-testid="bv-autolearn-replay"
            :disabled="replayState.running"
            @click="replayHistoryNow"
          >
            {{ replayState.running
              ? `Replaying ${replayState.processed}/${replayState.total}…`
              : 'Replay history →' }}
          </button>
        </div>
        <p
          v-if="replayState.lastResultLine"
          class="bv-cog-desc"
          data-testid="bv-autolearn-replay-result"
          style="margin-top:6px;"
        >
          {{ replayState.lastResultLine }}
        </p>
      </template>
    </section>

    <!-- ── AI decision-making toggles ──────────────────────────────────────── -->
    <section
      class="bp-module"
      data-testid="bv-ai-decisions"
    >
      <header class="bp-module-head">
        <div class="bp-module-head-left">
          <div class="bp-module-eyebrow">
            <span class="ix">12</span> Automation
          </div>
          <h2 class="bp-module-title">
            AI decision-making
          </h2>
          <p class="bp-module-sub">
            Toggle opinionated routing decisions. Settings persist locally.
          </p>
        </div>
      </header>
      <div
        class="bv-config-list"
        data-testid="bv-ai-decisions-list"
      >
        <label
          v-for="row in decisionToggleRows"
          :key="row.key"
          class="bv-aidp-row"
          style="display:flex;align-items:flex-start;gap:8px;padding:6px 0;"
        >
          <input
            type="checkbox"
            :checked="aiDecisionPolicy[row.key]"
            :data-testid="row.testid"
            style="margin-top:3px;flex:none;"
            @change="onToggleDecision(row.key, ($event.target as HTMLInputElement).checked)"
          >
          <span style="display:flex;flex-direction:column;gap:2px;">
            <span style="font-weight:600;">{{ row.label }}</span>
            <span
              class="bv-cog-desc"
              style="font-size:0.85em;"
            >{{ row.description }}</span>
          </span>
        </label>
      </div>
      <div style="display:flex;gap:8px;margin-top:8px;">
        <button
          class="bv-link"
          data-testid="bv-aidp-reset"
          @click="resetDecisionPolicy"
        >
          Reset to defaults →
        </button>
      </div>
    </section>

    <!-- ── 13 · Preferences ───────────────────────────────────────────────── -->
    <section class="bp-module">
      <header class="bp-module-head">
        <div class="bp-module-head-left">
          <div class="bp-module-eyebrow">
            <span class="ix">13</span> Preferences
          </div>
          <h2 class="bp-module-title">
            Brain policies & auto-tag
          </h2>
        </div>
      </header>
      <div class="bp-row">
        <div class="bp-row-text">
          <span class="bp-row-label">First-Launch Brain Policy</span>
          <span class="bp-row-desc">
            Auto-configure a local Ollama model from the §26 catalogue at first launch.
            Falls back to free cloud if Ollama is unreachable.
          </span>
        </div>
        <button
          type="button"
          class="bp-switch"
          :data-on="(appSettings.settings?.prefer_local_brain !== false) ? 'true' : 'false'"
          data-testid="bv-prefer-local-toggle"
          @click="onTogglePreferLocal(appSettings.settings?.prefer_local_brain === false)"
        />
      </div>
      <div class="bp-row">
        <div class="bp-row-text">
          <span class="bp-row-label">Auto-Tag</span>
          <span class="bp-row-desc">
            Classify new memories with LLM tags (<code>personal:*</code>, <code>domain:*</code>, <code>code:*</code>).
          </span>
        </div>
        <button
          type="button"
          class="bp-switch"
          :data-on="(appSettings.settings?.auto_tag ?? true) ? 'true' : 'false'"
          data-testid="bv-autotag-toggle"
          @click="onToggleAutoTag(!(appSettings.settings?.auto_tag ?? true))"
        />
      </div>
    </section>

    <!-- ── 14 · RPG stat sheet ─────────────────────────────────────────────── -->
    <section class="bp-module">
      <header class="bp-module-head">
        <div class="bp-module-head-left">
          <div class="bp-module-eyebrow">
            <span class="ix">14</span> Stat Sheet
          </div>
          <h2 class="bp-module-title">
            Brain RPG stats
          </h2>
        </div>
      </header>
      <BrainStatSheet />
    </section>

    <!-- ── 15 · Plugins — Phase 22 ────────────────────────────────────────── -->
    <section
      class="bp-module"
      data-testid="bv-plugins-section"
    >
      <header class="bp-module-head">
        <div class="bp-module-head-left">
          <div class="bp-module-eyebrow">
            <span class="ix">15</span> Plugins
          </div>
          <h2 class="bp-module-title">
            Installed extensions
            <span
              v-if="openclawActive"
              class="bp-badge bp-badge--active"
              data-testid="bv-openclaw-active-badge"
            >OpenClaw active</span>
          </h2>
        </div>
      </header>
      <PluginsView />
    </section>

    <!-- ── 16 · Prompt Commands ────────────────────────────────────────────── -->
    <section
      class="bp-module"
      data-testid="bv-prompt-commands-section"
    >
      <header class="bp-module-head">
        <div class="bp-module-head-left">
          <div class="bp-module-eyebrow">
            <span class="ix">16</span> Prompt Commands
          </div>
          <h2 class="bp-module-title">
            Custom shortcuts
          </h2>
        </div>
      </header>
      <PromptCommandsPanel />
    </section>

    <!-- ── 17 · AI Coding Integrations ─────────────────────────────────────── -->
    <section
      class="bp-module"
      data-testid="bv-aiv-section"
    >
      <header class="bp-module-head">
        <div class="bp-module-head-left">
          <div class="bp-module-eyebrow">
            <span class="ix">17</span> AI Coding
          </div>
          <h2 class="bp-module-title">
            External integrations
          </h2>
        </div>
      </header>
      <AICodingIntegrationsView />
    </section>

    <!-- ── 18 · LAN Brain Sharing ──────────────────────────────────────────── -->
    <section
      class="bp-module"
      data-testid="bv-lan-share-section"
    >
      <header class="bp-module-head">
        <div class="bp-module-head-left">
          <div class="bp-module-eyebrow">
            <span class="ix">18</span> LAN Sharing
          </div>
          <h2 class="bp-module-title">
            Remote MCP retrieval
          </h2>
          <p class="bp-module-sub">
            Expose brain to your local network. Discovery on UDP 7424;
            retrieval requires MCP bearer token.
          </p>
        </div>
        <div class="bp-module-head-right">
          <button
            type="button"
            class="bp-switch"
            :data-on="(appSettings.settings?.lan_enabled ?? false) ? 'true' : 'false'"
            data-testid="bv-lan-enabled-toggle"
            @click="onToggleLanEnabled(!(appSettings.settings?.lan_enabled ?? false))"
          />
        </div>
      </header>
      <LanSharePanel v-if="appSettings.settings?.lan_enabled ?? false" />
      <p
        v-else
        class="bp-module-sub"
      >
        Enable LAN sharing, start the MCP server in the integrations panel, then
        use this panel to share or connect to another TerranSoul brain.
      </p>
    </section>

    <!-- Persona panel was moved to Settings → Character so it is clearly
         scoped to the active character/model, not a TerranSoul-wide
         config. The PersonaTraits store still applies globally as a
         fallback when the active character has no override, but the
         editor lives next to the active-model picker now. -->

    <!-- ── DANGER ZONE ─────────────────────────────────────────────────────── -->
    <section
      class="bp-module bp-danger"
      data-testid="bv-danger-zone"
    >
      <header class="bp-module-head">
        <div class="bp-module-head-left">
          <div class="bp-module-eyebrow">
            ⚠️ Danger Zone
          </div>
          <h2 class="bp-module-title">
            Irreversible actions
          </h2>
        </div>
      </header>
      <div class="bp-danger-list">
        <div class="bp-row">
          <div class="bp-row-text">
            <span class="bp-row-label">Factory reset</span>
            <span class="bp-row-desc">Remove all auto-configured components, clear memories and history. Reverts to first-launch state.</span>
          </div>
          <button
            class="bp-btn bp-btn--danger bp-btn--sm"
            data-testid="bv-factory-reset"
            @click="confirmFactoryReset"
          >
            Factory reset
          </button>
        </div>
        <div class="bp-row">
          <div class="bp-row-text">
            <span class="bp-row-label">Clean all data</span>
            <span class="bp-row-desc">Permanently erase everything: memories, brain config, voice, persona, quests, preferences.</span>
          </div>
          <button
            class="bp-btn bp-btn--danger bp-btn--sm"
            data-testid="bv-clear-all-data"
            @click="confirmClearAllData"
          >
            Clean all data
          </button>
        </div>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useBrainStore } from '../stores/brain';
import { useMemoryStore } from '../stores/memory';
import { useConversationStore } from '../stores/conversation';
import { useAiDecisionPolicyStore, type AiDecisionPolicy } from '../stores/ai-decision-policy';
import { useSettingsStore } from '../stores/settings';
import { useSelfImproveStore } from '../stores/self-improve';
import { useTaskStore } from '../stores/tasks';
import { usePluginStore } from '../stores/plugins';
import BrainOrb from '../components/BrainOrb.vue';
import BrainCapacityPanel from '../components/BrainCapacityPanel.vue';
import BrainStatSheet from '../components/BrainStatSheet.vue';
import CodingWorkflowConfigPanel from '../components/CodingWorkflowConfigPanel.vue';
import WikiPanel from '../components/WikiPanel.vue';
import TaskProgressBar from '../components/TaskProgressBar.vue';
import LanSharePanel from '../components/LanSharePanel.vue';
import PromptCommandsPanel from '../components/PromptCommandsPanel.vue';
import PluginsView from './PluginsView.vue';
import AICodingIntegrationsView from './AICodingIntegrationsView.vue';
import AppBreadcrumb from '../components/ui/AppBreadcrumb.vue';
import { summariseCognitiveKinds } from '../utils/cognitive-kind';
import { formatRam } from '../utils/format';

const emit = defineEmits<{
  /** Navigate to another tab; values match the App.vue tab ids. */
  (e: 'navigate', target: string): void;
}>();
const emitNavigate = (target: 'chat' | 'memory' | 'marketplace' | 'voice' | 'skills' | 'brain-setup') => {
  emit('navigate', target);
};

const brain = useBrainStore();
const memory = useMemoryStore();
const appSettings = useSettingsStore();
const taskStore = useTaskStore();
taskStore.init();
const pluginStore = usePluginStore();

const openclawActive = computed(() =>
  pluginStore.plugins.some(p => p.manifest.id === 'openclaw-bridge' && typeof p.state === 'string' && p.state.toLowerCase() === 'active'),
);

const isRefreshing = ref(false);

// ── Embedding queue status (Chunk 38.2) ─────────────────────────────────
type EmbeddingQueueStatus = {
  pending: number;
  failing: number;
  next_retry_at: number | null;
};
type EmbedWorkerStatus = {
  rate_limited: boolean;
  pause_remaining_secs: number;
  hard_failures: number;
  total_embedded: number;
  rate_limit_pauses: number;
};
type PendingEmbeddingDebugRow = {
  memory_id: number;
  attempts: number;
  last_error: string | null;
  next_retry_at: number;
  content_preview: string;
};
type EmbeddingQueueDiagnostics = {
  status: EmbeddingQueueStatus;
  worker: EmbedWorkerStatus;
  recent_failures: PendingEmbeddingDebugRow[];
  brain_configured: boolean;
  brain_mode_label: string | null;
  ollama_chat_skip_active: boolean;
  last_chat_at_ms: number;
  reason: string;
  now_ms: number;
};
const embedQueueStatus = ref<EmbeddingQueueStatus | null>(null);
const embedQueueDebugOpen = ref(false);
const embedQueueDiagnostics = ref<EmbeddingQueueDiagnostics | null>(null);
let embedQueuePollHandle: ReturnType<typeof setInterval> | null = null;
let embedQueueDebugPollHandle: ReturnType<typeof setInterval> | null = null;

async function refreshEmbedQueueStatus() {
  try {
    embedQueueStatus.value = await invoke<EmbeddingQueueStatus>('embedding_queue_status');
  } catch (e) {
    // Silent — backend may not be ready, or we hit a brief lock contention.
    void e;
  }
}

async function refreshEmbedQueueDiagnostics() {
  try {
    embedQueueDiagnostics.value = await invoke<EmbeddingQueueDiagnostics>(
      'embedding_queue_diagnostics',
    );
  } catch (e) {
    void e;
  }
}

function toggleEmbedQueueDebug() {
  embedQueueDebugOpen.value = !embedQueueDebugOpen.value;
  if (embedQueueDebugOpen.value) {
    void refreshEmbedQueueDiagnostics();
    if (embedQueueDebugPollHandle === null) {
      embedQueueDebugPollHandle = setInterval(refreshEmbedQueueDiagnostics, 5_000);
    }
  } else if (embedQueueDebugPollHandle !== null) {
    clearInterval(embedQueueDebugPollHandle);
    embedQueueDebugPollHandle = null;
  }
}

function formatRetryEta(retryAtMs: number, nowMs: number): string {
  const deltaSec = Math.round((retryAtMs - nowMs) / 1000);
  if (deltaSec <= 0) return 'due now';
  if (deltaSec < 60) return `in ${deltaSec}s`;
  if (deltaSec < 3600) return `in ${Math.round(deltaSec / 60)}m`;
  return `in ${Math.round(deltaSec / 3600)}h`;
}

// ── Hero text ──────────────────────────────────────────────────────────────

const moodKey = computed<'none' | 'free' | 'paid' | 'local'>(() => {
  const m = brain.brainMode;
  if (!m) return 'none';
  if (m.mode === 'free_api') return 'free';
  if (m.mode === 'paid_api') return 'paid';
  if (m.mode === 'local_ollama' || m.mode === 'local_lm_studio') return 'local';
  return 'none';
});

const heroTitle = computed(() => {
  switch (moodKey.value) {
    case 'free': return 'Your brain is alive ☁️';
    case 'paid': return 'Your brain is alive 💎';
    case 'local': return 'Your brain is alive 🖥';
    default: return 'No brain configured yet';
  }
});

const heroSubtitle = computed(() => {
  if (!brain.brainMode) return 'Connect a brain to start having conversations.';
  const provider = providerName.value;
  return provider
    ? `Powered by ${provider}. ${memoryCount.value} memories shape every reply.`
    : `${memoryCount.value} memories shape every reply.`;
});

const localProviderLabel = computed(() => {
  const m = brain.brainMode;
  if (!m) return '';
  if (m.mode === 'local_ollama') return 'Ollama';
  if (m.mode === 'local_lm_studio') return 'LM Studio';
  return '';
});

const moodPillLabel = computed(() => ({
  none: '⚠ No brain',
  free: '☁️ Free cloud',
  paid: '💎 Paid cloud',
  local: '🖥 Local LLM',
}[moodKey.value]));

// heroExpression was used by the legacy BrainAvatar; the new BrainOrb derives
// its state directly from brain.brainMode so we no longer compute it.

// ── Cognitive kind breakdown (docs §3.5) ───────────────────────────────────

const cognitiveKinds = computed(() => summariseCognitiveKinds(memory.memories ?? []));

const cognitiveRows = computed(() => {
  const total = cognitiveKinds.value.total || 1; // avoid div-by-zero
  const pct = (n: number) => Math.round((n / total) * 100);
  return [
    {
      key: 'episodic' as const,
      label: 'Episodic',
      emoji: '📅',
      count: cognitiveKinds.value.episodic,
      percent: pct(cognitiveKinds.value.episodic),
      description: 'Time- and place-anchored experiences (decays fastest)',
    },
    {
      key: 'semantic' as const,
      label: 'Semantic',
      emoji: '📚',
      count: cognitiveKinds.value.semantic,
      percent: pct(cognitiveKinds.value.semantic),
      description: 'Stable knowledge & preferences (decays slowest)',
    },
    {
      key: 'procedural' as const,
      label: 'Procedural',
      emoji: '🛠',
      count: cognitiveKinds.value.procedural,
      percent: pct(cognitiveKinds.value.procedural),
      description: 'How-to knowledge & repeatable workflows',
    },
    {
      key: 'judgment' as const,
      label: 'Judgment',
      emoji: '⚖',
      count: cognitiveKinds.value.judgment,
      percent: pct(cognitiveKinds.value.judgment),
      description: 'Persisted rules, heuristics & operating preferences',
    },
  ];
});

// ── RAG capability strip (docs §4 / §10) ───────────────────────────────────
//
// Mirrors the per-mode capability table in `docs/brain-advanced-design.md`
// § "Brain Modes & Provider Architecture". Only Local Ollama can compute
// embeddings (via `nomic-embed-text`), so vector search (40% of the hybrid
// score) is unavailable in the cloud modes.

interface RagSignal {
  key: string;
  label: string;
  weight: string;
  available: boolean;
  unavailableReason: string;
}

const ragSignals = computed<RagSignal[]>(() => {
  const isLocal = moodKey.value === 'local';
  const isOnline = moodKey.value !== 'none';
  return [
    {
      key: 'vector', label: 'Vector', weight: '40%',
      available: isLocal,
      unavailableReason: 'Switch to Local Ollama or Local LM Studio to enable embeddings',
    },
    {
      key: 'keyword', label: 'Keyword', weight: '20%',
      available: isOnline, unavailableReason: 'Configure a brain first',
    },
    {
      key: 'recency', label: 'Recency', weight: '15%',
      available: isOnline, unavailableReason: 'Configure a brain first',
    },
    {
      key: 'importance', label: 'Importance', weight: '10%',
      available: isOnline, unavailableReason: 'Configure a brain first',
    },
    {
      key: 'decay', label: 'Decay', weight: '10%',
      available: isOnline, unavailableReason: 'Configure a brain first',
    },
    {
      key: 'tier', label: 'Tier', weight: '5%',
      available: isOnline, unavailableReason: 'Configure a brain first',
    },
  ];
});

const ragQuality = computed(() => {
  if (moodKey.value === 'none') {
    return { effective: 0, note: 'No brain configured.' };
  }
  if (moodKey.value === 'local') {
    return { effective: 100, note: 'Full hybrid search with vector embeddings.' };
  }
  return {
    effective: 60,
    note: 'Cloud APIs cannot compute embeddings — vector signal is offline.',
  };
});



const memoryStats = computed(() => memory.stats ?? {
  total: 0, short_count: 0, working_count: 0, long_count: 0,
  total_tokens: 0, avg_decay: 1.0,
});
const memoryCount = computed(() => memoryStats.value.total);
const edgeCount = computed(() => memory.edgeStats?.total_edges ?? memory.edges?.length ?? 0);

// ── Configuration card ────────────────────────────────────────────────────

const providerName = computed(() => {
  const m = brain.brainMode;
  if (!m) return null;
  if (m.mode === 'free_api') {
    const p = brain.freeProviders.find(fp => fp.id === m.provider_id);
    return p?.display_name ?? m.provider_id;
  }
  if (m.mode === 'paid_api') return m.base_url;
  if (m.mode === 'local_ollama') return 'Ollama (Local LLM)';
  if (m.mode === 'local_lm_studio') return 'LM Studio (Local LLM)';
  return null;
});

const configRows = computed(() => {
  const m = brain.brainMode;
  if (!m) {
    return { mode: 'Not configured', provider: '—', model: '—', endpoint: '' };
  }
  if (m.mode === 'free_api') {
    const p = brain.freeProviders.find(fp => fp.id === m.provider_id);
    return {
      mode: 'Free Cloud API',
      provider: p?.display_name ?? m.provider_id,
      model: p?.model ?? '—',
      endpoint: p?.base_url ?? '',
    };
  }
  if (m.mode === 'paid_api') {
    return {
      mode: 'Paid Cloud API',
      provider: 'Custom OpenAI-compatible',
      model: m.model,
      endpoint: m.base_url,
    };
  }
  if (m.mode === 'local_ollama') {
    return {
      mode: 'Local LLM',
      provider: 'Ollama',
      model: m.model,
      endpoint: 'http://localhost:11434',
    };
  }
  if (m.mode === 'local_lm_studio') {
    return {
      mode: 'Local LLM',
      provider: 'LM Studio',
      model: m.model,
      endpoint: m.base_url,
      embeddingModel: m.embedding_model ?? undefined,
    };
  }
  return { mode: 'Unknown', provider: '—', model: '—', endpoint: '' };
});

function shortUrl(url: string): string {
  try {
    const u = new URL(url);
    return u.host + (u.pathname === '/' ? '' : u.pathname);
  } catch {
    return url;
  }
}

// ── Hardware card ──────────────────────────────────────────────────────────

const hardwareRows = computed(() => {
  const sys = brain.systemInfo;
  if (!sys) return { os: 'Loading…', cpu: 'Loading…', ram: 'Loading…', gpu: '' };
  return {
    os: `${sys.os_name || 'Unknown'} (${sys.arch || '?'})`,
    cpu: `${sys.cpu_name || 'Unknown'} · ${sys.cpu_cores || 0} cores`,
    ram: formatRam(sys.total_ram_mb || 0) + (sys.ram_tier_label ? ` · ${sys.ram_tier_label}` : ''),
    gpu: sys.gpu_name || '',
  };
});

const ramTier = computed(() => {
  const sys = brain.systemInfo;
  if (!sys || !sys.total_ram_mb) return null;
  const gb = sys.total_ram_mb / 1024;
  // 4 → 8 → 16 → 32+ GB tiers map to 25/50/75/100% with colors.
  let percent = Math.min(100, (gb / 32) * 100);
  let color: string;
  if (gb >= 32) { percent = 100; color = 'var(--ts-success)'; }
  else if (gb >= 16) color = 'var(--ts-accent-blue)';
  else if (gb >= 8) color = 'var(--ts-warning)';
  else color = 'var(--ts-error)';
  return { percent, color, label: sys.ram_tier_label || '' };
});

// ── Quick mode switcher ────────────────────────────────────────────────────

interface ModeOption {
  key: 'free' | 'paid' | 'local';
  label: string;
  emoji: string;
  detail: string;
  description: string;
  disabled: boolean;
  disabledReason: string;
  action: () => void | Promise<void>;
}

const localLlmDetail = computed(() => {
  const ollamaUp = brain.ollamaStatus.running;
  const lmUp = brain.lmStudioStatus?.running;
  if (moodKey.value === 'local') {
    // Show which provider is active
    return `${localProviderLabel.value} · active`;
  }
  if (ollamaUp && lmUp) return 'Ollama + LM Studio available';
  if (ollamaUp) return `Ollama · ${brain.installedModels.length} model${brain.installedModels.length === 1 ? '' : 's'} ready`;
  if (lmUp) return `LM Studio · ${(brain.lmStudioModels ?? []).length} model${(brain.lmStudioModels ?? []).length === 1 ? '' : 's'} available`;
  return 'No local provider running';
});

const modeOptions = computed<ModeOption[]>(() => [
  {
    key: 'free',
    label: 'Free cloud',
    emoji: '☁️',
    detail: 'OpenRouter / Gemini / Pollinations',
    description: 'Open the wizard to connect a free-tier provider key or token',
    disabled: false,
    disabledReason: '',
    action: () => switchToFree(),
  },
  {
    key: 'paid',
    label: 'Paid cloud',
    emoji: '💎',
    detail: 'OpenAI / Anthropic · best quality',
    description: 'Open the wizard to configure a paid OpenAI-compatible provider',
    disabled: false,
    disabledReason: '',
    action: () => emitNavigate('brain-setup'),
  },
  {
    key: 'local',
    label: 'Local LLM',
    emoji: '🖥',
    detail: localLlmDetail.value,
    description: 'Configure a local LLM provider (Ollama, LM Studio, or more)',
    disabled: !brain.ollamaStatus.running && !brain.lmStudioStatus?.running,
    disabledReason: 'No local provider running — start Ollama or LM Studio',
    action: () => emitNavigate('marketplace'),
  },
]);

// Re-export emit so module functions can call it. (kept simple — emitNavigate above wraps it.)
defineExpose({});

function switchToFree() {
  emitNavigate('brain-setup');
}

// ── Coding LLM picker (Phase 25 — Self-Improve foundation) ────────────────
const selfImprove = useSelfImproveStore();
const selectedCodingRecKey = ref<string | null>(null);
const codingModelInput = ref('');
const codingBaseUrlInput = ref('');
const codingApiKeyInput = ref('');
const codingTestInFlight = ref(false);
const localCodingModels = ref<string[]>([]);
const loadingLocalModels = ref(false);

function recKey(rec: { provider: string; display_name: string }): string {
  return `${rec.provider}::${rec.display_name}`;
}

const selectedCodingRec = computed(() =>
  (selfImprove.recommendations ?? []).find(
    (r) => recKey(r) === selectedCodingRecKey.value,
  ) ?? null,
);

/**
 * True when the selected recommendation is the Local Ollama preset —
 * recognised by `requires_api_key === false` plus an `127.0.0.1` /
 * `localhost` base URL. Drives the model-dropdown UI.
 */
const isLocalOllamaSelection = computed(() => {
  const rec = selectedCodingRec.value;
  if (!rec) return false;
  if (rec.requires_api_key) return false;
  const url = (codingBaseUrlInput.value || rec.base_url || '').toLowerCase();
  return url.includes('127.0.0.1') || url.includes('localhost');
});

async function refreshLocalCodingModels() {
  loadingLocalModels.value = true;
  try {
    const url = codingBaseUrlInput.value || selectedCodingRec.value?.base_url;
    localCodingModels.value = await selfImprove.loadLocalCodingModels(url);
    // Auto-select the first installed model if the user hasn't typed one
    // and the recommendation default isn't actually installed.
    if (
      isLocalOllamaSelection.value &&
      localCodingModels.value.length > 0 &&
      !localCodingModels.value.includes(codingModelInput.value)
    ) {
      const recDefault = selectedCodingRec.value?.default_model || '';
      codingModelInput.value =
        localCodingModels.value.find((m) => m === recDefault) ??
        localCodingModels.value[0];
    }
  } finally {
    loadingLocalModels.value = false;
  }
}

watch(selectedCodingRecKey, async (key) => {
  if (!key) return;
  const rec = (selfImprove.recommendations ?? []).find((r) => recKey(r) === key);
  if (!rec) return;
  // Replace defaults from the new recommendation. We *do* overwrite here
  // because the user just expressed intent by clicking a different card.
  codingModelInput.value = rec.default_model;
  codingBaseUrlInput.value = rec.base_url;
  if (!rec.requires_api_key) {
    codingApiKeyInput.value = '';
  }
  if (isLocalOllamaSelection.value) {
    await refreshLocalCodingModels();
  } else {
    localCodingModels.value = [];
  }
});

async function saveCodingLlm() {
  if (!selectedCodingRec.value) return;
  const rec = selectedCodingRec.value;
  const model = codingModelInput.value || rec.default_model;
  const baseUrl = codingBaseUrlInput.value || rec.base_url;
  if (!model || !baseUrl) return;
  if (rec.requires_api_key && !codingApiKeyInput.value) return;
  try {
    await selfImprove.setCodingLlm({
      provider: rec.provider,
      model,
      base_url: baseUrl,
      // Empty string when the recommendation does not require auth
      // (local Ollama). The Rust client skips the bearer header when
      // this is empty.
      api_key: codingApiKeyInput.value,
    });
    if (rec.requires_api_key) {
      codingApiKeyInput.value = ''; // never linger in the input
    }
  } catch (err) {
    console.warn('[BrainView] save coding LLM failed:', err);
  }
}

async function clearCodingLlm() {
  try {
    await selfImprove.setCodingLlm(null);
    selectedCodingRecKey.value = null;
    codingModelInput.value = '';
    codingBaseUrlInput.value = '';
    codingApiKeyInput.value = '';
    localCodingModels.value = [];
  } catch (err) {
    console.warn('[BrainView] clear coding LLM failed:', err);
  }
}

async function testCodingLlm() {
  codingTestInFlight.value = true;
  try {
    await selfImprove.testCodingLlmConnection();
  } catch (err) {
    console.warn('[BrainView] test coding LLM failed:', err);
  } finally {
    codingTestInFlight.value = false;
  }
}

// Pre-select the persisted provider (if any) when the picker first loads.
watch(
  () => selfImprove.codingLlm,
  (cfg) => {
    if (cfg && !selectedCodingRecKey.value) {
      // Find the matching recommendation by provider + base_url so the
      // local-Ollama vs custom-OpenAI-compatible cards (which share the
      // `Custom` provider) are disambiguated correctly.
      const recs = selfImprove.recommendations ?? [];
      const match =
        recs.find((r) => r.provider === cfg.provider && r.base_url === cfg.base_url) ??
        recs.find((r) => r.provider === cfg.provider);
      if (match) {
        selectedCodingRecKey.value = recKey(match);
      }
      codingModelInput.value = cfg.model;
      codingBaseUrlInput.value = cfg.base_url;
    }
  },
  { immediate: true },
);

// ── Refresh ────────────────────────────────────────────────────────────────

const conversation = useConversationStore();
const aiDecisionPolicyStore = useAiDecisionPolicyStore();
const aiDecisionPolicy = aiDecisionPolicyStore.policy;

interface DecisionToggleRow {
  key: keyof AiDecisionPolicy;
  label: string;
  description: string;
  testid: string;
}
const decisionToggleRows: DecisionToggleRow[] = [
  {
    key: 'intentClassifierEnabled',
    label: 'LLM-powered intent classifier',
    description:
      'Run every chat turn through the brain to detect learn-with-docs / teach-ingest / gated-setup intents. Off = every message goes straight to streaming chat.',
    testid: 'bv-aidp-intent',
  },
  {
    key: 'dontKnowGateEnabled',
    label: 'Offer Gemini-search / context upload after "I don\'t know"',
    description:
      'Watch assistant replies for hedging language and push a System message offering the upgrade paths. Off = no follow-up prompt.',
    testid: 'bv-aidp-dontknow',
  },
  {
    key: 'questSuggestionsEnabled',
    label: 'Auto-suggest quests after replies',
    description:
      'Open a quest overlay when the reply or your message mentions getting-started keywords. Off = quests only launch from the Skill Tree.',
    testid: 'bv-aidp-quest',
  },
  {
    key: 'chatBasedLlmSwitchEnabled',
    label: 'Chat-based LLM switching commands',
    description:
      'Recognise "switch to groq", "use my openai api key sk-…" etc. and reconfigure the brain in-place. Off = those messages reach the LLM unchanged.',
    testid: 'bv-aidp-llm-switch',
  },
  {
    key: 'quickRepliesEnabled',
    label: 'Yes/No quick-reply suggestions',
    description:
      'Show one-tap "Yes / No" buttons under the latest reply when it ends with a yes/no question pattern. Off = always type your full reply.',
    testid: 'bv-aidp-quick-replies',
  },
  {
    key: 'capacityDetectionEnabled',
    label: 'Auto-suggest model upgrade when struggling',
    description:
      'Watch free-API replies for "I can\'t / cannot / am only an AI / beyond my capabilities" phrasings; after a few low-quality replies, pop the upgrade dialog. Off = no auto-prompts.',
    testid: 'bv-aidp-capacity',
  },
];

function onToggleDecision(key: keyof AiDecisionPolicy, enabled: boolean): void {
  aiDecisionPolicy[key] = enabled;
}

function resetDecisionPolicy(): void {
  aiDecisionPolicyStore.reset();
}

// Active selection snapshot (docs §20)
interface BrainSelectionSnapshot {
  provider: { kind: string; configured_provider_id?: string; effective_provider_id?: string;
    rotator_healthy?: boolean; provider?: string; model?: string; base_url?: string };
  embedding: { available: boolean; preferred_model: string; unavailable_reason: string | null };
  memory: { total: number; short_count: number; working_count: number; long_count: number;
    embedded_count: number; schema_version: number };
  search: { default_method: string; top_k: number; relevance_threshold: number | null };
  storage: { backend: string; is_local: boolean; schema_label: string };
  agents: { registered: string[]; default_agent_id: string };
  rag_quality_percent: number;
  rag_quality_note: string;
}
const brainSelection = ref<BrainSelectionSnapshot | null>(null);

const selectionProviderLine = computed(() => {
  const p = brainSelection.value?.provider;
  if (!p) return '—';
  switch (p.kind) {
    case 'none': return 'Not configured';
    case 'free_api': {
      const same = p.configured_provider_id === p.effective_provider_id;
      const health = p.rotator_healthy ? 'healthy' : 'falling back';
      return same
        ? `Free API → ${p.effective_provider_id} (${health})`
        : `Free API → ${p.effective_provider_id} (rotated from ${p.configured_provider_id}, ${health})`;
    }
    case 'paid_api': return `Paid API → ${p.provider} · ${p.model} @ ${p.base_url}`;
    case 'local_ollama': return `Local Ollama → ${p.model}`;
    case 'local_lm_studio': return `Local LM Studio → ${p.model} @ ${p.base_url}`;
    default: return p.kind;
  }
});
const selectionEmbeddingLine = computed(() => {
  const e = brainSelection.value?.embedding;
  if (!e) return '—';
  return e.available ? `✓ ${e.preferred_model}` : `✗ unavailable — ${e.unavailable_reason ?? ''}`;
});
const selectionSearchLine = computed(() => {
  const s = brainSelection.value?.search;
  if (!s) return '—';
  const thr = s.relevance_threshold == null ? 'no threshold' : `score ≥ ${s.relevance_threshold}`;
  return `${s.default_method} · top-${s.top_k} · ${thr}`;
});
const selectionStorageLine = computed(() => {
  const s = brainSelection.value?.storage;
  if (!s) return '—';
  return `${s.backend} (${s.is_local ? 'local' : 'remote'}) · ${s.schema_label}`;
});
const selectionAgentsLine = computed(() => {
  const a = brainSelection.value?.agents;
  if (!a) return '—';
  return `${a.registered.length} registered · default = "auto" → ${a.default_agent_id}`;
});

// Daily-learning policy (docs §21)
interface AutoLearnPolicy {
  enabled: boolean;
  every_n_turns: number;
  min_cooldown_turns: number;
}
const autoLearnPolicy = ref<AutoLearnPolicy | null>(null);

const autoLearnSessionLine = computed(() => {
  const turns = conversation.totalAssistantTurns;
  const last = conversation.lastAutoLearnTurn;
  const saved = conversation.lastAutoLearnSavedCount;
  const lastNote = last == null
    ? 'has not auto-learned yet'
    : `last auto-learn at turn ${last} (saved ${saved})`;
  return `${turns} assistant ${turns === 1 ? 'turn' : 'turns'} · ${lastNote}`;
});
const autoLearnStatusLine = computed(() => {
  const d = conversation.lastAutoLearnDecision;
  if (!d) return 'idle (waiting for first turn)';
  if (d.should_fire) return 'firing now…';
  if (d.reason === 'disabled') return 'disabled — toggle on to enable';
  if (d.reason === 'below_threshold') return `next auto-learn in ${d.turns_remaining} ${d.turns_remaining === 1 ? 'turn' : 'turns'}`;
  if (d.reason === 'cooldown') return `cooling down (${d.turns_remaining} ${d.turns_remaining === 1 ? 'turn' : 'turns'} left)`;
  return d.reason;
});

async function loadBrainSelection() {
  try {
    brainSelection.value = await invoke<BrainSelectionSnapshot>('get_brain_selection');
  } catch (err) {
    console.warn('[BrainView] get_brain_selection failed:', err);
    brainSelection.value = null;
  }
}
async function loadAutoLearnPolicy() {
  try {
    autoLearnPolicy.value = await invoke<AutoLearnPolicy>('get_auto_learn_policy');
  } catch (err) {
    console.warn('[BrainView] get_auto_learn_policy failed:', err);
    autoLearnPolicy.value = null;
  }
}
async function onToggleAutoLearn(enabled: boolean) {
  if (!autoLearnPolicy.value) return;
  const next = { ...autoLearnPolicy.value, enabled };
  try {
    await invoke('set_auto_learn_policy', { policy: next });
    autoLearnPolicy.value = next;
  } catch (err) {
    console.warn('[BrainView] set_auto_learn_policy failed; reverting UI:', err);
    // Revert on failure — keep UI in sync with persisted state.
    // loadAutoLearnPolicy logs its own errors if the revert read also fails.
    await loadAutoLearnPolicy();
  }
}

async function onToggleAutoTag(enabled: boolean) {
  try {
    await appSettings.saveSettings({ auto_tag: enabled });
  } catch (err) {
    console.warn('[BrainView] save auto_tag failed; reverting UI:', err);
    await appSettings.loadSettings();
  }
}

async function onTogglePreferLocal(enabled: boolean) {
  try {
    await appSettings.saveSettings({ prefer_local_brain: enabled });
  } catch (err) {
    console.warn('[BrainView] save prefer_local_brain failed; reverting UI:', err);
    await appSettings.loadSettings();
  }
}

async function onToggleLanEnabled(enabled: boolean) {
  try {
    await appSettings.saveSettings({ lan_enabled: enabled });
  } catch (err) {
    console.warn('[BrainView] save lan_enabled failed; reverting UI:', err);
    await appSettings.loadSettings();
  }
}

async function forceExtractNow() {
  try {
    const count = await invoke<number>('extract_memories_from_session');
    conversation.lastAutoLearnSavedCount = count;
    conversation.lastAutoLearnTurn = conversation.totalAssistantTurns;
    await memory.fetchAll();
    await memory.getStats();
  } catch (err) {
    console.warn('[BrainView] force extract_memories_from_session failed:', err);
  }
}

// ── Replay-from-history backfill (Chunk 26.4) ──────────────────────────────
interface ReplayProgressEvent {
  processed: number;
  total: number;
  new_memories: number;
  current_summary_created_at: number | null;
  current_summary_id: number | null;
  done: boolean;
}

const replayState = ref({
  running: false,
  processed: 0,
  total: 0,
  newMemories: 0,
  lastResultLine: '',
});

let replayUnlisten: (() => void) | null = null;

async function replayHistoryNow() {
  if (replayState.value.running) return;
  replayState.value.running = true;
  replayState.value.processed = 0;
  replayState.value.total = 0;
  replayState.value.newMemories = 0;
  replayState.value.lastResultLine = '';

  if (!replayUnlisten) {
    replayUnlisten = await listen<ReplayProgressEvent>(
      'brain-replay-progress',
      (e) => {
        const p = e.payload;
        replayState.value.processed = p.processed;
        replayState.value.total = p.total;
        replayState.value.newMemories = p.new_memories;
      },
    );
  }

  try {
    const final = await invoke<ReplayProgressEvent>('replay_extract_history', {
      sinceTimestampMs: null,
      dryRun: false,
      maxSummaries: null,
    });
    replayState.value.lastResultLine = final.total === 0
      ? 'No session summaries to replay.'
      : `Replayed ${final.processed} session summaries · ${final.new_memories} new memories saved.`;
    await memory.fetchAll();
    await memory.getStats();
  } catch (err) {
    console.warn('[BrainView] replay_extract_history failed:', err);
    replayState.value.lastResultLine = `Replay failed: ${String(err)}`;
  } finally {
    replayState.value.running = false;
  }
}

async function confirmFactoryReset() {
  if (!confirm('Factory reset?\n\nThis will remove all auto-configured components (brain, voice, quests), erase all memories, and clear conversation history.\n\nYou will see the first-launch wizard again.\n\nThis cannot be undone.')) {
    return;
  }
  try {
    await brain.factoryReset();
    await refresh();
  } catch (err) {
    console.warn('[BrainView] factory reset failed:', err);
  }
}

async function confirmClearAllData() {
  if (!confirm('Clean ALL data?\n\nThis will permanently erase everything:\n• All memories, connections, and version history\n• Brain configuration and provider settings\n• Voice settings\n• Persona traits, expressions, and motions\n• Quest progress\n• App preferences\n\nOnly device identity is preserved.\n\nThis cannot be undone.')) {
    return;
  }
  try {
    await memory.clearAllData();
    await refresh();
    await appSettings.loadSettings();
  } catch (err) {
    console.warn('[BrainView] clean all data failed:', err);
  }
}

async function refresh() {
  isRefreshing.value = true;
  try {
    await Promise.allSettled([
      brain.loadBrainMode(),
      brain.fetchFreeProviders(),
      brain.fetchSystemInfo(),
      brain.checkOllamaStatus(),
      brain.fetchInstalledModels(),
      brain.checkLmStudioStatus(),
      brain.fetchLmStudioModels(),
      brain.refreshModelCatalogue().catch(() => brain.fetchRecommendations()),
      memory.fetchAll(),
      memory.getStats(),
      memory.fetchEdges(),
      memory.getEdgeStats(),
      loadBrainSelection(),
      loadAutoLearnPolicy(),
      appSettings.loadSettings(),
      refreshContextFolders(),
    ]);
  } finally {
    isRefreshing.value = false;
  }
}

// ── Context Folders ──────────────────────────────────────────────────────
type ContextFolderEntry = { path: string; label: string; enabled: boolean; last_synced_at: number; last_file_count: number };
type SyncResult = { folders_synced: number; files_ingested: number; errors: string[] };

const contextFolderInput = ref('');
const contextFolders = ref<ContextFolderEntry[]>([]);
const isSyncing = ref(false);
const syncResult = ref<SyncResult | null>(null);

async function refreshContextFolders() {
  try {
    contextFolders.value = await invoke<ContextFolderEntry[]>('list_context_folders');
  } catch { /* Tauri unavailable */ }
}

async function addContextFolder() {
  const path = contextFolderInput.value.trim();
  if (!path) return;
  try {
    await invoke('add_context_folder', { folderPath: path });
    contextFolderInput.value = '';
    await refreshContextFolders();
  } catch (err) {
    console.warn('[BrainView] add context folder failed:', err);
    alert(String(err));
  }
}

async function removeFolder(path: string) {
  try {
    await invoke('remove_context_folder', { folderPath: path });
    await refreshContextFolders();
  } catch (err) {
    console.warn('[BrainView] remove context folder failed:', err);
  }
}

async function toggleFolder(path: string, enabled: boolean) {
  try {
    await invoke('toggle_context_folder', { folderPath: path, enabled });
    await refreshContextFolders();
  } catch (err) {
    console.warn('[BrainView] toggle context folder failed:', err);
  }
}

async function syncAllContextFolders() {
  isSyncing.value = true;
  syncResult.value = null;
  try {
    syncResult.value = await invoke<SyncResult>('sync_context_folders');
    await refreshContextFolders();
  } catch (err) {
    console.warn('[BrainView] sync context folders failed:', err);
    alert(String(err));
  } finally {
    isSyncing.value = false;
  }
}

function formatDate(ms: number): string {
  if (!ms) return 'Never';
  return new Date(ms).toLocaleString();
}

// ── Context ↔ Knowledge Conversion ──────────────────────────────────────
type ContextMemoryInfo = { total_memories: number; total_tokens: number; by_folder: { label: string; count: number }[] };
type ConversionResult = { source_chunks: number; knowledge_entries_created: number; summary: string };
type ExportResult = { files_written: number; output_dir: string };

const contextMemoryInfo = ref<ContextMemoryInfo | null>(null);
const isConverting = ref(false);
const conversionResult = ref<ConversionResult | null>(null);
const exportFolderInput = ref('');
const isExporting = ref(false);
const exportResult = ref<ExportResult | null>(null);

async function refreshContextMemoryInfo() {
  try {
    contextMemoryInfo.value = await invoke<ContextMemoryInfo>('list_context_folder_memories');
  } catch { /* Tauri unavailable */ }
}

async function convertToKnowledge() {
  isConverting.value = true;
  conversionResult.value = null;
  try {
    conversionResult.value = await invoke<ConversionResult>('convert_context_to_knowledge', {});
    await refreshContextMemoryInfo();
  } catch (err) {
    console.warn('[BrainView] convert context failed:', err);
    alert(String(err));
  } finally {
    isConverting.value = false;
  }
}

async function exportKnowledge() {
  const dir = exportFolderInput.value.trim();
  if (!dir) return;
  isExporting.value = true;
  exportResult.value = null;
  try {
    exportResult.value = await invoke<ExportResult>('export_knowledge_to_folder', {
      outputDir: dir,
      tagFilter: 'context-folder',
      minImportance: 1,
    });
  } catch (err) {
    console.warn('[BrainView] export knowledge failed:', err);
    alert(String(err));
  } finally {
    isExporting.value = false;
  }
}

// ── Knowledge Graph ↔ Files ──────────────────────────────────────────
type FileToKgResult = { file_path: string; chunks_created: number; edges_created: number; root_id: number | null; summary: string };
type KgSubtreeExportResult = { root_ids: number[]; nodes_exported: number; edges_exported: number; files_written: number; output_dir: string };

const kgImportFilePath = ref('');
const isKgImporting = ref(false);
const kgImportResult = ref<FileToKgResult | null>(null);

const kgExportRootIds = ref('');
const kgExportDir = ref('');
const isKgExporting = ref(false);
const kgExportResult = ref<KgSubtreeExportResult | null>(null);

async function importFileToKg() {
  const fp = kgImportFilePath.value.trim();
  if (!fp) return;
  isKgImporting.value = true;
  kgImportResult.value = null;
  try {
    kgImportResult.value = await invoke<FileToKgResult>('import_file_to_knowledge_graph', {
      filePath: fp,
    });
    await refreshContextMemoryInfo();
  } catch (err) {
    console.warn('[BrainView] file-to-KG import failed:', err);
    alert(String(err));
  } finally {
    isKgImporting.value = false;
  }
}

async function exportKgSubtree() {
  const ids = kgExportRootIds.value
    .split(',')
    .map((s) => parseInt(s.trim(), 10))
    .filter((n) => !isNaN(n) && n > 0);
  const dir = kgExportDir.value.trim();
  if (ids.length === 0 || !dir) return;
  isKgExporting.value = true;
  kgExportResult.value = null;
  try {
    kgExportResult.value = await invoke<KgSubtreeExportResult>('export_kg_subtree', {
      rootIds: ids,
      outputDir: dir,
      maxHops: 2,
    });
  } catch (err) {
    console.warn('[BrainView] KG subtree export failed:', err);
    alert(String(err));
  } finally {
    isKgExporting.value = false;
  }
}

onMounted(async () => {
  await refresh();
  await selfImprove.initialise();
  await refreshEmbedQueueStatus();
  await refreshContextMemoryInfo();
  embedQueuePollHandle = setInterval(refreshEmbedQueueStatus, 5_000);
});

onUnmounted(() => {
  if (embedQueuePollHandle !== null) {
    clearInterval(embedQueuePollHandle);
    embedQueuePollHandle = null;
  }
  if (embedQueueDebugPollHandle !== null) {
    clearInterval(embedQueueDebugPollHandle);
    embedQueueDebugPollHandle = null;
  }
});
</script>

<style scoped>
.brain-view {
  /* Make the bp-shell behave as a scrollable view within app-main's
     flex column (which has overflow:hidden). Without this, BrainView's
     content gets clipped on both desktop and mobile because the cockpit
     hero + modules are taller than the viewport. We keep bp-shell's own
     padding/gap/max-width and only add the flex sizing + scroll. */
  flex: 1 1 auto;
  min-height: 0;
  height: auto;
  max-height: none;
  width: 100%;
  overflow-x: hidden;
  overflow-y: auto;
  overscroll-behavior: contain;
  scrollbar-gutter: stable;
}

/* ── Hero ───────────────────────────────────────────────────────────────── */
.bv-hero {
  display: grid;
  grid-template-columns: auto 1fr auto;
  gap: 1.25rem;
  align-items: center;
  padding: 1.25rem 1.5rem;
  background: var(--ts-quest-bg, linear-gradient(160deg, rgba(20, 18, 40, 0.85) 0%, rgba(12, 10, 28, 0.92) 100%));
  border: 1px solid var(--ts-border);
  border-radius: 12px;
}
.bv-hero-avatar {
  display: flex;
  align-items: center;
  justify-content: center;
}
.bv-hero-text { min-width: 0; }
.bv-hero-title {
  margin: 0 0 0.25rem;
  font-size: 1.5rem;
  color: var(--ts-text-primary);
}
.bv-hero-subtitle {
  margin: 0 0 0.75rem;
  color: var(--ts-text-secondary);
  font-size: 0.9rem;
}
.bv-hero-pills {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
}
.bv-pill {
  font-size: 0.75rem;
  padding: 0.2rem 0.6rem;
  border-radius: 999px;
  background: var(--ts-bg-input);
  color: var(--ts-text-secondary);
  border: 1px solid var(--ts-border);
}
.bv-pill-mood.bv-pill-free { background: rgba(123, 224, 179, 0.18); color: var(--ts-mode-free); border-color: rgba(123, 224, 179, 0.4); }
.bv-pill-mood.bv-pill-paid { background: rgba(124, 200, 255, 0.18); color: var(--ts-mode-paid); border-color: rgba(124, 200, 255, 0.4); }
.bv-pill-mood.bv-pill-local { background: rgba(200, 164, 255, 0.18); color: var(--ts-mode-local); border-color: rgba(200, 164, 255, 0.4); }
.bv-pill-mood.bv-pill-none { background: rgba(248, 113, 113, 0.18); color: var(--ts-error); border-color: rgba(248, 113, 113, 0.4); }

.bv-hero-actions {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

/* ── Quick mode switcher ────────────────────────────────────────────────── */
.bv-section-title {
  font-size: 0.75rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--ts-text-muted);
  margin-bottom: 0.5rem;
}
.bv-mode-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 0.5rem;
}

/* ── Coding LLM (Phase 25) ──────────────────────────────────────────── */
.bv-coding-llm {
  background: rgba(255, 255, 255, 0.02);
  border: 1px solid rgba(124, 111, 255, 0.18);
  border-radius: 12px;
  padding: 16px;
  margin: 1rem 0;
}
.bv-section-sub {
  font-size: 0.75rem;
  font-weight: 400;
  color: var(--ts-text-muted, #94a3b8);
  margin-left: 6px;
}
.bv-coding-llm-desc {
  margin: 0 0 12px;
  font-size: 0.85rem;
  color: var(--ts-text-muted, #94a3b8);
}
.bv-coding-llm-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 8px;
  margin-bottom: 14px;
}
.bv-coding-card {
  text-align: left;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 10px;
  padding: 12px;
  cursor: pointer;
  transition: border-color 0.15s, background 0.15s, transform 0.12s;
  color: inherit;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.bv-coding-card:hover { background: rgba(124, 111, 255, 0.08); transform: translateY(-1px); }
.bv-coding-card.active {
  border-color: rgba(124, 111, 255, 0.6);
  background: rgba(124, 111, 255, 0.12);
  box-shadow: 0 0 0 1px rgba(124, 111, 255, 0.3);
}
.bv-coding-card.top {
  border-color: rgba(251, 191, 36, 0.4);
}
.bv-coding-card-head { display: flex; justify-content: space-between; align-items: center; }
.bv-coding-badge {
  font-size: 0.68rem;
  font-weight: 700;
  background: rgba(251, 191, 36, 0.18);
  color: var(--ts-warning);
  padding: 2px 6px;
  border-radius: 999px;
  border: 1px solid rgba(251, 191, 36, 0.35);
}
.bv-coding-notes {
  margin: 0;
  font-size: 0.78rem;
  color: var(--ts-text-muted, #94a3b8);
  line-height: 1.4;
}
.bv-coding-card code {
  background: rgba(0, 0, 0, 0.25);
  padding: 1px 5px;
  border-radius: 4px;
  font-size: 0.74rem;
}
.bv-coding-form {
  display: flex;
  flex-direction: column;
  gap: 6px;
  background: rgba(0, 0, 0, 0.2);
  padding: 12px;
  border-radius: 10px;
  border: 1px solid rgba(255, 255, 255, 0.06);
}
.bv-coding-form label {
  font-size: 0.78rem;
  font-weight: 600;
  color: var(--ts-text-muted, #94a3b8);
  margin-top: 4px;
}
.bv-input {
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.1);
  color: var(--ts-text-primary, #eaecf4);
  border-radius: 6px;
  padding: 8px 10px;
  font-size: 0.85rem;
  font-family: inherit;
}
.bv-input:focus { outline: 2px solid var(--ts-accent, #7c6fff); outline-offset: 1px; border-color: transparent; }
.bv-coding-actions { display: flex; gap: 8px; margin-top: 10px; }
.bv-btn {
  border: 1px solid transparent;
  border-radius: 8px;
  padding: 8px 14px;
  font-size: 0.85rem;
  font-weight: 600;
  cursor: pointer;
}
.bv-btn:disabled { opacity: 0.5; cursor: not-allowed; }
.bv-btn--accent {
  background: var(--ts-accent);
  color: var(--ts-text-on-accent);
}
.bv-btn--accent:hover {
  background: var(--ts-accent-hover);
}
.bv-btn-primary {
  background: var(--ts-gradient-accent);
  color: white;
}
.bv-btn-ghost {
  background: rgba(255, 255, 255, 0.06);
  color: inherit;
  border-color: rgba(255, 255, 255, 0.1);
}
.bv-coding-status { margin: 8px 0 0; font-size: 0.82rem; }
.bv-coding-status--ok { color: var(--ts-success); }
.bv-coding-status--err { color: var(--ts-error); }
.bv-coding-detail { color: var(--ts-text-muted, #94a3b8); font-style: italic; margin-left: 4px; }
.bv-coding-hint {
  margin: 4px 0 8px;
  font-size: 0.78rem;
  color: var(--ts-text-secondary, #94a3b8);
}
.bv-coding-hint code {
  background: var(--ts-bg-input, rgba(255, 255, 255, 0.06));
  padding: 1px 6px;
  border-radius: 4px;
  color: var(--ts-text-bright, #e2e8f0);
}
.bv-local-model-row {
  display: flex;
  gap: 6px;
  align-items: stretch;
}
.bv-local-model-row .bv-input { flex: 1; }
.bv-mode-card {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.25rem;
  padding: 0.75rem 1rem;
  border-radius: 10px;
  background: var(--ts-bg-surface);
  border: 1px solid var(--ts-border);
  color: var(--ts-text-primary);
  cursor: pointer;
  text-align: left;
  transition: transform 0.15s ease, border-color 0.15s ease, background 0.15s ease;
}
.bv-mode-card:hover:not(:disabled) {
  transform: translateY(-2px);
  border-color: var(--ts-border-strong);
}
.bv-mode-card:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
.bv-mode-card.active.bv-mode-free { border-color: var(--ts-mode-free); background: rgba(123, 224, 179, 0.10); }
.bv-mode-card.active.bv-mode-paid { border-color: var(--ts-mode-paid); background: rgba(124, 200, 255, 0.10); }
.bv-mode-card.active.bv-mode-local { border-color: var(--ts-mode-local); background: rgba(200, 164, 255, 0.10); }
.bv-mode-emoji { font-size: 1.5rem; }
.bv-mode-label { font-weight: 700; }
.bv-mode-detail { font-size: 0.75rem; color: var(--ts-text-muted); }

/* ── Self-healing embedding queue strip ─────────────────────────────────── */
.bv-embed-queue {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding: 0.625rem 0.875rem;
  border-radius: 0.5rem;
  background: var(--ts-bg-card);
  border: 1px solid var(--ts-border);
  font-size: 0.8125rem;
  color: var(--ts-text-primary);
}
.bv-embed-queue--open {
  background: var(--ts-bg-overlay);
}
.bv-embed-queue__row {
  display: flex;
  align-items: center;
  gap: 0.625rem;
  flex-wrap: wrap;
}
.bv-embed-queue__icon {
  font-size: 1rem;
  animation: bv-embed-spin 2s linear infinite;
}
.bv-embed-queue__text { flex: 0 0 auto; }
.bv-embed-queue__hint {
  margin-left: auto;
  color: var(--ts-text-muted);
  font-size: 0.75rem;
}
.bv-embed-queue__warn { color: var(--ts-warning); font-weight: 600; }
.bv-embed-queue__toggle {
  appearance: none;
  background: transparent;
  border: 1px solid var(--ts-border);
  color: var(--ts-text-primary);
  font-size: 0.75rem;
  font-weight: 600;
  padding: 4px 10px;
  border-radius: 999px;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 4px;
}
.bv-embed-queue__toggle:hover {
  border-color: var(--ts-accent);
  color: var(--ts-accent);
}
.bv-embed-queue__toggle:focus-visible {
  outline: 2px solid var(--ts-accent);
  outline-offset: 2px;
}
.bv-embed-queue__debug {
  display: flex;
  flex-direction: column;
  gap: 0.625rem;
  padding-top: 0.5rem;
  border-top: 1px dashed var(--ts-border);
}
.bv-embed-queue__reason {
  margin: 0;
  font-size: 0.8125rem;
  color: var(--ts-text-primary);
  font-weight: 500;
}
.bv-embed-queue__facts {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 6px 12px;
  margin: 0;
  font-size: 0.75rem;
}
.bv-embed-queue__facts > div {
  display: flex;
  flex-direction: column;
  gap: 1px;
  padding: 6px 8px;
  background: var(--ts-bg-input);
  border-radius: 6px;
  border: 1px solid var(--ts-border);
}
.bv-embed-queue__facts dt {
  font-size: 0.68rem;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--ts-text-muted);
}
.bv-embed-queue__facts dd {
  margin: 0;
  font-weight: 600;
  color: var(--ts-text-primary);
}
.bv-embed-queue__failures-title {
  margin: 0;
  font-size: 0.78rem;
  font-weight: 700;
  color: var(--ts-text-primary);
}
.bv-embed-queue__failure-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
  max-height: 280px;
  overflow-y: auto;
}
.bv-embed-queue__failure {
  display: flex;
  flex-direction: column;
  gap: 3px;
  padding: 8px 10px;
  border-radius: 6px;
  background: var(--ts-bg-input);
  border: 1px solid var(--ts-border);
}
.bv-embed-queue__failure-head {
  display: flex;
  gap: 10px;
  font-size: 0.72rem;
  color: var(--ts-text-secondary);
}
.bv-embed-queue__failure-id {
  font-family: var(--ts-font-mono, monospace);
  color: var(--ts-accent);
  font-weight: 600;
}
.bv-embed-queue__failure-attempts {
  color: var(--ts-warning);
  font-weight: 600;
}
.bv-embed-queue__failure-eta { margin-left: auto; }
.bv-embed-queue__failure-error {
  font-family: var(--ts-font-mono, monospace);
  font-size: 0.72rem;
  color: var(--ts-error);
  word-break: break-word;
}
.bv-embed-queue__failure-preview {
  font-size: 0.72rem;
  color: var(--ts-text-muted);
  font-style: italic;
  word-break: break-word;
}
.bv-embed-queue__no-failures {
  margin: 0;
  font-size: 0.75rem;
  color: var(--ts-text-muted);
  font-style: italic;
}
@keyframes bv-embed-spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

/* ── Brain Capacity section ─────────────────────────────────────────────── */
.bv-capacity-section {
  margin: 0.5rem 0;
}

/* ── Context Folders section ────────────────────────────────────────────── */
.bv-context-folders-section {
  margin: 0.5rem 0;
  padding: var(--ts-space-md);
  background: var(--ts-bg-surface);
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-md);
}
.bv-section-title {
  margin: 0 0 var(--ts-space-sm);
  font-size: var(--ts-text-lg);
  color: var(--ts-text-primary);
}
.bv-context-warning {
  margin: 0 0 var(--ts-space-sm);
  padding: var(--ts-space-sm);
  font-size: var(--ts-text-sm);
  color: var(--ts-warning);
  background: rgba(251, 191, 36, 0.08);
  border: 1px solid rgba(251, 191, 36, 0.2);
  border-radius: var(--ts-radius-sm);
}
.bv-context-add {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ts-space-sm);
  margin-bottom: var(--ts-space-sm);
}
.bv-context-input {
  flex: 1;
  min-width: 0;
  padding: var(--ts-space-xs) var(--ts-space-sm);
  background: var(--ts-bg-input);
  border: 1px solid var(--ts-border-medium);
  border-radius: var(--ts-radius-sm);
  color: var(--ts-text-primary);
  font-size: var(--ts-text-sm);
}
.bv-context-input::placeholder {
  color: var(--ts-text-muted);
}
.bv-context-empty {
  padding: var(--ts-space-md);
  text-align: center;
  color: var(--ts-text-muted);
  font-size: var(--ts-text-sm);
}
.bv-context-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: var(--ts-space-xs);
}
.bv-context-item {
  display: flex;
  align-items: center;
  gap: var(--ts-space-sm);
  padding: var(--ts-space-xs) var(--ts-space-sm);
  background: var(--ts-bg-elevated);
  border: 1px solid var(--ts-border-subtle);
  border-radius: var(--ts-radius-sm);
}
.bv-context-item--disabled {
  opacity: 0.5;
}
.bv-context-toggle input {
  cursor: pointer;
}
.bv-context-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}
.bv-context-label {
  font-size: var(--ts-text-base);
  color: var(--ts-text-primary);
  font-weight: 500;
}
.bv-context-path {
  font-size: var(--ts-text-xs);
  color: var(--ts-text-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.bv-context-meta {
  font-size: var(--ts-text-xs);
  color: var(--ts-text-secondary);
}
.bv-context-result {
  margin-top: var(--ts-space-sm);
  padding: var(--ts-space-sm);
  font-size: var(--ts-text-sm);
  color: var(--ts-success);
  background: rgba(52, 211, 153, 0.08);
  border-radius: var(--ts-radius-sm);
}
.bv-context-errors {
  color: var(--ts-error);
}
.bv-btn--danger-sm {
  padding: 2px 6px;
  font-size: var(--ts-text-xs);
  background: transparent;
  color: var(--ts-error);
  border: 1px solid rgba(248, 113, 113, 0.3);
  border-radius: var(--ts-radius-sm);
  cursor: pointer;
}
.bv-btn--danger-sm:hover {
  background: rgba(248, 113, 113, 0.15);
}
.bv-btn--secondary {
  padding: var(--ts-space-xs) var(--ts-space-sm);
  font-size: var(--ts-text-sm);
  background: var(--ts-bg-elevated);
  color: var(--ts-text-secondary);
  border: 1px solid var(--ts-border-medium);
  border-radius: var(--ts-radius-sm);
  cursor: pointer;
}
.bv-btn--secondary:hover {
  background: var(--ts-bg-hover);
  color: var(--ts-text-primary);
}

/* Context ↔ Knowledge conversion */
.bv-context-conversion {
  margin-top: var(--ts-space-md);
  padding: var(--ts-space-sm);
  border: 1px dashed var(--ts-border-medium);
  border-radius: var(--ts-radius-sm);
}
.bv-context-conversion-title {
  margin: 0 0 var(--ts-space-xs);
  font-size: var(--ts-text-sm);
  color: var(--ts-text-primary);
}
.bv-context-conversion-hint {
  margin: 0 0 var(--ts-space-sm);
  font-size: var(--ts-text-xs);
  color: var(--ts-text-muted);
}
.bv-context-stats {
  margin-bottom: var(--ts-space-sm);
  font-size: var(--ts-text-xs);
  color: var(--ts-text-secondary);
}
.bv-context-conversion-actions {
  display: flex;
  flex-direction: column;
  gap: var(--ts-space-xs);
}
.bv-context-export-row {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ts-space-xs);
  align-items: center;
}
.bv-context-export-input {
  flex: 1;
}

/* Knowledge Graph ↔ Files */
.bv-kg-section {
  margin-top: var(--ts-space-md);
  padding: var(--ts-space-sm);
  border: 1px dashed var(--ts-border-medium);
  border-radius: var(--ts-radius-sm);
}
.bv-kg-import-row,
.bv-kg-export-row {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ts-space-xs);
  align-items: center;
  margin-bottom: var(--ts-space-xs);
}
.bv-kg-ids-input {
  max-width: 200px;
}

.bv-wiki-section {
  margin: 0.5rem 0;
}

/* ── Cards grid ─────────────────────────────────────────────────────────── */
.bv-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
  gap: 0.75rem;
}
.bv-card {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding: 0.85rem 1rem;
  background: var(--ts-bg-surface);
  border: 1px solid var(--ts-border);
  border-radius: 10px;
}
.bv-card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.bv-card-header h3 { margin: 0; font-size: 0.95rem; color: var(--ts-text-primary); }
.bv-card-link {
  background: none;
  border: none;
  color: var(--ts-accent-blue, #60a5fa);
  font-size: 0.75rem;
  cursor: pointer;
  padding: 0;
}
.bv-card-link:hover { text-decoration: underline; }
.bv-dl {
  margin: 0;
  display: grid;
  gap: 0.25rem;
  font-size: 0.85rem;
}
.bv-dl-row {
  display: flex;
  justify-content: space-between;
  gap: 0.5rem;
  padding: 0.15rem 0;
}
.bv-dl-row dt { color: var(--ts-text-muted); }
.bv-dl-row dd { margin: 0; color: var(--ts-text-primary); text-align: right; min-width: 0; }
.bv-model code, .bv-endpoint {
  font-family: var(--ts-font-mono, monospace);
  font-size: 0.75rem;
  color: var(--ts-text-secondary);
  word-break: break-all;
}

.bv-ram-bar {
  position: relative;
  height: 6px;
  background: var(--ts-bg-input);
  border-radius: 3px;
  overflow: hidden;
}
.bv-ram-fill { display: block; height: 100%; transition: width 0.3s ease; }

/* Memory tiers row */
.bv-memory-tiers { display: flex; gap: 0.4rem; }
.bv-mem-tier {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 0.4rem 0;
  border-radius: 6px;
  background: var(--ts-bg-input);
}
.bv-mem-tier.tier-short { color: var(--ts-warning); border: 1px solid var(--ts-warning-bg); }
.bv-mem-tier.tier-working { color: var(--ts-accent-blue); border: 1px solid var(--ts-accent-glow); }
.bv-mem-tier.tier-long { color: var(--ts-success); border: 1px solid var(--ts-success-bg); }
.bv-mem-num { font-size: 1.1rem; font-weight: 700; }
.bv-mem-label { font-size: 0.7rem; text-transform: uppercase; letter-spacing: 0.05em; opacity: 0.8; }

.bv-decay-bar {
  display: inline-block;
  width: 60px;
  height: 6px;
  background: var(--ts-bg-input);
  border-radius: 3px;
  overflow: hidden;
  vertical-align: middle;
  margin-right: 6px;
}
.bv-decay-fill { display: block; height: 100%; background: var(--ts-success); }
.bv-decay-num { font-size: 0.75rem; color: var(--ts-text-secondary); }

/* ── Stats section ─────────────────────────────────────────────────────── */
.bv-stats-section { /* BrainStatSheet brings its own styling. */ }

.bv-link {
  background: none;
  border: none;
  color: var(--ts-accent-blue, #60a5fa);
  cursor: pointer;
  padding: 0;
  font: inherit;
}
.bv-link:hover { text-decoration: underline; }

/* ── Cognitive-kind breakdown ──────────────────────────────────────────── */
.bv-cognitive { padding: 0.85rem 1rem; }
.bv-card-subtle {
  font-size: 0.75rem;
  color: var(--ts-text-muted);
}
.bv-cognitive-empty {
  padding: 1rem;
  color: var(--ts-text-muted);
  text-align: center;
  font-size: 0.85rem;
}
.bv-cognitive-bars {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 0.75rem;
}
.bv-cog-row {
  padding: 0.5rem 0.75rem;
  border-radius: 8px;
  background: var(--ts-bg-input);
  border: 1px solid var(--ts-border);
}
.bv-cog-head {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  margin-bottom: 0.3rem;
}
.bv-cog-emoji { font-size: 1.1rem; }
.bv-cog-name { flex: 1; font-weight: 600; color: var(--ts-text-primary); font-size: 0.85rem; }
.bv-cog-count { font-variant-numeric: tabular-nums; font-size: 0.85rem; color: var(--ts-text-secondary); }
.bv-cog-count small { color: var(--ts-text-muted); margin-left: 0.2rem; font-size: 0.75rem; }
.bv-cog-bar {
  height: 6px;
  background: var(--ts-bg-input);
  border-radius: 3px;
  overflow: hidden;
  margin-bottom: 0.3rem;
}
.bv-cog-bar-fill {
  height: 100%;
  border-radius: 3px;
  transition: width 0.4s ease;
}
.bv-cog-episodic .bv-cog-bar-fill { background: linear-gradient(90deg, var(--ts-warning), var(--ts-warning-text)); }
.bv-cog-semantic .bv-cog-bar-fill { background: linear-gradient(90deg, var(--ts-accent-blue-hover), var(--ts-accent-blue)); }
.bv-cog-procedural .bv-cog-bar-fill { background: linear-gradient(90deg, var(--ts-success-dim), var(--ts-success)); }
.bv-cog-judgment .bv-cog-bar-fill { background: linear-gradient(90deg, var(--ts-accent-violet-hover), var(--ts-accent-violet)); }
.bv-cog-desc { font-size: 0.7rem; color: var(--ts-text-muted); }

/* ── RAG capability strip ──────────────────────────────────────────────── */
.bv-rag { padding: 0.85rem 1rem; }
.bv-rag-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(110px, 1fr));
  gap: 0.4rem;
  margin: 0.4rem 0 0.6rem;
}
.bv-rag-cell {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.15rem;
  padding: 0.4rem 0.5rem;
  border-radius: 6px;
  border: 1px solid var(--ts-border);
  font-size: 0.8rem;
}
.bv-rag-cell.is-on {
  background: var(--ts-success-bg);
  border-color: rgba(52, 211, 153, 0.4);
  color: var(--ts-success);
}
.bv-rag-cell.is-off {
  background: var(--ts-bg-input);
  color: var(--ts-text-muted);
}
.bv-rag-icon { font-size: 1rem; font-weight: 700; }
.bv-rag-label { font-weight: 600; }
.bv-rag-weight { font-size: 0.7rem; opacity: 0.8; font-variant-numeric: tabular-nums; }
.bv-rag-summary {
  margin: 0.3rem 0 0;
  font-size: 0.8rem;
  color: var(--ts-text-secondary);
}
.bv-rag-summary strong { color: var(--ts-text-primary); }

.btn-primary {
  padding: 0.5rem 1rem;
  background: var(--ts-accent-blue-hover, #4f9eea);
  color: var(--ts-text-on-accent, #fff);
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-weight: 600;
}
.btn-primary:hover:not(:disabled) { background: var(--ts-accent-blue, #60a5fa); }
.btn-primary:disabled { opacity: 0.6; cursor: not-allowed; }
.btn-secondary {
  padding: 0.5rem 1rem;
  background: var(--ts-bg-elevated, rgba(255,255,255,0.06));
  color: var(--ts-text-primary, #e8eaee);
  border: 1px solid var(--ts-border, rgba(255,255,255,0.08));
  border-radius: 6px;
  cursor: pointer;
}
.btn-secondary:hover:not(:disabled) { background: var(--ts-bg-hover, rgba(255,255,255,0.10)); }

/* ── Danger zone ────────────────────────────────────────────────────────── */
.bv-danger-zone {
  border-color: var(--ts-error, #f87171);
  background: rgba(248, 113, 113, 0.04);
}
.bv-danger-zone .bv-card-header h3 { color: var(--ts-error, #f87171); }
.bv-danger-actions {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}
.bv-danger-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.5rem 0;
  border-top: 1px solid var(--ts-border);
}
.bv-danger-info {
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
  min-width: 0;
}
.bv-danger-label {
  font-weight: 600;
  font-size: 0.85rem;
  color: var(--ts-text-primary);
}
.btn-danger {
  flex: none;
  padding: 0.4rem 0.85rem;
  border: 1px solid var(--ts-error, #f87171);
  border-radius: 6px;
  background: transparent;
  color: var(--ts-error, #f87171);
  font-weight: 600;
  font-size: 0.8rem;
  cursor: pointer;
  white-space: nowrap;
  transition: background 0.15s ease, color 0.15s ease;
}
.btn-danger:hover {
  background: var(--ts-error, #f87171);
  color: var(--ts-text-on-accent);
}

@media (max-width: 720px) {
  .bv-hero { grid-template-columns: auto 1fr; }
  .bv-hero-actions {
    grid-column: 1 / -1;
    flex-direction: row;
    flex-wrap: wrap;
  }
  .bv-grid { grid-template-columns: 1fr 1fr; }
  .bv-cognitive-bars { grid-template-columns: 1fr; }
}
@media (max-width: 480px) {
  .bv-hero { grid-template-columns: 1fr; text-align: center; }
  .bv-hero-avatar { justify-self: center; }
  .bv-hero-pills { justify-content: center; }
  .bv-hero-actions { justify-content: center; }
  .bv-grid { grid-template-columns: 1fr; }
  .bv-mode-grid { grid-template-columns: 1fr 1fr; }
  .bv-rag-grid { grid-template-columns: repeat(auto-fit, minmax(90px, 1fr)); }
  .brain-view { padding: 0.75rem 0.5rem; gap: 0.5rem; }
  .bv-hero { padding: 0.85rem 0.75rem; gap: 0.75rem; }
  .bv-hero-title { font-size: 1.2rem; }
  .bv-section-title { font-size: 0.7rem; }
  .bv-context-input,
  .bv-context-export-input,
  .bv-kg-ids-input {
    flex-basis: 100%;
    max-width: none;
  }
  .bv-kg-import-row .bv-btn,
  .bv-kg-export-row .bv-btn,
  .bv-context-export-row .bv-btn {
    width: 100%;
    white-space: normal;
  }
}
</style>
