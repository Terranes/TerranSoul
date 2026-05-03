import { useConversationStore } from './conversation';
import { useRemoteConversationStore } from './remote-conversation';
import { shouldUseRemoteConversation } from '../utils/runtime-target';

export type ChatConversationStore =
  | ReturnType<typeof useConversationStore>
  | ReturnType<typeof useRemoteConversationStore>;

export function shouldUseRemoteChatStore(): boolean {
  return shouldUseRemoteConversation();
}

export function useChatConversationStore(): ChatConversationStore {
  return shouldUseRemoteChatStore()
    ? useRemoteConversationStore()
    : useConversationStore();
}