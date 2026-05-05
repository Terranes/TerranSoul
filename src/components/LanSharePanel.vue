<template>
  <section class="lan-share" aria-label="LAN Brain Sharing">
    <h3 class="lan-share__title">LAN Brain Sharing</h3>

    <!-- Error banner -->
    <div v-if="store.error" class="lan-share__error" role="alert">
      {{ store.error }}
    </div>

    <!-- Host Mode Section -->
    <fieldset class="lan-share__section">
      <legend>Share Your Brain</legend>
      <p class="lan-share__desc">
        Share your brain's knowledge with other TerranSoul instances on the
        same network. Choose whether peers need your token or can use a
        public read-only connection.
      </p>

      <div class="lan-share__auth-mode">
        <span class="lan-share__label-text">Access mode</span>
        <label class="lan-share__choice">
          <input
            :checked="hostAccessMode === 'token_required'"
            type="radio"
            name="lan-auth-mode"
            @change="void updateLanAuthMode('token_required')"
          >
          <span>Token required</span>
        </label>
        <label class="lan-share__choice">
          <input
            :checked="hostAccessMode === 'public_read_only'"
            type="radio"
            name="lan-auth-mode"
            @change="void updateLanAuthMode('public_read_only')"
          >
          <span>Public read-only</span>
        </label>
      </div>
      <p class="lan-share__hint">
        Changes apply to new MCP server starts. If the server is already running,
        restart it from AI Coding Integrations before sharing again.
      </p>

      <template v-if="!store.hosting">
        <label class="lan-share__label">
          Brain Name
          <input
            v-model="brainNameInput"
            type="text"
            class="lan-share__input"
            placeholder="e.g., HR Company Rules"
          />
        </label>
        <button
          class="lan-share__btn lan-share__btn--primary"
          :disabled="!brainNameInput.trim() || store.loading"
          @click="handleStartHosting"
        >
          {{ store.loading ? 'Starting…' : 'Start Sharing' }}
        </button>
      </template>

      <template v-else>
        <div class="lan-share__info">
          <div class="lan-share__info-row">
            <span class="lan-share__info-label">Status:</span>
            <span class="lan-share__badge lan-share__badge--active">Sharing</span>
          </div>
          <div class="lan-share__info-row">
            <span class="lan-share__info-label">Name:</span>
            <span>{{ store.hostBrainName }}</span>
          </div>
          <div class="lan-share__info-row">
            <span class="lan-share__info-label">Port:</span>
            <span>{{ store.hostPort }}</span>
          </div>
          <div class="lan-share__info-row">
            <span class="lan-share__info-label">Access:</span>
            <span>{{ store.hostAuthMode === 'public_read_only' ? 'Public read-only' : 'Token required' }}</span>
          </div>
          <div v-if="store.hostToken" class="lan-share__info-row">
            <span class="lan-share__info-label">Token:</span>
            <code class="lan-share__token" @click="copyToken">
              {{ tokenDisplay }}
            </code>
            <button class="lan-share__btn lan-share__btn--small" @click="copyToken">
              Copy
            </button>
          </div>
          <p class="lan-share__hint">
            {{ store.hostAuthMode === 'public_read_only'
              ? 'Anyone on your LAN can search this brain without a token. Only read-only MCP methods are exposed.'
              : 'Share this token with colleagues so they can connect to your brain.' }}
          </p>
        </div>
        <button class="lan-share__btn lan-share__btn--danger" @click="handleStopHosting">
          Stop Sharing
        </button>
      </template>
    </fieldset>

    <!-- Discovery Section -->
    <fieldset class="lan-share__section">
      <legend>Discover Shared Brains</legend>
      <p class="lan-share__desc">
        Find other TerranSoul instances sharing their brain on the network.
      </p>

      <div class="lan-share__actions">
        <button
          class="lan-share__btn"
          :disabled="store.loading"
          @click="handleDiscover"
        >
          {{ store.loading ? 'Scanning…' : 'Scan Network' }}
        </button>
        <button
          v-if="store.discovering"
          class="lan-share__btn lan-share__btn--secondary"
          @click="store.stopDiscovery()"
        >
          Stop
        </button>
      </div>

      <ul v-if="store.discovered.length" class="lan-share__list">
        <li
          v-for="brain in store.discovered"
          :key="`${brain.host}:${brain.port}`"
          class="lan-share__card"
        >
          <div class="lan-share__card-header">
            <strong>{{ brain.brain_name }}</strong>
            <span class="lan-share__badge" :class="brain.read_only ? 'lan-share__badge--ro' : 'lan-share__badge--rw'">
              {{ brain.read_only ? 'Read-only' : 'Read/Write' }}
            </span>
          </div>
          <div class="lan-share__card-details">
            <span>{{ brain.host }}:{{ brain.port }}</span>
            <span>{{ brain.memory_count }} memories</span>
            <span>{{ brain.provider }}</span>
            <span>{{ brain.token_required ? 'Token required' : 'Public read-only' }}</span>
          </div>
          <button
            class="lan-share__btn lan-share__btn--small"
            @click="handleConnectDiscovered(brain)"
          >
            Connect
          </button>
        </li>
      </ul>
      <p v-else-if="!store.loading && store.discovering" class="lan-share__empty">
        No brains found on the network yet…
      </p>
    </fieldset>

    <!-- Manual Connect Section -->
    <fieldset class="lan-share__section">
      <legend>Manual Connect</legend>
      <div class="lan-share__form-row">
        <label class="lan-share__label">
          Host
          <input v-model="connectHost" type="text" class="lan-share__input" placeholder="192.168.1.100" />
        </label>
        <label class="lan-share__label lan-share__label--small">
          Port
          <input v-model.number="connectPort" type="number" class="lan-share__input" placeholder="7421" />
        </label>
      </div>
      <label class="lan-share__label">
        Access Mode
        <select v-model="manualAccessMode" class="lan-share__input">
          <option value="token_required">Token required</option>
          <option value="public_read_only">Public read-only</option>
        </select>
      </label>
      <label v-if="manualAccessMode === 'token_required'" class="lan-share__label">
        Token
        <input v-model="connectToken" type="text" class="lan-share__input" placeholder="Bearer token from host" />
      </label>
      <button
        class="lan-share__btn"
        :disabled="!canConnect || store.loading"
        @click="handleManualConnect"
      >
        Connect
      </button>
    </fieldset>

    <!-- Connected Brains Section -->
    <fieldset v-if="store.connections.length" class="lan-share__section">
      <legend>Connected Brains ({{ store.connectedCount }})</legend>

      <ul class="lan-share__list">
        <li
          v-for="conn in store.connections"
          :key="conn.id"
          class="lan-share__card"
        >
          <div class="lan-share__card-header">
            <strong>{{ conn.brain_name }}</strong>
            <span class="lan-share__badge lan-share__badge--connected">Connected</span>
          </div>
          <div class="lan-share__card-details">
            <span>{{ conn.host }}:{{ conn.port }}</span>
          </div>
          <div class="lan-share__card-actions">
            <button
              class="lan-share__btn lan-share__btn--small"
              @click="handleSearchRemote(conn.id)"
            >
              Search
            </button>
            <button
              class="lan-share__btn lan-share__btn--small lan-share__btn--danger"
              @click="store.disconnect(conn.id)"
            >
              Disconnect
            </button>
          </div>
        </li>
      </ul>

      <!-- Search all brains -->
      <div class="lan-share__search">
        <label class="lan-share__label">
          Search All Connected Brains
          <input
            v-model="searchQuery"
            type="text"
            class="lan-share__input"
            placeholder="Ask a question…"
            @keydown.enter="handleSearchAll"
          />
        </label>
        <button
          class="lan-share__btn"
          :disabled="!searchQuery.trim()"
          @click="handleSearchAll"
        >
          Search
        </button>
      </div>

      <ul v-if="searchResults.length" class="lan-share__results">
        <li
          v-for="(r, idx) in searchResults"
          :key="idx"
          class="lan-share__result"
        >
          <div class="lan-share__result-header">
            <span class="lan-share__result-brain">{{ r.brain_name }}</span>
            <span class="lan-share__result-score">{{ (r.result.score * 100).toFixed(0) }}%</span>
          </div>
          <p class="lan-share__result-content">{{ r.result.content }}</p>
          <div v-if="r.result.tags" class="lan-share__result-tags">
            {{ r.result.tags }}
          </div>
        </li>
      </ul>
    </fieldset>
  </section>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue';
