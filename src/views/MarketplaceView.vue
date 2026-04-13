<template>
  <div class="marketplace-view">
    <header class="mp-header">
      <h2>🏪 Agent Marketplace</h2>
      <div class="mp-header-actions">
        <button class="btn-secondary" @click="refreshAll" :disabled="isLoading || !tauriAvailable">
          {{ isLoading ? 'Loading…' : '🔄 Refresh' }}
        </button>
      </div>
    </header>

    <p v-if="packageStore.error && tauriAvailable" class="mp-error">{{ packageStore.error }}</p>

    <!-- Tabs -->
    <nav class="mp-tabs">
      <button
        v-for="tab in tabs"
        :key="tab.id"
        :class="['mp-tab', { active: activeTab === tab.id }]"
        @click="activeTab = tab.id"
      >{{ tab.icon }} {{ tab.label }}</button>
    </nav>

    <!-- ── Browse tab ── -->
    <div v-if="activeTab === 'browse'" class="mp-panel">
      <!-- No Tauri: show inline Tauri notification banner -->
      <template v-if="!tauriAvailable">
        <div class="tauri-banner">
          <!-- Header row -->
          <div class="tauri-banner-main">
            <span class="tauri-banner-icon">⚠️</span>
            <div class="tauri-banner-text">
              <strong>Tauri Desktop Backend Unavailable</strong>
              <span class="tauri-banner-sub">
                {{ hostingContext }}
                — Agents, local Ollama, and device pairing require the desktop app.
              </span>
            </div>
          </div>

          <!-- Brain status -->
          <div v-if="brainStore.isFreeApiMode" class="tauri-brain-row">
            <span class="tauri-brain-dot" />
            <span>☁️ Free Cloud LLM active — <strong>{{ activeProviderName }}</strong></span>
            <span class="tauri-brain-badge">✅ Ready to chat</span>
          </div>

          <!-- Collapsible details -->
          <button class="tauri-details-toggle" @click="showDetails = !showDetails">
            {{ showDetails ? '▾ Hide details' : '▸ Show details — why &amp; how to fix' }}
          </button>

          <div v-if="showDetails" class="tauri-details">
            <div class="tauri-section">
              <h4>Why am I seeing this?</h4>
              <p>
                TerranSoul uses <a href="https://v2.tauri.app" target="_blank" rel="noopener">Tauri</a>,
                a Rust-based desktop runtime.
                When running as a web app {{ isVercel ? 'on Vercel' : 'in the browser' }},
                the native backend isn't available.
                A free cloud LLM ({{ activeProviderName || 'Pollinations AI' }}) was auto-configured
                so you can still chat.
              </p>
            </div>

            <div class="tauri-section">
              <h4>What works {{ isVercel ? 'on Vercel' : 'in browser mode' }}?</h4>
              <ul class="tauri-feature-list">
                <li class="avail">✅ Chat with free cloud LLM</li>
                <li class="avail">✅ 3D character &amp; animations</li>
                <li class="avail">✅ Model / background selection</li>
                <li class="unavail">❌ Agent Marketplace (install / manage agents)</li>
                <li class="unavail">❌ Local Ollama models</li>
                <li class="unavail">❌ Long-term memory persistence</li>
                <li class="unavail">❌ Device pairing &amp; sync</li>
              </ul>
            </div>

            <div v-if="isVercel" class="tauri-section">
              <h4>Deploying on Vercel (UAT)</h4>
              <p>
                Vercel serves only the static frontend — it cannot run Tauri's Rust backend.
                This is expected for UAT testing of the web UI. To get full functionality:
              </p>
              <ol class="tauri-steps">
                <li>
                  <strong>For full desktop features:</strong> build the Tauri app locally with
                  <code>npm run tauri build</code> or <code>npm run tauri dev</code>.
                </li>
                <li>
                  <strong>For Vercel UAT:</strong> the web-only mode auto-configures a free cloud LLM.
                  No additional Vercel config is needed — it works out of the box.
                </li>
                <li>
                  <strong>Custom provider (optional):</strong> set
                  <code>VITE_DEFAULT_PROVIDER</code> in Vercel project settings to override the
                  default free provider (e.g. <code>groq</code>), and
                  <code>VITE_FREE_API_KEY</code> for providers that require an API key.
                </li>
              </ol>
            </div>

            <div v-else class="tauri-section">
              <h4>Getting the full experience</h4>
              <p>
                Download the TerranSoul desktop app or run
                <code>npm run tauri dev</code> locally to access all features including the
                agent marketplace, local Ollama models, and device pairing.
              </p>
            </div>
          </div>
        </div>
      </template>

      <!-- Desktop mode: full marketplace -->
      <template v-else>
        <div class="mp-search-row">
          <input
            v-model="searchQuery"
            placeholder="Search agents…"
            class="mp-search"
            aria-label="Search agents"
            @keyup.enter="doSearch"
          />
          <button class="btn-secondary" @click="doSearch">🔍 Search</button>
        </div>

        <p v-if="isLoading" class="mp-status">Loading agents…</p>
        <p v-else-if="displayedAgents.length === 0" class="mp-status">No agents found.</p>

        <div v-else class="mp-grid">
          <div
            v-for="agent in displayedAgents"
            :key="agent.name"
            class="mp-card"
          >
            <div class="mp-card-header">
              <h3 class="mp-agent-name">{{ agent.name }}</h3>
              <span class="mp-version">v{{ agent.version }}</span>
            </div>
            <p class="mp-description">{{ agent.description }}</p>
            <div class="mp-caps">
              <span
                v-for="cap in agent.capabilities"
                :key="cap"
                class="mp-cap-badge"
              >{{ cap }}</span>
            </div>
            <div v-if="agent.homepage" class="mp-homepage">
              <span class="mp-link-label">🔗 {{ agent.homepage }}</span>
            </div>
            <div class="mp-card-actions">
              <template v-if="isInstalled(agent.name)">
                <span class="mp-installed-badge">✅ Installed</span>
                <button
                  class="btn-secondary btn-sm"
                  @click="handleUpdate(agent)"
                  :disabled="isLoading"
                >⬆ Update</button>
                <button
                  class="btn-danger btn-sm"
                  @click="handleRemove(agent.name)"
                  :disabled="isLoading"
                >🗑 Remove</button>
              </template>
              <template v-else>
                <button
                  class="btn-primary btn-sm"
                  @click="promptInstall(agent)"
                  :disabled="isLoading"
                >⬇ Install</button>
              </template>
            </div>
          </div>
        </div>
      </template>
    </div>

    <!-- ── Installed tab ── -->
    <div v-if="activeTab === 'installed'" class="mp-panel">
      <template v-if="!tauriAvailable">
        <div class="tauri-banner tauri-banner-compact">
          <div class="tauri-banner-main">
            <span class="tauri-banner-icon">📦</span>
            <div class="tauri-banner-text">
              <strong>No Desktop Agents</strong>
              <span class="tauri-banner-sub">
                Agent installation requires the TerranSoul desktop app
                (<code>npm run tauri dev</code>).
                In {{ isVercel ? 'Vercel' : 'browser' }} mode, the free cloud LLM handles conversations.
              </span>
            </div>
          </div>
        </div>
      </template>
      <p v-else-if="packageStore.installedAgents.length === 0" class="mp-status">No agents installed yet.</p>

      <div v-else class="mp-grid">
        <div
          v-for="agent in packageStore.installedAgents"
          :key="agent.name"
          class="mp-card mp-card-installed"
        >
          <div class="mp-card-header">
            <h3 class="mp-agent-name">{{ agent.name }}</h3>
            <span class="mp-version">v{{ agent.version }}</span>
          </div>
          <p class="mp-description">{{ agent.description }}</p>
          <div class="mp-sandbox-status">
            <span class="mp-sandbox-badge" :class="sandboxBadgeClass(agent.name)">
              {{ sandboxLabel(agent.name) }}
            </span>
          </div>
          <div class="mp-card-actions">
            <button
              class="btn-secondary btn-sm"
              @click="viewCapabilities(agent.name)"
            >🔐 Capabilities</button>
            <button
              class="btn-danger btn-sm"
              @click="handleRemove(agent.name)"
              :disabled="isLoading"
            >🗑 Remove</button>
          </div>
        </div>
      </div>
    </div>

    <!-- Consent dialog -->
    <CapabilityConsentDialog
      v-if="consentAgent"
      :agent-name="consentAgent.name"
      :capabilities="consentAgent.capabilities"
      :sensitive-capabilities="consentAgent.sensitiveCapabilities"
      @confirm="confirmInstall"
      @cancel="consentAgent = null"
    />

    <!-- Capabilities detail modal -->
    <div v-if="capDetailAgent" class="mp-modal-backdrop" @click.self="capDetailAgent = null">
      <div class="mp-modal">
        <h3>🔐 {{ capDetailAgent }} — Capabilities</h3>
        <p v-if="sandboxStore.isLoading" class="mp-status">Loading…</p>
        <ul v-else-if="sandboxStore.consents.length > 0" class="mp-cap-list">
          <li
            v-for="c in sandboxStore.consents"
            :key="c.capability"
            class="mp-cap-row"
          >
            <span>{{ c.capability }}</span>
            <span :class="['mp-grant-badge', c.granted ? 'granted' : 'denied']">
              {{ c.granted ? '✅ Granted' : '❌ Denied' }}
            </span>
          </li>
        </ul>
        <p v-else class="mp-status">No capability consents recorded.</p>
        <div class="mp-modal-btns">
          <button class="btn-secondary" @click="capDetailAgent = null">Close</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { usePackageStore } from '../stores/package';
