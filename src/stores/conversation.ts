import { defineStore } from 'pinia';
import { ref, computed, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { Message } from '../types';
import { useBrainStore } from './brain';
import { useStreamingStore } from './streaming';
import { useProviderHealthStore } from './provider-health';
import { useSkillTreeStore } from './skill-tree';
import { useTaskStore } from './tasks';
import { usePersonaStore } from './persona';
import { useCharismaStore } from './charisma';
import { useMemoryStore } from './memory';
import type { DriftReport } from './persona-types';
import { streamChatCompletion, buildHistory, getSystemPrompt } from '../utils/free-api-client';
import { parseTags } from '../utils/emotion-parser';
import { collectCharismaTurnAssets } from '../utils/charisma-turn-assets';
import { useAiDecisionPolicyStore } from './ai-decision-policy';
import { useAgentRosterStore } from './agent-roster';
import { buildHandoffBlock } from '../utils/handoff-prompt';
import { browserDirectFallbackProviders, resolveBrowserBrainTransport } from '../transport/browser-brain';
import { normaliseTranslatorLanguage, type TranslatorLanguage } from '../utils/translator-languages';

// ── LLM-powered intent classifier (Rust: `brain::intent_classifier`) ──
// Mirrors the wire format emitted by the `classify_intent` Tauri command.
// Mirrors the wire format emitted by the `classify_intent` Tauri command.
// Contentful side-channel routing is owned by the backend brain classifier
// for every brain mode; the frontend only handles the returned decision.
export type GatedSetupKind = 'upgrade_gemini' | 'provide_context';
export type IntentDecision =
  | { kind: 'chat' }
  | { kind: 'learn_with_docs'; topic: string }
  | { kind: 'teach_ingest'; topic: string }
  | { kind: 'gated_setup'; setup: GatedSetupKind }
  | { kind: 'unknown' };

const DOCUMENT_LEARNING_EXACT_PHRASES = [
  'learn from my documents',
  'learn my documents',
  'learn documents',
  'learn from my docs',
  'learn from my files',
  'learn from my notes',
  'learn using my documents',
  'learn with my documents',
  'study my documents',
] as const;

/**
 * High-confidence side-channel phrases should wait for the classifier before
 * streaming starts. Otherwise the user can hear a normal LLM answer while the
 * visible UI is already switching to Scholar's Quest.
 */
export function shouldAwaitIntentBeforeStreaming(text: string): boolean {
  const normalized = text.trim().toLowerCase().replace(/\s+/g, ' ');
  if (!normalized) return false;
  if (DOCUMENT_LEARNING_EXACT_PHRASES.some((phrase) => normalized === phrase)) {
    return true;
  }
  if (
    normalized.includes('my provided documents') &&
    /\b(learn|study)\b/.test(normalized)
  ) {
    return true;
  }
  return /\b(learn|study)\b/.test(normalized) &&
    /\b(my|provided|own)\b.*\b(documents|docs|files|notes|pdfs|sources)\b/.test(normalized);
}

interface TranslatorModeState {
  active: boolean;
  source: TranslatorLanguage;
  target: TranslatorLanguage;
  nextDirection: 'source_to_target' | 'target_to_source';
}

export function detectTranslatorModeRequest(userInput: string): { source: TranslatorLanguage; target: TranslatorLanguage } | null {
  const compact = userInput.trim().replace(/\s+/g, ' ');
  const between = compact.match(/(?:translate|translator).*?between\s+(.+?)\s+and\s+(.+?)(?:[.?!]|$)/i);
  const directional = compact.match(/(?:translate|translator).*?from\s+(.+?)\s+to\s+(.+?)(?:[.?!]|$)/i);
  const match = between ?? directional;
  if (!match) return null;
  const source = normaliseTranslatorLanguage(match[1] ?? '');
  const target = normaliseTranslatorLanguage(match[2] ?? '');
  if (!source || !target || source.code === target.code) return null;
  return { source, target };
}

export function isStopTranslatorModeRequest(userInput: string): boolean {
  const lower = userInput.trim().toLowerCase();
  return lower === 'stop translator mode'
    || lower === 'exit translator mode'
    || lower === 'disable translator mode'
    || lower === 'turn off translator mode';
}


/**
 * Default sentiment when the LLM does not emit an `<anim>` emotion tag.
 * The LLM decides emotion via `<anim>{"emotion":"..."}` in the stream;
 * this function is only the last-resort fallback and always returns neutral.
 */
export function detectSentiment(_text: string): 'happy' | 'sad' | 'angry' | 'relaxed' | 'surprised' | 'neutral' {
  return 'neutral';
}

/**
 * Returns true for short content-light chat turns that should stay on the
 * fastest possible path. These turns are too small to benefit from RAG or an
 * LLM intent-classifier pass, and running either can contend with LocalLLM.
 */
export function shouldUseFastChatPath(text: string): boolean {
  const trimmed = text.trim();
  if (!trimmed) return true;
  const tokens = trimmed.split(/\s+/);
  return tokens.length <= 3 && tokens.every((token) => Array.from(token).length <= 5);
}

function shouldRunIntentClassifierForTurn(
  text: string,
  brain: ReturnType<typeof useBrainStore>,
): boolean {
  if (shouldUseFastChatPath(text)) return false;
  return brain.hasBrain || isTauriAvailable();
}

/**
 * Patterns that indicate a provider/service warning in the response text.
 * When detected, the warning is stripped and converted into a quest suggestion.
 */
const WARNING_PATTERNS = [
  /⚠️?\s*\*{0,2}IMPORTANT NOTICE\*{0,2}[\s\S]*?(?:deprecated|shutting down|end of life|discontinued)[\s\S]*?(?:\n\n|$)/i,
  /⚠️?\s*\*{0,2}(?:Deprecation|Service|Migration)\s*(?:Notice|Warning|Alert)\*{0,2}[:\s][\s\S]*?(?:\n\n|$)/i,
  /(?:^|\n)\[?(?:WARNING|NOTICE)\]?:?\s*(?:This|The)\s+(?:API|service|endpoint|model)[\s\S]*?(?:deprecated|removed|sunset|discontinued)[\s\S]*?(?:\n\n|$)/i,
];

/** Detect and extract a provider warning from response text. Returns cleaned text and the warning if found. */
function extractWarning(text: string): { clean: string; warning: string | null } {
  for (const pattern of WARNING_PATTERNS) {
    const match = text.match(pattern);
    if (match) {
      const warning = match[0].trim();
      const clean = text.replace(pattern, '').trim();
      return { clean, warning };
    }
  }
  return { clean: text, warning: null };
}

/** Convert a warning into quest choices attached to the message. */
function applyWarningAsQuest(msg: Message, _warning: string): void {
  msg.questId = 'migrate-brain';
  msg.questChoices = [
    { label: 'Upgrade to Paid API', value: 'navigate:brain-setup', icon: '⚡' },
    { label: 'Use llmfit (Local AI)', value: 'navigate:brain-setup', icon: '🏰' },
    { label: 'Switch Provider', value: 'navigate:marketplace', icon: '🔄' },
    { label: 'Dismiss', value: 'dismiss', icon: '💤' },
  ];
}

/** Browser-side persona fallback when no brain is configured at all.
 *  Without an LLM, no emotion inference is possible — always neutral. */
function createPersonaResponse(_content: string): Message {
  return {
    id: crypto.randomUUID(),
    role: 'assistant',
    content: 'Hello! I\'m TerranSoul. Please configure a brain (free cloud API or paid API) in the Marketplace so I can have a real conversation with you!',
    agentName: 'TerranSoul',
    sentiment: 'neutral',
    timestamp: Date.now(),
  };
}

/** Detect if the Tauri IPC bridge is available (synchronous check). */
function isTauriAvailable(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

/**
 * Ask the Rust backend to classify a user message into a structured
 * `IntentDecision` via the configured brain (Free → Paid → Local).
 *
 * Falls back to `{ kind: 'chat' }` when the Tauri backend is unavailable
 * (browser-only mode / unit tests that don't mock the command). This keeps
 * non-Tauri smoke tests working while still letting any test mock
 * `invoke('classify_intent', …)` to exercise specific decisions.
 *
 * Any thrown error from the IPC call is also mapped to `chat` so a
 * misbehaving classifier never blocks a user's turn — the live message
 * path always proceeds to the streaming LLM response when classification
 * doesn't decisively pick a side-channel intent.
 */
async function classifyIntent(text: string, hasBrain: boolean): Promise<IntentDecision> {
  // Respects the "Intent classifier" toggle in the Brain panel. When off,
  // skip the IPC entirely and treat every turn as plain chat — side-channel
  // intents (learn-with-docs, teach-ingest, gated-setup) won't auto-trigger.
  try {
    if (!useAiDecisionPolicyStore().policy.intentClassifierEnabled) {
      return { kind: 'chat' };
    }
  } catch {
    // Pinia not active — fall through to default-on behaviour.
  }
  // In the desktop app the Rust classifier owns deterministic shortcuts such
  // as "Learn from my documents" even before a brain provider is configured.
  // Browser-only/unit-test runtimes still keep the old persona fallback.
  if (!hasBrain && !isTauriAvailable()) return { kind: 'chat' };
  try {
    const decision = await invoke<IntentDecision>('classify_intent', { text });
    if (decision && typeof decision === 'object' && 'kind' in decision) {
      return decision;
    }
    return { kind: 'chat' };
  } catch {
    return { kind: 'chat' };
  }
}


/**
 * After the LLM responds, check if the user should be shown a quest overlay.
 *
 * Uses a hybrid approach:
 * 1. Check if the LLM response mentions quests (brain-driven detection)
 * 2. Check if the user's original message expressed getting-started intent
 *
 * If either signal is present AND there are available quests, show the overlay.
 * This avoids hard-coded regex that blocks the LLM, while still ensuring
 * quest suggestions appear reliably for getting-started queries.
 */
function maybeShowQuestFromResponse(responseText: string, userInput?: string): void {
  // Respects the "Auto-suggest quests after replies" toggle. When off, never
  // open a quest overlay from a chat response — the user can still launch
  // quests manually from the Skill Tree.
  try {
    if (!useAiDecisionPolicyStore().policy.questSuggestionsEnabled) return;
  } catch {
    // Pinia not active — fall through to default behaviour.
  }
  const responseLower = responseText.toLowerCase();
  const inputLower = (userInput ?? '').toLowerCase();

  // Capability questions like "what can you do?" should be answered naturally
  // by the LLM first. Only open a quest overlay after the user explicitly
  // signals that they like/want a specific capability or accepts a suggested
  // next action.
  if (!hasQuestOptInSignal(inputLower)) return;

  try {
    const skillTree = useSkillTreeStore();
    const preferredQuest = findPreferredQuestFromInput(skillTree, inputLower);

    // Response-side quest mentions are still useful context for generic
    // approvals like "that sounds good", but they no longer trigger quests by
    // themselves.
    const questWordRe = /\bquests?\b/i;
    const hasResponseSignal = questWordRe.test(responseText) || responseLower.includes('skill tree');

    if (!preferredQuest && !hasResponseSignal) return;

    const availableQuests = skillTree.nodes.filter(n => {
      try { return skillTree.getSkillStatus(n.id) === 'available'; }
      catch { return false; }
    });
    const questId = preferredQuest?.id ?? availableQuests[0]?.id;
    if (questId) {
      skillTree.triggerQuestEvent(questId);
    }
  } catch {
    // Skill tree not ready — skip quest overlay
  }
}

function hasQuestOptInSignal(inputLower: string): boolean {
  const normalized = inputLower.trim().replace(/\s+/g, ' ');
  if (!normalized) return false;
  return /\b(?:i\s+(?:like|love|prefer|choose|pick)|i\s+want\s+to\s+(?:try|use|start|open|enable)|i(?:'|’)d\s+like|i\s+would\s+like|i(?:'|’)m\s+interested\s+in|i\s+am\s+interested\s+in|that\s+sounds\s+good|sounds\s+good|looks\s+good|let(?:'|’)s\s+(?:do|try|start|begin|use)|can\s+we\s+(?:do|try|start|use)|please\s+(?:start|begin|show|open)|start\s+that|try\s+that|do\s+that|use\s+that)\b/i.test(normalized);
}

function findPreferredQuestFromInput(
  skillTree: ReturnType<typeof useSkillTreeStore>,
  inputLower: string,
) {
  const candidates = skillTree.nodes.filter((node) => {
    try {
      const status = skillTree.getSkillStatus(node.id);
      return status !== 'active' && !skillTree.tracker.dismissedQuestIds.includes(node.id);
    } catch {
      return false;
    }
  });

  let best: { id: string; score: number } | null = null;
  for (const node of candidates) {
    const aliases = [
      node.id.replace(/-/g, ' '),
      node.name,
      node.tagline,
      ...node.rewards,
    ].map((value) => value.toLowerCase());
    let score = 0;
    if (aliases.some((alias) => alias.length > 2 && inputLower.includes(alias))) {
      score += 6;
    }
    const tokens = new Set(
      [node.id, node.name, node.tagline, node.description, ...node.rewards]
        .join(' ')
        .toLowerCase()
        .split(/[^a-z0-9]+/)
        .filter((token) => token.length >= 4 && !QUEST_MATCH_STOP_WORDS.has(token)),
    );
    for (const token of tokens) {
      if (inputLower.includes(token)) score += 1;
    }
    if (score > (best?.score ?? 0)) best = { id: node.id, score };
  }
  return best && best.score >= 2
    ? skillTree.nodes.find((node) => node.id === best.id) ?? null
    : null;
}

const QUEST_MATCH_STOP_WORDS = new Set([
  'your',
  'with',
  'from',
  'this',
  'that',
  'mode',
  'quest',
  'skill',
  'tree',
  'companion',
  'terran',
  'soul',
]);

/**
 * Push the Scholar's Quest message for an explicit teach/ingest intent
 * with the topic already extracted (by the LLM intent classifier).
 */
function pushTeachScholarQuestForTopic(topic: string): void {
  const conversation = useConversationStore();
  conversation.messages.push({
    id: crypto.randomUUID(),
    role: 'assistant',
    content:
      `Got it — let's ingest **${topic}** into my long-term memory. ` +
      `I'll walk you through a **📚 Scholar's Quest** so we can add URLs and files ` +
      `and I can give you source-grounded answers afterwards.`,
    agentName: 'TerranSoul',
    sentiment: 'happy',
    timestamp: Date.now(),
    questId: 'scholar-quest',
    questChoices: [
      { label: 'Start Knowledge Quest', value: 'knowledge-quest-start', icon: '⚔️' },
      { label: 'No thanks', value: 'dismiss', icon: '💤' },
    ],
  });
}

/**
 * Phrases that indicate the assistant does not have a reliable answer.
 * When any of these appear in the LLM response, we offer the user two
 * gated upgrade paths (Gemini search, or provide-your-own-context).
 */
const DONT_KNOW_PATTERNS: RegExp[] = [
  /\bi\s+(?:do\s+not|don'?t)\s+(?:know|have\s+(?:access|information|enough|specific|detailed|data))\b/i,
  /\b(?:i\s+am\s+not|i'?m\s+not)\s+(?:sure|certain)\b/i,
  /\bi\s+(?:cannot|can'?t)\s+(?:answer|tell|say|confirm|verify)\b/i,
  /\bno\s+(?:specific\s+|reliable\s+|detailed\s+)?(?:information|data|record|details?)\b/i,
  /\bmy\s+(?:knowledge|training\s+data)\s+(?:is\s+limited|doesn'?t|does\s+not|may\s+be\s+out\s+of\s+date)\b/i,
  /\b(?:beyond|outside)\s+my\s+(?:knowledge|training)\b/i,
  /\bnot\s+(?:up-?to-?date|current)\s+(?:on|with|about)\b/i,
];

export function detectDontKnow(responseText: string): boolean {
  return DONT_KNOW_PATTERNS.some((p) => p.test(responseText));
}

/**
 * After an LLM response, if the model signalled uncertainty, push a System
 * message offering the two gated upgrade paths.  The user must TYPE one of
 * the commands (or click the corresponding button) to actually kick off the
 * setup — we never mutate brain mode or open a quest silently.
 */
function maybeShowDontKnowPrompt(responseText: string): void {
  // Respects the "Don't-know gate" toggle in the Brain panel. When the user
  // has disabled this opinionated prompt, never push it.
  try {
    if (!useAiDecisionPolicyStore().policy.dontKnowGateEnabled) return;
  } catch {
    // Pinia not active (e.g. legacy unit tests) — fall through to default.
  }
  if (!detectDontKnow(responseText)) return;

  const conversation = useConversationStore();
  // Avoid duplicate prompts if the last System message is already one.
  const recent = conversation.messages.slice(-3);
  if (recent.some((m) => m.questId === 'dont-know')) return;

  conversation.messages.push({
    id: crypto.randomUUID(),
    role: 'assistant',
    content:
      `I don't have reliable information on that — my current model's knowledge is limited here.\n\n` +
      `Two ways forward:\n` +
      `• Type **"upgrade to Gemini model"** — I'll switch to Google Gemini with Search grounding (API key needed).\n` +
      `• Type **"provide your own context"** — I'll start a **📚 Scholar's Quest** so you can feed me URLs / files.`,
    agentName: 'System',
    sentiment: 'neutral',
    timestamp: Date.now(),
    questId: 'dont-know',
    questChoices: [
      { label: 'Upgrade to Gemini model', value: 'command:upgrade to Gemini model', icon: '🔮' },
      { label: 'Provide your own context', value: 'command:provide your own context', icon: '📚' },
      { label: 'Dismiss', value: 'dismiss', icon: '💤' },
    ],
  });
}

/**
 * Execute a gated setup command.  Returns the assistant message to push into
 * the conversation; the message carries quest choices that wire up into the
 * existing overlay so the user can proceed with a single click.
 *
 * The shape `{ type: 'upgrade_gemini' | 'provide_context' }` mirrors the
 * `IntentDecision::GatedSetup` branch returned by the backend classifier.
 */
function executeGatedSetupCommand(
  cmd: { type: 'upgrade_gemini' } | { type: 'provide_context' },
): Message {
  if (cmd.type === 'upgrade_gemini') {
    return {
      id: crypto.randomUUID(),
      role: 'assistant',
      content:
        `🔮 **Upgrading to Google Gemini.**\n\n` +
        `Gemini 2.0 Flash supports Google Search grounding, so it can answer questions ` +
        `that need fresh, web-sourced information.  It needs a free API key from ` +
        `[Google AI Studio](https://aistudio.google.com/apikey).\n\n` +
        `I'll take you to the Marketplace → **Configure LLM** so you can paste the key.`,
      agentName: 'TerranSoul',
      sentiment: 'happy',
      timestamp: Date.now(),
      questId: 'upgrade-gemini',
      questChoices: [
        { label: 'Open Marketplace', value: 'navigate:marketplace', icon: '🏪' },
        { label: 'Not now', value: 'dismiss', icon: '💤' },
      ],
    };
  }

  // provide_context
  return {
    id: crypto.randomUUID(),
    role: 'assistant',
    content:
      `📚 Let's set up your own context.  I'll run a **Scholar's Quest** that walks you ` +
      `through adding URLs and files — I'll chunk, embed and store them in long-term memory ` +
      `so my next answers are grounded in your sources.`,
    agentName: 'TerranSoul',
    sentiment: 'happy',
    timestamp: Date.now(),
    questId: 'scholar-quest',
    questChoices: [
      { label: 'Start Knowledge Quest', value: 'knowledge-quest-start', icon: '⚔️' },
      { label: 'No thanks', value: 'dismiss', icon: '💤' },
    ],
  };
}

// ── "Learn X using my documents" flow ────────────────────────────────────────

/**
 * Walk the prerequisite chain of `targetQuestId` and return the ordered list
 * of quest IDs that aren't yet `active`. The target quest itself is included
 * at the end of the list when it isn't active either.
 *
 * The order is dependency-first: every prerequisite appears before the quest
 * that depends on it, so the auto-install loop can simply iterate left → right.
 */
export function getMissingPrereqQuests(
  skillTree: ReturnType<typeof useSkillTreeStore>,
  targetQuestId: string,
): string[] {
  const ordered: string[] = [];
  const seen = new Set<string>();

  function visit(id: string): void {
    if (seen.has(id)) return;
    seen.add(id);
    const node = skillTree.nodes.find((n) => n.id === id);
    if (!node) return;
    for (const req of node.requires) visit(req);
    let status: string;
    try {
      status = skillTree.getSkillStatus(id);
    } catch (err) {
      // Defensive: treat status-check failures as "not active yet" so the
      // quest is included in the install list rather than silently skipped.
      console.warn(`getSkillStatus(${id}) failed; assuming not active`, err);
      status = 'available';
    }
    if (status !== 'active') ordered.push(id);
  }

  visit(targetQuestId);
  return ordered;
}

/** Look up a quest's display name (fallback: the id itself). */
function questDisplayName(skillTree: ReturnType<typeof useSkillTreeStore>, id: string): string {
  const node = skillTree.nodes.find((n) => n.id === id);
  return node ? `${node.icon} ${node.name}` : id;
}

/**
 * Push the initial assistant prompt for the new learn-with-docs flow:
 *   1. List the prerequisite quests that still need to be active, then
 *   2. Offer **Install all**, **Install one by one**, **Cancel**.
 *
 * `topic` is round-tripped through the choice values so we can resume the
 * flow without keeping any extra state in the store.
 */
function pushMissingComponentsPrompt(topic: string, missingIds: string[]): void {
  const conversation = useConversationStore();
  const skillTree = useSkillTreeStore();
  const list = missingIds
    .map((id) => `• **${questDisplayName(skillTree, id)}**`)
    .join('\n');
  const enc = encodeURIComponent(topic);
  conversation.messages.push({
    id: crypto.randomUUID(),
    role: 'assistant',
    content:
      `To learn **${topic}** from your documents I need a few quests to be active first:\n\n` +
      `${list}\n\n` +
      `How would you like to proceed?`,
    agentName: 'System',
    sentiment: 'neutral',
    timestamp: Date.now(),
    questId: 'learn-docs-missing',
    questChoices: [
      { label: 'Auto install all', value: `learn-docs:install-all:${enc}`, icon: '⚡' },
      { label: 'Start chain quest', value: `learn-docs:install-each:${enc}`, icon: '📋' },
      { label: 'Cancel', value: 'dismiss', icon: '❌' },
    ],
  });
}

/** Sub-prompt for "Install one by one": one button per missing quest. */
function pushInstallEachPrompt(topic: string, missingIds: string[]): void {
  if (missingIds.length === 0) {
    pushReadyForLearnDocs(topic);
    return;
  }
  const conversation = useConversationStore();
  const skillTree = useSkillTreeStore();
  const enc = encodeURIComponent(topic);
  conversation.messages.push({
    id: crypto.randomUUID(),
    role: 'assistant',
    content: `Pick the next quest to install:`,
    agentName: 'System',
    sentiment: 'neutral',
    timestamp: Date.now(),
    questId: 'learn-docs-install-each',
    questChoices: [
      ...missingIds.map((id) => ({
        label: `Install ${questDisplayName(skillTree, id)}`,
        value: `learn-docs:install-quest:${id}:${enc}`,
        icon: '⚔️',
      })),
      { label: 'Cancel', value: 'dismiss', icon: '❌' },
    ],
  });
}

/**
 * Push the existing Scholar's Quest invitation so the user can pick which
 * documents about `topic` to import. This is the same overlay the rest of
 * the codebase uses for the document-gather step.
 */
function pushReadyForLearnDocs(topic: string): void {
  const conversation = useConversationStore();
  conversation.messages.push({
    id: crypto.randomUUID(),
    role: 'assistant',
    content:
      `Everything I need is ready. Let's start a **📚 Scholar's Quest** so you can pick the documents about **${topic}** to import.`,
    agentName: 'TerranSoul',
    sentiment: 'happy',
    timestamp: Date.now(),
    questId: 'scholar-quest',
    questChoices: [
      { label: 'Start Knowledge Quest', value: 'knowledge-quest-start', icon: '⚔️' },
      { label: 'No thanks', value: 'dismiss', icon: '💤' },
    ],
  });
}

/**
 * Entry point for the new flow. Walks the Scholar's Quest prerequisite chain
 * and either:
 *   • short-circuits straight into Scholar's Quest when nothing is missing,
 *   • or pushes the install-choice prompt above.
 */
function startLearnDocsFlow(topic: string): void {
  const skillTree = useSkillTreeStore();
  const missing = getMissingPrereqQuests(skillTree, 'scholar-quest');
  if (missing.length === 0) {
    pushReadyForLearnDocs(topic);
    return;
  }
  pushMissingComponentsPrompt(topic, missing);
}

/**
 * Auto-install path: actually activate every missing quest in the prereq
 * chain by performing the real configuration (brain setup, memory bootstrap,
 * manual-completion marking) rather than just clicking "accept" on the
 * quest-guide chat flow.
 *
 * Install order (from brain-advanced-design.md):
 *   1. 🧠 Awaken the Mind  — free cloud LLM provider
 *   2. 📖 Long-Term Memory  — SQLite memory store (auto-active once brain set)
 *   3. 📚 Sage's Library    — RAG pipeline (needs brain + ≥1 memory)
 *   4. 📚 Scholar's Quest   — document ingestion (chain quest, mark complete)
 */
async function runAutoInstall(topic: string): Promise<void> {
  const skillTree = useSkillTreeStore();
  const conversation = useConversationStore();
  const brain = useBrainStore();

  const missing = getMissingPrereqQuests(skillTree, 'scholar-quest');
  const installed: string[] = [];

  conversation.messages.push({
    id: crypto.randomUUID(),
    role: 'assistant',
    content: `⚡ Auto-installing all required quests…`,
    agentName: 'System',
    sentiment: 'happy',
    timestamp: Date.now(),
  });

  for (const id of missing) {
    try {
      // Perform the actual activation for each quest type.
      if (id === 'free-brain') {
        // Configure a free cloud LLM provider (Pollinations).
        try {
          await brain.autoConfigureForDesktop();
        } catch {
          brain.autoConfigureFreeApi();
        }
      } else if (id === 'memory') {
        // Memory auto-activates once the brain is configured.
        // Ensure brain mode is set (should be from free-brain step).
        if (!brain.brainMode) {
          try { await brain.autoConfigureForDesktop(); }
          catch { brain.autoConfigureFreeApi(); }
        }
      } else if (id === 'rag-knowledge') {
        // RAG requires brain + at least one memory.
        // Seed a bootstrap memory so the RAG quest becomes active.
        const memStore = useMemoryStore();
        if (memStore.memories.length === 0) {
          // addMemory catches errors internally and returns null (never throws).
          const entry = await memStore.addMemory({
            content: `I want to learn about ${topic} from my own documents.`,
            tags: 'learning,goal',
            importance: 5,
            memory_type: 'context',
          });
          if (!entry) {
            // Invoke failed — push a local-only entry so the status check passes.
            const now = Date.now();
            memStore.memories.push({
              id: now,
              content: `I want to learn about ${topic} from my own documents.`,
              tags: 'learning,goal',
              importance: 5,
              memory_type: 'context',
              created_at: now,
              last_accessed: null,
              access_count: 0,
              tier: 'short',
              decay_score: 1,
              session_id: null,
              parent_id: null,
              token_count: 12,
              confidence: 1.0,
            });
          }
        }
      } else if (id === 'scholar-quest') {
        // Scholar's Quest is a chain quest — mark it manually completed.
        skillTree.markComplete(id);
      }
      installed.push(id);
    } catch (err) {
      const detail = err instanceof Error ? err.message : String(err);
      conversation.messages.push({
        id: crypto.randomUUID(),
        role: 'assistant',
        content: `⚠️ Could not auto-install **${questDisplayName(skillTree, id)}**: ${detail}. Try installing it manually from the Quests tab.`,
        agentName: 'System',
        sentiment: 'sad',
        timestamp: Date.now(),
      });
    }
  }

  // Report what was installed.
  if (installed.length > 0) {
    const list = installed
      .map((id) => `✅ ${questDisplayName(skillTree, id)}`)
      .join('\n');
    conversation.messages.push({
      id: crypto.randomUUID(),
      role: 'assistant',
      content: `Installed ${installed.length} quest(s):\n\n${list}`,
      agentName: 'System',
      sentiment: 'happy',
      timestamp: Date.now(),
    });
  }

  // Recompute — anything still inactive is something we couldn't auto-finish.
  const stillMissing = getMissingPrereqQuests(skillTree, 'scholar-quest');
  if (stillMissing.length > 0) {
    const list = stillMissing.map((id) => `• ${questDisplayName(skillTree, id)}`).join('\n');
    conversation.messages.push({
      id: crypto.randomUUID(),
      role: 'assistant',
      content:
        `Some quests still need attention before I can ingest documents:\n\n${list}\n\n` +
        `Open the **Quests** tab to finish them.`,
      agentName: 'System',
      sentiment: 'neutral',
      timestamp: Date.now(),
      questId: 'learn-docs-followup',
      questChoices: [
        { label: 'Open Quests tab', value: 'navigate:skills', icon: '🗺️' },
        { label: 'Dismiss', value: 'dismiss', icon: '💤' },
      ],
    });
    return;
  }
  pushReadyForLearnDocs(topic);
}

/**
 * Manual install: surface the same per-quest list as "Install one by one".
 * The two paths share the same per-quest button — the only difference is the
 * intro message — but they're kept distinct so future prompts can diverge.
 */
function runManualInstall(topic: string): void {
  const skillTree = useSkillTreeStore();
  const missing = getMissingPrereqQuests(skillTree, 'scholar-quest');
  pushInstallEachPrompt(topic, missing);
}

/** Trigger a single missing quest (used by the per-quest buttons). */
async function runInstallSingleQuest(topic: string, questId: string): Promise<void> {
  const skillTree = useSkillTreeStore();
  const conversation = useConversationStore();
  try {
    skillTree.triggerQuestEvent(questId);
  } catch (err) {
    // triggerQuestEvent throws before handleQuestChoice runs — surface the
    // error explicitly so the user isn't left with a silent no-op.
    const detail = err instanceof Error ? err.message : String(err);
    conversation.messages.push({
      id: crypto.randomUUID(),
      role: 'assistant',
      content: `⚠️ Could not start **${questDisplayName(skillTree, questId)}**: ${detail}. Try opening it from the Quests tab.`,
      agentName: 'System',
      sentiment: 'sad',
      timestamp: Date.now(),
    });
  }
  // Re-evaluate after the user finishes interacting with this quest. We
  // surface the rest of the missing list so the user can keep going, or the
  // ready-for-docs prompt if everything's now active.
  const stillMissing = getMissingPrereqQuests(skillTree, 'scholar-quest').filter((id) => id !== questId);
  if (stillMissing.length === 0) {
    pushReadyForLearnDocs(topic);
  } else {
    pushInstallEachPrompt(topic, stillMissing);
  }
}

/**
 * Route a `learn-docs:*` quest-choice value emitted by the chat overlay.
 * Returns `true` when the value was handled.
 */
export async function handleLearnDocsChoice(value: string): Promise<boolean> {
  if (!value.startsWith('learn-docs:')) return false;
  const rest = value.slice('learn-docs:'.length);
  // Forms:
  //   install-all:<topic>
  //   install-each:<topic>
  //   install-auto:<topic>
  //   install-manual:<topic>
  //   install-back:<topic>
  //   install-quest:<questId>:<topic>
  if (rest.startsWith('install-quest:')) {
    const tail = rest.slice('install-quest:'.length);
    const colon = tail.indexOf(':');
    if (colon < 0) return false;
    const questId = tail.slice(0, colon);
    const topic = decodeURIComponent(tail.slice(colon + 1));
    await runInstallSingleQuest(topic, questId);
    return true;
  }
  const colon = rest.indexOf(':');
  if (colon < 0) return false;
  const action = rest.slice(0, colon);
  const topic = decodeURIComponent(rest.slice(colon + 1));
  switch (action) {
    case 'install-all': {
      await runAutoInstall(topic);
      return true;
    }
    case 'install-each': {
      const skillTree = useSkillTreeStore();
      const missing = getMissingPrereqQuests(skillTree, 'scholar-quest');
      pushInstallEachPrompt(topic, missing);
      return true;
    }
    case 'install-auto': {
      await runAutoInstall(topic);
      return true;
    }
    case 'install-manual': {
      runManualInstall(topic);
      return true;
    }
    case 'install-back': {
      const skillTree = useSkillTreeStore();
      const missing = getMissingPrereqQuests(skillTree, 'scholar-quest');
      pushMissingComponentsPrompt(topic, missing);
      return true;
    }
    default:
      return false;
  }
}

export async function handleModelUpdateChoice(value: string): Promise<boolean> {
  if (!value.startsWith('model-update:')) return false;
  const rest = value.slice('model-update:'.length);
  const colon = rest.indexOf(':');
  if (colon < 0) return false;
  const action = rest.slice(0, colon);
  const modelTag = rest.slice(colon + 1);
  const conversation = useConversationStore();
  if (action === 'dismiss') {
    try {
      await invoke('dismiss_model_update', { modelTag });
    } catch {
      // Best-effort in browser-only tests.
    }
    conversation.addMessage({
      id: crypto.randomUUID(),
      role: 'assistant',
      content: `Got it - I won't suggest **${modelTag}** again.`,
      agentName: 'System',
      sentiment: 'neutral',
      timestamp: Date.now(),
    });
    return true;
  }
  if (action === 'upgrade') {
    const brain = useBrainStore();
    conversation.addMessage({
      id: crypto.randomUUID(),
      role: 'assistant',
      content: `Downloading **${modelTag}**. This may take a few minutes.`,
      agentName: 'System',
      sentiment: 'neutral',
      timestamp: Date.now(),
    });
    const ok = await brain.pullModel(modelTag);
    if (ok) {
      try {
        await brain.setBrainMode({ mode: 'local_ollama', model: modelTag });
        conversation.addMessage({
          id: crypto.randomUUID(),
          role: 'assistant',
          content: `Upgrade complete! Now running **${modelTag}**.`,
          agentName: 'System',
          sentiment: 'happy',
          timestamp: Date.now(),
        });
        const installed = brain.installedModels;
        const recTags = new Set(
          brain.recommendations.filter((model) => !model.is_cloud).map((model) => model.model_tag),
        );
        for (const model of installed) {
          if (model.name === modelTag) continue;
          if (!recTags.has(model.name)) continue;
          try {
            await invoke('delete_ollama_model', { modelName: model.name });
          } catch {
            // Best-effort cleanup of older recommended local models.
          }
        }
        await brain.fetchInstalledModels();
      } catch {
        conversation.addMessage({
          id: crypto.randomUUID(),
          role: 'assistant',
          content: `Downloaded **${modelTag}** but could not activate it. You can switch manually in the Brain tab.`,
          agentName: 'System',
          sentiment: 'sad',
          timestamp: Date.now(),
        });
      }
    } else {
      conversation.addMessage({
        id: crypto.randomUUID(),
        role: 'assistant',
        content: `Failed to download **${modelTag}**. Check that Ollama is running and try again later.`,
        agentName: 'System',
        sentiment: 'sad',
        timestamp: Date.now(),
      });
    }
    return true;
  }
  return false;
}

// ── Chat-based LLM switching ─────────────────────────────────────────────────

/** Known free-provider keywords mapped to provider IDs. */
const PROVIDER_KEYWORDS: Record<string, string> = {
  pollinations: 'pollinations',
  groq: 'groq',
  cerebras: 'cerebras',
  siliconflow: 'siliconflow',
  mistral: 'mistral',
  'github models': 'github-models',
  openrouter: 'openrouter',
  nvidia: 'nvidia-nim',
  gemini: 'gemini',
};

/**
 * Detect if the user message is an LLM switching command.
 * Recognises patterns like:
 *   "switch to groq", "use pollinations", "change brain to cerebras",
 *   "use my openai api key sk-..."
 * Returns a result object if a command was detected, or null otherwise.
 */
function detectLlmCommand(text: string): {
  type: 'switch_free';
  providerId: string;
  providerName: string;
} | {
  type: 'switch_paid';
  provider: string;
  apiKey: string;
  model: string;
} | null {
  const lower = text.toLowerCase().trim();

  // Free provider switching: "switch to groq", "use cerebras", "change to pollinations"
  // Pattern: (switch|change|use|set) [to|brain to|model to|provider to] <provider_name> [api|provider|model|brain]
  const switchPattern = /(?:switch|change|use|set)\s+(?:to\s+|brain\s+to\s+|model\s+to\s+|provider\s+to\s+)?(\w[\w\s]*?)(?:\s+(?:api|provider|model|brain))?$/i;
  const match = lower.match(switchPattern);
  if (match) {
    const keyword = match[1].trim();
    for (const [name, id] of Object.entries(PROVIDER_KEYWORDS)) {
      if (keyword.includes(name)) {
        return { type: 'switch_free', providerId: id, providerName: name };
      }
    }
  }

  // Paid API: "use my openai api key sk-..." or "set openai key sk-..."
  const paidPattern = /(?:use|set)\s+(?:my\s+)?(\w+)\s+(?:api\s+)?key\s+(sk-\S+)/i;
  const paidMatch = text.match(paidPattern);
  if (paidMatch) {
    const provider = paidMatch[1].toLowerCase();
    const apiKey = paidMatch[2];
    const model = provider === 'anthropic' ? 'claude-sonnet-4-20250514' : 'gpt-4o';
    const baseUrl = provider === 'anthropic' ? 'https://api.anthropic.com' : 'https://api.openai.com';
    return { type: 'switch_paid', provider: baseUrl, apiKey, model };
  }

  return null;
}

/**
 * Execute an LLM switching command and return a confirmation message.
 */
async function executeLlmCommand(
  cmd: NonNullable<ReturnType<typeof detectLlmCommand>>,
  brain: ReturnType<typeof useBrainStore>,
): Promise<Message> {
  if (cmd.type === 'switch_free') {
    const provider = brain.freeProviders.find((p) => p.id === cmd.providerId);
    if (provider?.requires_api_key) {
      return {
        id: crypto.randomUUID(),
        role: 'assistant',
        content: `I'd love to switch to ${provider.display_name}, but it requires an API key. You can configure it in the Marketplace (🏪) under "Configure LLM".`,
        agentName: 'TerranSoul',
        sentiment: 'neutral',
        timestamp: Date.now(),
      };
    }
    const mode = { mode: 'free_api' as const, provider_id: cmd.providerId, api_key: null };
    try {
      await brain.setBrainMode(mode);
    } catch {
      brain.brainMode = mode;
    }
    const displayName = provider?.display_name ?? cmd.providerName;
    return {
      id: crypto.randomUUID(),
      role: 'assistant',
      content: `Done! I've switched to ${displayName}. Let's chat!`,
      agentName: 'TerranSoul',
      sentiment: 'happy',
      timestamp: Date.now(),
    };
  }

  // Paid API
  const mode = {
    mode: 'paid_api' as const,
    provider: cmd.provider,
    api_key: cmd.apiKey,
    model: cmd.model,
    base_url: cmd.provider,
  };
  try {
    await brain.setBrainMode(mode);
  } catch {
    brain.brainMode = mode;
  }
  return {
    id: crypto.randomUUID(),
    role: 'assistant',
    content: `Done! Paid API configured with model ${cmd.model}. Your API key is saved. Let's chat!`,
    agentName: 'TerranSoul',
    sentiment: 'happy',
    timestamp: Date.now(),
  };
}

/**
 * Resolve direct provider details for browser-side streaming.
 * Local Ollama/LM Studio are intentionally excluded here: browser mode can only
 * use them through the explicit RemoteHost path (`remote-conversation`).
 */
function resolveBrowserProvider(brain: ReturnType<typeof useBrainStore>): {
  baseUrl: string;
  model: string;
  apiKey: string | null;
  providerId: string;
} | null {
  const transport = resolveBrowserBrainTransport(brain.brainMode, brain.freeProviders);
  return transport.kind === 'direct' ? transport.provider : null;
}

function canUseTranslatorMode(brain: ReturnType<typeof useBrainStore>): boolean {
  const mode = brain.brainMode;
  if (!mode) return false;
  if (mode.mode === 'free_api') {
    const provider = brain.freeProviders.find((item) => item.id === mode.provider_id);
    return Boolean(provider && (!provider.requires_api_key || mode.api_key));
  }
  return mode.mode === 'paid_api' || mode.mode === 'local_ollama' || mode.mode === 'local_lm_studio';
}

function translatorUnavailableMessage(): Message {
  return {
    id: crypto.randomUUID(),
    role: 'assistant',
    content:
      'Translator mode needs an available free browser LLM, a paid API, or a local brain before it can translate reliably.',
    agentName: 'Translator Mode',
    sentiment: 'neutral',
    timestamp: Date.now(),
  };
}

function translatorReadyMessage(mode: TranslatorModeState): Message {
  return {
    id: crypto.randomUUID(),
    role: 'assistant',
    content: `Translator mode ready: ${mode.source.name} ↔ ${mode.target.name}.`,
    agentName: 'Translator Mode',
    sentiment: 'happy',
    timestamp: Date.now(),
  };
}

function translatorStoppedMessage(): Message {
  return {
    id: crypto.randomUUID(),
    role: 'assistant',
    content: 'Translator mode stopped.',
    agentName: 'Translator Mode',
    sentiment: 'neutral',
    timestamp: Date.now(),
  };
}

function formatRetrievedContextPack(memoryLines: string[]): string {
  return '\n\n[RETRIEVED CONTEXT]\n'
    + 'Source: TerranSoul queryable memory/RAG store.\n'
    + 'Contract: These are relevant retrieved records, not an exhaustive transcript or complete database. Use them when helpful, ignore irrelevant records, and say when retrieved context is insufficient.\n'
    + '[LONG-TERM MEMORY]\n'
    + 'The following facts from your memory were retrieved for this turn:\n'
    + memoryLines.join('\n')
    + '\n[/LONG-TERM MEMORY]\n'
    + '[/RETRIEVED CONTEXT]';
}

// Re-export detectLlmCommand for tests
export { detectLlmCommand };

export const useConversationStore = defineStore('conversation', () => {
  const messages = ref<Message[]>([]);
  const currentAgent = ref<string>('auto');
  const isThinking = ref(false);
  /** Live streaming text shown in the UI while the LLM is generating. */
  const streamingText = ref('');
  /** Whether a streaming response is in progress. */
  const isStreaming = ref(false);
  const translatorMode = ref<TranslatorModeState | null>(null);

  /** Maximum total size of all message content in bytes (~1MB). */
  const MAX_HISTORY_BYTES = 1_000_000;

  // ── Per-agent conversation threading ────────────────────────────────────

  /** The effective agent ID, or `undefined` when `currentAgent` is 'auto'. */
  function activeAgentId(): string | undefined {
    return currentAgent.value === 'auto' ? undefined : currentAgent.value;
  }

  /** Messages belonging to the currently selected agent.
   *  When agent is 'auto', all messages are returned (unfiltered). */
  const agentMessages = computed<Message[]>(() => {
    const aid = activeAgentId();
    if (!aid) return messages.value;
    return messages.value.filter(
      (m) => !m.agentId || m.agentId === aid,
    );
  });

  /** History of agent IDs that have been active in this session.
   *  Used by the UI to show agent switch markers. */
  const agentSwitchHistory = ref<{ agentId: string; timestamp: number }[]>([]);

  /** Switch to a different agent, recording the transition. */
  function setAgent(agentId: string): void {
    const prev = currentAgent.value;
    currentAgent.value = agentId;
    if (prev !== agentId && agentId !== 'auto') {
      agentSwitchHistory.value.push({ agentId, timestamp: Date.now() });
    }
  }

  /** Stamp a message with the current agent ID (mutates in place). */
  function stampAgent(msg: Message): Message {
    const aid = activeAgentId();
    if (aid) msg.agentId = aid;
    return msg;
  }

  // ── Long-running task controls (VS Code Copilot-style) ────────────────
  //
  // When the LLM is streaming/thinking, the user can:
  //   • Stop — cancel generation, discard partial output
  //   • Stop & Send — cancel generation, keep partial output as the response
  //   • Add to Queue — queue a follow-up message to send after current finishes
  //   • Steer — inject a message that redirects the current generation
  //
  // These mirror VS Code Copilot's agent-mode controls.

  /** Abort controller for the current generation. Set before each send,
   *  checked during streaming sync intervals. */
  let activeAbortController: AbortController | null = null;

  /** Concurrency gate — true while a generation (stream or fallback) is
   *  in progress.  `sendMessage()` auto-queues when this is set. */
  const generationActive = ref(false);

  /** Whether a "stop and send" was requested (keep partial text). */
  const stopAndSendRequested = ref(false);

  /** Queued messages to send after the current generation completes. */
  const messageQueue = ref<string[]>([]);

  /** Stop the current generation and discard partial output. */
  function stopGeneration(): void {
    if (activeAbortController) {
      activeAbortController.abort();
    }
    stopAndSendRequested.value = false;
  }

  /** Stop generation but keep partial output as the assistant response. */
  function stopAndSend(): void {
    stopAndSendRequested.value = true;
    if (activeAbortController) {
      activeAbortController.abort();
    }
  }

  /** Add a message to the queue — it will be sent after the current
   *  generation completes (FIFO order). */
  function addToQueue(message: string): void {
    if (message.trim()) {
      messageQueue.value.push(message.trim());
    }
  }

  /** Send a steering message that redirects the current generation.
   *  Stops the current stream, keeps partial text, then sends the
   *  steering message as a new user turn. */
  function steerWithMessage(message: string): void {
    if (!message.trim()) return;
    // Stop current generation, keep what we have
    stopAndSend();
    // Queue the steering message as the next thing to send
    messageQueue.value.unshift(message.trim());
  }

  /** Drain and send the next queued message (called after generation ends). */
  async function drainQueue(): Promise<void> {
    if (messageQueue.value.length > 0) {
      const next = messageQueue.value.shift()!;
      await sendMessage(next);
    }
  }

  async function translateWithBrowserProvider(content: string, brain: ReturnType<typeof useBrainStore>): Promise<void> {
    const mode = translatorMode.value;
    if (!mode) return;
    const provider = resolveBrowserProvider(brain);
    if (!provider) {
      messages.value.push(translatorUnavailableMessage());
      return;
    }

    const from = mode.nextDirection === 'source_to_target' ? mode.source : mode.target;
    const to = mode.nextDirection === 'source_to_target' ? mode.target : mode.source;
    const prompt = `Translate the user's message from ${from.name} (${from.code}) to ${to.name} (${to.code}). Return only the translation. User message: ${content}`;
    const fullText = await new Promise<string>((resolve, reject) => {
      let settled = false;
      let timeout: ReturnType<typeof setTimeout> | undefined;
      const abortController = streamChatCompletion(
        provider.baseUrl,
        provider.model,
        provider.apiKey,
        [{ role: 'user', content: prompt }],
        {
          onChunk: (text) => {
            if (!isStreaming.value && text) isStreaming.value = true;
            streamingText.value += text;
          },
          onSentence: (sentence) => {
            if (typeof window !== 'undefined') {
              window.dispatchEvent(
                new CustomEvent('ts:llm-sentence', { detail: { sentence, language: to.code } }),
              );
            }
          },
          onDone: (full) => {
            if (!settled) {
              settled = true;
              clearTimeout(timeout);
              resolve(full);
            }
          },
          onError: (err) => {
            if (!settled) {
              settled = true;
              clearTimeout(timeout);
              reject(new Error(err));
            }
          },
        },
        getSystemPrompt(false) + usePersonaStore().personaBlock,
      );
      if (activeAbortController) {
        activeAbortController.signal.addEventListener('abort', () => {
          if (!settled) {
            settled = true;
            clearTimeout(timeout);
            abortController.abort();
            reject(new Error('AbortError'));
          }
        }, { once: true });
      }
      timeout = setTimeout(() => {
        if (!settled) {
          settled = true;
          abortController.abort();
          reject(new Error('Translator stream timeout'));
        }
      }, 30_000);
    });

    const parsed = parseTags(fullText);
    messages.value.push({
      id: crypto.randomUUID(),
      role: 'assistant',
      content: `${from.name} → ${to.name}: ${parsed.text.trim() || fullText.trim()}`,
      agentName: 'Translator Mode',
      sentiment: 'neutral',
      timestamp: Date.now(),
    });
    mode.nextDirection = mode.nextDirection === 'source_to_target' ? 'target_to_source' : 'source_to_target';
  }

  // ── Auto-learn (daily-conversation → brain write-back loop) ─────────────
  //
  // After every assistant turn we ask the Rust-side `evaluate_auto_learn`
  // command (a pure function over `AppSettings.auto_learn_policy` + the
  // counters below) whether to fire `extract_memories_from_session`.
  //
  // The cadence policy lives in Rust so the same logic can be unit-tested
  // and so the user's chosen settings persist across reloads. See
  // `docs/brain-advanced-design.md` § 21.
  /** Total assistant replies seen in this session. */
  const totalAssistantTurns = ref(0);
  /** Value of `totalAssistantTurns` at the moment of the last auto-run. */
  const lastAutoLearnTurn = ref<number | null>(null);
  /** Latest decision for the UI ("Next auto-learn in N turns…"). */
  const lastAutoLearnDecision = ref<{
    should_fire: boolean;
    reason: string;
    turns_remaining: number | null;
  } | null>(null);
  /** Number of facts extracted on the last auto-fire. */
  const lastAutoLearnSavedCount = ref<number>(0);

  // ── Persona drift detection (Chunk 14.8) ──────────────────────────────
  //
  // After every auto-learn extraction, we accumulate the count of saved
  // facts. When the running total since the last drift check crosses a
  // threshold (default 25), we ask the backend to compare the active
  // persona against the latest `personal:*` memory cluster. If drift is
  // detected the report is surfaced in `lastDriftReport` so the UI can
  // show a suggestion banner.

  /** Running total of facts saved since the last drift check. */
  const factsSinceDriftCheck = ref(0);
  /** Default: fire a drift check every 25 accumulated facts. */
  const DRIFT_FACT_THRESHOLD = 25;
  /** Latest drift report for the UI. Null = no check has run yet. */
  const lastDriftReport = ref<DriftReport | null>(null);

  async function maybeAutoLearn(): Promise<void> {
    try {
      const decision = await invoke<{
        should_fire: boolean;
        reason: string;
        turns_remaining: number | null;
      }>('evaluate_auto_learn', {
        totalTurns: totalAssistantTurns.value,
        lastAutolearnTurn: lastAutoLearnTurn.value,
      });
      lastAutoLearnDecision.value = decision;
      if (decision.should_fire) {
        const count = await invoke<number>('extract_memories_from_session');
        lastAutoLearnSavedCount.value = count;
        lastAutoLearnTurn.value = totalAssistantTurns.value;

        if (count > 0) {
          // Refresh memory store so the Memory tab reflects new entries.
          try { await useMemoryStore().fetchAll(); } catch { /* non-fatal */ }
          console.info(`[auto-learn] Extracted ${count} memories at turn ${totalAssistantTurns.value}`);
        }

        // Accumulate facts for drift detection.
        factsSinceDriftCheck.value += count;
        if (factsSinceDriftCheck.value >= DRIFT_FACT_THRESHOLD) {
          try {
            const report = await invoke<DriftReport>('check_persona_drift');
            lastDriftReport.value = report;
          } catch {
            // Drift check failure is non-fatal — the user can still
            // use the app normally. Persona drift is a nice-to-have.
          }
          factsSinceDriftCheck.value = 0;
        }
      }
    } catch {
      // Auto-learn failures must never break a chat turn — the user can
      // always trigger extraction manually from the Memory tab.
    }
  }

  // Track assistant-turn completions by watching the messages array.
  // We deliberately key on `length + last role + isStreaming=false` so
  // we fire exactly once per landed assistant message and never during
  // streaming.
  let lastSeenLength = 0;
  watch(
    () => messages.value.length,
    (len) => {
      if (len <= lastSeenLength) {
        lastSeenLength = len;
        return;
      }
      lastSeenLength = len;
      const last = messages.value[len - 1];
      if (!last || last.role !== 'assistant') return;
      if (isStreaming.value) return;
      totalAssistantTurns.value += 1;
      void maybeAutoLearn();
    },
  );

  /** Trim oldest messages when total content size exceeds the limit. */
  function trimHistory(): void {
    let totalBytes = 0;
    for (const m of messages.value) {
      totalBytes += m.content.length * 2; // rough UTF-16 estimate
    }
    while (totalBytes > MAX_HISTORY_BYTES && messages.value.length > 1) {
      const removed = messages.value.shift();
      if (removed) totalBytes -= removed.content.length * 2;
    }
  }

  async function rememberBrowserTurn(userContent: string, assistantContent: string): Promise<void> {
    if (isTauriAvailable()) return;
    const content = `User said: ${userContent}\nTerranSoul replied: ${assistantContent}`;
    try {
      await useMemoryStore().addMemory({
        content: content.slice(0, 4000),
        tags: 'session,browser-rag,conversation',
        importance: 3,
        memory_type: 'summary',
      });
    } catch {
      // Browser memory is best-effort; never block chat completion.
    }
  }

  async function respondWithE2ePersonaFallback(content: string): Promise<void> {
    const response = stampAgent(createPersonaResponse(content));
    annotateCharismaTurn(response);
    const chunks = response.content.length > 1
      ? [
          response.content.slice(0, Math.ceil(response.content.length / 2)),
          response.content.slice(Math.ceil(response.content.length / 2)),
        ]
      : [response.content];

    isStreaming.value = true;
    streamingText.value = '';
    for (const chunk of chunks) {
      streamingText.value += chunk;
      if (typeof window !== 'undefined') {
        window.dispatchEvent(new CustomEvent('ts:llm-sentence', { detail: { sentence: chunk } }));
      }
      await new Promise((resolve) => setTimeout(resolve, 25));
    }

    messages.value.push(response);
    maybeShowQuestFromResponse(response.content, content);
    maybeShowDontKnowPrompt(response.content);
    isThinking.value = false;
    isStreaming.value = false;
    streamingText.value = '';
    activeAbortController = null;
    generationActive.value = false;
    void drainQueue();
  }

  function annotateCharismaTurn(message: Message): void {
    if (message.role !== 'assistant' || message.agentName === 'System') return;
    const persona = usePersonaStore();
    const assets = collectCharismaTurnAssets({
      text: message.content,
      motion: message.motion,
      traits: persona.traits,
      learnedExpressions: persona.learnedExpressions,
      learnedMotions: persona.learnedMotions,
    });
    if (assets.length === 0) return;
    message.charismaAssets = assets;
    if (isTauriAvailable()) {
      void useCharismaStore().recordTurnUsage(assets);
    }
  }

  async function rateCharismaTurn(messageId: string, rating: number): Promise<boolean> {
    const message = messages.value.find((m) => m.id === messageId);
    if (!message?.charismaAssets?.length) return false;
    const normalizedRating = Math.max(1, Math.min(5, Math.round(rating)));
    const rated = await useCharismaStore().rateTurnAssets(message.charismaAssets, normalizedRating);
    if (rated.length === 0) return false;
    message.charismaTurnRating = normalizedRating;
    return true;
  }

  async function sendMessage(content: string) {
    // ── Concurrency gate: only one generation at a time ──
    // If a stream/generation is already active, queue this message and
    // return immediately.  drainQueue() will pick it up when the current
    // generation finishes.
    if (generationActive.value) {
      addToQueue(content);
      return;
    }
    generationActive.value = true;

    // Set up abort controller for this generation
    activeAbortController = new AbortController();
    stopAndSendRequested.value = false;

    // Check if agent is busy with a background task
    try {
      const taskStore = useTaskStore();
      if (taskStore.isAgentBusy) {
        const running = taskStore.runningTask;
        const busyMsg: Message = {
          id: crypto.randomUUID(),
          role: 'assistant',
          content: `I'm currently working on: **${running?.description ?? 'a task'}** (${running?.progress ?? 0}%). You can cancel it or wait for it to finish.`,
          timestamp: Date.now(),
        };
        messages.value.push({
          id: crypto.randomUUID(),
          role: 'user',
          content,
          timestamp: Date.now(),
        });
        messages.value.push(busyMsg);
        generationActive.value = false;
        return;
      }
    } catch {
      // Task store not available
    }

    const userMsg: Message = {
      id: crypto.randomUUID(),
      role: 'user',
      content,
      timestamp: Date.now(),
    };
    stampAgent(userMsg);
    messages.value.push(userMsg);
    trimHistory();
    isThinking.value = true;
    streamingText.value = '';
    isStreaming.value = false;

    const brain = useBrainStore();

    const llmCmd = detectLlmCommand(content);
    // Respect the "Chat-based LLM switching" toggle: when off, ignore commands
    // like "switch to groq" / "use my openai api key …" and let them flow
    // through to the LLM as ordinary messages.
    let llmSwitchEnabled = true;
    try {
      llmSwitchEnabled = useAiDecisionPolicyStore().policy.chatBasedLlmSwitchEnabled;
    } catch {
      // Pinia not active — keep default-on behaviour.
    }
    if (llmCmd && llmSwitchEnabled) {
      const response = await executeLlmCommand(llmCmd, brain);
      messages.value.push(response);
      isThinking.value = false;
      generationActive.value = false;
      void drainQueue();
      return;
    }

    if (isStopTranslatorModeRequest(content)) {
      translatorMode.value = null;
      messages.value.push(translatorStoppedMessage());
      isThinking.value = false;
      generationActive.value = false;
      void drainQueue();
      return;
    }

    const translatorRequest = detectTranslatorModeRequest(content);
    if (translatorRequest) {
      if (!canUseTranslatorMode(brain)) {
        messages.value.push(translatorUnavailableMessage());
      } else {
        translatorMode.value = {
          active: true,
          source: translatorRequest.source,
          target: translatorRequest.target,
          nextDirection: 'source_to_target',
        };
        messages.value.push(translatorReadyMessage(translatorMode.value));
      }
      isThinking.value = false;
      generationActive.value = false;
      void drainQueue();
      return;
    }

    if (translatorMode.value?.active) {
      try {
        await translateWithBrowserProvider(content, brain);
      } catch {
        messages.value.push(translatorUnavailableMessage());
      } finally {
        isThinking.value = false;
        isStreaming.value = false;
        streamingText.value = '';
        activeAbortController = null;
        generationActive.value = false;
        void drainQueue();
      }
      return;
    }

    // ── LLM-powered intent classification (non-blocking) ─────────────
    // Fire classification concurrently with the streaming path so casual
    // chat messages (the ~95% case) don't pay a 0-3s classification
    // penalty.  If the classifier comes back with a side-channel intent
    // (learn_with_docs, teach_ingest, gated_setup) we abort the stream
    // and handle it.  For `chat` / `unknown` the stream is already
    // running — zero wasted time.
    // Short content-light turns still bypass the classifier. Contentful setup
    // requests, including LocalLLM turns, go through the backend classifier so
    // the brain can use its app/RAG knowledge to route side-channel intents.
    const classifyPromise: Promise<IntentDecision> = shouldRunIntentClassifierForTurn(content, brain)
      ? classifyIntent(content, brain.hasBrain)
      : Promise.resolve({ kind: 'chat' });
    let classifyDecision: IntentDecision | null = null;
    const shouldAwaitIntent = shouldAwaitIntentBeforeStreaming(content);
    // Check synchronously — if the cache already has the answer it
    // resolves immediately (microtask), so we can short-circuit before
    // even starting the stream for non-chat intents.
    const quickDecision = shouldAwaitIntent
      ? await classifyPromise
      : await Promise.race([
          classifyPromise.then((decision) => decision),
          // 0ms normally only picks up cached results. When the desktop app has no
          // brain configured, wait briefly so backend-owned deterministic shortcuts
          // can pre-empt the persona fallback before the streaming path fails.
          new Promise<null>((resolve) => setTimeout(() => resolve(null), brain.hasBrain ? 0 : 500)),
        ]);
    if (quickDecision && quickDecision.kind !== 'chat' && quickDecision.kind !== 'unknown') {
      classifyDecision = quickDecision;
    }
    if (classifyDecision) {
      switch (classifyDecision.kind) {
        case 'gated_setup': {
          messages.value.push(executeGatedSetupCommand({ type: classifyDecision.setup }));
          isThinking.value = false;
          generationActive.value = false;
          void drainQueue();
          return;
        }
        case 'learn_with_docs': {
          startLearnDocsFlow(classifyDecision.topic);
          isThinking.value = false;
          generationActive.value = false;
          void drainQueue();
          return;
        }
        case 'teach_ingest': {
          pushTeachScholarQuestForTopic(classifyDecision.topic);
          isThinking.value = false;
          generationActive.value = false;
          void drainQueue();
          return;
        }
      }
    }
    // For non-cached results, classification continues in the background
    // and is checked after streaming starts (see below).


    if (import.meta.env.VITE_E2E && !isTauriAvailable()) {
      await respondWithE2ePersonaFallback(content);
      return;
    }

    // Path 1: Tauri backend available → use streaming IPC command
    if (isTauriAvailable()) {
      try {
        const streaming = useStreamingStore();

        // ── One-shot handoff briefing (Chunk 23.2b) ───────────────────
        // If the user just switched to this agent and the previous agent
        // left a context summary, push it to the Rust streaming pipeline
        // (which read-and-clears it on the next system-prompt build).
        try {
          const roster = useAgentRosterStore();
          const targetAgentId =
            currentAgent.value === 'auto' ? null : currentAgent.value;
          if (targetAgentId) {
            const consumed = roster.consumeHandoff(targetAgentId);
            if (consumed) {
              const block = buildHandoffBlock({
                prevAgentName: consumed.prevAgentName,
                context: consumed.context,
              });
              if (block) {
                await invoke('set_handoff_block', { block });
              }
            }
          }
        } catch {
          // Handoff is best-effort; never block chat on it.
        }

        // Don't set isStreaming immediately - wait for first chunk
        // Keep character in thinking state until text actually arrives

        // Background classifier: when a side-channel intent arrives, abort
        // the stream and handle the intent. For document-learning and
        // teach/ingest intents we always divert — even if some stream text
        // has already arrived — because the user explicitly asked for a
        // workflow, not a chat answer. For gated_setup we only divert when
        // no text has streamed yet, since setup confirmations are less
        // disruptive as a post-response prompt.
        let bgIntentHandled = false;
        if (!classifyDecision) {
          classifyPromise.then((d) => {
            if (bgIntentHandled) return;
            if (d.kind === 'chat' || d.kind === 'unknown') return;
            // gated_setup only pre-empts when no text has streamed yet.
            if (d.kind === 'gated_setup' && streaming.streamText) return;
            bgIntentHandled = true;
            // Push the quest/setup flow first, then abort — the abort
            // handler discards partial stream text and drains the queue.
            switch (d.kind) {
              case 'gated_setup':
                messages.value.push(executeGatedSetupCommand({ type: d.setup }));
                break;
              case 'learn_with_docs':
                startLearnDocsFlow(d.topic);
                break;
              case 'teach_ingest':
                pushTeachScholarQuestForTopic(d.topic);
                break;
            }
            activeAbortController?.abort();
          }).catch(() => { /* classifier errors are non-fatal */ });
        }

        // While `invoke` blocks, mirror `streaming.streamText` into this
        // store's `streamingText` at ~16ms intervals so reactive UI stays live.
        // Also checks the abort signal for user-initiated stop.
        const abortSignal = activeAbortController?.signal;
        const syncInterval = setInterval(() => {
          streamingText.value = streaming.streamText;
          // Sync isStreaming state with the streaming store.
          // Clear isThinking as soon as actual text arrives so the avatar
          // transitions from "thinking" to "talking" immediately — without
          // this, isThinking stayed true for the entire stream and the 3D
          // model kept showing the thinking animation even while text was
          // visibly rendering in the chat bubble.
          if (streaming.isStreaming && !isStreaming.value) {
            isStreaming.value = true;
            // Only clear isThinking when answer text arrives (not during
            // the extended-thinking phase).
            if (!streaming.isThinkingPhase) {
              isThinking.value = false;
            }
          } else if (!streaming.isStreaming && isStreaming.value) {
            isStreaming.value = false;
          }
        }, 16);

        const TAURI_STREAM_TIMEOUT_MS = 180_000; // 180s timeout (large models need cold-start loading)
        let sendOk = false;
        let wasAborted = false;
        try {
          sendOk = await Promise.race([
            streaming.sendStreaming(content),
            new Promise<boolean>((_, reject) =>
              setTimeout(() => reject(new Error('Tauri streaming timeout')), TAURI_STREAM_TIMEOUT_MS),
            ),
            new Promise<boolean>((_, reject) => {
              if (abortSignal) {
                if (abortSignal.aborted) { reject(new Error('AbortError')); return; }
                abortSignal.addEventListener('abort', () => reject(new Error('AbortError')), { once: true });
              }
            }),
          ]);
        } catch (e) {
          if (String(e).includes('AbortError')) {
            wasAborted = true;
            sendOk = false;
          } else {
            throw e;
          }
        } finally {
          clearInterval(syncInterval);
        }

        // Handle user-initiated stop
        if (wasAborted) {
          const partialText = streaming.streamText;
          streaming.reset();
          isStreaming.value = false;
          streamingText.value = '';

          if (stopAndSendRequested.value && partialText) {
            // Stop & Send: keep partial output as the response
            const parsed = parseTags(partialText);
            const sentiment = streaming.currentEmotion ?? parsed.emotion ?? detectSentiment(content);
            const assistantMsg: Message = {
              id: crypto.randomUUID(),
              role: 'assistant',
              content: parsed.text + '\n\n*[Generation stopped by user]*',
              agentName: 'TerranSoul',
              sentiment: sentiment as Message['sentiment'],
              timestamp: Date.now(),
            };
            stampAgent(assistantMsg);
            annotateCharismaTurn(assistantMsg);
            messages.value.push(assistantMsg);
          }
          // else: pure Stop — discard partial output

          isThinking.value = false;
          stopAndSendRequested.value = false;
          activeAbortController = null;
          generationActive.value = false;
          void drainQueue();
          return;
        }

        if (!sendOk) throw new Error(streaming.error ?? 'Streaming failed');

        // Grace period for any in-flight events after invoke resolves.
        // Rust emits done:true reliably, so we only need a very short
        // grace — just enough for the Tauri event to transit IPC.
        if (streaming.isStreaming) {
          const graceWait = 200;
          const start = Date.now();
          while (streaming.isStreaming && Date.now() - start < graceWait) {
            streamingText.value = streaming.streamText;
            await new Promise((r) => setTimeout(r, 16));
          }
        }

        isStreaming.value = false;
        streamingText.value = '';

        // Create the final assistant message from accumulated text.
        // Text is already clean (anim blocks stripped by Rust parser),
        // but the LLM may still return JSON-wrapped text outside tags.
        const parsed = parseTags(streaming.streamText);
        const cleanText = parsed.text;

        // ── Post-stream classifier check ──────────────────────────────
        // With LocalOllama (NUM_PARALLEL=1) the classifier request is
        // queued behind the chat stream in Ollama's request queue. The
        // stream finishes before the classifier, so the background
        // `.then()` handler can't abort in time. Now that the GPU is
        // free, wait briefly for the classifier; if it returns a
        // side-channel intent, show the quest flow instead of the chat
        // response.
        if (!bgIntentHandled) {
          try {
            const POST_STREAM_CLASSIFY_WAIT_MS = 5000;
            const postDecision = await Promise.race([
              classifyPromise,
              new Promise<IntentDecision>((r) =>
                setTimeout(() => r({ kind: 'chat' }), POST_STREAM_CLASSIFY_WAIT_MS),
              ),
            ]);
            if (
              postDecision.kind !== 'chat' &&
              postDecision.kind !== 'unknown'
            ) {
              bgIntentHandled = true;
              streaming.reset();
              switch (postDecision.kind) {
                case 'learn_with_docs':
                  startLearnDocsFlow(postDecision.topic);
                  break;
                case 'teach_ingest':
                  pushTeachScholarQuestForTopic(postDecision.topic);
                  break;
                case 'gated_setup':
                  messages.value.push(
                    executeGatedSetupCommand({ type: postDecision.setup }),
                  );
                  break;
              }
              // Skip the chat response — quest flow handles the turn.
              // The finally block cleans up isThinking/isStreaming/etc.
              return;
            }
          } catch {
            // Classifier error — proceed with the normal chat response.
          }
        }

        if (cleanText) {
          // Emotion comes from the streaming store (set by llm-animation events).
          const sentiment = streaming.currentEmotion ?? parsed.emotion ?? detectSentiment(content);
          const motion = streaming.currentMotion ?? parsed.motion ?? undefined;
          const { clean, warning } = extractWarning(cleanText);
          const thinkingContent = streaming.thinkingText || undefined;
          const assistantMsg: Message = {
            id: crypto.randomUUID(),
            role: 'assistant',
            content: clean || cleanText,
            agentName: 'TerranSoul',
            sentiment: sentiment as Message['sentiment'],
            timestamp: Date.now(),
            emoji: parsed.emoji ?? undefined,
            motion,
            thinkingContent,
          };
          stampAgent(assistantMsg);
          if (warning) applyWarningAsQuest(assistantMsg, warning);
          annotateCharismaTurn(assistantMsg);
          messages.value.push(assistantMsg);
          maybeShowQuestFromResponse(clean || cleanText, content);
          maybeShowDontKnowPrompt(clean || cleanText);
        } else {
          // Streaming completed but no text accumulated (events not received or
          // API returned empty) — fall back to non-streaming invoke which also
          // routes through brain_mode on the backend.
          throw new Error('Streaming produced no text');
        }
        streaming.reset();
      } catch {
        // Tauri streaming failed — fall back to non-streaming invoke with timeout
        try {
          const FALLBACK_TIMEOUT_MS = 120_000;
          const response = await Promise.race([
            invoke<Message>('send_message', {
              message: content,
              agentId: currentAgent.value === 'auto' ? null : currentAgent.value,
            }),
            new Promise<never>((_, reject) =>
              setTimeout(() => reject(new Error('Fallback invoke timeout')), FALLBACK_TIMEOUT_MS),
            ),
          ]);
          annotateCharismaTurn(response);
          messages.value.push(response);
          maybeShowQuestFromResponse(response.content, content);
          maybeShowDontKnowPrompt(response.content);
        } catch {
          const response = createPersonaResponse(content);
          annotateCharismaTurn(response);
          messages.value.push(response);
          pushNetworkOrProviderWarning();
        }
      } finally {
        isThinking.value = false;
        isStreaming.value = false;
        streamingText.value = '';
        activeAbortController = null;
        generationActive.value = false;
        void drainQueue();
      }
      return;
    }

    // Path 2: No Tauri, but brain is configured → browser-side streaming
    if (brain.hasBrain) {
      const provider = resolveBrowserProvider(brain);
      if (provider) {
        try {
        const healthStore = useProviderHealthStore();
        const history = buildHistory(
          messages.value.map((m) => ({ role: m.role, content: m.content })),
        );

        // RAG: fetch relevant memories from Tauri or browser-native storage.
        let memoryBlock = '';
        if (!shouldUseFastChatPath(content)) {
          try {
            const results = await useMemoryStore().hybridSearch(content, 5);
            if (results && results.length > 0) {
              memoryBlock = formatRetrievedContextPack(results.map((m) => `- ${m.content}`));
            }
          } catch {
            // No memories — continue without RAG.
          }
        }

        // Try the primary provider, then rotate to next healthy on rate-limit
        const providersToTry = browserDirectFallbackProviders(provider, brain.brainMode, brain.freeProviders);

        let succeeded = false;
        for (const prov of providersToTry) {
          const provId = prov.providerId;
          // Skip if already known to be rate-limited
          const healthInfo = healthStore.providers.find((p) => p.id === provId);
          if (healthInfo?.is_rate_limited) continue;

          try {
            // Don't set isStreaming immediately - wait for first chunk
            streamingText.value = '';

            // Use enhanced prompt for non-Pollinations providers (upgraded models)
            const useEnhanced = provId !== 'pollinations';

            const STREAM_TIMEOUT_MS = 30_000; // 30s timeout to prevent stuck thinking
            const fullText = await new Promise<string>((resolve, reject) => {
              let timeout: ReturnType<typeof setTimeout> | undefined;
              let settled = false;
              const abortController = streamChatCompletion(
                prov.baseUrl,
                prov.model,
                prov.apiKey,
                history,
                {
                  onChunk: (text) => {
                    // Set isStreaming only when first chunk arrives
                    if (!isStreaming.value && text) {
                      isStreaming.value = true;
                    }
                    streamingText.value += text;
                  },
                  onSentence: (sentence) => {
                    // Sentence-by-sentence event — lets TTS, animation,
                    // and analytics react before the full response is
                    // ready. Decoupled via a window CustomEvent so we
                    // don't pull the TTS/animation modules into the
                    // conversation store.
                    if (typeof window !== 'undefined') {
                      window.dispatchEvent(
                        new CustomEvent('ts:llm-sentence', { detail: { sentence } }),
                      );
                    }
                  },
                  onDone: (full) => { if (!settled) { settled = true; clearTimeout(timeout); resolve(full); } },
                  onError: (err) => { if (!settled) { settled = true; clearTimeout(timeout); reject(new Error(err)); } },
                },
                getSystemPrompt(useEnhanced) +
                  usePersonaStore().personaBlock +
                  memoryBlock +
                  (() => {
                    // ── One-shot handoff briefing (Chunk 23.2b, browser path) ────
                    try {
                      const roster = useAgentRosterStore();
                      const targetAgentId =
                        currentAgent.value === 'auto'
                          ? null
                          : currentAgent.value;
                      if (!targetAgentId) return '';
                      const consumed = roster.consumeHandoff(targetAgentId);
                      if (!consumed) return '';
                      return buildHandoffBlock({
                        prevAgentName: consumed.prevAgentName,
                        context: consumed.context,
                      });
                    } catch {
                      return '';
                    }
                  })(),
              );
              // User-initiated abort
              if (activeAbortController) {
                activeAbortController.signal.addEventListener('abort', () => {
                  if (!settled) {
                    settled = true;
                    clearTimeout(timeout);
                    abortController.abort();
                    if (stopAndSendRequested.value) {
                      resolve(streamingText.value);
                    } else {
                      reject(new Error('AbortError'));
                    }
                  }
                }, { once: true });
              }
              timeout = setTimeout(() => {
                if (!settled) { settled = true; abortController.abort(); reject(new Error('Stream timeout: no response within 60s')); }
              }, STREAM_TIMEOUT_MS);
            });

            isStreaming.value = false;
            streamingText.value = '';

            const parsed = parseTags(fullText);
            // Use keyword detection on user input as fallback when LLM has no emotion tags
            const sentiment = parsed.emotion ?? detectSentiment(content);
            const { clean, warning } = extractWarning(parsed.text);
            const assistantMsg: Message = {
              id: crypto.randomUUID(),
              role: 'assistant',
              content: clean || parsed.text,
              agentName: 'TerranSoul',
              sentiment: sentiment as Message['sentiment'],
              timestamp: Date.now(),
              emoji: parsed.emoji ?? undefined,
              motion: parsed.motion ?? undefined,
            };
            stampAgent(assistantMsg);
            if (warning) applyWarningAsQuest(assistantMsg, warning);
            annotateCharismaTurn(assistantMsg);
            messages.value.push(assistantMsg);
            void rememberBrowserTurn(content, clean || parsed.text);
            maybeShowQuestFromResponse(clean || parsed.text, content);
            maybeShowDontKnowPrompt(clean || parsed.text);
            succeeded = true;
            break;
          } catch (err) {
            isStreaming.value = false;
            streamingText.value = '';
            // Check if it's a rate-limit error — mark and try next provider
            const errMsg = String(err);
            if (errMsg.includes('429') || errMsg.toLowerCase().includes('rate limit')) {
              healthStore.markRateLimited(provId);
              continue; // Try next provider
            }
            // Check for network / timeout errors
            if (errMsg.includes('Network error') || errMsg.includes('Network timeout') || errMsg.includes('Stream timeout') || errMsg.includes('Failed to fetch')) {
              // Network issue — try next provider
              continue;
            }
            // Other error — don't retry
            break;
          }
        }

        if (!succeeded) {
          const response = createPersonaResponse(content);
          annotateCharismaTurn(response);
          messages.value.push(response);
          pushNetworkOrProviderWarning();
        }
        } finally {
          isThinking.value = false;
          isStreaming.value = false;
          streamingText.value = '';
          activeAbortController = null;
          generationActive.value = false;
          void drainQueue();
        }
        return;
      }
    }

    // Path 3: No brain configured — persona fallback
    await new Promise((r) => setTimeout(r, 500));
    const response = createPersonaResponse(content);
    annotateCharismaTurn(response);
    messages.value.push(response);
    isThinking.value = false;
    activeAbortController = null;
    generationActive.value = false;
    void drainQueue();
  }

  /** Push a warning message with upgrade quest when all providers are exhausted. */
  function pushProviderWarning(): void {
    // Avoid duplicate warnings within the last few messages
    const recent = messages.value.slice(-5);
    if (recent.some(m => m.agentName === 'System' && m.content.includes('rate limit'))) return;

    messages.value.push({
      id: crypto.randomUUID(),
      role: 'assistant',
      content:
        '⚠️ The free AI providers are currently rate-limited. ' +
        'Responses may be slower or use a basic fallback until limits reset.\n\n' +
        'You can upgrade to a paid API or use llmfit to run a local model for unlimited, faster responses!',
      agentName: 'System',
      sentiment: 'neutral',
      timestamp: Date.now(),
      questChoices: [
        { label: 'Upgrade to Paid API', value: 'navigate:brain-setup', icon: '⚡' },
        { label: 'Use llmfit (Local AI)', value: 'navigate:brain-setup', icon: '🏰' },
        { label: 'I\'ll wait', value: 'dismiss', icon: '⏳' },
      ],
      questId: 'paid-brain',
    });
  }

  /**
   * Detect whether the failure was network-related and push the appropriate
   * warning. Falls back to the rate-limit warning when no network signal is
   * found.
   */
  function pushNetworkOrProviderWarning(): void {
    const recent = messages.value.slice(-5);
    if (recent.some(m => m.agentName === 'System' && (m.content.includes('network') || m.content.includes('rate limit')))) return;

    messages.value.push({
      id: crypto.randomUUID(),
      role: 'assistant',
      content:
        '⚠️ Could not reach the AI provider — this is likely a network issue or the service is temporarily down.\n\n' +
        'For a more reliable experience, consider installing a **local LLM** so TerranSoul can work offline without depending on external servers.',
      agentName: 'System',
      sentiment: 'neutral',
      timestamp: Date.now(),
      questChoices: [
        { label: 'Install Local LLM (Recommended)', value: 'navigate:brain-setup', icon: '🏰' },
        { label: 'Upgrade to Paid API', value: 'navigate:brain-setup', icon: '⚡' },
        { label: 'Retry', value: 'retry', icon: '🔄' },
      ],
      questId: 'local-brain',
    });
  }

  async function getConversation() {
    try {
      const history = await invoke<Message[]>('get_conversation');
      messages.value = history;
      trimHistory();
    } catch {
      // ignore
    }
  }

  /** Add a message directly to the conversation without AI processing. */
  function addMessage(message: Message): void {
    stampAgent(message);
    messages.value.push(message);
    trimHistory();
  }

  return {
    messages,
    currentAgent,
    translatorMode,
    agentMessages,
    agentSwitchHistory,
    setAgent,
    isThinking,
    streamingText,
    isStreaming,
    sendMessage,
    rateCharismaTurn,
    getConversation,
    addMessage,
    pushProviderWarning,
    // Long-running task controls (VS Code Copilot-style)
    generationActive,
    messageQueue,
    stopGeneration,
    stopAndSend,
    addToQueue,
    steerWithMessage,
    // Auto-learn surface (see docs/brain-advanced-design.md § 21)
    totalAssistantTurns,
    lastAutoLearnTurn,
    lastAutoLearnDecision,
    lastAutoLearnSavedCount,
    // Persona drift detection (Chunk 14.8)
    lastDriftReport,
    factsSinceDriftCheck,
  };
});