import { useLanShareStore, type DiscoveredBrain, type TaggedRemoteResult } from '../stores/lan-share';
import { useSettingsStore } from '../stores/settings';

const store = useLanShareStore();
const settingsStore = useSettingsStore();

// Host mode
const brainNameInput = ref('');

// Connect form
const connectHost = ref('');
const connectPort = ref(7421);
const connectToken = ref('');
const manualAccessMode = ref<'token_required' | 'public_read_only'>(
  settingsStore.settings.lan_auth_mode ?? 'token_required',
);

// Search
const searchQuery = ref('');
const searchResults = ref<TaggedRemoteResult[]>([]);

const tokenDisplay = computed(() => {
  const t = store.hostToken;
  if (!t) return '';
  if (t.length <= 12) return t;
  return `${t.slice(0, 6)}…${t.slice(-6)}`;
});

const hostAccessMode = computed(
  () => settingsStore.settings.lan_auth_mode ?? 'token_required',
);

const canConnect = computed(
  () => connectHost.value.trim()
    && connectPort.value > 0
    && (manualAccessMode.value === 'public_read_only' || connectToken.value.trim()),
);

async function updateLanAuthMode(mode: 'token_required' | 'public_read_only') {
  manualAccessMode.value = mode;
  await settingsStore.saveSettings({ lan_auth_mode: mode });
}

