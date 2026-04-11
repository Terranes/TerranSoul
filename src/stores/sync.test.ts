/**
 * Tests for the sync store.
 * Pure state management — no IPC mocking needed since the sync store
 * uses direct state updates via applySyncState().
 */
import { describe, it, expect, beforeEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { useSyncStore } from './sync';
import type { SyncState } from '../types';

const sampleState: SyncState = {
  conversation_count: 42,
  character_selection: 'chosen.vrm',
  agent_count: 3,
  last_synced_at: 1700000000,
};

describe('sync store', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('initial state: all zeroed, no error, not synced', () => {
    const store = useSyncStore();
    expect(store.conversationCount).toBe(0);
    expect(store.characterSelection).toBeNull();
    expect(store.agentCount).toBe(0);
    expect(store.lastSyncedAt).toBeNull();
    expect(store.error).toBeNull();
    expect(store.isSynced).toBe(false);
  });

  it('applySyncState updates all fields', () => {
    const store = useSyncStore();
    store.applySyncState(sampleState);
    expect(store.conversationCount).toBe(42);
    expect(store.characterSelection).toBe('chosen.vrm');
    expect(store.agentCount).toBe(3);
    expect(store.lastSyncedAt).toBe(1700000000);
    expect(store.isSynced).toBe(true);
    expect(store.error).toBeNull();
  });

  it('applySyncState clears previous error', () => {
    const store = useSyncStore();
    store.setError('something went wrong');
    expect(store.error).not.toBeNull();
    store.applySyncState(sampleState);
    expect(store.error).toBeNull();
  });

  it('setError sets the error field', () => {
    const store = useSyncStore();
    store.setError('merge conflict');
    expect(store.error).toBe('merge conflict');
  });

  it('clearError resets error to null', () => {
    const store = useSyncStore();
    store.setError('oops');
    store.clearError();
    expect(store.error).toBeNull();
  });

  it('reset returns all fields to initial values', () => {
    const store = useSyncStore();
    store.applySyncState(sampleState);
    store.reset();
    expect(store.conversationCount).toBe(0);
    expect(store.characterSelection).toBeNull();
    expect(store.agentCount).toBe(0);
    expect(store.lastSyncedAt).toBeNull();
    expect(store.error).toBeNull();
    expect(store.isSynced).toBe(false);
  });

  it('isSynced is true when lastSyncedAt is set', () => {
    const store = useSyncStore();
    expect(store.isSynced).toBe(false);
    store.applySyncState({ ...sampleState, last_synced_at: 999 });
    expect(store.isSynced).toBe(true);
  });

  it('isSynced is false when lastSyncedAt is null', () => {
    const store = useSyncStore();
    store.applySyncState({ ...sampleState, last_synced_at: null });
    expect(store.isSynced).toBe(false);
  });
});
