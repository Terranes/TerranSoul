<template>
  <div
    :class="['brain-avatar', `mood-${mood}`, `expression-${expression}`, { 'no-brain': !active }]"
    :style="cssVars"
    role="img"
    :aria-label="ariaLabel"
    data-testid="brain-avatar"
  >
    <svg
      class="brain-svg"
      :width="size"
      :height="size"
      viewBox="0 0 200 200"
      xmlns="http://www.w3.org/2000/svg"
      aria-hidden="true"
    >
      <!-- Soft outer aura (only when active) -->
      <defs>
        <radialGradient
          :id="gradientId"
          cx="50%"
          cy="45%"
          r="60%"
        >
          <stop
            offset="0%"
            :stop-color="colors.aura"
            stop-opacity="0.55"
          />
          <stop
            offset="70%"
            :stop-color="colors.aura"
            stop-opacity="0.05"
          />
          <stop
            offset="100%"
            :stop-color="colors.aura"
            stop-opacity="0"
          />
        </radialGradient>
        <linearGradient
          :id="leftHemiId"
          x1="0%"
          y1="0%"
          x2="100%"
          y2="100%"
        >
          <stop
            offset="0%"
            :stop-color="colors.left"
          />
          <stop
            offset="100%"
            :stop-color="colors.leftDeep"
          />
        </linearGradient>
        <linearGradient
          :id="rightHemiId"
          x1="100%"
          y1="0%"
          x2="0%"
          y2="100%"
        >
          <stop
            offset="0%"
            :stop-color="colors.right"
          />
          <stop
            offset="100%"
            :stop-color="colors.rightDeep"
          />
        </linearGradient>
      </defs>

      <!-- Aura (the pulsing glow when active) -->
      <circle
        v-if="active"
        class="brain-aura"
        cx="100"
        cy="95"
        r="92"
        :fill="`url(#${gradientId})`"
      />

      <!-- Synapse dots — count scales with memory density -->
      <g
        v-if="active"
        class="brain-synapses"
        data-testid="brain-synapses"
      >
        <circle
          v-for="(s, i) in synapses"
          :key="i"
          :cx="s.x"
          :cy="s.y"
          :r="s.r"
          :fill="colors.spark"
          :style="{ animationDelay: s.delay + 's' }"
          class="brain-synapse-dot"
        />
      </g>

      <!-- Brain body: two hemispheres + cerebellum -->
      <g class="brain-body">
        <!-- Cerebellum (bottom) — subtle rounded base -->
        <ellipse
          cx="100"
          cy="148"
          rx="44"
          ry="20"
          :fill="colors.cerebellum"
        />

        <!-- Left hemisphere -->
        <path
          class="brain-hemi left"
          :fill="`url(#${leftHemiId})`"
          d="M 95 38
             C 60 38, 30 65, 32 100
             C 30 125, 50 150, 80 152
             C 92 152, 95 145, 95 132
             Z"
        />
        <!-- Right hemisphere -->
        <path
          class="brain-hemi right"
          :fill="`url(#${rightHemiId})`"
          d="M 105 38
             C 140 38, 170 65, 168 100
             C 170 125, 150 150, 120 152
             C 108 152, 105 145, 105 132
             Z"
        />

        <!-- Sulci / brain folds (non-functional decoration) -->
        <g
          class="brain-sulci"
          :stroke="colors.fold"
          stroke-width="1.5"
          fill="none"
          stroke-linecap="round"
        >
          <path d="M 55 70 Q 70 75, 65 95" />
          <path d="M 50 105 Q 70 105, 75 125" />
          <path d="M 78 55 Q 88 75, 80 95" />
          <path d="M 145 70 Q 130 75, 135 95" />
          <path d="M 150 105 Q 130 105, 125 125" />
          <path d="M 122 55 Q 112 75, 120 95" />
        </g>

        <!-- Central fissure -->
        <path
          d="M 100 38 L 100 152"
          :stroke="colors.fissure"
          stroke-width="2"
          fill="none"
          opacity="0.55"
        />
      </g>

      <!-- Cute face — expression -->
      <g
        class="brain-face"
        :transform="faceTransform"
      >
        <!-- Eyes -->
        <g
          v-if="expression !== 'sleepy'"
          class="brain-eyes"
        >
          <ellipse
            cx="80"
            cy="100"
            rx="5"
            ry="6"
            :fill="colors.eye"
          />
          <ellipse
            cx="120"
            cy="100"
            rx="5"
            ry="6"
            :fill="colors.eye"
          />
          <!-- Eye sparkle -->
          <circle
            cx="82"
            cy="98"
            r="1.5"
            fill="#ffffff"
          />
          <circle
            cx="122"
            cy="98"
            r="1.5"
            fill="#ffffff"
          />
        </g>
        <!-- Sleepy / no-brain: closed eye lines -->
        <g
          v-else
          class="brain-eyes-closed"
        >
          <path
            d="M 75 100 Q 80 104, 85 100"
            :stroke="colors.eye"
            stroke-width="2"
            fill="none"
            stroke-linecap="round"
          />
          <path
            d="M 115 100 Q 120 104, 125 100"
            :stroke="colors.eye"
            stroke-width="2"
            fill="none"
            stroke-linecap="round"
          />
        </g>

        <!-- Mouth -->
        <path
          v-if="expression === 'happy'"
          d="M 88 118 Q 100 130, 112 118"
          :stroke="colors.mouth"
          stroke-width="2.5"
          fill="none"
          stroke-linecap="round"
        />
        <path
          v-else-if="expression === 'thinking'"
          d="M 92 122 Q 100 119, 108 122"
          :stroke="colors.mouth"
          stroke-width="2.5"
          fill="none"
          stroke-linecap="round"
        />
        <path
          v-else-if="expression === 'sad'"
          d="M 88 124 Q 100 114, 112 124"
          :stroke="colors.mouth"
          stroke-width="2.5"
          fill="none"
          stroke-linecap="round"
        />
        <circle
          v-else-if="expression === 'sleepy'"
          cx="100"
          cy="120"
          r="2"
          :fill="colors.mouth"
        />
        <!-- Default 'idle' -->
        <path
          v-else
          d="M 92 120 Q 100 124, 108 120"
          :stroke="colors.mouth"
          stroke-width="2.5"
          fill="none"
          stroke-linecap="round"
        />

        <!-- Tiny rosy cheeks when happy / idle (active) -->
        <g
          v-if="active && (expression === 'happy' || expression === 'idle')"
          class="brain-cheeks"
        >
          <ellipse
            cx="72"
            cy="115"
            rx="4"
            ry="2.5"
            :fill="colors.cheek"
            opacity="0.7"
          />
          <ellipse
            cx="128"
            cy="115"
            rx="4"
            ry="2.5"
            :fill="colors.cheek"
            opacity="0.7"
          />
        </g>

        <!-- Thinking dots above when expression='thinking' -->
        <g
          v-if="expression === 'thinking'"
          class="brain-thoughts"
          data-testid="brain-thoughts"
        >
          <circle
            cx="138"
            cy="48"
            r="3"
            :fill="colors.spark"
            class="brain-thought-dot"
            style="animation-delay: 0s"
          />
          <circle
            cx="148"
            cy="38"
            r="4"
            :fill="colors.spark"
            class="brain-thought-dot"
            style="animation-delay: 0.3s"
          />
          <circle
            cx="160"
            cy="26"
            r="5"
            :fill="colors.spark"
            class="brain-thought-dot"
            style="animation-delay: 0.6s"
          />
        </g>
      </g>
    </svg>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import type { BrainMode } from '../types';

