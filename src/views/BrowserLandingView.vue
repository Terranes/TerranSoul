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
            A local-first 3D AI companion built on a harness + context
            engineering stack. Hybrid RAG (vector + knowledge graph +
            temporal memory), a self-running MCP brain that other coding
            agents share, voice, device sync, and a quest-led setup path
            across local, free, and paid LLMs — fully offline-capable.
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
            <li><strong>Hybrid RAG</strong><span>Vector + knowledge graph + temporal memory, RRF, HyDE, reranking</span></li>
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

<style src="./BrowserLandingView.css" scoped />
