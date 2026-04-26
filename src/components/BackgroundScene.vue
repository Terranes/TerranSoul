<template>
  <!--
    BackgroundScene — animated 3D background that adapts to the active theme.

    Architecture
    ------------
    Single fixed-position component injected once at the root of App.vue.
    Inside, a perspective stage hosts:
      • 3 depth layers (.bs-back / .bs-mid / .bs-near) — gradient washes
        translated on the Z axis to create parallax depth;
      • 24 generic particle slots (.bs-p.p1 … .bs-p24) — per-theme CSS
        decorates them as stars, petals, dust, code-lines, etc;
      • a top vignette (.bs-vignette) for edge falloff.

    All visuals (colors, shapes, animations) are defined per theme in
    src/style.css under html[data-theme="*"] .bs-* selectors so this
    component itself stays tiny and theme-agnostic.

    The component is hidden in pet mode (transparent Tauri window) via the
    body[data-ts-mode="pet"] selector in style.css.
  -->
  <div
    class="bs-root"
    aria-hidden="true"
  >
    <div class="bs-stage">
      <div class="bs-back" />
      <div class="bs-mid" />
      <div class="bs-near" />
      <div class="bs-particles">
        <span
          v-for="i in PARTICLE_COUNT"
          :key="i"
          :class="['bs-p', `p${i}`]"
        />
      </div>
      <div class="bs-vignette" />
    </div>
  </div>
</template>

<script setup lang="ts">
/**
 * Particle slot count — kept small so the DOM stays cheap.  Each per-theme
 * CSS block uses :nth-child() to vary position, depth, animation delay and
 * appearance across the 24 slots.
 */
const PARTICLE_COUNT = 24;
</script>

<style>
/* NOT scoped: per-theme overrides in src/style.css target these classes
   directly. The `bs-` prefix prevents collisions with other components. */
.bs-root {
  position: fixed;
  inset: 0;
  z-index: -1;
  pointer-events: none;
  overflow: hidden;
  /* Establish the 3D viewing frustum for child layers. */
  perspective: 1500px;
  perspective-origin: 50% 50%;
}

.bs-stage {
  position: absolute;
  inset: 0;
  transform-style: preserve-3d;
}

/* Depth layers — pure positioning here; per-theme CSS in style.css decorates
   their background and animation. */
.bs-back,
.bs-mid,
.bs-near,
.bs-vignette {
  position: absolute;
  inset: 0;
  pointer-events: none;
  will-change: transform, opacity;
}

.bs-back     { transform: translateZ(-600px) scale(1.50); }
.bs-mid      { transform: translateZ(-250px) scale(1.20); }
.bs-near     { transform: translateZ(-50px)  scale(1.04); }
.bs-vignette { transform: translateZ(0); }

/* Particle layer container — particles position themselves absolutely. */
.bs-particles {
  position: absolute;
  inset: 0;
  transform-style: preserve-3d;
  pointer-events: none;
}

.bs-p {
  position: absolute;
  display: block;
  pointer-events: none;
  will-change: transform, opacity;
}

/* Default styling — per-theme rules in style.css fully override.  Without a
   theme block the scene is invisible (no shape/colour given to particles). */
</style>
