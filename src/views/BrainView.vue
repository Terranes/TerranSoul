<template>
  <div class="brain-view" data-testid="brain-view">
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
        <p class="bv-hero-subtitle">{{ heroSubtitle }}</p>
        <div class="bv-hero-pills">
          <span class="bv-pill bv-pill-mood" :class="`bv-pill-${moodKey}`">
            {{ moodPillLabel }}
          </span>
          <span v-if="memoryCount > 0" class="bv-pill bv-pill-memory">
            🧠 {{ memoryCount }} memories
          </span>
          <span v-if="edgeCount > 0" class="bv-pill bv-pill-edges">
            🔗 {{ edgeCount }} connections
          </span>
          <span v-if="brain.ollamaStatus.running" class="bv-pill bv-pill-ollama">
            🖥 Ollama running
          </span>
        </div>
      </div>
      <div class="bv-hero-actions">
        <button class="btn-primary" @click="$emit('navigate', 'brain-setup')">
          ⚙ Brain setup
        </button>
        <button class="btn-secondary" @click="$emit('navigate', 'marketplace')">
          🏪 Switch model
        </button>
        <button class="btn-secondary" @click="refresh" :disabled="isRefreshing">
          {{ isRefreshing ? '⟳ Refreshing…' : '↻ Refresh' }}
        </button>
      </div>
    </section>

    <!-- ── Quick mode switcher ─────────────────────────────────────────────── -->
    <section class="bv-mode-switcher" data-testid="bv-mode-switcher">
      <div class="bv-section-title">⚡ Quick mode</div>
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
      <article class="bv-card" data-testid="bv-card-config">
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
            <dd class="bv-model"><code>{{ configRows.model }}</code></dd>
          </div>
          <div v-if="configRows.endpoint" class="bv-dl-row">
            <dt>Endpoint</dt>
            <dd class="bv-endpoint" :title="configRows.endpoint">{{ shortUrl(configRows.endpoint) }}</dd>
          </div>
        </dl>
      </article>

      <!-- Hardware card -->
      <article class="bv-card" data-testid="bv-card-hardware">
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
          <div v-if="hardwareRows.gpu" class="bv-dl-row">
            <dt>GPU</dt>
            <dd>{{ hardwareRows.gpu }}</dd>
          </div>
        </dl>
        <div v-if="ramTier" class="bv-ram-bar" :title="`RAM tier: ${ramTier.label}`">
          <div class="bv-ram-fill" :style="{ width: ramTier.percent + '%', background: ramTier.color }" />
        </div>
      </article>

      <!-- Memory health card -->
      <article class="bv-card" data-testid="bv-card-memory">
        <header class="bv-card-header">
          <h3>🧠 Memory health</h3>
          <button class="bv-card-link" @click="$emit('navigate', 'memory')">Open explorer →</button>
        </header>
        <div class="bv-memory-tiers">
          <div class="bv-mem-tier tier-short" :title="`Short-term: ${memoryStats.short_count}`">
            <span class="bv-mem-num">{{ memoryStats.short_count }}</span>
            <span class="bv-mem-label">short</span>
          </div>
          <div class="bv-mem-tier tier-working" :title="`Working: ${memoryStats.working_count}`">
            <span class="bv-mem-num">{{ memoryStats.working_count }}</span>
            <span class="bv-mem-label">working</span>
          </div>
          <div class="bv-mem-tier tier-long" :title="`Long-term: ${memoryStats.long_count}`">
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
                <span class="bv-decay-fill" :style="{ width: (memoryStats.avg_decay * 100) + '%' }" />
              </span>
              <span class="bv-decay-num">{{ Math.round(memoryStats.avg_decay * 100) }}%</span>
            </dd>
          </div>
        </dl>
      </article>
    </section>

    <!-- ── RPG stat sheet ──────────────────────────────────────────────────── -->
    <section class="bv-stats-section">
      <BrainStatSheet />
    </section>

    <!-- ── Mini memory graph ───────────────────────────────────────────────── -->
    <section class="bv-graph-section">
      <header class="bv-graph-header">
        <h3>🌌 Memory graph</h3>
        <span class="bv-graph-subtitle">
          Top {{ topMemories.length }} most-connected
          {{ topMemories.length === 1 ? 'memory' : 'memories' }} ·
          <button class="bv-link" @click="$emit('navigate', 'memory')">Open full explorer →</button>
        </span>
      </header>
      <div v-if="topMemories.length === 0" class="bv-graph-empty">
        No memories yet — chat with your brain or
        <button class="bv-link" @click="$emit('navigate', 'memory')">add one</button>.
      </div>
      <div v-else class="bv-graph-wrap">
        <MemoryGraph
          :memories="topMemories"
          :edges="topEdges"
          edge-mode="typed"
        />
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useBrainStore } from '../stores/brain';
import { useMemoryStore } from '../stores/memory';
import BrainAvatar from '../components/BrainAvatar.vue';
import BrainStatSheet from '../components/BrainStatSheet.vue';
import MemoryGraph from '../components/MemoryGraph.vue';
import type { MemoryEntry } from '../types';