import { useSandboxStore } from '../stores/sandbox';
import { useBrainStore } from '../stores/brain';
import CapabilityConsentDialog from '../components/CapabilityConsentDialog.vue';
import type { AgentSearchResult } from '../types';

const packageStore = usePackageStore();
const sandboxStore = useSandboxStore();
const brainStore = useBrainStore();

/** Whether the Tauri IPC bridge is available. */
const tauriAvailable = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

/** Detect Vercel hosting via known URL patterns. */
const isVercel = computed(() => {
  if (typeof window === 'undefined') return false;
  const host = window.location.hostname;
  return host.endsWith('.vercel.app') || host.endsWith('.vercel.sh') || host.includes('.now.sh');
});

const hostingContext = computed(() => {
  if (isVercel.value) return 'Running on Vercel (web-only mode)';
  if (typeof window !== 'undefined' && window.location.protocol === 'file:') return 'Running from a local file';
  return 'Running in browser mode';
});

const activeProviderName = computed(() => {
  const mode = brainStore.brainMode;
  if (!mode || mode.mode !== 'free_api') return '';
  const p = brainStore.freeProviders.find((fp) => fp.id === mode.provider_id);
  return p?.display_name ?? mode.provider_id ?? '';
});

const showDetails = ref(false);

const activeTab = ref<'browse' | 'installed'>('browse');
const tabs = [
  { id: 'browse' as const, icon: '🔍', label: 'Browse' },
  { id: 'installed' as const, icon: '📦', label: 'Installed' },
];

