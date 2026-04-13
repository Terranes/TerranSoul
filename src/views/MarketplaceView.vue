<template>
  <div class="marketplace-view">
    <header class="mp-header">
      <h2>🏪 Agent Marketplace</h2>
      <div class="mp-header-actions">
        <button class="btn-secondary" @click="refreshAll" :disabled="isLoading">
          {{ isLoading ? 'Loading…' : '🔄 Refresh' }}
        </button>
      </div>
    </header>

    <p v-if="packageStore.error" class="mp-error">{{ packageStore.error }}</p>

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
    </div>

    <!-- ── Installed tab ── -->
    <div v-if="activeTab === 'installed'" class="mp-panel">
      <p v-if="packageStore.installedAgents.length === 0" class="mp-status">No agents installed yet.</p>

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
import CapabilityConsentDialog from '../components/CapabilityConsentDialog.vue';
import type { AgentSearchResult } from '../types';

const packageStore = usePackageStore();
const sandboxStore = useSandboxStore();

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
</style>
