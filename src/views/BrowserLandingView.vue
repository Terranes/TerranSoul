<template>
  <div class="browser-landing">
    <header class="landing-nav">
      <a
        href="#top"
        class="brand-lockup"
        aria-label="TerranSoul — back to top"
      >
        <img
          :src="brandIconSrc"
          alt=""
          class="brand-icon"
        >
        <span class="brand-name">TerranSoul</span>
      </a>
      <nav
        class="landing-links"
        aria-label="Landing page navigation"
      >
        <a href="#features">Features</a>
        <a href="#brain">Brain</a>
        <a href="#lan">LAN</a>
        <a href="#quests">Quests</a>
        <a href="#browser-docs">Docs</a>
        <a
          href="https://github.com/Terranes/TerranSoul"
          target="_blank"
          rel="noopener"
        >GitHub</a>
      </nav>
      <div class="nav-actions">
        <LandingThemeSwitch />
        <button
          type="button"
          class="nav-cta"
          @click="openAppWindow"
        >
          <span
            class="cta-dot"
            aria-hidden="true"
          />
          Open companion
        </button>
      </div>
    </header>

    <main
      id="top"
      class="landing-main"
    >
      <section
        class="hero-section"
        aria-labelledby="landing-title"
      >
        <div class="hero-copy-block">
          <p class="eyebrow">
            <span
              class="eyebrow-pulse"
              aria-hidden="true"
            />
            Open-source 3D AI companion
          </p>
          <h1 id="landing-title">
            TerranSoul
          </h1>
          <p class="hero-copy">
            A Vue 3 + Tauri companion that gives your AI a 3D body, a memory
            system, voice, device sync, coding integrations, and a quest-led
            setup path across local, free, and paid brains.
          </p>
          <div class="hero-actions">
            <button
              type="button"
              class="primary-action"
              @click="openAppWindow"
            >
              Open live companion
            </button>
            <button
              type="button"
              class="secondary-action"
              @click="openProviderModal"
            >
              Connect web LLM
            </button>
            <a
              class="secondary-action"
              href="#lan"
            >Connect LAN</a>
          </div>
          <ul
            class="hero-meta"
            aria-label="Highlights"
          >
            <li><strong>Brain modes</strong><span>OpenRouter, Gemini, NVIDIA, Pollinations, paid APIs, local Ollama</span></li>
            <li><strong>Memory RAG</strong><span>Hybrid retrieval, RRF, HyDE, reranking, decay, tiers</span></li>
            <li><strong>Playable setup</strong><span>30+ skills, combos, and real state detection</span></li>
          </ul>
        </div>
        <aside
          class="pet-stage"
          aria-label="Live TerranSoul pet companion"
        >
          <BrowserPetCompanion @request-provider-connect="openProviderModal" />
        </aside>
      </section>

      <section
        class="proof-strip"
        aria-label="TerranSoul product pillars"
      >
        <article>
          <strong>Browser build</strong>
          <span>Static Vercel-friendly Vue app with direct user-owned provider keys.</span>
        </article>
        <article>
          <strong>Desktop brain</strong>
          <span>Tauri commands, Rust providers, SQLite memory, and local Ollama paths.</span>
        </article>
        <article>
          <strong>Companion layer</strong>
          <span>VRM character, emotions, voice, translator toggle, and pet mode.</span>
        </article>
      </section>

      <section
        id="lan"
        class="lan-section"
        aria-labelledby="lan-title"
      >
        <header class="section-head">
          <p class="card-kicker">
            LAN link
          </p>
          <h2 id="lan-title">
            Connect to a desktop host you already trust.
          </h2>
        </header>
        <div class="lan-layout">
          <article class="lan-limit-panel">
            <strong>Automatic discovery is unavailable</strong>
            <p>{{ browserLan.autoDiscovery.reason }}</p>
          </article>

          <form
            class="lan-connect-panel"
            @submit.prevent="probeLanHost"
          >
            <div class="lan-fields">
              <label class="lan-field host-field">
                <span>Host</span>
                <input
                  v-model="browserLan.hostInput"
                  type="text"
                  inputmode="url"
                  placeholder="192.168.1.42 or https://desktop.local:7422"
                >
              </label>
              <label class="lan-field port-field">
                <span>Port</span>
                <input
                  v-model="browserLan.portInput"
                  type="number"
                  min="1"
                  max="65535"
                >
              </label>
              <label class="lan-secure-toggle">
                <input
                  v-model="browserLan.secure"
                  type="checkbox"
                >
                <span>HTTPS</span>
              </label>
            </div>

            <p
              v-if="browserLan.endpointPreview"
              class="lan-endpoint"
            >
              {{ browserLan.endpointPreview }}
            </p>
            <p
              v-if="browserLan.remoteSummary"
              class="lan-status success"
            >
              Connected: {{ browserLan.remoteSummary }}
            </p>
            <p
              v-if="browserLan.error"
              class="lan-status error"
            >
              {{ browserLan.error }}
            </p>

            <div class="lan-actions">
              <button
                type="submit"
                class="secondary-action lan-button"
                :disabled="browserLan.status === 'probing'"
              >
                {{ browserLan.status === 'probing' ? 'Testing…' : 'Test host' }}
              </button>
              <button
                type="button"
                class="primary-action lan-button"
                :disabled="browserLan.status === 'probing' || (!browserLan.canOpenRemoteChat && !browserLan.hostInput.trim())"
                @click="openRemoteLanChat"
              >
                Open remote chat
              </button>
              <button
                v-if="browserLan.savedHost"
                type="button"
                class="secondary-action lan-button"
                @click="browserLan.clearSaved"
              >
                Forget host
              </button>
            </div>
          </form>
        </div>
      </section>

      <section
        id="features"
        class="feature-grid"
        aria-labelledby="features-title"
      >
        <header class="section-head">
          <p class="card-kicker">
            What you get
          </p>
          <h2 id="features-title">
            Built from the docs up, not as a thin chat skin.
          </h2>
        </header>
        <ul class="feature-cards">
          <li class="feature-card">
            <span
              class="feature-icon"
              aria-hidden="true"
            >01</span>
            <h3>Provider-aware brain</h3>
            <p>
              Browser chat asks for an existing provider when needed; desktop
              can use free APIs, paid APIs, LM Studio, or local Ollama.
            </p>
          </li>
          <li class="feature-card">
            <span
              class="feature-icon"
              aria-hidden="true"
            >02</span>
            <h3>Memory that earns context</h3>
            <p>
              SQLite-backed memories combine vector similarity, keyword match,
              recency, importance, decay, and tier priority before prompt injection.
            </p>
          </li>
          <li class="feature-card">
            <span
              class="feature-icon"
              aria-hidden="true"
            >03</span>
            <h3>Embodied conversation</h3>
            <p>
              Three.js and @pixiv/three-vrm drive expressions, motions, pet mode,
              voice playback, ASR, and translator controls from one character surface.
            </p>
          </li>
          <li class="feature-card">
            <span
              class="feature-icon"
              aria-hidden="true"
            >04</span>
            <h3>Local-first when configured</h3>
            <p>
              Ollama and local embeddings keep private workflows on-device;
              cloud providers stay explicit and user-selected.
            </p>
          </li>
          <li class="feature-card">
            <span
              class="feature-icon"
              aria-hidden="true"
            >05</span>
            <h3>Quest skill tree</h3>
            <p>
              Skills activate from actual app state, so setup becomes a guided
              path through brain, voice, avatar, social, and utility abilities.
            </p>
          </li>
          <li class="feature-card">
            <span
              class="feature-icon"
              aria-hidden="true"
            >06</span>
            <h3>Assistant integrations</h3>
            <p>
              MCP, package-manager workflows, sandboxed plugins, and coding-agent
              routes let the companion become part of a real developer workspace.
            </p>
          </li>
        </ul>
      </section>

      <section
        id="quests"
        class="skill-showcase"
        aria-labelledby="quests-title"
      >
        <div class="skill-copy">
          <p class="card-kicker">
            Skill tree
          </p>
          <h2 id="quests-title">
            Setup is a progression system.
          </h2>
          <p>
            The quest layer reads real store state instead of pretending progress:
            configured brain, saved memories, voice providers, avatar readiness,
            sync links, and companion habits all feed the unlock graph.
          </p>
          <ul class="quest-points">
            <li><strong>Foundation:</strong> awaken brain, speech, avatar, and ambient presence.</li>
            <li><strong>Combos:</strong> abilities unlock together when the app can actually use them.</li>
            <li><strong>Roadmap-ready:</strong> advanced memory, mobile pairing, and assistant tooling stay visible.</li>
          </ul>
        </div>
        <figure class="skill-visual">
          <img
            :src="skillTreeImageUrl"
            alt="TerranSoul skill tree showing connected quest nodes"
          >
          <figcaption>Current quest UI captured from the project recording assets.</figcaption>
        </figure>
      </section>

      <section
        id="brain"
        class="brain-section"
        aria-labelledby="brain-title"
      >
        <header class="section-head">
          <p class="card-kicker">
            Brain system
          </p>
          <h2 id="brain-title">
            Retrieval, provider choice, and persona are first-class systems.
          </h2>
        </header>
        <div class="brain-layout">
          <article class="brain-panel">
            <h3>Memory pipeline</h3>
            <p>
              Chat messages can retrieve long-term context through hybrid search,
              Reciprocal Rank Fusion, optional HyDE, optional reranking, and a
              relevance threshold before memories reach the system prompt.
            </p>
          </article>
          <article class="brain-panel">
            <h3>Provider path</h3>
            <p>
              Static browser mode launches provider pages first: OpenRouter is
              recommended for free model variety, with Gemini, NVIDIA NIM, and
              Pollinations available by user-owned key or token.
            </p>
          </article>
          <article class="brain-panel">
            <h3>Local path</h3>
            <p>
              Desktop mode can route private workloads through local Ollama and
              local embeddings while still preserving the same chat, memory,
              avatar, and quest surfaces.
            </p>
          </article>
        </div>
      </section>

      <section
        id="browser-docs"
        class="docs-panel"
        aria-labelledby="docs-title"
      >
        <header class="section-head">
          <p class="card-kicker">
            Docs
          </p>
          <h2 id="docs-title">
            Browser mode is a real app surface.
          </h2>
        </header>
        <ul>
          <li>No hidden default provider: chat and pet mode request a provider when no backend brain is connected.</li>
          <li>OpenRouter, Gemini, NVIDIA NIM, ChatGPT/OpenAI, and Pollinations launch their provider pages first.</li>
          <li>Manual key entry is a secondary option for direct browser calls from a static deployment.</li>
          <li>LocalStorage keeps only the browser session choice; the Tauri shell unlocks durable memory and local providers.</li>
          <li>The pet preview and chat window reuse the same stores and renderer paths as the desktop-facing UI.</li>
        </ul>
      </section>
    </main>

    <div
      v-if="showProviderModal"
      class="provider-modal-backdrop"
      role="presentation"
      @click.self="closeProviderModal"
    >
      <section
        class="provider-modal"
        role="dialog"
        aria-modal="true"
        aria-label="Choose a web LLM provider"
      >
        <button
          type="button"
          class="provider-modal-close"
          aria-label="Close provider chooser"
          @click="closeProviderModal"
        >
          Close
        </button>
        <BrowserAuthPanel
          compact
          @configured="closeProviderModal"
        />
      </section>
    </div>

    <footer class="landing-footer">
      <p>
        © {{ year }} TerranSoul ·
        <a
          href="https://github.com/Terranes/TerranSoul"
          target="_blank"
          rel="noopener"
        >GitHub</a> ·
        <a
          href="https://github.com/Terranes/TerranSoul/blob/main/LICENSE"
          target="_blank"
          rel="noopener"
        >MIT License</a>
      </p>
    </footer>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import BrowserAuthPanel from '../components/BrowserAuthPanel.vue';
