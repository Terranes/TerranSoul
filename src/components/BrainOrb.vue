<script setup lang="ts">
/**
 * BrainOrb — neon side-profile brain emblem with radiating synapse antennas.
 *
 * Ported from the reference design (Rag Brain/brain-panel-sections.jsx). Three
 * lighting states (off / dim / bright) control glow intensity. Used by the
 * Brain panel cockpit hero and any other surface that wants the live "brain"
 * visual.
 *
 * All CSS lives in src/styles/brain-panel.css under .bp-orb. The component is
 * pure SVG so it scales to any size via the wrapper.
 */

defineProps<{
  lighting?: 'off' | 'dim' | 'bright';
  state?: 'healthy' | 'degraded' | 'offline';
}>();

// 14 synapse antennas around the brain — each a dotted line + dots + endpoint.
const ANTENNAS = [
  { x1: 72, y1: 86, x2: 46, y2: 56, mid: [[60, 73]] },
  { x1: 92, y1: 78, x2: 84, y2: 38, mid: [[88, 56]] },
  { x1: 112, y1: 75, x2: 112, y2: 32, mid: [[112, 50], [112, 40]] },
  { x1: 134, y1: 76, x2: 142, y2: 36, mid: [[138, 54]] },
  { x1: 156, y1: 82, x2: 178, y2: 48, mid: [[168, 64]] },
  { x1: 174, y1: 98, x2: 206, y2: 82, mid: [[190, 88]] },
  { x1: 184, y1: 122, x2: 218, y2: 118, mid: [[202, 120]] },
  { x1: 178, y1: 146, x2: 208, y2: 158, mid: [[194, 152]] },
  { x1: 168, y1: 162, x2: 188, y2: 192, mid: [[178, 178]] },
  { x1: 148, y1: 170, x2: 152, y2: 208, mid: [[150, 190]] },
  { x1: 116, y1: 174, x2: 112, y2: 214, mid: [[114, 196], [114, 206]] },
  { x1: 86, y1: 162, x2: 68, y2: 196, mid: [[78, 180]] },
  { x1: 62, y1: 142, x2: 36, y2: 158, mid: [[48, 150]] },
  { x1: 58, y1: 116, x2: 26, y2: 110, mid: [[40, 112]] },
];

const STATE_LABEL = {
  healthy: { label: 'BRAIN READY', word: 'online' },
  degraded: { label: 'DEGRADED RECALL', word: 'degraded' },
  offline: { label: 'BRAIN OFFLINE', word: 'offline' },
} as const;
</script>

