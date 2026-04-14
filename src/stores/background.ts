import { computed, ref } from 'vue';
import { defineStore } from 'pinia';

export interface BackgroundOption {
  id: string;
  name: string;
  url: string;
  kind: 'preset' | 'local';
}

const STORAGE_KEY = 'terransoul_background_id';

const PRESET_BACKGROUNDS: BackgroundOption[] = [
  {
    id: 'studio-soft',
    name: 'Studio Soft',
    url: '/backgrounds/studio-soft.svg',
    kind: 'preset',
  },
  {
    id: 'studio-night',
    name: 'Studio Night',
    url: '/backgrounds/studio-night.svg',
    kind: 'preset',
  },
  {
    id: 'sunset-glow',
    name: 'Sunset Glow',
    url: '/backgrounds/sunset-glow.svg',
    kind: 'preset',
  },
  {
    id: 'cyberpunk-city',
    name: 'Cyberpunk City',
    url: '/backgrounds/cyberpunk-city.svg',
    kind: 'preset',
  },
  {
    id: 'enchanted-forest',
    name: 'Enchanted Forest',
    url: '/backgrounds/enchanted-forest.svg',
    kind: 'preset',
  },
  {
    id: 'deep-ocean',
    name: 'Deep Ocean',
    url: '/backgrounds/deep-ocean.svg',
    kind: 'preset',
  },
  {
    id: 'cosmic-nebula',
    name: 'Cosmic Nebula',
    url: '/backgrounds/cosmic-nebula.svg',
    kind: 'preset',
  },
];

function loadStoredBackgroundId() {
  if (typeof localStorage === 'undefined') {
    return PRESET_BACKGROUNDS[0].id;
  }

  return localStorage.getItem(STORAGE_KEY) ?? PRESET_BACKGROUNDS[0].id;
}

export const useBackgroundStore = defineStore('background', () => {
  const presetBackgrounds = ref<BackgroundOption[]>(PRESET_BACKGROUNDS);
  const localBackgrounds = ref<BackgroundOption[]>([]);
  const selectedBackgroundId = ref(loadStoredBackgroundId());
  const importError = ref<string | undefined>(undefined);

  const allBackgrounds = computed(() => [
    ...presetBackgrounds.value,
    ...localBackgrounds.value,
  ]);

  const currentBackground = computed(() => {
    return allBackgrounds.value.find((bg) => bg.id === selectedBackgroundId.value)
      ?? presetBackgrounds.value[0];
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
      selectedBackgroundId.value = presetBackgrounds.value[0].id;
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
    presetBackgrounds,
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
