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
    <div class="top-controls" @click.stop style="display:none">
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
    <div v-if="false && backgroundStore.importError" class="background-error-banner">
      {{ backgroundStore.importError }}
    </div>

    <!-- Side panel tabs (hidden - moved to right-click menu) -->
    <div class="side-tabs" @click.stop style="display:none">
      <!-- Tab buttons -->
      <div class="side-tab-buttons">
        <button
          class="side-tab-btn"
          :class="{ active: activeTab === 'bg' }"
          @click="activeTab = activeTab === 'bg' ? null : 'bg'"
        >🖼</button>
        <button
          class="side-tab-btn"
          :class="{ active: activeTab === 'mood' }"
          @click="activeTab = activeTab === 'mood' ? null : 'mood'"
        >😊</button>
      </div>

      <!-- Panel content -->
      <div v-if="activeTab === 'bg'" class="side-panel">
        <span class="panel-label">BACKGROUND</span>
        <button
          v-for="background in backgroundStore.allBackgrounds"
          :key="background.id"
          class="background-chip"
          :class="{ active: backgroundStore.selectedBackgroundId === background.id }"
          @click="backgroundStore.selectBackground(background.id)"
        >{{ background.name }}</button>
        <button class="import-bg-btn" @click="openBackgroundPicker">Import BG</button>
        <input ref="backgroundInputRef" class="hidden-file-input" type="file" accept="image/*" @change="handleBackgroundImport" />
      </div>

      <div v-if="activeTab === 'mood'" class="side-panel">
        <span class="panel-label">MOOD</span>
        <button
          v-for="state in previewStates"
          :key="state"
          class="state-chip"
          :class="[state, { active: characterStore.state === state }]"
          @click="characterStore.setState(state)"
        >{{ stateLabels[state] }}</button>
      </div>
    </div>
    <div class="state-badge" :class="characterStore.state" style="display:none">
      {{ stateLabels[characterStore.state] }}
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
import type { CharacterState } from '../types';

const canvasRef = ref<HTMLCanvasElement | null>(null);
const characterStore = useCharacterStore();
const backgroundStore = useBackgroundStore();
const showDebug = ref(false);
const activeTab = ref<'bg' | 'mood' | null>(null);

// Mouse look target in NDC space (-1..1)
const mouseLook = { x: 0, y: 0 };
function onGlobalMouseMove(e: MouseEvent) {
  // Use the full window as reference so even off-canvas movement works
  mouseLook.x = (e.clientX / window.innerWidth) * 2 - 1;
  mouseLook.y = -((e.clientY / window.innerHeight) * 2 - 1);
}
const debugInfo = ref<RendererInfo>({ triangles: 0, calls: 0, programs: 0 });
const backgroundInputRef = ref<HTMLInputElement | null>(null);
const vrmInputRef = ref<HTMLInputElement | null>(null);
const localVrmObjectUrl = ref<string | null>(null);
const previewStates: CharacterState[] = ['idle', 'thinking', 'talking', 'happy', 'sad', 'angry', 'surprised', 'shy', 'sitting'];
const stateLabels: Record<CharacterState, string> = {
  idle: '💤 Idle',
  thinking: '🤔 Thinking',
  talking: '🗣 Talking',
  happy: '😊 Happy',
  sad: '😢 Sad',
  angry: '😠 Angry',
  surprised: '😲 Surprised',
  shy: '☺️ Shy',
  sitting: '🪑 Sitting',
};

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

