<template>
  <div class="browser-landing">
    <!-- Decorative background orbs (CSS only — no extra JS load). -->
    <div
      class="aura-layer"
      aria-hidden="true"
    >
      <span class="aura aura-a" />
      <span class="aura aura-b" />
      <span class="aura aura-c" />
    </div>

    <header class="landing-nav">
      <a
        href="#top"
        class="brand-lockup"
        aria-label="TerranSoul — back to top"
      >
        <img
          src="/icon.png"
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
        <a href="#missions">Missions</a>
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
          @click="$emit('open-app-window')"
        >
          <span
            class="cta-dot"
            aria-hidden="true"
          />
          Try the live app
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
            Give your AI a body, a memory, and a soul.
          </h1>
          <p class="hero-copy">
            TerranSoul is a desktop, mobile and browser companion that pairs a
            live VRM character with multi-provider LLM chat, persistent memory,
            voice I/O, cross-device sync, and RPG-style quests — so setting up
            your AI feels like meeting a friend, not configuring a tool.
          </p>
          <div class="hero-actions">
            <button
              type="button"
              class="primary-action"
              @click="$emit('open-app-window')"
            >
              Open the live app
              <span aria-hidden="true">→</span>
            </button>
            <a
              class="secondary-action"
              href="#features"
            >
              See what's inside
            </a>
          </div>
          <ul
            class="hero-meta"
            aria-label="Highlights"
          >
            <li><strong>3 brain modes</strong><span>Free • Paid • Local Ollama</span></li>
            <li><strong>VRM character</strong><span>Live Three.js renderer</span></li>
            <li><strong>Cross-device</strong><span>Desktop · Mobile · Web</span></li>
          </ul>
        </div>
        <aside
          class="pet-stage"
          aria-label="Live TerranSoul pet companion"
        >
          <BrowserPetCompanion />
        </aside>
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
            One companion, every layer of your AI stack.
          </h2>
        </header>
        <ul class="feature-cards">
          <li class="feature-card">
            <span
              class="feature-icon"
              aria-hidden="true"
            >🧠</span>
            <h3>Multi-provider brain</h3>
            <p>
              Free Pollinations cloud, paid OpenAI/Anthropic/Groq keys, or
              fully-local Ollama — one frontend, the same conversation.
            </p>
          </li>
          <li class="feature-card">
            <span
              class="feature-icon"
              aria-hidden="true"
            >🎭</span>
            <h3>Live VRM avatar</h3>
            <p>
              Three.js + @pixiv/three-vrm with mood-driven animations,
              expressions, and procedural pose biases.
            </p>
          </li>
          <li class="feature-card">
            <span
              class="feature-icon"
              aria-hidden="true"
            >📚</span>
            <h3>Persistent memory</h3>
            <p>
              Hybrid 6-signal retrieval, RRF fusion, optional HyDE and LLM
              reranking — your companion actually remembers.
            </p>
          </li>
          <li class="feature-card">
            <span
              class="feature-icon"
              aria-hidden="true"
            >🎙️</span>
            <h3>Voice I/O</h3>
            <p>
              Edge TTS and Web Speech ASR out of the box, with hooks for
              local Whisper and your own TTS pipelines.
            </p>
          </li>
          <li class="feature-card">
            <span
              class="feature-icon"
              aria-hidden="true"
            >🗡️</span>
            <h3>Quest skill tree</h3>
            <p>
              Discover features through 30+ skills across 5 categories with
              combos that unlock when abilities chain together.
            </p>
          </li>
          <li class="feature-card">
            <span
              class="feature-icon"
              aria-hidden="true"
            >🔗</span>
            <h3>Device sync</h3>
            <p>
              CRDT-based pairing over QUIC/WebSocket so chat history, memory,
              and persona stay coherent across machines.
            </p>
          </li>
        </ul>
      </section>

      <section
        id="missions"
        class="mission-section"
        aria-labelledby="missions-title"
      >
        <header class="section-head">
          <p class="card-kicker">
            Missions
          </p>
          <h2 id="missions-title">
            Why we're building TerranSoul
          </h2>
        </header>
        <ol class="mission-list">
          <li>
            <span
              class="mission-index"
              aria-hidden="true"
            >01</span>
            <h3>Unify your AI tools</h3>
            <p>Coordinate chat, agents, workflows, memory, and device context from one companion UI.</p>
          </li>
          <li>
            <span
              class="mission-index"
              aria-hidden="true"
            >02</span>
            <h3>Make setup playable</h3>
            <p>Unlock brain, voice, avatar, social, and utility abilities through quests and combos.</p>
          </li>
          <li>
            <span
              class="mission-index"
              aria-hidden="true"
            >03</span>
            <h3>Keep privacy optional</h3>
            <p>Start free in the browser, upgrade to paid APIs, or move private workloads to local Ollama.</p>
          </li>
        </ol>
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
            Browser mode in 30 seconds
          </h2>
        </header>
        <ul>
          <li>The landing page is pure Vue — it runs without a Tauri backend.</li>
          <li>Click <strong>Try the live app</strong> to open the in-page chat &amp; 3D window.</li>
          <li>The pet preview uses the same Three.js + VRM renderer as the desktop build.</li>
          <li>Stores fall back to in-memory or localStorage when backend commands are missing.</li>
          <li>Install the desktop or mobile shell to unlock voice, sync, and persistent memory.</li>
        </ul>
      </section>
    </main>

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
import { computed } from 'vue';
import BrowserPetCompanion from '../components/BrowserPetCompanion.vue';
import LandingThemeSwitch from '../components/LandingThemeSwitch.vue';

