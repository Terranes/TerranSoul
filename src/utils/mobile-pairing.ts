import type { SecurePairingRecord } from './secure-pairing-store';

export interface MobilePairPayload {
  host: string;
  port: number;
  tokenB64: string;
  fingerprintB64: string;
  expiresAtUnixMs: number;
  rawUri: string;
}

export function parsePairingUri(input: string, nowUnixMs = Date.now()): MobilePairPayload {
  const rawUri = input.trim();
  let url: URL;
  try {
    url = new URL(rawUri);
  } catch {
    throw new Error('Pairing QR must be a valid terransoul://pair URI.');
  }

  if (url.protocol !== 'terransoul:') {
    throw new Error('Pairing QR must use the terransoul:// scheme.');
  }
  if (url.hostname !== 'pair') {
    throw new Error('Pairing QR must target terransoul://pair.');
  }

  const host = requiredParam(url, 'host');
  const port = parseIntegerParam(url, 'port');
  const tokenB64 = requiredParam(url, 'token');
  const fingerprintB64 = requiredParam(url, 'fp');
  const expiresAtUnixMs = parseIntegerParam(url, 'exp');

  if (port <= 0 || port > 65535) {
    throw new Error('Pairing QR contains an invalid port.');
  }
  if (expiresAtUnixMs <= nowUnixMs) {
    throw new Error('Pairing QR has expired.');
  }

  return { host, port, tokenB64, fingerprintB64, expiresAtUnixMs, rawUri };
}

export function pairingFingerprintMismatch(
  record: SecurePairingRecord | null,
  payload: MobilePairPayload | null,
): boolean {
  const existing = record?.credentials.desktopFingerprintB64;
  return Boolean(existing && payload && existing !== payload.fingerprintB64);
}

export function formatPairingEndpoint(payload: MobilePairPayload): string {
  return `${payload.host}:${payload.port}`;
}

export function fingerprintPreview(fingerprintB64: string): string {
  if (fingerprintB64.length <= 18) return fingerprintB64;
  return `${fingerprintB64.slice(0, 8)}...${fingerprintB64.slice(-6)}`;
}

export async function scanPairingQrCode(): Promise<string> {
  const barcodeScanner = await import('@tauri-apps/plugin-barcode-scanner');
  const result = await barcodeScanner.scan({
    windowed: true,
    formats: [barcodeScanner.Format.QRCode],
  });
  return extractScanValue(result);
}

export function extractScanValue(result: unknown): string {
  if (typeof result === 'string' && result.trim()) return result.trim();
  if (Array.isArray(result)) {
    for (const item of result) {
      const value = extractScanValueOrNull(item);
      if (value) return value;
    }
  }
  const value = extractScanValueOrNull(result);
  if (value) return value;
  throw new Error('No QR code data returned by scanner.');
}

function extractScanValueOrNull(result: unknown): string | null {
  if (typeof result === 'string') return result.trim() || null;
  if (!result || typeof result !== 'object') return null;
  const record = result as Record<string, unknown>;
  for (const key of ['rawValue', 'value', 'text', 'content', 'data']) {
    const value = record[key];
    if (typeof value === 'string' && value.trim()) return value.trim();
  }
  return null;
}

function requiredParam(url: URL, name: string): string {
  const value = url.searchParams.get(name)?.trim();
  if (!value) throw new Error(`Pairing QR is missing ${name}.`);
  return value;
}

function parseIntegerParam(url: URL, name: string): number {
  const value = requiredParam(url, name);
  const parsed = Number(value);
  if (!Number.isInteger(parsed)) throw new Error(`Pairing QR has an invalid ${name}.`);
  return parsed;
}