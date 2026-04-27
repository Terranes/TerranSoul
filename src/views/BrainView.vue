<template>
  <div
    class="brain-view"
    data-testid="brain-view"
  >
    <!-- ── Hero ────────────────────────────────────────────────────────────── -->
    <section class="bv-hero">
      <div class="bv-hero-avatar">
        <BrainAvatar
          :mode="brain.brainMode"
          :memory-count="memoryCount"
          :expression="heroExpression"
          :size="180"
        />
      </div>
      <div class="bv-hero-text">
        <h1 class="bv-hero-title">
          {{ heroTitle }}
        </h1>
        <p class="bv-hero-subtitle">
          {{ heroSubtitle }}
        </p>
        <div class="bv-hero-pills">
          <span
            class="bv-pill bv-pill-mood"
            :class="`bv-pill-${moodKey}`"
          >
            {{ moodPillLabel }}
          </span>
          <span
            v-if="memoryCount > 0"
            class="bv-pill bv-pill-memory"
          >
            🧠 {{ memoryCount }} memories
          </span>
          <span
            v-if="edgeCount > 0"
            class="bv-pill bv-pill-edges"
          >
            🔗 {{ edgeCount }} connections
          </span>
          <span
            v-if="brain.ollamaStatus.running || brain.lmStudioStatus?.running"
            class="bv-pill bv-pill-ollama"
          >
            🖥 {{ brain.ollamaStatus.running && brain.lmStudioStatus?.running ? 'Ollama + LM Studio' : brain.ollamaStatus.running ? 'Ollama running' : 'LM Studio running' }}
          </span>
        </div>
      </div>
      <div class="bv-hero-actions">
        <button
          class="btn-primary"
          @click="$emit('navigate', 'brain-setup')"
        >
          ⚙ Brain setup
        </button>
        <button
          class="btn-secondary"
          @click="$emit('navigate', 'marketplace')"
        >
          🏪 Switch model
        </button>
        <button
          class="btn-secondary"
          :disabled="isRefreshing"
          @click="refresh"
        >
          {{ isRefreshing ? '⟳ Refreshing…' : '↻ Refresh' }}
        </button>
      </div>
    </section>

    <!-- ── Quick mode switcher ─────────────────────────────────────────────── -->
    <section
      class="bv-mode-switcher"
      data-testid="bv-mode-switcher"
    >
      <div class="bv-section-title">
        ⚡ Quick mode
      </div>
      <div class="bv-mode-grid">
        <button
          v-for="opt in modeOptions"
          :key="opt.key"
          :class="['bv-mode-card', `bv-mode-${opt.key}`, { active: opt.key === moodKey }]"
          :disabled="opt.disabled"
          :title="opt.disabled ? opt.disabledReason : opt.description"
          @click="opt.disabled ? null : opt.action()"
        >
          <span class="bv-mode-emoji">{{ opt.emoji }}</span>
          <span class="bv-mode-label">{{ opt.label }}</span>
          <span class="bv-mode-detail">{{ opt.detail }}</span>
        </button>
      </div>
    </section>

    <!-- ── 3-column data grid ──────────────────────────────────────────────── -->
    <section class="bv-grid">
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
    </section>

    <!-- ── Cognitive-kind breakdown (docs §3.5) ───────────────────────────── -->
    <section
      class="bv-card bv-cognitive"
      data-testid="bv-cognitive-breakdown"
    >
      <header class="bv-card-header">
        <h3>🧩 Cognitive kinds</h3>
        <span class="bv-card-subtle">Episodic / Semantic / Procedural — derived from tags + content</span>
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
      class="bv-card bv-rag"
      data-testid="bv-rag-capability"
    >
      <header class="bv-card-header">
        <h3>📡 RAG capability</h3>
        <span class="bv-card-subtle">6-signal hybrid scoring — vector search needs a local embedding model</span>
      </header>
      <div class="bv-rag-grid">
        <div
          v-for="sig in ragSignals"
          :key="sig.key"
          class="bv-rag-cell"
          :class="{ 'is-on': sig.available, 'is-off': !sig.available }"
          :title="sig.available ? `${sig.label} active` : sig.unavailableReason"
        >
          <span class="bv-rag-icon">{{ sig.available ? '✓' : '✗' }}</span>
          <span class="bv-rag-label">{{ sig.label }}</span>
          <span class="bv-rag-weight">{{ sig.weight }}</span>
        </div>
      </div>
      <p class="bv-rag-summary">
        <strong>Effective quality:</strong> {{ ragQuality.effective }}% —
        {{ ragQuality.note }}
      </p>
    </section>


    <!-- ── Active selection (docs §20) ─────────────────────────────────────── -->
    <section
      class="bv-card"
      data-testid="bv-active-selection"
    >
      <header class="bv-card-header">
        <h3>🎯 Active selection</h3>
        <span class="bv-card-subtle">
          <a
            class="bv-link"
            href="https://github.com/Terranes/TerranSoul/blob/main/docs/brain-advanced-design.md#brain-component-selection--routing--how-the-llm-knows-what-to-use"
            target="_blank"
            rel="noopener"
          >
            How the brain picks each component →
          </a>
        </span>
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
      class="bv-card"
      data-testid="bv-daily-learning"
    >
      <header class="bv-card-header">
        <h3>📚 Daily learning</h3>
        <span class="bv-card-subtle">
          <a
            class="bv-link"
            href="https://github.com/Terranes/TerranSoul/blob/main/docs/brain-advanced-design.md#how-daily-conversation-updates-the-brain--write-back--learning-loop"
            target="_blank"
            rel="noopener"
          >
            How conversation becomes long-term memory →
          </a>
        </span>
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
        <div style="display:flex;gap:8px;margin-top:8px;">
          <button
            class="bv-link"
            data-testid="bv-autolearn-force"
            @click="forceExtractNow"
          >
            Extract now →
          </button>
        </div>
      </template>
    </section>

    <!-- ── AI decision-making toggles ──────────────────────────────────────── -->
    <section
      class="bv-card"
      data-testid="bv-ai-decisions"
    >
      <header class="bv-card-header">
        <h3>🧭 AI decision-making</h3>
        <span class="bv-card-subtle">
          <a
            class="bv-link"
            href="https://github.com/Terranes/TerranSoul/blob/main/docs/brain-advanced-design.md#25-intent-classification"
            target="_blank"
            rel="noopener"
          >
            How TerranSoul decides what to do →
          </a>
        </span>
      </header>
      <p class="bv-cog-desc">
        TerranSoul makes a few opinionated routing decisions on your behalf — classifying intent, offering follow-up gates,
        and suggesting quests. Toggle any of them off for a strictly-pass-through experience. Settings persist locally.
      </p>
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

    <!-- ── Auto-tag toggle (Chunk 18.1) ────────────────────────────────────── -->
    <section class="bv-autolearn-section">
      <header class="bv-autolearn-header">
        <span class="bv-section-title">🏷 Auto-Tag</span>
      </header>
      <label
        class="bv-config-list"
        style="display:flex;align-items:center;gap:8px;"
      >
        <input
          type="checkbox"
          :checked="appSettings.settings?.auto_tag ?? false"
          data-testid="bv-autotag-toggle"
          @change="onToggleAutoTag(($event.target as HTMLInputElement).checked)"
        >
        <span>Auto-classify new memories with LLM tags</span>
      </label>
      <p class="bv-cog-desc">
        When enabled, each new memory is classified into curated prefix tags
        (<code>personal:*</code>, <code>domain:*</code>, <code>code:*</code>, …)
        via one LLM call. Tags are merged with any user-supplied tags.
      </p>
    </section>


    <!-- ── RPG stat sheet ──────────────────────────────────────────────────── -->
    <section class="bv-stats-section">
      <BrainStatSheet />
    </section>

    <!-- ── Code knowledge (GitNexus mirror) — Phase 13 Tier 4 ────────────── -->
    <section class="bv-code-knowledge-section">
      <CodeKnowledgePanel />
    </section>

    <!-- ── Persona panel (data storage & management) ──────────────────────── -->
    <section class="bv-persona-section">
      <PersonaPanel />
    </section>

    <!-- ── Mini memory graph ───────────────────────────────────────────────── -->
    <section class="bv-graph-section">
      <header class="bv-graph-header">
        <h3>🌌 Memory graph</h3>
        <span class="bv-graph-subtitle">
          Top {{ topMemories.length }} most-connected
          {{ topMemories.length === 1 ? 'memory' : 'memories' }} ·
          <button
            class="bv-link"
            @click="$emit('navigate', 'memory')"
          >Open full explorer →</button>
        </span>
      </header>
      <div
        v-if="topMemories.length === 0"
        class="bv-graph-empty"
      >
        No memories yet — chat with your brain or
        <button
          class="bv-link"
          @click="$emit('navigate', 'memory')"
        >
          add one
        </button>.
      </div>
      <div
        v-else
        class="bv-graph-wrap"
      >
        <MemoryGraph
          :memories="topMemories"
          :edges="topEdges"
          edge-mode="typed"
        />
      </div>
    </section>

    <!-- ── Danger zone ─────────────────────────────────────────────────────── -->
    <section
      class="bv-card bv-danger-zone"
      data-testid="bv-danger-zone"
    >
      <header class="bv-card-header">
        <h3>⚠️ Danger zone</h3>
      </header>
      <p class="bv-cog-desc">
        These actions are irreversible. Proceed with caution.
      </p>
      <div class="bv-danger-actions">
        <div class="bv-danger-row">
          <div class="bv-danger-info">
            <span class="bv-danger-label">Factory reset</span>
            <span class="bv-cog-desc">Remove all auto-configured components (brain, voice, quests), clear all memories and conversation history. Reverts to first-launch state.</span>
          </div>
          <button
            class="btn-danger"
            data-testid="bv-factory-reset"
            @click="confirmFactoryReset"
          >
            🔄 Factory reset
          </button>
        </div>
        <div class="bv-danger-row">
          <div class="bv-danger-info">
            <span class="bv-danger-label">Clean all data</span>
            <span class="bv-cog-desc">Permanently erase everything: memories, brain config, voice settings, persona, quests, app preferences. Returns to a fresh install.</span>
          </div>
          <button
            class="btn-danger"
            data-testid="bv-clear-all-data"
            @click="confirmClearAllData"
          >
            🗑 Clean all data
          </button>
        </div>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useBrainStore } from '../stores/brain';
