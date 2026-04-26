<script setup lang="ts">
import { useTheme } from '../composables/useTheme';
import type { ThemeDefinition } from '../config/themes';

const { themeId, themes, setTheme } = useTheme();

function pickTheme(theme: ThemeDefinition): void {
  setTheme(theme.id);
}
</script>

<template>
  <div class="theme-picker" data-testid="theme-picker">
    <div class="tp-header">
      <span class="tp-title">🎨 Appearance</span>
    </div>

    <div class="tp-grid">
      <button
        v-for="theme in themes"
        :key="theme.id"
        class="tp-card"
        :class="{ 'tp-card--active': themeId === theme.id }"
        :data-testid="`theme-${theme.id}`"
        :title="theme.description"
        @click="pickTheme(theme)"
      >
        <!-- Color preview dots -->
        <div class="tp-preview">
          <span
            class="tp-dot"
            :style="{ background: theme.tokens['--ts-bg-base'] || '#0f172a' }"
          />
          <span
            class="tp-dot"
            :style="{ background: theme.tokens['--ts-accent'] || '#7c6fff' }"
          />
          <span
            class="tp-dot"
            :style="{ background: theme.tokens['--ts-text-primary'] || '#f1f5f9' }"
          />
        </div>

        <span class="tp-icon">{{ theme.icon }}</span>
        <span class="tp-label">{{ theme.label }}</span>
        <span class="tp-category">{{ theme.category }}</span>
      </button>
    </div>
  </div>
</template>

<style scoped>
.theme-picker {
  display: flex;
  flex-direction: column;
  gap: var(--ts-space-md);
}

.tp-header {
  display: flex;
  align-items: center;
  gap: var(--ts-space-sm);
}

.tp-title {
  font-size: var(--ts-text-sm);
  font-weight: 600;
  color: var(--ts-text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.tp-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(100px, 1fr));
  gap: var(--ts-space-sm);
}

.tp-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: var(--ts-space-md) var(--ts-space-sm);
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-md);
  background: var(--ts-bg-input);
  cursor: pointer;
  transition: all var(--ts-transition-fast);
  position: relative;
  overflow: hidden;
}

.tp-card:hover {
  background: var(--ts-bg-hover);
  border-color: var(--ts-accent);
  transform: translateY(-1px);
}

.tp-card--active {
  border-color: var(--ts-accent);
  background: var(--ts-accent-glow);
  box-shadow: 0 0 12px var(--ts-accent-glow);
}

.tp-card--active::after {
  content: '✓';
  position: absolute;
  top: 4px;
  right: 6px;
  font-size: 10px;
  color: var(--ts-accent);
  font-weight: 700;
}

.tp-preview {
  display: flex;
  gap: 3px;
  margin-bottom: 2px;
}

.tp-dot {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  border: 1px solid var(--ts-border);
}

.tp-icon {
  font-size: 1.2rem;
  line-height: 1;
}

.tp-label {
  font-size: var(--ts-text-xs);
  font-weight: 600;
  color: var(--ts-text-primary);
  white-space: nowrap;
}

.tp-category {
  font-size: 9px;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--ts-text-muted);
  font-weight: 600;
}
</style>
