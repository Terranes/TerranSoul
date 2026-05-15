<template>
  <div
    class="viewport-wrapper"
    :class="{ 'viewport-wrapper--pet': isPetMode }"
  >
    <!-- Background layer: only shown when a user-imported local image is active.
         In Auto mode the body CSS gradient (--ts-bg-gradient) shows through. -->
    <div
      v-if="!isPetMode && backgroundStore.currentBackground.kind !== 'auto'"
      class="background-layer"
      :style="backgroundStyle"
    />
    <div
      v-if="!isPetMode"
      class="background-tint"
    />
    <canvas
      ref="canvasRef"
      class="viewport-canvas"
    />
    <!-- Loading overlay -->
    <Transition name="fade">
      <div
        v-if="characterStore.isLoading"
        class="loading-overlay"
      >
        <div class="loading-spinner" />
        <span class="loading-text">Loading model…</span>
      </div>
    </Transition>
    <!-- Error overlay -->
    <Transition name="fade">
      <div
        v-if="characterStore.loadError && !characterStore.isLoading"
        class="loading-overlay load-error-overlay"
      >
        <span class="load-error-icon">⚠️</span>
        <span class="loading-text">{{ characterStore.loadError }}</span>
        <button
          class="load-error-retry"
          @click="retryModelLoad"
        >
          Retry
        </button>
      </div>
    </Transition>
    <!-- ── Quest progress portal & on-demand Settings modal ─────────────────
         The visible Settings gear/chip was removed in favour of the global
         AppChromeActions cluster (top-right of every panel). This cluster
         is now only the quest-progress portal anchor + a v-modeled
         SettingsPanel modal that the parent can toggle. -->
    <div
      v-if="!isPetMode"
      ref="settingsRef"
      class="corner-cluster"
    >
      <div class="settings-host">
        <Transition name="dropdown">
          <SettingsPanel
            v-if="settingsOpen && !props.hideSettingsDialog"
            :is-pet-mode="isPetMode"
            :bgm="bgm"
            v-model:bgm-enabled="bgmEnabled"
            v-model:bgm-volume="bgmVolume"
            v-model:bgm-track-id="bgmTrackId"
            @close="settingsOpen = false"
            @request-set-display-mode="(mode) => emit('set-display-mode', mode)"
            @request-toggle-pet-mode="emit('toggle-pet-mode')"
            @toggle-system-info="showSystemInfo = !showSystemInfo"
            @toggle-audio-controls="showAudioControls = !showAudioControls"
            @url-dialog-toggle="(open: boolean) => { showUrlDialog = open; }"
          />
        </Transition>

        <!-- Full-screen overlays (rendered outside the dropdown to avoid z-index issues) -->
        <SystemInfoPanel
          v-if="showSystemInfo"
          @close="showSystemInfo = false"
        />
        <AudioControlsPanel
          v-if="showAudioControls"
          @close="showAudioControls = false"
          @navigate="(target: string) => { showAudioControls = false; emit('navigate', target); }"
          @update:bgm-volume="handleAudioBgmVolumeChange"
          @update:bgm-track-id="handleAudioBgmTrackChange"
        />
      </div> <!-- /.settings-host -->
    </div>

    <div
      v-if="backgroundStore.importError"
      class="background-error-banner"
    >
      {{ backgroundStore.importError }}
    </div>

    <div
      v-if="showDebug"
      class="debug-overlay"
    >
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
import { listen } from '@tauri-apps/api/event';
import { useCharacterStore } from '../stores/character';
import { useBackgroundStore } from '../stores/background';
import { useSettingsStore } from '../stores/settings';
import { useAudioStore } from '../stores/audio';
import { useWindowStore } from '../stores/window';
import { usePersonaStore } from '../stores/persona';
import { DEFAULT_MODELS } from '../config/default-models';
import { initScene, type RendererInfo, type SceneContext } from '../renderer/scene';
import { loadVRMSafe, createPlaceholderCharacter } from '../renderer/vrm-loader';
import { CharacterAnimator } from '../renderer/character-animator';
import { VrmaManager, getAnimationForMood, getAnimationForMotion, getIdleAnimationForGender, getStandingAnimationForMood, SITTING_ANIMATION_PATHS } from '../renderer/vrma-manager';
import { LearnedMotionPlayer, applyLearnedExpression, clearExpressionPreview } from '../renderer/learned-motion-player';
import { PoseAnimator, type LlmPoseFrame } from '../renderer/pose-animator';
import { EmotionPoseBias, type BiasEmotion } from '../renderer/emotion-pose-bias';
import { SittingPropController } from '../renderer/sitting-props-controller';
import { getSharedBgmPlayer } from '../composables/useBgmPlayer';
import { subscribeLlmPoseFrames, type LlmPoseListen } from '../utils/llm-pose-events';
import SystemInfoPanel from './SystemInfoPanel.vue';
import AudioControlsPanel from './AudioControlsPanel.vue';
import SettingsPanel from './SettingsPanel.vue';

