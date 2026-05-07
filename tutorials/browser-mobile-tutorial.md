# Browser & Mobile — Web Deployment, Phone Pairing & Remote Control

> TerranSoul runs as a desktop Tauri app by default, but also supports
> browser deployment (Vercel/static hosting), mobile pairing via QR code,
> and remote control from a phone over gRPC-Web. This tutorial covers all
> three modes.

---

## Requirements

| Requirement | Notes |
|---|---|
| **Desktop app** | Required as the host for phone pairing and LAN bridge |
| **Mobile device** (optional) | For phone pairing — iOS or Android |
| **Modern browser** (optional) | For browser/Vercel mode — Chrome, Edge, Firefox, Safari |

---

## Part 1 — Browser Mode (Static Web Deployment)

TerranSoul's Vue frontend can run standalone in a browser without Tauri:

### Step 1: Build for Web

```bash
npm run build
```

This produces a static bundle in `dist/` that can be deployed to any hosting provider.

### Step 2: Deploy

- **Vercel:** Push to Git → auto-deploys
- **Netlify:** Drag `dist/` folder to dashboard
- **Self-host:** Serve `dist/` with any static file server (nginx, caddy, etc.)

### Step 3: Browser Limitations

In browser mode (without Tauri), some features are reduced:

| Feature | Desktop (Tauri) | Browser (Web) |
|---------|----------------|---------------|
| VRM 3D character | ✅ Full | ✅ Full (Three.js) |
| Chat with brain | ✅ All providers | ✅ Free/Paid cloud only |
| Local Ollama | ✅ Direct | ⚠️ Requires LAN bridge |
| File system access | ✅ Native | ❌ Browser sandbox |
| Voice (ASR/TTS) | ✅ All providers | ✅ Web Speech API only |
| Pet mode | ✅ Transparent overlay | ❌ Not possible |
| Memory persistence | ✅ SQLite | ⚠️ IndexedDB fallback |
| Device pairing | ✅ QUIC/WS | ⚠️ WebSocket only |
| MCP server | ✅ Full | ❌ Server-side only |

### Step 4: LAN Bridge (Browser → Desktop)

Browser builds can connect to a running desktop instance for full capabilities:

1. Run TerranSoul desktop with LAN enabled (`lan_enabled: true`).
2. Open the browser app.
3. Enter the desktop's local IP in **Settings → Network → "Connect to Desktop"**.
4. The browser app uses gRPC-Web to proxy all brain/memory operations through the desktop.

**Connection flow:**
1. Browser probes the desktop endpoint
2. Checks for mixed-content security (HTTPS page → HTTP LAN is blocked by browsers)
3. Creates a `RemoteHost` via gRPC-Web
4. Verifies with `brainHealth()` + `getSystemStatus()`
5. Status changes to `connected`

> **Security note:** The LAN bridge only works on the same network. If the browser page is served over HTTPS, the desktop must also use HTTPS (or use `localhost`).

---

## Part 2 — Mobile Pairing (Phone Link)

### Step 1: Enable on Desktop

1. Open **Settings → Network** on your desktop TerranSoul.
2. Enable **"LAN Sharing"**.
3. The app binds to your local network interfaces.

### Step 2: Open Phone Pairing View

On the desktop, navigate to **Settings → Devices → "Pair Phone"** (or the Mobile Pairing view in the sidebar).

You'll see:
- A QR code for scanning
- Your local IP addresses
- The pairing URI

### Step 3: Scan QR on Phone

On your mobile device:

1. Open TerranSoul mobile (or the browser version on your phone).
2. Go to **Settings → Pair Device → "Scan QR Code"**.
3. Point the camera at the desktop QR code.
4. The URI contains: host address, port, one-time token, device public key.

### Step 4: Confirm Pairing

1. The desktop shows a confirmation dialog with the phone's device name.
2. Accept the pairing.
3. A mutual TLS client certificate bundle is exchanged.
4. Both devices show "Paired ✓".

### Step 5: Manage Paired Devices

In the Mobile Pairing view:
- **List** all paired devices with their last-seen time
- **Revoke** a device to unpair (removes its certificate)
- **Vault password** — set a password to encrypt the pairing credentials

---

## Part 3 — Remote Control (gRPC/gRPC-Web)

Once paired, the phone can control the desktop companion:

### Available Phone Tools

| Feature | Description |
|---------|-------------|
| **Chat** | Send messages to the companion from your phone |
| **Memory search** | Query the brain from mobile |
| **Conversation continue** | Pick up desktop conversations on phone |
| **Workflow triggers** | Start/stop coding workflows remotely |
| **Status monitoring** | See brain health, active agents, running jobs |

### How It Works

1. Desktop runs a gRPC server (part of the RemoteHost system).
2. Phone connects via gRPC-Web (HTTP/2 with protobuf).
3. All operations are authenticated with the paired certificate.
4. Real-time events stream from desktop → phone (agent status, chat messages).

### Copilot Narration (Phone → Desktop)

The phone can narrate context to the desktop's coding agent:

1. Open the phone's chat interface.
2. Type or speak instructions.
3. The message routes to the desktop's active coding workflow.
4. The coding agent receives it as additional context.

---

## Part 4 — Provider Onboarding (First Use in Browser)

When opening TerranSoul in a browser for the first time (no Tauri backend):

1. The First Launch Wizard appears (same as desktop).
2. **Provider options are limited to cloud:**
   - Free tier (Pollinations, OpenRouter free models)
   - Paid API (OpenAI, Anthropic, Groq)
3. Local Ollama requires the LAN bridge to a desktop.
4. The wizard configures the available provider and you're chatting immediately.

---

## Part 5 — Settings Sync Between Devices

Once devices are paired:

| Setting | Sync Behavior |
|---------|--------------|
| Brain provider config | Last-writer-wins |
| Voice preferences | Per-device (phone may have different voice) |
| Persona traits | Merged (union) |
| Theme | Per-device |
| Memories | CRDT sync (see Device Sync tutorial) |
| Conversations | Full sync after reconnect |

---

## Troubleshooting

| Problem | Solution |
|---------|----------|
| Browser can't connect to desktop | Check: same network, LAN enabled on desktop, no mixed-content blocking (try HTTP). |
| QR code won't scan | Ensure camera permissions. Try zooming in. Manual entry of the pairing URI works too. |
| Phone shows "disconnected" | Desktop must be running with LAN enabled. Check firewall allows the port. |
| gRPC-Web timeout | Desktop may be asleep or the network changed. Re-open the connection. |
| Browser mode — no local models | Expected. Use cloud providers, or set up the LAN bridge to a desktop with Ollama. |
| HTTPS mixed-content error | Browser blocks HTTP LAN calls from HTTPS pages. Use `localhost` or deploy a local HTTPS cert. |

---

## Where to Go Next

- **[Quick Start](quick-start-tutorial.md)** — Desktop-first setup if you haven't started there
- **[Device Sync & Hive](device-sync-hive-tutorial.md)** — Full memory sync and privacy controls
- **[Voice Setup](voice-setup-tutorial.md)** — Voice works on mobile too (Web Speech API)
