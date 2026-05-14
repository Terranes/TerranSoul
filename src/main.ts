import { createApp } from 'vue';
import { createPinia } from 'pinia';
import PrimeVue from 'primevue/config';
import Aura from '@primeuix/themes/aura';
import App from './App.vue';
import './style.css';
import './styles/brain-panel.css';

const app = createApp(App);
app.use(createPinia());
/**
 * PrimeVue v4 — Vue 3 component framework (MIT, TypeScript-native, tree-shakeable).
 *
 * This is the project-mandated UI framework — see the "UI Framework — No CSS
 * Hacking" section in `rules/coding-standards.md`.  Use PrimeVue components
 * (Button, Toolbar, Popover, Menu, Drawer, Dialog, Toast, Tag, …) for any new
 * UI surface instead of hand-rolled absolutely-positioned siblings or
 * hand-tuned `top`/`right` magic numbers.
 *
 * The Aura preset gives PrimeVue components a visual baseline; theming is
 * still driven by our `--ts-*` tokens (per-theme blocks in `src/style.css`).
 * `cssLayer.name = 'primevue'` lets our cascade override PrimeVue defaults
 * cleanly.
 */
app.use(PrimeVue, {
  theme: {
    preset: Aura,
    options: {
      darkModeSelector: 'html[data-theme]:not([data-theme="corporate"]):not([data-theme="pastel"])',
      cssLayer: { name: 'primevue', order: 'tailwind-base, primevue, tailwind-utilities' },
    },
  },
  ripple: true,
});
app.mount('#app');