async function handleStartHosting() {
  await store.startHosting(brainNameInput.value.trim());
}

async function handleStopHosting() {
  await store.stopHosting();
}

async function handleDiscover() {
  await store.startDiscovery();
}

async function handleConnectDiscovered(brain: DiscoveredBrain) {
  connectHost.value = brain.host;
  connectPort.value = brain.port;
  manualAccessMode.value = brain.token_required ? 'token_required' : 'public_read_only';
}

async function handleManualConnect() {
  await store.connect(
    connectHost.value.trim(),
    connectPort.value,
    manualAccessMode.value === 'token_required' ? connectToken.value.trim() : null,
    manualAccessMode.value === 'token_required',
  );
  // Clear form on success.
  if (!store.error) {
    connectHost.value = '';
    connectPort.value = 7421;
    connectToken.value = '';
    manualAccessMode.value = hostAccessMode.value;
  }
}

async function handleSearchRemote(connectionId: string) {
  const query = prompt('Search query:');
  if (!query) return;
  const results = await store.searchRemote(connectionId, query);
  searchResults.value = results.map((r) => ({
    connection_id: connectionId,
    brain_name:
      store.connections.find((c) => c.id === connectionId)?.brain_name ?? 'Remote',
    result: r,
  }));
}

async function handleSearchAll() {
  if (!searchQuery.value.trim()) return;
  searchResults.value = await store.searchAll(searchQuery.value.trim());
}

function copyToken() {
  if (store.hostToken) {
    navigator.clipboard.writeText(store.hostToken);
  }
}
</script>

