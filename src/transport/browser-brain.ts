import type { BrainMode, FreeProvider } from '../types';
import type { RemoteHost } from './remote-host';

export interface BrowserDirectProvider {
  baseUrl: string;
  model: string;
  apiKey: string | null;
  providerId: string;
}

export type BrowserBrainTransport =
  | { kind: 'direct'; provider: BrowserDirectProvider }
  | { kind: 'remote-host'; host: RemoteHost }
  | { kind: 'unconfigured'; reason: string }
  | { kind: 'remote-required'; reason: string };

export function resolveBrowserBrainTransport(
  mode: BrainMode | null,
  freeProviders: FreeProvider[],
  remoteHost?: RemoteHost | null,
): BrowserBrainTransport {
  if (remoteHost) return { kind: 'remote-host', host: remoteHost };
  if (!mode) {
    return { kind: 'unconfigured', reason: 'No browser brain mode is configured.' };
  }

  if (mode.mode === 'free_api') {
    const provider = freeProviders.find((item) => item.id === mode.provider_id);
    if (!provider) {
      return { kind: 'unconfigured', reason: `Free provider ${mode.provider_id} is not available in this browser build.` };
    }
    if (provider.requires_api_key && !mode.api_key) {
      return { kind: 'unconfigured', reason: `${provider.display_name} requires an API key before browser chat can use it.` };
    }
    return {
      kind: 'direct',
      provider: {
        baseUrl: provider.base_url,
        model: provider.model,
        apiKey: mode.api_key ?? null,
        providerId: provider.id,
      },
    };
  }

  if (mode.mode === 'paid_api') {
    return {
      kind: 'direct',
      provider: {
        baseUrl: mode.base_url,
        model: mode.model,
        apiKey: mode.api_key,
        providerId: mode.provider,
      },
    };
  }

  return {
    kind: 'remote-required',
    reason: 'Local browser chat requires a paired TerranSoul host; local Ollama/LM Studio are not browser-direct transports.',
  };
}

export function browserDirectFallbackProviders(
  primary: BrowserDirectProvider,
  mode: BrainMode | null,
  freeProviders: FreeProvider[],
): BrowserDirectProvider[] {
  const providers = [primary];
  for (const provider of freeProviders) {
    if (providers.some((item) => item.baseUrl === provider.base_url)) continue;
    const apiKey = mode?.mode === 'free_api' && mode.provider_id === provider.id ? mode.api_key : null;
    if (provider.requires_api_key && !apiKey) continue;
    providers.push({
      baseUrl: provider.base_url,
      model: provider.model,
      apiKey: apiKey ?? null,
      providerId: provider.id,
    });
  }
  return providers;
}
