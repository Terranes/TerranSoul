<template>
  <div class="viewport-wrapper">
    <canvas ref="canvasRef" class="viewport-canvas" />
    <div class="character-name-overlay">TerranSoul</div>
    <div class="state-badge" :class="characterStore.state">
      {{ characterStore.state }}
    </div>
    <div v-if="showDebug" class="debug-overlay">
      <span>{{ rendererType.toUpperCase() }}</span>
      <span>▲ {{ debugInfo.triangles }}</span>
      <span>⬡ {{ debugInfo.calls }} draws</span>
      <span>⚙ {{ debugInfo.programs }} progs</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue';
import { useCharacterStore } from '../stores/character';
import { initScene, type RendererInfo, type RendererType } from '../renderer/scene';
import { createPlaceholderCharacter } from '../renderer/vrm-loader';
import { CharacterAnimator } from '../renderer/character-animator';

const canvasRef = ref<HTMLCanvasElement | null>(null);
const characterStore = useCharacterStore();
const showDebug = ref(false);
const rendererType = ref<RendererType>('webgl');
const debugInfo = ref<RendererInfo>({ triangles: 0, calls: 0, programs: 0 });

let animFrameId = 0;
let disposeScene: (() => void) | null = null;
let getRendererInfo: (() => RendererInfo) | null = null;
const animator = new CharacterAnimator();

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
  disposeScene = ctx.dispose;
  getRendererInfo = ctx.getRendererInfo;
  rendererType.value = ctx.rendererType;

  const placeholder = createPlaceholderCharacter(ctx.scene);
  animator.setPlaceholder(placeholder);

  function loop() {
    animFrameId = requestAnimationFrame(loop);
    const delta = ctx.clock.getDelta();
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
</style>
