# TerranSoul — Instructions

This folder contains guides for working with 3D models in TerranSoul.

## Contents

| Guide | Description |
|-------|-------------|
| [Importing Models](./IMPORTING-MODELS.md) | How to import VRM models into TerranSoul and use them as your AI character |
| [Extending](./EXTENDING.md) | How developers can add custom models, create new character behaviors, and build extensions |

## Quick Start

1. Launch TerranSoul — **Model 1** loads automatically as the default character
2. Click the **ℹ** button in the top-right of the 3D viewport to open the Model Panel
3. Use the **Default Model** dropdown to switch between bundled models (Model 1, Model 2)
4. Or click **Import VRM Model** to load a custom `.vrm` file from your computer
5. Your character appears in the viewport and reacts to chat messages

## What is VRM?

[VRM](https://vrm.dev/en/) is an open file format for 3D humanoid avatars. It's built on top of glTF 2.0 and adds standardized bone structures, blend shapes (facial expressions), and metadata.

TerranSoul supports:
- **VRM 0.0** — Original VRM specification
- **VRM 1.0** — Current VRM specification with improved features

## Default Character

TerranSoul ships with two bundled VRM models in `public/models/default/`:

| Model | File | Description |
|-------|------|-------------|
| Model 1 | `Model1.vrm` | Default character (loaded on startup) |
| Model 2 | `Model2.vrm` | Additional bundled character |

You can switch between them using the dropdown in the Model Panel. The default model registry is defined in `src/config/default-models.ts`.

If a VRM model fails to load, TerranSoul falls back to a built-in placeholder character (a capsule figure with eyes). This placeholder supports all animation states:

- **Idle** — Gentle sway and rotation
- **Thinking** — Spinning with bobbing motion
- **Talking** — Scale pulse with head movement
- **Happy** — Bouncing with scale increase
- **Sad** — Drooping downward with slight tilt

## Where to Find VRM Models

- [VRoid Hub](https://hub.vroid.com/) — Community VRM models (free and paid)
- [VRoid Studio](https://vroid.com/en/studio) — Free tool to create your own VRM characters
- [Ready Player Me](https://readyplayer.me/) — Avatar creation platform (exports to VRM)
- [Booth.pm](https://booth.pm/) — Japanese marketplace with many VRM models
