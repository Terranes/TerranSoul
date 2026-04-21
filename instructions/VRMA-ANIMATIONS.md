# VRMA Animation System

TerranSoul uses **VRMA** (VRM Animation) files for body animations on VRM character models. This system is referenced from and inspired by [flarom/figure](https://github.com/flarom/figure/tree/main), a web-based VRM/VRMA viewer.

## Library Stack

| Package | Version | Purpose |
|---------|---------|---------|
| `three` | ^0.175 | 3D rendering, `AnimationMixer` for playback |
| `@pixiv/three-vrm` | ^3.4 | VRM model loading, humanoid bone access, expressions |
| `@pixiv/three-vrm-animation` | ^3 | VRMA file loading via `VRMAnimationLoaderPlugin`, clip creation via `createVRMAnimationClip` |

## How VRMA Works

VRMA files are glTF containers holding VRM-specific animation data (bone transforms, expression tracks). The loading pipeline:

```
GLTFLoader + VRMAnimationLoaderPlugin
    → gltf.userData.vrmAnimations[0]   (VRMAnimation data object)
    → createVRMAnimationClip(vrmAnimation, vrm)   (THREE.AnimationClip bound to model)
    → mixer.clipAction(clip).play()   (playback via THREE.AnimationMixer)
```

### Key Difference from Figure

Figure (`flarom/figure`) uses a single global `currentMixer` and `vrmaAnimationClip`:
```js
// figure's approach (single global animation)
currentMixer = new THREE.AnimationMixer(vrm.scene);
const clip = createVRMAnimationClip(vrmAnimationData, currentVrm);
currentAction = currentMixer.clipAction(clip);
currentAction.play();
```

TerranSoul wraps this in a `VrmaManager` class that handles clip caching, crossfading, and coordination with the procedural `CharacterAnimator`:
```ts
// TerranSoul's approach (managed, cached, crossfaded)
vrmaManager.setVRM(vrm);               // bind to loaded model
vrmaManager.play('/animations/clapping.vrma', false, 0.4);  // load + crossfade
vrmaManager.update(delta);              // tick in render loop
vrmaManager.stop(0.4);                  // fade out, return to procedural
```

## Type Compatibility Workaround

`@pixiv/three-vrm` and `@pixiv/three-vrm-animation` have slightly different internal `VRMCore` type definitions (private fields differ). This causes TypeScript errors when passing `VRM` to `createVRMAnimationClip`. The workaround:

```ts
const clip = createVRMAnimationClip(
  vrmAnimation,
  vrm as unknown as Parameters<typeof createVRMAnimationClip>[1]
);
```

## Animation Files

All VRMA files live in `public/animations/`. Source origins:

| File | Source | Loop |
|------|--------|------|
| `idle.vrma` | flarom | Yes |
| `walk.vrma` | flarom | Yes |
| `greeting.vrma` | VRoid Project | No |
| `peace-sign.vrma` | VRoid Project | No |
| `spin.vrma` | VRoid Project | No |
| `model-pose.vrma` | VRoid Project | No |
| `squat.vrma` | VRoid Project | No |
| `angry.vrma` | tk256ailab | Yes |
| `sad.vrma` | tk256ailab | Yes |
| `thinking.vrma` | tk256ailab | Yes |
| `surprised.vrma` | tk256ailab | No |
| `relax.vrma` | tk256ailab | Yes |
| `sleepy.vrma` | tk256ailab | Yes |
| `clapping.vrma` | tk256ailab | No |
| `jump.vrma` | tk256ailab | No |

> **Note:** Figure commented out the tk256ailab animations ("kinda bad"), but we include them because they fill important emotion/motion gaps for LLM-driven animation.

## Animation Registry

Animations are registered in `src/renderer/vrma-manager.ts` → `VRMA_ANIMATIONS` array. Each entry has:

```ts
interface VrmaAnimationEntry {
  label: string;          // Display name
  path: string;           // URL path to .vrma file
  loop: boolean;          // Whether to loop
  mood?: CharacterState;  // Auto-play when this mood is set
  motionKey?: string;     // LLM motion key (e.g. 'clapping')
}
```

## Motion Key Aliases

The LLM may use natural words instead of exact motion keys. `getAnimationForMotion()` resolves aliases:

| LLM says | Resolves to | Animation |
|----------|-------------|-----------|
| `clap`, `applause` | `clapping` | clapping.vrma |
| `wave`, `hello`, `hi`, `bye` | `greeting` | greeting.vrma |
| `dance`, `twirl` | `spin` | spin.vrma |
| `mad`, `furious` | `angry` | angry.vrma |
| `cry`, `sigh` | `sad` | sad.vrma |
| `think`, `wonder`, `ponder` | `thinking` | thinking.vrma |
| `shock`, `gasp` | `surprised` | surprised.vrma |
| `chill`, `rest` | `relax` | relax.vrma |
| `sleep`, `yawn` | `sleepy` | sleepy.vrma |
| `hop`, `leap` | `jump` | jump.vrma |
| `victory` | `peace` | peace-sign.vrma |
| `model`, `strike` | `pose` | model-pose.vrma |
| `crouch` | `squat` | squat.vrma |
| `stroll` | `walk` | walk.vrma |

## LLM → Animation Pipeline

### System Prompt
All three system prompts (Rust streaming, browser basic, browser enhanced) instruct the LLM:
```
<anim>{"emotion":"happy","motion":"clap"}</anim>
```

### Parsing
- **Tauri path:** Rust `StreamTagParser` extracts `<anim>` blocks during streaming → emits `llm-animation` Tauri event → `streaming.handleAnimation()` stores `currentMotion` → `ChatView` calls `viewportRef.playMotion(motion)`
- **Browser path:** `parseTags()` in `emotion-parser.ts` extracts emotion/motion from completed text → stored on `Message.motion` → `ChatView.handleSend()` calls `viewportRef.playMotion(lastMsg.motion)`

### Mood vs. Motion Priority
When the LLM emits both emotion and motion, the mood watcher would normally auto-play a mood-mapped animation (e.g. `happy` → `greeting.vrma`). The `VrmaManager.suppressMoodAnimation()` flag prevents this — explicit `playMotion()` calls take priority over mood auto-play. The flag is cleared when `stop()` is called (idle timeout).

## Adding New Animations

1. Place the `.vrma` file in `public/animations/`
2. Add an entry to `VRMA_ANIMATIONS` in `src/renderer/vrma-manager.ts`
3. Add aliases to `MOTION_ALIASES` if needed
4. Update the system prompts in:
   - `src/utils/free-api-client.ts` (both `SYSTEM_PROMPT` and `ENHANCED_SYSTEM_PROMPT`)
   - `src-tauri/src/commands/chat.rs` (`SYSTEM_PROMPT_FOR_STREAMING`)

## Finding VRMA Files

- **VRoid Hub:** Export from VRoid Studio with animations
- **figure repo:** `VRMA/` directory at [flarom/figure](https://github.com/flarom/figure/tree/main/VRMA)
  - `flarom/` — idle, walk (custom)
  - `VRoid Project/` — greeting, peace sign, spin, model pose, squat, shoot, show full body
  - `tk256ailab/` — angry, blush, clapping, goodbye, jump, look around, relax, sad, sleepy, surprised, thinking
- **Mixamo → VRM converter tools** — convert Mixamo FBX animations to VRMA format
