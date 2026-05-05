<template>
  <section
    class="ai-integrations-view"
    data-testid="ai-integrations-view"
  >
    <header class="aiv-header">
      <h3>🔌 AI Coding Integrations</h3>
      <button
        class="aiv-link"
        data-testid="aiv-refresh"
        :disabled="store.loading"
        @click="onRefreshAll"
      >
        {{ store.loading ? 'Refreshing…' : 'Refresh' }}
      </button>
    </header>

    <p class="aiv-help">
      Expose TerranSoul's brain to external AI coding assistants
      (VS Code Copilot, Claude Desktop, Codex CLI) over the
      <a
        href="https://modelcontextprotocol.io"
        target="_blank"
        rel="noopener"
      >Model Context Protocol</a>.
      The MCP server runs on <code>127.0.0.1</code> only — never exposed to LAN.
    </p>

    <!-- ── Server status ─────────────────────────────────────────────── -->
    <div
      class="aiv-card"
      data-testid="aiv-server-card"
    >
      <h4>Server status</h4>
      <p class="aiv-row">
        <span
          :class="['aiv-pill', store.isRunning ? 'aiv-pill-on' : 'aiv-pill-off']"
          data-testid="aiv-server-pill"
        >{{ store.isRunning ? 'Running' : 'Stopped' }}</span>
        <span
          v-if="store.port"
          class="aiv-meta"
          data-testid="aiv-server-port"
        >port <code>{{ store.port }}</code></span>
        <span
          v-if="store.serverStatus?.is_dev"
          class="aiv-meta aiv-meta-dev"
        >dev build</span>
      </p>
      <p
        v-if="store.tokenPreview"
        class="aiv-row aiv-token-row"
      >
        <span>token:</span>
        <code data-testid="aiv-token-preview">{{ store.tokenPreview }}</code>
        <button
          class="aiv-link"
          data-testid="aiv-copy-token"
          @click="onCopyToken"
        >
          Copy full
        </button>
        <span
          v-if="copyMessage"
          class="aiv-meta aiv-meta-ok"
          data-testid="aiv-copy-msg"
        >{{ copyMessage }}</span>
      </p>
      <div class="aiv-actions">
        <button
          v-if="!store.isRunning"
          class="aiv-btn aiv-btn-primary"
          :disabled="store.loading"
          data-testid="aiv-start"
          @click="store.startServer()"
        >
          Start server
        </button>
        <button
          v-else
          class="aiv-btn"
          :disabled="store.loading"
          data-testid="aiv-stop"
          @click="store.stopServer()"
        >
          Stop server
        </button>
        <button
          class="aiv-btn"
          :disabled="store.loading"
          data-testid="aiv-regen-token"
          @click="onRegenToken"
        >
          Regenerate token
        </button>
      </div>
    </div>

    <!-- ── Auto-setup writers ────────────────────────────────────────── -->
    <div
      class="aiv-card"
      data-testid="aiv-clients-card"
    >
      <h4>External clients</h4>
      <p class="aiv-help-small">
        TerranSoul writes the integration config for each editor.
        Restart the editor after setup. Workspace path:
        <code>{{ workspaceRoot }}</code>.
      </p>
      <div class="aiv-row aiv-transport-row">
        <span>Transport:</span>
        <label>
          <input
            type="radio"
            :checked="store.preferredTransport === 'stdio'"
            data-testid="aiv-transport-stdio"
            @change="store.setTransport('stdio')"
          >
          stdio (recommended)
        </label>
        <label>
          <input
            type="radio"
            :checked="store.preferredTransport === 'http'"
            data-testid="aiv-transport-http"
            @change="store.setTransport('http')"
          >
          http
        </label>
      </div>
      <ul
        v-if="store.clientStatuses.length > 0"
        class="aiv-client-list"
        data-testid="aiv-client-list"
      >
        <li
          v-for="c in store.clientStatuses"
          :key="c.client"
          class="aiv-client-item"
          :data-testid="`aiv-client-${clientKey(c.client)}`"
        >
          <div class="aiv-client-head">
            <strong>{{ c.client }}</strong>
            <span
              :class="['aiv-pill', c.configured ? 'aiv-pill-on' : 'aiv-pill-off']"
              :data-testid="`aiv-client-status-${clientKey(c.client)}`"
            >
              {{ c.configured ? 'Configured' : 'Not configured' }}
            </span>
          </div>
          <p
            v-if="c.config_path"
            class="aiv-meta"
          >
            <code>{{ c.config_path }}</code>
          </p>
          <div class="aiv-actions">
            <button
              v-if="!c.configured"
              class="aiv-btn aiv-btn-primary"
              :data-testid="`aiv-setup-${clientKey(c.client)}`"
              @click="onSetup(clientKey(c.client))"
            >
              Set up via {{ store.preferredTransport }}
            </button>
            <button
              v-else
              class="aiv-btn aiv-btn-danger"
              :data-testid="`aiv-remove-${clientKey(c.client)}`"
              @click="onRemove(clientKey(c.client))"
            >
              Remove
            </button>
          </div>
        </li>
      </ul>
      <p
        v-if="lastSetupMessage"
        class="aiv-row aiv-meta-ok"
        data-testid="aiv-setup-msg"
      >
        ✓ {{ lastSetupMessage }}
      </p>
    </div>

    <!-- ── VS Code workspaces ────────────────────────────────────────── -->
    <div
      v-if="store.vscodeWindows.length > 0"
      class="aiv-card"
      data-testid="aiv-windows-card"
    >
      <h4>Known VS Code windows</h4>
      <ul class="aiv-window-list">
        <li
          v-for="w in store.vscodeWindows"
          :key="w.pid"
          class="aiv-window-item"
        >
          <span><code>{{ w.root }}</code></span>
          <span class="aiv-meta">pid {{ w.pid }}</span>
          <button
            class="aiv-link"
            :data-testid="`aiv-forget-${w.pid}`"
            @click="store.forgetWindow(w.pid)"
          >
            Forget
          </button>
        </li>
      </ul>
    </div>

    <!-- ── LAN exposure (locked) ─────────────────────────────────────── -->
    <div
      class="aiv-card aiv-card-warn"
      data-testid="aiv-lan-card"
    >
      <h4>Network exposure</h4>
      <p>
        <strong>LAN exposure is disabled.</strong>
        TerranSoul binds the MCP server to <code>127.0.0.1</code> only.
        Allowing LAN access would let any device on your network read your
        memories — a future release may add an opt-in toggle with a TLS
        certificate, but it is intentionally not available today.
      </p>
    </div>

    <div
      class="aiv-card aiv-card-accent"
      data-testid="aiv-workbench-card"
    >
      <h4>Native code workbench</h4>
      <p class="aiv-help-small">
        TerranSoul keeps code intelligence clean-room and local-first. The target
        UX is a dense coding cockpit: graph canvas, grounded citations, visible
        tool activity, repo status, and blast-radius awareness inside the app.
      </p>
      <div class="aiv-workbench-grid">
        <div class="aiv-workbench-item">
          <strong>Graph-first navigation</strong>
          <span>Browse structure before grepping files.</span>
        </div>
        <div class="aiv-workbench-item">
          <strong>Grounded context</strong>
          <span>Tool cards, code citations, and symbol context in one place.</span>
        </div>
        <div class="aiv-workbench-item">
          <strong>Risk visibility</strong>
          <span>Impact probes and rename planning before edits land.</span>
        </div>
      </div>
      <p class="aiv-meta">
        License boundary: public GitNexus behavior can inspire UX direction, but
        TerranSoul ships only native Rust/Vue implementation and does not bundle
        external GitNexus binaries or assets.
      </p>
    </div>

    <p
      v-if="store.error"
      class="aiv-error"
      data-testid="aiv-error"
    >
      {{ store.error }}
    </p>
  </section>
