import { ref, computed } from 'vue';
import { useWindowStore } from '../stores/window';

/**
 * Global chat expansion state management
 * Tracks whether chat interfaces are expanded across different views
 */
const chatDrawerExpanded = ref(false); // Main chat view drawer state
const petChatExpanded = ref(true); // Pet overlay chat state

export function useChatExpansion() {
  const windowStore = useWindowStore();

  // Computed property to determine if any chat interface is currently expanded
  const isChatExpanded = computed(() => {
    if (windowStore.mode === 'pet') {
      return petChatExpanded.value;
    } else {
      return chatDrawerExpanded.value;
    }
  });

  // Functions to update chat expansion states
  function setChatDrawerExpanded(expanded: boolean) {
    chatDrawerExpanded.value = expanded;
  }

  function setPetChatExpanded(expanded: boolean) {
    petChatExpanded.value = expanded;
  }

  function toggleChatDrawer() {
    chatDrawerExpanded.value = !chatDrawerExpanded.value;
    return chatDrawerExpanded.value;
  }

  function togglePetChat() {
    petChatExpanded.value = !petChatExpanded.value;
    return petChatExpanded.value;
  }

  return {
    // States
    isChatExpanded,
    chatDrawerExpanded: computed(() => chatDrawerExpanded.value),
    petChatExpanded: computed(() => petChatExpanded.value),
    
    // Actions
    setChatDrawerExpanded,
    setPetChatExpanded,
    toggleChatDrawer,
    togglePetChat,
  };
}