import { describe, it, expect, beforeEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import {
  useAiDecisionPolicyStore,
  DEFAULT_AI_DECISION_POLICY,
  type AiDecisionPolicy,
} from './ai-decision-policy';

const STORAGE_KEY = 'terransoul.ai-decision-policy.v1';

beforeEach(() => {
  localStorage.clear();
  setActivePinia(createPinia());
});

describe('ai-decision-policy store', () => {
  it('starts from documented defaults when localStorage is empty', () => {
    const store = useAiDecisionPolicyStore();
    expect(store.policy).toEqual(DEFAULT_AI_DECISION_POLICY);
    // Sanity: every default is true so historical UX is preserved.
    for (const key of Object.keys(DEFAULT_AI_DECISION_POLICY) as Array<keyof AiDecisionPolicy>) {
      expect(store.policy[key]).toBe(true);
    }
    // Lock in the full schema so accidental field renames break this test.
    expect(Object.keys(DEFAULT_AI_DECISION_POLICY).sort()).toEqual([
      'capacityDetectionEnabled',
      'chatBasedLlmSwitchEnabled',
      'dontKnowGateEnabled',
      'intentClassifierEnabled',
      'questSuggestionsEnabled',
      'quickRepliesEnabled',
      'unknownFallbackToInstall',
    ]);
  });

  it('persists toggles to localStorage on the next tick', async () => {
    const store = useAiDecisionPolicyStore();
    store.policy.intentClassifierEnabled = false;
    store.policy.dontKnowGateEnabled = false;
    await new Promise((r) => setTimeout(r, 0));
    const raw = localStorage.getItem(STORAGE_KEY);
    expect(raw).toBeTruthy();
    const parsed = JSON.parse(raw!);
    expect(parsed.intentClassifierEnabled).toBe(false);
    expect(parsed.dontKnowGateEnabled).toBe(false);
    expect(parsed.questSuggestionsEnabled).toBe(true);
  });

  it('rehydrates persisted toggles on store re-creation', () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ intentClassifierEnabled: false, questSuggestionsEnabled: false }),
    );
    setActivePinia(createPinia());
    const store = useAiDecisionPolicyStore();
    expect(store.policy.intentClassifierEnabled).toBe(false);
    expect(store.policy.questSuggestionsEnabled).toBe(false);
    // Unspecified keys retain their defaults.
    expect(store.policy.dontKnowGateEnabled).toBe(true);
    expect(store.policy.chatBasedLlmSwitchEnabled).toBe(true);
  });

  it('falls back to defaults on corrupt JSON', () => {
    localStorage.setItem(STORAGE_KEY, '{ this is not json');
    setActivePinia(createPinia());
    const store = useAiDecisionPolicyStore();
    expect(store.policy).toEqual(DEFAULT_AI_DECISION_POLICY);
  });

  it('drops fields with non-boolean values from persisted state', () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        intentClassifierEnabled: false,
        // attacker / corrupt write — non-boolean
        dontKnowGateEnabled: 'yes please',
        unknownFallbackToInstall: 1,
      }),
    );
    setActivePinia(createPinia());
    const store = useAiDecisionPolicyStore();
    expect(store.policy.intentClassifierEnabled).toBe(false);
    // Both invalid fields revert to default true.
    expect(store.policy.dontKnowGateEnabled).toBe(true);
    expect(store.policy.unknownFallbackToInstall).toBe(true);
  });

  it('reset() restores every field to its default', async () => {
    const store = useAiDecisionPolicyStore();
    for (const key of Object.keys(DEFAULT_AI_DECISION_POLICY) as Array<keyof AiDecisionPolicy>) {
      store.policy[key] = false;
    }
    store.reset();
    expect(store.policy).toEqual(DEFAULT_AI_DECISION_POLICY);
    await new Promise((r) => setTimeout(r, 0));
    const raw = localStorage.getItem(STORAGE_KEY);
    expect(JSON.parse(raw!)).toEqual(DEFAULT_AI_DECISION_POLICY);
  });
});
