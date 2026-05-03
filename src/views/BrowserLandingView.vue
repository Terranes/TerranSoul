<template>
  <section class="browser-landing">
    <header class="landing-nav">
      <div class="brand-lockup">
        <img
          src="/icon.png"
          alt=""
          class="brand-icon"
        >
        <span class="brand-name">TerranSoul</span>
      </div>
      <nav
        class="landing-links"
        aria-label="Landing page navigation"
      >
        <a href="#about">About</a>
        <a href="#missions">Missions</a>
        <a href="#browser-docs">Docs</a>
      </nav>
      <button
        class="nav-cta"
        @click="$emit('open-app-window')"
      >
        Live test
      </button>
    </header>

    <main class="landing-main">
      <section
        class="hero-section"
        aria-labelledby="landing-title"
      >
        <p class="eyebrow">
          Open-source 3D AI companion
        </p>
        <h1 id="landing-title">
          Build your own contextual AI familiar.
        </h1>
        <p class="hero-copy">
          TerranSoul combines a live VRM character, multi-provider LLM chat,
          persistent memory, voice I/O, cross-device sync, and RPG-style quests
          into one companion that can run as a desktop app, mobile shell, or
          browser preview.
        </p>
        <div class="hero-actions">
          <button
            class="primary-action"
            @click="$emit('open-app-window')"
          >
            Open browser app window
          </button>
          <a
            class="secondary-action"
            href="#browser-docs"
          >
            Read browser architecture
          </a>
        </div>
      </section>

      <section
        id="about"
        class="content-grid"
        aria-labelledby="about-title"
      >
        <article class="info-card wide">
          <p class="card-kicker">
            About
          </p>
          <h2 id="about-title">
            A companion interface for complete-context engineering.
          </h2>
          <p>
            The browser build presents TerranSoul like a normal product site
            while still mounting the real character renderer for live WebGL
            testing. It auto-configures a free cloud brain when no Tauri backend
            is present, so visitors can inspect the experience before installing
            the desktop shell.
          </p>
        </article>
        <article class="info-card">
          <p class="card-kicker">
            Brain
          </p>
          <h3>Multi-provider LLM routing</h3>
          <p>Free, paid, local Ollama, and remote desktop brain transports share one frontend contract.</p>
        </article>
        <article class="info-card">
          <p class="card-kicker">
            Avatar
          </p>
          <h3>Live VRM model</h3>
          <p>The bottom-right pet uses the same Three.js + VRM renderer as the desktop app.</p>
        </article>
      </section>

      <section
        id="missions"
        class="mission-section"
        aria-labelledby="missions-title"
      >
        <p class="eyebrow">
          Missions
        </p>
        <h2 id="missions-title">
          What TerranSoul is built to do
        </h2>
        <div class="mission-list">
          <article>
            <span class="mission-index">01</span>
            <h3>Unify your AI tools</h3>
            <p>Coordinate chat, agents, workflows, memory, and device context from one companion UI.</p>
          </article>
          <article>
            <span class="mission-index">02</span>
            <h3>Make setup playable</h3>
            <p>Unlock brain, voice, avatar, social, and utility abilities through quests and combos.</p>
          </article>
          <article>
            <span class="mission-index">03</span>
            <h3>Keep privacy optional</h3>
            <p>Start free in the browser, upgrade to paid APIs, or move private workloads to local Ollama.</p>
          </article>
        </div>
      </section>

      <section
        id="browser-docs"
        class="docs-panel"
        aria-labelledby="docs-title"
      >
        <p class="card-kicker">
          Docs
        </p>
        <h2 id="docs-title">
          Browser mode architecture
        </h2>
        <ul>
          <li>The landing page is pure Vue and runs without Tauri IPC.</li>
          <li>The pet preview forces transparent pet rendering and stays anchored to the bottom-right.</li>
          <li>Opening another mode creates a compact in-page app window instead of a native Tauri window.</li>
          <li>Stores fall back to in-memory or localStorage behavior when backend commands are unavailable.</li>
        </ul>
      </section>
    </main>

    <aside
      class="pet-preview"
      aria-label="Live TerranSoul model preview"
    >
      <CharacterViewport force-pet />
      <div class="pet-caption">
        Live model test
      </div>
    </aside>
  </section>
</template>

<script setup lang="ts">
import CharacterViewport from '../components/CharacterViewport.vue';

defineEmits<{
  'open-app-window': [];
}>();
</script>

<style scoped>
.browser-landing {
  --landing-pet-width: clamp(150px, 18vw, 210px);
  --landing-pet-height: clamp(210px, 28vh, 300px);

  min-height: 100vh;
  min-height: 100dvh;
  overflow-x: hidden;
  color: var(--ts-text-primary);
  background:
    radial-gradient(circle at 15% 20%, var(--ts-accent-glow), transparent 34%),
    radial-gradient(circle at 82% 8%, color-mix(in srgb, var(--ts-accent) 22%, transparent), transparent 28%),
    var(--ts-bg-gradient);
}

