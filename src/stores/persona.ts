/**
 * Persona Pinia store.
 *
 * Owns the single active {@link PersonaTraits} record + the libraries of
 * camera-learned expressions and motions. Persisted via Tauri commands
 * (see `src-tauri/src/commands/persona.rs`) with a localStorage fallback
 * so the persona-prompt block keeps working in browser/dev/E2E contexts
 * where Tauri is unavailable.
 *
 * **Privacy note (per `docs/persona-design.md` § 5):** this store
 * intentionally has NO persistent `cameraEnabled` flag. The only
 * camera-related state lives in `cameraSession` which is in-memory only,
 * scoped to the current chat, and reset on chat change / reload / restart.
 */

import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import {
  defaultPersona,
  migratePersonaTraits,
  PERSONA_SCHEMA_VERSION,
  type PersonaTraits,
  type LearnedExpression,
  type LearnedMotion,
} from './persona-types';
import { buildPersonaBlock, type LearnedMotionRef } from '../utils/persona-prompt';

const TRAITS_STORAGE_KEY = 'terransoul.persona.traits.v1';

/**
 * Ephemeral per-session camera state. Never persisted, never serialised
 * to Tauri. Reset by `stopCameraSession()` on chat change, idle timeout,
 * window blur, etc.
 */
export interface CameraSessionState {
  /** True iff a `MediaStream` is currently open in this chat session. */
  active: boolean;
  /** ms epoch when the user clicked "Allow this session". */
  startedAt: number | null;
  /** The chat / conversation id this consent was granted for. */
  chatId: string | null;
}

function freshSession(): CameraSessionState {
  return { active: false, startedAt: null, chatId: null };
}

