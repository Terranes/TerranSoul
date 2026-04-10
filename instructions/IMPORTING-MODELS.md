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

## Step-by-Step Import

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

### 5. Switch Back to Default

To switch back to the built-in placeholder character, open the Model Panel and click the **Default Placeholder** card.

## How the Model is Used

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
