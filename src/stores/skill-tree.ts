import { defineStore } from 'pinia';
import { ref, computed, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useBrainStore } from './brain';
import { useVoiceStore } from './voice';
import { useSettingsStore } from './settings';
import { useConversationStore } from './conversation';
import { streamChatCompletion, type ChatMessage } from '../utils/free-api-client';
import type { QuestChoice } from '../types';

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
}

export interface DailySuggestion {
  node: SkillNode;
  status: SkillStatus;
  reason: string | null;
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
    description: 'Uses Pollinations AI — no API key, no setup. Your companion can think and respond in real time. This is the starting point for all intelligence.',
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
    combos: [],
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
    combos: [],
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
    combos: [],
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
    tagline: 'Run AI locally with llmfit',
    description: 'Full privacy — no data leaves your machine. Use llmfit to automatically select and run the best local model for your hardware. Requires decent hardware (8GB+ RAM).',
    icon: '🏰',
    tier: 'advanced',
    requires: ['free-brain'],
    rewards: ['100% private AI', 'No API costs', 'Offline capability', 'Unlimited usage'],
    rewardIcons: ['🔒', '💰', '✈️', '♾️'],
    questSteps: [
      { label: 'Install llmfit from GitHub', action: 'external', target: 'https://github.com/AlexsJones/llmfit' },
      { label: 'Open Brain Setup', action: 'navigate', target: 'brain-setup' },
      { label: 'Choose "Local" tier and let llmfit pick the best model', action: 'configure', target: 'brain_mode' },
    ],
    category: 'brain',
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
    combos: [],
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
    tagline: 'Floating companion on your desktop',
    description: 'Your character breaks free of the app window and floats on your desktop as a transparent overlay. Chat from any app!',
    icon: '🐾',
    tier: 'ultimate',
    requires: ['avatar', 'tts'],
    rewards: ['Transparent desktop overlay', 'Always-visible companion', 'Chat from any application'],
    rewardIcons: ['🪟', '📌', '💬'],
    questSteps: [
      { label: 'Run TerranSoul desktop app (Tauri required)', action: 'info' },
      { label: 'Click the paw icon in the sidebar to enter Pet Mode', action: 'navigate', target: 'pet-mode' },
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
    platforms: ['windows', 'macos', 'linux'], // Desktop only
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
        return true; // Always loaded by default
      case 'bgm':
        return settings.settings?.bgm_enabled ?? false;
      case 'bgm-custom':
        return (settings.settings?.bgm_custom_tracks?.length ?? 0) > 0;
      case 'bgm-video':
        return false; // Requires manual completion
      case 'hotwords':
        return voice.config.asr_provider !== null;
      case 'memory':
        return brain.brainMode !== null;
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
      // Windows-specific features
      case 'windows-notifications':
        return currentPlatform.value === 'windows';
      case 'windows-shortcuts':
        return currentPlatform.value === 'windows';
      case 'windows-taskbar':
        return currentPlatform.value === 'windows';
      case 'windows-startup':
        return currentPlatform.value === 'windows';
        // TODO: Add startup_enabled to settings when implemented
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

  /** Initialise tracker from persistent storage and refresh suggestions if needed. */
  async function initialise(): Promise<void> {
    await loadTracker();
    recordActivations();
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
    // Daily suggestions
    dailySuggestions,
    needsRefresh,
    refreshDailySuggestions,
    generateLocalSuggestions,
    // Persistence
    loadTracker,
    saveTracker,
    initialise,
    // Random quest event
    questEventActive,
    questEventNode,
    pickRandomQuest,
    triggerQuestEvent,
    handleQuestChoice,
    closeQuestEvent,
  };
});
