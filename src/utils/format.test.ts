import { describe, it, expect } from 'vitest';
import { formatRam } from './format';

describe('formatRam', () => {
  it('returns MB for values under 1024', () => {
    expect(formatRam(0)).toBe('0 MB');
    expect(formatRam(512)).toBe('512 MB');
    expect(formatRam(1023)).toBe('1023 MB');
  });

  it('returns GB with one decimal for values ≥ 1024', () => {
    expect(formatRam(1024)).toBe('1.0 GB');
    expect(formatRam(2048)).toBe('2.0 GB');
    expect(formatRam(8192)).toBe('8.0 GB');
    expect(formatRam(16_384)).toBe('16.0 GB');
  });

  it('rounds non-power-of-two values to one decimal place', () => {
    expect(formatRam(1536)).toBe('1.5 GB');
    expect(formatRam(6144)).toBe('6.0 GB');
    expect(formatRam(12_288)).toBe('12.0 GB');
  });
});
