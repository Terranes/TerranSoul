<template>
  <div class="viewport-wrapper">
    <canvas ref="canvasRef" class="viewport-canvas" />
    <!-- Loading overlay -->
    <Transition name="fade">
      <div v-if="characterStore.isLoading" class="loading-overlay">
        <div class="loading-spinner" />
        <span class="loading-text">Loading model…</span>
      </div>
    </Transition>
    <div class="character-name-overlay">{{ characterName }}</div>
    <div v-if="characterStore.vrmMetadata" class="character-meta-overlay">
      <span>by {{ characterStore.vrmMetadata.author }}</span>
    </div>
    <!-- Model selector dropdown -->
    <select
      class="model-selector"
      :value="characterStore.selectedModelId"
      @change="handleModelChange"
    >
      <option
        v-for="model in characterStore.defaultModels"
        :key="model.id"
        :value="model.id"
      >{{ model.name }}</option>
    </select>
    <div class="state-badge" :class="characterStore.state">
      {{ characterStore.state }}
    </div>
    <div v-if="showDebug" class="debug-overlay">
      <span>WebGL</span>
      <span>▲ {{ debugInfo.triangles }}</span>
      <span>⬡ {{ debugInfo.calls }} draws</span>
      <span>⚙ {{ debugInfo.programs }} progs</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import * as THREE from 'three';
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';
import { useCharacterStore } from '../stores/character';
import { DEFAULT_MODELS } from '../config/default-models';
import { initScene, type RendererInfo, type SceneContext } from '../renderer/scene';
import { loadVRMSafe } from '../renderer/vrm-loader';
import { CharacterAnimator } from '../renderer/character-animator';

const canvasRef = ref<HTMLCanvasElement | null>(null);
const characterStore = useCharacterStore();
const showDebug = ref(false);
const debugInfo = ref<RendererInfo>({ triangles: 0, calls: 0, programs: 0 });

const characterName = computed(() => {
  return characterStore.vrmMetadata?.title ?? 'TerranSoul';
});

let animFrameId = 0;
let disposeScene: (() => void) | null = null;
let getRendererInfo: (() => RendererInfo) | null = null;
let sceneCtx: SceneContext | null = null;
let currentVrmScene: THREE.Object3D | null = null;
const animator = new CharacterAnimator();

function handleModelChange(e: Event) {
  const select = e.target as HTMLSelectElement;
  characterStore.selectModel(select.value);
}

function handleKeyDown(e: KeyboardEvent) {
  if (e.ctrlKey && e.key === 'd') {
    e.preventDefault();
    showDebug.value = !showDebug.value;
  }
}

onMounted(async () => {
  const canvas = canvasRef.value;
  if (!canvas) return;

  const ctx = await initScene(canvas);
  sceneCtx = ctx;
  disposeScene = ctx.dispose;
  getRendererInfo = ctx.getRendererInfo;

  // Auto-load the default VRM model (loading overlay shows until ready)
  characterStore.loadDefaultModel();

  function loop() {
    animFrameId = requestAnimationFrame(loop);
    const delta = ctx.clock.getDelta();
    // Update OrbitControls (damping requires per-frame update)
    ctx.controls.update();
    // Keep lookAt target at camera position so VRM eyes track the viewer
    ctx.lookAtTarget.position.copy(ctx.camera.position);
    animator.update(delta);
    ctx.renderer.render(ctx.scene, ctx.camera);

    if (showDebug.value && getRendererInfo) {
      debugInfo.value = getRendererInfo();
    }
  }
  loop();

  window.addEventListener('keydown', handleKeyDown);
});

onUnmounted(() => {
  cancelAnimationFrame(animFrameId);
  disposeScene?.();
  window.removeEventListener('keydown', handleKeyDown);
});

watch(
  () => characterStore.state,
  (newState) => animator.setState(newState),
);