defineEmits<{
  /** Navigate to another tab; values match the App.vue tab ids. */
  (e: 'navigate', target: 'chat' | 'memory' | 'marketplace' | 'voice' | 'skills' | 'brain-setup'): void;
}>();
const emitNavigate = (target: 'chat' | 'memory' | 'marketplace' | 'voice' | 'skills' | 'brain-setup') => {
  // Re-trigger the same event using the parent emit by dispatching a custom
  // event on the host element — simpler than re-binding `defineEmits` inside
  // helpers. Parent listens via `@navigate` which Vue maps from `emit('navigate')`.
  // We fall back to the standard emit pattern below by exposing a wrapper.
  void target;
};

const brain = useBrainStore();
const memory = useMemoryStore();

const isRefreshing = ref(false);

// ── Hero text ──────────────────────────────────────────────────────────────

const moodKey = computed<'none' | 'free' | 'paid' | 'local'>(() => {
  const m = brain.brainMode;
  if (!m) return 'none';
  if (m.mode === 'free_api') return 'free';
  if (m.mode === 'paid_api') return 'paid';
  if (m.mode === 'local_ollama') return 'local';
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

// ── Memory stats ───────────────────────────────────────────────────────────

const memoryStats = computed(() => memory.stats ?? {
  total: 0, short_count: 0, working_count: 0, long_count: 0,
  total_tokens: 0, avg_decay: 1.0,
});
const memoryCount = computed(() => memoryStats.value.total);
const edgeCount = computed(() => memory.edgeStats?.total_edges ?? memory.edges.length);

// ── Configuration card ────────────────────────────────────────────────────

const providerName = computed(() => {
  const m = brain.brainMode;
  if (!m) return null;
  if (m.mode === 'free_api') {
    const p = brain.freeProviders.find(fp => fp.id === m.provider_id);
    return p?.display_name ?? m.provider_id;
  }
  if (m.mode === 'paid_api') return m.base_url;
  if (m.mode === 'local_ollama') return 'Ollama';
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
      mode: 'Local Ollama',
      provider: 'localhost',
      model: m.model,
      endpoint: 'http://localhost:11434',
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

function formatRam(mb: number): string {
  if (mb >= 1024) return (mb / 1024).toFixed(1) + ' GB';
  return mb + ' MB';
}

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
    label: 'Local Ollama',
    emoji: '🖥',
    detail: brain.ollamaStatus.running
      ? `${brain.installedModels.length} model${brain.installedModels.length === 1 ? '' : 's'} ready`
      : 'Requires Ollama running',
    description: 'Pick a local Ollama model from the marketplace',
    disabled: !brain.ollamaStatus.running,
    disabledReason: 'Ollama is not running — start it with `ollama serve`',
    action: () => emitNavigate('marketplace'),
  },
]);

// Re-export emit so module functions can call it.
let emitFn: (name: 'navigate', target: string) => void = () => {};
function emitNavigate(target: string) {
  emitFn('navigate', target);
}
defineExpose({});
// Capture the runtime emit at setup-time.
const _emit = (() => {
  // Vue's defineEmits returns a function in <script setup>. We re-bind it
  // here so it's reachable from helpers above without prop drilling.
  const e = defineEmits<{ (n: 'navigate', t: string): void }>();
  emitFn = e as unknown as typeof emitFn;
  return e;
})();
void _emit;

async function switchToFree() {
  await brain.autoConfigureForDesktop();
}

// ── Top-N memory subgraph ──────────────────────────────────────────────────

const topMemories = computed<MemoryEntry[]>(() => {
  const memories = memory.memories;
  if (memories.length === 0) return [];
  // Score = edge degree + importance + decay so the mini-graph shows what
  // matters most. Cap at 12 nodes so the viewport stays readable.
  const degree = new Map<number, number>();
  for (const e of memory.edges) {
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
  return memory.edges.filter(e => ids.has(e.src_id) && ids.has(e.dst_id));
});

// ── Refresh ────────────────────────────────────────────────────────────────

async function refresh() {
  isRefreshing.value = true;
  try {
    await Promise.allSettled([
      brain.loadBrainMode(),
      brain.fetchFreeProviders(),
      brain.fetchSystemInfo(),
      brain.checkOllamaStatus(),
      brain.fetchInstalledModels(),
      memory.fetchAll(),
      memory.getStats(),
      memory.fetchEdges(),
      memory.getEdgeStats(),
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
  background: linear-gradient(160deg, rgba(20, 18, 40, 0.85) 0%, rgba(12, 10, 28, 0.92) 100%);
  border: 1px solid var(--ts-border, rgba(255,255,255,0.08));
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
  background: rgba(255, 255, 255, 0.06);
  color: var(--ts-text-secondary);
  border: 1px solid rgba(255, 255, 255, 0.08);
}
.bv-pill-mood.bv-pill-free { background: rgba(123, 224, 179, 0.18); color: #7be0b3; border-color: rgba(123, 224, 179, 0.4); }
.bv-pill-mood.bv-pill-paid { background: rgba(124, 200, 255, 0.18); color: #7cc8ff; border-color: rgba(124, 200, 255, 0.4); }
.bv-pill-mood.bv-pill-local { background: rgba(200, 164, 255, 0.18); color: #c8a4ff; border-color: rgba(200, 164, 255, 0.4); }
.bv-pill-mood.bv-pill-none { background: rgba(248, 113, 113, 0.18); color: #f87171; border-color: rgba(248, 113, 113, 0.4); }

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
  border-color: rgba(255, 255, 255, 0.2);
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
  background: rgba(255, 255, 255, 0.06);
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
  background: rgba(255, 255, 255, 0.04);
}
.bv-mem-tier.tier-short { color: #fbbf24; border: 1px solid rgba(251, 191, 36, 0.3); }
.bv-mem-tier.tier-working { color: #60a5fa; border: 1px solid rgba(96, 165, 250, 0.3); }
.bv-mem-tier.tier-long { color: #34d399; border: 1px solid rgba(52, 211, 153, 0.3); }
.bv-mem-num { font-size: 1.1rem; font-weight: 700; }
.bv-mem-label { font-size: 0.7rem; text-transform: uppercase; letter-spacing: 0.05em; opacity: 0.8; }

.bv-decay-bar {
  display: inline-block;
  width: 60px;
  height: 6px;
  background: rgba(255, 255, 255, 0.06);
  border-radius: 3px;
  overflow: hidden;
  vertical-align: middle;
  margin-right: 6px;
}
.bv-decay-fill { display: block; height: 100%; background: #34d399; }
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

@media (max-width: 720px) {
  .bv-hero { grid-template-columns: auto 1fr; }
  .bv-hero-actions {
    grid-column: 1 / -1;
    flex-direction: row;
    flex-wrap: wrap;
  }
}
@media (max-width: 480px) {
  .bv-hero { grid-template-columns: 1fr; text-align: center; }
  .bv-hero-avatar { justify-self: center; }
  .bv-hero-pills { justify-content: center; }
}
</style>
