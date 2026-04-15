/**
 * Composable for managing ASR hotwords (domain-specific keywords).
 *
 * Hotwords boost recognition of character names, game terms, and other
 * domain-specific vocabulary in the ASR pipeline.
 */

import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

export interface Hotword {
  phrase: string;
  boost: number;
}

export function useHotwords() {
  const hotwords = ref<Hotword[]>([]);
  const isLoading = ref(false);
  const error = ref<string | null>(null);

  async function loadHotwords(): Promise<void> {
    isLoading.value = true;
    error.value = null;
    try {
      hotwords.value = await invoke<Hotword[]>('get_hotwords');
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
    } finally {
      isLoading.value = false;
    }
  }

  async function addHotword(phrase: string, boost?: number): Promise<void> {
    isLoading.value = true;
    error.value = null;
    try {
      await invoke('add_hotword', { phrase, boost: boost ?? 5.0 });
      await loadHotwords();
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
    } finally {
      isLoading.value = false;
    }
  }

  async function removeHotword(phrase: string): Promise<void> {
    isLoading.value = true;
    error.value = null;
    try {
      await invoke('remove_hotword', { phrase });
      await loadHotwords();
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
    } finally {
      isLoading.value = false;
    }
  }

  async function clearHotwords(): Promise<void> {
    isLoading.value = true;
    error.value = null;
    try {
      await invoke('clear_hotwords');
      hotwords.value = [];
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
    } finally {
      isLoading.value = false;
    }
  }

  return { hotwords, isLoading, error, loadHotwords, addHotword, removeHotword, clearHotwords };
}
