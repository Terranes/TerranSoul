<template>
  <!--
    BackgroundScene — animated WebGL background driven by a Three.js
    fragment shader.  This component is a slim mount/unmount wrapper;
    every visual concern lives in `src/renderer/background/`.

    The canvas is appended to <body> directly by the scene module so the
    HTML written here remains essentially empty.  We keep an empty root
    element only so Vue has somewhere to mount.

    Hidden in pet mode by parent (`<BackgroundScene v-if="!isPetMode" />`
    in App.vue) so we never spend GPU memory on the transparent overlay.
  -->
  <div
    class="ts-bg-mount"
    aria-hidden="true"
  />
</template>

<script setup lang="ts">
import { onBeforeUnmount, onMounted } from 'vue';
import {
  createBackgroundScene,
  type BackgroundScene,
} from '../renderer/background/scene';

let handle: BackgroundScene | null = null;

onMounted(() => {
  // createBackgroundScene returns null when WebGL is unavailable; in
  // that case the body's --ts-bg-gradient (kept as a fallback in
  // src/style.css) provides the backdrop.
  handle = createBackgroundScene();
});

onBeforeUnmount(() => {
  handle?.dispose();
  handle = null;
});
</script>

<style scoped>
/* The mount point itself is invisible — the real canvas is appended
   to <body> by the scene module so it can sit at z-index: -1 without
   being affected by parent stacking contexts. */
.ts-bg-mount {
  display: none;
}
</style>
