# Stopping Docker Desktop After Testing

TerranSoul's local LLM setup (`auto_setup_local_llm`) launches Docker Desktop
automatically. Docker Desktop stays running in the background and consumes
**1–2 GB of RAM** even when idle. Use these steps to shut it down when you're
done testing.

---

## Option 1 — From the TerranSoul App (Recommended)

Call the Tauri command from the frontend:

```ts
import { invoke } from '@tauri-apps/api/core';
await invoke('stop_docker_desktop');
```

This sends a graceful shutdown signal to Docker Desktop on all platforms.

---

## Option 2 — Manual (by OS)

### Windows

```powershell
# From PowerShell / Terminal
taskkill /IM "Docker Desktop.exe" /F
```

Or right-click the Docker whale icon in the system tray → **Quit Docker Desktop**.

### macOS

```bash
osascript -e 'quit app "Docker"'
```

Or click the Docker whale icon in the menu bar → **Quit Docker Desktop**.

### Linux

```bash
sudo systemctl stop docker
```

---

## Verifying Docker Is Stopped

```bash
docker info
```

If Docker is stopped you'll see: `Cannot connect to the Docker daemon`.

---

## Re-starting When Needed

TerranSoul will automatically restart Docker Desktop the next time you use
`auto_setup_local_llm` or manually call `start_docker_desktop`. No manual
action needed.
