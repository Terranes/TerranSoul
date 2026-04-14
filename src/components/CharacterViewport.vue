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

    <!-- ── Corner settings dropdown ── -->
    <div class="settings-corner" ref="settingsRef">
      <button class="settings-toggle" @click.stop="settingsOpen = !settingsOpen" aria-label="Settings">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
          <path d="M19.14,12.94c0.04-0.3,0.06-0.61,0.06-0.94c0-0.32-0.02-0.64-0.07-0.94l2.03-1.58c0.18-0.14,0.23-0.41,0.12-0.61 l-1.92-3.32c-0.12-0.22-0.37-0.29-0.59-0.22l-2.39,0.96c-0.5-0.38-1.03-0.7-1.62-0.94L14.4,2.81c-0.04-0.24-0.24-0.41-0.48-0.41 h-3.84c-0.24,0-0.43,0.17-0.47,0.41L9.25,5.35C8.66,5.59,8.12,5.92,7.63,6.29L5.24,5.33c-0.22-0.08-0.47,0-0.59,0.22L2.74,8.87 C2.62,9.08,2.66,9.34,2.86,9.48l2.03,1.58C4.84,11.36,4.8,11.69,4.8,12s0.02,0.64,0.07,0.94l-2.03,1.58 c-0.18,0.14-0.23,0.41-0.12,0.61l1.92,3.32c0.12,0.22,0.37,0.29,0.59,0.22l2.39-0.96c0.5,0.38,1.03,0.7,1.62,0.94l0.36,2.54 c0.05,0.24,0.24,0.41,0.48,0.41h3.84c0.24,0,0.44-0.17,0.47-0.41l0.36-2.54c0.59-0.24,1.13-0.56,1.62-0.94l2.39,0.96 c0.22,0.08,0.47,0,0.59-0.22l1.92-3.32c0.12-0.22,0.07-0.47-0.12-0.61L19.14,12.94z M12,15.6c-1.98,0-3.6-1.62-3.6-3.6 s1.62-3.6,3.6-3.6s3.6,1.62,3.6,3.6S13.98,15.6,12,15.6z"/>
        </svg>
      </button>
      <Transition name="dropdown">
        <div v-if="settingsOpen" class="settings-dropdown" @click.stop>
          <!-- Model selector -->
          <div class="dropdown-section">
            <label class="dropdown-label">Character</label>
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
            <button class="dropdown-btn" @click="openVrmPicker">📁 Import VRM</button>
            <input
              ref="vrmInputRef"
              class="hidden-file-input"
              type="file"
              accept=".vrm"
              @change="handleVrmImport"
            />
          </div>
          <!-- Background selector -->
          <div class="dropdown-section">
            <label class="dropdown-label">Background</label>
            <div class="bg-chips">
              <button
                v-for="background in backgroundStore.allBackgrounds"
                :key="background.id"
                class="background-chip"
                :class="{ active: backgroundStore.selectedBackgroundId === background.id }"
                @click="backgroundStore.selectBackground(background.id)"
              >
                {{ background.name }}
              </button>
            </div>
            <button class="dropdown-btn" @click="openBackgroundPicker">🖼 Import BG</button>
            <input
              ref="backgroundInputRef"
              class="hidden-file-input"
              type="file"
              accept="image/*"
              @change="handleBackgroundImport"
            />
          </div>
        </div>
      </Transition>
    </div>

    <div v-if="backgroundStore.importError" class="background-error-banner">
      {{ backgroundStore.importError }}
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
const settingsOpen = ref(false);
const settingsRef = ref<HTMLElement | null>(null);

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

function handleClickOutside(e: MouseEvent) {
  if (settingsRef.value && e.target instanceof Node && !settingsRef.value.contains(e.target)) {
    settingsOpen.value = false;
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
  document.addEventListener('click', handleClickOutside);
});

