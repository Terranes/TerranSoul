import { describe, it, expect, beforeEach, vi } from 'vitest';
import { reactive } from 'vue';
import { useChatExpansion } from './useChatExpansion';

// Mock window store — use reactive() so property access returns unwrapped values
// (matching Pinia's composition-API store behaviour)
const mockWindowStore = reactive({
  mode: 'window' as 'window' | 'pet'
});

vi.mock('../stores/window', () => ({
  useWindowStore: () => mockWindowStore
}));

describe('useChatExpansion', () => {
  beforeEach(() => {
    // Reset to default state
    const { setChatDrawerExpanded, setPetChatExpanded } = useChatExpansion();
    setChatDrawerExpanded(false);
    setPetChatExpanded(true);
    mockWindowStore.mode = 'window';
  });

  it('should initialize with correct default states', () => {
    const { chatDrawerExpanded, petChatExpanded, isChatExpanded } = useChatExpansion();
    
    expect(chatDrawerExpanded.value).toBe(false);
    expect(petChatExpanded.value).toBe(true);
    expect(isChatExpanded.value).toBe(false); // Window mode, drawer not expanded
  });

  it('should reflect chat drawer expansion in window mode', () => {
    mockWindowStore.mode = 'window';
    const { setChatDrawerExpanded, isChatExpanded } = useChatExpansion();
    
    setChatDrawerExpanded(true);
    expect(isChatExpanded.value).toBe(true);
    
    setChatDrawerExpanded(false);
    expect(isChatExpanded.value).toBe(false);
  });

  it('should reflect pet chat expansion in pet mode', () => {
    mockWindowStore.mode = 'pet';
    const { setPetChatExpanded, isChatExpanded } = useChatExpansion();
    
    setPetChatExpanded(true);
    expect(isChatExpanded.value).toBe(true);
    
    setPetChatExpanded(false);
    expect(isChatExpanded.value).toBe(false);
  });

  it('should toggle chat drawer correctly', () => {
    const { toggleChatDrawer, chatDrawerExpanded } = useChatExpansion();
    
    expect(chatDrawerExpanded.value).toBe(false);
    
    let result = toggleChatDrawer();
    expect(result).toBe(true);
    expect(chatDrawerExpanded.value).toBe(true);
    
    result = toggleChatDrawer();
    expect(result).toBe(false);
    expect(chatDrawerExpanded.value).toBe(false);
  });

  it('should toggle pet chat correctly', () => {
    const { togglePetChat, petChatExpanded } = useChatExpansion();
    
    expect(petChatExpanded.value).toBe(true);
    
    let result = togglePetChat();
    expect(result).toBe(false);
    expect(petChatExpanded.value).toBe(false);
    
    result = togglePetChat();
    expect(result).toBe(true);
    expect(petChatExpanded.value).toBe(true);
  });

  it('should prioritize correct chat state based on window mode', () => {
    const { setChatDrawerExpanded, setPetChatExpanded, isChatExpanded } = useChatExpansion();
    
    // Set both to true
    setChatDrawerExpanded(true);
    setPetChatExpanded(true);
    
    // In window mode, should use drawer state
    mockWindowStore.mode = 'window';
    expect(isChatExpanded.value).toBe(true);
    
    setChatDrawerExpanded(false);
    expect(isChatExpanded.value).toBe(false);
    
    // In pet mode, should use pet chat state
    mockWindowStore.mode = 'pet';
    expect(isChatExpanded.value).toBe(true); // petChatExpanded is still true
    
    setPetChatExpanded(false);
    expect(isChatExpanded.value).toBe(false);
  });
});