<style scoped>
.lan-share {
  padding: 1rem;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.lan-share__title {
  font-size: 1.25rem;
  font-weight: 600;
  color: var(--ts-text-primary);
  margin: 0;
}

.lan-share__error {
  padding: 0.5rem 0.75rem;
  background: var(--ts-danger-bg, #fdd);
  color: var(--ts-danger-text, #c00);
  border-radius: 6px;
  font-size: 0.85rem;
}

.lan-share__section {
  border: 1px solid var(--ts-border-subtle);
  border-radius: 8px;
  padding: 1rem;
}

.lan-share__section legend {
  font-weight: 600;
  color: var(--ts-text-primary);
  padding: 0 0.5rem;
}

.lan-share__desc {
  font-size: 0.85rem;
  color: var(--ts-text-secondary);
  margin: 0 0 0.75rem;
}

.lan-share__label {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  font-size: 0.85rem;
  color: var(--ts-text-secondary);
  margin-bottom: 0.5rem;
}

.lan-share__label-text {
  font-size: 0.85rem;
  color: var(--ts-text-secondary);
}

.lan-share__auth-mode {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  margin-bottom: 0.5rem;
}

.lan-share__choice {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  color: var(--ts-text-primary);
  font-size: 0.9rem;
}

.lan-share__label--small {
  max-width: 80px;
}

.lan-share__input {
  padding: 0.4rem 0.6rem;
  border: 1px solid var(--ts-border-subtle);
  border-radius: 4px;
  background: var(--ts-bg-input);
  color: var(--ts-text-primary);
  font-size: 0.9rem;
}

.lan-share__btn {
  padding: 0.4rem 0.8rem;
  border-radius: 4px;
  border: 1px solid var(--ts-border-subtle);
  background: var(--ts-bg-button);
  color: var(--ts-text-primary);
  cursor: pointer;
  font-size: 0.85rem;
  transition: background 0.15s;
}

.lan-share__btn:hover:not(:disabled) {
  background: var(--ts-bg-button-hover);
}

.lan-share__btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.lan-share__btn--primary {
  background: var(--ts-accent);
  color: var(--ts-accent-text);
  border-color: var(--ts-accent);
}

.lan-share__btn--danger {
  background: var(--ts-danger-bg, #fdd);
  color: var(--ts-danger-text, #c00);
  border-color: var(--ts-danger-border, #fcc);
}

.lan-share__btn--secondary {
  background: transparent;
}

.lan-share__btn--small {
  padding: 0.25rem 0.5rem;
  font-size: 0.8rem;
}

.lan-share__actions {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 0.75rem;
}

.lan-share__list {
  list-style: none;
  padding: 0;
  margin: 0.75rem 0;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.lan-share__card {
  border: 1px solid var(--ts-border-subtle);
  border-radius: 6px;
  padding: 0.75rem;
}

.lan-share__card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 0.25rem;
}

.lan-share__card-details {
  display: flex;
  gap: 0.75rem;
  font-size: 0.8rem;
  color: var(--ts-text-secondary);
  margin-bottom: 0.5rem;
}

.lan-share__card-actions {
  display: flex;
  gap: 0.5rem;
}

.lan-share__badge {
  font-size: 0.75rem;
  padding: 0.15rem 0.4rem;
  border-radius: 4px;
  font-weight: 500;
}

.lan-share__badge--active {
  background: var(--ts-success-bg, #dfd);
  color: var(--ts-success-text, #070);
}

.lan-share__badge--connected {
  background: var(--ts-accent-bg, #def);
  color: var(--ts-accent, #07c);
}

.lan-share__badge--ro {
  background: var(--ts-warning-bg, #ffd);
  color: var(--ts-warning-text, #a80);
}

.lan-share__badge--rw {
  background: var(--ts-success-bg, #dfd);
  color: var(--ts-success-text, #070);
}

.lan-share__info {
  margin: 0.5rem 0;
}

.lan-share__info-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 0.25rem;
  font-size: 0.85rem;
}

.lan-share__info-label {
  font-weight: 500;
  color: var(--ts-text-secondary);
  min-width: 4rem;
}

.lan-share__token {
  font-family: monospace;
  background: var(--ts-bg-code);
  padding: 0.2rem 0.4rem;
  border-radius: 3px;
  cursor: pointer;
  font-size: 0.8rem;
}

.lan-share__hint {
  font-size: 0.8rem;
  color: var(--ts-text-tertiary);
  margin: 0.5rem 0 0;
}

.lan-share__form-row {
  display: flex;
  gap: 0.75rem;
}

.lan-share__empty {
  font-size: 0.85rem;
  color: var(--ts-text-tertiary);
  text-align: center;
  padding: 1rem;
}

.lan-share__search {
  margin-top: 0.75rem;
  padding-top: 0.75rem;
  border-top: 1px solid var(--ts-border-subtle);
}

.lan-share__results {
  list-style: none;
  padding: 0;
  margin: 0.75rem 0 0;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.lan-share__result {
  border: 1px solid var(--ts-border-subtle);
  border-radius: 6px;
  padding: 0.6rem;
}

.lan-share__result-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.25rem;
}

.lan-share__result-brain {
  font-size: 0.8rem;
  font-weight: 500;
  color: var(--ts-accent);
}

.lan-share__result-score {
  font-size: 0.75rem;
  color: var(--ts-text-tertiary);
}

.lan-share__result-content {
  font-size: 0.85rem;
  margin: 0;
  color: var(--ts-text-primary);
  line-height: 1.4;
}

.lan-share__result-tags {
  margin-top: 0.25rem;
  font-size: 0.75rem;
  color: var(--ts-text-tertiary);
}
</style>
