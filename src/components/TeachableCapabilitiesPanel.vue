<template>
  <section
    class="tc-panel"
    aria-labelledby="tc-panel-title"
  >
    <header class="tc-header">
      <div class="tc-title-row">
        <span
          class="tc-icon"
          aria-hidden="true"
        >⚙</span>
        <h2
          id="tc-panel-title"
          class="tc-title"
        >
          Teachable Capabilities
        </h2>
        <span class="tc-pill">
          {{ store.summary.enabled }} / {{ store.summary.total }} enabled
        </span>
      </div>

      <p class="tc-subtitle">
        Tune companion features, test them from this panel, and promote proven
        settings into bundled source defaults through the workflow runner.
      </p>

      <div class="tc-summary">
        <div class="tc-summary-cell">
          <strong>{{ store.summary.untested }}</strong>
          <span>Untested</span>
        </div>
        <div class="tc-summary-cell tc-summary-learning">
          <strong>{{ store.summary.learning }}</strong>
          <span>Learning</span>
        </div>
        <div class="tc-summary-cell tc-summary-proven">
          <strong>{{ store.summary.proven }}</strong>
          <span>Proven</span>
        </div>
        <div class="tc-summary-cell tc-summary-canon">
          <strong>{{ store.summary.canon }}</strong>
          <span>Canon</span>
        </div>
      </div>

      <nav
        class="tc-tabs"
        role="tablist"
      >
        <button
          v-for="entry in store.categoriesWithCounts"
          :key="entry.category"
          type="button"
          role="tab"
          :aria-selected="activeCategory === entry.category"
          :class="['tc-tab', { active: activeCategory === entry.category }]"
          @click="activeCategory = entry.category"
        >
          <span>{{ entry.label }}</span>
          <small>{{ entry.count }}</small>
        </button>
      </nav>
    </header>

    <div class="tc-body">
      <p
        v-if="store.loading"
        class="tc-status"
      >
        Loading capabilities...
      </p>
      <p
        v-else-if="visibleCapabilities.length === 0"
        class="tc-status"
      >
        No capabilities in this category yet.
      </p>

      <ul
        v-else
        class="tc-list"
      >
        <li
          v-for="capability in visibleCapabilities"
          :key="capability.id"
          class="tc-card"
        >
          <div class="tc-card-head">
            <div class="tc-card-title">
              <strong>{{ capability.display_name }}</strong>
              <span>{{ capability.summary }}</span>
            </div>
            <label class="tc-toggle">
              <input
                type="checkbox"
                :checked="capability.enabled"
                @change="onToggle(capability, ($event.target as HTMLInputElement).checked)"
              >
              <span>{{ capability.enabled ? 'On' : 'Off' }}</span>
            </label>
          </div>

          <div class="tc-meta-row">
            <span
              class="tc-maturity"
              :style="maturityStyle(capability)"
            >
              {{ capabilityMaturityLabel(maturityOf(capability)) }}
            </span>
            <span>Used {{ capability.usage_count }}x</span>
            <span v-if="capability.last_used_at > 0">Last {{ formatRelative(capability.last_used_at) }}</span>
            <span v-if="capability.rating_count > 0">
              Rating {{ avgCapabilityRating(capability).toFixed(1) }} ({{ capability.rating_count }})
            </span>
          </div>

          <details class="tc-config">
            <summary>Configuration</summary>

            <div
              v-if="isJsonOnly(capability)"
              class="tc-json-editor"
            >
              <label>
                <span>JSON config</span>
                <textarea
                  :value="jsonDrafts[capability.id] ?? '{}'"
                  rows="8"
                  spellcheck="false"
                  @input="jsonDrafts[capability.id] = ($event.target as HTMLTextAreaElement).value"
                />
              </label>
            </div>

            <div
              v-else
              class="tc-field-grid"
            >
              <label
                v-for="field in schemaFields(capability)"
                :key="field.key"
                class="tc-field"
              >
                <span>{{ field.label }}</span>

                <input
                  v-if="field.type === 'boolean'"
                  type="checkbox"
                  :checked="Boolean(draftValue(capability.id, field.key))"
                  @change="updateDraft(capability.id, field.key, ($event.target as HTMLInputElement).checked)"
                >
                <select
                  v-else-if="field.type === 'enum'"
                  :value="String(draftValue(capability.id, field.key) ?? '')"
                  @change="updateDraft(capability.id, field.key, ($event.target as HTMLSelectElement).value)"
                >
                  <option
                    v-for="option in field.options"
                    :key="option"
                    :value="option"
                  >
                    {{ option }}
                  </option>
                </select>
                <textarea
                  v-else-if="field.type === 'string_list' || field.type === 'path_list' || field.type === 'enum_list'"
                  rows="3"
                  :value="formatListInput(draftValue(capability.id, field.key))"
                  @input="updateDraft(capability.id, field.key, parseListInput(($event.target as HTMLTextAreaElement).value))"
                />
                <textarea
                  v-else-if="field.type === 'object_list'"
                  rows="5"
                  spellcheck="false"
                  :value="formatJsonValue(draftValue(capability.id, field.key))"
                  @input="updateJsonField(capability.id, field.key, ($event.target as HTMLTextAreaElement).value)"
                />
                <input
                  v-else-if="field.type === 'number' || field.type === 'integer'"
                  type="number"
                  :min="field.min"
                  :max="field.max"
                  :step="field.step ?? (field.type === 'integer' ? 1 : 0.05)"
                  :value="Number(draftValue(capability.id, field.key) ?? 0)"
                  @input="updateDraft(capability.id, field.key, numberFromInput(($event.target as HTMLInputElement).value, field.type === 'integer'))"
                >
                <input
                  v-else-if="field.type === 'color'"
                  type="color"
                  :value="String(draftValue(capability.id, field.key) ?? '')"
                  @input="updateDraft(capability.id, field.key, ($event.target as HTMLInputElement).value)"
                >
                <input
                  v-else
                  type="text"
                  :value="String(draftValue(capability.id, field.key) ?? '')"
                  @input="updateDraft(capability.id, field.key, ($event.target as HTMLInputElement).value)"
                >

                <small v-if="field.hint">{{ field.hint }}</small>
              </label>
            </div>

            <div class="tc-config-actions">
              <button
                type="button"
                class="tc-btn primary"
                @click="onSaveConfig(capability)"
              >
                Save config
              </button>
              <button
                type="button"
                class="tc-btn"
                @click="resetDraft(capability)"
              >
                Revert draft
              </button>
            </div>
          </details>

          <div class="tc-actions-row">
            <div
              class="tc-rating"
              role="radiogroup"
              :aria-label="`Rate ${capability.display_name}`"
            >
              <button
                v-for="rating in 5"
                :key="rating"
                type="button"
                role="radio"
                :aria-checked="Math.round(avgCapabilityRating(capability)) >= rating"
                :class="['tc-star', { active: Math.round(avgCapabilityRating(capability)) >= rating }]"
                @click="store.setRating(capability.id, rating)"
              >
                *
              </button>
            </div>

            <div class="tc-buttons">
              <button
                type="button"
                class="tc-btn"
                @click="store.recordUsage(capability.id)"
              >
                Test
              </button>
              <button
                type="button"
                class="tc-btn"
                @click="onReset(capability)"
              >
                Reset
              </button>
              <button
                v-if="maturityOf(capability) === 'proven'"
                type="button"
                class="tc-btn primary"
                @click="store.promote(capability.id)"
              >
                Promote
              </button>
              <span
                v-else-if="maturityOf(capability) === 'canon'"
                class="tc-canon"
                :title="capability.last_promotion_plan_id ?? ''"
              >
                Canon
              </span>
            </div>
          </div>
        </li>
      </ul>

      <p
        v-if="store.lastPromotionPlanId"
        class="tc-result"
      >
        Created workflow plan <code>{{ store.lastPromotionPlanId }}</code>.
      </p>
      <p
        v-if="store.error"
        class="tc-error"
      >
        {{ store.error }}
      </p>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import {
  CAPABILITY_CATEGORY_ORDER,
  avgCapabilityRating,
  capabilityMaturityColor,
  capabilityMaturityLabel,
  categoryLabel,
  deriveCapabilityMaturity,
  type CapabilityCategory,
  type ConfigObject,
  type ConfigValue,
  type ConfigFieldSchema,
  type TeachableCapability,
  useTeachableCapabilitiesStore,
} from '../stores/teachable-capabilities';
import { formatRelativeTime as formatRelative } from '../utils/teaching-maturity';

