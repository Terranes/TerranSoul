<template>
  <div class="ple-field">
    <span class="ple-label">{{ label }}</span>
    <ul
      v-if="items.length > 0"
      class="ple-list"
    >
      <li
        v-for="(item, idx) in items"
        :key="idx"
        class="ple-item"
      >
        <span class="ple-text">{{ item }}</span>
        <button
          type="button"
          class="ple-remove"
          :data-testid="`ple-remove-${idx}`"
          :aria-label="`Remove ${item}`"
          @click="remove(idx)"
        >
          ×
        </button>
      </li>
    </ul>
    <div class="ple-add">
      <input
        v-model="entry"
        type="text"
        :placeholder="placeholder"
        maxlength="120"
        class="ple-input"
        :data-testid="`ple-input-${label.toLowerCase().replace(/\s+/g, '-')}`"
        @keydown.enter.prevent="add"
      >
      <button
        type="button"
        class="ple-add-btn"
        :data-testid="`ple-add-${label.toLowerCase().replace(/\s+/g, '-')}`"
        :disabled="!entry.trim()"
        @click="add"
      >
        Add
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
/**
 * Small chip-style list editor used inside PersonaPanel for tone / quirks /
 * avoid arrays. Emits the full updated list each time.
 */
import { ref } from 'vue';

const props = defineProps<{
  label: string;
  placeholder: string;
  items: readonly string[];
}>();

const emit = defineEmits<{
  (e: 'update', items: string[]): void;
}>();

const entry = ref('');

function add(): void {
  const v = entry.value.trim();
  if (!v) return;
  // Case-insensitive dedup, keeps original order.
  const lower = v.toLowerCase();
  if (props.items.some((x) => x.toLowerCase() === lower)) {
    entry.value = '';
    return;
  }
  emit('update', [...props.items, v]);
  entry.value = '';
}

function remove(idx: number): void {
  emit('update', props.items.filter((_, i) => i !== idx));
}
</script>

<style scoped>
.ple-field { display: flex; flex-direction: column; gap: 0.3rem; }
.ple-label { font-size: 0.8rem; color: var(--ts-text-muted, #aab); }
.ple-list { list-style: none; padding: 0; margin: 0; display: flex; flex-wrap: wrap; gap: 0.3rem; }
.ple-item {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  background: var(--ts-bg-input);
  border-radius: 999px;
  padding: 0.2rem 0.6rem 0.2rem 0.7rem;
  font-size: 0.85rem;
}
.ple-remove {
  background: transparent;
  border: 0;
  color: var(--ts-text-muted, #aab);
  cursor: pointer;
  font-size: 1rem;
  line-height: 1;
  padding: 0;
}
.ple-remove:hover { color: var(--ts-danger, #c44); }
.ple-add { display: flex; gap: 0.4rem; }
.ple-input {
  flex: 1;
  background: var(--ts-input-bg, rgba(0, 0, 0, 0.25));
  border: 1px solid var(--ts-border, rgba(255, 255, 255, 0.12));
  color: var(--ts-text, #eee);
  border-radius: var(--ts-radius-sm, 6px);
  padding: 0.35rem 0.6rem;
  font: inherit;
}
.ple-add-btn {
  padding: 0.35rem 0.7rem;
  border-radius: var(--ts-radius-sm, 6px);
  border: 1px solid var(--ts-border, rgba(255, 255, 255, 0.15));
  background: var(--ts-bg-input);
  color: var(--ts-text, #eee);
  cursor: pointer;
}
.ple-add-btn:disabled { opacity: 0.4; cursor: not-allowed; }
</style>
