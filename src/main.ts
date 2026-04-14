import { createApp } from 'vue';
import { createPinia } from 'pinia';
import { inject } from '@vercel/analytics';
import App from './App.vue';
import './style.css';

try {
  inject();
} catch {
  // analytics failures must not block app startup
}

const app = createApp(App);
app.use(createPinia());
app.mount('#app');