import BrowserPetCompanion from '../components/BrowserPetCompanion.vue';
import LandingThemeSwitch from '../components/LandingThemeSwitch.vue';
import { useBrowserLanStore } from '../stores/browser-lan';
import { setRemoteConversationRuntimeOverride } from '../utils/runtime-target';

const emit = defineEmits<{
  'open-app-window': [];
}>();

const year = computed(() => new Date().getFullYear());
const brandIconSrc = '/icon.png';
const skillTreeImageUrl = new URL('../../recording/skill-tree.png', import.meta.url).href;
const showProviderModal = ref(false);
const browserLan = useBrowserLanStore();

function openAppWindow(): void {
  emit('open-app-window');
}

function openProviderModal(): void {
  showProviderModal.value = true;
}

function closeProviderModal(): void {
  showProviderModal.value = false;
}

async function probeLanHost(): Promise<void> {
  await browserLan.probeAndSave();
}

async function openRemoteLanChat(): Promise<void> {
  let host = browserLan.loadSaved();
  if (!host) {
    host = await browserLan.probeAndSave();
  }
  if (!host) return;
  setRemoteConversationRuntimeOverride(true);
  emit('open-app-window');
}

onMounted(() => {
  browserLan.loadSaved();
  window.addEventListener('ts:browser-llm-config-request', openProviderModal);
});

