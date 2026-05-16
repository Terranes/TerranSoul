<script setup lang="ts">
/**
 * CompanionsPanel
 * ----------------
 * Marketplace surface for the companion AI ecosystem registered in
 * `src-tauri/src/integrations/companions.rs` (chunk INTEGRATE-1). The same
 * registry is also the data source for the chat-driven
 * `companion-ecosystem` quest (chunk INTEGRATE-5): both surfaces call
 * `companions_list`, render Install buttons, and only ever invoke
 * `companions_run_guided_install` after an explicit user click. No
 * background scanning, no silent installs.
 *
 * Detect calls are explicit-click only and surface the result as a status
 * pill next to each card.
 *
 * Temporal.io is shown as a "design reference only" pill, per the
 * 2026-05-14 user reminder, with NO Install button — it is documented as
 * an inspiration, not an integration.
 */
import { ref, onMounted, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';

interface ShellCommand {
  program: string;
  args: string[];
  description: string;
}

interface CompanionApp {
  id: string;
  display_name: string;
  role: string;
  official_url: string;
  windows_install: ShellCommand | null;
  macos_install: ShellCommand | null;
  linux_install: ShellCommand | null;
  detect: ShellCommand | null;
  requires_elevation: boolean;
  github_repo: string | null;
}

type DetectStatus =
  | { type: 'installed'; version: string }
  | { type: 'not_installed' }
  | { type: 'unknown'; reason: string }
  | null;

interface LatestReleaseInfo {
  version: string;
  tag: string;
  html_url: string;
}

interface UpdateCheckResult {
  id: string;
  installed_version: string | null;
  latest: LatestReleaseInfo | null;
  update_available: boolean;
  note: string | null;
}

const companions = ref<CompanionApp[]>([]);
const detectStatus = ref<Record<string, DetectStatus>>({});
const updateStatus = ref<Record<string, UpdateCheckResult | null>>({});
const lastAction = ref<Record<string, string>>({});
const loadError = ref<string | null>(null);
const isLoading = ref(false);
const busyCompanionId = ref<string | null>(null);

// Static "design reference only" entries — never installable, no command call.
const designReferenceOnly = [
  {
    id: 'temporal-io',
    display_name: 'Temporal.io',
    role: 'workflow-engine (design reference only)',
    official_url: 'https://temporal.io',
    note: 'Cited in docs/coding-workflow-design.md as the durable-history pattern TerranSoul\'s own runner is inspired by. NOT an installable integration.',
  },
];

const totalSubQuests = computed(() => companions.value.length + designReferenceOnly.length);

async function loadCompanions(): Promise<void> {
  isLoading.value = true;
  loadError.value = null;
  try {
    companions.value = await invoke<CompanionApp[]>('companions_list');
  } catch (err) {
    loadError.value = err instanceof Error ? err.message : String(err);
  } finally {
    isLoading.value = false;
  }
}

async function detect(id: string): Promise<void> {
  busyCompanionId.value = id;
  try {
    const status = await invoke<unknown>('companions_detect_one', { id });
    detectStatus.value[id] = normaliseDetectStatus(status);
    lastAction.value[id] = 'Detect complete';
  } catch (err) {
    detectStatus.value[id] = {
      type: 'unknown',
      reason: err instanceof Error ? err.message : String(err),
    };
    lastAction.value[id] = 'Detect failed';
  } finally {
    busyCompanionId.value = null;
  }
  // Update check piggy-backs on Detect so the user always sees both the
  // installed pill and the "Update available" badge after one click.
  void checkUpdate(id);
}

async function checkUpdate(id: string): Promise<void> {
  try {
    const result = await invoke<UpdateCheckResult>('companions_check_update', { id });
    updateStatus.value[id] = result;
    // Mirror the installed version into detectStatus so the pill stays
    // accurate even if the user only clicked "Check for updates".
    if (result.installed_version && !detectStatus.value[id]) {
      detectStatus.value[id] = { type: 'installed', version: result.installed_version };
    }
  } catch (err) {
    updateStatus.value[id] = {
      id,
      installed_version: null,
      latest: null,
      update_available: false,
      note: `Update check failed: ${err instanceof Error ? err.message : String(err)}`,
    };
  }
}

async function install(id: string): Promise<void> {
  busyCompanionId.value = id;
  lastAction.value[id] = 'Awaiting OS elevation prompt…';
  try {
    const outcome = await invoke<unknown>('companions_run_guided_install', { id });
    lastAction.value[id] = describeOutcome(outcome);
  } catch (err) {
    lastAction.value[id] = `Install error: ${err instanceof Error ? err.message : String(err)}`;
  } finally {
    busyCompanionId.value = null;
  }
}

async function openOfficialPage(id: string): Promise<void> {
  busyCompanionId.value = id;
  try {
    await invoke('companions_open_install_page', { id });
    lastAction.value[id] = 'Opened official page in browser';
  } catch (err) {
    lastAction.value[id] = `Open failed: ${err instanceof Error ? err.message : String(err)}`;
  } finally {
    busyCompanionId.value = null;
  }
}

function normaliseDetectStatus(raw: unknown): DetectStatus {
  // Rust serde tag for unit variants is a bare string ("NotInstalled");
  // tuple/struct variants come through as { VariantName: ... }.
  if (raw === 'NotInstalled') return { type: 'not_installed' };
  if (!raw || typeof raw !== 'object') return null;
  const obj = raw as Record<string, unknown>;
  if ('Installed' in obj) {
    const inner = obj.Installed as { version?: string };
    return { type: 'installed', version: inner?.version ?? '' };
  }
  if ('NotInstalled' in obj) {
    return { type: 'not_installed' };
  }
  if ('Unknown' in obj) {
    const inner = obj.Unknown as { reason?: string };
    return { type: 'unknown', reason: inner?.reason ?? 'Unknown' };
  }
  return null;
}

function describeOutcome(raw: unknown): string {
  if (!raw || typeof raw !== 'object') return 'Install dispatched';
  const obj = raw as Record<string, unknown>;
  if ('RequiresElevation' in obj) {
    return 'OS elevation prompt opened — confirm UAC / sudo to continue';
  }
  if ('DirectInstall' in obj) {
    return 'Installer spawned in a visible terminal';
  }
  if ('NoInstallerForOs' in obj) {
    const inner = obj.NoInstallerForOs as { os?: string };
    return `No installer for ${inner?.os ?? 'this OS'} — open the official page instead`;
  }
  if ('Unknown' in obj) {
    const inner = obj.Unknown as { id?: string };
    return `Unknown companion id: ${inner?.id ?? '?'}`;
  }
  return 'Install dispatched';
}

function installedVersion(id: string): string {
  const status = detectStatus.value[id];
  return status && status.type === 'installed' ? status.version : '';
}

function updateAvailable(id: string): boolean {
  return updateStatus.value[id]?.update_available === true;
}

function latestVersion(id: string): string {
  return updateStatus.value[id]?.latest?.version ?? '';
}

function updateNote(id: string): string {
  return updateStatus.value[id]?.note ?? '';
}

onMounted(() => {
  void loadCompanions();
});

defineExpose({ loadCompanions, detect, install, openOfficialPage, checkUpdate });
</script>

<template>
  <section
    class="companions-panel"
    data-testid="companions-panel"
  >
    <header class="cp-header">
      <h3>Companion AI Ecosystem</h3>
      <p class="cp-sub">
        {{ totalSubQuests }} sub-quests — explicit-click installers with OS-level
        elevation. Never silent. Detection runs only when you click Detect.
      </p>
    </header>

    <p
      v-if="loadError"
      class="cp-error"
      data-testid="companions-load-error"
    >
      {{ loadError }}
    </p>

    <p
      v-if="isLoading"
      class="cp-loading"
    >
      Loading companions…
    </p>

    <ul
      v-else
      class="cp-list"
    >
      <li
        v-for="app in companions"
        :key="app.id"
        class="cp-card"
        :data-testid="`companion-card-${app.id}`"
      >
        <header class="cp-card-head">
          <strong>{{ app.display_name }}</strong>
          <span
            v-if="updateAvailable(app.id)"
            class="cp-badge cp-badge-update"
            :data-testid="`companion-update-badge-${app.id}`"
          >
            update available → v{{ latestVersion(app.id) }}
          </span>
          <span
            v-if="app.requires_elevation"
            class="cp-badge cp-badge-elev"
          >
            requires elevation
          </span>
        </header>
        <p class="cp-role">
          {{ app.role }}
        </p>

        <div
          class="cp-status"
          :data-testid="`companion-status-${app.id}`"
        >
          <template v-if="detectStatus[app.id]?.type === 'installed'">
            ✓ Installed (v{{ installedVersion(app.id) }})
          </template>
          <template v-else-if="detectStatus[app.id]?.type === 'not_installed'">
            • Not installed
          </template>
          <template v-else-if="detectStatus[app.id]?.type === 'unknown'">
            ? Status unknown
          </template>
          <template v-else>
            ? Not detected yet
          </template>
        </div>

        <div class="cp-actions">
          <button
            type="button"
            class="cp-btn cp-btn-secondary"
            :disabled="busyCompanionId === app.id"
            :data-testid="`companion-detect-${app.id}`"
            @click="detect(app.id)"
          >
            Detect
          </button>
          <button
            v-if="updateAvailable(app.id)"
            type="button"
            class="cp-btn cp-btn-primary"
            :disabled="busyCompanionId === app.id"
            :data-testid="`companion-update-${app.id}`"
            @click="install(app.id)"
          >
            Update to v{{ latestVersion(app.id) }}
          </button>
          <button
            v-else
            type="button"
            class="cp-btn cp-btn-primary"
            :disabled="busyCompanionId === app.id"
            :data-testid="`companion-install-${app.id}`"
            @click="install(app.id)"
          >
            Install
          </button>
          <button
            type="button"
            class="cp-btn cp-btn-link"
            :disabled="busyCompanionId === app.id"
            :data-testid="`companion-open-${app.id}`"
            @click="openOfficialPage(app.id)"
          >
            Open page
          </button>
        </div>

        <p
          v-if="updateNote(app.id) && !updateAvailable(app.id)"
          class="cp-action-line cp-action-line-muted"
          :data-testid="`companion-update-note-${app.id}`"
        >
          {{ updateNote(app.id) }}
        </p>

        <p
          v-if="lastAction[app.id]"
          class="cp-action-line"
          :data-testid="`companion-action-${app.id}`"
        >
          {{ lastAction[app.id] }}
        </p>
      </li>

      <li
        v-for="ref_app in designReferenceOnly"
        :key="ref_app.id"
        class="cp-card cp-card-ref"
        :data-testid="`companion-card-${ref_app.id}`"
      >
        <header class="cp-card-head">
          <strong>{{ ref_app.display_name }}</strong>
          <span class="cp-badge cp-badge-ref">design reference only</span>
        </header>
        <p class="cp-role">
          {{ ref_app.role }}
        </p>
        <p class="cp-note">
          {{ ref_app.note }}
        </p>
      </li>
    </ul>
  </section>
</template>

<style scoped>
.companions-panel {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  padding: 0.75rem;
}
.cp-header h3 {
  margin: 0 0 0.25rem 0;
}
.cp-sub {
  margin: 0;
  color: var(--ts-text-muted);
  font-size: 0.875rem;
}
.cp-error {
  color: var(--ts-error, #d33);
}
.cp-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: grid;
  gap: 0.5rem;
}
.cp-card {
  border: 1px solid var(--ts-border);
  border-radius: 0.5rem;
  padding: 0.75rem;
  background: var(--ts-surface);
}
.cp-card-ref {
  background: var(--ts-surface-muted, var(--ts-surface));
}
.cp-card-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
}
.cp-role {
  margin: 0.25rem 0 0.5rem 0;
  color: var(--ts-text-muted);
  font-size: 0.875rem;
}
.cp-note {
  margin: 0.25rem 0 0 0;
  font-size: 0.8125rem;
  font-style: italic;
  color: var(--ts-text-muted);
}
.cp-status {
  margin-bottom: 0.5rem;
  font-size: 0.875rem;
}
.cp-actions {
  display: flex;
  gap: 0.5rem;
  flex-wrap: wrap;
}
.cp-btn {
  padding: 0.375rem 0.75rem;
  border-radius: 0.375rem;
  border: 1px solid var(--ts-border);
  background: var(--ts-surface);
  cursor: pointer;
  font-size: 0.875rem;
}
.cp-btn:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}
.cp-btn-primary {
  background: var(--ts-accent);
  color: var(--ts-on-accent, white);
  border-color: var(--ts-accent);
}
.cp-btn-link {
  background: transparent;
  border-color: transparent;
  text-decoration: underline;
}
.cp-action-line {
  margin: 0.5rem 0 0 0;
  font-size: 0.8125rem;
  color: var(--ts-text-muted);
}
.cp-badge {
  font-size: 0.75rem;
  padding: 0.125rem 0.375rem;
  border-radius: 999px;
  border: 1px solid var(--ts-border);
}
.cp-badge-elev {
  border-color: var(--ts-warning, #c80);
  color: var(--ts-warning, #c80);
}
.cp-badge-update {
  border-color: var(--ts-accent, #5b8def);
  color: var(--ts-accent, #5b8def);
  background: color-mix(in srgb, var(--ts-accent, #5b8def) 12%, transparent);
}
.cp-action-line-muted {
  font-style: italic;
}
.cp-badge-ref {
  border-color: var(--ts-text-muted);
  color: var(--ts-text-muted);
}
</style>
