import { afterEach, describe, expect, it } from 'vitest';
import {
  isIosRuntime,
  resetRuntimeTargetOverrides,
  setRemoteConversationRuntimeOverride,
  shouldUseRemoteConversation,
} from './runtime-target';
import { BROWSER_LAN_HOST_STORAGE_KEY, parseBrowserLanEndpoint, saveBrowserLanHost } from './browser-lan';

describe('runtime target detection', () => {
  afterEach(() => {
    resetRuntimeTargetOverrides();
    localStorage.removeItem(BROWSER_LAN_HOST_STORAGE_KEY);
  });

  it('detects iPhone and iPad runtimes', () => {
    expect(isIosRuntime({ userAgent: 'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X)' })).toBe(true);
    expect(isIosRuntime({ platform: 'MacIntel', maxTouchPoints: 5 })).toBe(true);
    expect(isIosRuntime({ userAgent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64)' })).toBe(false);
  });

  it('uses explicit query params and test overrides before user-agent detection', () => {
    expect(shouldUseRemoteConversation({ search: '?remoteConversation=1' })).toBe(true);
    expect(shouldUseRemoteConversation({ userAgent: 'iPhone', search: '?remoteChat=local' })).toBe(false);

    setRemoteConversationRuntimeOverride(true);
    expect(shouldUseRemoteConversation({ search: '?remoteConversation=0' })).toBe(true);
  });

  it('uses remote conversation when a browser LAN host is saved', () => {
    saveBrowserLanHost(parseBrowserLanEndpoint('https://desktop.local:7422'));

    expect(shouldUseRemoteConversation({ userAgent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64)' })).toBe(true);
  });

  it('does not enable remote conversation on iOS without a paired host', () => {
    // iOS Safari on Vercel/static deployment has no Tauri backend and no LAN
    // host saved — the app must fall through to browser/cloud-LLM mode so the
    // user is prompted to configure a provider instead of being stranded with
    // a stale "Remote Desktop" badge that points nowhere.
    expect(
      shouldUseRemoteConversation({ userAgent: 'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X)' }),
    ).toBe(false);
    expect(shouldUseRemoteConversation({ platform: 'MacIntel', maxTouchPoints: 5 })).toBe(false);
  });

  it('still enables remote conversation on iOS once a LAN host is paired', () => {
    saveBrowserLanHost(parseBrowserLanEndpoint('https://desktop.local:7422'));

    expect(
      shouldUseRemoteConversation({ userAgent: 'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X)' }),
    ).toBe(true);
  });
});