/**
 * BrainAvatar — a small animated SVG cute brain used by the unified Brain hub.
 *
 * - Hemisphere colour reflects the current brain mode (free / paid / local / none).
 * - Synapse dot count scales with the user's memory density (`memoryCount`).
 * - `expression` lets parent views show idle / thinking / happy / sad / sleepy.
 *
 * Pure SVG + CSS, no external dependencies. The component is presentational —
 * all state is passed as props so it's trivial to test and reuse in the
 * Brain hub, the Pet overlay, and the system info panel.
 */
const props = withDefaults(
  defineProps<{
    /** Currently configured brain mode, or null when none is set. */
    mode?: BrainMode | null;
    /** Total memory entries — synapse dot count is derived from this. */
    memoryCount?: number;
    /** Animated facial expression. */
    expression?: 'idle' | 'thinking' | 'happy' | 'sad' | 'sleepy';
    /** Pixel size (square). */
    size?: number;
    /** Disable all aura/pulse animation (useful when offscreen). */
    paused?: boolean;
  }>(),
  {
    mode: null,
    memoryCount: 0,
    expression: 'idle',
    size: 160,
    paused: false,
  },
);

const active = computed(() => props.mode !== null);

/** Mood is derived from the brain mode — drives the colour scheme. */
const mood = computed<'none' | 'free' | 'paid' | 'local'>(() => {
  if (!props.mode) return 'none';
  switch (props.mode.mode) {
    case 'free_api': return 'free';
    case 'paid_api': return 'paid';
    case 'local_ollama':
    case 'local_lm_studio':
      return 'local';
    default: return 'none';
  }
});

