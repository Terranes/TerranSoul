# LLM-Driven 3D Animation — Research & Taxonomy (Chunk 14.16a)

> **Status.** Research-only deliverable for Chunk 14.16a. No code
> changes. Provides the foundation for downstream sub-chunks
> 14.16b–14.16f (LLM-as-animator, motion library, retarget, self-improve
> loop, marketplace).

---

## 1. Problem statement

TerranSoul ships a VRM character that today plays one of:

1. Pre-baked **VRMA clips** (`public/animations/*.vrma`) selected by the
   intent classifier or stream tags.
2. Procedural **idle / breathing / blink** animations driven by
   `IdleAnimator`.
3. **Lip-sync visemes** computed from TTS audio.
4. **Webcam Master-Mirror** (Chunk 14.7) — full-body retarget from
   BlazePose landmarks to VRM bones.

What's missing: **novel** motion. When the brain says "Alice tilts her
head curiously," there's no clip for "tilts her head curiously" and the
fallback is either a static idle or the closest pre-baked emote — both
break presence.

Goal: the brain generates context-aware 3D animation in real time. The
self-improve system discovers, learns, and refines new patterns
autonomously without human re-authoring.

---

## 2. State-of-the-art techniques

A non-exhaustive survey of techniques applicable to a Vue + WebGL
desktop app talking to a local Ollama daemon.

### 2.1 Comparison matrix

| # | Technique | Reference | Output | Inference latency | Model size | Quality | License |
|---|---|---|---|---|---|---|---|
| 1 | **MotionGPT** | Jiang et al. 2024 (ICML) | text → motion-token sequence → SMPL-X joints | ~150 ms / clip on CPU after ONNX export | ~600 MB | ⭐⭐⭐⭐ | Open (BSD-style for code; SMPL-X has its own licence — research-only by default; commercial use requires an SMPL-X commercial licence agreement) |
| 2 | **MotionDiffuse** | Zhang et al. 2022 | text → diffusion → joint angle frames | ~1 s / 60-frame clip on GPU; 5–10 s on CPU | ~400 MB | ⭐⭐⭐⭐⭐ | Apache-2.0 (code) |
| 3 | **MoMask** | Guo et al. 2024 | masked motion prediction from sparse keypoints | ~80 ms / frame on CPU | ~250 MB | ⭐⭐⭐⭐ | MIT (code) |
| 4 | **AI4Animation MANN** | Holden et al. 2020 | mode-adaptive neural network, expert blending | ~5 ms / frame | ~50 MB | ⭐⭐⭐ | MIT |
| 5 | **T2M-GPT** | Zhang et al. 2023 | VQ-VAE motion tokens + GPT-2 decoder | ~200 ms / clip on CPU | ~50 MB (codebook) | ⭐⭐⭐⭐ | MIT |
| 6 | **LLM-as-animator** (novel) | TerranSoul-internal | structured JSON pose tags from chat LLM | shares chat latency (~free) | 0 (reuses chat model) | ⭐⭐⭐ | n/a — pure prompting |
| 7 | **Hunyuan-Motion** | Tencent 2025 | text → 60 fps motion clip | ~3 s / clip on RTX 30+ | ~1.2 GB | ⭐⭐⭐⭐⭐ | Apache-2.0 |
| 8 | **PriorMDM** | Tevet et al. 2024 | motion-prior diffusion conditioned on a sparse-pose prior | ~2 s / clip on CPU | ~300 MB | ⭐⭐⭐⭐ | MIT |

### 2.2 Categorisation

The eight techniques split cleanly into four families:

1. **LLM-as-animator (zero extra model).** Reuse the existing chat LLM
   to emit structured pose data alongside text. Free, zero install
   weight, low quality but zero-friction. **Best fit for the v1
   chunk.**

2. **Token-based generative (small model).** T2M-GPT and the codebook
   half of MotionGPT. ~50 MB extra weight, fast CPU inference, output
   is a discrete token sequence the brain can interleave with chat.

3. **Diffusion-based (large model).** MotionDiffuse, Hunyuan-Motion,
   PriorMDM. Best quality, but only viable on GPU machines and only
   for offline polish (precompute high-quality clips into the
   `LearnedMotion` library — see 14.16d).

4. **Continuous control (recurrent).** AI4Animation MANN. ~5 ms /
   frame, but trained per-character — not directly applicable
   without re-training, so we treat MANN as a research reference,
   not a shipping dependency.

---

## 3. VRM bone-mapping strategy

VRM exposes ~55 bones, but only **11** are needed for expressive
upper-body animation. The same 11 are already used by Master-Mirror
(Chunk 14.7) — keep them as the canonical contract.

| Group | Bone (Three.js / VRM) | Master-Mirror? | LLM-animator? | Token codec? |
|---|---|---|---|---|
| Head | `head` | ✅ | ✅ | ✅ |
| Head | `neck` | ✅ | ✅ | ✅ |
| Spine | `spine` | ✅ | ✅ | ✅ |
| Spine | `chest` | ✅ | ✅ | ✅ |
| Hips | `hips` | ✅ | ✅ | ✅ |
| Arms | `leftUpperArm` | ✅ | ✅ | ✅ |
| Arms | `rightUpperArm` | ✅ | ✅ | ✅ |
| Arms | `leftLowerArm` | ✅ | ✅ | ✅ |
| Arms | `rightLowerArm` | ✅ | ✅ | ✅ |
| Shoulders | `leftShoulder` | ✅ | ✅ | ✅ |
| Shoulders | `rightShoulder` | ✅ | ✅ | ✅ |
| Legs | `leftUpperLeg`, `rightUpperLeg`, `leftLowerLeg`, `rightLowerLeg` | partial | ❌ (v1) | ❌ (v1) |
| Hands | finger bones | ❌ | ❌ | ❌ |
| Face | blendshapes (52) | ❌ | optional `expression` | ❌ |