</template>

<script setup lang="ts">
/**
 * AICodingIntegrationsView (Chunk 15.4).
 *
 * Control panel for the MCP brain gateway: server start/stop, token
 * regeneration, per-editor auto-setup buttons, VS Code window
 * registry, and a locked LAN-exposure notice.
 */
import { computed, onMounted, ref } from 'vue';
import { useAiIntegrationsStore } from '../stores/ai-integrations';

const store = useAiIntegrationsStore();

/**
 * Workspace root passed to `list_mcp_clients` and `setup_vscode_mcp`.
 *
 * In a desktop app we'd resolve this from a "current workspace"
 * concept; for now we use the user's home dir as a reasonable default
 * and let consumers override via prop in tests.
 */
const props = defineProps<{
  /** Override for the workspace root (mostly for tests). */
  workspaceRoot?: string;
}>();
const workspaceRoot = computed(() => props.workspaceRoot ?? '.');

const copyMessage = ref<string | null>(null);
const lastSetupMessage = ref<string | null>(null);

/** Map a backend `ClientStatus.client` label to the wire client id. */
function clientKey(label: string): 'vscode' | 'claude' | 'codex' {
  const lower = label.toLowerCase();
  if (lower.includes('claude')) return 'claude';
  if (lower.includes('codex')) return 'codex';
  return 'vscode';
}

async function onRefreshAll() {
  await Promise.all([
    store.refreshStatus(),
    store.refreshClients(workspaceRoot.value),
    store.refreshVscodeWindows(),
  ]);
}

async function onCopyToken() {
  const t = store.serverStatus?.token;
  if (!t) return;
  try {
    await navigator.clipboard.writeText(t);
    copyMessage.value = 'copied';
    setTimeout(() => {
      copyMessage.value = null;
    }, 2000);
  } catch {
    copyMessage.value = 'copy failed';
  }
}