const emit = defineEmits<{
  'request-add-music': [];
  'overlay-open': [open: boolean];
  'set-display-mode': [mode: 'desktop' | 'chatbox'];
  'toggle-pet-mode': [];
  navigate: [target: string];
}>();

const props = withDefaults(defineProps<{
  /** Force transparent pet rendering even when the app window is in normal mode. */
  forcePet?: boolean;
  /** Hide/close the settings dialog while chat history is expanded. */
  hideSettingsDialog?: boolean;
}>(), {
  forcePet: false,
  hideSettingsDialog: false,
});

const canvasRef = ref<HTMLCanvasElement | null>(null);
const characterStore = useCharacterStore();
const backgroundStore = useBackgroundStore();
const settingsStore = useSettingsStore();
const audioStore = useAudioStore();
const windowStoreLocal = useWindowStore();
const personaStore = usePersonaStore();
/** Viewport behaves differently in pet mode: no background, no chrome,
 *  transparent clear colour, and pedestal hidden in the 3D scene. */
const isPetMode = computed(() => props.forcePet || windowStoreLocal.mode === 'pet');

const showDebug = ref(false);
const debugInfo = ref<RendererInfo>({ triangles: 0, calls: 0, programs: 0 });
const settingsOpen = ref(false);
const settingsRef = ref<HTMLElement | null>(null);
const showSystemInfo = ref(false);
const showAudioControls = ref(false);

// ── BGM player ────────────────────────────────────────────────────────────────
// State stays here because it is consumed by multiple systems beyond the
// settings dropdown: AudioControlsPanel, ChatView (via defineExpose's
// enableBgm), and the audio-store mute watcher. SettingsPanel binds to it
// via v-model:bgm-* props.
// Use the app-wide shared instance so the global SettingsModal and any
// other BGM surfaces drive the same audio element.
const bgm = getSharedBgmPlayer();
const bgmEnabled = ref(false);
const bgmVolume = ref(0.15);
const bgmTrackId = ref('prelude');
const showUrlDialog = ref(false);

/** Restore BGM state from persisted settings. */
function restoreBgmFromSettings() {
  const s = settingsStore.settings;
  bgmEnabled.value = s.bgm_enabled;
  bgmVolume.value = s.bgm_volume;
  bgmTrackId.value = s.bgm_track_id;
  // Load persisted custom tracks
  if (s.bgm_custom_tracks?.length) {
    bgm.loadCustomTracks(s.bgm_custom_tracks);
  }
  // Don't auto-play here — browser autoplay policy blocks AudioContext.resume()
  // without a user gesture. Instead, defer playback until the first interaction.
  if (bgmEnabled.value) {
    bgm.setVolume(bgmVolume.value);
    deferBgmPlayback();
  }
}

/** Wait for the first user interaction, then start BGM. */
let bgmDeferredCleanup: (() => void) | null = null;
function deferBgmPlayback() {
  if (bgmDeferredCleanup) return; // already deferred
  const startBgm = () => {
    if (bgmEnabled.value) {
      bgm.setVolume(bgmVolume.value);
      bgm.play(bgmTrackId.value);
    }
    cleanup();
  };
  const cleanup = () => {
    document.removeEventListener('click', startBgm, true);
    document.removeEventListener('keydown', startBgm, true);
    document.removeEventListener('touchstart', startBgm, true);
    bgmDeferredCleanup = null;
  };
  bgmDeferredCleanup = cleanup;
  document.addEventListener('click', startBgm, { capture: true, once: true });
  document.addEventListener('keydown', startBgm, { capture: true, once: true });
  document.addEventListener('touchstart', startBgm, { capture: true, once: true });
}

