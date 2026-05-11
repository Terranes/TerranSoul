import { defineStore } from 'pinia';
import { ref, computed, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useBrainStore } from './brain';
import { useVoiceStore } from './voice';
import { useSettingsStore } from './settings';
import { usePersonaStore } from './persona';
import { useConversationStore } from './conversation';
import { useMemoryStore } from './memory';
import { streamChatCompletion, type ChatMessage } from '../utils/free-api-client';
import type { QuestChoice } from '../types';
import { DEFAULT_THEME_ID } from '../config/themes';

// ── Skill Node Types ──────────────────────────────────────────────────────────

export type SkillTier = 'foundation' | 'advanced' | 'ultimate';
export type Platform = 'windows' | 'macos' | 'linux' | 'web' | 'mobile';

/** Get the current platform */
function getCurrentPlatform(): Platform {
  if (typeof window === 'undefined') return 'web';
  
  // Check if we're in Tauri (desktop app)
  if ((window as any).__TAURI_INTERNALS__) {
    const userAgent = navigator.userAgent.toLowerCase();
    if (userAgent.includes('windows')) return 'windows';
    if (userAgent.includes('mac')) return 'macos';
    if (userAgent.includes('linux')) return 'linux';
  }
  
  // Check for mobile
  if (/android|iphone|ipad|ipod|mobile/i.test(navigator.userAgent)) {
    return 'mobile';
  }
  
  return 'web';
}

export interface SkillNode {
  /** Unique identifier for this skill. */
  id: string;
  /** Display name shown in the tree. */
  name: string;
  /** Short tagline describing what it does. */
  tagline: string;
  /** Full description of the feature unlocked. */
  description: string;
  /** Gaming icon/emoji for the node. */
  icon: string;
  /** Tier in the tech tree (foundation → advanced → ultimate). */
  tier: SkillTier;
  /** IDs of prerequisite skills that must be unlocked first. */
  requires: string[];
  /** What rewards enabling this skill gives. */
  rewards: string[];
  /** Per-reward icon emojis — parallel array with `rewards`. */
  rewardIcons: string[];
  /** Steps the user needs to complete to unlock this skill. */
  questSteps: QuestStep[];
  /** Category grouping for layout. */
  category: 'brain' | 'voice' | 'avatar' | 'social' | 'utility';
  /** Combo skills unlocked when this + other skills are all active. */
  combos: ComboRef[];
  /** YouTube video ID showing this feature in action (optional). */
  videoRef?: string;
  /** Platform compatibility - if specified, only shows on these platforms */
  platforms?: Platform[];
  /** Whether this skill is recommended for easy setup */
  recommended?: boolean;
}

export interface QuestStep {
  /** Instruction text for this step. */
  label: string;
  /** Action type: 'navigate' opens a view, 'configure' is in-app, 'external' opens a link. */
  action: 'navigate' | 'configure' | 'external' | 'info';
  /** Navigation target tab id, config field, or URL. */
  target?: string;
}

export interface ComboRef {
  /** IDs of other skills needed for this combo. */
  withSkills: string[];
  /** Name of the combo. */
  name: string;
  /** Description of what the combo unlocks. */
  description: string;
  /** Combo icon. */
  icon: string;
}

/** Runtime status derived from actual store state. */
export type SkillStatus = 'locked' | 'available' | 'active';

// ── Quest Tracker Types ──────────────────────────────────────────────────────

/**
 * Persisted quest tracker data. Designed to survive fresh installs:
 * - Skill completion is DERIVED from actual feature state (brain, voice, etc.)
 *   and never stored here — reinstalling + re-configuring restores progress.
 * - Only user-specific tracker metadata (dismissed, pinned, timestamps)
 *   needs persistence.
 */
export interface QuestTrackerData {
  /** Schema version for migration safety. */
  version: number;
  /** IDs of quests the user has explicitly dismissed from daily suggestions. */
  dismissedQuestIds: string[];
  /** IDs of quests the user has pinned for tracking. */
  pinnedQuestIds: string[];
  /** ISO date string of the last daily suggestion refresh (e.g. '2026-04-17'). */
  lastSuggestionDate: string | null;
  /** Skill IDs from the last AI-generated daily suggestion. */
  dailySuggestionIds: string[];
  /** AI reasoning for the daily suggestions. */
  dailySuggestionReason: string | null;
  /** Timestamps of first skill activation (for progress timeline). */
  activationTimestamps: Record<string, number>;
  /** IDs of quests the user has manually marked as completed. */
  manuallyCompletedIds: string[];
  /** Combo keys the user has already been notified about (Chunk 131). */
  seenComboKeys: string[];
  /** Last activation timestamp the user has seen a reward ceremony for (Chunk 132). */
  lastSeenActivationTimestamp: number;
  /** Daily brief quest: cached brief result with ISO date. */
  dailyBrief: DailyBriefCache | null;
}

export interface DailySuggestion {
  node: SkillNode;
  status: SkillStatus;
  reason: string | null;
}

/** Cached daily brief result (one per day). */
export interface DailyBriefCache {
  /** ISO date of when this brief was fetched (e.g. '2026-05-06'). */
  date: string;
  /** Memory items surfaced in the brief. */
  items: DailyBriefItem[];
  /** Total memories found in the time range before filtering. */
  totalInRange: number;
}

export interface DailyBriefItem {
  id: number;
  content: string;
  tags: string;
  importance: number;
  createdAt: number;
}

const CURRENT_VERSION = 1;

function freshTracker(): QuestTrackerData {
  return {
    version: CURRENT_VERSION,
    dismissedQuestIds: [],
    pinnedQuestIds: [],
    lastSuggestionDate: null,
    dailySuggestionIds: [],
    dailySuggestionReason: null,
    activationTimestamps: {},
    manuallyCompletedIds: [],
    seenComboKeys: [],
    lastSeenActivationTimestamp: 0,
    dailyBrief: null,
  };
}

/**
 * Safely coerce any parsed object into a valid QuestTrackerData.
 * Preserves as many fields as possible from older / unknown versions
 * instead of discarding everything.
 */
function migrateTracker(raw: unknown): QuestTrackerData {
  if (!raw || typeof raw !== 'object') return freshTracker();
  const obj = raw as Record<string, unknown>;
  const base = freshTracker();
  if (Array.isArray(obj.dismissedQuestIds))
    base.dismissedQuestIds = obj.dismissedQuestIds.filter((id): id is string => typeof id === 'string');
  if (Array.isArray(obj.pinnedQuestIds))
    base.pinnedQuestIds = obj.pinnedQuestIds.filter((id): id is string => typeof id === 'string');
  if (typeof obj.lastSuggestionDate === 'string')
    base.lastSuggestionDate = obj.lastSuggestionDate;
  if (Array.isArray(obj.dailySuggestionIds))
    base.dailySuggestionIds = obj.dailySuggestionIds.filter((id): id is string => typeof id === 'string');
  if (typeof obj.dailySuggestionReason === 'string')
    base.dailySuggestionReason = obj.dailySuggestionReason;
  if (obj.activationTimestamps && typeof obj.activationTimestamps === 'object')
    base.activationTimestamps = Object.fromEntries(
      Object.entries(obj.activationTimestamps as Record<string, unknown>)
        .filter(([, v]) => typeof v === 'number'),
    ) as Record<string, number>;
  if (Array.isArray(obj.manuallyCompletedIds))
    base.manuallyCompletedIds = obj.manuallyCompletedIds.filter((id): id is string => typeof id === 'string');
  if (Array.isArray(obj.seenComboKeys))
    base.seenComboKeys = obj.seenComboKeys.filter((id): id is string => typeof id === 'string');
  if (typeof obj.lastSeenActivationTimestamp === 'number')
    base.lastSeenActivationTimestamp = obj.lastSeenActivationTimestamp;
  if (obj.dailyBrief && typeof obj.dailyBrief === 'object')
    base.dailyBrief = obj.dailyBrief as DailyBriefCache;
  return base;
}

/**
 * Merge two tracker snapshots, keeping the superset of user-curated data.
 * Prefers the tracker with **more** activation timestamps (most progress),
 * and takes the union of dismissed / pinned lists.
 */
function mergeTrackers(a: QuestTrackerData, b: QuestTrackerData): QuestTrackerData {
  const merged = freshTracker();
  merged.dismissedQuestIds = [...new Set([...a.dismissedQuestIds, ...b.dismissedQuestIds])];
  merged.pinnedQuestIds = [...new Set([...a.pinnedQuestIds, ...b.pinnedQuestIds])];
  merged.manuallyCompletedIds = [...new Set([...a.manuallyCompletedIds, ...b.manuallyCompletedIds])];
  merged.seenComboKeys = [...new Set([...a.seenComboKeys, ...b.seenComboKeys])];
  merged.lastSeenActivationTimestamp = Math.max(a.lastSeenActivationTimestamp || 0, b.lastSeenActivationTimestamp || 0);
  // Keep the latest suggestion set
  if ((a.lastSuggestionDate ?? '') >= (b.lastSuggestionDate ?? '')) {
    merged.lastSuggestionDate = a.lastSuggestionDate;
    merged.dailySuggestionIds = a.dailySuggestionIds;
    merged.dailySuggestionReason = a.dailySuggestionReason;
  } else {
    merged.lastSuggestionDate = b.lastSuggestionDate;
    merged.dailySuggestionIds = b.dailySuggestionIds;
    merged.dailySuggestionReason = b.dailySuggestionReason;
  }
  // Merge timestamps — keep the earliest (first activation)
  const allKeys = new Set([...Object.keys(a.activationTimestamps), ...Object.keys(b.activationTimestamps)]);
  for (const key of allKeys) {
    const ta = a.activationTimestamps[key] ?? Infinity;
    const tb = b.activationTimestamps[key] ?? Infinity;
    merged.activationTimestamps[key] = Math.min(ta, tb);
  }
  // Keep the most recent daily brief
  const briefA = a.dailyBrief;
  const briefB = b.dailyBrief;
  if (briefA && briefB) {
    merged.dailyBrief = (briefA.date >= briefB.date) ? briefA : briefB;
  } else {
    merged.dailyBrief = briefA ?? briefB ?? null;
  }
  return merged;
}

const STORAGE_KEY = 'terransoul-quest-tracker';

// ── Skill Catalogue ──────────────────────────────────────────────────────────

