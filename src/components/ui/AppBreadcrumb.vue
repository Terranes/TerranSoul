<template>
  <nav
    class="bp-crumb"
    aria-label="Breadcrumb"
    data-testid="app-breadcrumb"
  >
    <button
      type="button"
      class="bp-crumb-link"
      :title="`Go to ${props.homeLabel.toLowerCase()}`"
      @click="emit('navigate', props.homeTarget)"
    >
      {{ props.homeLabel }}
    </button>
    <span
      class="bp-crumb-sep"
      aria-hidden="true"
    >›</span>
    <button
      type="button"
      class="bp-crumb-link"
      :title="`Go to ${props.rootLabel.toLowerCase()}`"
      @click="emit('navigate', props.rootTarget ?? props.homeTarget)"
    >
      {{ props.rootLabel }}
    </button>
    <span
      class="bp-crumb-sep"
      aria-hidden="true"
    >›</span>
    <span
      class="bp-crumb-now"
      aria-current="page"
    >{{ here }}</span>
  </nav>
</template>

<script setup lang="ts">
const props = withDefaults(
  defineProps<{
    /** Current page label (rightmost crumb), e.g. "BRAIN PANEL". */
    here: string;
    /** Root crumb label (default "COMPANION"). */
    rootLabel?: string;
    /** Target emitted when the root crumb is clicked (defaults to homeTarget). */
    rootTarget?: string;
    /** Leftmost crumb label (default "TERRANSOUL"). */
    homeLabel?: string;
    /** Target emitted by the home crumb (default 'chat'). */
    homeTarget?: string;
  }>(),
  {
    rootLabel: 'COMPANION',
    rootTarget: undefined,
    homeLabel: 'TERRANSOUL',
    homeTarget: 'chat',
  },
);

const emit = defineEmits<{
  (event: 'navigate', target: string): void;
}>();
</script>
