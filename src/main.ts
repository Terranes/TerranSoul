import { createApp } from 'vue';
import { createPinia } from 'pinia';
import { createRouter, createMemoryHistory } from 'vue-router';
import App from './App.vue';
import './style.css';

// Minimal router required by @vercel/analytics and @vercel/speed-insights
// which unconditionally call useRoute() in their setup functions.
const router = createRouter({
  history: createMemoryHistory(),
  routes: [{ path: '/:pathMatch(.*)*', component: App }],
});

const app = createApp(App);
app.use(createPinia());
app.use(router);
app.mount('#app');