**Why not legs / hands in v1?** Locomotion (legs) needs IK and root-
motion; hands need finger IK or curl presets. Both are doable in
later sub-chunks (14.16e/f), but they double the prompt token count
and the failure surface. v1 keeps to the upper body.

**Coordinate system.** Right-handed, Y-up, Euler XYZ in radians.
Range: ±0.5 rad per bone (about 28°). Larger values produce
non-anatomical poses and the `PoseAnimator` must clamp.

---

## 4. Latency budget

The chat pipeline already has ~600 ms of speech-to-render latency
(LLM stream + TTS + viseme sync). Animation generation must fit
inside that envelope without adding perceptible lag.

| Sub-chunk | Technique | Per-frame budget | Per-clip budget |
|---|---|---|---|
| 14.16b | LLM-as-animator (in-stream pose tags) | n/a (interpolated) | shares chat tokens (free) |
| 14.16c | Motion library (offline LLM clip generation) | n/a — precomputed | <2 s for 2 s clip is fine |
| 14.16d | Diffusion polish (background job) | n/a | 5–30 s acceptable |
| 14.16e | T2M-GPT token brain capability | <250 ms / clip | <250 ms / clip |
| 14.16f | Self-improve loop | offline, batched | hours |

**Hard floor.** 14.16b must add **zero** measurable latency to the
chat stream — pose tags are interleaved with text and parsed
incrementally by `StreamTagParser`.

---

## 5. Recommended implementation order

| Order | Sub-chunk | Risk | Effort | User-visible benefit |
|---|---|---|---|---|
| 1 | **14.16b LLM-as-animator** | low | M | Immediate — every chat turn can emit a pose |
| 2 | **14.16c Motion library** | low | M | Reusable named clips ("/wave", "/bow") |
| 3 | **14.16d Offline diffusion polish** | medium | L | High-quality motion library backed by Hunyuan / MotionDiffuse |
| 4 | **14.16e T2M-GPT brain capability** | medium | L | Capability gate — opt-in, ships own model |
| 5 | **14.16f Self-improve loop** | high | XL | Closes the loop: brain catalogues + reuses generated motions |

Lock step 1 first. Steps 2–5 each unlock independently and can be
shipped over multiple sessions.

---

## 6. Risks & mitigations

| Risk | Mitigation |
|---|---|
| LLM emits non-anatomical poses (limbs through torso) | `PoseAnimator` clamps each bone to ±0.5 rad and applies critical-damping spring; collisions are handled by clamp not LLM smarts |
| Pose tags collide with existing `<anim>` tag stream | New `<pose>` tag, parser already extensible; covered by `StreamTagParser` tests |
| VRMA clip + LLM pose conflict | VRMA always wins (existing precedence). `PoseAnimator` yields when VRMA lock is active |
| Pose tag floods the chat stream (token waste) | System prompt instructs ≤1 `<pose>` per response; orchestrator enforces with regex truncation |
| GPU-only models (MotionDiffuse, Hunyuan) on CPU-only machines | Capability-gate them — only enabled when `gpu_acceleration` capability is granted (already exists for diffusion image gen) |
| Hard licence constraints on SMPL-X (research-only by default) | We do not ship SMPL-X. MotionGPT integration uses its own (licence-clean) motion-token codec retargeted directly to VRM 11 bones — see chunk 14.15 (already shipped) |

---

## 7. Out of scope (future work)

- **Locomotion / root motion** — needs IK + ground-contact solving;
  separate phase.
- **Finger / facial animation from LLM** — needs a per-finger /
  per-blendshape token vocabulary; separate phase.
- **Multi-character coordination** — TerranSoul has one VRM scene at
  a time; group choreography is not on the roadmap.
- **VR / AR retarget** — desktop-only for now.

---

## 8. References

1. Jiang, B. et al. *MotionGPT: Human Motion as a Foreign Language.*
   ICML 2024.
2. Zhang, M. et al. *MotionDiffuse: Text-Driven Human Motion
   Generation with Diffusion Model.* arXiv:2208.15001 (2022).
3. Guo, C. et al. *MoMask: Generative Masked Modeling of 3D Human
   Motions.* CVPR 2024.
4. Holden, D. et al. *Mode-Adaptive Neural Networks for Quadruped
   Motion Control.* SIGGRAPH 2018; carried into 2020 AI4Animation.
5. Zhang, J. et al. *Generating Human Motion from Textual Descriptions
   with Discrete Representations* (T2M-GPT). CVPR 2023.
6. Tevet, G. et al. *PriorMDM: Human Motion Diffusion as a Generative
   Prior.* ICLR 2024.
7. Tencent. *Hunyuan-Motion 1.0 Technical Report.* 2025.
8. `docs/persona-design.md` § 14.7 — Master-Mirror reference for the
   11-bone VRM contract used here.
9. `docs/research-reverse-engineering.md` § 7 — earlier MANN analysis.

---

## 9. Sign-off

This document is the deliverable for **Chunk 14.16a**. It does not
mandate a final implementation choice — that's reserved for each
follow-up sub-chunk's design notes. What it does establish:

- The 11-bone upper-body contract is the canonical surface.
- LLM-as-animator (14.16b) is the v1 path because it is
  zero-install, zero-latency, and reuses the existing brain.
- Diffusion / token-based techniques are deferred to optional
  capability-gated chunks; they will not block 14.16b.
- All techniques retarget to the same 11-bone VRM rig — no separate
  skeleton format proliferation.
