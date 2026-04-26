<template>
  <component
    :is="as"
    class="ts-floating-badge"
    :class="{
      'ts-floating-badge--warning': tone === 'warning',
      'ts-floating-badge--readonly': readonly,
    }"
    v-bind="$attrs"
  >
    <slot />
  </component>
</template>

<script setup lang="ts">
withDefaults(defineProps<{
  as?: string;
  tone?: 'warning';
  readonly?: boolean;
}>(), {
  as: 'div',
  tone: 'warning',
  readonly: false,
});
</script>