onBeforeUnmount(() => {
  window.removeEventListener('ts:browser-llm-config-request', openProviderModal);
});
</script>

<style scoped>
.browser-landing {
  position: relative;
  isolation: isolate;
  min-height: 100vh;
  min-height: 100dvh;
  overflow-x: hidden;
  color: var(--ts-text-primary);
  background:
    linear-gradient(135deg, color-mix(in srgb, var(--ts-accent) 12%, transparent), transparent 34%),
    linear-gradient(180deg, color-mix(in srgb, var(--ts-bg-panel) 65%, transparent), transparent 46%),
    var(--ts-bg-gradient);
}

/* ── Futuristic grid lattice overlay (subtle, behind content) ─────── */
.browser-landing::before {
  content: '';
  position: absolute;
  inset: 0;
  z-index: -1;
  pointer-events: none;
  background-image:
    linear-gradient(color-mix(in srgb, var(--ts-accent) 8%, transparent) 1px, transparent 1px),
    linear-gradient(90deg, color-mix(in srgb, var(--ts-accent) 8%, transparent) 1px, transparent 1px);
  background-size: 56px 56px;
  mask-image: radial-gradient(ellipse 80% 60% at 50% 0%, #000 30%, transparent 80%);
  -webkit-mask-image: radial-gradient(ellipse 80% 60% at 50% 0%, #000 30%, transparent 80%);
  opacity: 0.55;
}

/* ── Top nav ──────────────────────────────────────────────────────────── */
.landing-nav {
  position: sticky;
  top: 0;
  z-index: var(--ts-z-sticky);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--ts-space-md);
  padding: var(--ts-space-md) clamp(var(--ts-space-md), 4vw, var(--ts-space-2xl));
  background: color-mix(in srgb, var(--ts-bg-panel) 70%, transparent);
  border-bottom: 1px solid color-mix(in srgb, var(--ts-border) 70%, transparent);
  backdrop-filter: blur(22px) saturate(140%);
  -webkit-backdrop-filter: blur(22px) saturate(140%);
}

.brand-lockup {
  display: inline-flex;
  align-items: center;
  gap: 0.6rem;
  color: inherit;
  text-decoration: none;
}

.brand-icon {
  width: 32px;
  height: 32px;
  border-radius: var(--ts-radius-md);
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--ts-accent) 35%, transparent);
}