// Event handlers for audio controls panel
function handleAudioBgmVolumeChange(volume: number) {
  bgmVolume.value = volume;
  bgm.setVolume(volume);
  if (bgmEnabled.value) {
    settingsStore.saveBgmState(bgmEnabled.value, bgmVolume.value, bgmTrackId.value);
  }
}

// React to mute toggles — silence BGM immediately and restore prior volume
// on unmute. Voice (TTS) is muted independently inside `useTtsPlayback`
// via the `mutedRef` option wired up in ChatView / PetOverlayView.
watch(
  () => audioStore.muted,
  (isMuted) => {
    bgm.setVolume(isMuted ? 0 : bgmVolume.value);
  },
);

function handleAudioBgmTrackChange(trackId: string) {
  bgmTrackId.value = trackId;
  if (bgmEnabled.value) {
    bgm.play(trackId);
    settingsStore.saveBgmState(bgmEnabled.value, bgmVolume.value, bgmTrackId.value);
  }
}

const backgroundStyle = computed(() => {
  const bg = backgroundStore.currentBackground;
  // Auto: no inline style — body CSS gradient (via --ts-bg-gradient) shows through.
  if (bg.kind === 'auto') return {};
  return { backgroundImage: `url("${bg.url}")` };
});

let animFrameId = 0;
let disposeScene: (() => void) | null = null;
let getRendererInfo: (() => RendererInfo) | null = null;
let sceneCtx: SceneContext | null = null;
let currentVrmScene: THREE.Object3D | null = null;
const animator = new CharacterAnimator();
const vrmaManager = new VrmaManager();
const motionPlayer = new LearnedMotionPlayer(vrmaManager);
const poseAnimator = new PoseAnimator();
const emotionBias = new EmotionPoseBias();
let expressionPreviewTimer: ReturnType<typeof setTimeout> | null = null;
let unlistenLlmPose: (() => void) | null = null;
let viewportUnmounted = false;

// ── Sitting chair prop ───────────────────────────────────────────────
const sittingPropController = new SittingPropController();
// In pet / forcePet mode the floating preview has no floor — a chair would
// visibly hover beside the avatar. Disable the prop entirely in that case.
sittingPropController.disabled = isPetMode.value;
watch(isPetMode, (pet) => {
  sittingPropController.disabled = pet;
  if (pet) {
    sittingPropController.dispose(sceneCtx?.scene ?? null);
  }
});

function disposeSittingProps() {
  sittingPropController.dispose(sceneCtx?.scene ?? null);
}

async function subscribeToLlmPoseEvents() {
  try {
    const unlisten = await subscribeLlmPoseFrames(listen as LlmPoseListen, (frame) => {
      poseAnimator.applyFrame(frame);
    });
    if (viewportUnmounted) {
      unlisten();
    } else {
      unlistenLlmPose = unlisten;
    }
  } catch {
    // Browser mode has no Tauri event bus; pose frames arrive only in native shells.
  }
}

function syncSittingProps(playing: boolean) {
  sittingPropController.sync(sceneCtx?.scene ?? null, playing, vrmaManager.currentPath);
}

// Wire VRMA playback state to the animator + lazy-load sitting props.
// When a non-looping mood animation finishes, return the character to idle
// so the idle VRMA loop restarts and the character doesn't appear frozen.
vrmaManager.onPlaybackChange((playing) => {
  animator.setVrmaPlaying(playing);
  poseAnimator.setVrmaPlaying(playing);
  syncSittingProps(playing);

  if (!playing) {
    // A VRMA clip just ended — if the character is in an emotional state
    // (not idle/talking/thinking) that means a one-shot mood animation finished.
    // Transition back to idle so the idle loop restarts.
    const s = characterStore.state;
    if (s !== 'idle' && s !== 'talking' && s !== 'thinking') {
      characterStore.setState('idle');
    }
  }
});

