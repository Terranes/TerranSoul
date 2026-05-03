import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { useChatConversationStore } from './chat-store-router';
import {
  resetRuntimeTargetOverrides,
  setRemoteConversationRuntimeOverride,
} from '../utils/runtime-target';

describe('chat store router', () => {
  beforeEach(() => setActivePinia(createPinia()));
  afterEach(() => resetRuntimeTargetOverrides());

  it('selects the remote conversation store when the runtime asks for remote chat', () => {
    setRemoteConversationRuntimeOverride(true);
    const store = useChatConversationStore();
    expect(store.$id).toBe('remote-conversation');
  });

  it('keeps the local conversation store for desktop runtimes', () => {
    setRemoteConversationRuntimeOverride(false);
    const store = useChatConversationStore();
    expect(store.$id).toBe('conversation');
  });
});