const SKILL_NODES: SkillNode[] = [
  // ── FOUNDATION TIER ─────────────────────────────────────────────────────
  {
    id: 'free-brain',
    name: 'Awaken the Mind',
    tagline: 'Connect to a free cloud AI',
    description: 'Connects a free-tier LLM provider such as OpenRouter, Gemini, NVIDIA NIM, or Pollinations with your own key/token. This is the starting point for all intelligence.',
    icon: '🧠',
    tier: 'foundation',
    requires: [],
    rewards: ['Real-time AI chat', 'Emotion-tagged responses', 'Sentiment-based avatar reactions'],
    rewardIcons: ['💬', '🎭', '😊'],
    questSteps: [
      { label: 'The free brain auto-configures on first launch!', action: 'info' },
      { label: 'Open Brain Setup to verify', action: 'navigate', target: 'brain-setup' },
    ],
    category: 'brain',
    combos: [
      {
        withSkills: ['avatar'],
        name: 'First Light',
        description: 'Free brain + avatar = a chatty 3D companion the moment you open the app.',
        icon: '🌅',
      },
    ],
    videoRef: 'dQw4w9WgXcQ',
  },
  {
    id: 'tts',
    name: 'Gift of Speech',
    tagline: 'Let your companion speak aloud',
    description: 'Edge TTS (free Microsoft neural voices) gives your character a voice. You\'ll hear responses spoken with personality-matched pitch and speed.',
    icon: '🗣️',
    tier: 'foundation',
    requires: [],
    rewards: ['Character speaks responses aloud', 'Gender-matched voice (cute/deep)', 'Lip-sync animation'],
    rewardIcons: ['🔊', '👧', '👄'],
    questSteps: [
      { label: 'Go to Voice Setup', action: 'navigate', target: 'voice' },
      { label: 'Select "Edge TTS" as your TTS provider', action: 'configure', target: 'tts_provider' },
    ],
    category: 'voice',
    combos: [
      {
        withSkills: ['avatar'],
        name: 'Living Doll',
        description: 'TTS + avatar = lip-synced spoken responses from your 3D companion.',
        icon: '🎙️',
      },
    ],
    videoRef: 'dQw4w9WgXcQ',
  },
  {
    id: 'avatar',
    name: 'Summon Avatar',
    tagline: 'Bring your 3D companion to life',
    description: 'Load a VRM model to see your anime companion on screen. They react to emotions, idle-animate, and lip-sync to speech.',
    icon: '✨',
    tier: 'foundation',
    requires: [],
    rewards: ['3D anime avatar on screen', 'Idle animations (breathing, blinking)', 'Emotion-driven expressions'],
    rewardIcons: ['🧍', '💨', '😄'],
    questSteps: [
      { label: 'An avatar loads automatically with the default character!', action: 'info' },
      { label: 'Switch characters from the chat view model panel', action: 'navigate', target: 'chat' },
    ],
    category: 'avatar',
    combos: [
      {
        withSkills: ['free-brain', 'tts'],
        name: 'Companion Awake',
        description: 'Avatar + brain + voice = the full talking, expressive companion experience.',
        icon: '💫',
      },
    ],
    videoRef: 'dQw4w9WgXcQ',
  },
  {
    id: 'bgm',
    name: 'Ambient Aura',
    tagline: 'Enable background music',
    description: 'Original JRPG-style ambient tracks (Crystal Theme, Starlit Village, Eternity) fill the silence with warm atmosphere.',
    icon: '🎵',
    tier: 'foundation',
    requires: [],
    rewards: ['Ambient background music', '3 built-in tracks', 'Volume control'],
    rewardIcons: ['🎶', '🎹', '🔉'],
    questSteps: [
      { label: 'Open Settings (gear icon in chat view)', action: 'info' },
      { label: 'Toggle BGM on and pick a track', action: 'configure', target: 'bgm_enabled' },
    ],
    category: 'utility',
    combos: [],
    videoRef: 'dQw4w9WgXcQ',
  },

  // ── BGM CHAIN (advanced / ultimate) ─────────────────────────────────────
  {
    id: 'bgm-custom',
    name: 'Jukebox',
    tagline: 'Add your own music tracks',
    description: 'Import custom audio files or paste URLs to build your personal playlist. Your companion\'s world, your soundtrack.',
    icon: '📀',
    tier: 'advanced',
    requires: ['bgm'],
    rewards: ['Import local audio files', 'Add tracks from URL', 'Delete custom tracks', 'Persistent playlist'],
    rewardIcons: ['📁', '🔗', '🗑️', '💾'],
    questSteps: [
      { label: 'Open the floating music bar (bottom of chat view)', action: 'info' },
      { label: 'Click the + button to add a file or URL', action: 'configure', target: 'bgm_custom' },
      { label: 'Play your custom track!', action: 'info' },
    ],
    category: 'utility',
    combos: [
      {
        withSkills: ['tts'],
        name: 'DJ Companion',
        description: 'Custom BGM + TTS = your companion narrates over your personal soundtrack.',
        icon: '🎧',
      },
    ],
  },
  {
    id: 'bgm-video',
    name: 'Watch Party',
    tagline: 'Watch videos with your companion',
    description: 'Paste a video URL and watch it together with your AI companion. They can react, comment, and discuss what you\'re watching!',
    icon: '🎬',
    tier: 'ultimate',
    requires: ['bgm-custom', 'free-brain'],
    rewards: ['Video playback in-app', 'Companion reacts to videos', 'Shared watch experience', 'Video discussion in chat'],
    rewardIcons: ['▶️', '😮', '👫', '💬'],
    questSteps: [
      { label: 'Open the floating music bar', action: 'info' },
      { label: 'Click + and paste a video URL (YouTube, etc.)', action: 'configure', target: 'bgm_video' },
      { label: 'Ask your companion about the video in chat!', action: 'info' },
    ],
    category: 'utility',
    combos: [
      {
        withSkills: ['paid-brain'],
        name: 'Film Critic',
        description: 'Watch Party + Premium AI = deeper analysis and personalized commentary on videos.',
        icon: '🎥',
      },
      {
        withSkills: ['tts'],
        name: 'Movie Night',
        description: 'Watch Party + TTS = your companion narrates reactions out loud during the video.',
        icon: '🍿',
      },
    ],
  },

  // ── ADVANCED TIER ──────────────────────────────────────────────────────
  {
    id: 'migrate-brain',
    name: 'Evolve Beyond',
    tagline: 'Migrate from the deprecated Pollinations API',
    description: 'The Pollinations legacy text API is being deprecated. Upgrade to a paid API (OpenAI, Anthropic, Groq) or run locally with llmfit before service ends.',
    icon: '⚠️',
    tier: 'advanced',
    requires: ['free-brain'],
    rewards: ['Future-proof AI connection', 'Better reliability', 'Faster responses', 'No deprecation risk'],
    rewardIcons: ['🛡️', '🔄', '⚡', '✅'],
    questSteps: [
      { label: 'Open Brain Setup', action: 'navigate', target: 'brain-setup' },
      { label: 'Choose a new provider: Paid API or use llmfit for local AI', action: 'configure', target: 'brain_mode' },
      { label: 'Verify your companion still responds in chat', action: 'navigate', target: 'chat' },
    ],
    category: 'brain',
    combos: [],
  },
  {
    id: 'asr',
    name: 'Voice Command',
    tagline: 'Talk to your companion with your mic',
    description: 'Speech-to-text lets you speak instead of type. Web Speech API works instantly in browser. Upgrade to Whisper for much better accuracy.',
    icon: '🎤',
    tier: 'advanced',
    requires: ['free-brain'],
    rewards: ['Microphone input button', 'Hands-free conversation', 'Real-time transcription'],
    rewardIcons: ['🎙️', '🙌', '📝'],
    questSteps: [
      { label: 'Go to Voice Setup', action: 'navigate', target: 'voice' },
      { label: 'Select an ASR provider (Web Speech = instant)', action: 'configure', target: 'asr_provider' },
    ],
    category: 'voice',
    combos: [
      {
        withSkills: ['tts'],
        name: 'Full Conversation',
        description: 'Talk AND listen — hands-free two-way dialogue with your companion!',
        icon: '💬',
      },
    ],
  },
  {
    id: 'paid-brain',
    name: 'Superior Intellect',
    tagline: 'Connect a paid LLM API',
    description: 'Use OpenAI, Anthropic, or any OpenAI-compatible API for premium intelligence. Faster responses, better reasoning, longer context.',
    icon: '⚡',
    tier: 'advanced',
    requires: ['free-brain'],
    rewards: ['Premium LLM quality', 'Faster response times', 'Longer context window', 'Better emotion detection'],
    rewardIcons: ['👑', '⚡', '📚', '🎯'],
    questSteps: [
      { label: 'Get an API key from your preferred provider', action: 'external', target: 'https://platform.openai.com/api-keys' },
      { label: 'Open Brain Setup', action: 'navigate', target: 'brain-setup' },
      { label: 'Choose "Paid API" tier and enter your key', action: 'configure', target: 'brain_mode' },
    ],
    category: 'brain',
    combos: [
      {
        withSkills: ['memory'],
        name: 'True Recall',
        description: 'Semantic memory search powered by a premium brain — your companion truly remembers you.',
        icon: '🔮',
      },
    ],
  },
  {
    id: 'local-brain',
    name: 'Inner Sanctum',
    tagline: 'Run AI locally with Ollama',
    description: 'Full privacy — no data leaves your machine. Install Ollama, then let TerranSoul recommend the best local model for your hardware. Works offline with 8GB+ RAM.',
    icon: '🏰',
    tier: 'advanced',
    requires: ['free-brain'],
    rewards: ['100% private AI', 'No API costs', 'Offline capability', 'Unlimited usage'],
    rewardIcons: ['🔒', '💰', '✈️', '♾️'],
    questSteps: [
      { label: 'Install Ollama from ollama.com', action: 'external', target: 'https://ollama.com/download' },
      { label: 'Open Brain Setup and choose "Local LLM (Ollama)"', action: 'navigate', target: 'brain-setup' },
      { label: 'Pick a recommended model and click Download', action: 'configure', target: 'brain_mode' },
      { label: 'Send a test message in Chat to verify it works', action: 'navigate', target: 'chat' },
    ],
    category: 'brain',
    recommended: true,
    combos: [
      {
        withSkills: ['memory'],
        name: 'Offline Sage',
        description: 'Full memory + local AI — your companion works completely offline with persistent memory.',
        icon: '📜',
      },
    ],
  },
  {
    id: 'memory',
    name: 'Long-Term Memory',
    tagline: 'Your companion remembers you',
    description: 'Store facts, preferences, and context that persist across conversations. Requires Tauri desktop app for SQLite storage.',
    icon: '📖',
    tier: 'advanced',
    requires: ['free-brain'],
    rewards: ['Persistent memory across sessions', 'Fact/preference/context storage', 'Memory graph visualization'],
    rewardIcons: ['💾', '📋', '🕸️'],
    questSteps: [
      { label: 'Run TerranSoul as a desktop app (Tauri)', action: 'info' },
      { label: 'Open the Memory tab to manage memories', action: 'navigate', target: 'memory' },
      { label: 'Add your first memory manually or let AI extract them', action: 'configure', target: 'memory_add' },
    ],
    category: 'brain',
    combos: [
      {
        withSkills: ['presence'],
        name: 'Welcome Home',
        description: 'Memory + presence = your companion remembers what you were doing last time and picks up where you left off.',
        icon: '🏡',
      },
    ],
  },
  {
    id: 'whisper-asr',
    name: 'Dragon\'s Ear',
    tagline: 'High-accuracy cloud transcription',
    description: 'Upgrade from Web Speech to Groq Whisper or OpenAI Whisper for far better accuracy, especially with accents and technical terms.',
    icon: '🐉',
    tier: 'advanced',
    requires: ['asr'],
    rewards: ['95%+ transcription accuracy', 'Multi-accent support', 'Technical vocabulary'],
    rewardIcons: ['🎯', '🌐', '📖'],
    questSteps: [
      { label: 'Get a Groq API key (generous free tier)', action: 'external', target: 'https://console.groq.com/keys' },
      { label: 'Go to Voice Setup', action: 'navigate', target: 'voice' },
      { label: 'Select "Groq Whisper" as ASR provider and enter key', action: 'configure', target: 'asr_provider' },
    ],
    category: 'voice',
    combos: [
      {
        withSkills: ['hotwords'],
        name: 'Perfect Hearing',
        description: 'Whisper accuracy + custom hotwords = your companion never mishears key phrases.',
        icon: '👂',
      },
    ],
  },
  {
    id: 'hotwords',
    name: 'Power Words',
    tagline: 'Boost recognition of custom phrases',
    description: 'Configure hotwords so the ASR engine prioritizes your character names, product names, or custom terms.',
    icon: '🔤',
    tier: 'advanced',
    requires: ['asr'],
    rewards: ['Custom keyword boosting', 'Better name recognition', 'Fewer transcription errors'],
    rewardIcons: ['🔑', '🏷️', '✅'],
    questSteps: [
      { label: 'Go to Voice Setup', action: 'navigate', target: 'voice' },
      { label: 'Add hotwords in the Hotwords section', action: 'configure', target: 'hotwords' },
    ],
    category: 'voice',
    combos: [],
  },
  {
    id: 'presence',
    name: 'Sixth Sense',
    tagline: 'Companion detects when you return',
    description: 'When you leave and come back, your companion notices and greets you based on how long you were away. No setup needed — just works!',
    icon: '👁️',
    tier: 'advanced',
    requires: ['free-brain', 'tts'],
    rewards: ['Contextual return greetings', 'Away-duration awareness', '"Welcome back!" interactions'],
    rewardIcons: ['👋', '⏱️', '💕'],
    questSteps: [
      { label: 'This activates automatically when Brain + TTS are both enabled!', action: 'info' },
    ],
    category: 'utility',
    combos: [],
  },

  // ── ULTIMATE TIER ──────────────────────────────────────────────────────
  {
    id: 'device-link',
    name: 'Soul Link',
    tagline: 'Sync across multiple devices',
    description: 'Pair your desktop and mobile via QR code. Conversations, character selection, and settings sync in real-time via encrypted peer-to-peer.',
    icon: '🔗',
    tier: 'ultimate',
    requires: ['free-brain', 'memory'],
    rewards: ['Multi-device sync', 'QR code pairing', 'Encrypted P2P connection', 'Remote command routing'],
    rewardIcons: ['📱', '📷', '🛡️', '🎮'],
    questSteps: [
      { label: 'Run TerranSoul desktop app (Tauri required)', action: 'info' },
      { label: 'Open a second device and scan the QR code', action: 'navigate', target: 'chat' },
    ],
    category: 'social',
    combos: [],
  },
  {
    id: 'agents',
    name: 'Agent Summoning',
    tagline: 'Install community AI agents',
    description: 'Browse and install WASM agents from the marketplace. Each agent runs sandboxed with capability gates you control.',
    icon: '🤖',
    tier: 'ultimate',
    requires: ['paid-brain'],
    rewards: ['Community agent marketplace', 'Sandboxed execution', 'Capability-gated permissions', 'Agent-to-agent messaging'],
    rewardIcons: ['🛒', '📦', '🔐', '💌'],
    questSteps: [
      { label: 'Run TerranSoul desktop app (Tauri required)', action: 'info' },
      { label: 'Open the Marketplace tab', action: 'navigate', target: 'marketplace' },
      { label: 'Search and install an agent', action: 'configure', target: 'agent_install' },
    ],
    category: 'social',
    combos: [
      {
        withSkills: ['device-link'],
        name: 'Hive Mind',
        description: 'Agents + Device Link = agents run on desktop but respond to commands from your phone.',
        icon: '🌐',
      },
    ],
  },
  {
    id: 'diarization',
    name: 'Voice Splitter',
    tagline: 'Identify who is speaking',
    description: 'Speaker diarization separates voices in a conversation — your companion knows which person said what.',
    icon: '🎭',
    tier: 'ultimate',
    requires: ['whisper-asr'],
    rewards: ['Multi-speaker identification', 'Per-speaker transcripts', 'Group conversation support'],
    rewardIcons: ['🎭', '📃', '👥'],
    questSteps: [
      { label: 'Run TerranSoul desktop app (Tauri required)', action: 'info' },
      { label: 'Enable diarization in Voice Setup', action: 'navigate', target: 'voice' },
    ],
    category: 'voice',
    combos: [
      {
        withSkills: ['memory'],
        name: 'Social Memory',
        description: 'Diarization + Memory = your companion remembers what each person said across sessions.',
        icon: '👥',
      },
    ],
  },
  {
    id: 'translation',
    name: 'Babel Tongue',
    tagline: 'Real-time translation',
    description: 'Your companion can translate between languages in real-time during conversation.',
    icon: '🌍',
    tier: 'ultimate',
    requires: ['paid-brain', 'tts'],
    rewards: ['Real-time text translation', 'Multi-language support', 'Language auto-detection'],
    rewardIcons: ['🌍', '🗣️', '🔍'],
    questSteps: [
      { label: 'Run TerranSoul desktop app (Tauri required)', action: 'info' },
      { label: 'Translation activates automatically with a paid brain + TTS', action: 'info' },
    ],
    category: 'utility',
    combos: [
      {
        withSkills: ['asr'],
        name: 'Universal Translator',
        description: 'Speak in any language → companion translates and responds in your target language.',
        icon: '🗺️',
      },
    ],
  },
  {
    id: 'vision',
    name: 'All-Seeing Eye',
    tagline: 'Screen capture & vision analysis',
    description: 'Your companion can see your screen and understand what you\'re doing. Requires a vision-capable AI backend.',
    icon: '📸',
    tier: 'ultimate',
    requires: ['paid-brain'],
    rewards: ['Screen capture', 'Activity understanding', 'Visual context in conversations'],
    rewardIcons: ['📷', '🧠', '👁️'],
    questSteps: [
      { label: 'Run TerranSoul desktop app (Tauri required)', action: 'info' },
      { label: 'Connect a vision-capable LLM (GPT-4V, etc.)', action: 'configure', target: 'brain_mode' },
    ],
    category: 'brain',
    combos: [
      {
        withSkills: ['memory', 'asr'],
        name: 'Omniscient Companion',
        description: 'Vision + Memory + Voice = a companion that sees, hears, and remembers everything.',
        icon: '🌟',
      },
    ],
  },
  {
    id: 'pet-mode',
    name: 'Desktop Familiar',
    tagline: 'Floating companion for desktop and web',
    description: 'Your character breaks free of the normal chat layout: a transparent desktop overlay in Tauri, or a browser pet preview on the web.',
    icon: '🐾',
    tier: 'ultimate',
    requires: ['avatar', 'tts'],
    rewards: ['Transparent desktop overlay', 'Browser pet preview', 'Always-visible companion', 'Chat from any application'],
    rewardIcons: ['🪟', '🌐', '📌', '💬'],
    questSteps: [
      { label: 'Open Pet Mode from the paw button or browser pet preview', action: 'navigate', target: 'pet-mode' },
      { label: 'For transparent desktop overlay mode, run TerranSoul as the Tauri desktop app', action: 'info' },
    ],
    category: 'avatar',
    combos: [
      {
        withSkills: ['asr', 'presence'],
        name: 'Living Desktop Pet',
        description: 'Pet Mode + Voice + Presence = a desktop companion that listens and greets you throughout your day.',
        icon: '🦊',
      },
    ],
    platforms: ['windows', 'macos', 'linux', 'web'],
  },
  
  // ── WINDOWS-SPECIFIC FEATURES ──────────────────────────────────────────
  {
    id: 'windows-notifications',
    name: 'Windows Alerts',
    tagline: 'Native Windows notifications',
    description: 'Your companion can send Windows Toast notifications for important messages, reminders, or when you return to your desk.',
    icon: '🪟',
    tier: 'advanced',
    requires: ['free-brain'],
    rewards: ['Native Windows notifications', 'Toast messages', 'System tray alerts'],
    rewardIcons: ['🔔', '💬', '📱'],
    questSteps: [
      { label: 'Run TerranSoul desktop app on Windows', action: 'info' },
      { label: 'Grant notification permissions in Windows Settings', action: 'external', target: 'ms-settings:notifications' },
    ],
    category: 'utility',
    combos: [
      {
        withSkills: ['presence'],
        name: 'Smart Alerts',
        description: 'Windows notifications + presence detection = get notified only when you need it.',
        icon: '🧠',
      },
    ],
    platforms: ['windows'],
  },
  {
    id: 'windows-shortcuts',
    name: 'Power User',
    tagline: 'Global Windows keyboard shortcuts',
    description: 'Set up global hotkeys to quickly talk to your companion from any app. Press Ctrl+Shift+T to instantly open chat.',
    icon: '⌨️',
    tier: 'advanced',
    requires: ['free-brain', 'asr'],
    rewards: ['Global keyboard shortcuts', 'Quick access from any app', 'Customizable hotkeys'],
    rewardIcons: ['🔥', '⚡', '🎛️'],
    questSteps: [
      { label: 'Enable global shortcuts in TerranSoul settings', action: 'configure', target: 'shortcuts' },
      { label: 'Test your shortcut: Press Ctrl+Shift+T anywhere', action: 'info' },
    ],
    category: 'utility',
    combos: [
      {
        withSkills: ['pet-mode'],
        name: 'Instant Companion',
        description: 'Shortcuts + Pet Mode = summon your companion instantly while working.',
        icon: '⚡',
      },
    ],
    platforms: ['windows'],
  },
  {
    id: 'windows-taskbar',
    name: 'System Integration',
    tagline: 'Windows taskbar & system tray',
    description: 'TerranSoul integrates deeply with Windows - progress in taskbar, rich system tray menu, and native Windows UI.',
    icon: '🏗️',
    tier: 'foundation',
    requires: [],
    rewards: ['Taskbar progress indicators', 'Rich system tray menu', 'Native Windows feel'],
    rewardIcons: ['📊', '📋', '🎯'],
    questSteps: [
      { label: 'This activates automatically on Windows desktop!', action: 'info' },
      { label: 'Check your system tray (bottom right) for the TerranSoul icon', action: 'info' },
    ],
    category: 'utility',
    combos: [],
    platforms: ['windows'],
  },
  {
    id: 'windows-startup',
    name: 'Boot Companion',
    tagline: 'Start with Windows',
    description: 'Your companion starts automatically when Windows boots. Never lose your digital friend!',
    icon: '🚀',
    tier: 'advanced',
    requires: ['windows-taskbar'],
    rewards: ['Auto-start on Windows boot', 'Background operation', 'Always available'],
    rewardIcons: ['🔄', '🌙', '♾️'],
    questSteps: [
      { label: 'Enable "Start with Windows" in TerranSoul settings', action: 'configure', target: 'startup' },
      { label: 'Restart Windows to test auto-start', action: 'info' },
    ],
    category: 'utility',
    combos: [
      {
        withSkills: ['pet-mode', 'presence'],
        name: 'Always There',
        description: 'Auto-start + Pet Mode + Presence = your companion is always with you.',
        icon: '💝',
      },
    ],
    platforms: ['windows'],
  },

  // ── EXPANDED BRAIN NODES ────────────────────────────────────────────────
  {
    id: 'rag-knowledge',
    name: 'Sage\'s Library',
    tagline: 'Local semantic-search RAG',
    description: 'Your companion uses the brain\'s LLM to semantically search your memories and inject relevant context into every conversation. No external services needed — works entirely with your local Ollama model or cloud-routed model like Kimi K2.6.',
    icon: '📚',
    tier: 'advanced',
    requires: ['memory'],
    rewards: ['Semantic memory retrieval', 'Context-aware answers', 'Automatic RAG in every chat'],
    rewardIcons: ['🔍', '🧠', '💬'],
    questSteps: [
      { label: 'Open the Memory tab', action: 'navigate', target: 'memory' },
      { label: 'Add at least one memory (fact, preference, or context)', action: 'configure', target: 'memory_add' },
      { label: 'Ask your companion something related to your stored memory', action: 'navigate', target: 'chat' },
    ],
    category: 'brain',
    recommended: true,
    combos: [
      {
        withSkills: ['paid-brain'],
        name: 'Scholar\'s Gambit',
        description: 'RAG + premium brain = research-grade answers with citations from your private library.',
        icon: '🎓',
      },
      {
        withSkills: ['vision'],
        name: 'Visual Encyclopedia',
        description: 'RAG + vision = your companion can read what\'s on screen and cross-reference your library.',
        icon: '🔎',
      },
    ],
  },
  {
    id: 'scholar-quest',
    name: 'Scholar\'s Quest',
    tagline: 'Deep-dive into any topic with guided knowledge ingestion',
    description: 'Start a chain quest to teach your companion about a specific topic. Provide URLs and files — TerranSoul will ingest, chunk, embed, and learn them for RAG-augmented answers.',
    icon: '📚',
    tier: 'advanced',
    requires: ['rag-knowledge'],
    rewards: ['Topic-specific RAG answers', 'Source-grounded knowledge', 'Persistent expert memory', 'Chain quest experience'],
    rewardIcons: ['🎯', '📖', '🧠', '⚔️'],
    questSteps: [
      { label: 'Add URLs or files about the topic', action: 'configure', target: 'knowledge_sources' },
      { label: 'Wait for ingestion and embedding', action: 'info' },
      { label: 'Ask questions to test your new knowledge', action: 'navigate', target: 'chat' },
    ],
    category: 'brain',
    recommended: true,
    combos: [
      {
        withSkills: ['paid-brain'],
        name: 'Master Scholar',
        description: 'Scholar\'s Quest + premium brain = research-grade deep dives with citations.',
        icon: '🎓',
      },
    ],
  },
  {
    id: 'quest-daily-brief',
    name: 'Morning Report',
    tagline: 'Daily memory digest of commitments and deadlines',
    description: 'Once per day, your companion searches recent memories for overdue tasks, upcoming events, and commitments you\'ve made. The brief surfaces in the skill-tree UI so you never lose track of what matters.',
    icon: '📋',
    tier: 'advanced',
    requires: ['rag-knowledge'],
    rewards: ['Daily commitment reminder', 'Overdue task surfacing', 'Upcoming event digest', 'Temporal memory awareness'],
    rewardIcons: ['⏰', '⚠️', '📅', '🧠'],
    questSteps: [
      { label: 'Ensure brain and memories are configured', action: 'info' },
      { label: 'Add at least one memory containing a commitment or deadline', action: 'navigate', target: 'memory' },
      { label: 'Open the Skill Tree and check the Daily Brief card', action: 'navigate', target: 'quest' },
    ],
    category: 'brain',
    recommended: true,
    combos: [
      {
        withSkills: ['tts'],
        name: 'Voice Briefing',
        description: 'Daily Brief + TTS = your companion reads your morning report aloud.',
        icon: '🎙️',
      },
    ],
  },
  {
    id: 'multi-agent',
    name: 'Council of Minds',
    tagline: 'Orchestrate multiple AI agents in parallel',
    description: 'Run several agents at once and let them collaborate on tasks — debate, plan, execute. A board of advisors at your beck and call.',
    icon: '⚖️',
    tier: 'ultimate',
    requires: ['agents', 'paid-brain'],
    rewards: ['Parallel agent execution', 'Inter-agent debate', 'Plan-and-execute pipelines', 'Role-playing committees'],
    rewardIcons: ['⚙️', '🗣️', '📋', '🎭'],
    questSteps: [
      { label: 'Run TerranSoul desktop app (Tauri required)', action: 'info' },
      { label: 'Open the Marketplace tab and install at least two agents', action: 'navigate', target: 'marketplace' },
      { label: 'Activate Council Mode in agent settings', action: 'configure', target: 'agent_install' },
    ],
    category: 'brain',
    combos: [
      {
        withSkills: ['memory'],
        name: 'Round Table',
        description: 'Council + Memory = persistent debate history across sessions.',
        icon: '🛡️',
      },
    ],
  },
  {
    id: 'dream-journal',
    name: 'Dream Journal',
    tagline: 'Auto-summarized session diary',
    description: 'After each long conversation, your companion writes a private journal entry summarizing what you talked about, what you decided, and what to follow up on.',
    icon: '🌙',
    tier: 'ultimate',
    requires: ['memory', 'free-brain'],
    rewards: ['Auto-generated session summaries', 'Searchable diary', 'Daily/weekly digests'],
    rewardIcons: ['📓', '🗓️', '📰'],
    questSteps: [
      { label: 'Open the Memory tab', action: 'navigate', target: 'memory' },
      { label: 'Enable the Dream Journal toggle', action: 'configure', target: 'memory_add' },
      { label: 'Have a conversation, then check the journal next session', action: 'navigate', target: 'chat' },
    ],
    category: 'brain',
    combos: [],
  },

  // ── EXPANDED VOICE NODES ────────────────────────────────────────────────
  {
    id: 'voice-emotion',
    name: 'Heart of the Voice',
    tagline: 'Emotion-modulated TTS',
    description: 'Your companion\'s spoken voice changes pitch, pace, and warmth based on the detected emotion of each line — happy lines sparkle, sad lines slow down.',
    icon: '💗',
    tier: 'advanced',
    requires: ['tts'],
    rewards: ['Emotion-aware speech synthesis', 'Dynamic pitch & tempo', 'More natural delivery'],
    rewardIcons: ['🎚️', '🎼', '🎙️'],
    questSteps: [
      { label: 'Open Voice Setup', action: 'navigate', target: 'voice' },
      { label: 'Enable "Emotion-aware TTS" in advanced settings', action: 'configure', target: 'tts_provider' },
    ],
    category: 'voice',
    combos: [
      {
        withSkills: ['avatar'],
        name: 'Soul Performance',
        description: 'Emotional voice + avatar = synchronized facial expression and tone.',
        icon: '🎭',
      },
    ],
  },
  {
    id: 'wake-word',
    name: 'Summoning Word',
    tagline: 'Activate by speaking your companion\'s name',
    description: 'A lightweight wake-word detector listens in the background. Say your companion\'s name and they wake up — no button press needed.',
    icon: '🔔',
    tier: 'advanced',
    requires: ['asr'],
    rewards: ['Always-listening wake word', 'Custom trigger phrase', 'Battery-friendly background detection'],
    rewardIcons: ['👂', '🏷️', '🔋'],
    questSteps: [
      { label: 'Go to Voice Setup', action: 'navigate', target: 'voice' },
      { label: 'Enable Wake Word and choose a trigger phrase', action: 'configure', target: 'asr_provider' },
      { label: 'Try saying the trigger from across the room', action: 'info' },
    ],
    category: 'voice',
    combos: [
      {
        withSkills: ['tts'],
        name: 'Hey, Companion!',
        description: 'Wake word + TTS = a true voice assistant — say their name and they answer back.',
        icon: '📣',
      },
    ],
  },
  {
    id: 'voice-clone',
    name: 'Mirrored Voice',
    tagline: 'Clone a voice from a short sample',
    description: 'Provide a few seconds of clean audio and your companion will speak with that voice. Use only voices you have rights to.',
    icon: '🪞',
    tier: 'ultimate',
    requires: ['tts', 'paid-brain'],
    rewards: ['Few-shot voice cloning', 'Custom voice profiles', 'Per-character voices'],
    rewardIcons: ['🧬', '🎤', '👥'],
    questSteps: [
      { label: 'Open Voice Setup', action: 'navigate', target: 'voice' },
      { label: 'Upload a short voice sample (with consent of the speaker!)', action: 'configure', target: 'tts_provider' },
      { label: 'Save the new voice profile and assign it to your companion', action: 'configure', target: 'tts_provider' },
    ],
    category: 'voice',
    combos: [
      {
        withSkills: ['voice-emotion'],
        name: 'Living Voice',
        description: 'Cloned voice + emotion modulation = a custom voice that truly emotes.',
        icon: '🌹',
      },
    ],
  },
  {
    id: 'singing',
    name: 'Songweaver',
    tagline: 'Your companion sings along',
    description: 'When BGM is playing, your companion can hum or sing along to the rhythm — even improvise short verses about the moment.',
    icon: '🎶',
    tier: 'ultimate',
    requires: ['tts', 'bgm'],
    rewards: ['Rhythm-aware vocal synthesis', 'Improv lyrics', 'Hum-along mode'],
    rewardIcons: ['🥁', '✍️', '🎵'],
    questSteps: [
      { label: 'Start BGM from the floating music bar', action: 'info' },
      { label: 'Enable "Sing along" in voice settings', action: 'configure', target: 'tts_provider' },
      { label: 'Ask your companion to hum the current track', action: 'navigate', target: 'chat' },
    ],
    category: 'voice',
    combos: [],
  },

  // ── EXPANDED AVATAR NODES ───────────────────────────────────────────────
  {
    id: 'expressions-pack',
    name: 'Mask of a Thousand Faces',
    tagline: 'Webcam-recorded facial expression presets (side quest)',
    description: 'Capture a face with the camera, save it as a named expression preset, and let the brain trigger it on its own. Privacy: per-session camera consent only — never always-on. Side quest in the Persona chain; ships after the main persona chain. See docs/persona-design.md § 10.',
    icon: '🎭',
    tier: 'advanced',
    requires: ['avatar', 'soul-mirror'],
    rewards: ['Named custom expression presets', 'Per-session camera consent flow', 'Brain-triggered personal faces'],
    rewardIcons: ['😏', '📸', '✨'],
    questSteps: [
      { label: 'Open Brain → Persona panel', action: 'navigate', target: 'brain' },
      { label: 'Click "Teach an expression" — grants camera for THIS session only', action: 'info' },
      { label: 'Hold a pose, name it, save. Browse / delete from the same panel.', action: 'info' },
    ],
    category: 'avatar',
    combos: [
      {
        withSkills: ['voice-emotion'],
        name: 'Emotion Sync',
        description: 'Expressions + emotional voice = perfectly aligned face and tone.',
        icon: '💞',
      },
    ],
  },
  {
    id: 'outfit-system',
    name: 'Wardrobe of Worlds',
    tagline: 'Swap costumes & accessories',
    description: 'Save outfit presets per VRM — uniforms, casual wear, holiday gear. Your companion remembers what they wore when.',
    icon: '👗',
    tier: 'advanced',
    requires: ['avatar'],
    rewards: ['Outfit presets', 'Accessory toggles', 'Per-occasion wardrobe'],
    rewardIcons: ['🧥', '🎀', '📅'],
    questSteps: [
      { label: 'Open the chat view\'s model panel', action: 'navigate', target: 'chat' },
      { label: 'Save the current look as an outfit preset', action: 'configure', target: 'tts_provider' },
    ],
    category: 'avatar',
    combos: [],
  },
  {
    id: 'motion-capture',
    name: 'Mirror Dance',
    tagline: 'Webcam-driven motion capture (side quest)',
    description: 'Record a short motion clip from your webcam and let the brain trigger it as a personal gesture. Privacy: per-session camera consent only — never always-on. Side quest in the Persona chain; ships after the main persona chain. See docs/persona-design.md § 10.',
    icon: '🪩',
    tier: 'ultimate',
    requires: ['avatar', 'soul-mirror'],
    rewards: ['Webcam motion capture', 'Named personal gesture clips', 'Per-session camera consent flow'],
    rewardIcons: ['📹', '🧍', '✋'],
    questSteps: [
      { label: 'Run TerranSoul desktop app (Tauri required)', action: 'info' },
      { label: 'Open Brain → Persona panel', action: 'navigate', target: 'brain' },
      { label: 'Click "Teach a motion" — grants camera for THIS session only', action: 'info' },
    ],
    category: 'avatar',
    combos: [
      {
        withSkills: ['expressions-pack'],
        name: 'Living Mirror',
        description: 'Mocap + expressions = the avatar reflects your face and body together.',
        icon: '🌟',
      },
    ],
  },

  // ── PERSONA MAIN CHAIN (research-driven, no camera) ─────────────────────
  // See docs/persona-design.md § 10. These quests teach the user how to
  // author the LLM's persona via traits / brain extraction. They form the
  // gateway and main path; the camera quests above are the side chain.
  {
    id: 'soul-mirror',
    name: 'Soul Mirror',
    tagline: 'Open the Persona panel and meet your default companion',
    description: 'Discover the Persona panel inside the Brain hub. The default "Soul" persona materialises on disk and is injected into every chat. From here you can customise who your companion is — without ever needing the camera.',
    icon: '🪞',
    tier: 'advanced',
    requires: ['avatar'],
    rewards: ['Persona panel in Brain hub', '[PERSONA] block in every chat prompt', 'Default "Soul" persona on first launch'],
    rewardIcons: ['🎭', '📜', '✨'],
    questSteps: [
      { label: 'Open the Brain tab', action: 'navigate', target: 'brain' },
      { label: 'Scroll to the Persona panel — review the default traits', action: 'info' },
    ],
    category: 'avatar',
    combos: [],
  },
  {
    id: 'my-persona',
    name: 'My Persona',
    tagline: 'Teach your companion who they are',
    description: 'Edit the persona name, role, background, tone, quirks and "never" list. Every chat from now on is shaped by this block, alongside your long-term memory. No camera required.',
    icon: '🎭',
    tier: 'advanced',
    requires: ['soul-mirror', 'free-brain'],
    rewards: ['Custom persona traits', 'Live system-prompt preview', 'Per-trait list editor'],
    rewardIcons: ['📝', '👁️', '➕'],
    questSteps: [
      { label: 'Open Brain → Persona panel', action: 'navigate', target: 'brain' },
      { label: 'Edit name / role / bio / tone, then click Save persona', action: 'info' },
    ],
    category: 'avatar',
    combos: [
      {
        withSkills: ['free-brain'],
        name: 'Soul of the Words',
        description: 'Persona block now flows into every chat turn\'s system prompt.',
        icon: '💬',
      },
    ],
  },
  {
    id: 'master-echo',
    name: "Master's Echo",
    tagline: 'Let the brain propose a persona from your past chats',
    description: 'Ask the brain to read your conversations and personal-tier memories, then propose a persona that mirrors who you are. Review, edit, and accept the suggestion. The persona keeps learning over time without any camera.',
    icon: '🌒',
    tier: 'ultimate',
    requires: ['my-persona', 'memory'],
    rewards: ['Brain-extracted persona suggestions', 'One-click accept / edit', 'Drift-detection prompts as you grow'],
    rewardIcons: ['🧠', '✅', '🌱'],
    questSteps: [
      { label: 'Open Brain → Persona panel', action: 'navigate', target: 'brain' },
      { label: 'Click "Suggest a persona from my chats" (requires brain + memories)', action: 'info' },
    ],
    category: 'avatar',
    combos: [
      {
        withSkills: ['rag-knowledge'],
        name: 'Soul of the Library',
        description: 'Persona drawn from your long-term memory — the companion now mirrors who you actually are.',
        icon: '📚',
      },
    ],
  },

  // ── EXPANDED SOCIAL NODES ───────────────────────────────────────────────
  {
    id: 'chat-rooms',
    name: 'Council Chamber',
    tagline: 'Multi-user chat rooms',
    description: 'Open a private room and invite friends through Soul Link. Everyone shares the same companion and conversation thread.',
    icon: '🪑',
    tier: 'advanced',
    requires: ['device-link'],
    rewards: ['Shared chat rooms', 'Per-user identity', 'Companion as moderator'],
    rewardIcons: ['🚪', '🪪', '👮'],
    questSteps: [
      { label: 'Open the Soul Link panel', action: 'navigate', target: 'chat' },
      { label: 'Create a Room and share the join code', action: 'configure', target: 'tts_provider' },
    ],
    category: 'social',
    combos: [
      {
        withSkills: ['diarization'],
        name: 'Round Table Discussion',
        description: 'Rooms + diarization = the companion knows who said what across the call.',
        icon: '🗣️',
      },
    ],
  },
  {
    id: 'share-quests',
    name: 'Bonded Adventures',
    tagline: 'Share quest progress with friends',
    description: 'Friends on Soul Link can see your skill tree progress, cheer on completed combos, and tackle co-op quests together.',
    icon: '🤝',
    tier: 'advanced',
    requires: ['device-link'],
    rewards: ['Friend leaderboards', 'Co-op quests', 'Cheer reactions'],
    rewardIcons: ['🏆', '🤲', '🎉'],
    questSteps: [
      { label: 'Pair a friend\'s device through Soul Link', action: 'navigate', target: 'chat' },
      { label: 'Enable "Share quest progress" in privacy settings', action: 'configure', target: 'tts_provider' },
    ],
    category: 'social',
    combos: [],
  },
  {
    id: 'community-hub',
    name: 'World Tree',
    tagline: 'Community marketplace & social feed',
    description: 'Discover community-made characters, agents, and BGM packs. Follow creators, leave reactions, and remix what you find.',
    icon: '🌳',
    tier: 'ultimate',
    requires: ['agents', 'device-link'],
    rewards: ['Creator profiles', 'Pack remixing', 'Social feed'],
    rewardIcons: ['🎨', '🔁', '📰'],
    questSteps: [
      { label: 'Run TerranSoul desktop app (Tauri required)', action: 'info' },
      { label: 'Open the Marketplace and sign in to your community profile', action: 'navigate', target: 'marketplace' },
    ],
    category: 'social',
    combos: [],
  },

  // ── EXPANDED UTILITY NODES ──────────────────────────────────────────────
  {
    id: 'themes',
    name: 'Aetherweave',
    tagline: 'Pick your UI theme & accent',
    description: 'Switch between Adventurer, Corporate, Midnight, Aurora, Neural, Sakura, Neko, Playground, and Pastel themes — the companion\'s UI shifts color, blur, and glow to match.',
    icon: '🎨',
    tier: 'foundation',
    requires: [],
    rewards: ['9 built-in UI themes', 'Light & dark modes', 'Instant hot-swap'],
    rewardIcons: ['🎨', '☀️', '⚡'],
    questSteps: [
      { label: 'Open Settings (gear icon in chat view)', action: 'info' },
      { label: 'Pick a theme from the Appearance section', action: 'configure', target: 'themes' },
    ],
    category: 'utility',
    combos: [],
    videoRef: 'dQw4w9WgXcQ',
  },
  {
    id: 'focus-timer',
    name: 'Hourglass',
    tagline: 'Pomodoro & focus sessions',
    description: 'Start a focus timer and your companion will quiet down, mute notifications, and check in at the end of each interval.',
    icon: '⏳',
    tier: 'foundation',
    requires: [],
    rewards: ['Pomodoro timer', 'Quiet-mode during focus', 'End-of-session check-in'],
    rewardIcons: ['⏱️', '🤫', '✅'],
    questSteps: [
      { label: 'Open Settings (gear icon in chat view)', action: 'info' },
      { label: 'Start a 25-minute focus session from the Hourglass widget', action: 'configure', target: 'bgm_enabled' },
    ],
    category: 'utility',
    combos: [
      {
        withSkills: ['bgm'],
        name: 'Lofi Sanctuary',
        description: 'Focus timer + BGM = an automatic deep-work mood.',
        icon: '🎧',
      },
    ],
    videoRef: 'dQw4w9WgXcQ',
  },
  {
    id: 'clipboard-watcher',
    name: 'Quill of Echoes',
    tagline: 'Clipboard-aware quick actions',
    description: 'Copy text and your companion offers quick actions — translate, summarize, explain, rewrite — through a tiny floating bubble.',
    icon: '📋',
    tier: 'advanced',
    requires: ['free-brain'],
    rewards: ['Clipboard quick actions', 'Floating action bubble', 'Customizable prompt presets'],
    rewardIcons: ['✂️', '🫧', '🧰'],
    questSteps: [
      { label: 'Run TerranSoul desktop app (Tauri required)', action: 'info' },
      { label: 'Enable Clipboard Quick Actions in settings', action: 'configure', target: 'shortcuts' },
      { label: 'Copy a paragraph and click the floating bubble', action: 'info' },
    ],
    category: 'utility',
    combos: [
      {
        withSkills: ['translation'],
        name: 'Pocket Translator',
        description: 'Clipboard + translation = instant translate-on-copy.',
        icon: '🌐',
      },
    ],
  },
  {
    id: 'notes-vault',
    name: 'Codex',
    tagline: 'Personal notes workspace',
    description: 'A lightweight markdown notes panel that lives next to chat. Pin snippets, quote your companion, and tag entries to skill nodes.',
    icon: '📒',
    tier: 'advanced',
    requires: ['memory'],
    rewards: ['Markdown notes', 'Pinnable snippets', 'Skill-tagged entries'],
    rewardIcons: ['📝', '📌', '🏷️'],
    questSteps: [
      { label: 'Open the Memory tab', action: 'navigate', target: 'memory' },
      { label: 'Open the Codex side-panel and add your first note', action: 'configure', target: 'memory_add' },
    ],
    category: 'utility',
    combos: [
      {
        withSkills: ['rag-knowledge'],
        name: 'Living Codex',
        description: 'Codex + RAG = notes are searchable knowledge your companion can cite.',
        icon: '📖',
      },
    ],
  },

  // ── LLM-DRIVEN ANIMATION (Phase 14.16) ──────────────────────────────────
  {
    id: 'animation-mastery',
    name: 'Living Canvas',
    tagline: 'Self-taught animations from the brain',
    description: 'The brain generates novel 3D animations from conversation context — no pre-baked clips needed. The self-improve loop discovers motion gaps and autonomously learns new gestures, poses, and reactions over time.',
    icon: '🎬',
    tier: 'ultimate',
    requires: ['local-brain', 'soul-mirror'],
    rewards: ['LLM-generated bone animations', 'Self-improving motion vocabulary', 'Emotion-reactive procedural blending'],
    rewardIcons: ['🦴', '🧠', '💃'],
    questSteps: [
      { label: 'Install a local LLM via the Brain Setup', action: 'navigate', target: 'brain-setup' },
      { label: 'Open Persona panel and generate 5+ learned motions', action: 'navigate', target: 'brain' },
      { label: 'Enable self-improve to let the brain discover new animations', action: 'navigate', target: 'self-improve' },
    ],
    category: 'avatar',
    combos: [
      {
        withSkills: ['expressions-pack'],
        name: 'Full Spectrum',
        description: 'Brain-generated body animations + learned facial expressions = the avatar performs with its whole being.',
        icon: '🌈',
      },
      {
        withSkills: ['master-echo'],
        name: 'Mirror Dance',
        description: 'A self-aware persona that teaches itself how to move — the avatar\'s gestures evolve alongside its personality.',
        icon: '🪩',
      },
    ],
  },
];