interface RenderField {
  key: string;
  label: string;
  type: string;
  hint?: string;
  min?: number;
  max?: number;
  step?: number;
  options: string[];
}

const store = useTeachableCapabilitiesStore();
const activeCategory = ref<CapabilityCategory>('voice');
const draftConfigs = ref<Record<string, ConfigObject>>({});
const jsonDrafts = ref<Record<string, string>>({});

const visibleCapabilities = computed(() => store.byCategory[activeCategory.value] ?? []);

watch(
  () => store.capabilities,
  (capabilities) => {
    for (const capability of capabilities) {
      if (!draftConfigs.value[capability.id]) resetDraft(capability);
    }
    if (!visibleCapabilities.value.length) {
      activeCategory.value = store.categoriesWithCounts[0]?.category ?? 'voice';
    }
  },
  { deep: true },
);

onMounted(async () => {
  await store.load();
  activeCategory.value = store.categoriesWithCounts[0]?.category ?? CAPABILITY_CATEGORY_ORDER[0];
});

function maturityOf(capability: TeachableCapability) {
  return deriveCapabilityMaturity(capability);
}

function maturityStyle(capability: TeachableCapability): Record<string, string> {
  const color = capabilityMaturityColor(maturityOf(capability));
  return {
    color,
    background: `color-mix(in srgb, ${color} 14%, transparent)`,
  };
}

