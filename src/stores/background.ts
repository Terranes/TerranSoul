import { computed, ref } from 'vue';
import { defineStore } from 'pinia';

export interface BackgroundOption {
  id: string;
  name: string;
  /** Empty string for the auto option — body CSS gradient is used instead. */
  url: string;
  kind: 'auto' | 'local';
}

const STORAGE_KEY = 'terransoul_background_id';

/** The single built-in option: delegates to the CSS theme gradient. */
const AUTO_BACKGROUND: BackgroundOption = {
  id: 'auto',
  name: 'Auto',
  url: '',
  kind: 'auto',
};

function loadStoredBackgroundId(): string {
  if (typeof localStorage === 'undefined') return AUTO_BACKGROUND.id;
  const stored = localStorage.getItem(STORAGE_KEY);
  // Migrate away from old preset ids that no longer exist.
  if (!stored || stored !== 'auto' && !stored.startsWith('local-')) {
    return AUTO_BACKGROUND.id;
  }
  return stored;
}

export const useBackgroundStore = defineStore('background', () => {
  const localBackgrounds = ref<BackgroundOption[]>([]);
  const selectedBackgroundId = ref(loadStoredBackgroundId());
  const importError = ref<string | undefined>(undefined);

  const allBackgrounds = computed<BackgroundOption[]>(() => [
    AUTO_BACKGROUND,
    ...localBackgrounds.value,
  ]);

  const currentBackground = computed<BackgroundOption>(() => {
    return allBackgrounds.value.find((bg) => bg.id === selectedBackgroundId.value)
      ?? AUTO_BACKGROUND;
  });

  function persistSelection() {
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(STORAGE_KEY, selectedBackgroundId.value);
    }
  }

  function selectBackground(id: string) {
    selectedBackgroundId.value = id;
    persistSelection();
  }

  function ensureValidSelection() {
    if (!allBackgrounds.value.some((bg) => bg.id === selectedBackgroundId.value)) {
      selectedBackgroundId.value = AUTO_BACKGROUND.id;
      persistSelection();
    }
  }

  async function importLocalBackground(file: File) {
    importError.value = undefined;

    if (!file.type.startsWith('image/')) {
      importError.value = 'Please choose an image file.';
      return false;
    }

    const url = URL.createObjectURL(file);
    const option: BackgroundOption = {
      id: `local-${Date.now()}`,
      name: file.name.replace(/\.[^.]+$/, ''),
      url,
      kind: 'local',
    };

    localBackgrounds.value.unshift(option);
    selectBackground(option.id);
    return true;
  }

  return {
    localBackgrounds,
    selectedBackgroundId,
    currentBackground,
    allBackgrounds,
    importError,
    selectBackground,
    ensureValidSelection,
    importLocalBackground,
  };
});
