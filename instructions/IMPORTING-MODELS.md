# Importing 3D Models into TerranSoul

This guide explains how to import VRM models into TerranSoul for use as your AI character avatar.

## Supported Formats

| Format | Version | Status |
|--------|---------|--------|
| VRM | 0.0 | ✅ Supported |
| VRM | 1.0 | ✅ Supported |
| glTF | 2.0 | ❌ Not directly — must be packaged as VRM |
| FBX | — | ❌ Not supported |

## Prerequisites

- A `.vrm` file (see [Where to Find Models](#where-to-find-models))
- TerranSoul running on your desktop

## Built-in Default Models

TerranSoul ships with four bundled VRM models that are available out of the box:

| Model | File | Description |
|-------|------|-------------|
| Annabelle the Sorcerer | `public/models/default/Annabelle the Sorcerer.vrm` | Default character (loaded on startup) |
| M58 | `public/models/default/M58.vrm` | Additional bundled character |
| Miyoura Toshie | `public/models/default/Miyoura Toshie.vrm` | Additional bundled character |
| Nogami Juto | `public/models/default/Nogami Juto.vrm` | Additional bundled character |

You can switch between default models using the **dropdown** in the Model Panel, or by clicking the corresponding model card.

### How Default Models Are Protected

Default VRM models are encrypted at build time to protect the original creators' work.
You don't need to do anything — encryption and decryption are handled automatically.

```
Build time (CI):
  Raw .vrm files (stored in a private source, never in the Git repo)
      ↓ AES-256-GCM encryption
  .vrm.enc files bundled into the installer

Runtime (your machine):
  App requests a default model
      ↓ Rust backend decrypts in memory
  Decrypted bytes passed directly to the 3D renderer
      ↓ Model appears in the viewport
  Plaintext .vrm files are NEVER written to disk
```

- **Default models** are encrypted (`.vrm.enc`) and decrypted in memory by the
  Rust backend. The decryption key is compiled into the app binary at build time —
  you never need to configure or manage keys.
- **Your imported models** (see below) are loaded directly from the path you choose.
  They are your files, so they are **not** encrypted or modified by TerranSoul.

### Managing Default Models

Default models are managed by the TerranSoul team and updated automatically via
app updates. You cannot add, remove, or replace them manually — they are part of
the app bundle. To use your own models, see [Importing a Custom VRM Model](#importing-a-custom-vrm-model).

## Switching Between Default Models

### 1. Open the Model Panel

Click the **ℹ** button in the top-right corner of the 3D viewport (next to the state badge).

### 2. Use the Dropdown or Click a Model Card

Select a model from the **Default Model** dropdown, or click a model card in the list. The selected model will load immediately in the viewport.

## Importing a Custom VRM Model

### 1. Open the Model Panel

Click the **ℹ** button in the top-right corner of the 3D viewport (next to the state badge).

### 2. Click "Import VRM Model"

Click the **Import VRM Model** button in the panel. A file dialog will open.

### 3. Select Your VRM File

Navigate to your `.vrm` file and select it. TerranSoul will:
1. Send the file path to the Rust backend for persistence
2. Load the VRM using Three.js + @pixiv/three-vrm
3. Extract metadata (name, author, license)
4. Display the model in the viewport

### 4. Verify the Import

After loading:
- The character name appears in the top-left of the viewport
- The author name appears below the character name
- The model responds to chat messages with animations:
  - **Thinking** — Head tilts with gentle bobbing
  - **Talking** — Mouth opens/closes using BlendShapes, body sways
  - **Happy** — Bouncing with head tilts
  - **Sad** — Head droops forward

### 5. Switch Back to a Default Model

To switch back to a bundled default model, open the Model Panel and select a model from the **Default Model** dropdown or click the corresponding model card.

## How the Model is Used

### Default models (encrypted)

```
App Startup / Model Switch
        ↓
invoke('load_vrm_secure', { modelId })
        ↓
Rust backend reads .vrm.enc from resource dir
        ↓ AES-256-GCM decryption (in memory)
Raw VRM bytes returned via Tauri IPC
        ↓
Frontend creates Blob URL → GLTFLoader → VRM scene
        ↓
CharacterAnimator → Three.js Render
```

### User-imported models (direct file load)

```
User selects .vrm file via "Import VRM Model"
        ↓
File path sent to Rust backend for persistence
        ↓
Frontend loads VRM directly via GLTFLoader
        ↓
CharacterAnimator → Three.js Render
```

### Chat interaction flow (both model types)

```
Chat Message → Rust Backend → Stub Agent (with sentiment)
                                    ↓
                              Response + Sentiment
                                    ↓
                              Frontend ChatView
                                    ↓
                        Character Store (setState)
                                    ↓
                   CharacterAnimator → VRM or Placeholder
                                    ↓
                          Three.js Scene Render
```

1. You type a message in the chat
2. The character enters the **thinking** state (head bob, spinning)
3. The agent responds with a message and a **sentiment** (happy/sad/neutral)
4. The character transitions to the appropriate state:
   - Happy sentiment → **happy** animation (bounce)
   - Sad sentiment → **sad** animation (droop)
   - Neutral sentiment → **talking** animation (mouth movement)
5. After 3 seconds, the character returns to **idle**

## VRM Model Requirements

For best results, your VRM model should have:

### Required
- **Humanoid bones** — At minimum: hips, spine, head
- **Valid VRM metadata** — Title, author, license

### Recommended for Full Animation
- **BlendShapes / Expressions**:
  - `aa` — Mouth open (used for talking animation)
  - `oh` — Mouth round (used for talking variation)
  - `happy` — Smile expression (used for happy state)
- **Head bone** — Enables head bob and tilt animations
- **Hips bone** — Enables body sway and bounce

### Metadata Fields

VRM 1.0:
```json
{
  "name": "My Character",
  "authors": ["Author Name"],
  "licenseUrl": "https://example.com/license"
}
```

VRM 0.0:
```json
{
  "title": "My Character",
  "author": "Author Name",
  "licenseName": "CC-BY-4.0"
}
```

## Where to Find Models

### Free Models
- [VRoid Hub](https://hub.vroid.com/) — Large community library
- [Booth.pm](https://booth.pm/en/search/VRM) — Search for free VRM models
- [Mixamo](https://www.mixamo.com/) → Convert to VRM using UniVRM

### Create Your Own
- [VRoid Studio](https://vroid.com/en/studio) — Free character creation tool (exports VRM directly)
- [Blender](https://www.blender.org/) + [VRM Add-on for Blender](https://github.com/saturday06/VRM-Addon-for-Blender) — Full 3D modeling pipeline

## Troubleshooting

| Problem | Solution |
|---------|----------|
| Model doesn't load | Check the file is a valid `.vrm` file (not `.glb` or `.fbx`) |
| No mouth animation | Model may not have `aa` or `oh` BlendShapes — animation falls back to body movement |
| Model appears backwards | VRM models face -Z by default; TerranSoul rotates them 180° automatically |
| "Failed to load VRM model" | File may be corrupt or incompatible — try a different VRM model |
| Model too large/small | Camera is positioned for standard VRM scale (1 unit ≈ 1 meter); adjust your model in VRoid Studio |