async function onRegenToken() {
  if (
    !confirm(
      'Regenerate the MCP bearer token? Connected clients will need to be re-set-up.',
    )
  ) {
    return;
  }
  await store.regenerateToken();
}

async function onSetup(client: 'vscode' | 'claude' | 'codex') {
  lastSetupMessage.value = null;
  const result = await store.setupClient(client, workspaceRoot.value);
  if (result) {
    lastSetupMessage.value = `${client}: ${result.message ?? 'configured'}`;
  }
}

async function onRemove(client: 'vscode' | 'claude' | 'codex') {
  lastSetupMessage.value = null;
  const result = await store.removeClient(client, workspaceRoot.value);
  if (result) {
    lastSetupMessage.value = `${client}: ${result.message ?? 'removed'}`;
  }
}

onMounted(() => {
  onRefreshAll().catch((e) => {
    console.warn('[AICodingIntegrationsView] initial refresh failed:', e);
  });
});

defineExpose({ clientKey });
</script>

<style scoped>
.ai-integrations-view {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.aiv-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin: 0;
}
.aiv-header h3 {
  margin: 0;
}
.aiv-help {
  margin: 0;
  font-size: 0.9rem;
  color: var(--ts-text-muted, #aaa);
}
.aiv-help-small {
  margin: 0 0 8px;
  font-size: 0.85rem;
  color: var(--ts-text-muted, #888);
}
.aiv-card {
  background: var(--ts-card-bg, #1a1a1a);
  border: 1px solid var(--ts-card-border, #333);
  border-radius: 8px;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.aiv-card h4 {
  margin: 0;
  font-size: 0.95rem;
}
.aiv-card-warn {
  border-color: var(--ts-warning, #f0c040);
  background: var(--ts-warning-bg, rgba(240, 192, 64, 0.05));
}
.aiv-card-accent {
  border-color: var(--ts-link, #6db3ff);
  background:
    linear-gradient(135deg, rgba(109, 179, 255, 0.12), rgba(109, 179, 255, 0.02)),
    var(--ts-card-bg, #1a1a1a);
}
.aiv-row {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
  margin: 0;
  font-size: 0.9rem;
}
.aiv-token-row code {
  font-family: var(--ts-mono, monospace);
}
.aiv-meta {
  color: var(--ts-text-muted, #888);
  font-size: 0.85rem;
}
.aiv-meta-dev {
  color: var(--ts-warning, #f0c040);
}
.aiv-meta-ok {
  color: var(--ts-success, #4caf50);
}
.aiv-pill {
  font-size: 0.75rem;
  padding: 2px 8px;
  border-radius: 999px;
  background: var(--ts-card-bg-elev, #2a2a2a);
}
.aiv-pill-on {
  background: var(--ts-success-bg, rgba(76, 175, 80, 0.18));
  color: var(--ts-success, #4caf50);
}
.aiv-pill-off {
  background: var(--ts-card-bg-elev, #2a2a2a);
  color: var(--ts-text-muted, #888);
}
.aiv-actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}
.aiv-btn {
  padding: 6px 12px;
  border-radius: 6px;
  border: 1px solid var(--ts-card-border, #333);
  background: var(--ts-card-bg, #1a1a1a);
  color: inherit;
  cursor: pointer;
  font-size: 0.85rem;
}
.aiv-btn:hover:not(:disabled) {
  background: var(--ts-card-bg-hover, #2a2a2a);
}
.aiv-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.aiv-btn-primary {
  border-color: var(--ts-link, #6db3ff);
  color: var(--ts-link, #6db3ff);
}
.aiv-btn-danger {
  border-color: var(--ts-error, #f55);
  color: var(--ts-error, #f55);
}
.aiv-link {
  background: none;
  border: none;
  color: var(--ts-link, #6db3ff);
  cursor: pointer;
  font-size: inherit;
  padding: 0;
  text-decoration: underline;
}
.aiv-transport-row label {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}
.aiv-client-list,
.aiv-window-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.aiv-client-item {
  border: 1px solid var(--ts-card-border, #333);
  border-radius: 6px;
  padding: 8px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.aiv-client-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.aiv-window-item {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
  font-size: 0.85rem;
}
.aiv-error {
  margin: 0;
  font-size: 0.85rem;
  color: var(--ts-error, #f55);
}
.aiv-workbench-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 8px;
}
.aiv-workbench-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 10px;
  border-radius: 6px;
  border: 1px solid var(--ts-card-border, #333);
  background: rgba(255, 255, 255, 0.02);
  font-size: 0.85rem;
}
.aiv-workbench-item span {
  color: var(--ts-text-muted, #888);
}
</style>
