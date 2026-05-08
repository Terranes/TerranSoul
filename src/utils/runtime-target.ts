import { hasStoredBrowserLanHost } from './browser-lan';

export interface RuntimeTargetSnapshot {
  userAgent?: string;
  platform?: string;
  maxTouchPoints?: number;
  search?: string;
}

let remoteConversationOverride: boolean | null = null;

export function setRemoteConversationRuntimeOverride(value: boolean | null): void {
  remoteConversationOverride = value;
}

export function resetRuntimeTargetOverrides(): void {
  remoteConversationOverride = null;
}

export function isIosRuntime(snapshot: RuntimeTargetSnapshot = currentRuntimeSnapshot()): boolean {
  const userAgent = snapshot.userAgent ?? '';
  const platform = snapshot.platform ?? '';
  if (/iPad|iPhone|iPod/i.test(userAgent)) return true;
  return platform === 'MacIntel' && (snapshot.maxTouchPoints ?? 0) > 1;
}

export function shouldUseRemoteConversation(
  snapshot: RuntimeTargetSnapshot = currentRuntimeSnapshot(),
): boolean {
  if (remoteConversationOverride !== null) return remoteConversationOverride;
  const param = readRemoteConversationParam(snapshot.search ?? '');
  if (param !== null) return param;
  if (isBrowserRuntime() && hasStoredBrowserLanHost()) return true;
  // iOS Safari without a paired desktop has no real backend — fall through to
  // browser/cloud-LLM mode so the user is prompted to configure a provider
  // instead of silently selecting the remote-conversation store and showing
  // a stale "Remote Desktop" badge against an unreachable host.
  return isIosRuntime(snapshot) && hasStoredBrowserLanHost();
}

function currentRuntimeSnapshot(): RuntimeTargetSnapshot {
  if (typeof window === 'undefined') return {};
  return {
    userAgent: window.navigator.userAgent,
    platform: window.navigator.platform,
    maxTouchPoints: window.navigator.maxTouchPoints,
    search: window.location.search,
  };
}

function readRemoteConversationParam(search: string): boolean | null {
  if (!search) return null;
  const params = new URLSearchParams(search.startsWith('?') ? search : `?${search}`);
  const value = params.get('remoteConversation') ?? params.get('remoteChat');
  if (value === null) return null;
  if (['1', 'true', 'yes', 'ios'].includes(value.toLowerCase())) return true;
  if (['0', 'false', 'no', 'local'].includes(value.toLowerCase())) return false;
  return null;
}

function isBrowserRuntime(): boolean {
  return typeof window !== 'undefined' && !('__TAURI_INTERNALS__' in window);
}