export const usePersonaStore = defineStore('persona', () => {
  // ── Persistent state ────────────────────────────────────────────────
  const traits = ref<PersonaTraits>(defaultPersona());
  const traitsLoaded = ref(false);
  const learnedExpressions = ref<LearnedExpression[]>([]);
  const learnedMotions = ref<LearnedMotion[]>([]);
  /** ms epoch of the last successful brain-extracted persona suggestion. */
  const lastBrainExtractedAt = ref<number | null>(null);

  // ── Ephemeral, session-only state (NOT persisted, see § 5) ──────────
  const cameraSession = ref<CameraSessionState>(freshSession());

  // ── Computed ────────────────────────────────────────────────────────

  /**
   * Personalised motion triggers extracted from the learned library — fed
   * into the persona prompt block so the brain can pick them on its own.
   */
  const learnedMotionRefs = computed<LearnedMotionRef[]>(() => [
    ...learnedExpressions.value.map((e) => ({ name: e.name, trigger: e.trigger })),
    ...learnedMotions.value.map((m) => ({ name: m.name, trigger: m.trigger })),
  ]);

  /**
   * The fully-rendered `[PERSONA]` block ready to splice into the system
   * prompt. Empty string when persona is inactive — same contract as the
   * `[LONG-TERM MEMORY]` block (see brain-advanced-design.md § 4).
   */
  const personaBlock = computed(() => buildPersonaBlock(traits.value, learnedMotionRefs.value));

  /**
   * True iff the user has customised the default persona — used by the
   * skill-tree `my-persona` quest activation rule.
   */
  const hasCustomPersona = computed(() => {
    const t = traits.value;
    if (!t.active) return false;
    const def = defaultPersona();
    return (
      t.name.trim() !== def.name ||
      t.role.trim() !== def.role ||
      t.bio.trim() !== def.bio ||
      t.tone.length !== def.tone.length ||
      t.tone.some((x, i) => x !== def.tone[i]) ||
      t.quirks.length > 0
    );
  });

  // ── Loaders / persisters ────────────────────────────────────────────

  function loadFromLocalStorage(): PersonaTraits | null {
    try {
      const raw = typeof localStorage !== 'undefined' ? localStorage.getItem(TRAITS_STORAGE_KEY) : null;
      if (!raw) return null;
      return migratePersonaTraits(JSON.parse(raw));
    } catch {
      return null;
    }
  }

  function saveToLocalStorage(t: PersonaTraits): void {
    try {
      if (typeof localStorage !== 'undefined') {
        localStorage.setItem(TRAITS_STORAGE_KEY, JSON.stringify(t));
      }
    } catch {
      // Quota / private mode — non-critical.
    }
  }

  /** Push the rendered persona block to the Rust side so server-driven
   *  streaming paths can splice it into the system prompt. Best-effort
   *  — silently ignored when Tauri isn't available. */
  async function syncBlockToBackend(): Promise<void> {
    try {
      await invoke('set_persona_block', { block: personaBlock.value });
    } catch (err) {
      // Browser-only or backend not yet ready — fine, the browser path
      // assembles the block itself from `personaBlock.value`. Logged at
      // debug level so Tauri-side troubleshooting is still possible.
      if (typeof console !== 'undefined' && typeof console.debug === 'function') {
        console.debug('[persona] set_persona_block unavailable:', err);
      }
    }
  }

  /** Load persona state from disk on startup. Tauri first, then local
   *  storage as fallback / override-merge. */
  async function load(): Promise<void> {
    let backendTraits: PersonaTraits | null = null;
    try {
      const raw = await invoke<string | null>('get_persona');
      if (typeof raw === 'string' && raw.length > 0) {
        backendTraits = migratePersonaTraits(JSON.parse(raw));
      }
    } catch {
      // Tauri unavailable — fall back to localStorage only.
    }

    const localTraits = loadFromLocalStorage();
    traits.value = backendTraits ?? localTraits ?? defaultPersona();
    saveToLocalStorage(traits.value);

    try {
      const resp = await invoke<LearnedExpression[] | null>('list_learned_expressions');
      learnedExpressions.value = Array.isArray(resp) ? resp : [];
    } catch {
      learnedExpressions.value = [];
    }
    try {
      const resp = await invoke<LearnedMotion[] | null>('list_learned_motions');
      learnedMotions.value = Array.isArray(resp) ? resp : [];
    } catch {
      learnedMotions.value = [];
    }

    traitsLoaded.value = true;
    await syncBlockToBackend();
  }

  /** Persist the current traits to disk (Tauri + localStorage), then
   *  push the rendered block to the backend so the next chat turn picks
   *  it up. */
  async function saveTraits(next: Partial<PersonaTraits>): Promise<void> {
    traits.value = {
      ...traits.value,
      ...next,
      version: PERSONA_SCHEMA_VERSION,
      updatedAt: Date.now(),
    };
    saveToLocalStorage(traits.value);
    try {
      await invoke('save_persona', { json: JSON.stringify(traits.value) });
    } catch {
      // Browser-only — localStorage already has it.
    }
    await syncBlockToBackend();
  }

  /** Reset the persona to its cold-start defaults. */
  async function resetToDefault(): Promise<void> {
    await saveTraits(defaultPersona());
  }

  /** Mark that the brain just produced a persona suggestion. */
  function recordBrainExtraction(): void {
    lastBrainExtractedAt.value = Date.now();
  }

  /**
   * Ask the active brain (via `extract_persona_from_brain`) to propose
   * a persona based on recent chat history + long-term `personal:*`
   * memories. Returns `null` when no brain is configured, the brain
   * could not be reached, or the reply could not be parsed — caller is
   * responsible for surfacing a "try again" message.
   *
   * **Does not auto-apply.** The caller wires the candidate into the
   * draft state of `PersonaPanel.vue` and the user clicks Apply (which
   * routes through the existing `saveTraits` flow). This matches the
   * human-in-the-loop contract documented in `docs/persona-design.md`
   * § 9.3.
   */
  async function suggestPersonaFromBrain(): Promise<Partial<PersonaTraits> | null> {
    let raw: string;
    try {
      raw = await invoke<string>('extract_persona_from_brain');
    } catch {
      // No brain configured / Tauri unavailable — UI surfaces this as
      // a disabled-button tooltip; nothing to do here.
      return null;
    }
    if (typeof raw !== 'string' || raw.trim().length === 0) {
      // Brain reachable but reply not parseable.
      return null;
    }
    let parsed: unknown;
    try {
      parsed = JSON.parse(raw);
    } catch {
      return null;
    }
    if (!parsed || typeof parsed !== 'object') return null;
    const p = parsed as Record<string, unknown>;
    const name = typeof p.name === 'string' ? p.name : '';
    const role = typeof p.role === 'string' ? p.role : '';
    const bio = typeof p.bio === 'string' ? p.bio : '';
    if (!name.trim() || !role.trim() || !bio.trim()) return null;
    const candidate: Partial<PersonaTraits> = {
      name,
      role,
      bio,
      tone: Array.isArray(p.tone) ? p.tone.filter((x): x is string => typeof x === 'string') : [],
      quirks: Array.isArray(p.quirks)
        ? p.quirks.filter((x): x is string => typeof x === 'string')
        : [],
      avoid: Array.isArray(p.avoid)
        ? p.avoid.filter((x): x is string => typeof x === 'string')
        : [],
    };
    recordBrainExtraction();
    return candidate;
  }

  // ── Per-session camera consent (§ 5) ────────────────────────────────

  /**
   * Record that the user just clicked "Allow this session" in the consent
   * dialog. Caller is responsible for actually opening the MediaStream.
   * Throws if a session is already active for a different chat.
   */
  function startCameraSession(chatId: string): void {
    if (cameraSession.value.active && cameraSession.value.chatId !== chatId) {
      // Hard guarantee from § 5: switching chat tears down any prior session.
      stopCameraSession();
    }
    cameraSession.value = {
      active: true,
      startedAt: Date.now(),
      chatId,
    };
  }

  /** Tear down the per-session camera consent. Idempotent. */
  function stopCameraSession(): void {
    cameraSession.value = freshSession();
  }

  return {
    // state
    traits,
    traitsLoaded,
    learnedExpressions,
    learnedMotions,
    lastBrainExtractedAt,
    cameraSession,
    // computed
    personaBlock,
    learnedMotionRefs,
    hasCustomPersona,
    // actions
    load,
    saveTraits,
    resetToDefault,
    recordBrainExtraction,
    suggestPersonaFromBrain,
    startCameraSession,
    stopCameraSession,
  };
});