const searchQuery = ref('');
const isLoading = computed(() => packageStore.isLoading);

const displayedAgents = computed(() => {
  return packageStore.searchResults;
});

// Consent dialog state
const consentAgent = ref<{
  name: string;
  capabilities: string[];
  sensitiveCapabilities: string[];
} | null>(null);

// Capability detail modal
const capDetailAgent = ref<string | null>(null);

// Sandbox badge state cache
const agentSandboxStatus = ref<Record<string, boolean>>({});

function isInstalled(name: string): boolean {
  return packageStore.installedAgents.some((a) => a.name === name);
}

function sandboxBadgeClass(name: string): string {
  if (agentSandboxStatus.value[name] === true) return 'sandboxed';
  if (agentSandboxStatus.value[name] === false) return 'unrestricted';
  return 'unknown';
}

function sandboxLabel(name: string): string {
  if (agentSandboxStatus.value[name] === true) return '🔒 Sandboxed';
  if (agentSandboxStatus.value[name] === false) return '🔓 Unrestricted';
  return '❓ Unknown';
}

async function refreshAll() {
  if (!tauriAvailable) return;
  await packageStore.searchAgents('');
  await packageStore.fetchInstalledAgents();
  await refreshSandboxStatus();
}

async function refreshSandboxStatus() {
  for (const agent of packageStore.installedAgents) {
    const caps = await sandboxStore.listCapabilities(agent.name);
    agentSandboxStatus.value[agent.name] = caps.length > 0;
  }
}

async function doSearch() {
  await packageStore.searchAgents(searchQuery.value);
}

function promptInstall(agent: AgentSearchResult) {
  // Determine sensitive capabilities for the consent dialog
  const sensitiveCaps = agent.capabilities.filter((c) =>
    ['filesystem', 'network', 'clipboard', 'process_spawn'].includes(c),
  );
  consentAgent.value = {
    name: agent.name,
    capabilities: agent.capabilities,
    sensitiveCapabilities: sensitiveCaps,
  };
}

async function confirmInstall() {
  if (!consentAgent.value) return;
  const name = consentAgent.value.name;
  const sensitiveCaps = consentAgent.value.sensitiveCapabilities;
  consentAgent.value = null;

  // Grant sensitive capabilities the user consented to
  for (const cap of sensitiveCaps) {
    const capNames = capabilityToSandboxNames(cap);
    for (const capName of capNames) {
      await sandboxStore.grantCapability(name, capName);
    }
  }

  await packageStore.installAgent(name);
  await refreshSandboxStatus();
}