function schemaFields(capability: TeachableCapability): RenderField[] {
  return Object.entries(capability.config_schema)
    .filter((entry): entry is [string, ConfigFieldSchema] => {
      const [key, value] = entry;
      return key !== 'type' && typeof value === 'object' && value !== null;
    })
    .map(([key, value]) => ({
      key,
      label: value.label ?? labelize(key),
      type: value.type ?? 'string',
      hint: value.hint,
      min: value.min,
      max: value.max,
      step: value.step,
      options: value.options ?? [],
    }));
}

function isJsonOnly(capability: TeachableCapability): boolean {
  return typeof capability.config_schema.type === 'string' || schemaFields(capability).length === 0;
}

function draftValue(id: string, key: string): ConfigValue | undefined {
  return draftConfigs.value[id]?.[key];
}

function updateDraft(id: string, key: string, value: ConfigValue): void {
  draftConfigs.value[id] = {
    ...(draftConfigs.value[id] ?? {}),
    [key]: value,
  };
}

function updateJsonField(id: string, key: string, raw: string): void {
  try {
    updateDraft(id, key, JSON.parse(raw));
  } catch {
    updateDraft(id, key, raw);
  }
}

function resetDraft(capability: TeachableCapability): void {
  draftConfigs.value[capability.id] = JSON.parse(JSON.stringify(capability.config)) as ConfigObject;
  jsonDrafts.value[capability.id] = JSON.stringify(capability.config, null, 2);
}

async function onToggle(capability: TeachableCapability, enabled: boolean): Promise<void> {
  await store.setEnabled(capability.id, enabled);
}

async function onSaveConfig(capability: TeachableCapability): Promise<void> {
  if (isJsonOnly(capability)) {
    try {
      const parsed = JSON.parse(jsonDrafts.value[capability.id] ?? '{}') as ConfigObject;
      await store.setConfig(capability.id, parsed);
    } catch {
      store.error = 'JSON config is invalid.';
    }
    return;
  }
  await store.setConfig(capability.id, draftConfigs.value[capability.id] ?? {});
}

async function onReset(capability: TeachableCapability): Promise<void> {
  const resetCapability = await store.reset(capability.id);
  if (resetCapability) resetDraft(resetCapability);
}

