<template>
  <div class="viewport-wrapper">
    <div class="background-layer" :style="backgroundStyle" />
    <div class="background-tint" />
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
    <div class="top-controls">
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
      <button class="import-vrm-btn" @click="openVrmPicker">Import VRM</button>
      <input
        ref="vrmInputRef"
        class="hidden-file-input"
        type="file"
        accept=".vrm"
        @change="handleVrmImport"
      />
    </div>
    <div class="background-controls">
      <span class="background-label">Background</span>
      <button
        v-for="background in backgroundStore.allBackgrounds"
        :key="background.id"
        class="background-chip"
        :class="{ active: backgroundStore.selectedBackgroundId === background.id }"
        @click="backgroundStore.selectBackground(background.id)"
      >
        {{ background.name }}
      </button>
      <button class="import-bg-btn" @click="openBackgroundPicker">Import BG</button>
      <input
        ref="backgroundInputRef"
        class="hidden-file-input"
        type="file"
        accept="image/*"
        @change="handleBackgroundImport"
      />
    </div>
    <div v-if="backgroundStore.importError" class="background-error-banner">
      {{ backgroundStore.importError }}
    </div>
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
import { useBackgroundStore } from '../stores/background';
import { DEFAULT_MODELS } from '../config/default-models';
import { initScene, type RendererInfo, type SceneContext } from '../renderer/scene';
import { loadVRMSafe } from '../renderer/vrm-loader';
import { CharacterAnimator } from '../renderer/character-animator';

const canvasRef = ref<HTMLCanvasElement | null>(null);
const characterStore = useCharacterStore();
const backgroundStore = useBackgroundStore();
const showDebug = ref(false);
const debugInfo = ref<RendererInfo>({ triangles: 0, calls: 0, programs: 0 });
const backgroundInputRef = ref<HTMLInputElement | null>(null);
const vrmInputRef = ref<HTMLInputElement | null>(null);
const localVrmObjectUrl = ref<string | null>(null);

const characterName = computed(() => {
  return characterStore.vrmMetadata?.title ?? 'TerranSoul';
});

const backgroundStyle = computed(() => ({
  backgroundImage: `url("${backgroundStore.currentBackground.url}")`,
}));

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

function openVrmPicker() {
  vrmInputRef.value?.click();
}

async function handleVrmImport(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) {
    return;
  }

  if (!file.name.toLowerCase().endsWith('.vrm')) {
    characterStore.setLoadError('Please choose a .vrm file.');
    input.value = '';
    return;
  }

  characterStore.setLoadError(undefined);

  if (localVrmObjectUrl.value) {
    URL.revokeObjectURL(localVrmObjectUrl.value);
  }

  const objectUrl = URL.createObjectURL(file);
  localVrmObjectUrl.value = objectUrl;
  await characterStore.loadVrm(objectUrl);
  input.value = '';
}

function openBackgroundPicker() {
  backgroundInputRef.value?.click();
}

async function handleBackgroundImport(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (file) {
    await backgroundStore.importLocalBackground(file);
  }
  input.value = '';
}

function handleKeyDown(e: KeyboardEvent) {
  if (e.ctrlKey && e.key === 'd') {
    e.preventDefault();
    showDebug.value = !showDebug.value;
  }
}

onMounted(async () => {
  backgroundStore.ensureValidSelection();

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
    // Adjust orbit target height based on zoom (face ↔ full body)
    ctx.updateZoomTarget();
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
  if (localVrmObjectUrl.value) {
    URL.revokeObjectURL(localVrmObjectUrl.value);
  }
});

watch(
  () => characterStore.state,
  (newState) => animator.setState(newState),
);