// Pre-allocated vectors for eye-tracking (avoids per-frame GC)
const _eyeFwd   = new THREE.Vector3();
const _eyeRight = new THREE.Vector3();
const _eyeUp    = new THREE.Vector3();
const _eyeTarget = new THREE.Vector3();

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
    // Eyes follow mouse cursor — project NDC mouse onto a plane 1.5m in front of character
    {
      const dist = 1.5;
      // getWorldDirection() forces a matrixWorld update and returns the
      // camera's true forward direction (points INTO the scene).
      ctx.camera.getWorldDirection(_eyeFwd);
      // Right = forward × world-up (safe since polar angle is locked to π/2)
      _eyeRight.crossVectors(_eyeFwd, new THREE.Vector3(0, 1, 0)).normalize();
      // Camera-local up
      _eyeUp.crossVectors(_eyeRight, _eyeFwd).normalize();
      const fovY = (ctx.camera.fov * Math.PI) / 180;
      const halfH = Math.tan(fovY / 2) * dist;
      const halfW = halfH * ctx.camera.aspect;
      _eyeTarget.copy(ctx.camera.position)
        .addScaledVector(_eyeFwd, dist)
        .addScaledVector(_eyeRight, mouseLook.x * halfW)
        .addScaledVector(_eyeUp, mouseLook.y * halfH);
      // Smooth interpolation so eyes glide rather than snap
      ctx.lookAtTarget.position.lerp(_eyeTarget, 0.08);
    }
    animator.update(delta);
    ctx.renderer.render(ctx.scene, ctx.camera);

    if (showDebug.value && getRendererInfo) {
      debugInfo.value = getRendererInfo();
    }
  }
  loop();

  window.addEventListener('keydown', handleKeyDown);
  window.addEventListener('mousemove', onGlobalMouseMove);
});

