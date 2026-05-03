<!-- eslint-disable vue/no-v-html -->
<template>
  <div
    v-if="tag === 'div'"
    v-html="html"
  />
  <span
    v-else-if="tag === 'span'"
    v-html="html"
  />
  <p
    v-else
    v-html="html"
  />
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { renderMarkdown } from '../utils/render-markdown';

const props = withDefaults(defineProps<{
  text: string;
  tag?: 'div' | 'span' | 'p';
  cursor?: boolean;
}>(), {
  tag: 'div',
  cursor: false,
});

const html = computed(() => {
  const rendered = renderMarkdown(props.text);
  return props.cursor ? `${rendered}<span class="cursor-blink">&#9614;</span>` : rendered;
});
</script>