.brand-name {
  font-weight: 800;
  letter-spacing: 0;
}

.landing-links {
  display: flex;
  align-items: center;
  gap: clamp(0.75rem, 2.5vw, 1.6rem);
}

/* Right-side action cluster (theme switcher + primary CTA). */
.nav-actions {
  display: flex;
  align-items: center;
  gap: var(--ts-space-sm);
}

.landing-links a,
.secondary-action {
  color: var(--ts-text-secondary);
  text-decoration: none;
  font-weight: 700;
  font-size: 0.92rem;
  transition: color var(--ts-transition-fast, 0.15s ease);
}

.landing-links a:hover,
.landing-links a:focus-visible,
.secondary-action:hover,
.secondary-action:focus-visible {
  color: var(--ts-accent);
}

.nav-cta,
.primary-action,
.secondary-action {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  border: 1px solid transparent;
  border-radius: var(--ts-radius-pill);
  padding: 0.6rem 1rem;
  font-weight: 700;
  cursor: pointer;
  transition: transform var(--ts-transition-fast, 0.15s ease), box-shadow var(--ts-transition-fast, 0.15s ease), background var(--ts-transition-fast, 0.15s ease);
}

.nav-cta,
.primary-action {
  color: var(--ts-text-on-accent);
  background: linear-gradient(135deg, var(--ts-accent), var(--ts-accent-violet, var(--ts-accent)));
  box-shadow: 0 8px 22px color-mix(in srgb, var(--ts-accent) 35%, transparent);
}