onUnmounted(() => {
  cancelAnimationFrame(animFrameId);
  disposeScene?.();
  window.removeEventListener('keydown', handleKeyDown);
  window.removeEventListener('mousemove', onGlobalMouseMove);
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
  background: transparent;
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

.side-tabs {
  position: absolute;
  right: 0;
  top: 50%;
  transform: translateY(-50%);
  display: flex;
  flex-direction: row;
  align-items: flex-start;
  z-index: 6;
}

.side-tab-buttons {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 6px 4px;
  background: rgba(15, 23, 42, 0.7);
  backdrop-filter: blur(10px);
  border-radius: 10px 0 0 10px;
  border: 1px solid rgba(255,255,255,0.1);
  border-right: none;
}

.side-tab-btn {
  width: 32px;
  height: 32px;
  border-radius: 8px;
  border: 1px solid rgba(255,255,255,0.12);
  background: rgba(255,255,255,0.08);
  color: #fff;
  font-size: 1rem;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.15s;
}

.side-tab-btn:hover,
.side-tab-btn.active {
  background: rgba(59, 130, 246, 0.7);
}

.side-panel {
  display: flex;
  flex-direction: column;
  gap: 5px;
  padding: 8px 10px;
  background: rgba(15, 23, 42, 0.82);
  backdrop-filter: blur(12px);
  border: 1px solid rgba(255,255,255,0.1);
  border-radius: 10px 0 0 10px;
  min-width: 110px;
  max-height: 400px;
  overflow-y: auto;
}

.panel-label {
  color: rgba(255, 255, 255, 0.6);
  font-size: 0.65rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  margin-bottom: 2px;
}

.background-chip,
.import-bg-btn {
  padding: 5px 10px;
  border-radius: 8px;
  border: 1px solid rgba(255, 255, 255, 0.16);
  background: rgba(255, 255, 255, 0.1);
  color: #fff;
  font-size: 0.72rem;
  font-weight: 600;
  cursor: pointer;
  text-align: left;
  transition: background 0.15s ease;
}

.background-chip:hover,
.import-bg-btn:hover {
  background: rgba(255, 255, 255, 0.2);
}

.background-chip.active {
  background: rgba(59, 130, 246, 0.8);
  border-color: rgba(191, 219, 254, 0.85);
}

.import-bg-btn {
  background: rgba(168, 85, 247, 0.7);
  border-color: rgba(233, 213, 255, 0.5);
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

.state-controls {
  position: absolute;
  left: 50%;
  bottom: 18px;
  transform: translateX(-50%);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-wrap: wrap;
  gap: 5px;
  max-width: min(94vw, 1040px);
  padding: 7px 10px;
  border-radius: 14px;
  background: rgba(15, 23, 42, 0.66);
  border: 1px solid rgba(255, 255, 255, 0.12);
  box-shadow: 0 10px 30px rgba(15, 23, 42, 0.24);
  backdrop-filter: blur(14px);
  z-index: 6;
}

.state-controls-label {
  color: rgba(255, 255, 255, 0.92);
  font-size: 0.8rem;
  font-weight: 800;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  margin-right: 6px;
}

.state-chip {
  min-width: 72px;
  padding: 5px 9px;
  border-radius: 999px;
  border: 1px solid rgba(255, 255, 255, 0.16);
  background: rgba(255, 255, 255, 0.14);
  color: #fff;
  font-size: 0.7rem;
  font-weight: 700;
  cursor: pointer;
  transition: transform 0.15s ease, background 0.15s ease, border-color 0.15s ease, box-shadow 0.15s ease, opacity 0.15s ease;
}

.state-chip:hover {
  transform: translateY(-2px) scale(1.02);
  background: rgba(255, 255, 255, 0.24);
}

.state-chip.active {
  transform: translateY(-1px) scale(1.06);
  box-shadow: 0 10px 24px rgba(15, 23, 42, 0.34);
}

.state-badge {
  position: absolute;
  top: 12px;
  right: 16px;
  padding: 8px 16px;
  border-radius: 999px;
  font-size: 0.84rem;
  font-weight: 800;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  pointer-events: none;
  background: rgba(15, 23, 42, 0.82);
  color: #ffffff;
  border: 1px solid rgba(255, 255, 255, 0.18);
  box-shadow: 0 8px 20px rgba(0, 0, 0, 0.32);
  backdrop-filter: blur(8px);
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

.state-badge.angry,
.state-chip.angry.active {
  background: rgba(220, 38, 38, 0.92);
  color: #fef2f2;
  border-color: rgba(252, 165, 165, 0.72);
}

.state-badge.surprised,
.state-chip.surprised.active {
  background: rgba(249, 115, 22, 0.92);
  color: #fff7ed;
  border-color: rgba(253, 186, 116, 0.72);
}

.state-badge.shy,
.state-chip.shy.active {
  background: rgba(236, 72, 153, 0.9);
  color: #fdf2f8;
  border-color: rgba(251, 207, 232, 0.72);
}

.state-chip.idle.active {
  background: rgba(37, 99, 235, 0.9);
  color: #eff6ff;
  border-color: rgba(147, 197, 253, 0.7);
}

.state-chip.thinking.active {
  background: rgba(245, 158, 11, 0.92);
  color: #fff7ed;
  border-color: rgba(253, 230, 138, 0.7);
}

.state-chip.talking.active {
  background: rgba(22, 163, 74, 0.9);
  color: #f0fdf4;
  border-color: rgba(134, 239, 172, 0.7);
}

.state-chip.happy.active {
  background: rgba(8, 145, 178, 0.92);
  color: #ecfeff;
  border-color: rgba(103, 232, 249, 0.7);
}

.state-chip.sad.active {
  background: rgba(126, 34, 206, 0.9);
  color: #faf5ff;
  border-color: rgba(216, 180, 254, 0.7);
}

.state-badge.happy,
.state-badge.angry,
.state-badge.surprised {
  animation: state-badge-pulse 0.9s ease-in-out infinite alternate;
}

.state-badge.shy,
.state-badge.sad {
  animation: state-badge-float 1.8s ease-in-out infinite;
}

@keyframes state-badge-pulse {
  from { transform: scale(1); }
  to { transform: scale(1.08); }
}

@keyframes state-badge-float {
  0%, 100% { transform: translateY(0); }
  50% { transform: translateY(-3px); }
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