// Watch for brain-triggered random animation requests
watch(
  () => characterStore.randomAnimTrigger,
  () => animator.triggerRandomAnimation(),
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
      const persona = model?.persona ?? 'witch';
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

.background-layer {
  position: absolute;
  inset: 0;
  background-position: center;
  background-repeat: no-repeat;
  background-size: cover;
  z-index: 0;
}

.background-tint {
  position: absolute;
  inset: 0;
  background: linear-gradient(180deg, rgba(15, 23, 42, 0.08) 0%, rgba(15, 23, 42, 0.16) 100%);
  z-index: 1;
  pointer-events: none;
}

.viewport-canvas {
  position: relative;
  z-index: 2;
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

.top-controls {
  position: absolute;
  top: 12px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  align-items: center;
  gap: 10px;
  z-index: 6;
}

.background-controls {
  position: absolute;
  top: 56px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  justify-content: center;
  max-width: min(92vw, 920px);
  padding: 8px 12px;
  border-radius: 14px;
  background: rgba(15, 23, 42, 0.46);
  backdrop-filter: blur(10px);
  z-index: 6;
}

.background-label {
  color: rgba(255, 255, 255, 0.82);
  font-size: 0.76rem;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.background-chip,
.import-bg-btn {
  padding: 7px 12px;
  border-radius: 999px;
  border: 1px solid rgba(255, 255, 255, 0.16);
  background: rgba(255, 255, 255, 0.12);
  color: #fff;
  font-size: 0.76rem;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.15s ease, transform 0.15s ease, border-color 0.15s ease;
}

.background-chip:hover,
.import-bg-btn:hover {
  background: rgba(255, 255, 255, 0.22);
  transform: translateY(-1px);
}

.background-chip.active {
  background: rgba(59, 130, 246, 0.9);
  border-color: rgba(191, 219, 254, 0.85);
}

.import-bg-btn {
  background: rgba(168, 85, 247, 0.88);
  border-color: rgba(233, 213, 255, 0.7);
}

.hidden-file-input {
  display: none;
}

.background-error-banner {
  position: absolute;
  top: 108px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 6;
  padding: 8px 12px;
  border-radius: 10px;
  background: rgba(127, 29, 29, 0.82);
  color: #fee2e2;
  font-size: 0.76rem;
  font-weight: 600;
  backdrop-filter: blur(8px);
}

.model-selector {
  min-width: 220px;
  padding: 7px 28px 7px 12px;
  border-radius: 8px;
  border: 1px solid rgba(255, 255, 255, 0.22);
  background: rgba(0, 0, 0, 0.55);
  color: rgba(255, 255, 255, 0.92);
  font-size: 0.82rem;
  backdrop-filter: blur(6px);
  cursor: pointer;
  outline: none;
  appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='10' height='6'%3E%3Cpath d='M0 0l5 6 5-6z' fill='rgba(255,255,255,0.7)'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 10px center;
}
.model-selector:hover {
  background-color: rgba(59, 130, 246, 0.3);
  border-color: rgba(59, 130, 246, 0.5);
}
.model-selector option {
  background: #1e293b;
  color: #f1f5f9;
}

.import-vrm-btn {
  padding: 7px 12px;
  border-radius: 8px;
  border: 1px solid rgba(99, 102, 241, 0.55);
  background: rgba(79, 70, 229, 0.82);
  color: #fff;
  font-size: 0.8rem;
  font-weight: 700;
  letter-spacing: 0.02em;
  cursor: pointer;
  box-shadow: 0 6px 18px rgba(79, 70, 229, 0.28);
  transition: transform 0.15s ease, background 0.15s ease, border-color 0.15s ease;
}

.import-vrm-btn:hover {
  background: rgba(99, 102, 241, 0.96);
  border-color: rgba(129, 140, 248, 0.9);
  transform: translateY(-1px);
}

.state-badge {
  position: absolute;
  top: 12px;
  right: 16px;
  padding: 4px 12px;
  border-radius: 999px;
  font-size: 0.75rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  pointer-events: none;
  background: rgba(15, 23, 42, 0.72);
  color: #ffffff;
  border: 1px solid rgba(255, 255, 255, 0.18);
  box-shadow: 0 4px 14px rgba(0, 0, 0, 0.28);
  backdrop-filter: blur(6px);
}

.state-badge.idle {
  background: rgba(37, 99, 235, 0.9);
  color: #eff6ff;
  border-color: rgba(147, 197, 253, 0.7);
}

.state-badge.thinking {
  background: rgba(245, 158, 11, 0.92);
  color: #fff7ed;
  border-color: rgba(253, 230, 138, 0.7);
}

.state-badge.talking {
  background: rgba(22, 163, 74, 0.9);
  color: #f0fdf4;
  border-color: rgba(134, 239, 172, 0.7);
}

.state-badge.happy {
  background: rgba(8, 145, 178, 0.92);
  color: #ecfeff;
  border-color: rgba(103, 232, 249, 0.7);
}

.state-badge.sad {
  background: rgba(126, 34, 206, 0.9);
  color: #faf5ff;
  border-color: rgba(216, 180, 254, 0.7);
}

.state-badge.angry {
  background: rgba(255, 80, 60, 0.35);
  color: #ff8a80;
}

.state-badge.relaxed {
  background: rgba(80, 200, 180, 0.35);
  color: #80e8d0;
}

.state-badge.surprised {
  background: rgba(255, 180, 50, 0.35);
  color: #ffc850;
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
