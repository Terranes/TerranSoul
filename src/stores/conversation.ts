import { defineStore } from 'pinia';
import { ref, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { Message } from '../types';
import { useBrainStore } from './brain';
import { useStreamingStore } from './streaming';
import { useProviderHealthStore } from './provider-health';
import { useSkillTreeStore } from './skill-tree';
import { useTaskStore } from './tasks';
import { usePersonaStore } from './persona';
import { streamChatCompletion, buildHistory, getSystemPrompt } from '../utils/free-api-client';
import { parseTags } from '../utils/emotion-parser';

/**
 * Keyword-based sentiment detection from text content.
 * Used as a fallback when the LLM response doesn't include emotion tags.
 * Checks both user input and assistant response for emotional cues.
 */
export function detectSentiment(text: string): 'happy' | 'sad' | 'angry' | 'relaxed' | 'surprised' | 'neutral' {
  const lower = text.toLowerCase();

  if (lower.includes('angry') || lower.includes('furious') || lower.includes('annoyed') || lower.includes('frustrat')) {
    return 'angry';
  }
  if (lower.includes('surprise') || lower.includes('wow') || lower.includes('unexpected') || lower.includes('amazing') || lower.includes('whoa') || lower.includes('omg')) {
    return 'surprised';
  }
  if (lower.includes('relax') || lower.includes('calm') || lower.includes('peaceful') || lower.includes('chill') || lower.includes('meditat')) {
    return 'relaxed';
  }
  if (lower.includes('sad') || lower.includes('bad') || lower.includes('hate') || lower.includes('sorry') || lower.includes('cry')) {
    return 'sad';
  }
  if (lower.includes('hello') || /\bhi\b/.test(lower) || lower.startsWith('hey') || lower.includes('happy') || lower.includes('great') || lower.includes('awesome') || lower.includes('love')) {
    return 'happy';
  }
  return 'neutral';
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

/** Browser-side persona fallback when no brain is configured at all. */
function createPersonaResponse(content: string): Message {
  const sentiment = detectSentiment(content);
  let response: string;

  switch (sentiment) {
    case 'angry':
      response = "I can sense your frustration. Take a deep breath — I'm here to help. 🔥";
      break;
    case 'surprised':
      response = "Wow, that's surprising! Tell me more! 😮";
      break;
    case 'relaxed':
      response = "That sounds so peaceful. Let's take a moment to enjoy the calm. 🧘";
      break;
    case 'sad':
      response = "I understand you're going through something difficult. I'm here for you. 💙";
      break;
    case 'happy':
      response = "That's wonderful to hear! Your positive energy is contagious! ✨";
      break;
    default:
      response = `Hello! I'm TerranSoul. Please configure a brain (free cloud API or paid API) in the Marketplace so I can have a real conversation with you!`;
      break;
  }

  return {
    id: crypto.randomUUID(),
    role: 'assistant',
    content: response,
    agentName: 'TerranSoul',
    sentiment,
    timestamp: Date.now(),
  };
}

/** Detect if the Tauri IPC bridge is available (synchronous check). */
function isTauriAvailable(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
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
  const responseLower = responseText.toLowerCase();
  const inputLower = (userInput ?? '').toLowerCase();

  // Signal 1: LLM response specifically mentions quests (word-boundary match to avoid
  // false positives like "requests" containing "quest" as a substring)
  const questWordRe = /\bquest\b/i;
  const hasResponseSignal = questWordRe.test(responseText) || responseLower.includes('skill tree');

  // Signal 2: User's intent is getting-started (broad keyword matching, not strict regex)
  const gettingStartedWords = ['start', 'begin', 'first', 'should i do', 'can i do', 'what next', 'get started', 'where do i', 'how do i'];
  const hasInputSignal = gettingStartedWords.some(w => inputLower.includes(w));

  if (!hasResponseSignal && !hasInputSignal) return;

  try {
    const skillTree = useSkillTreeStore();
    const availableQuests = skillTree.nodes.filter(n => {
      try { return skillTree.getSkillStatus(n.id) === 'available'; }
      catch { return false; }
    });
    if (availableQuests.length > 0) {
      skillTree.triggerQuestEvent(availableQuests[0].id);
    }
  } catch {
    // Skill tree not ready — skip quest overlay
  }
}

/**
 * Detect an explicit "teach the AI" instruction from the user.
 *
 * We only fire Scholar's Quest on phrases that clearly request ingestion of
 * new source material.  A plain question like *"I want to learn about X"* is
 * NOT an instruction to ingest — it's a question the LLM should answer
 * directly, so it must NOT match here.
 *
 * Matching phrases (case-insensitive):
 *   "remember the following law: …"
 *   "remember this law …"
 *   "learn the following …"
 *   "memori(s|z)e this …"
 *   "ingest this …" / "import this document/file/url …"
 *   "provide your own context" / "provide my own context"
 *   "here is the context/source/document …"
 */
export function detectTeachIntent(userInput: string): { topic: string } | null {
  const trimmed = userInput.trim();

  const patterns: Array<{ re: RegExp; capture: number }> = [
    // "remember/learn (the|this) following [law|article|rule|text|content|…]: …"
    { re: /^(?:please\s+)?(?:remember|learn|memori[sz]e)\s+(?:the|this)\s+following(?:\s+\w+)?\s*[:\-–]\s*(.+)$/i, capture: 1 },
    { re: /^(?:please\s+)?(?:remember|learn|memori[sz]e)\s+(?:the|this)\s+following\b(.*)$/i, capture: 1 },
    // "remember/memorize this law/article/rule/fact: …"
    { re: /^(?:please\s+)?(?:remember|memori[sz]e)\s+this\s+(?:law|article|rule|fact|statute|regulation|text|document)\b[:\s\-–]*(.*)$/i, capture: 1 },
    // "ingest/import this/the following document/file/url: …"
    { re: /^(?:please\s+)?(?:ingest|import)\s+(?:this|the\s+following)\s*(?:document|file|url|page)?\b[:\s\-–]*(.*)$/i, capture: 1 },
    // "provide (my|your) own context"  (typo-tolerant: prov(i)?de)
    { re: /^(?:i(?:'ll|\s+will)?\s+)?prov[i]?de\s+(?:my|your)\s+own\s+context\b[:\s\-–]*(.*)$/i, capture: 1 },
    // "here is / here's (the|my) context/source/document/article"
    { re: /^here\s*(?:'s|is)\s+(?:the|my)\s+(?:context|source|document|article|law|text)\b[:\s\-–]*(.*)$/i, capture: 1 },
  ];

  for (const { re, capture } of patterns) {
    const m = trimmed.match(re);
    if (m) {
      const topic = (m[capture] ?? '').trim() || 'the provided content';
      return { topic };
    }
  }
  return null;
}

/**
 * When the user explicitly asks the AI to remember / ingest new content,
 * push a Scholar's Quest suggestion message (with overlay choices).  This is
 * the ONLY path that auto-triggers Scholar's Quest from chat — we never fire
 * it on plain questions about a topic.
 */
function maybeShowScholarQuestFromTeachIntent(userInput: string): void {
  const teach = detectTeachIntent(userInput);
  if (!teach) return;

  const conversation = useConversationStore();
  conversation.messages.push({
    id: crypto.randomUUID(),
    role: 'assistant',
    content:
      `Got it — let's ingest **${teach.topic}** into my long-term memory. ` +
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
 * Detect the two gated setup commands the user can type after a
 * "don't-know" prompt.  These are literal confirmations — we only fire the
 * matching setup flow when the user explicitly types one.
 */
export function detectGatedSetupCommand(userInput: string):
  | { type: 'upgrade_gemini' }
  | { type: 'provide_context' }
  | null {
  const lower = userInput.toLowerCase().trim();

  // "upgrade to gemini [model]"  (typo-tolerant: gem(i)?ni)
  if (/^(?:please\s+)?upgrade\s+to\s+gem[i]?ni(?:\s+model)?\s*[.!]?$/i.test(lower)
    || /^switch\s+to\s+gem[i]?ni(?:\s+model)?\s+(?:for\s+)?(?:google\s+)?search\b/i.test(lower)
    || /^use\s+gem[i]?ni\s+(?:for|with)\s+(?:google\s+)?search\b/i.test(lower)) {
    return { type: 'upgrade_gemini' };
  }

  // "provide (my|your) own context"  (typo-tolerant: prov(i)?de)
  if (/^(?:please\s+|i(?:'ll|\s+will)?\s+)?prov[i]?de\s+(?:my|your)\s+own\s+context\s*[.!]?$/i.test(lower)) {
    return { type: 'provide_context' };
  }

  return null;
}

/**
 * Execute a gated setup command.  Returns the assistant message to push into
 * the conversation; the message carries quest choices that wire up into the
 * existing overlay so the user can proceed with a single click.
 */
function executeGatedSetupCommand(cmd: NonNullable<ReturnType<typeof detectGatedSetupCommand>>): Message {
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
 * Detect an explicit "learn / study X using my documents" request.
 *
 * Examples that match:
 *   "Learn Vietnamese laws using my provided documents"
 *   "Study quantum physics with my files"
 *   "Learn about contract law from my notes"
 *
 * Returns the extracted topic, or null when the phrase doesn't match.
 */
export function detectLearnWithDocsIntent(userInput: string): { topic: string } | null {
  const trimmed = userInput.trim();
  const re =
    /^(?:please\s+|i\s+(?:want|would\s+like)\s+to\s+)?(?:learn|study)\s+(?:about\s+)?(.+?)\s+(?:using|with|from|via)\s+(?:my|the|our)\s+(?:own\s+|provided\s+)?(?:documents?|docs?|files?|sources?|notes?|materials?|pdfs?|articles?)\b[\s.!?]*$/i;
  const m = trimmed.match(re);
  if (m) {
    const topic = (m[1] ?? '').trim();
    if (topic) return { topic };
  }
  return null;
}

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
      { label: 'Install all', value: `learn-docs:install-all:${enc}`, icon: '⚡' },
      { label: 'Install one by one', value: `learn-docs:install-each:${enc}`, icon: '📋' },
      { label: 'Cancel', value: 'dismiss', icon: '❌' },
    ],
  });
}

/** Sub-prompt: choose Auto vs Manual install after picking "Install all". */
function pushInstallAllModePrompt(topic: string): void {
  const conversation = useConversationStore();
  const enc = encodeURIComponent(topic);
  conversation.messages.push({
    id: crypto.randomUUID(),
    role: 'assistant',
    content:
      `Great — installing everything. Should I do it **automatically** ` +
      `(I'll trigger and accept each quest for you) or **manually** ` +
      `(you accept each quest one by one)?`,
    agentName: 'System',
    sentiment: 'neutral',
    timestamp: Date.now(),
    questId: 'learn-docs-install-mode',
    questChoices: [
      { label: 'Auto install', value: `learn-docs:install-auto:${enc}`, icon: '⚡' },
      { label: 'Manual install', value: `learn-docs:install-manual:${enc}`, icon: '🛠️' },
      { label: 'Back', value: `learn-docs:install-back:${enc}`, icon: '↩️' },
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
 * Auto-install path: trigger every still-missing quest in the prereq chain
 * and immediately accept it on the user's behalf — leveraging the existing
 * skill-tree quest engine rather than implementing any installer from
 * scratch. After the chain is walked, push the Scholar's Quest invitation
 * so the user can pick documents to import (same as the existing flow).
 */
async function runAutoInstall(topic: string): Promise<void> {
  const skillTree = useSkillTreeStore();
  const conversation = useConversationStore();

  conversation.messages.push({
    id: crypto.randomUUID(),
    role: 'assistant',
    content: `⚡ Auto-installing all required quests…`,
    agentName: 'System',
    sentiment: 'happy',
    timestamp: Date.now(),
  });

  // Re-evaluate the missing list at run time — the user may have completed
  // a quest by hand between the prompt and clicking "Auto install".
  let missing = getMissingPrereqQuests(skillTree, 'scholar-quest');
  for (const id of missing) {
    try {
      skillTree.triggerQuestEvent(id);
      await skillTree.handleQuestChoice(id, 'accept');
    } catch (err) {
      // Quest engine refused — surface it to the user (with the actual
      // error) but keep going so the remaining quests still get a chance
      // to install.
      const detail = err instanceof Error ? err.message : String(err);
      conversation.messages.push({
        id: crypto.randomUUID(),
        role: 'assistant',
        content: `⚠️ Could not auto-accept **${questDisplayName(skillTree, id)}**: ${detail}. Try installing it manually from the Quests tab.`,
        agentName: 'System',
        sentiment: 'sad',
        timestamp: Date.now(),
      });
    }
  }

  // Recompute — anything still inactive is something we couldn't auto-finish.
  missing = getMissingPrereqQuests(skillTree, 'scholar-quest');
  if (missing.length > 0) {
    const list = missing.map((id) => `• ${questDisplayName(skillTree, id)}`).join('\n');
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
      pushInstallAllModePrompt(topic);
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
 * Resolve the provider details (base_url, model, api_key) from the brain store
 * for browser-side streaming.  Supports free_api, paid_api, and local_ollama modes.
 */
function resolveBrowserProvider(brain: ReturnType<typeof useBrainStore>): {
  baseUrl: string;
  model: string;
  apiKey: string | null;
} | null {
  if (!brain.brainMode) return null;

  const mode = brain.brainMode;

  if (mode.mode === 'free_api') {
    const provider = brain.freeProviders.find((p) => p.id === mode.provider_id);
    if (!provider) return null;
    return {
      baseUrl: provider.base_url,
      model: provider.model,
      apiKey: mode.api_key ?? null,
    };
  }

  if (mode.mode === 'paid_api') {
    return {
      baseUrl: mode.base_url,
      model: mode.model,
      apiKey: mode.api_key,
    };
  }

  if (mode.mode === 'local_ollama') {
    return {
      baseUrl: 'http://localhost:11434',
      model: mode.model,
      apiKey: null,
    };
  }

  return null;
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

  /** Maximum total size of all message content in bytes (~1MB). */
  const MAX_HISTORY_BYTES = 1_000_000;

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

  async function sendMessage(content: string) {
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
    messages.value.push(userMsg);
    trimHistory();
    isThinking.value = true;
    streamingText.value = '';
    isStreaming.value = false;

    const brain = useBrainStore();

    const llmCmd = detectLlmCommand(content);
    if (llmCmd) {
      const response = await executeLlmCommand(llmCmd, brain);
      messages.value.push(response);
      isThinking.value = false;
      return;
    }

    // Gated setup commands — user explicitly typed "upgrade to Gemini model"
    // or "provide your own context" (usually after a don't-know prompt).
    const gatedCmd = detectGatedSetupCommand(content);
    if (gatedCmd) {
      messages.value.push(executeGatedSetupCommand(gatedCmd));
      isThinking.value = false;
      return;
    }

    // Explicit "Learn X using my [provided] documents" intent — never calls
    // the LLM. Walks the Scholar's Quest prerequisite chain, lists the
    // quests that aren't active yet, and offers Install all / Install one
    // by one / Cancel. The "Auto install all" path simply auto-triggers and
    // accepts each missing quest via the existing skill-tree engine.
    const learnDocs = detectLearnWithDocsIntent(content);
    if (learnDocs) {
      startLearnDocsFlow(learnDocs.topic);
      isThinking.value = false;
      return;
    }

    // Explicit "teach the AI" intent — triggers Scholar's Quest directly,
    // without calling the LLM (the user wants to ingest, not ask).
    if (detectTeachIntent(content)) {
      maybeShowScholarQuestFromTeachIntent(content);
      isThinking.value = false;
      return;
    }

    // Path 1: Tauri backend available → use streaming IPC command
    if (isTauriAvailable()) {
      try {
        const streaming = useStreamingStore();

        // Don't set isStreaming immediately - wait for first chunk
        // Keep character in thinking state until text actually arrives

        // While `invoke` blocks, mirror `streaming.streamText` into this
        // store's `streamingText` at ~50ms intervals so reactive UI stays live.
        const syncInterval = setInterval(() => {
          streamingText.value = streaming.streamText;
          // Sync isStreaming state with the streaming store
          if (streaming.isStreaming && !isStreaming.value) {
            isStreaming.value = true;
          } else if (!streaming.isStreaming && isStreaming.value) {
            isStreaming.value = false;
          }
        }, 50);

        const TAURI_STREAM_TIMEOUT_MS = 60_000; // 60s timeout
        let sendOk = false;
        try {
          sendOk = await Promise.race([
            streaming.sendStreaming(content),
            new Promise<boolean>((_, reject) =>
              setTimeout(() => reject(new Error('Tauri streaming timeout')), TAURI_STREAM_TIMEOUT_MS),
            ),
          ]);
        } finally {
          clearInterval(syncInterval);
        }

        if (!sendOk) throw new Error(streaming.error ?? 'Streaming failed');

        // Grace period for any in-flight events after invoke resolves.
        if (streaming.isStreaming) {
          const graceWait = 1_500;
          const start = Date.now();
          while (streaming.isStreaming && Date.now() - start < graceWait) {
            streamingText.value = streaming.streamText;
            await new Promise((r) => setTimeout(r, 50));
          }
        }

        isStreaming.value = false;
        streamingText.value = '';

        // Create the final assistant message from accumulated text.
        // Text is already clean (anim blocks stripped by Rust parser),
        // but the LLM may still return JSON-wrapped text outside tags.
        const parsed = parseTags(streaming.streamText);
        const cleanText = parsed.text;
        if (cleanText) {
          // Emotion comes from the streaming store (set by llm-animation events).
          const sentiment = streaming.currentEmotion ?? parsed.emotion ?? detectSentiment(content);
          const motion = streaming.currentMotion ?? parsed.motion ?? undefined;
          const { clean, warning } = extractWarning(cleanText);
          const assistantMsg: Message = {
            id: crypto.randomUUID(),
            role: 'assistant',
            content: clean || cleanText,
            agentName: 'TerranSoul',
            sentiment: sentiment as Message['sentiment'],
            timestamp: Date.now(),
            emoji: parsed.emoji ?? undefined,
            motion,
          };
          if (warning) applyWarningAsQuest(assistantMsg, warning);
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
          const FALLBACK_TIMEOUT_MS = 30_000;
          const response = await Promise.race([
            invoke<Message>('send_message', {
              message: content,
              agentId: currentAgent.value === 'auto' ? null : currentAgent.value,
            }),
            new Promise<never>((_, reject) =>
              setTimeout(() => reject(new Error('Fallback invoke timeout')), FALLBACK_TIMEOUT_MS),
            ),
          ]);
          messages.value.push(response);
          maybeShowQuestFromResponse(response.content, content);
          maybeShowDontKnowPrompt(response.content);
        } catch {
          messages.value.push(createPersonaResponse(content));
          pushNetworkOrProviderWarning();
        }
      } finally {
        isThinking.value = false;
        isStreaming.value = false;
        streamingText.value = '';
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

        // RAG: fetch relevant memories from Tauri backend if available
        let memoryBlock = '';
        try {
          const results = await invoke<{ id: number; content: string }[]>('search_memories', { query: content });
          if (results && results.length > 0) {
            const topMemories = results.slice(0, 5);
            memoryBlock = '\n\n[LONG-TERM MEMORY]\nThe following facts from your memory are relevant to this conversation:\n'
              + topMemories.map((m) => `- ${m.content}`).join('\n')
              + '\n[/LONG-TERM MEMORY]';
          }
        } catch {
          // Tauri unavailable (browser-only) or no memories — continue without RAG
        }

        // Try the primary provider, then rotate to next healthy on rate-limit
        const providersToTry = [provider];
        // Add fallback providers from the brain store
        const primaryProviderId = brain.brainMode?.mode === 'free_api' ? brain.brainMode.provider_id : '';
        for (const fp of brain.freeProviders) {
          if (fp.id !== primaryProviderId && !providersToTry.some((p) => p.baseUrl === fp.base_url)) {
            providersToTry.push({ baseUrl: fp.base_url, model: fp.model, apiKey: provider.apiKey });
          }
        }

        let succeeded = false;
        for (const prov of providersToTry) {
          const provId = brain.freeProviders.find((f) => f.base_url === prov.baseUrl)?.id ?? 'unknown';
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
                  onDone: (full) => { if (!settled) { settled = true; clearTimeout(timeout); resolve(full); } },
                  onError: (err) => { if (!settled) { settled = true; clearTimeout(timeout); reject(new Error(err)); } },
                },
                getSystemPrompt(useEnhanced) + usePersonaStore().personaBlock + memoryBlock,
              );
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
            if (warning) applyWarningAsQuest(assistantMsg, warning);
            messages.value.push(assistantMsg);
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
          messages.value.push(createPersonaResponse(content));
          pushNetworkOrProviderWarning();
        }
        } finally {
          isThinking.value = false;
          isStreaming.value = false;
          streamingText.value = '';
        }
        return;
      }
    }

    // Path 3: No brain configured — persona fallback
    await new Promise((r) => setTimeout(r, 500));
    messages.value.push(createPersonaResponse(content));
    isThinking.value = false;
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
    messages.value.push(message);
    trimHistory();
  }

  return {
    messages,
    currentAgent,
    isThinking,
    streamingText,
    isStreaming,
    sendMessage,
    getConversation,
    addMessage,
    pushProviderWarning,
    // Auto-learn surface (see docs/brain-advanced-design.md § 21)
    totalAssistantTurns,
    lastAutoLearnTurn,
    lastAutoLearnDecision,
    lastAutoLearnSavedCount,
  };
});