// Expose the avatar state machine for direct mutation by ChatView (coarse state bridge)
defineExpose({
  /** The layered AvatarStateMachine — ChatView mutates body/emotion here. */
  get avatarStateMachine() {
    return animator.avatarStateMachine;
  },
  /** Enable BGM playback (called by ChatView when BGM quest is accepted). */
  enableBgm() {
    if (!bgmEnabled.value) {
      bgmEnabled.value = true;
      bgm.setVolume(bgmVolume.value);
      bgm.play(bgmTrackId.value);
      settingsStore.saveBgmState(true, bgmVolume.value, bgmTrackId.value);
    }
  },
  /** Toggle the quick-settings modal — called by the global AppChromeActions
   *  gear so the chat tab has a single settings entry point. */
  toggleSettingsDialog,
  /** Scene context — used by PetOverlayView to project 3D positions and rotate. */
  get sceneContext() {
    return sceneCtx;
  },
  /**
   * Play a VRMA body animation by motion key (e.g. 'greeting', 'clapping').
   * Called by ChatView when the LLM emits a motion tag.
   * Suppresses mood-auto-play so the mood watcher doesn't override this.
   */
  playMotion(motionKey: string) {
    const entry = getAnimationForMotion(motionKey);
    if (entry) {
      vrmaManager.suppressMoodAnimation();
      vrmaManager.play(entry.path, entry.loop, 0.4);
    }
  },
  /** Stop any playing VRMA animation and return to procedural. */
  stopMotion() {
    vrmaManager.stop(0.4);
  },
  /** Whether a mood-suppressed VRMA animation is actively playing (e.g. angry.vrma). */
  get isAnimationActive(): boolean {
    return vrmaManager.isMoodSuppressed && vrmaManager.isPlaying;
  },
  /**
   * Play a learned motion clip on the avatar. Bakes the JSON frames
   * into an AnimationClip on the fly.
   */
  playLearnedMotion(motion: import('../stores/persona-types').LearnedMotion) {
    vrmaManager.suppressMoodAnimation();
    motionPlayer.play(motion, false, 0.4);
  },
  /**
   * Preview a learned expression by applying its weights to the VRM
   * for 3 seconds, then resetting.
   */
  previewExpression(expr: import('../stores/persona-types').LearnedExpression) {
    const vrm = vrmaManager.vrm;
    if (!vrm) return;
    if (expressionPreviewTimer) clearTimeout(expressionPreviewTimer);
    applyLearnedExpression(vrm, expr);
    expressionPreviewTimer = setTimeout(() => {
      clearExpressionPreview(vrm);
      expressionPreviewTimer = null;
    }, 3000);
  },
  /**
   * Apply an LLM-generated pose frame (chunk 14.16b3). The frame is
   * additively layered on top of the procedural idle animation; a
   * VRMA-driven body animation, when active, wins automatically.
   */
  playPose(frame: LlmPoseFrame) {
    poseAnimator.applyFrame(frame);
  },
  /** Clear any in-flight LLM pose and fade back to procedural idle. */
  clearPose() {
    poseAnimator.reset();
  },
});