import { useMemoryStore } from '../stores/memory';
import { useConversationStore } from '../stores/conversation';
import { useAiDecisionPolicyStore, type AiDecisionPolicy } from '../stores/ai-decision-policy';
import { useSettingsStore } from '../stores/settings';
import BrainAvatar from '../components/BrainAvatar.vue';
import BrainStatSheet from '../components/BrainStatSheet.vue';
import CodeKnowledgePanel from '../components/CodeKnowledgePanel.vue';
import MemoryGraph from '../components/MemoryGraph.vue';
import PersonaPanel from '../components/PersonaPanel.vue';
import type { MemoryEntry } from '../types';
import { summariseCognitiveKinds } from '../utils/cognitive-kind';
import { formatRam } from '../utils/format';

const emit = defineEmits<{
  /** Navigate to another tab; values match the App.vue tab ids. */
  (e: 'navigate', target: 'chat' | 'memory' | 'marketplace' | 'voice' | 'skills' | 'brain-setup'): void;
}>();
const emitNavigate = (target: 'chat' | 'memory' | 'marketplace' | 'voice' | 'skills' | 'brain-setup') => {
  emit('navigate', target);
};

const brain = useBrainStore();
const memory = useMemoryStore();
const appSettings = useSettingsStore();

const isRefreshing = ref(false);

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

