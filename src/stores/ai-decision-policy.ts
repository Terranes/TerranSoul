/**
 * AI Decision-Making Policy
 *
 * TerranSoul has several places where the system makes routing decisions on
 * the user's behalf — most notably the LLM-powered intent classifier, the
 * "I don't know" gate that offers Gemini-search / context upload, the
 * post-response quest auto-suggestion, and the chat-based LLM-switching
 * commands. These are all *opinionated* defaults, but power users want to
 * disable them for a strictly-pass-through chat experience.
 *
 * This store is the single source of truth for those toggles. It is
 * intentionally **frontend-only** and persisted via `localStorage` so:
 *
 *   1. Toggling a setting takes effect on the very next message — no Tauri
 *      round-trip, no database migration.
 *   2. The Rust backend stays free of UI-only knobs.
 *   3. Settings survive across app restarts.
 *
 * UI for these toggles lives in `src/views/BrainView.vue` under the
 * "🧭 AI Decision-Making" section.
 *
 * Wiring is in `src/stores/conversation.ts` — every gated path early-returns
 * when its corresponding policy field is `false`.
 */
import { defineStore } from 'pinia';
import { reactive, watch } from 'vue';

/**
 * The full set of user-controllable AI decision toggles.
 *
 * Each field defaults to `true` to preserve the existing opinionated UX.
 * Setting any field to `false` disables exactly one decision surface.
 */
export interface AiDecisionPolicy {
  /**
   * Run every chat turn through the LLM intent classifier
   * (`brain::intent_classifier::classify_user_intent`) before streaming.
   *
   * When `false`, the IPC call is skipped entirely and every message goes
   * straight to the streaming chat path. Side-channel intents
   * (learn-with-docs, teach-ingest, gated-setup) will no longer auto-trigger
   * — the user must launch them from the UI instead.
   */
  intentClassifierEnabled: boolean;
  /**
   * When the intent classifier returns `Unknown` (no brain reachable,
   * timeout, malformed JSON), trigger the Auto-Install-All overlay so the
   * user can install a local Ollama brain.
   *
   * When `false`, `Unknown` falls through to the normal streaming chat path
   * — useful for users who have already declined the local-install offer
   * and don't want to be re-prompted.
   */
  unknownFallbackToInstall: boolean;
  /**
   * After the assistant responds, scan the reply for hedging phrases
   * ("I don't know", "I'm not sure", …) and push a System message offering
   * the Gemini-with-search upgrade or a Scholar's Quest for context upload.
   *
   * When `false`, no don't-know prompt is ever shown — useful for users on
   * deliberately small local models who already understand the trade-off.
   */
  dontKnowGateEnabled: boolean;
  /**
   * After the assistant responds, opportunistically suggest the next
   * available quest when the reply or the user's input mentions
   * getting-started keywords.
   *
   * When `false`, quest suggestions are never auto-pushed from chat — the
   * user can still open the Skill Tree manually.
   */
  questSuggestionsEnabled: boolean;
  /**
   * Recognise chat commands like *"switch to groq"*, *"use my openai api
   * key sk-…"* and reconfigure the brain mode in-place.
   *
   * When `false`, those messages are treated as ordinary chat and reach the
   * LLM unchanged.
   */
  chatBasedLlmSwitchEnabled: boolean;
  /**
   * Show one-tap "Yes / No" quick-reply buttons under the latest assistant
   * message when its trailing sentence matches a yes/no question pattern
   * (`shall we…?`, `would you like…?`, `want me to…?`, etc.).
   *
   * This is a **regex-based heuristic** — it can't read the model's intent,
   * only its surface phrasing. Disable it for a strictly-typed chat where
   * the user always composes the full reply.
   */
  quickRepliesEnabled: boolean;
  /**
   * Watch each free-API assistant reply for incapability signals
   * (very short responses, "I can't / cannot / am only an AI / beyond my
   * capabilities" phrasings) via a 7-regex bank in `capacity-detector.ts`.
   * After a few low-quality replies in a sliding window, pop the in-chat
   * "Upgrade to a more capable model" dialog.
   *
   * When `false`, `assessCapacity()` is never consulted from the chat view
   * — useful for users on deliberately small / cheap models who don't want
   * to be nagged.
   */
  capacityDetectionEnabled: boolean;
}

const STORAGE_KEY = 'terransoul.ai-decision-policy.v1';

/**
 * Hard-coded defaults preserve the historical "opinionated by default" UX:
 * everything on. The Brain panel surfaces the toggles so users can opt out.
 */
export const DEFAULT_AI_DECISION_POLICY: AiDecisionPolicy = Object.freeze({
  intentClassifierEnabled: true,
  unknownFallbackToInstall: true,
  dontKnowGateEnabled: true,
  questSuggestionsEnabled: true,
  chatBasedLlmSwitchEnabled: true,
  quickRepliesEnabled: true,
  capacityDetectionEnabled: true,
});

/**
 * Read the policy from `localStorage`, merging any persisted partial object
 * over the defaults. Unknown keys are ignored; missing keys fall back to
 * the default. Corrupt JSON resets to defaults without throwing.
 */
function loadPolicy(): AiDecisionPolicy {
  if (typeof localStorage === 'undefined') {
    return { ...DEFAULT_AI_DECISION_POLICY };
  }
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULT_AI_DECISION_POLICY };
    const parsed = JSON.parse(raw) as Partial<AiDecisionPolicy>;
    return {
      ...DEFAULT_AI_DECISION_POLICY,
      ...sanitise(parsed),
    };
  } catch {
    return { ...DEFAULT_AI_DECISION_POLICY };
  }
}

/** Drop any field whose value isn't a boolean. */
function sanitise(input: Partial<AiDecisionPolicy>): Partial<AiDecisionPolicy> {
  const out: Partial<AiDecisionPolicy> = {};
  for (const key of Object.keys(DEFAULT_AI_DECISION_POLICY) as Array<keyof AiDecisionPolicy>) {
    const v = input[key];
    if (typeof v === 'boolean') out[key] = v;
  }
  return out;
}

/**
 * Pinia store exposing the reactive policy plus a `reset()` helper.
 *
 * Reads on construction, writes on every mutation via a deep `watch`. Tests
 * can call `reset()` in `beforeEach` to start from the documented defaults.
 */
export const useAiDecisionPolicyStore = defineStore('ai-decision-policy', () => {
  const policy = reactive<AiDecisionPolicy>(loadPolicy());

  // Persist on every change. Errors (quota exceeded, private mode) are
  // swallowed because the policy is purely UX-affecting and a write failure
  // shouldn't break the chat flow.
  watch(
    () => ({ ...policy }),
    (next) => {
      if (typeof localStorage === 'undefined') return;
      try {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
      } catch {
        // Ignore — see comment above.
      }
    },
    { deep: true },
  );

  function reset(): void {
    Object.assign(policy, DEFAULT_AI_DECISION_POLICY);
  }

  return { policy, reset };
});