<template>
  <div
    class="bp-orb"
    :data-lighting="lighting ?? 'bright'"
  >
    <svg
      class="bp-orb-svg"
      viewBox="0 0 240 240"
      aria-hidden="true"
    >
      <defs>
        <linearGradient
          id="bpNeonGrad"
          x1="0%"
          y1="20%"
          x2="100%"
          y2="80%"
        >
          <stop
            offset="0%"
            stop-color="#22e6ff"
          />
          <stop
            offset="35%"
            stop-color="#7a8bff"
          />
          <stop
            offset="65%"
            stop-color="#ff6bb5"
          />
          <stop
            offset="100%"
            stop-color="#ff9450"
          />
        </linearGradient>
        <radialGradient
          id="bpBrainHalo"
          cx="50%"
          cy="50%"
          r="50%"
        >
          <stop
            offset="0%"
            stop-color="#7a8bff"
            stop-opacity="0.30"
          />
          <stop
            offset="50%"
            stop-color="#22e6ff"
            stop-opacity="0.10"
          />
          <stop
            offset="100%"
            stop-color="#22e6ff"
            stop-opacity="0"
          />
        </radialGradient>
        <filter
          id="bpNeonBloom"
          x="-30%"
          y="-30%"
          width="160%"
          height="160%"
        >
          <feGaussianBlur
            stdDeviation="2.4"
            result="b1"
          />
          <feGaussianBlur
            stdDeviation="6"
            in="SourceGraphic"
            result="b2"
          />
          <feMerge>
            <feMergeNode in="b2" />
            <feMergeNode in="b1" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
        <filter
          id="bpDotGlow"
          x="-100%"
          y="-100%"
          width="300%"
          height="300%"
        >
          <feGaussianBlur stdDeviation="1.6" />
          <feMerge>
            <feMergeNode />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
      </defs>

      <!-- halo behind brain -->
      <circle
        class="brain-halo"
        cx="120"
        cy="120"
        r="100"
        fill="url(#bpBrainHalo)"
      />

      <!-- synapse antennas -->
      <g
        class="brain-synapses"
        stroke="url(#bpNeonGrad)"
        stroke-linecap="round"
        fill="none"
      >
        <g
          v-for="(a, i) in ANTENNAS"
          :key="i"
          :class="`brain-syn brain-syn-${i % 7}`"
        >
          <line
            :x1="a.x1"
            :y1="a.y1"
            :x2="a.x2"
            :y2="a.y2"
            stroke-width="0.8"
            stroke-dasharray="0.6 4"
            opacity="0.85"
          />
          <circle
            v-for="(pt, j) in a.mid"
            :key="j"
            :cx="pt[0]"
            :cy="pt[1]"
            r="1.2"
            fill="url(#bpNeonGrad)"
            stroke="none"
            filter="url(#bpDotGlow)"
          />
          <circle
            :cx="a.x2"
            :cy="a.y2"
            r="2.8"
            fill="url(#bpNeonGrad)"
            stroke="none"
            filter="url(#bpDotGlow)"
          />
        </g>
      </g>

      <!-- the brain itself: two hemispheres + inner sulci -->
      <g class="brain-shape">
        <!-- bloom underlay -->
        <g
          class="brain-glow"
          fill="none"
          stroke="url(#bpNeonGrad)"
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="5"
          opacity="0.7"
          filter="url(#bpNeonBloom)"
        >
          <path d="M 121.96 63.15 a 23.49 23.49 0 1 0 -46.957 0.979 31.32 31.32 0 0 0 -19.779 45.179 31.32 31.32 0 0 0 4.353 51.584 A 31.32 31.32 0 1 0 121.96 164.94 Z" />
          <path d="M 121.96 63.15 a 23.49 23.49 0 1 1 46.957 0.979 31.32 31.32 0 0 1 19.779 45.179 31.32 31.32 0 0 1 -4.353 51.584 A 31.32 31.32 0 1 1 121.96 164.94 Z" />
          <path d="M 145.45 125.79 a 35.235 35.235 0 0 1 -23.49 -31.32 35.235 35.235 0 0 1 -23.49 31.32" />
          <path d="M 165.8 74.895 a 23.49 23.49 0 0 0 3.124 -10.766" />
          <path d="M 75.003 64.129 A 23.49 23.49 0 0 0 78.12 74.895" />
          <path d="M 55.225 109.316 a 31.32 31.32 0 0 1 4.581 -3.101" />
          <path d="M 184.115 106.215 a 31.32 31.32 0 0 1 4.581 3.101" />
          <path d="M 74.98 164.94 a 31.32 31.32 0 0 1 -15.402 -4.04" />
          <path d="M 184.342 160.9 A 31.32 31.32 0 0 1 168.94 164.94" />
        </g>

        <!-- sharp top stroke -->
        <g
          class="brain-line"
          fill="none"
          stroke="url(#bpNeonGrad)"
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2.4"
        >
          <path d="M 121.96 63.15 a 23.49 23.49 0 1 0 -46.957 0.979 31.32 31.32 0 0 0 -19.779 45.179 31.32 31.32 0 0 0 4.353 51.584 A 31.32 31.32 0 1 0 121.96 164.94 Z" />
          <path d="M 121.96 63.15 a 23.49 23.49 0 1 1 46.957 0.979 31.32 31.32 0 0 1 19.779 45.179 31.32 31.32 0 0 1 -4.353 51.584 A 31.32 31.32 0 1 1 121.96 164.94 Z" />
          <path d="M 145.45 125.79 a 35.235 35.235 0 0 1 -23.49 -31.32 35.235 35.235 0 0 1 -23.49 31.32" />
          <path d="M 165.8 74.895 a 23.49 23.49 0 0 0 3.124 -10.766" />
          <path d="M 75.003 64.129 A 23.49 23.49 0 0 0 78.12 74.895" />
          <path d="M 55.225 109.316 a 31.32 31.32 0 0 1 4.581 -3.101" />
          <path d="M 184.115 106.215 a 31.32 31.32 0 0 1 4.581 3.101" />
          <path d="M 74.98 164.94 a 31.32 31.32 0 0 1 -15.402 -4.04" />
          <path d="M 184.342 160.9 A 31.32 31.32 0 0 1 168.94 164.94" />

          <!-- inner sulci squiggles -->
          <path
            d="M 92 102 C 100 110, 108 110, 112 102 M 128 102 C 132 110, 140 110, 148 102"
            stroke-width="1.6"
            opacity="0.85"
          />
          <path
            d="M 96 134 C 106 140, 116 140, 122 134 M 118 134 C 124 140, 134 140, 144 134"
            stroke-width="1.4"
            opacity="0.85"
          />
        </g>
      </g>

      <!-- neural-firing dots ON the brain -->
      <g
        class="brain-dots"
        fill="url(#bpNeonGrad)"
        stroke="none"
        filter="url(#bpDotGlow)"
      >
        <circle cx="98" cy="96" r="1.8" />
        <circle cx="128" cy="84" r="1.5" />
        <circle cx="158" cy="96" r="1.8" />
        <circle cx="112" cy="118" r="1.5" />
        <circle cx="148" cy="118" r="1.8" />
        <circle cx="120" cy="138" r="1.5" />
        <circle cx="86" cy="124" r="1.5" />
        <circle cx="172" cy="124" r="1.5" />
        <circle cx="106" cy="156" r="1.4" />
        <circle cx="138" cy="156" r="1.4" />
      </g>
    </svg>

    <div
      class="bp-orb-status"
      :data-state="STATE_LABEL[state ?? 'healthy'].word"
    >
      {{ STATE_LABEL[state ?? 'healthy'].label }}
    </div>
  </div>
</template>
