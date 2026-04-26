<template>
  <img
    v-if="thumbnailUrl"
    :src="thumbnailUrl"
    :alt="alt"
    class="vrm-thumb"
  >
  <div
    v-else-if="isGenerating"
    class="vrm-thumb vrm-thumb--loading"
  >
    <span class="vrm-thumb-spinner" />
  </div>
  <div
    v-else
    class="vrm-thumb vrm-thumb--placeholder"
  >
    👤
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue';
import { useVrmThumbnail } from '../composables/useVrmThumbnail';

const props = defineProps<{
  cacheKey: string;
  modelPath?: string;
  userModelId?: string;
  alt?: string;
}>();

const { thumbnailUrl, isGenerating, generate } = useVrmThumbnail(
  props.cacheKey,
  { modelPath: props.modelPath, userModelId: props.userModelId },
);

onMounted(() => generate());
</script>

<style scoped>
.vrm-thumb {
  width: 56px;
  height: 56px;
  border-radius: 6px;
  flex-shrink: 0;
  object-fit: cover;
  background: rgba(255, 255, 255, 0.04);
}

.vrm-thumb--loading {
  display: flex;
  align-items: center;
  justify-content: center;
}

.vrm-thumb--placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 1.5rem;
}

.vrm-thumb-spinner {
  width: 20px;
  height: 20px;
  border: 2px solid var(--ts-border);
  border-top-color: var(--ts-accent);
  border-radius: 50%;
  animation: vrm-spin 0.8s linear infinite;
}

@keyframes vrm-spin {
  to { transform: rotate(360deg); }
}
</style>