/** Each mood gets a coordinated palette. Tuned for dark backgrounds. */
const PALETTES: Record<typeof mood['value'], {
  left: string; leftDeep: string;
  right: string; rightDeep: string;
  cerebellum: string; fold: string; fissure: string;
  eye: string; mouth: string; cheek: string; spark: string; aura: string;
}> = {
  none: {
    left: '#5a5e74', leftDeep: '#3a3d4d',
    right: '#5a5e74', rightDeep: '#3a3d4d',
    cerebellum: '#3a3d4d', fold: '#2a2c38', fissure: '#1f2129',
    eye: '#9aa0b4', mouth: '#9aa0b4', cheek: '#7a7e90', spark: '#9aa0b4',
    aura: '#5a5e74',
  },
  free: {
    left: '#7be0b3', leftDeep: '#3aa07a',
    right: '#a6f0c8', rightDeep: '#46b88a',
    cerebellum: '#2c8b67', fold: '#1f6e51', fissure: '#155b41',
    eye: '#0f3a2c', mouth: '#0f3a2c', cheek: '#ff9bb8', spark: '#fff5a0',
    aura: '#7be0b3',
  },
  paid: {
    left: '#7cc8ff', leftDeep: '#3a78cc',
    right: '#a4dcff', rightDeep: '#4a8fdc',
    cerebellum: '#2c69b8', fold: '#1f4f8e', fissure: '#143a6c',
    eye: '#0e2a55', mouth: '#0e2a55', cheek: '#ff9bb8', spark: '#ffe88a',
    aura: '#7cc8ff',
  },
  local: {
    left: '#c8a4ff', leftDeep: '#7a4adc',
    right: '#dabdff', rightDeep: '#8c5fe4',
    cerebellum: '#5e3aa4', fold: '#452680', fissure: '#321c5f',
    eye: '#220e4a', mouth: '#220e4a', cheek: '#ffb0c4', spark: '#fff5a0',
    aura: '#c8a4ff',
  },
};

const colors = computed(() => PALETTES[mood.value]);