/** Filter skills by current platform */
function getAvailableSkillsForPlatform(skills: SkillNode[]): SkillNode[] {
  const currentPlatform = getCurrentPlatform();
  return skills.filter(skill => 
    !skill.platforms || skill.platforms.includes(currentPlatform)
  );
}

// ── Store ──────────────────────────────────────────────────────────────────────

export const useSkillTreeStore = defineStore('skill-tree', () => {
  const allNodes = ref<SkillNode[]>(SKILL_NODES);
  const activeQuestId = ref<string | null>(null);
  const tracker = ref<QuestTrackerData>(freshTracker());
  const isLoadingSuggestions = ref(false);
  /** When true, QuestRewardCeremony and ComboToast skip showing notifications.
   *  Set during first-launch setup and the FirstLaunchWizard to avoid spamming
   *  the user with a stack of unlock popups for auto-detected features. */
  const notificationsSuppressed = ref(false);
  
  // Filter nodes by current platform
  const nodes = ref<SkillNode[]>(getAvailableSkillsForPlatform(SKILL_NODES));
  const currentPlatform = ref<Platform>(getCurrentPlatform());

  // ── Dual persistence (Tauri → localStorage fallback) ─────────────────────

  function loadTrackerFromLocalStorage(): QuestTrackerData {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (!raw) return freshTracker();
      const parsed = JSON.parse(raw);
      return migrateTracker(parsed);
    } catch {
      return freshTracker();
    }
  }

  function saveTrackerToLocalStorage(data: QuestTrackerData): void {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(data));
    } catch {
      // Storage full or unavailable — non-critical
    }
  }

  async function loadTracker(): Promise<void> {
    let tauriData: QuestTrackerData | null = null;
    try {
      const raw = await invoke<string>('get_quest_tracker');
      const parsed = JSON.parse(raw);
      tauriData = migrateTracker(parsed);
    } catch {
      // Tauri unavailable — will rely on localStorage only
    }
    const localData = loadTrackerFromLocalStorage();

    if (tauriData) {
      // Merge both sources so data from neither is lost
      tracker.value = mergeTrackers(tauriData, localData);
      // Write the merged result back to keep them in sync
      saveTrackerToLocalStorage(tracker.value);
    } else {
      tracker.value = localData;
    }
  }

  async function saveTracker(): Promise<void> {
    const data = tracker.value;
    saveTrackerToLocalStorage(data);
    try {
      await invoke('save_quest_tracker', { data: JSON.stringify(data) });
    } catch {
      // Tauri unavailable — localStorage is the fallback
    }
  }

  // ── Real-time status detection ────────────────────────────────────────────

  function getSkillStatus(skillId: string): SkillStatus {
    const brain = useBrainStore();
    const voice = useVoiceStore();
    const settings = useSettingsStore();

    // Manual completion override — user explicitly marked as done
    if (tracker.value.manuallyCompletedIds.includes(skillId)) return 'active';

    const isActive = checkActive(skillId, brain, voice, settings);
    if (isActive) return 'active';

    const node = nodes.value.find(n => n.id === skillId);
    if (!node) return 'locked';

    const prereqsMet = node.requires.every(reqId => getSkillStatus(reqId) === 'active');
    return prereqsMet ? 'available' : 'locked';
  }

  function checkActive(
    skillId: string,
    brain: ReturnType<typeof useBrainStore>,
    voice: ReturnType<typeof useVoiceStore>,
    settings: ReturnType<typeof useSettingsStore>,
  ): boolean {
    switch (skillId) {
      case 'free-brain':
        return brain.hasBrain;
      case 'paid-brain':
        return brain.brainMode?.mode === 'paid_api';
      case 'local-brain':
        return brain.brainMode?.mode === 'local_ollama';
      case 'tts':
        return voice.config.tts_provider !== null;
      case 'asr':
        return voice.config.asr_provider !== null;
      case 'whisper-asr':
        return voice.config.asr_provider === 'groq-whisper' || voice.config.asr_provider === 'whisper-api';
      case 'avatar':
        // Requires manual completion — the user should explore the avatar
        // panel at least once. Always-true made quests look pre-completed.
        return false;
      // ── Persona main chain (docs/persona-design.md § 10.2) ──────────────
      case 'soul-mirror': {
        // Active once the persona panel has been opened at least once and
        // the default traits have materialised on disk.
        const persona = usePersonaStore();
        return persona.traitsLoaded;
      }
      case 'my-persona': {
        // Active when the user customised the default persona AND a brain
        // is configured (so the [PERSONA] block actually flows somewhere).
        const persona = usePersonaStore();
        return persona.hasCustomPersona && brain.brainMode !== null;
      }
      case 'master-echo': {
        // Active once the brain has produced a persona suggestion at least
        // once. The frontend records this when the suggestion is accepted.
        const persona = usePersonaStore();
        return persona.lastBrainExtractedAt !== null;
      }
      // ── Persona side chain (camera-driven, durable artifacts) ───────────
      case 'expressions-pack': {
        const persona = usePersonaStore();
        return (persona.learnedExpressions?.length ?? 0) > 0;
      }
      case 'motion-capture': {
        const persona = usePersonaStore();
        return (persona.learnedMotions?.length ?? 0) > 0;
      }
      case 'animation-mastery': {
        // Auto-active when local brain is configured + 5+ learned motions exist
        const persona = usePersonaStore();
        return brain.brainMode?.mode === 'local_ollama'
          && (persona.learnedMotions?.length ?? 0) >= 5;
      }
      case 'bgm':
        return settings.settings?.bgm_enabled ?? false;
      case 'bgm-custom':
        return (settings.settings?.bgm_custom_tracks?.length ?? 0) > 0;
      case 'bgm-video':
        return false; // Requires manual completion
      case 'themes': {
        // Auto-active once the user has picked any non-default theme.
        try {
          return (localStorage.getItem('ts-active-theme') ?? DEFAULT_THEME_ID) !== DEFAULT_THEME_ID;
        } catch {
          return false;
        }
      }
      case 'hotwords':
        return voice.config.asr_provider !== null;
      case 'memory':
        return brain.brainMode !== null;
      case 'rag-knowledge': {
        // RAG is auto-active when: brain is configured + at least one memory exists
        const memStore = useMemoryStore();
        return brain.brainMode !== null && memStore.memories.length > 0;
      }
      case 'scholar-quest':
        return false; // Chain quest — manually completed via KnowledgeQuestDialog
      case 'quest-daily-brief': {
        // Auto-active when: brain configured + at least one memory exists
        const memStoreForBrief = useMemoryStore();
        return brain.brainMode !== null && memStoreForBrief.memories.length > 0;
      }
      case 'presence':
        return brain.hasBrain && voice.config.tts_provider !== null;
      case 'device-link':
        return false;
      case 'agents':
        return false;
      case 'diarization':
        return false;
      case 'translation':
        return false;
      case 'vision':
        return false;
      case 'pet-mode':
        return false;
      // Windows-specific features — require manual completion so they don't
      // show as "completed" on first launch. The user should discover and
      // acknowledge them through the skill tree.
      case 'windows-notifications':
        return false;
      case 'windows-shortcuts':
        return false;
      case 'windows-taskbar':
        return false;
      case 'windows-startup':
        return false;
      default:
        return false;
    }
  }

  // ── Activation timestamp tracking ─────────────────────────────────────────

  function recordActivations(): void {
    let changed = false;
    const now = Date.now();
    for (const node of nodes.value) {
      if (getSkillStatus(node.id) === 'active' && !tracker.value.activationTimestamps[node.id]) {
        tracker.value.activationTimestamps[node.id] = now;
        changed = true;
      }
    }
    if (changed) saveTracker();
  }

  // ── Computed aggregates ─────────────────────────────────────────────────

  const totalNodes = computed(() => nodes.value.length);
  const activeCount = computed(() => nodes.value.filter(n => getSkillStatus(n.id) === 'active').length);
  const availableCount = computed(() => nodes.value.filter(n => getSkillStatus(n.id) === 'available').length);
  const progressPercent = computed(() => Math.round((activeCount.value / totalNodes.value) * 100));

  const activeCombos = computed(() => {
    const result: { combo: ComboRef; sourceSkill: string }[] = [];
    for (const node of nodes.value) {
      if (getSkillStatus(node.id) !== 'active') continue;
      for (const combo of node.combos) {
        const allActive = combo.withSkills.every(id => getSkillStatus(id) === 'active');
        if (allActive) result.push({ combo, sourceSkill: node.id });
      }
    }
    return result;
  });

  // ── Quest tracker actions ─────────────────────────────────────────────────

  function dismissQuest(skillId: string): void {
    if (!tracker.value.dismissedQuestIds.includes(skillId)) {
      tracker.value.dismissedQuestIds.push(skillId);
      saveTracker();
    }
  }

  function undismissQuest(skillId: string): void {
    tracker.value.dismissedQuestIds = tracker.value.dismissedQuestIds.filter(id => id !== skillId);
    saveTracker();
  }

  function pinQuest(skillId: string): void {
    if (!tracker.value.pinnedQuestIds.includes(skillId)) {
      tracker.value.pinnedQuestIds.push(skillId);
      saveTracker();
    }
  }

  function unpinQuest(skillId: string): void {
    tracker.value.pinnedQuestIds = tracker.value.pinnedQuestIds.filter(id => id !== skillId);
    saveTracker();
  }

  function markComplete(skillId: string): void {
    if (!tracker.value.manuallyCompletedIds.includes(skillId)) {
      tracker.value.manuallyCompletedIds.push(skillId);
      saveTracker();
      recordActivations();
    }
  }

  function unmarkComplete(skillId: string): void {
    tracker.value.manuallyCompletedIds = tracker.value.manuallyCompletedIds.filter(id => id !== skillId);
    saveTracker();
  }

  /** Mark one or more combo keys as already-notified-about. */
  function markCombosSeen(keys: string[]): void {
    if (keys.length === 0) return;
    const before = tracker.value.seenComboKeys.length;
    tracker.value.seenComboKeys = [
      ...new Set([...tracker.value.seenComboKeys, ...keys]),
    ];
    if (tracker.value.seenComboKeys.length !== before) saveTracker();
  }

  /** Update the high-water mark for the most recent reward ceremony shown. */
  function setLastSeenActivationTimestamp(ts: number): void {
    if (ts > tracker.value.lastSeenActivationTimestamp) {
      tracker.value.lastSeenActivationTimestamp = ts;
      saveTracker();
    }
  }

  const pinnedQuests = computed(() =>
    tracker.value.pinnedQuestIds
      .map(id => nodes.value.find(n => n.id === id))
      .filter((n): n is SkillNode => n !== undefined),
  );

  // ── Daily AI quest suggestions ──────────────────────────────────────────

  function todayString(): string {
    return new Date().toISOString().slice(0, 10);
  }

  const dailySuggestions = computed<DailySuggestion[]>(() =>
    tracker.value.dailySuggestionIds
      .map(id => {
        const node = nodes.value.find(n => n.id === id);
        if (!node) return null;
        return {
          node,
          status: getSkillStatus(node.id),
          reason: tracker.value.dailySuggestionReason,
        };
      })
      .filter((s): s is DailySuggestion => s !== null),
  );

  const needsRefresh = computed(() => tracker.value.lastSuggestionDate !== todayString());

  /**
   * Use the locally-determined heuristic to pick optimal daily quests.
   * Fast, works offline, used as fallback when brain is unavailable.
   */
  function generateLocalSuggestions(): string[] {
    const available = nodes.value.filter(n => getSkillStatus(n.id) === 'available');
    const dismissed = new Set(tracker.value.dismissedQuestIds);
    const candidates = available.filter(n => !dismissed.has(n.id));

    // Priority: foundation first, then skills that unlock the most combos
    const scored = candidates.map(n => {
      let score = 0;
      if (n.tier === 'foundation') score += 30;
      else if (n.tier === 'advanced') score += 20;
      else score += 10;
      score += n.combos.length * 15;
      score += n.rewards.length * 5;
      // Boost skills that would satisfy other skills' prerequisites
      const wouldUnlock = nodes.value.filter(
        other => other.requires.includes(n.id) && getSkillStatus(other.id) === 'locked',
      ).length;
      score += wouldUnlock * 20;
      return { id: n.id, score };
    });

    scored.sort((a, b) => b.score - a.score);
    return scored.slice(0, 3).map(s => s.id);
  }

  /**
   * Ask the active brain (LLM) for personalized daily quest suggestions.
   * Falls back to local heuristic if brain is unavailable or fails.
   */
  async function refreshDailySuggestions(): Promise<void> {
    if (isLoadingSuggestions.value) return;
    isLoadingSuggestions.value = true;

    try {
      const brain = useBrainStore();
      if (!brain.hasBrain || !brain.brainMode) {
        applyLocalSuggestions();
        return;
      }

      // Build context about current state for the AI
      const statusMap = nodes.value.map(n => ({
        id: n.id,
        name: n.name,
        tier: n.tier,
        status: getSkillStatus(n.id),
        category: n.category,
        rewards: n.rewards,
        combos: n.combos.map(c => c.name),
      }));
      const active = statusMap.filter(s => s.status === 'active').map(s => s.name);
      const available = statusMap.filter(s => s.status === 'available').map(s => `${s.id}: ${s.name} (${s.tier}) — rewards: ${s.rewards.join(', ')}`);
      const dismissed = tracker.value.dismissedQuestIds;

      const prompt = `You are a gaming quest advisor for TerranSoul, an AI companion app with a skill tree system.

Current active skills: ${active.join(', ') || 'None'}
Available quests (prerequisites met):
${available.join('\n') || 'None available'}
User dismissed: ${dismissed.join(', ') || 'None'}

Pick the TOP 3 most impactful quests to recommend today. Consider:
1. Which skills unlock the most downstream abilities
2. Which combo abilities would be unlocked
3. Foundation skills first if not yet done
4. Skip dismissed quests

Respond with ONLY valid JSON (no markdown):
{"ids":["skill-id-1","skill-id-2","skill-id-3"],"reason":"One sentence explaining why these three"}`;

      const mode = brain.brainMode;
      let baseUrl: string;
      let model: string;
      let apiKey: string | null = null;

      if (mode.mode === 'free_api') {
        const provider = brain.freeProviders.find(p => p.id === mode.provider_id);
        if (!provider) { applyLocalSuggestions(); return; }
        baseUrl = provider.base_url;
        model = provider.model;
        apiKey = mode.api_key;
      } else if (mode.mode === 'paid_api') {
        baseUrl = mode.base_url;
        model = mode.model;
        apiKey = mode.api_key;
      } else {
        // local_ollama — use local endpoint
        baseUrl = 'http://localhost:11434';
        model = mode.model;
      }

      const history: ChatMessage[] = [{ role: 'user', content: prompt }];
      let fullText = '';

      await new Promise<void>((resolve, reject) => {
        streamChatCompletion(baseUrl, model, apiKey, history, {
          onChunk: (text) => { fullText += text; },
          onDone: () => resolve(),
          onError: (err) => reject(new Error(err)),
        });
      });

      // Parse AI response
      const jsonMatch = fullText.match(/\{[\s\S]*\}/);
      if (jsonMatch) {
        const parsed = JSON.parse(jsonMatch[0]) as { ids?: string[]; reason?: string };
        if (Array.isArray(parsed.ids) && parsed.ids.length > 0) {
          // Validate that all suggested IDs exist
          const valid = parsed.ids.filter(id =>
            typeof id === 'string' && nodes.value.some(n => n.id === id),
          );
          if (valid.length > 0) {
            tracker.value.dailySuggestionIds = valid.slice(0, 3);
            tracker.value.dailySuggestionReason = typeof parsed.reason === 'string' ? parsed.reason : null;
            tracker.value.lastSuggestionDate = todayString();
            await saveTracker();
            return;
          }
        }
      }

      // JSON parse failed — fall back to local
      applyLocalSuggestions();
    } catch {
      applyLocalSuggestions();
    } finally {
      isLoadingSuggestions.value = false;
    }
  }

  function applyLocalSuggestions(): void {
    tracker.value.dailySuggestionIds = generateLocalSuggestions();
    tracker.value.dailySuggestionReason = null;
    tracker.value.lastSuggestionDate = todayString();
    saveTracker();
    isLoadingSuggestions.value = false;
  }

  // ── Daily brief quest (Chunk 33B.3) ───────────────────────────────────────

  const dailyBrief = computed<DailyBriefCache | null>(() => {
    const cached = tracker.value.dailyBrief;
    if (!cached) return null;
    // Only return if still from today
    return cached.date === todayString() ? cached : null;
  });

  const dailyBriefNeedsRefresh = computed<boolean>(() => {
    return !dailyBrief.value;
  });

  /**
   * Fetch the daily brief via the backend command. Caches the result
   * in the tracker so subsequent reads within the same day are instant.
   * Idempotent: skips if already fetched today.
   */
  async function fetchDailyBrief(force = false): Promise<DailyBriefCache | null> {
    const today = todayString();
    if (!force && tracker.value.dailyBrief?.date === today) {
      return tracker.value.dailyBrief;
    }

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const result = await invoke<{
        memories: Array<{
          id: number;
          content: string;
          tags: string;
          importance: number;
          created_at: number;
        }>;
        time_range: { start_ms: number; end_ms: number };
        total_in_range: number;
      }>('daily_brief_query', { limit: 10 });

      const brief: DailyBriefCache = {
        date: today,
        items: result.memories.map(m => ({
          id: m.id,
          content: m.content,
          tags: m.tags,
          importance: m.importance,
          createdAt: m.created_at,
        })),
        totalInRange: result.total_in_range,
      };

      tracker.value.dailyBrief = brief;
      saveTracker();
      return brief;
    } catch {
      // If the backend isn't available (web mode), return null gracefully.
      return null;
    }
  }

  // ── Quest actions ─────────────────────────────────────────────────────────

  function openQuest(skillId: string) {
    activeQuestId.value = skillId;
  }

  function closeQuest() {
    activeQuestId.value = null;
  }

  // ── Random quest event (in-chat RPG interaction) ──────────────────────

  const questEventActive = ref(false);
  const questEventNode = ref<SkillNode | null>(null);

  /**
   * Pick a random quest from non-active, non-dismissed, prerequisites-met pool.
   * Weights foundation higher to guide new users.
   */
  function pickRandomQuest(): SkillNode | null {
    const dismissed = new Set(tracker.value.dismissedQuestIds);
    const candidates = nodes.value.filter(n =>
      getSkillStatus(n.id) === 'available' && !dismissed.has(n.id),
    );
    if (candidates.length === 0) return null;
    // Weighted random: foundation 3x, advanced 2x, ultimate 1x
    const weighted: SkillNode[] = [];
    for (const c of candidates) {
      const w = c.tier === 'foundation' ? 3 : c.tier === 'advanced' ? 2 : 1;
      for (let i = 0; i < w; i++) weighted.push(c);
    }
    return weighted[Math.floor(Math.random() * weighted.length)];
  }

  /**
   * Trigger a quest event by injecting an RPG-style message into the chat.
   * The model presents the quest with multiple choice options.
   */
  function triggerQuestEvent(specificQuestId?: string): void {
    const quest = specificQuestId
      ? nodes.value.find(n => n.id === specificQuestId) ?? pickRandomQuest()
      : pickRandomQuest();
    if (!quest) return;

    questEventNode.value = quest;
    questEventActive.value = true;

    const conversation = useConversationStore();
    const status = getSkillStatus(quest.id);
    const rewardList = quest.rewards.map((r, i) => `${quest.rewardIcons[i] || '🎁'} ${r}`).join('\n');

    const choices: QuestChoice[] = [
      { label: 'Accept Quest', value: 'accept', icon: '⚔️' },
      { label: 'Tell me more', value: 'info', icon: '📖' },
      { label: 'Maybe later', value: 'dismiss', icon: '💤' },
    ];

    const content = `🗡️ **A New Quest Appears!**\n\n` +
      `**${quest.icon} ${quest.name}** — *${quest.tagline}*\n\n` +
      `${quest.description}\n\n` +
      `**Rewards:**\n${rewardList}\n\n` +
      `*Status: ${status === 'available' ? '🟢 Ready to begin' : '🔒 Locked'}*\n\n` +
      `What would you like to do, adventurer?`;

    conversation.messages.push({
      id: crypto.randomUUID(),
      role: 'assistant',
      content,
      agentName: 'Quest Guide',
      sentiment: 'happy',
      timestamp: Date.now(),
      questChoices: choices,
      questId: quest.id,
    });
  }

  /**
   * Handle the player's quest choice from the chat buttons.
   */
  async function handleQuestChoice(questId: string, choiceValue: string): Promise<void> {
    const quest = nodes.value.find(n => n.id === questId);
    if (!quest) return;

    const conversation = useConversationStore();

    // Push the user's choice as a user message
    const choiceLabels: Record<string, string> = {
      accept: '⚔️ Accept Quest',
      info: '📖 Tell me more',
      dismiss: '💤 Maybe later',
      start_step: '🏃 Let\'s begin!',
      pin: '📌 Pin this quest',
      'bgm-autoplay': '🎶 Autoplay BGM',
      self: '💪 I\'ll handle it myself',
    };
    conversation.messages.push({
      id: crypto.randomUUID(),
      role: 'user',
      content: choiceLabels[choiceValue] || choiceValue,
      timestamp: Date.now(),
    });

    if (choiceValue === 'dismiss') {
      dismissQuest(questId);
      conversation.messages.push({
        id: crypto.randomUUID(),
        role: 'assistant',
        content: `No worries, adventurer! The **${quest.icon} ${quest.name}** quest will wait for you. Come back anytime you're ready! 🌟`,
        agentName: 'Quest Guide',
        sentiment: 'relaxed',
        timestamp: Date.now(),
      });
      questEventActive.value = false;
      questEventNode.value = null;
      return;
    }

    if (choiceValue === 'pin') {
      pinQuest(questId);
      conversation.messages.push({
        id: crypto.randomUUID(),
        role: 'assistant',
        content: `📌 **${quest.icon} ${quest.name}** has been pinned to your quest tracker! You can view it anytime in the Skill Tree.`,
        agentName: 'Quest Guide',
        sentiment: 'happy',
        timestamp: Date.now(),
      });
      questEventActive.value = false;
      questEventNode.value = null;
      return;
    }

    if (choiceValue === 'info') {
      const stepsText = quest.questSteps
        .map((step, i) => `${i + 1}. ${step.label}`)
        .join('\n');
      const comboText = quest.combos.length > 0
        ? `\n\n**Combo Unlocks:**\n${quest.combos.map(c => `${c.icon} **${c.name}** — ${c.description}`).join('\n')}`
        : '';

      const followUpChoices: QuestChoice[] = [
        { label: 'Let\'s begin!', value: 'start_step', icon: '🏃' },
        { label: 'Pin this quest', value: 'pin', icon: '📌' },
        { label: 'Maybe later', value: 'dismiss', icon: '💤' },
      ];

      conversation.messages.push({
        id: crypto.randomUUID(),
        role: 'assistant',
        content: `📖 **Quest Details: ${quest.icon} ${quest.name}**\n\n` +
          `**Steps to complete:**\n${stepsText}${comboText}\n\n` +
          `**Tier:** ${quest.tier.charAt(0).toUpperCase() + quest.tier.slice(1)} | **Category:** ${quest.category}\n\n` +
          `Ready to embark on this quest?`,
        agentName: 'Quest Guide',
        sentiment: 'happy',
        timestamp: Date.now(),
        questChoices: followUpChoices,
        questId: quest.id,
      });
      return;
    }

    if (choiceValue === 'accept' || choiceValue === 'start_step') {
      pinQuest(questId);
      const firstStep = quest.questSteps[0];
      const navigationChoices: QuestChoice[] = [];

      // BGM quest gets special choices: autoplay or manual
      if (questId === 'bgm') {
        navigationChoices.push({ label: 'Autoplay BGM', value: 'bgm-autoplay', icon: '🎶' });
        navigationChoices.push({ label: "I'll handle it myself", value: 'self', icon: '💪' });
      } else {
        if (firstStep?.target) {
          navigationChoices.push({ label: `Go to ${firstStep.target}`, value: `navigate:${firstStep.target}`, icon: '🧭' });
        }
        navigationChoices.push({ label: "I'll handle it myself", value: 'self', icon: '💪' });
      }

      conversation.messages.push({
        id: crypto.randomUUID(),
        role: 'assistant',
        content: `⚔️ **Quest Accepted: ${quest.icon} ${quest.name}!**\n\n` +
          `Your first step: **${firstStep?.label || 'Begin your journey'}**\n\n` +
          `I've pinned this quest to your tracker. Good luck, adventurer! 🌟`,
        agentName: 'Quest Guide',
        sentiment: 'happy',
        timestamp: Date.now(),
        questChoices: navigationChoices,
        questId: quest.id,
      });
      questEventActive.value = false;
      questEventNode.value = null;
      return;
    }

    // Handle navigation choices (e.g., "navigate:marketplace")
    if (choiceValue.startsWith('navigate:')) {
      questEventActive.value = false;
      questEventNode.value = null;
      // The view layer handles actual navigation via the emitted event
      return;
    }

    // Handle BGM autoplay choice
    if (choiceValue === 'bgm-autoplay') {
      conversation.messages.push({
        id: crypto.randomUUID(),
        role: 'assistant',
        content: `🎶 **BGM is now playing!** Ambient music will fill the background. Use the floating music bar to change tracks or adjust volume.`,
        agentName: 'Quest Guide',
        sentiment: 'happy',
        timestamp: Date.now(),
      });
      questEventActive.value = false;
      questEventNode.value = null;
      return;
    }

    // Default: close the event
    questEventActive.value = false;
    questEventNode.value = null;
  }

  function closeQuestEvent(): void {
    questEventActive.value = false;
    questEventNode.value = null;
  }

  // ── Visibility / unload guard ────────────────────────────────────────────

  let _guardInstalled = false;

  function installSaveGuard(): void {
    if (_guardInstalled || typeof window === 'undefined') return;
    _guardInstalled = true;

    // Sync-write to localStorage on visibility change or page unload so that
    // progress is never lost even if the Tauri async write hasn't finished.
    const flush = () => saveTrackerToLocalStorage(tracker.value);
    window.addEventListener('beforeunload', flush);
    document.addEventListener('visibilitychange', () => {
      if (document.visibilityState === 'hidden') flush();
    });
  }

  /** Suppress all quest-unlock and combo-unlock notifications.
   *  Call before batch operations (wizard setup, first-launch init). */
  function suppressNotifications(): void {
    notificationsSuppressed.value = true;
  }

  /** Resume notifications and mark everything currently active as "already
   *  seen" so no stale ceremonies or combo toasts fire. */
  function resumeNotifications(): void {
    const max = Math.max(0, ...Object.values(tracker.value.activationTimestamps));
    if (max > tracker.value.lastSeenActivationTimestamp) {
      tracker.value.lastSeenActivationTimestamp = max;
    }
    const keys = activeCombos.value.map(c => `${c.sourceSkill}::${c.combo.name}`);
    if (keys.length > 0) {
      tracker.value.seenComboKeys = [
        ...new Set([...tracker.value.seenComboKeys, ...keys]),
      ];
    }
    saveTracker();
    notificationsSuppressed.value = false;
  }

  /** Initialise tracker from persistent storage and refresh suggestions if needed. */
  async function initialise(): Promise<void> {
    await loadTracker();

    // On first run (empty tracker), suppress notifications so auto-detected
    // skills don't blast the user with a stack of unlock ceremonies.
    const isFirstRun = Object.keys(tracker.value.activationTimestamps).length === 0
                        && tracker.value.lastSeenActivationTimestamp === 0;
    if (isFirstRun) suppressNotifications();

    recordActivations();

    if (isFirstRun) resumeNotifications();

    installSaveGuard();

    // Watch for feature state changes and record activations in real-time
    // so quests are marked complete as soon as the user enables a feature.
    const brain = useBrainStore();
    const voice = useVoiceStore();
    const settings = useSettingsStore();
    watch(
      () => [
        brain.brainMode,
        brain.hasBrain,
        voice.config?.tts_provider,
        voice.config?.asr_provider,
        settings.settings?.bgm_enabled,
        settings.settings?.bgm_custom_tracks?.length ?? 0,
      ],
      () => recordActivations(),
    );

    if (needsRefresh.value) {
      await refreshDailySuggestions();
    }
  }

  return {
    // Static data
    nodes,
    allNodes,
    currentPlatform,
    // Quest dialog
    activeQuestId,
    openQuest,
    closeQuest,
    // Tracker state
    tracker,
    isLoadingSuggestions,
    notificationsSuppressed,
    // Aggregates
    totalNodes,
    activeCount,
    availableCount,
    progressPercent,
    activeCombos,
    // Status detection
    getSkillStatus,
    recordActivations,
    // Tracker actions
    dismissQuest,
    undismissQuest,
    pinQuest,
    unpinQuest,
    pinnedQuests,
    markComplete,
    unmarkComplete,
    markCombosSeen,
    setLastSeenActivationTimestamp,
    // Daily suggestions
    dailySuggestions,
    needsRefresh,
    refreshDailySuggestions,
    generateLocalSuggestions,
    // Daily brief quest
    dailyBrief,
    dailyBriefNeedsRefresh,
    fetchDailyBrief,
    // Persistence
    loadTracker,
    saveTracker,
    initialise,
    suppressNotifications,
    resumeNotifications,
    // Random quest event
    questEventActive,
    questEventNode,
    pickRandomQuest,
    triggerQuestEvent,
    handleQuestChoice,
    closeQuestEvent,
  };
});