.nav-cta:hover,
.primary-action:hover,
.nav-cta:focus-visible,
.primary-action:focus-visible {
  transform: translateY(-1px);
  box-shadow: 0 12px 28px color-mix(in srgb, var(--ts-accent) 45%, transparent);
}

.cta-dot {
  width: 0.45rem;
  height: 0.45rem;
  border-radius: 50%;
  background: #fff;
  box-shadow: 0 0 0 4px color-mix(in srgb, #fff 35%, transparent);
  animation: pulse-dot 1.8s ease-in-out infinite;
}
@keyframes pulse-dot {
  0%, 100% { opacity: 0.5; transform: scale(1); }
  50%      { opacity: 1;   transform: scale(1.25); }
}

.secondary-action {
  background: color-mix(in srgb, var(--ts-bg-panel) 60%, transparent);
  border-color: var(--ts-border);
  padding: 0.6rem 1rem;
}

.secondary-action:hover {
  background: var(--ts-bg-panel);
  border-color: color-mix(in srgb, var(--ts-accent) 45%, var(--ts-border));
}

/* ── Main content layout ──────────────────────────────────────────────── */
.landing-main {
  width: min(1180px, calc(100% - 2 * var(--ts-space-lg)));
  margin: 0 auto;
  padding: clamp(2.5rem, 6vw, 5rem) 0 clamp(3rem, 6vw, 5rem);
  display: flex;
  flex-direction: column;
  gap: clamp(3.5rem, 8vw, 6rem);
}

.section-head {
  max-width: 720px;
  margin-bottom: var(--ts-space-lg);
}

.eyebrow,
.card-kicker {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  margin: 0 0 0.6rem;
  color: var(--ts-accent);
  font-size: 0.74rem;
  font-weight: 800;
  letter-spacing: 0;
  text-transform: uppercase;
}

.eyebrow-pulse {
  width: 0.45rem;
  height: 0.45rem;
  border-radius: 50%;
  background: var(--ts-accent);
  box-shadow: 0 0 12px var(--ts-accent);
  animation: pulse-dot 1.8s ease-in-out infinite;
}

h1, h2, h3, p { margin-top: 0; }

h1 {
  margin-bottom: var(--ts-space-lg);
  font-size: 4.4rem;
  line-height: 1.02;
  letter-spacing: 0;
  background: linear-gradient(135deg, var(--ts-text-primary) 0%, color-mix(in srgb, var(--ts-text-primary) 60%, var(--ts-accent)) 100%);
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
}

h2 {
  font-size: 2.35rem;
  line-height: 1.15;
  letter-spacing: 0;
}

h3 {
  font-size: 1.05rem;
  margin-bottom: 0.35rem;
}

.hero-copy {
  max-width: 580px;
  color: var(--ts-text-secondary);
  font-size: 1.08rem;
  line-height: 1.7;
}

.hero-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--ts-space-md);
  margin-top: var(--ts-space-xl);
}