function capabilityToSandboxNames(
  cap: string,
): ('file_read' | 'file_write' | 'clipboard' | 'network' | 'process_spawn')[] {
  const map: Record<string, ('file_read' | 'file_write' | 'clipboard' | 'network' | 'process_spawn')[]> = {
    filesystem: ['file_read', 'file_write'],
    network: ['network'],
    clipboard: ['clipboard'],
    process_spawn: ['process_spawn'],
  };
  return map[cap] ?? [];
}

async function handleUpdate(agent: AgentSearchResult) {
  await packageStore.updateAgent(agent.name);
}

async function handleRemove(name: string) {
  await packageStore.removeAgent(name);
  await sandboxStore.clearCapabilities(name);
  await refreshSandboxStatus();
}

async function viewCapabilities(name: string) {
  capDetailAgent.value = name;
  await sandboxStore.listCapabilities(name);
}

onMounted(async () => {
  await refreshAll();
});
</script>

<style scoped>
.marketplace-view { display: flex; flex-direction: column; height: 100%; padding: 1rem; gap: 0.75rem; overflow: hidden; }
.mp-header { display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 0.5rem; }
.mp-header h2 { margin: 0; font-size: 1.25rem; }
.mp-header-actions { display: flex; gap: 0.5rem; }
.mp-error { padding: 0.5rem 1rem; background: #7f1d1d; color: #fca5a5; border-radius: 6px; margin: 0; }
.mp-tabs { display: flex; gap: 0.25rem; }
.mp-tab { padding: 0.4rem 1rem; border: none; border-radius: 6px; cursor: pointer; background: #1e293b; color: #94a3b8; }
.mp-tab.active { background: #3b82f6; color: #fff; }
.mp-panel { flex: 1; display: flex; flex-direction: column; gap: 0.75rem; overflow-y: auto; min-height: 0; }
.mp-search-row { display: flex; gap: 0.5rem; }
.mp-search { flex: 1; padding: 0.4rem 0.75rem; background: #1e293b; border: 1px solid #334155; border-radius: 6px; color: #f1f5f9; }
.mp-status { color: #64748b; text-align: center; padding: 2rem; }
.mp-grid { display: flex; flex-direction: column; gap: 0.75rem; }
.mp-card { padding: 1rem; background: #1e293b; border-radius: 8px; border-left: 4px solid #3b82f6; display: flex; flex-direction: column; gap: 0.5rem; }
.mp-card-installed { border-left-color: #22c55e; }
.mp-card-header { display: flex; align-items: baseline; gap: 0.5rem; }
.mp-agent-name { margin: 0; font-size: 1rem; }
.mp-version { font-size: 0.75rem; color: #64748b; }
.mp-description { margin: 0; color: #94a3b8; font-size: 0.85rem; }
.mp-caps { display: flex; gap: 0.3rem; flex-wrap: wrap; }
.mp-cap-badge { font-size: 0.7rem; padding: 0.15rem 0.5rem; background: #0f172a; border-radius: 999px; color: #94a3b8; }
.mp-homepage { font-size: 0.75rem; color: #64748b; }
.mp-link-label { word-break: break-all; }
.mp-card-actions { display: flex; align-items: center; gap: 0.5rem; margin-top: 0.25rem; }
.mp-installed-badge { font-size: 0.8rem; color: #22c55e; margin-right: auto; }
.mp-sandbox-status { display: flex; gap: 0.5rem; }
.mp-sandbox-badge { font-size: 0.75rem; padding: 0.2rem 0.6rem; border-radius: 999px; }
.mp-sandbox-badge.sandboxed { background: #052e16; color: #4ade80; }
.mp-sandbox-badge.unrestricted { background: #422006; color: #fb923c; }
.mp-sandbox-badge.unknown { background: #1e293b; color: #64748b; }
.btn-primary { padding: 0.4rem 1rem; background: #3b82f6; color: #fff; border: none; border-radius: 6px; cursor: pointer; }
.btn-secondary { padding: 0.4rem 1rem; background: #334155; color: #f1f5f9; border: none; border-radius: 6px; cursor: pointer; }
.btn-danger { padding: 0.35rem 0.75rem; background: #7f1d1d; color: #fca5a5; border: none; border-radius: 6px; cursor: pointer; }
.btn-sm { padding: 0.3rem 0.6rem; font-size: 0.8rem; }
.mp-modal-backdrop { position: fixed; inset: 0; background: rgba(0,0,0,0.6); display: flex; align-items: center; justify-content: center; z-index: 100; }
.mp-modal { background: #1e293b; border-radius: 12px; padding: 1.5rem; width: min(480px, 90vw); display: flex; flex-direction: column; gap: 0.75rem; }
.mp-cap-list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 0.4rem; }
.mp-cap-row { display: flex; justify-content: space-between; align-items: center; padding: 0.5rem 0.75rem; background: #0f172a; border-radius: 6px; font-size: 0.85rem; }
.mp-grant-badge { font-size: 0.75rem; }
.mp-grant-badge.granted { color: #22c55e; }
.mp-grant-badge.denied { color: #ef4444; }
.mp-modal-btns { display: flex; gap: 0.5rem; justify-content: flex-end; }

/* ── Tauri unavailable banner (inline in marketplace) ── */
.tauri-banner {
  background: linear-gradient(135deg, rgba(251, 191, 36, 0.10), rgba(245, 158, 11, 0.06));
  border: 1px solid rgba(251, 191, 36, 0.25);
  border-radius: 10px;
  display: flex;
  flex-direction: column;
  gap: 0;
  overflow: hidden;
}
.tauri-banner-compact { border-color: rgba(100, 116, 139, 0.25); background: rgba(30, 41, 59, 0.6); }

.tauri-banner-main {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  padding: 0.75rem 1rem;
}
.tauri-banner-icon { font-size: 1.1rem; flex-shrink: 0; }
.tauri-banner-text { flex: 1; min-width: 0; }
.tauri-banner-text strong { color: #fbbf24; font-size: 0.88rem; }
.tauri-banner-compact .tauri-banner-text strong { color: #e2e8f0; }
.tauri-banner-sub { display: block; color: #94a3b8; font-size: 0.78rem; margin-top: 2px; }
.tauri-banner-sub code { background: rgba(30, 41, 59, 0.8); padding: 1px 4px; border-radius: 3px; font-size: 0.74rem; color: #e2e8f0; }

/* Brain status row */
.tauri-brain-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 6px 1rem;
  background: rgba(22, 163, 74, 0.08);
  border-top: 1px solid rgba(34, 197, 94, 0.12);
  font-size: 0.78rem;
  color: #86efac;
}
.tauri-brain-dot { width: 6px; height: 6px; border-radius: 50%; background: #22c55e; animation: pulse-dot 2s ease-in-out infinite; flex-shrink: 0; }
@keyframes pulse-dot { 0%, 100% { opacity: 1; } 50% { opacity: 0.4; } }
.tauri-brain-badge { margin-left: auto; font-size: 0.72rem; color: #4ade80; white-space: nowrap; }

/* Details toggle */
.tauri-details-toggle {
  background: none;
  border: none;
  border-top: 1px solid rgba(251, 191, 36, 0.12);
  color: #fbbf24;
  font-size: 0.76rem;
  padding: 6px 1rem;
  text-align: left;
  cursor: pointer;
}
.tauri-details-toggle:hover { background: rgba(251, 191, 36, 0.06); }

/* Expandable details */
.tauri-details {
  padding: 0.5rem 1rem 1rem 2.5rem;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  border-top: 1px solid rgba(251, 191, 36, 0.08);
}
.tauri-section h4 { margin: 0 0 0.25rem; font-size: 0.82rem; color: #e2e8f0; }
.tauri-section p { margin: 0; color: #94a3b8; font-size: 0.78rem; line-height: 1.5; }
.tauri-section a { color: #60a5fa; text-decoration: none; }
.tauri-section a:hover { text-decoration: underline; }

.tauri-feature-list {
  list-style: none; margin: 0; padding: 0;
  display: grid; grid-template-columns: 1fr 1fr; gap: 2px 1rem;
  font-size: 0.78rem;
}
.tauri-feature-list .avail { color: #4ade80; }
.tauri-feature-list .unavail { color: #94a3b8; }

.tauri-steps {
  margin: 0.25rem 0 0; padding-left: 1.25rem;
  font-size: 0.78rem; color: #94a3b8;
  display: flex; flex-direction: column; gap: 0.4rem; line-height: 1.5;
}
.tauri-steps code { background: rgba(30, 41, 59, 0.8); padding: 1px 5px; border-radius: 3px; font-size: 0.74rem; color: #e2e8f0; }
.tauri-steps strong { color: #cbd5e1; }
</style>
