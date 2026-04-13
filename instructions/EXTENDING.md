# Extending TerranSoul — Developer Guide

This guide explains how developers can extend TerranSoul with custom models, new character behaviors, and additional features.

## Architecture Overview

```
┌─────────────────────────────────────────────┐
│                 Vue Frontend                 │
│                                             │
│  ChatView.vue                               │
│    ├── CharacterViewport.vue                │
│    │     ├── scene.ts (Three.js renderer)   │
│    │     ├── vrm-loader.ts (VRM loading)    │
│    │     └── character-animator.ts (states)  │
│    ├── ChatMessageList.vue                  │
│    ├── ChatInput.vue                        │
│    └── ModelPanel.vue                       │
│                                             │
│  Stores (Pinia)                             │
│    ├── conversation.ts (messages, IPC)      │
│    ├── character.ts (state, VRM path,       │
│    │                  default model select) │
│    └── config/default-models.ts (registry)  │
├─────────────────────────────────────────────┤
│              Tauri IPC Bridge               │
├─────────────────────────────────────────────┤
│                Rust Backend                  │
│    ├── commands/chat.rs (send_message)      │
│    ├── commands/character.rs (load_vrm)     │
│    ├── agent/stub_agent.rs (AI agent)       │
│    └── orchestrator/ (agent routing)        │
└─────────────────────────────────────────────┘
```

## Extension Points

### 1. Custom Character Animations

The `CharacterAnimator` class in `src/renderer/character-animator.ts` controls all character animations. You can add new states or modify existing ones.

**Current states:** `idle`, `thinking`, `talking`, `happy`, `sad`

**To add a new state:**

1. Add the state name to the `CharacterState` type in `src/types/index.ts`:
   ```
   CharacterState = 'idle' | 'thinking' | 'talking' | 'happy' | 'sad' | 'your-state'
   ```

2. Add a case in both `applyVRMAnimation()` and `applyPlaceholderAnimation()` in `character-animator.ts`

3. Add a matching `.state-badge.your-state` CSS rule in `CharacterViewport.vue`

4. Map a sentiment to your state in `ChatView.vue`'s `sentimentToState()` function

### 2. Custom VRM Loaders

The VRM loading pipeline is in `src/renderer/vrm-loader.ts`:

- `loadVRM(scene, path, onProgress?)` — Load and add a VRM to the scene
- `loadVRMSafe(scene, path, onProgress?)` — Same but returns `null` on error
- `extractVrmMetadata(vrm)` — Extract title/author/license from VRM
- `createPlaceholderCharacter(scene)` — Create the fallback capsule character

**To add support for additional 3D formats:**

1. Create a new loader file (e.g., `src/renderer/glb-loader.ts`)
2. Use Three.js `GLTFLoader` directly (without VRM plugin) for plain glTF/GLB files
3. Wire it into `CharacterViewport.vue` alongside the VRM loader

### 3. Custom AI Agents

The agent system uses a trait-based architecture in Rust:

```rust
// src-tauri/src/agent/mod.rs
#[async_trait]
pub trait AgentProvider: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    async fn respond(&self, message: &str) -> (String, Sentiment);
    async fn health_check(&self) -> bool;
}
```

**To add a new agent:**

1. Create a new file in `src-tauri/src/agent/` (e.g., `my_agent.rs`)
2. Implement the `AgentProvider` trait
3. Register it in the `AgentOrchestrator` (in `src-tauri/src/orchestrator/agent_orchestrator.rs`):
   ```rust
   orchestrator.register(Arc::new(MyAgent::new("my-agent")));
   ```
4. The `Sentiment` enum (`Happy`, `Sad`, `Neutral`) drives character animations — return the appropriate sentiment from your agent

### 4. Custom UI Components

The UI follows Vue 3 Composition API patterns with Pinia stores:

- **Components** are in `src/components/` — each has a colocated `.test.ts` file
- **Views** are in `src/views/` — top-level layouts
- **Stores** are in `src/stores/` — shared state via Pinia
- **Types** are in `src/types/index.ts` — shared TypeScript interfaces

**To add a new panel or overlay:**

1. Create your component in `src/components/`
2. Import it in the appropriate view (e.g., `ChatView.vue`)
3. Use stores for shared state (e.g., `useCharacterStore()`, `useConversationStore()`)

### 5. Custom Three.js Scene Elements

The 3D scene is set up in `src/renderer/scene.ts`:

- WebGPU renderer (with WebGL fallback)
- 3-point lighting (ambient + directional + rim)
- PerspectiveCamera at position (0, 1.4, 3) looking at (0, 1, 0)
- ResizeObserver for responsive canvas

**To add custom scene elements:**

1. Access the `SceneContext` returned by `initScene()` in `CharacterViewport.vue`
2. Add objects to `ctx.scene`
3. For animated objects, update them in the `loop()` function using `ctx.clock.getDelta()`

## Testing

### Frontend Tests (Vitest)
```bash
npm run test        # Run all tests once
npm run test:watch  # Watch mode
```

Tests are colocated with source files (`*.test.ts` alongside `*.vue` or `*.ts`).

### Rust Tests
```bash
cd src-tauri && cargo test --all-targets
```

### Type Checking
```bash
npm run build  # Runs vue-tsc first, then vite build
```

## File Structure for Custom Models

Default models shipped with TerranSoul are stored in the `public/models/default/` directory so Vite serves them as static assets. The current bundled models are:

```
public/
  models/
    default/
      Annabelle the Sorcerer.vrm   # Default character (loaded on startup)
      M58.vrm                      # Additional bundled character
```

The model registry is defined in `src/config/default-models.ts`:

```ts
export const DEFAULT_MODELS: DefaultModel[] = [
  { id: 'annabelle', name: 'Annabelle the Sorcerer', path: '/models/default/Annabelle the Sorcerer.vrm' },
  { id: 'm58', name: 'M58', path: '/models/default/M58.vrm' },
];

export const DEFAULT_MODEL_ID = 'annabelle';
```

**To add a new default model:**

1. Place the `.vrm` file in `public/models/default/`
2. Add an entry to the `DEFAULT_MODELS` array in `src/config/default-models.ts`
3. Optionally change `DEFAULT_MODEL_ID` to set a different startup default

The Model Panel displays a dropdown (`<select>`) and clickable model cards for all registered default models. Users can also still import custom VRM files via the **Import VRM Model** button.

The Rust backend persists the selected VRM path in `AppState.vrm_path`. In a future version, this will be saved to disk so your selection persists across app restarts.

## Sentiment → Animation Mapping

When an agent responds, it returns a `Sentiment` value that drives the character's emotional state:

| Sentiment | Character State | VRM Animation | Placeholder Animation |
|-----------|----------------|---------------|----------------------|
| Happy | `happy` | Bounce, head tilt, happy BlendShape | Bounce, scale increase, rotation |
| Sad | `sad` | Head droop, slow sway | Droop down, forward tilt, scale decrease |
| Neutral | `talking` | Mouth open/close (aa/oh BlendShapes), body sway | Scale pulse, position oscillation |

After 3 seconds, all states return to `idle`.

## Contributing Models

If you create a VRM model for TerranSoul:

1. Ensure it has the recommended BlendShapes (`aa`, `oh`, `happy`)
2. Include proper metadata (title, author, license)
3. Test all 5 animation states by sending chat messages that trigger each sentiment
4. Submit via pull request with the model in the `public/models/default/` directory and register it in `src/config/default-models.ts`