defineEmits<{
  'open-app-window': [];
}>();

const year = computed(() => new Date().getFullYear());
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
    radial-gradient(circle at 12% -10%, var(--ts-accent-glow), transparent 38%),
    radial-gradient(circle at 92% 0%, color-mix(in srgb, var(--ts-accent-violet, var(--ts-accent)) 22%, transparent), transparent 32%),
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

/* ── Decorative aura orbs ─────────────────────────────────────────────── */
.aura-layer {
  position: absolute;
  inset: 0;
  z-index: -1;
  overflow: hidden;
  pointer-events: none;
}
.aura {
  position: absolute;
  display: block;
  width: 38vmax;
  height: 38vmax;
  border-radius: 50%;
  filter: blur(80px);
  opacity: 0.55;
  mix-blend-mode: screen;
}
.aura-a {
  top: -12vmax;
  left: -10vmax;
  background: radial-gradient(circle, var(--ts-accent), transparent 65%);
  animation: drift 22s ease-in-out infinite alternate;
}
.aura-b {
  top: 10vmax;
  right: -14vmax;
  background: radial-gradient(circle, color-mix(in srgb, var(--ts-accent-violet, var(--ts-accent)) 80%, transparent), transparent 65%);
  animation: drift 28s ease-in-out -6s infinite alternate-reverse;
}
.aura-c {
  bottom: -18vmax;
  left: 18vmax;
  background: radial-gradient(circle, color-mix(in srgb, var(--ts-accent-blue, #60a5fa) 80%, transparent), transparent 65%);
  animation: drift 34s ease-in-out -12s infinite alternate;
}
@keyframes drift {
  from { transform: translate3d(-2%, -2%, 0) scale(1); }
  to   { transform: translate3d(4%, 6%, 0) scale(1.08); }
}
@media (prefers-reduced-motion: reduce) {
  .aura { animation: none; }
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
  letter-spacing: 0.04em;
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
  letter-spacing: 0.18em;
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
  font-size: clamp(2.4rem, 6vw, 4.6rem);
  line-height: 1.02;
  letter-spacing: -0.04em;
  background: linear-gradient(135deg, var(--ts-text-primary) 0%, color-mix(in srgb, var(--ts-text-primary) 60%, var(--ts-accent)) 100%);
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
}

h2 {
  font-size: clamp(1.6rem, 3.4vw, 2.4rem);
  line-height: 1.15;
  letter-spacing: -0.025em;
}

h3 {
  font-size: 1.05rem;
  margin-bottom: 0.35rem;
}

.hero-copy {
  max-width: 580px;
  color: var(--ts-text-secondary);
  font-size: clamp(1rem, 1.4vw, 1.18rem);
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
  font-size: 1.5rem;
  line-height: 1;
}

.feature-card p {
  margin: 0;
  color: var(--ts-text-secondary);
  font-size: 0.93rem;
  line-height: 1.55;
}

/* ── Mission list ─────────────────────────────────────────────────────── */
.mission-list {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: var(--ts-space-md);
  padding: 0;
  margin: 0;
  list-style: none;
  counter-reset: mission;
}

.mission-list li {
  position: relative;
  padding: clamp(1.2rem, 2vw, 1.6rem);
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-xl);
  background: color-mix(in srgb, var(--ts-bg-panel) 72%, transparent);
  box-shadow: var(--ts-shadow-md);
  backdrop-filter: blur(18px);
  -webkit-backdrop-filter: blur(18px);
}

.mission-list p {
  margin: 0;
  color: var(--ts-text-secondary);
  line-height: 1.6;
  font-size: 0.95rem;
}

.mission-index {
  display: inline-block;
  margin-bottom: var(--ts-space-sm);
  padding: 0.18rem 0.55rem;
  border-radius: var(--ts-radius-pill);
  background: color-mix(in srgb, var(--ts-accent) 18%, transparent);
  color: var(--ts-accent);
  font-weight: 900;
  font-size: 0.8rem;
  letter-spacing: 0.12em;
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

  .mission-list {
    grid-template-columns: 1fr;
  }

  h1 {
    font-size: clamp(2.1rem, 9vw, 3.2rem);
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
}
</style>