const heroExpression = computed<'idle' | 'thinking' | 'happy' | 'sad' | 'sleepy'>(() => {
  if (!brain.brainMode) return 'sleepy';
  if (memoryCount.value === 0) return 'idle';
  if (memoryCount.value >= 10) return 'happy';
  return 'idle';
});

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
  let color = '#fbbf24';
  if (gb >= 32) { percent = 100; color = '#34d399'; }
  else if (gb >= 16) color = '#60a5fa';
  else if (gb >= 8) color = '#fbbf24';
  else color = '#f87171';
  return { percent, color, label: sys.ram_tier_label || '' };
});

// ── Quick mode switcher ────────────────────────────────────────────────────

interface ModeOption {
  key: 'free' | 'paid' | 'local' | 'lm_studio';
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
    detail: 'Pollinations · instant',
    description: 'Switch to the no-key Pollinations free brain',
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

async function switchToFree() {
  await brain.autoConfigureForDesktop();
}

// ── Top-N memory subgraph ──────────────────────────────────────────────────

const topMemories = computed<MemoryEntry[]>(() => {
  const memories = memory.memories ?? [];
  if (memories.length === 0) return [];
  // Score = edge degree + importance + decay so the mini-graph shows what
  // matters most. Cap at 12 nodes so the viewport stays readable.
  const degree = new Map<number, number>();
  for (const e of memory.edges ?? []) {
    degree.set(e.src_id, (degree.get(e.src_id) ?? 0) + 1);
    degree.set(e.dst_id, (degree.get(e.dst_id) ?? 0) + 1);
  }
  const scored = [...memories].map(m => ({
    m,
    s: (degree.get(m.id) ?? 0) * 3 + m.importance + m.decay_score,
  }));
  scored.sort((a, b) => b.s - a.s);
  return scored.slice(0, 12).map(x => x.m);
});

const topEdges = computed(() => {
  const ids = new Set(topMemories.value.map(m => m.id));
  return (memory.edges ?? []).filter(e => ids.has(e.src_id) && ids.has(e.dst_id));
});

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
      memory.fetchAll(),
      memory.getStats(),
      memory.fetchEdges(),
      memory.getEdgeStats(),
      loadBrainSelection(),
      loadAutoLearnPolicy(),
      appSettings.loadSettings(),
    ]);
  } finally {
    isRefreshing.value = false;
  }
}

