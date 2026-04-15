import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { TranslationResult } from '../types';

export function useTranslation() {
  const isTranslating = ref(false);
  const error = ref<string | null>(null);
  const lastResult = ref<TranslationResult | null>(null);
  const supportedLanguages = ref<Array<{ code: string; name: string }>>([]);

  async function loadLanguages(): Promise<void> {
    try {
      const langs = await invoke<[string, string][]>('list_languages');
      supportedLanguages.value = langs.map(([code, name]) => ({ code, name }));
    } catch (e) {
      error.value = String(e);
    }
  }

  async function translate(
    text: string,
    sourceLang: string,
    targetLang: string,
  ): Promise<TranslationResult | null> {
    isTranslating.value = true;
    error.value = null;
    try {
      const result = await invoke<TranslationResult>('translate_text', {
        text,
        sourceLang,
        targetLang,
      });
      lastResult.value = result;
      return result;
    } catch (e) {
      error.value = String(e);
      return null;
    } finally {
      isTranslating.value = false;
    }
  }

  async function detectLanguage(text: string): Promise<{ lang: string; confidence: number } | null> {
    try {
      const [lang, confidence] = await invoke<[string, number]>('detect_language', { text });
      return { lang, confidence };
    } catch (e) {
      error.value = String(e);
      return null;
    }
  }

  return {
    isTranslating,
    error,
    lastResult,
    supportedLanguages,
    loadLanguages,
    translate,
    detectLanguage,
  };
}