function numberFromInput(value: string, integer: boolean): number {
  const parsed = integer ? Number.parseInt(value, 10) : Number.parseFloat(value);
  return Number.isFinite(parsed) ? parsed : 0;
}

function formatListInput(value: ConfigValue | undefined): string {
  if (Array.isArray(value)) return value.join('\n');
  if (typeof value === 'string') return value;
  return '';
}

function parseListInput(value: string): string[] {
  return value
    .split(/[\n,]/)
    .map((item) => item.trim())
    .filter(Boolean);
}

function formatJsonValue(value: ConfigValue | undefined): string {
  if (value === undefined) return '[]';
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

function labelize(key: string): string {
  return key.replace(/_/g, ' ').replace(/\b\w/g, (char) => char.toUpperCase());
}

defineExpose({ categoryLabel });
</script>

<style scoped>
.tc-panel {
  display: grid;
  gap: var(--ts-space-md);
  min-height: 600px;
  max-height: min(760px, calc(100dvh - 4rem));
  overflow: hidden;
  padding: var(--ts-space-md);
  color: var(--ts-text-primary);
  background: var(--ts-bg-panel);
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-lg);
}

.tc-header {
  display: grid;
  gap: var(--ts-space-sm);
}

.tc-title-row {
  display: flex;
  align-items: center;
  gap: var(--ts-space-sm);
}

.tc-icon {
  display: grid;
  place-items: center;
  width: 2rem;
  height: 2rem;
  border-radius: var(--ts-radius-md);
  color: var(--ts-accent);
  background: color-mix(in srgb, var(--ts-accent) 14%, transparent);
}

.tc-title {
  margin: 0;
  font-size: 1.25rem;
}

.tc-pill {
  margin-left: auto;
  padding: 0.22rem 0.58rem;
  border-radius: var(--ts-radius-pill);
  color: var(--ts-success);
  background: color-mix(in srgb, var(--ts-success) 14%, transparent);
  font-size: 0.76rem;
  font-weight: 850;
}

.tc-subtitle {
  margin: 0;
  color: var(--ts-text-secondary);
  font-size: 0.9rem;
  line-height: 1.45;
}

.tc-summary {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: var(--ts-space-xs);
}

.tc-summary-cell {
  display: grid;
  gap: 0.18rem;
  min-height: 3.25rem;
  padding: 0.45rem 0.55rem;
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-md);
  background: var(--ts-bg-surface);
  text-align: center;
}

.tc-summary-cell strong {
  color: var(--ts-text-primary);
  font-size: 1.2rem;
}

.tc-summary-cell span {
  color: var(--ts-text-secondary);
  font-size: 0.72rem;
  font-weight: 760;
}

.tc-summary-learning strong { color: var(--ts-info); }
.tc-summary-proven strong { color: var(--ts-success); }
.tc-summary-canon strong { color: var(--ts-accent-violet); }

.tc-tabs {
  display: flex;
  gap: var(--ts-space-xs);
  overflow-x: auto;
  padding-bottom: 0.1rem;
}

.tc-tab {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  min-height: 2.15rem;
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-md);
  padding: 0.4rem 0.65rem;
  color: var(--ts-text-secondary);
  background: var(--ts-bg-input);
  cursor: pointer;
  font-weight: 760;
  white-space: nowrap;
}

.tc-tab.active {
  color: var(--ts-text-primary);
  border-color: color-mix(in srgb, var(--ts-accent) 48%, var(--ts-border));
  background: color-mix(in srgb, var(--ts-accent) 12%, var(--ts-bg-input));
}

.tc-tab small {
  color: var(--ts-text-muted);
}

.tc-body {
  min-height: 0;
  overflow-y: auto;
  scrollbar-gutter: stable;
}

.tc-status,
.tc-error,
.tc-result {
  margin: 0;
  padding: var(--ts-space-sm);
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-md);
  color: var(--ts-text-secondary);
  background: var(--ts-bg-surface);
}

.tc-error {
  color: var(--ts-error);
  border-color: color-mix(in srgb, var(--ts-error) 40%, var(--ts-border));
}

