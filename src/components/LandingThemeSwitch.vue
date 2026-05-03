<script setup lang="ts">
/**
 * Compact theme switcher for the browser landing page nav.
 * Wraps the shared `ThemePicker` grid in a popover so visitors get the
 * same palette as the desktop Settings panel.
 */
import { onBeforeUnmount, onMounted, ref } from 'vue';
import ThemePicker from './ThemePicker.vue';
import { useTheme } from '../composables/useTheme';

const { activeTheme } = useTheme();
const open = ref(false);

function onDocumentClick(event: MouseEvent) {
  if (!open.value) return;
  const target = event.target as Element | null;
  if (target && !target.closest('.theme-popover-host')) {
    open.value = false;
  }
}

onMounted(() => document.addEventListener('click', onDocumentClick));
onBeforeUnmount(() => document.removeEventListener('click', onDocumentClick));
</script>

<template>
  <div
    class="theme-popover-host"
    @keydown.escape.stop="open = false"
  >
    <button
      type="button"
      class="theme-toggle"
      :aria-expanded="open"
      aria-haspopup="dialog"
      aria-label="Change app theme"
      :title="`Theme: ${activeTheme.label}`"
      @click="open = !open"
    >
      <span
        class="theme-toggle-swatch"
        :style="{ background: activeTheme.preview.accent }"
        aria-hidden="true"
      />
      <span
        class="theme-toggle-icon"
        aria-hidden="true"
      >🎨</span>
    </button>
    <Transition name="theme-pop">
      <div
        v-if="open"
        class="theme-popover"
        role="dialog"
        aria-label="Theme picker"
        @click.stop
      >
        <ThemePicker />
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.theme-popover-host { position: relative; display: inline-flex; }

.theme-toggle {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.45rem 0.6rem;
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-pill);
  background: color-mix(in srgb, var(--ts-bg-panel) 70%, transparent);
  color: var(--ts-text-secondary);
  cursor: pointer;
  transition:
    transform var(--ts-transition-fast, 0.15s ease),
    border-color var(--ts-transition-fast, 0.15s ease),
    background var(--ts-transition-fast, 0.15s ease);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
}

.theme-toggle:hover,
.theme-toggle[aria-expanded="true"] {
  color: var(--ts-text-primary);
  border-color: color-mix(in srgb, var(--ts-accent) 55%, var(--ts-border));
  background: color-mix(in srgb, var(--ts-bg-panel) 88%, transparent);
}

.theme-toggle-swatch {
  width: 0.85rem;
  height: 0.85rem;
  border-radius: 50%;
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--ts-accent) 35%, transparent);
}

.theme-toggle-icon { font-size: 0.95rem; line-height: 1; }

.theme-popover {
  position: absolute;
  top: calc(100% + 0.6rem);
  right: 0;
  z-index: 50;
  width: min(320px, calc(100vw - 2rem));
  padding: var(--ts-space-md);
  border: 1px solid color-mix(in srgb, var(--ts-accent) 25%, var(--ts-border));
  border-radius: var(--ts-radius-lg);
  background: color-mix(in srgb, var(--ts-bg-panel) 95%, transparent);
  box-shadow:
    0 24px 60px -20px color-mix(in srgb, var(--ts-accent) 40%, transparent),
    var(--ts-shadow-lg);
  backdrop-filter: blur(24px) saturate(150%);
  -webkit-backdrop-filter: blur(24px) saturate(150%);
}

.theme-pop-enter-active,
.theme-pop-leave-active {
  transition:
    opacity var(--ts-transition-fast, 0.15s ease),
    transform var(--ts-transition-fast, 0.15s ease);
}

.theme-pop-enter-from,
.theme-pop-leave-to {
  opacity: 0;
  transform: translateY(-4px) scale(0.98);
}
</style>