function retryModelLoad() {
  characterStore.selectModel(characterStore.selectedModelId);
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

function toggleSettingsDialog() {
  if (props.hideSettingsDialog) return;
  settingsOpen.value = !settingsOpen.value;
}

watch(
  () => props.hideSettingsDialog,
  (hidden) => {
    if (hidden && settingsOpen.value) settingsOpen.value = false;
  },
  { immediate: true },
);

watch(
  [settingsOpen, showSystemInfo, showAudioControls, showUrlDialog],
  ([settingsDialogOpen, systemInfoOpen, audioControlsOpen, urlDialogOpen]) => {
    const overlayOpen = (settingsDialogOpen && !props.hideSettingsDialog)
      || systemInfoOpen
      || audioControlsOpen
      || urlDialogOpen;
    emit('overlay-open', overlayOpen);
  },
  { immediate: true },
);

// WebGL context loss handlers (hoisted so onUnmounted can remove them)
function handleContextLost(e: Event) {
  e.preventDefault();
  console.warn('[TerranSoul] WebGL context lost');
}
function handleContextRestored() {
  console.warn('[TerranSoul] WebGL context restored — reloading model');
  if (characterStore.vrmPath) {
    loadModelIntoScene(characterStore.vrmPath);
  }
}

onMounted(async () => {
  viewportUnmounted = false;
  window.addEventListener('keydown', handleKeyDown);
  document.addEventListener('click', handleClickOutside);
  void subscribeToLlmPoseEvents();

  backgroundStore.ensureValidSelection();

  const canvas = canvasRef.value;
  if (!canvas) return;

  const ctx = await initScene(canvas);
  sceneCtx = ctx;
  disposeScene = ctx.dispose;
  getRendererInfo = ctx.getRendererInfo;

  // Apply the current pet-mode state now that the scene is available.
  // The reactive watch below only fires on subsequent changes — this
  // initial call catches the case where the user mounted already in pet
  // mode (e.g. re-open to a saved state).
  ctx.setPedestalVisible(!isPetMode.value);
  if (isPetMode.value) {
    // Browser landing preview keeps full LEFT-drag rotation so visitors
    // can interact; real desktop pet mode disables LEFT/RIGHT so the OS
    // window-drag handler can claim them. (Mirrors the watcher below.)
    ctx.controls.mouseButtons = props.forcePet
      ? {
          LEFT: THREE.MOUSE.ROTATE,
          MIDDLE: THREE.MOUSE.DOLLY,
          RIGHT: THREE.MOUSE.PAN,
        }
      : {
          LEFT: null as unknown as THREE.MOUSE,
          MIDDLE: THREE.MOUSE.ROTATE,
          RIGHT: null as unknown as THREE.MOUSE,
        };
  }

  // Persist camera state after user finishes orbiting or zooming.
  ctx.onCameraChange((azimuth, distance) => {
    settingsStore.saveCameraState(azimuth, distance);
  });

  // Restore persisted camera state (azimuth + distance).
  // Skip in pet mode — always start with full-body framing.
  const savedAzimuth = settingsStore.settings.camera_azimuth;
  const savedDistance = settingsStore.settings.camera_distance;
  if (savedDistance > 0 && !isPetMode.value) {
    // Set camera position from saved spherical coordinates (elevation = 0 = equatorial)
    const x = savedDistance * Math.sin(savedAzimuth);
    const z = savedDistance * Math.cos(savedAzimuth);
    ctx.camera.position.set(x, ctx.camera.position.y, z);
    ctx.controls.update();
  }

  // Auto-load the default VRM model (loading overlay shows until ready).
  // If vrmPath is already set (HMR re-mount), reload it directly since
  // the watcher won't fire for an unchanged value.
  if (characterStore.vrmPath) {
    loadModelIntoScene(characterStore.vrmPath);
  } else {
    characterStore.loadDefaultModel();
  }

  // Handle WebGL context loss — reload model when the GPU context is restored
  canvas.addEventListener('webglcontextlost', handleContextLost);
  canvas.addEventListener('webglcontextrestored', handleContextRestored);

  // ── Cursor tracking: head/eye follow mouse pointer ────────────────
  // Converts mouse position to normalised viewport coords (-1..1) and
  // feeds it into the animator each frame for head/eye tracking.
  function handleCursorMove(e: MouseEvent) {
    const rect = canvas!.getBoundingClientRect();
    if (rect.width === 0 || rect.height === 0) return;
    const nx = ((e.clientX - rect.left) / rect.width) * 2 - 1;
    const ny = ((e.clientY - rect.top) / rect.height) * 2 - 1;
    animator.setCursorPosition(nx, ny);
  }
  function handleCursorLeave() {
    // Smoothly return to centre when cursor leaves viewport
    animator.setCursorPosition(0, 0);
  }
  canvas.addEventListener('mousemove', handleCursorMove);
  canvas.addEventListener('mouseleave', handleCursorLeave);

  // Restore BGM state (track, volume, enabled) from persisted settings.
  restoreBgmFromSettings();

  // On-demand rendering: throttle to ~15 FPS when idle & settled
  const IDLE_INTERVAL = 1 / 15; // ~66ms
  let idleAccum = 0;

  function loop() {
    animFrameId = requestAnimationFrame(loop);
    const delta = ctx.clock.getDelta();

    // ── Auto-resize: ensure renderer matches canvas display size ──
    // This is the primary mechanism that prevents the "model invisible"
    // bug.  It catches v-show transitions, window resizes, and any other
    // case where the canvas display-size changes after initScene ran
    // with degenerate (0×0 or 1×1) dimensions.
    ctx.checkResize();

    // Adjust orbit target height based on zoom (face ↔ full body)
    ctx.updateZoomTarget();
    // Update OrbitControls (damping requires per-frame update)
    ctx.controls.update();

    // Eye tracking is now handled by the CharacterAnimator via cursor position.
    // The animator uses VRM lookAt yaw/pitch directly based on mouse position,
    // providing cursor-following eyes in both procedural and VRMA modes.

    const asm = animator.avatarStateMachine;
    const settled = animator.isAnimationSettled();
    const idle = asm.state.body === 'idle';

    if (settled && idle && !asm.state.needsRender && !vrmaManager.isPlaying) {
      // Throttle: accumulate time, only render at ~15 FPS
      idleAccum += delta;
      if (idleAccum < IDLE_INTERVAL) return;
      idleAccum = 0;
    } else {
      idleAccum = 0;
    }

    // Clear the one-shot render flag
    if (asm.state.needsRender) asm.state.needsRender = false;

    // Tick VRMA animation mixer (must be before animator.update which calls vrm.update)
    vrmaManager.update(delta);
    animator.update(delta);
    // Layer the LLM-as-Animator pose blender on top of the procedural
    // bones written by `animator.update`. Runs after so its additive
    // offsets sit on the most recent rotation values.
    poseAnimator.apply(vrmaManager.vrm, delta);
    // Emotion-reactive procedural pose bias (Chunk 14.16d). Yields
    // when a baked VRMA clip or an LLM pose is in charge so it never
    // fights the higher-priority animation source.
    emotionBias.apply(
      vrmaManager.vrm,
      delta,
      vrmaManager.isPlaying || poseAnimator.isActive,
    );
    ctx.renderer.render(ctx.scene, ctx.camera);

    if (showDebug.value && getRendererInfo) {
      debugInfo.value = getRendererInfo();
    }
  }
  loop();
});

onUnmounted(() => {
  viewportUnmounted = true;
  cancelAnimationFrame(animFrameId);
  unlistenLlmPose?.();
  unlistenLlmPose = null;
  disposeSittingProps();
  disposeScene?.();
  bgm.stop();
  bgmDeferredCleanup?.();
  window.removeEventListener('keydown', handleKeyDown);
  document.removeEventListener('click', handleClickOutside);
  // Remove WebGL context loss listeners
  const canvas = canvasRef.value;
  if (canvas) {
    canvas.removeEventListener('webglcontextlost', handleContextLost);
    canvas.removeEventListener('webglcontextrestored', handleContextRestored);
  }
  vrmaManager.dispose();
});

// Track last mood animation to prevent re-triggering the same one
// (e.g. multiple <anim> tags with the same emotion in one response).
let lastMoodAnimState: string | null = null;

watch(
  () => characterStore.state,
  (newState) => {
    animator.setState(newState, characterStore.emotionIntensity);
    // Drive the emotion-reactive procedural pose bias (Chunk 14.16d).
    // `thinking` / `talking` are body states with no postural mood —
    // map them to neutral so we don't double-stack with talking gestures.
    const biasMap: Record<typeof newState, BiasEmotion> = {
      idle: 'neutral',
      thinking: 'neutral',
      talking: 'neutral',
      happy: 'happy',
      sad: 'sad',
      angry: 'angry',
      relaxed: 'relaxed',
      surprised: 'surprised',
    };
    emotionBias.setEmotion(biasMap[newState] ?? 'neutral', characterStore.emotionIntensity);
    // Skip mood auto-play when an explicit motion is active (e.g. LLM said "clapping")
    if (vrmaManager.isMoodSuppressed) return;
    // Idle special-case: use character-gender weighted loop selection.
    if (newState === 'idle') {
      lastMoodAnimState = null; // reset so next emotion can play
      const idleEntry = getIdleAnimationForGender(
        characterStore.currentGender(),
        Math.random,
        isPetMode.value,
      );
      if (idleEntry) {
        // Keep looping until mood/state changes away from idle.
        vrmaManager.play(idleEntry.path, true, 0.4);
      } else {
        vrmaManager.stop(0.4);
      }
      return;
    }
    // Try to play a VRMA animation mapped to this mood (one-shot, then return to procedural).
    // Skip if the same mood animation is already playing — prevents
    // re-triggering per sentence when multiple <anim> tags emit the
    // same emotion during a streamed response.
    if (newState === lastMoodAnimState) return;
    // In pet/forcePet mode, prefer the standing variant so we don't spawn a chair
    // floating in mid-air next to the small floating preview.
    const entry = isPetMode.value
      ? (getStandingAnimationForMood(newState) ?? getAnimationForMood(newState))
      : getAnimationForMood(newState);
    if (entry && (!isPetMode.value || !SITTING_ANIMATION_PATHS.has(entry.path))) {
      lastMoodAnimState = newState;
      vrmaManager.suppressMoodAnimation();
      vrmaManager.play(entry.path, false, 0.4);
    } else if (newState === 'talking') {
      lastMoodAnimState = null; // reset so emotion after talking can play
      // Return to procedural animation for talking
      vrmaManager.stop(0.4);
    }
  },
);

// ── Persona preview bridge ────────────────────────────────────────────────
// PersonaPanel (BrainView) writes requests; we consume them here.

watch(
  () => personaStore.previewExpressionRequest,
  (expr) => {
    if (!expr) return;
    const vrm = vrmaManager.vrm;
    if (vrm) {
      if (expressionPreviewTimer) clearTimeout(expressionPreviewTimer);
      applyLearnedExpression(vrm, expr);
      expressionPreviewTimer = setTimeout(() => {
        clearExpressionPreview(vrm);
        expressionPreviewTimer = null;
      }, 3000);
    }
    personaStore.previewExpressionRequest = null;
  },
);

watch(
  () => personaStore.previewMotionRequest,
  (motion) => {
    if (!motion) return;
    vrmaManager.suppressMoodAnimation();
    motionPlayer.play(motion, false, 0.4);
    personaStore.previewMotionRequest = null;
  },
);

// Hide the pedestal (and any other floor decorations) in pet mode so the
// character floats cleanly on the desktop with nothing visible behind.
// Remap mouse buttons:
//   • Real pet mode (desktop overlay): left-drag moves the OS window
//     (handled by PetOverlayView), middle-button rotates the model.
//   • forcePet preview (browser landing): there is no draggable window,
//     so left-drag MUST rotate the model — otherwise visitors cannot
//     interact with the character at all.
watch(
  () => isPetMode.value,
  (pet) => {
    if (sceneCtx) {
      sceneCtx.setPedestalVisible(!pet);
      if (pet) {
        const isLandingPreview = props.forcePet;
        sceneCtx.controls.mouseButtons = isLandingPreview
          ? {
              // Browser landing preview — full interaction.
              LEFT: THREE.MOUSE.ROTATE,
              MIDDLE: THREE.MOUSE.DOLLY,
              RIGHT: THREE.MOUSE.PAN,
            }
          : {
              // Desktop pet overlay — left/right reserved for OS window drag.
              LEFT: null as unknown as THREE.MOUSE,
              MIDDLE: THREE.MOUSE.ROTATE,
              RIGHT: null as unknown as THREE.MOUSE,
            };
        // Reset camera to full-body zoom so the entire character is visible
        // by default in pet mode (including legs and shoes).
        sceneCtx.resetToFullBody();
      } else {
        // Restore default OrbitControls mouse mapping
        sceneCtx.controls.mouseButtons = {
          LEFT: THREE.MOUSE.ROTATE,
          MIDDLE: THREE.MOUSE.DOLLY,
          RIGHT: THREE.MOUSE.PAN,
        };
      }
    }
  },
  { immediate: true },
);

// Watch for VRM path changes and load the model
watch(
  () => characterStore.vrmPath,
  (newPath) => { loadModelIntoScene(newPath); },
);

/** Load a VRM model into the active scene. Shared by the watcher and onMounted (HMR recovery). */
async function loadModelIntoScene(newPath: string | undefined) {
    if (!newPath || !sceneCtx) return;

    // Remove the previous VRM model from the scene before loading a new one
    if (currentVrmScene) {
      sceneCtx.scene.remove(currentVrmScene);
      currentVrmScene = null;
    }

    try {
      // Race the VRM load against a timeout to prevent infinite "Loading model…"
      const VRM_LOAD_TIMEOUT_MS = 30_000;
      const loadPromise = loadVRMSafe(sceneCtx.scene, newPath);
      const timeoutPromise = new Promise<null>((resolve) =>
        setTimeout(() => resolve(null), VRM_LOAD_TIMEOUT_MS),
      );
      const result = await Promise.race([loadPromise, timeoutPromise]);
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
        // Bind VRMA manager to the loaded VRM for animation playback
        vrmaManager.setVRM(result.vrm);
        characterStore.setMetadata(result.metadata);

        // Expose VRM for E2E testing — allows Playwright to verify bone positions
        (window as unknown as Record<string, unknown>).__terransoul_vrm__ = result.vrm;

        // Run one animation tick so bones settle into the natural pose before
        // the first visible frame — this prevents the T-pose flash.
        animator.update(0);

        // Reframe the camera to fit this specific character's height so every
        // model appears fully visible and centred regardless of their size.
        sceneCtx.frameCameraToCharacter(result.vrm.scene);

        // Register the model for deferred reframe — if the canvas is still
        // hidden (display:none via v-show), the ResizeObserver will re-frame
        // once the canvas becomes visible with real dimensions.
        sceneCtx.setCurrentModel(result.vrm.scene);

        // Now reveal the fully-posed model and dismiss the loading overlay
        result.vrm.scene.visible = true;

        // Kick off the initial idle animation — the state watcher only fires
        // on *changes*, but the character is already in 'idle' state at load
        // time, so we need an explicit trigger here.
        if (characterStore.state === 'idle') {
          const idleEntry = getIdleAnimationForGender(
            characterStore.currentGender(),
            Math.random,
            isPetMode.value,
          );
          if (idleEntry) {
            vrmaManager.play(idleEntry.path, true, 0.4);
          }
        }

        characterStore.setLoaded();
      } else {
        // Show a placeholder character so the scene isn't empty (load failed or timed out)
        console.warn('[TerranSoul] VRM load returned null — showing placeholder');
        const placeholder = createPlaceholderCharacter(sceneCtx.scene);
        currentVrmScene = placeholder;
        characterStore.setLoadError('Failed to load VRM model — try retry or a different character');
        characterStore.setLoaded();
      }
    } catch (error) {
      console.error('[TerranSoul] Model setup failed after VRM load:', error);
      // Ensure loading overlay is dismissed even if post-load setup fails
      const placeholder = createPlaceholderCharacter(sceneCtx.scene);
      currentVrmScene = placeholder;
      characterStore.setLoadError('Model loaded but failed to initialise');
      characterStore.setLoaded();
    }
}
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