onUnmounted(() => {
  cancelAnimationFrame(animFrameId);
  disposeScene?.();
  window.removeEventListener('keydown', handleKeyDown);
  document.removeEventListener('click', handleClickOutside);
  if (localVrmObjectUrl.value) {
    URL.revokeObjectURL(localVrmObjectUrl.value);
  }
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

      // rotateVRM0() sets vrm.scene.rotation.y = Math.PI for VRM 0.x.
      // Capture whatever rotation the loader left on the scene root so the
      // animator preserves it every frame instead of overwriting it to 0.
      const model = DEFAULT_MODELS.find(m => m.path === newPath);
      const rotY = result.vrm.scene.rotation.y + (model?.rotationY ?? 0);
      animator.setVRM(result.vrm, rotY);
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
  background:
    radial-gradient(ellipse at center, transparent 40%, rgba(10, 15, 30, 0.35) 100%),
    linear-gradient(180deg, rgba(15, 23, 42, 0.06) 0%, rgba(15, 23, 42, 0.20) 100%);
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
  left: 56px;
  font-size: var(--ts-text-lg);
  font-weight: 700;
  color: rgba(255, 255, 255, 0.92);
  text-shadow: 0 1px 6px rgba(0, 0, 0, 0.7), 0 0 20px rgba(0, 0, 0, 0.3);
  pointer-events: none;
  letter-spacing: 0.05em;
}

.character-meta-overlay {
  position: absolute;
  top: 34px;
  left: 56px;
  font-size: 0.72rem;
  color: rgba(255, 255, 255, 0.55);
  text-shadow: 0 1px 4px rgba(0, 0, 0, 0.5);
  pointer-events: none;
  letter-spacing: 0.02em;
}

/* ── Corner settings dropdown ── */
.settings-corner {
  position: absolute;
  top: 12px;
  left: 16px;
  z-index: 20;
}

.settings-toggle {
  width: 38px;
  height: 38px;
  border-radius: 50%;
  border: 1px solid rgba(255, 255, 255, 0.18);
  background: rgba(11, 17, 32, 0.72);
  color: rgba(255, 255, 255, 0.85);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  backdrop-filter: blur(10px);
  transition: background var(--ts-transition-normal), transform var(--ts-transition-fast), box-shadow var(--ts-transition-normal);
  box-shadow: 0 2px 10px rgba(0, 0, 0, 0.3);
}
.settings-toggle:hover {
  background: rgba(124, 111, 255, 0.55);
  transform: scale(1.08);
  box-shadow: 0 4px 16px rgba(124, 111, 255, 0.3);
}

.settings-dropdown {
  position: absolute;
  top: 42px;
  left: 0;
  width: 280px;
  max-width: min(280px, 90vw);
  padding: 14px;
  border-radius: var(--ts-radius-lg);
  border: 1px solid rgba(255, 255, 255, 0.12);
  background: rgba(11, 17, 32, 0.94);
  backdrop-filter: blur(20px);
  display: flex;
  flex-direction: column;
  gap: 14px;
  box-shadow: var(--ts-shadow-lg);
}

.dropdown-section {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.dropdown-label {
  font-size: var(--ts-text-xs);
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--ts-text-dim);
}

.dropdown-btn {
  padding: 6px 10px;
  border-radius: var(--ts-radius-sm);
  border: 1px solid var(--ts-border);
  background: rgba(255, 255, 255, 0.06);
  color: rgba(255, 255, 255, 0.75);
  font-size: var(--ts-text-sm);
  cursor: pointer;
  transition: background var(--ts-transition-fast);
  text-align: left;
}
.dropdown-btn:hover {
  background: var(--ts-bg-hover);
}

.bg-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

/* Dropdown transition */
.dropdown-enter-active, .dropdown-leave-active {
  transition: opacity 0.18s ease, transform 0.18s ease;
}
.dropdown-enter-from, .dropdown-leave-to {
  opacity: 0;
  transform: translateY(-6px) scale(0.96);
}

.model-selector {
  width: 100%;
  padding: 7px 28px 7px 10px;
  border-radius: var(--ts-radius-md);
  border: 1px solid rgba(255, 255, 255, 0.15);
  background: rgba(255, 255, 255, 0.06);
  color: rgba(255, 255, 255, 0.92);
  font-size: 0.82rem;
  cursor: pointer;
  outline: none;
  appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='10' height='6'%3E%3Cpath d='M0 0l5 6 5-6z' fill='rgba(255,255,255,0.7)'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 10px center;
  transition: border-color var(--ts-transition-fast);
}
.model-selector:hover {
  border-color: rgba(108, 99, 255, 0.5);
}
.model-selector option {
  background: var(--ts-bg-surface);
  color: var(--ts-text-primary);
}

.hidden-file-input {
  display: none;
}

.background-chip {
  padding: 5px 10px;
  border-radius: var(--ts-radius-pill);
  border: 1px solid var(--ts-border);
  background: rgba(255, 255, 255, 0.08);
  color: #fff;
  font-size: 0.72rem;
  font-weight: 600;
  cursor: pointer;
  transition: background var(--ts-transition-fast), border-color var(--ts-transition-fast), transform var(--ts-transition-fast);
}
.background-chip:hover {
  background: rgba(255, 255, 255, 0.18);
  transform: translateY(-1px);
}
.background-chip.active {
  background: rgba(124, 111, 255, 0.85);
  border-color: rgba(200, 210, 255, 0.85);
}

.background-error-banner {
  position: absolute;
  top: 56px;
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

.debug-overlay {
  position: absolute;
  bottom: 10px;
  left: 10px;
  display: flex;
  gap: 10px;
  padding: 4px 10px;
  border-radius: var(--ts-radius-sm);
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(4px);
  font-size: 0.7rem;
  font-family: var(--ts-font-mono);
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
  border-top-color: rgba(108, 99, 255, 0.9);
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