/* ── Hero (two-column on desktop) ─────────────────────────────────────── */
.hero-section {
  display: grid;
  grid-template-columns: minmax(0, 1.15fr) minmax(0, 1fr);
  gap: clamp(2rem, 5vw, 4rem);
  align-items: center;
}

.hero-meta {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: var(--ts-space-md);
  margin: var(--ts-space-2xl) 0 0;
  padding: 0;
  list-style: none;
}

.hero-meta li {
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
  padding: 0.85rem 1rem;
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-lg);
  background: color-mix(in srgb, var(--ts-bg-panel) 55%, transparent);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
}

.hero-meta strong {
  font-size: 0.95rem;
  color: var(--ts-text-primary);
}

.hero-meta span {
  font-size: 0.78rem;
  color: var(--ts-text-secondary);
}

/* ── Pet stage ─────────────────────────────────────────────────────────── */
.pet-stage {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.85rem;
  position: relative;
}

/* ── Proof strip ──────────────────────────────────────────────────────── */
.proof-strip {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: var(--ts-space-md);
}

.proof-strip article,
.brain-panel {
  display: grid;
  gap: 0.35rem;
  padding: clamp(1rem, 2vw, 1.35rem);
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-lg);
  background: color-mix(in srgb, var(--ts-bg-panel) 76%, transparent);
  box-shadow: var(--ts-shadow-sm);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
}

.proof-strip strong,
.brain-panel h3 {
  color: var(--ts-text-primary);
  font-weight: 900;
}

.proof-strip span,
.brain-panel p {
  margin: 0;
  color: var(--ts-text-secondary);
  line-height: 1.55;
  font-size: 0.93rem;
}

/* ── LAN link ─────────────────────────────────────────────────────────── */
.lan-section {
  display: grid;
  gap: var(--ts-space-lg);
}

.lan-layout {
  display: grid;
  grid-template-columns: minmax(0, 0.9fr) minmax(0, 1.1fr);
  gap: var(--ts-space-md);
  align-items: start;
}

.lan-limit-panel,
.lan-connect-panel {
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-lg);
  background: color-mix(in srgb, var(--ts-bg-panel) 78%, transparent);
  box-shadow: var(--ts-shadow-sm);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
}

.lan-limit-panel {
  display: grid;
  gap: 0.45rem;
  padding: clamp(1rem, 2vw, 1.35rem);
}

.lan-limit-panel strong {
  color: var(--ts-text-primary);
  font-weight: 900;
}

.lan-limit-panel p {
  margin: 0;
  color: var(--ts-text-secondary);
  line-height: 1.55;
}

.lan-connect-panel {
  display: grid;
  gap: var(--ts-space-md);
  padding: clamp(1rem, 2vw, 1.35rem);
}

.lan-fields {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(92px, 0.24fr) auto;
  gap: var(--ts-space-sm);
  align-items: end;
}

.lan-field {
  display: grid;
  gap: 0.35rem;
  color: var(--ts-text-secondary);
  font-size: var(--ts-text-sm);
  font-weight: 700;
}

.lan-field input {
  min-width: 0;
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-sm);
  background: var(--ts-bg-input);
  color: var(--ts-text-primary);
  font: inherit;
  padding: 0.68rem 0.75rem;
}

