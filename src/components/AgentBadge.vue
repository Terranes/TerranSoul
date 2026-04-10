<template>
  <span class="agent-badge">{{ name }}</span>
</template>

<script setup lang="ts">
defineProps<{ name: string }>();
</script>

<style scoped>
.agent-badge {
  display: inline-block;
  font-size: 0.68rem;
  font-weight: 700;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: #a89cff;
  margin-bottom: 1px;
  padding-left: 4px;
}
</style>
