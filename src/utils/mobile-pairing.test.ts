import { describe, expect, it } from 'vitest';
import { extractScanValue, pairingFingerprintMismatch, parsePairingUri } from './mobile-pairing';
import type { SecurePairingRecord } from './secure-pairing-store';

const futureUri = 'terransoul://pair?host=192.168.1.42&port=7422&token=tok_123&fp=fp_abc&exp=9999999999999';

describe('mobile pairing utilities', () => {
  it('parses a TerranSoul pairing URI', () => {
    const payload = parsePairingUri(futureUri, 1000);
    expect(payload.host).toBe('192.168.1.42');
    expect(payload.port).toBe(7422);
    expect(payload.tokenB64).toBe('tok_123');
    expect(payload.fingerprintB64).toBe('fp_abc');
  });

  it('rejects expired pairing URIs', () => {
    expect(() => parsePairingUri('terransoul://pair?host=h&port=1&token=t&fp=f&exp=10', 11))
      .toThrow('expired');
  });

  it('extracts scanner payloads from common result shapes', () => {
    expect(extractScanValue([{ rawValue: '  one  ' }])).toBe('one');
    expect(extractScanValue({ text: futureUri })).toBe(futureUri);
  });

  it('detects fingerprint changes against stored Stronghold records', () => {
    const record: SecurePairingRecord = {
      schemaVersion: 1,
      savedAt: 1,
      credentials: {
        deviceId: 'phone-1',
        clientCertPem: 'cert',
        clientKeyPem: 'key',
        caCertPem: 'ca',
        desktopFingerprintB64: 'old-fingerprint',
      },
    };
    expect(pairingFingerprintMismatch(record, parsePairingUri(futureUri, 1000))).toBe(true);
  });
});