// Watch for VRM path changes and load the model
watch(
  () => characterStore.vrmPath,
  async (newPath) => {
    if (!newPath || !sceneCtx) return;

    // Remove the previous VRM model from the scene before loading a new one
    if (currentVrmScene) {
      sceneCtx.scene.remove(currentVrmScene);
      currentVrmScene = null;
    }

    const result = await loadVRMSafe(sceneCtx.scene, newPath);
    if (result) {
      currentVrmScene = result.vrm.scene;
      // Hide the model initially — loadVRM already added it to the scene,
      // but we keep it invisible until everything (textures, morphs, bones)
      // is fully parsed so the user never sees hair dropping or half-loaded
      // geometry.  We reveal it below after the animator is wired up.
      result.vrm.scene.visible = false;

      // Look up per-model persona and bone-pose config
      const model = DEFAULT_MODELS.find(m => m.path === newPath);
      // rotateVRM0() sets vrm.scene.rotation.y = Math.PI for VRM 0.x.
      // Capture whatever rotation the loader left on the scene root so the
      // animator preserves it every frame instead of overwriting it to 0.
      const rotY = result.vrm.scene.rotation.y + (model?.rotationY ?? 0);
      const persona = model?.persona ?? 'cool';
      animator.setVRM(result.vrm, rotY, persona);
      // Wire up eye tracking — lookAtTarget is in the scene, updated per frame
      animator.setLookAtTarget(sceneCtx.lookAtTarget);
      characterStore.setMetadata(result.metadata);

      // Expose VRM for E2E testing — allows Playwright to verify bone positions
      (window as any).__terransoul_vrm__ = result.vrm;

      // Run one animation tick so bones settle into the natural pose before
      // the first visible frame — this prevents the T-pose flash.
      animator.update(0);

      // Now reveal the fully-posed model and dismiss the loading overlay
      result.vrm.scene.visible = true;
      characterStore.setLoaded();
    } else {
      characterStore.setLoadError('Failed to load VRM model');
      characterStore.setLoaded();
    }
  },
);
</script>

<style scoped>
.viewport-wrapper {
  position: relative;
  width: 100%;
  height: 100%;
  overflow: hidden;
}

.viewport-canvas {
  width: 100%;
  height: 100%;
  display: block;
}

.character-name-overlay {
  position: absolute;
  top: 12px;
  left: 16px;
  font-size: 1.1rem;
  font-weight: 700;
  color: rgba(255, 255, 255, 0.85);
  text-shadow: 0 1px 4px rgba(0, 0, 0, 0.6);
  pointer-events: none;
  letter-spacing: 0.05em;
}

.character-meta-overlay {
  position: absolute;
  top: 34px;
  left: 16px;
  font-size: 0.72rem;
  color: rgba(255, 255, 255, 0.45);
  pointer-events: none;
  letter-spacing: 0.02em;
}

.model-selector {
  position: absolute;
  top: 12px;
  left: 50%;
  transform: translateX(-50%);
  padding: 4px 24px 4px 10px;
  border-radius: 6px;
  border: 1px solid rgba(255, 255, 255, 0.2);
  background: rgba(0, 0, 0, 0.5);
  color: rgba(255, 255, 255, 0.85);
  font-size: 0.78rem;
  backdrop-filter: blur(4px);
  cursor: pointer;
  z-index: 5;
  outline: none;
  appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='10' height='6'%3E%3Cpath d='M0 0l5 6 5-6z' fill='rgba(255,255,255,0.6)'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 8px center;
}
.model-selector:hover {
  background-color: rgba(59, 130, 246, 0.3);
  border-color: rgba(59, 130, 246, 0.5);
}
.model-selector option {
  background: #1e293b;
  color: #f1f5f9;
}

.state-badge {
  position: absolute;
  top: 12px;
  right: 16px;
  padding: 2px 10px;
  border-radius: 999px;
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  pointer-events: none;
  background: rgba(255, 255, 255, 0.15);
  color: #fff;
  backdrop-filter: blur(4px);
}

.state-badge.thinking {
  background: rgba(255, 200, 50, 0.35);
  color: #ffd700;
}

.state-badge.talking {
  background: rgba(100, 220, 130, 0.35);
  color: #7ef5a0;
}

.state-badge.happy {
  background: rgba(100, 180, 255, 0.35);
  color: #a0d4ff;
}

.state-badge.sad {
  background: rgba(160, 100, 200, 0.35);
  color: #d4a0ff;
}

.debug-overlay {
  position: absolute;
  bottom: 10px;
  left: 10px;
  display: flex;
  gap: 10px;
  padding: 4px 10px;
  border-radius: 6px;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(4px);
  font-size: 0.7rem;
  font-family: 'Courier New', monospace;
  color: #7ef5a0;
  pointer-events: none;
  letter-spacing: 0.02em;
}

/* Loading overlay */
.loading-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 16px;
  background: rgba(0, 0, 0, 0.55);
  backdrop-filter: blur(6px);
  z-index: 10;
  pointer-events: none;
}

.loading-spinner {
  width: 40px;
  height: 40px;
  border: 3px solid rgba(255, 255, 255, 0.15);
  border-top-color: rgba(100, 180, 255, 0.9);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.loading-text {
  font-size: 0.85rem;
  font-weight: 600;
  color: rgba(255, 255, 255, 0.8);
  letter-spacing: 0.05em;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.4s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
