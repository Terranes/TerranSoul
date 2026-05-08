<template>
  <div class="design-showcase">
    <h2 class="showcase-title">Design System Showcase</h2>
    <p class="showcase-subtitle">
      Validates TerranSoul tokens against docs/DESIGN.md spec
    </p>

    <!-- Color Palette -->
    <section class="showcase-section">
      <h3 class="section-heading">Color Palette</h3>
      <div class="swatch-grid">
        <div
          v-for="swatch in colorSwatches"
          :key="swatch.token"
          class="swatch-card"
        >
          <div
            class="swatch-block"
            :style="{ background: `var(${swatch.token})` }"
          />
          <span class="swatch-name">{{ swatch.name }}</span>
          <code class="swatch-token">{{ swatch.token }}</code>
        </div>
      </div>
    </section>

    <!-- Typography -->
    <section class="showcase-section">
      <h3 class="section-heading">Typography Scale</h3>
      <div class="type-stack">
        <p class="type-sample type-xl">XL — Page Titles (1.35rem)</p>
        <p class="type-sample type-lg">LG — Section Headings (1.1rem)</p>
        <p class="type-sample type-base">Base — Body Text (0.9rem)</p>
        <p class="type-sample type-sm">SM — Captions (0.8rem)</p>
        <p class="type-sample type-xs">XS — Badges (0.7rem)</p>
      </div>
      <div class="type-mono">
        <code class="mono-sample">const brain = useBrainStore() // JetBrains Mono</code>
      </div>
    </section>

    <!-- Spacing -->
    <section class="showcase-section">
      <h3 class="section-heading">Spacing Scale</h3>
      <div class="spacing-stack">
        <div v-for="space in spacingScale" :key="space.token" class="spacing-row">
          <div class="spacing-bar" :style="{ width: `var(${space.token})` }" />
          <span class="spacing-label">{{ space.name }} — {{ space.token }}</span>
        </div>
      </div>
    </section>

    <!-- Radius -->
    <section class="showcase-section">
      <h3 class="section-heading">Border Radius</h3>
      <div class="radius-grid">
        <div
          v-for="r in radiusScale"
          :key="r.token"
          class="radius-box"
          :style="{ borderRadius: `var(${r.token})` }"
        >
          <span class="radius-label">{{ r.name }}</span>
        </div>
      </div>
    </section>

    <!-- Shadows -->
    <section class="showcase-section">
      <h3 class="section-heading">Elevation / Shadows</h3>
      <div class="shadow-grid">
        <div class="shadow-card shadow-sm">--ts-shadow-sm</div>
        <div class="shadow-card shadow-md">--ts-shadow-md</div>
        <div class="shadow-card shadow-lg">--ts-shadow-lg</div>
      </div>
    </section>

    <!-- Component Examples -->
    <section class="showcase-section">
      <h3 class="section-heading">Component Patterns</h3>
      <div class="component-grid">
        <button class="btn-primary-demo">Primary Button</button>
        <button class="btn-secondary-demo">Secondary Button</button>
        <div class="card-demo">
          <h4 class="card-demo-title">Card Pattern</h4>
          <p class="card-demo-text">Surface with subtle border and shadow.</p>
        </div>
        <div class="message-demo">
          <div class="message-demo--assistant">
            Assistant message with accent border
          </div>
        </div>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
const colorSwatches = [
  { name: 'Cosmic Violet', token: '--ts-accent' },
  { name: 'Violet Hover', token: '--ts-accent-hover' },
  { name: 'Sky Blue', token: '--ts-accent-blue' },
  { name: 'Soft Violet', token: '--ts-accent-violet' },
  { name: 'Success', token: '--ts-success' },
  { name: 'Warning', token: '--ts-warning' },
  { name: 'Error', token: '--ts-error' },
  { name: 'Info', token: '--ts-info' },
  { name: 'Void Navy (bg)', token: '--ts-bg-base' },
  { name: 'Deep Slate (surface)', token: '--ts-bg-surface' },
  { name: 'Frost (text)', token: '--ts-text-primary' },
  { name: 'Steel (secondary)', token: '--ts-text-secondary' },
]

const spacingScale = [
  { name: 'XS (4px)', token: '--ts-space-xs' },
  { name: 'SM (8px)', token: '--ts-space-sm' },
  { name: 'MD (12px)', token: '--ts-space-md' },
  { name: 'LG (16px)', token: '--ts-space-lg' },
  { name: 'XL (24px)', token: '--ts-space-xl' },
]

const radiusScale = [
  { name: 'SM (6px)', token: '--ts-radius-sm' },
  { name: 'MD (10px)', token: '--ts-radius-md' },
  { name: 'LG (14px)', token: '--ts-radius-lg' },
  { name: 'XL (20px)', token: '--ts-radius-xl' },
  { name: 'Pill', token: '--ts-radius-pill' },
]
</script>

<style scoped>
.design-showcase {
  padding: var(--ts-space-xl);
  background: var(--ts-bg-base);
  color: var(--ts-text-primary);
  font-family: var(--ts-font-family);
  min-height: 100%;
  overflow-y: auto;
}

.showcase-title {
  font-size: var(--ts-text-xl);
  font-weight: 600;
  margin-bottom: var(--ts-space-xs);
}

.showcase-subtitle {
  font-size: var(--ts-text-sm);
  color: var(--ts-text-secondary);
  margin-bottom: var(--ts-space-xl);
}

.showcase-section {
  margin-bottom: var(--ts-space-xl);
  padding: var(--ts-space-lg);
  background: var(--ts-bg-surface);
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-md);
}

