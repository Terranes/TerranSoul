import { computed, ref } from 'vue';
import { defineStore } from 'pinia';
import type { RemoteHost } from '../transport';
import { createGrpcWebRemoteHost } from '../transport/grpc_web';
import {
  assessBrowserLanConnect,
  browserLanAutoDiscoverySupport,
  clearBrowserLanHost,
  loadBrowserLanHost,
  parseBrowserLanEndpoint,
  saveBrowserLanHost,
  type BrowserLanHostConfig,
} from '../utils/browser-lan';

export type BrowserLanStatus = 'idle' | 'blocked' | 'probing' | 'connected' | 'failed';

export interface BrowserLanAdapters {
  createRemoteHost: (baseUrl: string) => RemoteHost;
  now: () => number;
  pageProtocol: () => string;
}

let adapterOverrides: Partial<BrowserLanAdapters> = {};

export function configureBrowserLanAdapters(overrides: Partial<BrowserLanAdapters>): void {
  adapterOverrides = overrides;
}

export function resetBrowserLanAdapters(): void {
  adapterOverrides = {};
}

export const useBrowserLanStore = defineStore('browser-lan', () => {
  const initial = loadBrowserLanHost();
  const hostInput = ref(initial?.host ?? '');
  const portInput = ref(String(initial?.port ?? 7422));
  const secure = ref(initial?.secure ?? false);
  const savedHost = ref<BrowserLanHostConfig | null>(initial);
  const status = ref<BrowserLanStatus>(initial ? 'connected' : 'idle');
  const error = ref<string | null>(null);
  const remoteSummary = ref<string | null>(null);
  const autoDiscovery = browserLanAutoDiscoverySupport();

  const endpointPreview = computed(() => {
    try {
      return parseCurrentEndpoint().baseUrl;
    } catch {
      return '';
    }
  });
  const canOpenRemoteChat = computed(() => savedHost.value !== null);

  async function probeAndSave(): Promise<BrowserLanHostConfig | null> {
    error.value = null;
    remoteSummary.value = null;
    let endpoint;
    try {
      endpoint = parseCurrentEndpoint();
    } catch (parseError) {
      status.value = 'failed';
      error.value = errorMessage(parseError);
      return null;
    }

    const assessment = assessBrowserLanConnect(endpoint, currentAdapters().pageProtocol());
    if (!assessment.canAttempt) {
      status.value = 'blocked';
      error.value = assessment.reason;
      return null;
    }

    status.value = 'probing';
    try {
      const remoteHost = currentAdapters().createRemoteHost(endpoint.baseUrl);
      const [health, system] = await Promise.all([
        remoteHost.brainHealth(),
        remoteHost.getSystemStatus().catch(() => null),
      ]);
      const config = saveBrowserLanHost(endpoint, { now: currentAdapters().now });
      savedHost.value = config;
      status.value = 'connected';
      remoteSummary.value = `${health.brainProvider}${health.brainModel ? ` · ${health.brainModel}` : ''} · ${system?.memoryEntryCount ?? health.memoryTotal} memories`;
      return config;
    } catch (probeError) {
      status.value = 'failed';
      error.value = `Could not reach a browser-compatible TerranSoul host at ${endpoint.baseUrl}: ${errorMessage(probeError)}`;
      return null;
    }
  }

  function loadSaved(): BrowserLanHostConfig | null {
    savedHost.value = loadBrowserLanHost();
    if (savedHost.value) {
      hostInput.value = savedHost.value.host;
      portInput.value = String(savedHost.value.port);
      secure.value = savedHost.value.secure;
      status.value = 'connected';
    }
    return savedHost.value;
  }

  function clearSaved(): void {
    clearBrowserLanHost();
    savedHost.value = null;
    remoteSummary.value = null;
    status.value = 'idle';
  }

  function clearError(): void {
    error.value = null;
  }

  function parseCurrentEndpoint() {
    return parseBrowserLanEndpoint(hostInput.value, {
      defaultPort: Number(portInput.value) || 7422,
      secure: secure.value,
    });
  }

  return {
    hostInput,
    portInput,
    secure,
    savedHost,
    status,
    error,
    remoteSummary,
    autoDiscovery,
    endpointPreview,
    canOpenRemoteChat,
    probeAndSave,
    loadSaved,
    clearSaved,
    clearError,
  };
});

function currentAdapters(): BrowserLanAdapters {
  return {
    createRemoteHost: adapterOverrides.createRemoteHost ?? ((baseUrl) => createGrpcWebRemoteHost({ baseUrl })),
    now: adapterOverrides.now ?? Date.now,
    pageProtocol: adapterOverrides.pageProtocol ?? (() => (typeof window === 'undefined' ? 'https:' : window.location.protocol)),
  };
}

function errorMessage(value: unknown): string {
  return value instanceof Error ? value.message : String(value);
}