onMounted(async () => {
  await refresh();
});
</script>

<style scoped>
.brain-view {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  padding: 1rem;
  height: 100%;
  overflow-y: auto;
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
.bv-pill-mood.bv-pill-free { background: rgba(123, 224, 179, 0.18); color: #7be0b3; border-color: rgba(123, 224, 179, 0.4); }
.bv-pill-mood.bv-pill-paid { background: rgba(124, 200, 255, 0.18); color: #7cc8ff; border-color: rgba(124, 200, 255, 0.4); }
.bv-pill-mood.bv-pill-local { background: rgba(200, 164, 255, 0.18); color: #c8a4ff; border-color: rgba(200, 164, 255, 0.4); }
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
.bv-mode-card.active.bv-mode-free { border-color: #7be0b3; background: rgba(123, 224, 179, 0.10); }
.bv-mode-card.active.bv-mode-paid { border-color: #7cc8ff; background: rgba(124, 200, 255, 0.10); }
.bv-mode-card.active.bv-mode-local { border-color: #c8a4ff; background: rgba(200, 164, 255, 0.10); }
.bv-mode-emoji { font-size: 1.5rem; }
.bv-mode-label { font-weight: 700; }
.bv-mode-detail { font-size: 0.75rem; color: var(--ts-text-muted); }

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

/* ── Mini graph ────────────────────────────────────────────────────────── */
.bv-graph-section {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding: 0.85rem 1rem;
  background: var(--ts-bg-surface);
  border: 1px solid var(--ts-border);
  border-radius: 10px;
}
.bv-graph-header {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  flex-wrap: wrap;
  gap: 0.5rem;
}
.bv-graph-header h3 { margin: 0; font-size: 0.95rem; color: var(--ts-text-primary); }
.bv-graph-subtitle { font-size: 0.8rem; color: var(--ts-text-muted); }
.bv-graph-empty {
  padding: 2rem;
  text-align: center;
  color: var(--ts-text-muted);
}
.bv-graph-wrap { height: 320px; }
.bv-graph-wrap > * { height: 100%; }

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
.bv-cog-episodic .bv-cog-bar-fill { background: linear-gradient(90deg, #f97316, #fb923c); }
.bv-cog-semantic .bv-cog-bar-fill { background: linear-gradient(90deg, #60a5fa, #93c5fd); }
.bv-cog-procedural .bv-cog-bar-fill { background: linear-gradient(90deg, #34d399, #86efac); }
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
  color: #fff;
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
}
</style>
