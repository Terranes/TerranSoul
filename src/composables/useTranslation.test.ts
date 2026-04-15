/**
 * Tests for useTranslation composable.
 *
 * Tests cover:
 * - loadLanguages calls invoke and populates supportedLanguages
 * - loadLanguages handles error
 * - translate calls invoke with correct args
 * - translate sets lastResult on success
 * - translate handles error
 * - translate manages isTranslating state
 * - detectLanguage returns lang and confidence
 * - detectLanguage handles error
 * - Initial state is correct (no result, not translating)
 * - translate returns null on error
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useTranslation } from './useTranslation';
import type { TranslationResult } from '../types';

// ── Tauri IPC mock ────────────────────────────────────────────────────────────

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

const stubLanguages: [string, string][] = [
  ['en', 'English'],
  ['ja', 'Japanese'],
  ['es', 'Spanish'],
];

const stubResult: TranslationResult = {
  original: 'Hello',
  source_lang: 'en',
  translated: '[ja] Hello',
  target_lang: 'ja',
  confidence: 0.5,
};

describe('useTranslation', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it('initialises with correct default state', () => {
    const { isTranslating, error, lastResult, supportedLanguages } = useTranslation();
    expect(isTranslating.value).toBe(false);
    expect(error.value).toBeNull();
    expect(lastResult.value).toBeNull();
    expect(supportedLanguages.value).toEqual([]);
  });

  it('loadLanguages calls invoke and populates supportedLanguages', async () => {
    mockInvoke.mockResolvedValue(stubLanguages);

    const { supportedLanguages, loadLanguages } = useTranslation();
    await loadLanguages();

    expect(mockInvoke).toHaveBeenCalledWith('list_languages');
    expect(supportedLanguages.value).toEqual([
      { code: 'en', name: 'English' },
      { code: 'ja', name: 'Japanese' },
      { code: 'es', name: 'Spanish' },
    ]);
  });

  it('loadLanguages sets error on failure', async () => {
    mockInvoke.mockRejectedValue('load failed');

    const { error, loadLanguages } = useTranslation();
    await loadLanguages();

    expect(error.value).toBe('load failed');
  });

  it('translate calls invoke with correct args', async () => {
    mockInvoke.mockResolvedValue(stubResult);

    const { translate } = useTranslation();
    await translate('Hello', 'en', 'ja');

    expect(mockInvoke).toHaveBeenCalledWith('translate_text', {
      text: 'Hello',
      sourceLang: 'en',
      targetLang: 'ja',
    });
  });

  it('translate sets lastResult on success', async () => {
    mockInvoke.mockResolvedValue(stubResult);

    const { lastResult, translate } = useTranslation();
    const result = await translate('Hello', 'en', 'ja');

    expect(result).toEqual(stubResult);
    expect(lastResult.value).toEqual(stubResult);
  });

  it('translate handles error and returns null', async () => {
    mockInvoke.mockRejectedValue('translate failed');

    const { error, translate } = useTranslation();
    const result = await translate('Hello', 'en', 'ja');

    expect(result).toBeNull();
    expect(error.value).toBe('translate failed');
  });

  it('translate resets isTranslating after success', async () => {
    mockInvoke.mockResolvedValue(stubResult);

    const { isTranslating, translate } = useTranslation();
    await translate('Hello', 'en', 'ja');

    expect(isTranslating.value).toBe(false);
  });

  it('translate resets isTranslating after error', async () => {
    mockInvoke.mockRejectedValue('fail');

    const { isTranslating, translate } = useTranslation();
    await translate('Hello', 'en', 'ja');

    expect(isTranslating.value).toBe(false);
  });

  it('translate clears previous error on new call', async () => {
    mockInvoke.mockRejectedValueOnce('first error');
    mockInvoke.mockResolvedValueOnce(stubResult);

    const { error, translate } = useTranslation();
    await translate('Hello', 'en', 'ja');
    expect(error.value).toBe('first error');

    await translate('Hello', 'en', 'ja');
    expect(error.value).toBeNull();
  });

  it('detectLanguage returns lang and confidence', async () => {
    mockInvoke.mockResolvedValue(['en', 0.9]);

    const { detectLanguage } = useTranslation();
    const result = await detectLanguage('Hello world');

    expect(mockInvoke).toHaveBeenCalledWith('detect_language', { text: 'Hello world' });
    expect(result).toEqual({ lang: 'en', confidence: 0.9 });
  });

  it('detectLanguage handles error and returns null', async () => {
    mockInvoke.mockRejectedValue('detect failed');

    const { error, detectLanguage } = useTranslation();
    const result = await detectLanguage('Hello');

    expect(result).toBeNull();
    expect(error.value).toBe('detect failed');
  });
});
