<template>
  <component
    :is="as"
    class="ts-floating-menu"
    v-bind="$attrs"
    @click="handleClick"
  >
    <slot />
  </component>
</template>

<script setup lang="ts">
withDefaults(defineProps<{
  as?: string;
}>(), {
  as: 'div',
});

const emit = defineEmits<{
  click: [event: MouseEvent];
}>();

function handleClick(event: MouseEvent) {
  emit('click', event);
}
</script>
