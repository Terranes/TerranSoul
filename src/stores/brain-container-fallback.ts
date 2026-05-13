/**
 * Container runtime fallback logic for auto-configuring local LLM.
 *
 * Extracted from brain.ts to keep that file under the ESLint max-lines limit.
 * Handles Docker Desktop / Podman detection, auto-install, and Ollama setup.
 */
import { invoke } from '@tauri-apps/api/core';

export interface RuntimeDetection {
  docker_cli: boolean;
  docker_daemon: boolean;
  docker_desktop_installed: boolean;
  podman_cli: boolean;
  podman_working: boolean;
  auto_pick: string | null;
  both_available: boolean;
}

interface ContainerFallbackDeps {
  report: (msg: string) => void;
  modelTag: string;
  checkOllamaStatus: () => Promise<void>;
  fetchInstalledModels: () => Promise<void>;
  isOllamaRunning: () => boolean;
}

/**
 * Attempt to run Ollama via Docker or Podman.
 * Returns `true` if Ollama is running via a container after this call.
 */
export async function tryContainerFallback(deps: ContainerFallbackDeps): Promise<boolean> {
  const { report, modelTag, checkOllamaStatus, fetchInstalledModels, isOllamaRunning } = deps;

  report('Native Ollama unavailable — checking container runtimes...');

  let runtimes = await invoke<RuntimeDetection>('detect_container_runtimes');

  // No runtime at all — install one
  if (!runtimes.docker_cli && !runtimes.docker_desktop_installed && !runtimes.podman_cli) {
    report('No container runtime found — installing Docker Desktop...');
    try {
      await invoke<string>('install_docker_desktop');
      report('Docker Desktop installed');
      runtimes = await invoke<RuntimeDetection>('detect_container_runtimes');
    } catch (dockerInstErr) {
      report(`Docker Desktop install failed: ${dockerInstErr} — trying Podman...`);
      try {
        await invoke<string>('install_podman');
        report('Podman installed');
        runtimes = await invoke<RuntimeDetection>('detect_container_runtimes');
      } catch (podmanInstErr) {
        report(`Podman install also failed: ${podmanInstErr}`);
      }
    }
  }

  // Now try to use whichever runtime is available (prefer Docker)
  if (runtimes.docker_cli || runtimes.docker_desktop_installed) {
    if (!runtimes.docker_daemon && runtimes.docker_desktop_installed) {
      report('Starting Docker Desktop...');
      await invoke<string>('start_docker_desktop');
      const ready = await invoke<boolean>('wait_for_docker', { timeoutSecs: 90 });
      if (!ready) {
        report('Docker Desktop did not start in time');
      }
    }
    // Re-check daemon after potential start
    const recheckDocker = await invoke<{
      cli_found: boolean;
      daemon_running: boolean;
      desktop_installed: boolean;
    }>('check_docker_status');
    if (recheckDocker.daemon_running) {
      report(`Setting up Ollama via Docker (model: ${modelTag})...`);
      await invoke<string>('auto_setup_local_llm', { modelName: modelTag });
      await checkOllamaStatus();
      if (isOllamaRunning()) {
        report('Ollama running via Docker');
        await fetchInstalledModels();
        return true;
      }
    }
  } else if (runtimes.podman_cli) {
    report(`Setting up Ollama via Podman (model: ${modelTag})...`);
    try {
      await invoke<string>('auto_setup_local_llm_with_runtime', {
        modelName: modelTag,
        preference: 'podman',
      });
      await checkOllamaStatus();
      if (isOllamaRunning()) {
        report('Ollama running via Podman');
        await fetchInstalledModels();
        return true;
      }
    } catch (e) {
      report(`Podman-based Ollama setup failed: ${e}`);
    }
  }

  return false;
}