.section-heading {
  font-size: var(--ts-text-lg);
  font-weight: 500;
  margin-bottom: var(--ts-space-md);
  color: var(--ts-accent);
}

/* Color swatches */
.swatch-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
  gap: var(--ts-space-sm);
}

.swatch-card {
  display: flex;
  flex-direction: column;
  gap: var(--ts-space-xs);
  align-items: center;
}

.swatch-block {
  width: 100%;
  height: 48px;
  border-radius: var(--ts-radius-sm);
  border: 1px solid var(--ts-border);
}

.swatch-name {
  font-size: var(--ts-text-xs);
  color: var(--ts-text-secondary);
}

.swatch-token {
  font-size: var(--ts-text-xs);
  font-family: var(--ts-font-mono);
  color: var(--ts-text-muted);
}

/* Typography */
.type-stack {
  display: flex;
  flex-direction: column;
  gap: var(--ts-space-sm);
}

.type-sample {
  margin: 0;
}

.type-xl { font-size: var(--ts-text-xl); font-weight: 600; }
.type-lg { font-size: var(--ts-text-lg); font-weight: 500; }
.type-base { font-size: var(--ts-text-base); }
.type-sm { font-size: var(--ts-text-sm); color: var(--ts-text-secondary); }
.type-xs { font-size: var(--ts-text-xs); color: var(--ts-text-muted); }

.type-mono {
  margin-top: var(--ts-space-md);
  padding: var(--ts-space-md);
  background: var(--ts-bg-elevated);
  border-radius: var(--ts-radius-sm);
}

.mono-sample {
  font-family: var(--ts-font-mono);
  font-size: var(--ts-text-sm);
  color: var(--ts-accent-blue);
}

/* Spacing */
.spacing-stack {
  display: flex;
  flex-direction: column;
  gap: var(--ts-space-sm);
}

.spacing-row {
  display: flex;
  align-items: center;
  gap: var(--ts-space-md);
}

.spacing-bar {
  height: 16px;
  background: var(--ts-accent);
  border-radius: var(--ts-radius-sm);
  min-width: 4px;
}

.spacing-label {
  font-size: var(--ts-text-sm);
  color: var(--ts-text-secondary);
  font-family: var(--ts-font-mono);
}

/* Radius */
.radius-grid {
  display: flex;
  gap: var(--ts-space-md);
  flex-wrap: wrap;
}

.radius-box {
  width: 80px;
  height: 80px;
  background: var(--ts-bg-elevated);
  border: 1px solid var(--ts-border);
  display: flex;
  align-items: center;
  justify-content: center;
}

.radius-label {
  font-size: var(--ts-text-xs);
  color: var(--ts-text-secondary);
}

/* Shadows */
.shadow-grid {
  display: flex;
  gap: var(--ts-space-lg);
  flex-wrap: wrap;
}

.shadow-card {
  width: 140px;
  height: 80px;
  background: var(--ts-bg-elevated);
  border-radius: var(--ts-radius-md);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: var(--ts-text-xs);
  font-family: var(--ts-font-mono);
  color: var(--ts-text-secondary);
}

.shadow-sm { box-shadow: var(--ts-shadow-sm); }
.shadow-md { box-shadow: var(--ts-shadow-md); }
.shadow-lg { box-shadow: var(--ts-shadow-lg); }

/* Component demos */
.component-grid {
  display: flex;
  flex-direction: column;
  gap: var(--ts-space-md);
}

.btn-primary-demo {
  background: var(--ts-accent);
  color: var(--ts-text-on-accent);
  border: none;
  border-radius: var(--ts-radius-sm);
  padding: var(--ts-space-sm) var(--ts-space-lg);
  font-weight: 500;
  font-family: var(--ts-font-family);
  font-size: var(--ts-text-base);
  cursor: pointer;
  transition: background var(--ts-transition-fast);
  align-self: flex-start;
}

.btn-primary-demo:hover {
  background: var(--ts-accent-hover);
}

.btn-secondary-demo {
  background: transparent;
  color: var(--ts-accent);
  border: 1px solid var(--ts-accent);
  border-radius: var(--ts-radius-sm);
  padding: var(--ts-space-sm) var(--ts-space-lg);
  font-weight: 500;
  font-family: var(--ts-font-family);
  font-size: var(--ts-text-base);
  cursor: pointer;
  transition: all var(--ts-transition-fast);
  align-self: flex-start;
}

.btn-secondary-demo:hover {
  background: var(--ts-accent-glow);
}

.card-demo {
  background: var(--ts-bg-surface);
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-md);
  padding: var(--ts-space-lg);
  box-shadow: var(--ts-shadow-sm);
  transition: box-shadow var(--ts-transition-fast);
  max-width: 320px;
}

.card-demo:hover {
  box-shadow: var(--ts-shadow-md);
}

.card-demo-title {
  font-size: var(--ts-text-lg);
  font-weight: 500;
  margin-bottom: var(--ts-space-xs);
}

.card-demo-text {
  font-size: var(--ts-text-sm);
  color: var(--ts-text-secondary);
}

.message-demo {
  max-width: 400px;
}

.message-demo--assistant {
  background: var(--ts-bg-surface);
  border: 1px solid var(--ts-border-subtle);
  border-left: 3px solid var(--ts-accent);
  border-radius: var(--ts-radius-md);
  padding: var(--ts-space-md);
  font-size: var(--ts-text-base);
  color: var(--ts-text-primary);
}
</style>