/** Synapse positions are deterministic from `memoryCount` so tests are stable. */
const synapses = computed(() => {
  const n = Math.min(12, Math.max(0, Math.floor(Math.sqrt(props.memoryCount))));
  // Position around the brain on a circle, with deterministic jitter.
  const out: Array<{ x: number; y: number; r: number; delay: number }> = [];
  for (let i = 0; i < n; i++) {
    const angle = (i / n) * Math.PI * 2;
    const dist = 88 + ((i * 13) % 12);
    out.push({
      x: 100 + Math.cos(angle) * dist,
      y: 95 + Math.sin(angle) * dist * 0.85,
      r: 2 + (i % 3),
      delay: (i * 0.15) % 2,
    });
  }
  return out;
});

const ariaLabel = computed(() => {
  const moodLabel = {
    none: 'No brain configured',
    free: 'Free cloud brain',
    paid: 'Paid cloud brain',
    local: 'Local LLM brain',
  }[mood.value];
  return `${moodLabel} — ${props.memoryCount} memories — feeling ${props.expression}`;
});

// Unique gradient IDs per instance so multiple BrainAvatars on a page don't
// collide (the SVG <defs> id namespace is global per document).
const _instanceId = Math.random().toString(36).slice(2, 8);
const gradientId = `brain-aura-${_instanceId}`;
const leftHemiId = `brain-left-${_instanceId}`;
const rightHemiId = `brain-right-${_instanceId}`;

/** Tiny face wiggle when "thinking" — pure CSS. */
const faceTransform = computed(() => 'translate(0, 0)');

const cssVars = computed(() => ({
  '--brain-aura-color': colors.value.aura,
  '--brain-anim-state': props.paused ? 'paused' : 'running',
}));
</script>

<style scoped>
.brain-avatar {
  display: inline-flex;
  position: relative;
  align-items: center;
  justify-content: center;
}
.brain-svg {
  display: block;
  filter: drop-shadow(0 6px 18px rgba(0, 0, 0, 0.35));
}

/* Aura pulse — slow breathing. */
.brain-aura {
  transform-origin: 100px 95px;
  animation: brain-aura-pulse 3.6s ease-in-out infinite;
  animation-play-state: var(--brain-anim-state);
}
@keyframes brain-aura-pulse {
  0%, 100% { transform: scale(1); opacity: 0.9; }
  50% { transform: scale(1.06); opacity: 1; }
}

/* Hemispheres breathe gently when active. */
.brain-hemi {
  transform-origin: 100px 95px;
  animation: brain-hemi-breathe 3.6s ease-in-out infinite;
  animation-play-state: var(--brain-anim-state);
}
.brain-hemi.right { animation-delay: 0.1s; }
.no-brain .brain-hemi { animation: none; }
@keyframes brain-hemi-breathe {
  0%, 100% { transform: scale(1); }
  50% { transform: scale(1.02); }
}

/* Synapse dots twinkle. */
.brain-synapse-dot {
  animation: brain-synapse-twinkle 2.4s ease-in-out infinite;
  animation-play-state: var(--brain-anim-state);
}
@keyframes brain-synapse-twinkle {
  0%, 100% { opacity: 0.15; transform: scale(0.8); transform-origin: center; }
  50% { opacity: 1; transform: scale(1.4); }
}

/* Thinking thought bubbles. */
.brain-thought-dot {
  animation: brain-thought-rise 1.8s ease-in-out infinite;
  animation-play-state: var(--brain-anim-state);
  opacity: 0;
}
@keyframes brain-thought-rise {
  0% { opacity: 0; transform: translateY(8px); }
  40%, 60% { opacity: 1; transform: translateY(0); }
  100% { opacity: 0; transform: translateY(-6px); }
}

/* No-brain palette gets a sad, motionless look. */
.no-brain .brain-svg { filter: drop-shadow(0 4px 10px rgba(0, 0, 0, 0.4)) grayscale(0.4); }
.expression-thinking .brain-body { animation: brain-tilt 1.6s ease-in-out infinite; transform-origin: 100px 100px; }
@keyframes brain-tilt {
  0%, 100% { transform: rotate(-1deg); }
  50% { transform: rotate(2deg); }
}
</style>
