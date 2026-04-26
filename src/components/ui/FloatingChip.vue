<template>
  <component
    :is="as"
    class="ts-floating-chip"
    :class="{
      'ts-floating-chip--interactive': interactive,
      'ts-floating-chip--readonly': readonly,
    }"
    v-bind="$attrs"
    @click="handleClick"
  >
    <slot />
  </component>
</template>

<script setup lang="ts">
withDefaults(defineProps<{
  as?: string;
  interactive?: boolean;
  readonly?: boolean;
}>(), {
  as: 'div',
  interactive: false,
  readonly: false,
});

const emit = defineEmits<{
  click: [event: MouseEvent];
}>();

function handleClick(event: MouseEvent) {
  emit('click', event);
}
</script>
