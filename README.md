# TerranSoul

> **🚧 This project is just an idea and under construction from 10/04/2026.**
> If you are interested, please discuss via <https://discord.gg/RzXcvsabKD> to become a contributor.

> **💡 Why TerranSoul?**
> Tools like OpenClaw, Claude Cowork, and other AI agents can already perform like J.A.R.V.I.S. — but J.A.R.V.I.S. was never just an AI agent. It connected multiple devices, divided tasks across machines, hosted its own infrastructure, maintained the right RAG pipelines, and had persistent memory. Today's AI is powerful but fragmented: agents don't host infrastructure, don't manage retrieval or memory end-to-end, and can't split work across your PCs. So why not bring everything together under one roof? I'm just kicking this off — if you're interested, come drive it even further with your imagination.

**J.A.R.V.I.S. in Real Life — chat-first · cross-device · open-source**

[![TerranSoul CI](https://github.com/Terranes/TerranSoul/actions/workflows/terransoul-ci.yml/badge.svg)](https://github.com/Terranes/TerranSoul/actions/workflows/terransoul-ci.yml)

---

## Vision

TerranSoul is an open-source **3D virtual assistant + AI package manager** that runs across:

| Platform | Target |
|----------|--------|
| Desktop | Windows · macOS · Linux |
| Mobile | iOS · Android |

TerranSoul includes a **TerranSoul Link** layer that securely connects all your devices so you can:

- 💬 Chat with TerranSoul anywhere
- 🔄 Sync conversations and settings across devices
- 🖥️ Control other devices remotely (send commands to run on your PC from your phone)
- 🤖 Orchestrate multiple AI agents (OpenClaw, Claude Cowork, etc.)

> **Phase 1 requirement:** Start with **text chat + 3D character only** (no voice). Voice can be added later.

---

## Platform Strategy (One Codebase)

Built on **Tauri 2.0** as a unified shell across desktop + mobile:

| Layer | Technology |
|-------|-----------|
| Backend | Rust (shared) |
| UI Shell | WebView (shared) |
| Frontend | Vue 3 + TypeScript (shared) |
| 3D Rendering | Three.js with WebGPU (fallback to WebGL2) |

**Platform notes:**

- **Desktop:** transparent always-on-top overlay window + system tray
- **Mobile:** full-screen app (or compact mode), push notifications later, background sync later

---

## Core Products (What Users See)

### A) Chat + 3D Assistant (Phase 1)

A single screen showing:

- 🎭 3D character viewport (VRM model)
- 💬 Chat message list
- ⌨️ Text input bar
- 🤖 Optional agent selector ("auto" or choose agent)

### B) Settings / Management

- **Agents:** install · update · remove · start · stop
- **Characters:** import VRM · select built-ins
- **Plugins:** enable/disable (later)
- **Link devices:** pair + list devices + permissions + remote control

---

## High-Level Architecture (Per Device)

TerranSoul App (on each device) is a **Tauri 2.0** application:

```
┌─────────────────────────────────────────────────────┐
│  Frontend (WebView)                                 │
│  ├── 3D Character Viewport (Three.js)               │
│  ├── Chatbox UI (Vue)                               │
│  └── Settings UI (Vue)                              │
├─────────────────────────────────────────────────────┤
│  Rust Core Engine                                   │
│  ├── AI Package Manager                             │
│  ├── Agent Orchestrator                             │
│  ├── Conversation Router (text → agent)             │
│  ├── TerranSoul Link (cross-device sync + routing)  │
│  └── Plugin Loader (WASM sandbox; later)            │
├─────────────────────────────────────────────────────┤
│  AI Agents (separate processes or services)         │
│  ├── OpenClaw                                       │
│  ├── Claude Cowork bridge                           │
│  ├── Local LLM runtimes (optional)                  │
│  └── Other community integrations                   │
└─────────────────────────────────────────────────────┘
```

---

## Phase 1 Scope (Chat-First, No Voice)

Phase 1 delivers:

- [x] Text chat UI
- [x] 3D character rendering
- [ ] Basic character reactions driven by chat state (no voice):
  - "thinking" when waiting for response
  - "talking" during streaming text (optional)
  - "happy/sad" based on success/failure
  - "idle" when inactive
- [ ] Agent orchestration (minimal):
  - At least one working agent integration OR a stub "local agent" for early testing
- [ ] Character import:
  - Load VRM files
  - Select a default character
- [ ] TerranSoul Link (minimal):
  - Device pairing
  - Conversation sync across devices (optional in Phase 1; can be Phase 2/3)

---

## 3D Character System

| Property | Choice |
|----------|--------|
| Primary avatar format | **VRM 1.0** |
| Fallback | glTF 2.0 (non-humanoid props / simpler assets) |

**Why VRM:**

- Standard humanoid skeleton
- Facial expressions via BlendShapes
- Spring bone physics (hair/clothes)
- Metadata (author/license) helpful for open-source sharing
- Based on glTF 2.0 — works well with Three.js using `@pixiv/three-vrm`

**3D rendering approach:**

- Three.js renderer: prefer **WebGPU** on modern devices, fallback to **WebGL2**
- Performance rules:
  - Cap pixel ratio (especially on mobile)
  - Keep model polycount moderate for Phase 1
  - Use simple lighting and minimal post-processing initially

---

## Chat System (Text)

**Conversation model:**

```ts
interface Message {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  agent_name?: string;
  timestamp: number;
}
```

**UI supports:**

- Message list
- Typing indicator
- Agent badge per assistant message
- "Auto agent" routing

**Conversation routing:**

- Conversation Router inspects user message and decides which agent to use (auto) or respects user-selected agent

---

## AI Package Manager

**Goal:** Install/manage AI agents as packages across devices.

**Core commands:**

```bash
terransoul install <agent-name>
terransoul update <agent-name>
terransoul remove <agent-name>
terransoul list
terransoul start <agent-name>
terransoul stop <agent-name>
```

---

## Tech Stack

| Component | Technology |
|-----------|-----------|
| App Shell | [Tauri 2.0](https://tauri.app/) |
| Frontend | [Vue 3](https://vuejs.org/) + TypeScript |
| State Management | [Pinia](https://pinia.vuejs.org/) |
| 3D Engine | [Three.js](https://threejs.org/) + [@pixiv/three-vrm](https://github.com/pixiv/three-vrm) |
| Build Tool | [Vite](https://vitejs.dev/) |
| Backend | Rust |
| Package Manager | npm (frontend) · Cargo (backend) |

---

## Getting Started

### Prerequisites

- [Node.js](https://nodejs.org/) ≥ 20
- [Rust](https://www.rust-lang.org/tools/install) (latest stable)
- [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)
- [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (Windows only)
- VBScript feature enabled (Windows only)

> **Quick setup with Copilot:** Open GitHub Copilot Chat and type `/setup-prerequisites` to automatically check and install all prerequisites.

### Development

```bash
# Install frontend dependencies
npm install

# Pull private default VRM model bundle (required for built-in models)
TERRANSOUL_PRIVATE_MODELS_URL=<private-bundle-url> npm run models:pull

# Run in development mode (Tauri + Vite dev server)
npm run tauri dev
```

### Build

```bash
# Pull private default VRM model bundle before build
TERRANSOUL_PRIVATE_MODELS_URL=<private-bundle-url> npm run models:pull

# Build the frontend
npm run build

# Build the Tauri app
npm run tauri build
```

### Private Default Model Storage

Built-in VRM model files are no longer stored in this Git repository.

- Storage location: **private model bundle archive** (recommended: private GitHub Release asset).
- CI source URL secret: `TERRANSOUL_PRIVATE_MODELS_URL`
- Optional CI integrity secret: `TERRANSOUL_PRIVATE_MODELS_SHA256`
- Optional auth token secret for private assets: `TERRANSOUL_PRIVATE_MODELS_TOKEN`
- Retrieval command: `npm run models:pull` (extracts `.vrm` files into `public/models/default/`)

---

## Project Structure

```
TerranSoul/
├── src/                    # Vue 3 frontend
│   ├── components/         # UI components (ChatInput, ChatMessageList, etc.)
│   ├── views/              # Page-level views (ChatView)
│   ├── stores/             # Pinia stores (character, conversation)
│   ├── renderer/           # Three.js rendering (scene, VRM loader, animator)
│   └── types/              # TypeScript type definitions
├── src-tauri/              # Rust backend (Tauri)
│   └── src/
│       ├── agent/          # AI agent management
│       ├── orchestrator/   # Agent orchestration
│       ├── commands/        # Tauri commands
│       ├── lib.rs
│       └── main.rs
├── rules/                  # Architecture & coding standards docs
├── .github/workflows/      # CI/CD pipelines
├── package.json
├── vite.config.ts
└── tsconfig.json
```

---

## Contributing

This project is in its **earliest stages**. We welcome contributors of all skill levels!

1. Join the discussion on [Discord](https://discord.gg/RzXcvsabKD)
2. Fork the repository
3. Create a feature branch (`git checkout -b feature/amazing-feature`)
4. Commit your changes (`git commit -m 'Add amazing feature'`)
5. Push to the branch (`git push origin feature/amazing-feature`)
6. Open a Pull Request

---

## License

This project is open-source. License details coming soon.