.lan-secure-toggle {
  min-height: 42px;
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  color: var(--ts-text-secondary);
  font-size: var(--ts-text-sm);
  font-weight: 800;
}

.lan-endpoint,
.lan-status {
  margin: 0;
  border-radius: var(--ts-radius-sm);
  padding: 0.62rem 0.75rem;
  overflow-wrap: anywhere;
  font-size: var(--ts-text-sm);
  line-height: 1.45;
}

.lan-endpoint {
  color: var(--ts-text-secondary);
  background: color-mix(in srgb, var(--ts-bg-input) 75%, transparent);
  font-family: var(--ts-font-mono);
}

.lan-status.success {
  color: var(--ts-success);
  background: var(--ts-success-bg);
}

.lan-status.error {
  color: var(--ts-error);
  background: var(--ts-error-bg);
}

.lan-actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ts-space-sm);
}

.lan-button:disabled {
  cursor: not-allowed;
  opacity: 0.55;
  transform: none;
}

/* ── Feature cards ────────────────────────────────────────────────────── */
.feature-cards {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: var(--ts-space-md);
  padding: 0;
  margin: 0;
  list-style: none;
}

.feature-card {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
  padding: clamp(1.1rem, 2vw, 1.6rem);
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-xl);
  background: color-mix(in srgb, var(--ts-bg-panel) 78%, transparent);
  box-shadow: var(--ts-shadow-md);
  backdrop-filter: blur(18px);
  -webkit-backdrop-filter: blur(18px);
  transition: transform var(--ts-transition-fast, 0.15s ease), border-color var(--ts-transition-fast, 0.15s ease);
}

.feature-card:hover {
  transform: translateY(-3px);
  border-color: color-mix(in srgb, var(--ts-accent) 50%, var(--ts-border));
}

.feature-icon {
  width: fit-content;
  border-radius: var(--ts-radius-pill);
  padding: 0.22rem 0.55rem;
  color: var(--ts-accent);
  background: color-mix(in srgb, var(--ts-accent) 14%, transparent);
  font-size: 0.76rem;
  line-height: 1;
  font-weight: 900;
}

.feature-card p {
  margin: 0;
  color: var(--ts-text-secondary);
  font-size: 0.93rem;
  line-height: 1.55;
}

/* ── Skill tree showcase ──────────────────────────────────────────────── */
.skill-showcase {
  display: grid;
  grid-template-columns: minmax(0, 0.82fr) minmax(0, 1.18fr);
  gap: clamp(1.5rem, 4vw, 3rem);
  align-items: center;
}

.skill-copy p {
  color: var(--ts-text-secondary);
  line-height: 1.65;
}

.quest-points {
  display: grid;
  gap: 0.65rem;
  margin: var(--ts-space-lg) 0 0;
  padding: 0;
  list-style: none;
}

.quest-points li {
  padding: 0.82rem 1rem;
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-lg);
  color: var(--ts-text-secondary);
  background: color-mix(in srgb, var(--ts-bg-panel) 68%, transparent);
}

.quest-points strong {
  color: var(--ts-text-primary);
}

.skill-visual {
  margin: 0;
  padding: clamp(0.6rem, 1.2vw, 0.9rem);
  border: 1px solid color-mix(in srgb, var(--ts-accent) 30%, var(--ts-border));
  border-radius: var(--ts-radius-xl);
  background: color-mix(in srgb, var(--ts-bg-panel) 72%, transparent);
  box-shadow: var(--ts-shadow-md);
}

.skill-visual img {
  display: block;
  width: 100%;
  aspect-ratio: 16 / 10;
  object-fit: cover;
  border-radius: var(--ts-radius-lg);
}

.skill-visual figcaption {
  margin-top: 0.55rem;
  color: var(--ts-text-secondary);
  font-size: 0.8rem;
  line-height: 1.45;
}

