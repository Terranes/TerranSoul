<template>
  <span class="agent-badge">{{ name }}</span>
</template>

<script setup lang="ts">
defineProps<{ name: string }>();
</script>

<style scoped>
.agent-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 0.68rem;
  font-weight: 700;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: #a89cff;
  margin-bottom: 2px;
  padding-left: 4px;
}

.agent-badge::before {
  content: '';
  display: inline-block;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: currentColor;
  opacity: 0.7;
}
</style>
