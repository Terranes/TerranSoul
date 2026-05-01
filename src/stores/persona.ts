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

  /**
   * Preview requests — set by PersonaPanel, consumed by CharacterViewport.
   * Cross-view bridge since the two live in different routes.
   * Cleared by the consumer after handling.
   */
  const previewExpressionRequest = ref<LearnedExpression | null>(null);
  const previewMotionRequest = ref<LearnedMotion | null>(null);

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

  /**
   * Ask the active brain to generate a multi-frame motion clip from a
   * short text description (Chunk 14.16c2 — LLM-as-Animator). Returns
   * a parsed-but-not-saved [`LearnedMotion`] candidate plus parser
   * diagnostics so the UI can show "we cleaned N frames" hints.
   *
   * **Does not auto-save.** The caller previews the candidate (e.g.
   * via `requestMotionPreview`) and only commits via `saveLearnedMotion`
   * after the user clicks Accept — same human-in-the-loop contract as
   * `suggestPersonaFromBrain`.
   *
   * Returns `null` when no brain is configured / Tauri is unavailable.
   * Throws when the brain reply could not be parsed; caller surfaces
   * the message in a "couldn't generate, try again" toast.
   */
  async function generateMotionFromText(
    description: string,
    opts?: { durationS?: number; fps?: number },
  ): Promise<{
    motion: LearnedMotion;
    diagnostics: {
      droppedFrames: number;
      clampedComponents: number;
      repairedTimestamps: boolean;
      repairedDuration: boolean;
    };
  } | null> {
    interface BackendEnvelope {
      motion_json: string;
      trigger: string;
      dropped_frames: number;
      clamped_components: number;
      repaired_timestamps: boolean;
      repaired_duration: boolean;
    }
    let envelope: BackendEnvelope;
    try {
      envelope = await invoke<BackendEnvelope>('generate_motion_from_text', {
        description,
        durationS: opts?.durationS ?? null,
        fps: opts?.fps ?? null,
      });
    } catch (e) {
      // Surface configuration / network failures as throws so the UI
      // toast knows what to say. Returning null is reserved for the
      // "no brain configured" case which the underlying command
      // already returns as an Err — re-throw uniformly.
      throw new Error(typeof e === 'string' ? e : String(e));
    }
    let motion: LearnedMotion;
    try {
      motion = JSON.parse(envelope.motion_json) as LearnedMotion;
    } catch (e) {
      throw new Error(`Generated motion JSON was unreadable: ${String(e)}`);
    }
    return {
      motion,
      diagnostics: {
        droppedFrames: envelope.dropped_frames,
        clampedComponents: envelope.clamped_components,
        repairedTimestamps: envelope.repaired_timestamps,
        repairedDuration: envelope.repaired_duration,
      },
    };
  }

  /**
   * Persist a [`LearnedMotion`] (typically one returned by
   * [`generateMotionFromText`] after the user clicked Accept) to disk
   * and append it to the in-memory library. Mirrors the
   * `saveLearnedExpression` / `saveLearnedMotion` flow used by
   * `PersonaTeacher.vue` so callers don't need to duplicate the
   * `invoke('save_learned_motion', ...)` plumbing.
   */
  async function saveLearnedMotion(motion: LearnedMotion): Promise<void> {
    await invoke('save_learned_motion', { json: JSON.stringify(motion) });
    learnedMotions.value = [
      motion,
      ...learnedMotions.value.filter((m) => m.id !== motion.id),
    ];
  }

  /**
   * Record an accept/reject verdict on a generated motion clip
   * (Chunk 14.16e — self-improve loop). The backend appends a JSONL
   * entry to `<persona-root>/motion_feedback.jsonl` and the next
   * `generateMotionFromText` call uses the trusted-trigger leaderboard
   * to nudge the brain toward the user's preferred vocabulary.
   *
   * Best-effort: failures are logged but never thrown — the user
   * shouldn't see a save error just because the feedback couldn't be
   * persisted.
   */
  async function recordMotionFeedback(args: {
    description: string;
    trigger: string;
    verdict: 'accepted' | 'rejected';
    durationS?: number;
    fps?: number;
    droppedFrames?: number;
    clampedComponents?: number;
  }): Promise<void> {
    try {
      await invoke('record_motion_feedback', {
        payload: {
          description: args.description,
          trigger: args.trigger,
          verdict: args.verdict,
          duration_s: args.durationS ?? 0,
          fps: args.fps ?? 0,
          dropped_frames: args.droppedFrames ?? 0,
          clamped_components: args.clampedComponents ?? 0,
        },
      });
    } catch (e) {
      if (typeof console !== 'undefined' && typeof console.debug === 'function') {
        console.debug('[persona] record_motion_feedback failed:', e);
      }
    }
  }

  /** Frontend mirror of `crate::persona::motion_feedback::TrustedTrigger`. */
  interface TrustedTrigger {
    trigger: string;
    accepted: number;
    rejected: number;
  }

  /** Frontend mirror of `crate::persona::motion_feedback::MotionFeedbackStats`. */
  interface MotionFeedbackStats {
    total: number;
    accepted: number;
    rejected: number;
    trusted_triggers: TrustedTrigger[];
    discouraged_descriptions: string[];
  }

  /**
   * Read the motion-feedback aggregate stats. Returns `null` when
   * Tauri is unavailable (browser-only test harness) so the UI can
   * hide the leaderboard gracefully.
   */
  async function fetchMotionFeedbackStats(): Promise<MotionFeedbackStats | null> {
    try {
      return await invoke<MotionFeedbackStats>('get_motion_feedback_stats');
    } catch {
      return null;
    }
  }

  // ── Persona pack export / import (Chunk 14.7) ──────────────────────

  /**
   * Shape of the per-import / per-preview report returned by the
   * backend. Mirrors `crate::persona::pack::ImportReport`.
   */
  interface ImportReport {
    traits_applied: boolean;
    expressions_accepted: number;
    motions_accepted: number;
    motions_generated: number;
    motions_camera: number;
    skipped: string[];
  }

  /**
   * Build a self-describing JSON pack containing the active traits + every
   * learned expression / motion artifact and return the pretty-printed
   * string. Caller is responsible for surfacing the result (clipboard,
   * file download, share sheet…).
   *
   * `note` is an optional one-liner shown in the import preview on the
   * receiving side. Empty / whitespace-only is dropped.
   *
   * Returns `null` when Tauri is unavailable (browser-only / offline test
   * harness) so the UI can disable the button gracefully.
   */
  async function exportPack(note?: string): Promise<string | null> {
    try {
      const json = await invoke<string>('export_persona_pack', { note: note ?? null });
      return typeof json === 'string' && json.length > 0 ? json : null;
    } catch (e) {
      console.warn('[persona] export pack failed:', e);
      return null;
    }
  }

  /**
   * Dry-run a persona pack import: parse + validate every asset and
   * return the per-entry report **without writing anything**. Used by
   * the "Preview" button so the user can see what would change before
   * committing.
   *
   * Throws an Error with the human-readable parse failure when the
   * input is not a valid pack — callers route that into the inline
   * error message in the import card.
   */
  async function previewImportPack(json: string): Promise<ImportReport> {
    return await invoke<ImportReport>('preview_persona_pack', { json });
  }

  /**
   * Apply a persona pack: replace the active traits and merge the
   * learned-asset libraries (matching ids overwrite; others are kept).
   * After a successful import the in-memory store reloads from disk so
   * UI bindings reflect the new state.
   *
   * Throws on parse failure (matches `previewImportPack`).
   */
  async function importPack(json: string): Promise<ImportReport> {
    const report = await invoke<ImportReport>('import_persona_pack', { json });
    // Reload everything so UI reflects the merged state.
    await load();
    return report;
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

  /** Request the avatar preview an expression (consumed by CharacterViewport). */
  function requestExpressionPreview(expr: LearnedExpression): void {
    previewExpressionRequest.value = expr;
  }

  /** Request the avatar play a learned motion (consumed by CharacterViewport). */
  function requestMotionPreview(motion: LearnedMotion): void {
    previewMotionRequest.value = motion;
  }

  return {
    // state
    traits,
    traitsLoaded,
    learnedExpressions,
    learnedMotions,
    lastBrainExtractedAt,
    cameraSession,
    previewExpressionRequest,
    previewMotionRequest,
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
    generateMotionFromText,
    saveLearnedMotion,
    recordMotionFeedback,
    fetchMotionFeedbackStats,
    exportPack,
    previewImportPack,
    importPack,
    startCameraSession,
    stopCameraSession,
    requestExpressionPreview,
    requestMotionPreview,
  };
});