/* ── Top bubble strip: single Settings button ──
   The global NotificationBubble lives at top:16px / right:16px (z-index 1500).
   Offset this cluster leftwards so the gear and the bell never overlap. */
.corner-cluster {
  position: absolute;
  top: 18px;
  right: 72px;
  z-index: 40;
  display: flex;
  align-items: center;
}

/* Settings host — own positioned wrapper so the dropdown anchors to the
   trigger button, not to the whole flex cluster. */
.settings-host {
  position: relative;
  display: flex;
  justify-self: start;
}

.settings-toggle {
  appearance: none;
}
.settings-label {
  font-size: 0.72rem;
  font-weight: 600;
  letter-spacing: 0.03em;
}

/* Dropdown transition */
.dropdown-enter-active, .dropdown-leave-active {
  transition: opacity 0.18s ease, transform 0.18s ease;
}
.dropdown-enter-from, .dropdown-leave-to {
  opacity: 0;
  transform: translateY(-6px) scale(0.96);
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

/* Mobile adjustments for viewport overlays.
 * On mobile (<= 640px) the sidebar is hidden and space is limited.
 * We hide character name/meta (the user already knows their character)
 * and show only the essential controls: mode toggle (top-left), settings
 * gear (top-right beside AI pill), with brain/music below. */
@media (max-width: 640px) {
  /* Hide character name & meta on mobile — screen is too narrow */
  .character-name-overlay { display: none; }
  .character-meta-overlay { display: none; }
  /* Settings gear: compact circle in top-right area */
  .settings-toggle {
    height: 32px;
    width: 32px;
    padding: 0;
    font-size: 0.7rem;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .settings-label { display: none; }
  .corner-cluster {
    top: 6px;
    right: 62px;
  }
}
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
  border: 3px solid var(--ts-border, rgba(255, 255, 255, 0.15));
  border-top-color: var(--ts-accent, rgba(108, 99, 255, 0.9));
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.loading-text {
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--ts-viewport-text-med);
  letter-spacing: 0.05em;
}

.load-error-overlay {
  background: rgba(0, 0, 0, 0.45);
  pointer-events: auto;
}

.load-error-icon {
  font-size: 2rem;
}

.load-error-retry {
  margin-top: 4px;
  padding: 6px 20px;
  border: 1px solid var(--ts-accent, rgba(108, 99, 255, 0.5));
  border-radius: 6px;
  background: var(--ts-accent, rgba(108, 99, 255, 0.6));
  color: var(--ts-text-on-accent, #fff);
  font-size: 0.8rem;
  cursor: pointer;
  transition: background 0.2s;
}
.load-error-retry:hover {
  opacity: 0.85;
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
