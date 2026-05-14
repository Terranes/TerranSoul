import { describe, it, expect } from 'vitest';
import { estimateTokens, estimateTurnTokens } from './token-estimation';

describe('token-estimation', () => {
  describe('estimateTokens', () => {
    it('returns 0 for empty string', () => {
      expect(estimateTokens('')).toBe(0);
    });

    it('estimates ~1 token per 4 characters', () => {
      const text = 'Hello world'; // 11 chars → ceil(11/4) = 3
      expect(estimateTokens(text)).toBe(3);
    });

    it('handles long text', () => {
      const text = 'a'.repeat(16000); // 16000 chars → 4000 tokens
      expect(estimateTokens(text)).toBe(4000);
    });
  });

  describe('estimateTurnTokens', () => {
    it('sums user and assistant token estimates', () => {
      const user = 'x'.repeat(400); // 100 tokens
      const assistant = 'y'.repeat(15600); // 3900 tokens
      expect(estimateTurnTokens(user, assistant)).toBe(4000);
    });
  });
});
