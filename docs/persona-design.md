# Persona, Self-Learning Animation & Master-Mirror — Advanced Architecture Design

> **TerranSoul v0.1** — How the VRM companion learns a personality, expressions
> and motion *from its master* (the user) via the camera, while keeping the
> camera off by default and on a hard per-session consent leash.
> Last updated: 2026-04-24
> **Audience**: Developers, contributors, and architects working on the
> avatar / persona / animation surface (or any quest that gates it).

---

## Table of Contents

1. [System Overview](#1-system-overview)
2. [The Persona Model — What "Persona" Means in TerranSoul](#2-the-persona-model--what-persona-means-in-terransoul)
3. [The Master-Mirror Self-Learning Loop](#3-the-master-mirror-self-learning-loop)
4. [Camera Pipeline — From Webcam to VRM Bones](#4-camera-pipeline--from-webcam-to-vrm-bones)
5. [Privacy & Consent — The Per-Session Camera Leash](#5-privacy--consent--the-per-session-camera-leash)
6. [Facial Expression Mapping — ARKit Blendshapes ↔ VRM 1.0 Expressions](#6-facial-expression-mapping--arkit-blendshapes--vrm-10-expressions)
7. [Motion Mapping — PoseLandmarker → VRM Humanoid Bones](#7-motion-mapping--poselandmarker--vrm-humanoid-bones)
8. [The Learned Animation Library](#8-the-learned-animation-library)
9. [Persona ↔ Brain Integration](#9-persona--brain-integration)
10. [The Persona Quest Chain — How the User Discovers This](#10-the-persona-quest-chain--how-the-user-discovers-this)
11. [On-Disk Schema & Storage Layout](#11-on-disk-schema--storage-layout)
12. [Tauri Command Surface](#12-tauri-command-surface)
13. [Failure & Degradation Contract](#13-failure--degradation-contract)
14. [April 2026 Research Survey — Modern Persona, Motion & Expression Techniques](#14-april-2026-research-survey--modern-persona-motion--expression-techniques)
15. [Roadmap](#15-roadmap)
16. [Sources](#16-sources)

---

## 1. System Overview

The persona subsystem is the **third leg** of TerranSoul's "soul" stack, alongside
the brain (LLM + memory) and the voice (TTS / ASR). Where the brain decides
*what* the companion thinks and the voice decides *how* it sounds, the persona
subsystem decides *who* it is and *how it moves and feels on screen*.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       TerranSoul Persona Subsystem                          │
│                                                                             │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │                  FRONTEND (Vue 3 + TS, browser)                      │  │
│  │                                                                      │  │
│  │  ┌──────────────────┐   ┌──────────────────┐   ┌─────────────────┐   │  │
│  │  │ PersonaTeacher   │   │ CharacterViewport│   │ ChatView /      │   │  │
│  │  │ (consent + UI)   │──▶│ (Three.js + VRM) │◀──│ ConversationStore│   │  │
│  │  │ • Mirror toggle  │   │                  │   │ • injects persona│   │  │
│  │  │ • Record gesture │   │  AvatarStateMach │   │   prompt block  │   │  │
│  │  │ • "Live" badge   │   │  + CharacterAnim │   └─────────────────┘   │  │
│  │  └────────┬─────────┘   │  + VrmaManager   │            ▲             │  │
│  │           │             └────────▲─────────┘            │             │  │
│  │           │                      │                      │             │  │
│  │  ┌────────▼──────────────────────┴──────────────────────┴──────────┐  │  │
│  │  │                  persona.ts (Pinia store)                       │  │  │
│  │  │   traits • tone • quirks • do/don't • mirror state              │  │  │
│  │  │   custom expression library • learned motion library            │  │  │
│  │  │   ↑ ephemeral session-only camera state                         │  │  │
│  │  └─────────────────────────┬────────────────┬──────────────────────┘  │  │
│  │                            │                │                          │  │
│  │  ┌──────────────────┐   ┌──▼─────────────┐  │  ┌─────────────────────┐ │  │
│  │  │ useCameraCapture │   │ face-mirror.ts │  │  │ persona-prompt.ts   │ │  │
│  │  │ • getUserMedia   │──▶│ MediaPipe Face │──┘  │ pure trait → system │ │  │
│  │  │ • per-session    │   │ Landmarker +   │     │ prompt block        │ │  │
│  │  │   permission     │   │ ARKit-blendshape│    └─────────────────────┘ │  │
│  │  │ • idle timeout   │   │ → VRM expr map │                             │  │
│  │  └──────────────────┘   └────────────────┘                             │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│                            │ Tauri IPC (invoke / emit)                       │
│  ┌─────────────────────────▼────────────────────────────────────────────┐  │
│  │                       BACKEND (Rust + Tokio)                          │  │
│  │                                                                       │  │
│  │  ┌────────────────────────────────────────────────────────────────┐  │  │
│  │  │                 commands/persona.rs                            │  │  │
│  │  │   get_persona, save_persona,                                   │  │  │
│  │  │   list_learned_motions, save_learned_motion,                   │  │  │
│  │  │   delete_learned_motion                                        │  │  │
│  │  └────────┬───────────────────────────────────────────────────────┘  │  │
│  │           │                                                            │  │
│  │  ┌────────▼─────────────────────────────────────────────────────┐    │  │
│  │  │  <app_data_dir>/persona/                                     │    │  │
│  │  │    persona.json          — traits, quirks, do/don't list     │    │  │
│  │  │    motions/<id>.json     — learned motion samples (no video) │    │  │
│  │  │    expressions/<id>.json — learned expression presets        │    │  │
│  │  └──────────────────────────────────────────────────────────────┘    │  │
│  │                                                                       │  │
│  │  ⛔ The Rust backend NEVER receives webcam frames.                   │  │
│  │  ⛔ The Rust backend NEVER stores raw video.                         │  │
│  │  Only post-processed landmark deltas (already de-identified) are     │  │
│  │  ever crossed over the IPC boundary, and only on explicit user      │  │
│  │  "Save" action.                                                      │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```

The contract that makes the rest of this document make sense:

- **Camera frames never leave the browser process.** All face-landmark and
  pose-landmark inference runs in the WebView via MediaPipe Tasks Vision
  (WASM + WebGL backend). The Rust core never sees a single pixel.
- **Camera is off by default.** It is impossible for the camera to be live
  outside an explicit user click within the current chat session.
- **The brain is the only persistent personality store.** Persona traits
  are written into the system-prompt block exactly the way `[LONG-TERM
  MEMORY]` is — the LLM is the persona engine, the persona store just
  decorates the prompt.

---

## 2. The Persona Model — What "Persona" Means in TerranSoul

Per the user's wording: *"User can even teach their model become the persona
they want as well."* Persona is therefore **two things layered**:

| Layer | What it is | How it is produced | Where it lives |
|---|---|---|---|
| **Trait persona** | Name, role, tone, quirks, do/don't list, biography | User-edited in the Persona panel (or learned by the LLM from chat extraction) | `persona.json` on disk; loaded into `persona.ts` Pinia store |
| **Embodiment persona** | Custom expression presets + learned motion clips that the avatar uses to *show* the persona | Recorded from the camera in a teach session | `expressions/`, `motions/` on disk; indexed by `persona.ts` |

`PersonaTraits` schema (TypeScript, mirrored 1:1 in `persona.json`):

```ts
interface PersonaTraits {
  /** Schema version for migration safety. */
  version: number;
  /** Display name the LLM should use for itself. */
  name: string;
  /** One-line role / archetype, e.g. "studious librarian", "playful imp". */
  role: string;
  /** Free-form biography paragraph (max ~500 chars to keep prompt cheap). */
  bio: string;
  /** Tone descriptors: ["warm", "concise", "lightly sarcastic"]. */
  tone: string[];
  /** Quirks / catchphrases the LLM should sprinkle in. */
  quirks: string[];
  /** Hard "don't" list (negative constraints). */
  avoid: string[];
  /** Whether the persona block is currently injected into the system prompt. */
  active: boolean;
  /** Last edit timestamp (ms epoch). */
  updatedAt: number;
}
```

There is **exactly one** active `PersonaTraits` at any time — same constraint
as the active brain (`brain::selection`). Multiple persona profiles can live
on disk and be swapped, but only one is "the persona" for the current
conversation. This mirrors how the brain stack picks one provider per turn
(see `brain-advanced-design.md` §20.1).

### 2.1 Default persona (cold-start)

A fresh install ships with `default-persona.json`:

```json
{
  "version": 1,
  "name": "Soul",
  "role": "TerranSoul companion",
  "bio": "A curious AI companion who learns who you are over time.",
  "tone": ["warm", "concise"],
  "quirks": [],
  "avoid": ["unsolicited medical/legal/financial advice"],
  "active": true
}
```

This is the same fallback that `conversation.ts::createPersonaResponse()`
already keys off of when no brain is configured — we are formalizing what
was previously a hard-coded string into a real, editable, sharable persona.

---

## 3. The Master-Mirror Self-Learning Loop

The user explicitly asked for: *"focus on self learning animations, camera
facial and motion detection so AI can learn from their master."* This is the
canonical loop:

```
                  ┌────────────────────────────────────┐
                  │        Master (the user)           │
                  │     in front of the webcam         │
                  └────────────────┬───────────────────┘
                                   │
                                   │ webcam frames (browser-only)
                                   ▼
   ┌─────────────────────────────────────────────────────────────┐
   │                   PersonaTeacher panel                      │
   │                                                             │
   │   ① Consent dialog (per session, see §5)                    │
   │   ② User clicks "Start Mirror" → camera light goes on       │
   │   ③ MediaPipe FaceLandmarker emits 52 ARKit blendshapes     │
   │      + 478 facial landmarks + headPose @ 30 fps             │
   │   ④ MediaPipe PoseLandmarker emits 33 body keypoints @ 30   │
   │   ⑤ face-mirror.ts maps blendshapes → VRM expression weights│
   │      pose-mirror.ts maps keypoints → VRM bone rotations     │
   │   ⑥ AvatarStateMachine.setEmotion / setLookAt /             │
   │      CharacterAnimator.applyBoneOverrides   (already exist) │
   │   ⑦ VRM mirrors the master in real time on screen           │
   └─────────────────────────────┬───────────────────────────────┘
                                 │
                                 │ user clicks "Save this expression as ___"
                                 │ or "Save this 5-second motion as ___"
                                 ▼
   ┌─────────────────────────────────────────────────────────────┐
   │             Learned Asset (on user-confirmed save)          │
   │                                                             │
   │   • Expression preset → expressions/<id>.json               │
   │     { id, name, weights: {happy: .4, surprised: .2, …},     │
   │       trigger: "smug", learnedAt }                          │
   │                                                             │
   │   • Motion clip      → motions/<id>.json                    │
   │     { id, name, fps: 30, duration_s: 5,                     │
   │       frames: [{ bones: { head:[..], leftUpperArm:[..]…}}], │
   │       trigger: "shrug", learnedAt }                         │
   │                                                             │
   │   These are the "self-learned animations". They are         │
   │   indexed by `persona.ts` and registered as motion keys     │
   │   that the LLM can emit (see §9.2).                         │
   └─────────────────────────────────────────────────────────────┘
                                 │
                                 ▼
   ┌─────────────────────────────────────────────────────────────┐
   │        Live use, after the teach session ends               │
   │                                                             │
   │  • LLM emits   <anim>{"motion":"shrug"}</anim>              │
   │  • emotion-parser.ts extracts motion="shrug"                │
   │  • VrmaManager.getAnimationForMotion("shrug")               │
   │      now returns the *learned* clip from persona.ts library │
   │      instead of the bundled default                         │
   │  • CharacterAnimator plays the master's recorded gesture    │
   │  • The avatar literally shrugs the way you shrug.           │
   └─────────────────────────────────────────────────────────────┘
```

Three properties make this a **self-learning** loop and not just mocap:

1. **The recorded clip becomes a first-class motion key** — once saved, the
   LLM sees `shrug` in the prompted motion vocabulary and can choose it on
   its own initiative in any future turn. That is the closed loop: master
   demonstrates → companion learns → companion deploys.
2. **The persona-aware system prompt advertises the personalised motions.**
   `persona-prompt.ts` lists every learned motion key alongside the bundled
   ones in the system prompt's animation vocabulary block, so the brain
   *prefers* the learned ones for character-defining gestures.
3. **Every learned asset is human-readable JSON.** Users can edit / share /
   curate the persona file by hand and version-control it — the same design
   choice as the brain's SQLite-as-debug-store (see §14 of the brain doc,
   "Why SQLite?").

---

## 4. Camera Pipeline — From Webcam to VRM Bones

```
┌──────────────────────────────────────────────────────────────────────────┐
│                     Camera Pipeline (browser only)                        │
│                                                                           │
│                      ┌────────────────────┐                              │
│   navigator         │  useCameraCapture  │ ← per-session permission      │
│   .mediaDevices ───▶│    composable      │   (see §5)                    │
│   .getUserMedia     └─────────┬──────────┘                               │
│                               │ MediaStream                               │
│                               ▼                                           │
│                      ┌────────────────────┐                              │
│                      │   <video> element  │  hidden, 320×240, 30 fps     │
│                      └─────────┬──────────┘                              │
│                                │ HTMLVideoElement                         │
│              ┌─────────────────┼──────────────────┐                      │
│              ▼                                    ▼                      │
│   ┌─────────────────────┐              ┌─────────────────────┐           │
│   │ FaceLandmarker      │              │ PoseLandmarker      │           │
│   │ (MediaPipe Tasks)   │              │ (MediaPipe Tasks)   │           │
│   │ • 478 landmarks     │              │ • 33 keypoints      │           │
│   │ • 52 blendshapes    │              │ • per-keypoint vis  │           │
│   │ • headPose 4×4      │              │   confidence        │           │
│   │ • runs in WASM/WebGL│              │ • runs in WASM/WebGL│           │
│   └──────────┬──────────┘              └──────────┬──────────┘           │
│              │                                     │                      │
│              ▼                                     ▼                      │
│   ┌─────────────────────┐              ┌─────────────────────┐           │
│   │  face-mirror.ts     │              │  pose-mirror.ts     │           │
│   │  ARKit→VRM expr map │              │  IK retargeter      │           │
│   │  + EMA smoothing    │              │  + EMA smoothing    │           │
│   └──────────┬──────────┘              └──────────┬──────────┘           │
│              │                                     │                      │
│              └────────────────┬────────────────────┘                      │
│                               ▼                                           │
│                    ┌────────────────────┐                                │
│                    │ AvatarStateMachine │ ← already existed              │
│                    │ + CharacterAnim    │   (Chunks 120–124)             │
│                    └─────────┬──────────┘                                │
│                              ▼                                           │
│                       Three.js VRM render                                │
└──────────────────────────────────────────────────────────────────────────┘
```

### 4.1 Why MediaPipe Tasks Vision?

We considered the major options as of April 2026. Decision matrix:

| Library | Faces | Pose | License | Browser native | ARKit blendshapes | Verdict |
|---|---|---|---|---|---|---|
| **`@mediapipe/tasks-vision`** (Google) | ✅ 478 + 52 BS | ✅ 33 kp | Apache-2.0 | ✅ WASM + WebGL | ✅ direct output | **Selected** — official, maintained, 1:1 ARKit blendshapes |
| `@vladmandic/human` | ✅ | ✅ | MIT | ✅ TFJS | ⚠️ derived | Heavier (~7 MB), older landmark model |
| `kalidokit` | ✅ via mediapipe v1 | ✅ | MIT | ✅ | ⚠️ derived | Unmaintained since 2022, depends on legacy mediapipe |
| Tencent **Hunyuan-Motion** (4 / 2026) | — | ✅ text-to-motion + video-to-motion | Server-side / model weights only | ❌ | — | Used as a **research reference** for our roadmap (see §14), not as a runtime dep |
| ByteDance **MimicMotion** | — | ✅ video-to-character | Server-side | ❌ | — | Same — research reference, deferred to roadmap |

`@mediapipe/tasks-vision` wins on three independent axes that matter to
TerranSoul: (a) it ships the 52 ARKit ARFace blendshapes natively, which
map almost 1:1 to the VRM 1.0 expression preset list (see §6); (b) it runs
entirely client-side, which is the only way to honour the privacy contract
in §5; (c) Apache-2.0 with no per-seat licensing strings — same posture as
our `@pixiv/three-vrm` choice (memory: avatar rendering, "VRM is sole
avatar format, Live2D rejected for licensing").

It is **lazy-imported** from `face-mirror.ts` so the WASM payload only
downloads once the user has unlocked the relevant quest *and* clicked
"Start Mirror". The cold-start TerranSoul bundle is unaffected.

### 4.2 Frame budget

| Stage | Target | Hard cap |
|---|---|---|
| Camera frame interval | 33 ms (30 fps) | 50 ms (20 fps) |
| FaceLandmarker inference | 8 ms | 20 ms |
| PoseLandmarker inference (when on) | 12 ms | 30 ms |
| Mapper + state-machine update | <1 ms | 3 ms |
| Three.js VRM render (already on demand) | 8 ms | 16 ms |

If the device cannot meet the **hard cap**, the camera pipeline auto-degrades:
PoseLandmarker drops to 15 fps, then off; FaceLandmarker drops to 20 fps; if
*that* still fails, the mirror panel shows a non-blocking warning and the
user can switch to "snapshot teach" mode (single-shot facial expression
capture, no live mirror). No silent degradation — the live indicator changes
colour so the user can always see what fidelity they are getting.

---

## 5. Privacy & Consent — The Per-Session Camera Leash

> Per the user requirement: *"Don't activate the camera learning until user
> allows its permissions, it can only be allowed per chat or session, cannot
> always on for the privacy."*

This is the **single most important section** of the document. Every
implementation decision below must satisfy it; any change that weakens it
must be reviewed and explicitly approved.

### 5.1 The five consent invariants

1. **Cold off.** A freshly installed TerranSoul never asks for camera
   permission on launch and never opens a `MediaStream`. The browser
   permission prompt only fires after a real user click on the
   "Start Mirror" button inside the PersonaTeacher panel — which itself is
   only visible once the relevant quest is unlocked (see §10).

2. **Per-session only.** The persona store has **no** persistent
   `cameraEnabled` field. Consent state lives in the Pinia store as
   `cameraSession: { active: boolean; startedAt: number; chatId: string }`
   and is **never** serialised to disk, localStorage, or Tauri. Browser
   reload, app restart, or chat switch all return to "cold off".

3. **Hard stop on context change.**
   - Switching chat → `stopMirror()` is called automatically; consent must
     be re-given in the new chat.
   - Tab/window blur for >60 s → `stopMirror()` (idle timeout).
   - Window minimised, app backgrounded, system sleep → `stopMirror()`.
   - User navigates away from the chat tab → `stopMirror()`.
   - The `MediaStreamTrack` is `stop()`ed and the reference dropped, so the
     OS-level camera indicator goes off within ~1 s.

4. **Always-visible "Live" indicator.** Whenever the camera track is open,
   a red dot + "Camera live" label is rendered in the chat header (next to
   the existing brain-status badge) and in the corner of the
   CharacterViewport. Both are click-targets that immediately stop the
   mirror. There is no way for the camera to be live without this badge
   being visible — the badge is bound to the same Pinia ref the pipeline
   reads from.

5. **No frames cross the IPC boundary.** The Rust backend has zero camera
   commands. Webcam frames are processed entirely in the WebView; only
   user-confirmed *post-processed* artifacts (the JSON expression preset
   or the JSON motion-frame array from §3) ever cross IPC, and only via an
   explicit "Save" click. Even those artifacts are landmark numbers, not
   pixels.

### 5.2 The consent flow, end-to-end

```
User opens Chat
        │
        │ (no camera anywhere)
        ▼
User opens PersonaTeacher panel
        │
        │ "Start Mirror" button visible, OFF
        ▼
User clicks "Start Mirror"
        │
        ▼
Per-session ConsentDialog (Vue component) fires:
  ┌──────────────────────────────────────────────────────────┐
  │ 🎥  Mirror this session                                  │
  │                                                          │
  │ TerranSoul will use your camera until you stop the      │
  │ mirror or switch chats. Frames stay on this device —    │
  │ they are processed in your browser and never sent       │
  │ anywhere. Recording a gesture saves only landmark       │
  │ numbers, never pixels.                                  │
  │                                                          │
  │ This permission is for THIS chat only. It will be       │
  │ asked again next time.                                  │
  │                                                          │
  │  [ Cancel ]                  [ Allow this session ]    │
  └──────────────────────────────────────────────────────────┘
        │ Cancel → nothing happens. No camera opened.
        │ Allow → navigator.mediaDevices.getUserMedia({video})
        ▼
Browser native permission dialog (only here, never before)
        │ Block → frontend shows "Camera blocked — see browser settings"
        │ Allow → MediaStream opens
        ▼
"Live" badge turns on, mirror starts, idle-timer arms
```

The ConsentDialog is **not** a settings checkbox. It is a fresh dialog every
session, with the per-session scope language above, mirroring how SSH agent
forwarding asks per connection.

### 5.3 Threat model & what we are NOT defending against

We **are** defending against:

- Background camera activation by an updated build (impossible: the only
  callsite of `getUserMedia` is gated by the ConsentDialog).
- Persistent "always on" preference (impossible: no on-disk consent flag).
- Camera-open without UI feedback (impossible: badge is bound to the same
  reactive ref).
- Webcam frames being uploaded, logged, or stored (impossible: frames never
  leave the WebView; Rust has no camera commands).

We are **not** defending against:

- A user who deliberately re-clicks "Allow" every session — that is the
  whole point.
- An attacker with code-execution on the host. (TerranSoul is not a
  hardened sandbox; the OS / browser is the boundary.)
- The user pointing a phone at their screen. (Out of scope.)

### 5.4 Cross-references

- This contract is enforced in code by `useCameraCapture.ts` (the only
  module allowed to call `getUserMedia`) and audited by tests in
  `useCameraCapture.test.ts` — see §11.
- The architecture rule "Persona Documentation Sync" (`rules/architecture-rules.md`
  rule 12) requires this section to stay in sync with code.
- The Master-Mirror quest (§10) cannot be marked "active" by skill-tree
  auto-detection from anything except the *current-session* `cameraSession`
  ref, never from a persisted preference.

---

## 6. Facial Expression Mapping — ARKit Blendshapes ↔ VRM 1.0 Expressions

MediaPipe FaceLandmarker emits the **52 ARKit-style blendshape weights**
defined by Apple's `ARFaceAnchor.BlendShapeLocation`. VRM 1.0 defines a
preset list of expression names. We map the 52 → 6 VRM presets the
`AvatarStateMachine` already exposes (`happy`, `sad`, `angry`, `relaxed`,
`surprised`, `neutral`) plus the 5 viseme channels (`aa`, `ih`, `ou`,
`ee`, `oh`) and the auxiliary channels (`blink`, `lookAt`).

### 6.1 Canonical mapping table

| VRM target | ARKit source(s) | Aggregation | Notes |
|---|---|---|---|
| `happy` | `mouthSmileLeft`, `mouthSmileRight`, `cheekSquintLeft`, `cheekSquintRight` | mean(smile) × 0.7 + mean(squint) × 0.3 | Smile dominates; cheek squint adds Duchenne quality |
| `sad` | `mouthFrownLeft`, `mouthFrownRight`, `browDownLeft`, `browDownRight`, `browInnerUp` | mean(frown) × 0.6 + browInnerUp × 0.4 | Inner-brow raise is the strongest sadness cue per FACS |
| `angry` | `browDownLeft`, `browDownRight`, `noseSneerLeft`, `noseSneerRight`, `mouthPressLeft`, `mouthPressRight` | mean(browDown) × 0.5 + mean(sneer) × 0.3 + mean(press) × 0.2 | Brow-down + sneer is the AU4+AU9 combo |
| `relaxed` | inverse of arousal — derived as `1 − max(angry, sad, surprised)` clamped | computed | Not a direct ARKit channel; derived from absence of stress markers |
| `surprised` | `eyeWideLeft`, `eyeWideRight`, `jawOpen`, `browInnerUp`, `browOuterUpLeft`, `browOuterUpRight` | mean(eyeWide) × 0.5 + jawOpen × 0.3 + mean(browUp) × 0.2 | AU1+AU2+AU5+AU26 |
| `neutral` | `1 − sum(others)` clamped | computed | Falls out of normalisation |
| viseme `aa` | `jawOpen`, `mouthFunnel` | jawOpen × 0.7 + mouthFunnel × 0.3 | Open-mouth /a/ |
| viseme `ih` | `mouthSmileLeft`, `mouthSmileRight`, `mouthStretchLeft`, `mouthStretchRight` | mean(smile + stretch) | Wide /i/ |
| viseme `ou` | `mouthPucker`, `mouthFunnel` | mean | Rounded /u/ |
| viseme `ee` | `mouthStretchLeft`, `mouthStretchRight` | mean | Stretched /e/ |
| viseme `oh` | `mouthFunnel`, `jawOpen` × 0.5 | mean | Rounded /o/ |
| `blink` | `eyeBlinkLeft`, `eyeBlinkRight` | max | Either eye closing counts |
| `lookAt.x` | `eyeLookOutRight − eyeLookOutLeft` | difference | Normalized to (−1, 1) |
| `lookAt.y` | `eyeLookUp* − eyeLookDown*` | difference | Normalized to (−1, 1) |

### 6.2 Smoothing

Raw MediaPipe weights flicker even on a held expression. We apply a
**single-pole exponential moving average** with the same `damp()` helper
already used by `CharacterAnimator` (frame-rate-independent):

```ts
smoothed[k] = damp(smoothed[k], raw[k], lambda=12, deltaSeconds);
```

`λ = 12` gives a ~80 ms time constant — visually instant, no flicker.
This is identical to the lambda the existing animator uses for state
expression lerp, so the live mirror is *visually indistinguishable* from
the LLM-driven path. That symmetry is the whole point.

### 6.3 Why we do not use the 52 raw blendshapes directly

Two reasons:

1. **VRM 1.0 model authors only ship the 6 preset expressions reliably.**
   Most VRM models ship `Happy`/`Angry`/`Sad`/`Relaxed`/`Surprised` plus
   visemes. They typically do **not** ship the full 52-blendshape rig
   (that requires ARKit-grade authoring time). Mapping 52 → 6 lets *every*
   existing VRM the user owns benefit from the camera mirror.
2. **The brain emits the same 6 channels.** The LLM emission vocabulary
   in `emotion-parser.ts` is exactly these 6 emotions plus motion keys
   (memory: persona/avatar). Routing both the LLM and the camera through
   the same 6-channel `AvatarStateMachine` means the camera and the brain
   can never produce a state the rig cannot show.

A future "Mask of a Thousand Faces — Expanded" (§15 roadmap) will let
users opt in to per-blendshape passthrough on rigs that support it.

---

## 7. Motion Mapping — PoseLandmarker → VRM Humanoid Bones

Body retargeting is fundamentally harder than face retargeting because the
target rig (VRM humanoid) has joint limits and the source (a 33-keypoint
landmark cloud in normalised image coordinates) has neither depth nor
calibration. Phase 1 of the persona system therefore ships **upper-body
mirror only** — head, neck, shoulders, upper arms, lower arms — and treats
the legs and hips as locked to the existing idle pose.

### 7.1 Phase-1 retargeting (shipping)

| VRM bone | Source landmarks (mediapipe pose) | Method |
|---|---|---|
| `head` (yaw, pitch, roll) | nose + left/right eye + ears | Pose-from-3 cross product → Euler |
| `leftUpperArm`, `rightUpperArm` | shoulder, elbow | Direction vector → look-at quaternion → Euler |
| `leftLowerArm`, `rightLowerArm` | elbow, wrist | Same, scoped to elbow's local frame |
| `spine` | midShoulder − midHip projection | Lean angle, clamped ±12° |

The IK math reuses the soft-clamp helpers (`softClampMin`, `softClampMax`)
already in `character-animator.ts` so joint limits do not pop. EMA
smoothing identical to §6.2 is applied per-bone.

### 7.2 Phase-2 retargeting (roadmap, see §15)

Full-body retargeting needs depth resolution we do not have from a single
RGB camera. The roadmap path is to integrate one of:

- **MoMask** (CVPR 2024) — masked-token motion generator, supports
  pose-conditioned generation; we would use it as a *reconstruction*
  network: 33-keypoint cloud → full SMPL-X pose → VRM bones via the
  existing `@pixiv/three-vrm` humanoid mapping.
- **Hunyuan-Motion** (Tencent, March 2026) — text-or-video conditioned
  motion synthesis. Used not for live mirror but for offline "polish my
  recorded motion clip" passes (run on a saved clip in a background
  worker with explicit user trigger).
- **MagicAnimate / MimicMotion** — same posture: offline, user-triggered,
  for cleaning up a recorded teach session into a cinematic clip.

All three are deferred because (a) they require GPU inference budgets we
cannot assume on a desktop pet app, and (b) they are not Apache/MIT
licensed in their reference implementations and would need a proxy
service or local Ollama-style runner. They are first-class items in the
April 2026 research survey (§14).

---

## 8. The Learned Animation Library

A "learned asset" is one of two artifacts:

### 8.1 Learned expression preset

A snapshot of the live mirror at the moment the user clicks "Save this
expression as ___":

```json
{
  "id": "lex_01HX...",
  "kind": "expression",
  "name": "Smug",
  "trigger": "smug",
  "weights": {
    "happy": 0.45, "surprised": 0.05, "sad": 0,
    "angry": 0, "relaxed": 0.30,
    "viseme": { "aa": 0, "ih": 0.10, "ou": 0, "ee": 0.05, "oh": 0 }
  },
  "lookAt": { "x": -0.15, "y": 0.05 },
  "blink": 0,
  "learnedAt": 1714000000000
}
```

It is replayed by setting the `AvatarStateMachine` channels to the saved
weights for a configurable hold time (default 1.5 s) followed by a smooth
return to whatever state the LLM is driving.

### 8.2 Learned motion clip

A short (1–10 s) sampled bone-rotation sequence:

```json
{
  "id": "lmo_01HX...",
  "kind": "motion",
  "name": "Master shrug",
  "trigger": "shrug",
  "fps": 30,
  "duration_s": 2.4,
  "frames": [
    { "t": 0,    "bones": { "head": [0,0,0], "leftUpperArm": [0,0,1.35], "rightUpperArm": [0,0,-1.35] } },
    { "t": 0.033,"bones": { "head": [0.02,0,0], "leftUpperArm": [0.1,0,1.40], "rightUpperArm": [0.1,0,-1.40] } },
    "..."
  ],
  "learnedAt": 1714000000000
}
```

Playback is handled by a new lightweight `LearnedMotionPlayer` in
`src/renderer/learned-motion-player.ts` that drives `CharacterAnimator`
bone overrides on a frame schedule, just like `VrmaManager` drives the
mixer. When done it releases bone control back to the procedural animator.

A future chunk (§15) will **bake** learned motion clips to actual `.vrma`
files so they can be played by the existing VrmaManager and shared with
other users — but the JSON-frame format ships first because it is debuggable,
diffable, and editable by hand.

### 8.3 Storage layout

```
<app_data_dir>/persona/
├── persona.json                     ← single active PersonaTraits
├── expressions/
│   ├── lex_01HX...json              ← one file per learned expression
│   └── ...
└── motions/
    ├── lmo_01HX...json              ← one file per learned motion
    └── ...
```

Same "human-readable JSON files on disk" choice as the brain's debug
schema (`brain-advanced-design.md` §14). The user can hand-edit, version-
control, and share these files independently. Sharing a persona is just
zipping this folder.

---

## 9. Persona ↔ Brain Integration

The persona subsystem is **brain-aware** in three places. All three are
documented in `brain-advanced-design.md` already and we are slotting into
existing extension points, not inventing new ones.

### 9.1 System-prompt injection (read path)

`persona-prompt.ts::buildPersonaBlock(traits, learnedMotions)` returns a
text block that is prepended to the system prompt **before** the
`[LONG-TERM MEMORY]` block (per brain-design §4 RAG injection flow):

```
[PERSONA]
You are Soul, a studious librarian. Tone: warm, concise. Quirks:
sprinkles "indeed" occasionally. Avoid: medical / legal / financial advice.

When you want to act out an emotion or motion, emit
<anim>{"emotion":"happy","motion":"shrug"}</anim>. The available motion
keys are: idle, walk, greeting, peace, spin, pose, squat, angry, sad,
thinking, surprised, relax, sleepy, clapping, jump, shrug, headtilt.
(The last two are learned from the master and play their recorded gesture.)
[/PERSONA]

[LONG-TERM MEMORY]
...
```

The brain doesn't know the difference between a bundled motion and a
learned motion — it just sees a bigger vocabulary, with the same
`<anim>{"motion":"…"}</anim>` schema the existing `emotion-parser.ts`
already recognises.

### 9.2 Motion-key registry (write path)

`vrma-manager.ts::getAnimationForMotion(key)` is extended so it consults
`persona.ts.learnedMotions` first; only if no learned motion matches does
it fall back to the bundled `VRMA_ANIMATIONS` list. The alias table is
unchanged. Net effect: the master's learned `shrug` shadows the bundled
`shrug` (if any), but bundled `idle` still works untouched.

This is the same precedence shape as the brain's "user preference shadows
default" pattern (memory: brain-selection snapshot).

### 9.3 LLM-assisted persona authoring (optional)

When a brain is configured, the user can press **"Suggest a persona from
my chats"** in the Persona panel. This calls `brain::extract_persona`
(new Tauri command added to `commands/persona.rs`) which runs a one-shot
prompt over the last N chat turns plus any `tier=long` memories tagged
`personal:*` and proposes a `PersonaTraits` JSON. The user reviews and
approves before it overwrites `persona.json` — same human-in-the-loop
shape as the brain's `extract_memories_from_session` path
(`brain-advanced-design.md` §11).

This makes persona discovery itself a brain-powered feature, closing the
loop: the brain learns who the user is from chat (memory: auto-learn
cadence), and proposes a persona that the avatar then *embodies* via the
camera-learned expression library.

---

## 10. The Persona Quest Chain — How the User Discovers This

Per the user requirements: *"This should be a quest chain from the current
quest system"* AND *"camera quests should be a side quest, please focus on
the research conduct on April 2026 features. Camera quests & implementation
should come last."*

The persona surface is therefore split into a **main chain** (text- and
brain-driven persona; ships first) and a **side chain** (camera-driven
self-learning; ships last). Both sit in the existing skill-tree (memory:
App tabs — Quests is a top-level tab) under the **avatar** category, with
explicit prerequisite edges into the existing Brain category.

```
                            avatar/foundation
                            ┌──────────────┐
                            │  Avatar      │  (existing)
                            │  ✨ Summon   │
                            └──────┬───────┘
                                   │ requires
                                   ▼
                            avatar/advanced  ── MAIN CHAIN ───────────
                            ┌──────────────┐
                            │ Soul Mirror  │  ← NEW (gateway quest)
                            │ 🪞 Persona   │   Open the Persona panel,
                            │   panel      │   default persona materialises
                            └──┬───────────┘
                               │ requires
                               ▼
                            ┌──────────────────────────────┐
                            │ My Persona  🎭✨             │  ← NEW
                            │ Edit name / role / tone /    │
                            │ quirks. The persona block is │
                            │ injected into every chat.    │
                            │ Requires: free-brain          │
                            └──┬───────────────────────────┘
                               │ requires
                               ▼
                            ┌──────────────────────────────┐
                            │ Master's Echo  🌒            │  ← NEW
                            │ Brain-assisted persona       │
                            │ extraction from chats +      │
                            │ long-term memory (`personal:*`).
                            │ Requires: my-persona, memory │
                            └──────────────────────────────┘

                            ─── SIDE CHAIN (camera, ships LAST) ───
                            ┌──────────────────────────────┐
                            │ Mask of a Thousand Faces 🎭  │  (existing stub
                            │ Custom expression presets    │  → real, post
                            │ recorded from the camera.    │   camera lands)
                            │ Requires: soul-mirror        │
                            └──┬───────────────────────────┘
                               │ requires
                               ▼
                            ┌──────────────────────────────┐
                            │ Mirror Dance  🪩            │  (existing stub
                            │ Webcam motion mirror — the   │  → real, post
                            │ avatar mimics your motion.   │   camera lands)
                            │ Requires: soul-mirror        │
                            └──┬───────────────────────────┘
                               │ requires
                               ▼
                            ┌──────────────────────────────┐
                            │ Living Mirror (combo)        │  (existing combo)
                            │ Mocap + expressions together │
                            └──────────────────────────────┘
```

### 10.1 Why this split

The user explicitly directed: focus on the April-2026 *research-conducted*
persona features first; treat the camera path as a side quest that ships
last. That maps onto two clean facts about the codebase:

- The brain-driven part of persona (traits, prompt injection, LLM-assisted
  authoring, drift detection) reuses **only** existing infrastructure
  (skill-tree, brain, memory, conversation store). No new browser APIs,
  no new heavy deps. It can land in one PR with high confidence.
- The camera part introduces `getUserMedia`, MediaPipe WASM payloads,
  ARKit→VRM expression maths, IK retargeting, and a brand-new
  privacy-critical UI surface (the per-session ConsentDialog of §5).
  That is a meaningfully larger surface area and benefits from landing
  on top of a well-exercised main-chain foundation rather than alongside.

The split also matches the user's strict privacy requirement (§5): the
main chain is **camera-free**, so there is no path by which a user has to
go anywhere near the camera to get a meaningful, persona-driven companion.

### 10.2 Auto-detection rules (mirroring `skill-tree.ts::checkActive`)

| Quest id | Chain | Active when |
|---|---|---|
| `soul-mirror` | main | `personaStore.traitsLoaded === true` (the panel has been opened at least once and the default persona materialised on disk) |
| `my-persona` | main | `personaStore.traits.active && personaStore.traits.name !== 'Soul'` (i.e. user customised the default) AND a brain is configured |
| `master-echo` | main | `personaStore.lastBrainExtractedAt !== null` (the user has at least once asked the brain to propose a persona from their chats) |
| `expressions-pack` | side | `personaStore.learnedExpressions.length > 0`. Real activation, replacing the Chunk-128-era stub. **Camera-dependent — ships last.** |
| `motion-capture` | side | `personaStore.learnedMotions.length > 0`. **Camera-dependent — ships last.** |

**Critical:** none of these read the *current-session* `cameraSession`
ref. Quest activation is based on durable artifacts (saved presets, edited
traits) not on whether the camera happens to be live right now. The live
state is a privacy boundary, not a progress signal.

### 10.3 Combos

The existing `motion-capture × expressions-pack → "Living Mirror"` combo
stays untouched (side chain, ships when both sides ship). We add to the
**main chain**:

- `my-persona × free-brain → "Soul of the Words"` — explains that the
  persona traits are now flowing into every chat turn's system prompt.
- `master-echo × rag-knowledge → "Soul of the Library"` — explains that
  the persona is now bootstrapped from the user's long-term memory:
  the LLM read your past conversations, proposed who the companion
  should be, and you confirmed it. Closes the loop with the brain
  documentation's auto-learn cadence (memory: auto-learn cadence).

---

## 11. On-Disk Schema & Storage Layout

### 11.1 Files

See §8.3. JSON-on-disk under `<app_data_dir>/persona/`. There is **no**
SQLite involvement in the persona subsystem. This is deliberate:

- Personas are tiny (a few KB), low-frequency-write artifacts. SQLite would
  be over-engineered.
- Hand-editability and shareability beat indexed query speed for this data.
- Keeping persona out of `memory.db` means a brain wipe never destroys a
  persona, and a persona export never accidentally exposes long-term memory.

### 11.2 Schema versioning

Every JSON file carries `version: number`. A `migratePersona(raw)` pure
function in `src/utils/persona-migrate.ts` upgrades old payloads to the
current schema, mirroring `skill-tree.ts::migrateTracker`. We do not yet
have a V2 schema; this is forward-compatibility scaffolding identical to
how `QuestTrackerData` is handled today.

---

## 12. Tauri Command Surface

All in `src-tauri/src/commands/persona.rs`. **There are no camera
commands.** This is by design (§5).

| Command | Direction | Payload | Notes |
|---|---|---|---|
| `get_persona` | FE → BE | — | Returns the JSON contents of `persona.json` (or the default-persona stub if absent). |
| `save_persona` | FE → BE | `{ json: string }` | Atomic write (temp file + rename) of `persona.json`. |
| `list_learned_expressions` | FE → BE | — | Returns array of expression JSON objects, newest first. |
| `save_learned_expression` | FE → BE | `{ json: string }` | Validates JSON shape, writes to `expressions/<id>.json`. |
| `delete_learned_expression` | FE → BE | `{ id: string }` | Deletes the file. Idempotent. |
| `list_learned_motions` | FE → BE | — | Returns array of motion JSON objects, newest first. Frame arrays included (a motion clip is rarely >100 KB). |
| `save_learned_motion` | FE → BE | `{ json: string }` | As above for motions. |
| `delete_learned_motion` | FE → BE | `{ id: string }` | As above. |
| `extract_persona_from_brain` | FE → BE | `{ chat_history: ChatTurn[] }` | Optional, brain-aware. Routes through the existing `BrainService` and returns a proposed `PersonaTraits` for the user to review. Only callable when a brain is configured. |

All commands return `Result<T, String>` per the codebase convention
(memory: testing/coding standards). All file writes use atomic rename.
Path traversal is rejected by canonicalising the destination and asserting
it lives under `<app_data_dir>/persona/`.

---

## 13. Failure & Degradation Contract

| Failure | Detection | Behaviour | User-visible |
|---|---|---|---|
| Camera permission denied at OS / browser level | `getUserMedia` throws | Mirror does not start; consent dialog shows "Camera blocked — check your browser/OS camera settings". | Modal toast |
| MediaPipe WASM CDN unreachable | `FaceLandmarker.createFromOptions` throws | Mirror panel shows "Face detector failed to load — check your internet, or try again." Camera is *not* opened (we open the camera *after* the detector is ready). | Inline error in panel |
| Inference too slow for hard cap | rolling avg of frame time > cap for 3 s | Auto-degrade per §4.2; badge changes colour. Never crashes. | Badge colour + tooltip |
| Persona file corrupt on disk | `JSON.parse` throws on `get_persona` | Falls back to default-persona stub; corrupted file moved to `persona.json.bak`. | Toast: "Persona file was corrupt and reset." |
| Learned-motion file corrupt | `JSON.parse` throws | That entry is skipped; rest of library loads. | Console warning only — non-blocking |
| Brain unavailable when "Suggest persona" pressed | `BrainService` returns no provider | Button disabled; tooltip explains a brain must be configured. | Disabled button + tooltip |
| User clicks "Allow this session" but then loses the chat | `chatId` change watcher fires | `stopMirror()` + camera track stopped. New chat → consent dialog re-fires. | Badge goes off; user re-prompted on next start |

There is **no silent fallback that opens the camera without consent**. The
worst case if anything fails is "the persona panel cannot start the mirror
right now" — never "the camera is on but you can't tell".

---

## 14. April 2026 Research Survey — Modern Persona, Motion & Expression Techniques

> **Why this section exists**: the avatar-animation and persona-modelling
> landscape moved as fast as RAG did in 2024–2026. This section is the
> canonical "what are we missing?" map for this subsystem, modelled on
> §19 of `brain-advanced-design.md`.

### 14.1 Status legend

| Symbol | Meaning |
|---|---|
| ✅ | Shipped in the current binary |
| 🟡 | Partial / foundations in place, full feature pending |
| 🔵 | Documented gap with concrete roadmap item (§15) |
| ⚪ | Intentionally rejected or out of scope for desktop companion |

### 14.2 Technique → TerranSoul status map

| # | Technique (year, source) | What it is | TerranSoul status | Where / Roadmap |
|---|---|---|---|---|
| 1 | **MediaPipe Tasks Vision FaceLandmarker** ([Google, 2023; ARKit blendshapes 2024](https://developers.google.com/mediapipe/solutions/vision/face_landmarker)) | 478 facial landmarks + 52 ARKit blendshape coefficients in browser via WASM/WebGL | ✅ | §4, §6 |
| 2 | **MediaPipe Tasks Vision PoseLandmarker** ([Google, 2024](https://developers.google.com/mediapipe/solutions/vision/pose_landmarker)) | 33 body keypoints + per-keypoint visibility | ✅ (upper-body) | §7.1 |
| 3 | **VRM 1.0 expression presets** ([VRM Consortium, 2023](https://vrm.dev/en/vrm1/)) | Standardised 6-expression preset list shipped by every VRM 1.0 model | ✅ | §6 |
| 4 | **Hunyuan-Motion** ([Tencent, March 2026](https://hunyuan.tencent.com/)) | Text-and-video-conditioned motion diffusion — generates SMPL-X / Mixamo-compatible motion sequences from a prompt or a reference video | 🔵 | §15 — used as offline "polish my recorded clip" pass; cannot run live in browser. |
| 5 | **MoMask** ([Guo et al., CVPR 2024](https://ericguo5513.github.io/momask/)) | Masked-token motion generator; high-quality reconstruction from sparse keypoints | 🔵 | §15 — Phase 2 full-body retargeting pathway |
| 6 | **MimicMotion** ([Tencent, 2024](https://github.com/Tencent/MimicMotion)) | Reference-image + pose video → animated character video | 🔵 | §15 — offline "render my recorded motion as a cinematic clip" |
| 7 | **MagicAnimate** ([ByteDance, 2024](https://github.com/magic-research/magic-animate)) | Diffusion-based human image animation from pose sequences | 🔵 | §15 — alternative to MimicMotion; same use case |
| 8 | **MotionGPT** ([Jiang et al., NeurIPS 2023](https://github.com/OpenMotionLab/MotionGPT)) | Treats motion as a language; LLM emits motion tokens | 🔵 | §15 — natural Phase-3 "let the brain *generate* motion, not pick from a library" |
| 9 | **Audio2Face / NVIDIA ACE** (2023–2024) | Audio → blendshape weights for talking heads | 🟡 | We already do simpler band-energy lip sync in `lip-sync.ts`. ACE-quality phoneme-aware mapping is a §15 item. |
| 10 | **VASA-1** ([Microsoft, 2024](https://www.microsoft.com/en-us/research/project/vasa-1/)) | Single image + audio → high-realism talking head | ⚪ | Rejected: targets real-faces deepfakes, ethically incompatible with a personal companion. |
| 11 | **EMOTalk-3D / FaceFormer** (2022–2024) | Audio-conditioned 3D facial animation | 🔵 | §15 — would replace the band-energy viseme path (`lip-sync.ts`) with a phoneme-aware model |
| 12 | **OmniHuman-1** ([ByteDance, Feb 2025](https://omnihuman-lab.github.io/)) | Conditioning-mixed end-to-end human video generation | ⚪ | Same posture as #10. Out of scope for a local desktop pet. |
| 13 | **Persona / character cards** (open-source LLM community, 2023–2026) | Structured prompt block defining LLM character (name, traits, examples) | 🟡 | We ship traits in §2 + prompt block in §9.1; we do not yet ship an example-dialogue field. §15 |
| 14 | **Reflective-prompt persona drift detection** (industry, 2025) | Periodically check that LLM responses still match declared persona; nudge with corrective prompt | 🔵 | §15 — pairs naturally with the auto-learn cadence (memory: auto-learn cadence) |
| 15 | **Sharable persona format** (industry, 2024–2026; e.g. Tavern / Silly cards, V2) | Standard JSON for portable personas | 🔵 | §15 — our `persona/` folder is already JSON; need export/import + a schema spec |
| 16 | **Federated motion learning** (research, 2025) | Multiple users contribute learned motion clips to a shared library without sharing raw video | ⚪ | Tempting but rejected for v1: requires social/identity infrastructure we don't have. |

### 14.3 Implementation already shipped from this survey

**Main chain (this PR — research-conducted persona, no camera):**

- **`PersonaTraits` model + persona-aware system prompt** (§2, §9.1) —
  `src/stores/persona.ts` + pure `src/utils/persona-prompt.ts::buildPersonaBlock()`
  injects an `[PERSONA]` block ahead of the existing `[LONG-TERM MEMORY]`
  block. Same precedence shape as the brain doc's §4 RAG injection flow.
- **Default persona (cold start)** (§2.1) — formalises what was previously
  a hard-coded fallback string in `conversation.ts::createPersonaResponse()`.
- **Quest chain** (§10) — Soul Mirror / My Persona / Master's Echo added
  to `src/stores/skill-tree.ts` as the **main chain**. The existing
  `expressions-pack` and `motion-capture` quests are reclassified as the
  **side chain** (camera-driven, ships last) but stay in the tree as
  visible aspirational quests.
- **JSON-on-disk persistence** (§11) — `commands/persona.rs` ships
  `get_persona`, `save_persona`, `list_*`/`save_*`/`delete_*` for learned
  expressions and motions. **No camera commands** — the surface is built
  ready for the side-chain code drop without the camera itself.
- **Architecture rule "Persona Documentation Sync"** — added as rule 12
  in `rules/architecture-rules.md`, mirroring rule 11 (Brain Documentation
  Sync). Any change touching the persona surface must update both this
  doc and `README.md` in the same PR.

**Side chain (deferred per user requirement, ships last):**

- MediaPipe Tasks Vision FaceLandmarker / PoseLandmarker integration,
  per-session ConsentDialog + `useCameraCapture` composable, ARKit→VRM
  expression mapper, IK pose retargeter, learned-asset recording UI —
  all chunked as Phase 13.B (chunks 145–155, see §15) and **gated behind
  the consent contract of §5**.

### 14.4 Sources

- Apple — *ARKit BlendShapeLocation* (developer.apple.com)
- Google MediaPipe — *Tasks Vision: Face Landmarker, Pose Landmarker* (2023–2024 docs)
- VRM Consortium — *VRM 1.0 Specification* (2023)
- Guo et al. — *MoMask: Generative Masked Modeling of 3D Human Motions* (CVPR 2024)
- Jiang et al. — *MotionGPT: Human Motion as a Foreign Language* (NeurIPS 2023)
- Tencent — *Hunyuan Motion* (2026) and *MimicMotion* (2024)
- ByteDance — *MagicAnimate* (CVPR 2024) and *OmniHuman-1* (Feb 2025)
- Microsoft Research — *VASA-1* (2024)
- NVIDIA — *Audio2Face / Avatar Cloud Engine (ACE)* (2023–2024)
- VRM Consortium — *VRMA Animation Format* (2024)
- Open-source SillyTavern / TavernAI persona-card spec, V2 (2024)

---

## 15. Roadmap

Captured as Phase 13 — Persona & Self-Learning in `rules/milestones.md`.
Each chunk maps to a row in §14.2.

The roadmap is split into a **main chain** (research-conducted, brain-driven
persona; ships first) and a **side chain** (camera-driven self-learning;
ships last per the user's explicit ordering). The split mirrors §10.

### 15.1 Phase 13.A — Main chain (ships first)

| Chunk | Title | Maps to §14.2 row | Phase 1 dep |
|---|---|---|---|
| **140** | Persona MVP — `PersonaTraits` store + `persona-prompt.ts` injection + Persona panel + Soul Mirror quest activation | 13 | none |
| **141** | My Persona quest — full editable traits UI + brain-aware combo | 13 | 140, brain configured |
| **142** | Master's Echo (main-chain version) — `extract_persona_from_brain` LLM-assisted authoring from chat history + `personal:*` long-term memories | 13 | 141 + memory tier |
| **143** | Persona drift detection (auto-correction prompt fired by `auto_learn`) | 14 | 142 |
| **144** | Persona export / import as a `.terransoul-persona` JSON bundle (no camera assets in the main-chain bundle) | 15 | 140 |

These five chunks deliver everything the user asked for in the main-chain
sense: research-conducted (§14), brain-driven, persona-as-quest-chain,
no camera dependency.

### 15.2 Phase 13.B — Side chain (camera-driven, ships LAST)

> Per the user requirement: *"Camera quests & implementation should come
> last."* Every chunk below depends on the consent contract in §5 being
> implemented first; none may land before the main chain.

| Chunk | Title | Maps to §14.2 row | Phase 1 dep |
|---|---|---|---|
| **145** | Per-session camera consent dialog + `useCameraCapture` composable + always-visible "Camera live" badge | — (privacy infra, §5) | 140 |
| **146** | MediaPipe FaceLandmarker face mirror + ARKit→VRM expression mapper (face-mirror.ts) | 1, 3 | 145 |
| **147** | Save / load learned expression presets (JSON-on-disk) — promotes `expressions-pack` from stub to real | — (storage) | 146 |
| **148** | PoseLandmarker upper-body mirror + IK retargeting (pose-mirror.ts) | 2 | 146 |
| **149** | Save / load learned motion clips + `LearnedMotionPlayer` — promotes `motion-capture` from stub to real | — (storage) | 148 |
| **150** | Bake learned motion clips → `.vrma` files for VrmaManager / sharing | — | 149 |
| **151** | Side-chain export — bundle learned expressions + motions into the persona zip | 15 | 147, 149 |
| **152** | Phoneme-aware viseme model (FaceFormer / EMOTalk-class) | 11 | 146 |
| **153** | Hunyuan-Motion / MimicMotion offline polish pass (opt-in, deferred) | 4, 6 | 149 |
| **154** | MoMask reconstruction for full-body retarget from sparse keypoints | 5 | 148 |
| **155** | MotionGPT — let the brain *generate* motion tokens directly | 8 | 149, brain configured |

Chunks 140–144 (main chain) deliver the user-authored / brain-extracted
persona experience first. Chunks 145–155 (side chain, camera) layer the
self-learning embodiment on top, in strict consent-first order.

---

## 16. Sources

See §14.4 for the academic / industry citations behind this design. The
TerranSoul-internal cross-references are:

- `docs/brain-advanced-design.md` — particularly §4 (RAG injection flow,
  the model for how the persona block is injected), §10 (Brain modes,
  the model for how persona swaps mirror brain swaps), §14 (Why SQLite,
  the model for why we chose JSON-on-disk for persona instead),
  §19 (April 2026 research survey, the model for §14 here),
  §20 (Brain component selection routing, the model for "active persona
  is one record at a time"), §21 (Daily-conversation write-back loop,
  the model for the Master-Mirror self-learning loop in §3).
- `rules/architecture-rules.md` — rule 12 "Persona Documentation Sync"
  requires this doc and `README.md` to stay in sync with code changes
  to the persona subsystem.
- `rules/milestones.md` Phase 13 — the chunked implementation plan.
- `src/stores/persona.ts`, `src/stores/skill-tree.ts`,
  `src/composables/useCameraCapture.ts`, `src/renderer/face-mirror.ts`,
  `src/components/PersonaTeacher.vue`, `src-tauri/src/commands/persona.rs`
  — the implementation modules.

## Related Documents

- [Brain & Memory — Advanced Architecture Design](./brain-advanced-design.md)
- [Architecture Rules](../rules/architecture-rules.md)
- [Milestones](../rules/milestones.md)
- [README](../README.md)