/* ── Brain section ────────────────────────────────────────────────────── */
.brain-layout {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: var(--ts-space-md);
}

.brain-panel h3 {
  margin: 0;
}

/* ── Docs panel ───────────────────────────────────────────────────────── */
.docs-panel {
  padding: clamp(1.5rem, 3vw, 2.5rem);
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-xl);
  background: color-mix(in srgb, var(--ts-bg-panel) 78%, transparent);
  box-shadow: var(--ts-shadow-md);
  backdrop-filter: blur(18px);
  -webkit-backdrop-filter: blur(18px);
}

.docs-panel ul {
  display: grid;
  gap: 0.6rem;
  margin: 0;
  padding-left: 1.25rem;
  color: var(--ts-text-secondary);
  line-height: 1.6;
  font-size: 0.95rem;
}

.docs-panel strong {
  color: var(--ts-text-primary);
}

/* ── Provider modal ──────────────────────────────────────────────────── */
.provider-modal-backdrop {
  position: fixed;
  inset: 0;
  z-index: var(--ts-z-modal, 1000);
  display: grid;
  place-items: center;
  padding: var(--ts-space-lg);
  background: color-mix(in srgb, var(--ts-bg-app) 72%, transparent);
  backdrop-filter: blur(18px);
  -webkit-backdrop-filter: blur(18px);
}

.provider-modal {
  position: relative;
  width: min(720px, 100%);
  max-height: calc(100dvh - 2rem);
  overflow: hidden;
  border: 1px solid color-mix(in srgb, var(--ts-accent) 36%, var(--ts-border));
  border-radius: var(--ts-radius-xl);
  background: var(--ts-bg-panel);
  box-shadow: var(--ts-shadow-lg);
}

.provider-modal-close {
  position: absolute;
  top: 0.72rem;
  right: 0.72rem;
  z-index: 1;
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-md);
  padding: 0.42rem 0.7rem;
  color: var(--ts-text-primary);
  background: var(--ts-bg-input);
  font-weight: 800;
  cursor: pointer;
}

/* ── Footer ───────────────────────────────────────────────────────────── */
.landing-footer {
  padding: var(--ts-space-lg) clamp(var(--ts-space-md), 4vw, var(--ts-space-2xl));
  border-top: 1px solid color-mix(in srgb, var(--ts-border) 70%, transparent);
  text-align: center;
  font-size: 0.85rem;
  color: var(--ts-text-secondary);
}

.landing-footer a {
  color: inherit;
  text-decoration: none;
  font-weight: 700;
}

.landing-footer a:hover {
  color: var(--ts-accent);
}

/* ── Responsive ───────────────────────────────────────────────────────── */
@media (max-width: 980px) {
  .hero-section {
    grid-template-columns: 1fr;
  }

  .proof-strip,
  .lan-layout,
  .skill-showcase,
  .brain-layout {
    grid-template-columns: 1fr;
  }

  .pet-stage {
    order: -1;
    align-items: center;
    width: 100%;
  }
}

@media (max-width: 720px) {
  .landing-links {
    display: none;
  }

  .hero-meta {
    grid-template-columns: 1fr;
  }

  h1 {
    font-size: 3.2rem;
  }

  h2 {
    font-size: 1.85rem;
  }
}

@media (max-width: 560px) {
  h1 {
    font-size: 2.65rem;
  }
}

@media (max-width: 520px) {
  .landing-nav {
    flex-wrap: wrap;
    justify-content: center;
    gap: var(--ts-space-sm);
  }

  .nav-cta {
    order: 3;
    width: 100%;
    justify-content: center;
  }

  .hero-actions {
    flex-direction: column;
    align-items: stretch;
  }

  .primary-action,
  .secondary-action {
    justify-content: center;
    text-align: center;
  }

  .lan-fields {
    grid-template-columns: 1fr;
  }
}
</style>
