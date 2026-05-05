export const BROWSER_LAN_HOST_STORAGE_KEY = 'ts.browser.lan.host';
export const BROWSER_LAN_DEFAULT_PORT = 7422;

export const BROWSER_LAN_AUTO_DISCOVERY_REASON =
  'A backendless HTTPS browser page cannot enumerate TerranSoul hosts on a LAN. Browsers cannot listen for mDNS/UDP advertisements, cannot read ARP/router tables, and cannot safely scan private subnets from a public origin.';

export const BROWSER_LAN_DEFAULT_CAPABILITIES = [
  'chat',
  'copilot:read',
  'workflow:read',
  'workflow:continue',
];

export interface BrowserLanAutoDiscoverySupport {
  supported: false;
  reason: string;
}

export interface BrowserLanHostConfig {
  schemaVersion: 1;
  host: string;
  port: number;
  secure: boolean;
  baseUrl: string;
  savedAt: number;
  capabilities: string[];
  label?: string;
}

export interface ParsedBrowserLanEndpoint {
  host: string;
  port: number;
  secure: boolean;
  baseUrl: string;
}

export type BrowserLanHostClass = 'loopback' | 'private-lan' | 'public-or-dns';

export interface BrowserLanConnectAssessment {
  canAttempt: boolean;
  hostClass: BrowserLanHostClass;
  reason: string;
}

export function browserLanAutoDiscoverySupport(): BrowserLanAutoDiscoverySupport {
  return {
    supported: false,
    reason: BROWSER_LAN_AUTO_DISCOVERY_REASON,
  };
}

export function parseBrowserLanEndpoint(
  input: string,
  options: { defaultPort?: number; secure?: boolean } = {},
): ParsedBrowserLanEndpoint {
  const trimmed = input.trim();
  if (!trimmed) throw new Error('Enter a TerranSoul host or URL.');

  const defaultSecure = options.secure ?? false;
  const rawUrl = hasUrlScheme(trimmed)
    ? trimmed
    : `${defaultSecure ? 'https' : 'http'}://${trimmed}`;
  let url: URL;
  try {
    url = new URL(rawUrl);
  } catch {
    throw new Error('Enter a valid host, host:port, or http(s) URL.');
  }

  if (url.protocol !== 'http:' && url.protocol !== 'https:') {
    throw new Error('LAN hosts must use http or https.');
  }

  const host = stripIpv6Brackets(url.hostname.trim());
  if (!host) throw new Error('LAN host is required.');

  const port = parseEndpointPort(url.port, options.defaultPort ?? BROWSER_LAN_DEFAULT_PORT);
  const secure = url.protocol === 'https:';
  return {
    host,
    port,
    secure,
    baseUrl: `${secure ? 'https' : 'http'}://${formatHostForUrl(host)}:${port}`,
  };
}

export function assessBrowserLanConnect(
  endpoint: ParsedBrowserLanEndpoint,
  pageProtocol = currentPageProtocol(),
): BrowserLanConnectAssessment {
  const hostClass = classifyBrowserLanHost(endpoint.host);
  if (endpoint.secure) {
    return {
      canAttempt: true,
      hostClass,
      reason: 'HTTPS LAN hosts can be attempted when the certificate is browser-trusted and the server allows this origin.',
    };
  }

  if (pageProtocol === 'https:' && hostClass !== 'loopback') {
    return {
      canAttempt: false,
      hostClass,
      reason: 'This HTTPS page cannot call a plaintext private LAN host. Use a native paired app, same-machine loopback, or a browser-trusted HTTPS LAN endpoint.',
    };
  }

  return {
    canAttempt: true,
    hostClass,
    reason: 'Plain HTTP can be attempted only from local development pages or browser-trusted loopback; the TerranSoul host must still allow browser requests.',
  };
}

export function classifyBrowserLanHost(host: string): BrowserLanHostClass {
  const normalized = stripIpv6Brackets(host.trim().toLowerCase());
  if (isLoopbackHost(normalized)) return 'loopback';
  if (isPrivateIpv4(normalized) || normalized.endsWith('.local')) return 'private-lan';
  return 'public-or-dns';
}

export function saveBrowserLanHost(
  endpoint: ParsedBrowserLanEndpoint,
  options: { now?: () => number; capabilities?: string[]; label?: string } = {},
): BrowserLanHostConfig {
  const config: BrowserLanHostConfig = {
    schemaVersion: 1,
    host: endpoint.host,
    port: endpoint.port,
    secure: endpoint.secure,
    baseUrl: endpoint.baseUrl,
    savedAt: (options.now ?? Date.now)(),
    capabilities: options.capabilities ?? [...BROWSER_LAN_DEFAULT_CAPABILITIES],
    label: options.label,
  };
  localStorageSafe()?.setItem(BROWSER_LAN_HOST_STORAGE_KEY, JSON.stringify(config));
  return config;
}

export function loadBrowserLanHost(): BrowserLanHostConfig | null {
  const raw = localStorageSafe()?.getItem(BROWSER_LAN_HOST_STORAGE_KEY);
  if (!raw) return null;
  try {
    const parsed = JSON.parse(raw) as Partial<BrowserLanHostConfig>;
    if (parsed.schemaVersion !== 1 || !parsed.host || !parsed.port || !parsed.baseUrl) {
      return null;
    }
    return {
      schemaVersion: 1,
      host: parsed.host,
      port: parsed.port,
      secure: parsed.secure === true,
      baseUrl: parsed.baseUrl,
      savedAt: parsed.savedAt ?? 0,
      capabilities: parsed.capabilities ?? [...BROWSER_LAN_DEFAULT_CAPABILITIES],
      label: parsed.label,
    };
  } catch {
    return null;
  }
}

export function hasStoredBrowserLanHost(): boolean {
  return loadBrowserLanHost() !== null;
}

export function clearBrowserLanHost(): void {
  localStorageSafe()?.removeItem(BROWSER_LAN_HOST_STORAGE_KEY);
}

function hasUrlScheme(value: string): boolean {
  return /^[a-z][a-z0-9+.-]*:\/\//i.test(value);
}

function parseEndpointPort(rawPort: string, defaultPort: number): number {
  if (!rawPort) return defaultPort;
  const port = Number(rawPort);
  if (!Number.isInteger(port) || port <= 0 || port > 65535) {
    throw new Error('LAN host port must be between 1 and 65535.');
  }
  return port;
}

function stripIpv6Brackets(host: string): string {
  return host.startsWith('[') && host.endsWith(']') ? host.slice(1, -1) : host;
}

function formatHostForUrl(host: string): string {
  return host.includes(':') && !host.startsWith('[') ? `[${host}]` : host;
}

function isLoopbackHost(host: string): boolean {
  return host === 'localhost' || host === '::1' || host === '0:0:0:0:0:0:0:1' || host === '127.0.0.1' || host.startsWith('127.');
}

function isPrivateIpv4(host: string): boolean {
  const parts = host.split('.').map((part) => Number(part));
  if (parts.length !== 4 || parts.some((part) => !Number.isInteger(part) || part < 0 || part > 255)) {
    return false;
  }
  const [first, second] = parts;
  return first === 10
    || (first === 192 && second === 168)
    || (first === 172 && second >= 16 && second <= 31)
    || (first === 169 && second === 254);
}

function currentPageProtocol(): string {
  return typeof window === 'undefined' ? 'https:' : window.location.protocol;
}

function localStorageSafe(): Storage | null {
  try {
    return typeof window === 'undefined' ? null : window.localStorage;
  } catch {
    return null;
  }
}
