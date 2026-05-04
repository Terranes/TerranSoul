import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import {
  BROWSER_LAN_AUTO_DISCOVERY_REASON,
  BROWSER_LAN_HOST_STORAGE_KEY,
  assessBrowserLanConnect,
  browserLanAutoDiscoverySupport,
  classifyBrowserLanHost,
  clearBrowserLanHost,
  loadBrowserLanHost,
  parseBrowserLanEndpoint,
  saveBrowserLanHost,
} from './browser-lan';

describe('browser LAN helpers', () => {
  beforeEach(() => {
    localStorage.removeItem(BROWSER_LAN_HOST_STORAGE_KEY);
  });

  afterEach(() => {
    localStorage.removeItem(BROWSER_LAN_HOST_STORAGE_KEY);
  });

  it('states that backendless browser auto-discovery is unsupported', () => {
    expect(browserLanAutoDiscoverySupport()).toEqual({
      supported: false,
      reason: BROWSER_LAN_AUTO_DISCOVERY_REASON,
    });
  });

  it('parses host, host:port, and https URL inputs', () => {
    expect(parseBrowserLanEndpoint('192.168.1.42')).toMatchObject({
      host: '192.168.1.42',
      port: 7422,
      secure: false,
      baseUrl: 'http://192.168.1.42:7422',
    });
    expect(parseBrowserLanEndpoint('desktop.local:7444', { secure: true })).toMatchObject({
      host: 'desktop.local',
      port: 7444,
      secure: true,
      baseUrl: 'https://desktop.local:7444',
    });
    expect(parseBrowserLanEndpoint('https://host.example:9443/mcp')).toMatchObject({
      host: 'host.example',
      port: 9443,
      secure: true,
      baseUrl: 'https://host.example:9443',
    });
  });

  it('blocks plaintext private LAN calls from an HTTPS page but permits loopback attempts', () => {
    const privateHost = parseBrowserLanEndpoint('192.168.1.42');
    const loopback = parseBrowserLanEndpoint('127.0.0.1');

    expect(assessBrowserLanConnect(privateHost, 'https:')).toMatchObject({
      canAttempt: false,
      hostClass: 'private-lan',
    });
    expect(assessBrowserLanConnect(loopback, 'https:')).toMatchObject({
      canAttempt: true,
      hostClass: 'loopback',
    });
  });

  it('classifies common LAN hostnames', () => {
    expect(classifyBrowserLanHost('localhost')).toBe('loopback');
    expect(classifyBrowserLanHost('10.0.0.5')).toBe('private-lan');
    expect(classifyBrowserLanHost('desktop.local')).toBe('private-lan');
    expect(classifyBrowserLanHost('example.com')).toBe('public-or-dns');
  });

  it('saves and reloads a manual LAN host', () => {
    const endpoint = parseBrowserLanEndpoint('https://desktop.local:7422');
    const saved = saveBrowserLanHost(endpoint, { now: () => 1234, capabilities: ['chat'] });

    expect(saved).toMatchObject({ savedAt: 1234, capabilities: ['chat'] });
    expect(loadBrowserLanHost()).toMatchObject({ baseUrl: 'https://desktop.local:7422' });

    clearBrowserLanHost();
    expect(loadBrowserLanHost()).toBeNull();
  });
});