.landing-nav {
  position: sticky;
  top: 0;
  z-index: var(--ts-z-sticky);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--ts-space-md);
  padding: var(--ts-space-md) clamp(var(--ts-space-md), 4vw, var(--ts-space-2xl));
  background: color-mix(in srgb, var(--ts-bg-panel) 78%, transparent);
  border-bottom: 1px solid var(--ts-border);
  backdrop-filter: blur(18px);
  -webkit-backdrop-filter: blur(18px);
}

.brand-lockup,
.landing-links,
.hero-actions {
  display: flex;
  align-items: center;
  gap: var(--ts-space-md);
}

.brand-icon {
  width: 34px;
  height: 34px;
  border-radius: var(--ts-radius-md);
}

.brand-name {
  font-weight: 800;
  letter-spacing: 0.04em;
}

.landing-links a,
.secondary-action {
  color: var(--ts-text-secondary);
  text-decoration: none;
  font-weight: 700;
}

.landing-links a:hover,
.secondary-action:hover {
  color: var(--ts-accent);
}

.nav-cta,
.primary-action,
.secondary-action {
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-pill);
  padding: 0.7rem 1rem;
  font-weight: 800;
}

.nav-cta,
.primary-action {
  color: var(--ts-text-on-accent);
  background: var(--ts-accent);
  border-color: transparent;
  cursor: pointer;
}

.secondary-action {
  background: var(--ts-bg-panel);
}

.landing-main {
  width: min(1120px, calc(100% - 2 * var(--ts-space-lg)));
  margin: 0 auto;
  padding:
    clamp(3rem, 8vw, 7rem)
    0
    calc(var(--landing-pet-height) + clamp(var(--ts-space-2xl), 8vw, 6rem));
}

.hero-section {
  max-width: 760px;
}

.eyebrow,
.card-kicker {
  color: var(--ts-accent);
  font-size: 0.78rem;
  font-weight: 900;
  letter-spacing: 0.16em;
  text-transform: uppercase;
}

h1,
h2,
h3,
p {
  margin-top: 0;
}

h1 {
  margin-bottom: var(--ts-space-lg);
  font-size: clamp(2.7rem, 8vw, 6.7rem);
  line-height: 0.95;
  letter-spacing: -0.07em;
}

.hero-copy {
  max-width: 690px;
  color: var(--ts-text-secondary);
  font-size: clamp(1.04rem, 2vw, 1.26rem);
  line-height: 1.75;
}

.content-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--ts-space-lg);
  margin-top: clamp(4rem, 8vw, 7rem);
}

.wide {
  grid-column: 1 / -1;
}

.info-card,
.docs-panel,
.mission-list article {
  padding: clamp(var(--ts-space-lg), 3vw, var(--ts-space-2xl));
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-xl);
  background: color-mix(in srgb, var(--ts-bg-panel) 82%, transparent);
  box-shadow: var(--ts-shadow-md);
  backdrop-filter: blur(18px);
  -webkit-backdrop-filter: blur(18px);
}

.info-card p,
.mission-list p,
.docs-panel li {
  color: var(--ts-text-secondary);
  line-height: 1.7;
}

.mission-section,
.docs-panel {
  margin-top: clamp(4rem, 8vw, 7rem);
}

.mission-list {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: var(--ts-space-lg);
  margin-top: var(--ts-space-lg);
}

.mission-index {
  display: inline-flex;
  margin-bottom: var(--ts-space-md);
  color: var(--ts-accent);
  font-weight: 900;
}

.docs-panel ul {
  display: grid;
  gap: var(--ts-space-sm);
  padding-left: 1.2rem;
}

.pet-preview {
  position: fixed;
  right: clamp(var(--ts-space-sm), 3vw, var(--ts-space-xl));
  bottom: clamp(var(--ts-space-sm), 3vw, var(--ts-space-xl));
  z-index: var(--ts-z-overlay);
  width: var(--landing-pet-width);
  height: var(--landing-pet-height);
  pointer-events: auto;
}

.pet-caption {
  position: absolute;
  right: 0;
  bottom: 0;
  padding: 0.4rem 0.65rem;
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-pill);
  color: var(--ts-text-secondary);
  background: var(--ts-bg-panel);
  font-size: 0.72rem;
  font-weight: 800;
}

@media (max-width: 820px) {
  .browser-landing {
    --landing-pet-width: 138px;
    --landing-pet-height: 205px;
  }

  .landing-links {
    display: none;
  }

  .content-grid,
  .mission-list {
    grid-template-columns: 1fr;
  }

  .landing-main {
    width: min(100% - 2 * var(--ts-space-md), 1120px);
  }
}

@media (max-width: 520px) {
  .browser-landing {
    --landing-pet-width: 112px;
    --landing-pet-height: 168px;
  }

  .landing-nav,
  .hero-actions {
    align-items: stretch;
  }

  .landing-nav {
    flex-wrap: wrap;
  }

  .hero-actions {
    flex-direction: column;
  }

  .primary-action,
  .secondary-action {
    text-align: center;
  }

  .pet-caption {
    max-width: 9rem;
    text-align: center;
  }
}
</style>
