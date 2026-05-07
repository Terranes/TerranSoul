<template>
  <span
    v-if="isNegative"
    class="negative-badge"
    :title="triggerSummary"
  >
    ⛔ Negative
    <span
      v-if="triggerCount > 0"
      class="trigger-count"
    >
      ({{ triggerCount }} trigger{{ triggerCount === 1 ? '' : 's' }})
    </span>
  </span>
</template>

<script setup lang="ts">
import { computed } from 'vue';

const props = defineProps<{
  cognitiveKind: string;
  triggers?: string[];
}>();

const isNegative = computed(() => props.cognitiveKind === 'negative');
const triggerCount = computed(() => props.triggers?.length ?? 0);
const triggerSummary = computed(() =>
  props.triggers && props.triggers.length > 0
    ? `Triggers: ${props.triggers.join(', ')}`
    : 'Negative memory — avoid this pattern',
);
</script>

<style scoped>
.negative-badge {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.125rem 0.5rem;
  border-radius: 4px;
  background: var(--ts-danger-bg, rgba(220, 38, 38, 0.15));
  color: var(--ts-danger, #ef4444);
  font-size: 0.75rem;
  font-weight: 600;
  white-space: nowrap;
}

.trigger-count {
  font-weight: 400;
  opacity: 0.8;
}
</style>