.tc-result {
  margin-top: var(--ts-space-sm);
  color: var(--ts-success);
  border-color: color-mix(in srgb, var(--ts-success) 40%, var(--ts-border));
}

.tc-list {
  display: grid;
  gap: var(--ts-space-sm);
  margin: 0;
  padding: 0;
  list-style: none;
}

.tc-card {
  display: grid;
  gap: var(--ts-space-sm);
  padding: var(--ts-space-sm);
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-md);
  background: var(--ts-bg-surface);
}

.tc-card-head,
.tc-actions-row,
.tc-meta-row,
.tc-config-actions,
.tc-buttons {
  display: flex;
  align-items: center;
  gap: var(--ts-space-sm);
}

.tc-card-title {
  display: grid;
  gap: 0.2rem;
  min-width: 0;
  flex: 1;
}

.tc-card-title strong {
  font-size: 0.98rem;
}

.tc-card-title span,
.tc-field small,
.tc-meta-row {
  color: var(--ts-text-secondary);
  font-size: 0.82rem;
  line-height: 1.42;
}

.tc-toggle {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  color: var(--ts-text-secondary);
  font-size: 0.82rem;
  font-weight: 800;
}

.tc-meta-row {
  flex-wrap: wrap;
}

.tc-maturity,
.tc-canon {
  border-radius: var(--ts-radius-pill);
  padding: 0.16rem 0.5rem;
  font-size: 0.72rem;
  font-weight: 850;
}

.tc-config {
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-md);
  padding: var(--ts-space-xs) var(--ts-space-sm);
  background: var(--ts-bg-base);
}

.tc-config summary {
  cursor: pointer;
  color: var(--ts-text-primary);
  font-weight: 850;
}

.tc-field-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(190px, 1fr));
  gap: var(--ts-space-sm);
  margin-top: var(--ts-space-sm);
}

.tc-field,
.tc-json-editor label {
  display: grid;
  gap: 0.35rem;
  color: var(--ts-text-secondary);
  font-size: 0.82rem;
  font-weight: 800;
}

.tc-field input:not([type='checkbox']),
.tc-field select,
.tc-field textarea,
.tc-json-editor textarea {
  width: 100%;
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-md);
  padding: 0.45rem 0.55rem;
  color: var(--ts-text-primary);
  background: var(--ts-bg-input);
  font: inherit;
}

.tc-json-editor {
  margin-top: var(--ts-space-sm);
}

.tc-config-actions {
  justify-content: flex-end;
  margin-top: var(--ts-space-sm);
}

.tc-actions-row {
  justify-content: space-between;
  flex-wrap: wrap;
}

.tc-rating {
  display: inline-flex;
  align-items: center;
  gap: 0.1rem;
}

.tc-star {
  display: grid;
  place-items: center;
  width: 1.7rem;
  height: 1.7rem;
  border: 1px solid transparent;
  border-radius: var(--ts-radius-md);
  color: var(--ts-text-muted);
  background: transparent;
  cursor: pointer;
  font-size: 1rem;
}

.tc-star.active {
  color: var(--ts-warning);
  background: color-mix(in srgb, var(--ts-warning) 12%, transparent);
}

.tc-btn {
  min-height: 2rem;
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-md);
  padding: 0.35rem 0.68rem;
  color: var(--ts-text-primary);
  background: var(--ts-bg-input);
  cursor: pointer;
  font-weight: 820;
}

.tc-btn.primary {
  color: var(--ts-text-on-accent);
  border-color: var(--ts-accent);
  background: var(--ts-accent);
}

.tc-canon {
  color: var(--ts-accent-violet);
  background: color-mix(in srgb, var(--ts-accent-violet) 14%, transparent);
}

@media (max-width: 640px) {
  .tc-panel {
    max-height: calc(100dvh - 2rem);
  }

  .tc-summary {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .tc-card-head,
  .tc-actions-row {
    align-items: stretch;
    flex-direction: column;
  }

  .tc-toggle,
  .tc-buttons {
    justify-content: space-between;
  }
}
</style>