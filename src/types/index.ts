export interface Message {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  agentName?: string;
  /** The agent profile ID that produced this message. Enables per-agent
   *  conversation filtering and context isolation on agent swap. */
  agentId?: string;
  sentiment?: 'happy' | 'sad' | 'angry' | 'relaxed' | 'surprised' | 'neutral';
  timestamp: number;
  /** RPG-style quest choices attached to this message. */
  questChoices?: QuestChoice[];
  /** Associated quest/skill ID for quest-related messages. */
  questId?: string;
  /** Mark as system message (hidden from main chat UI). */
  system?: boolean;
  /** Emoji extracted from the model response, shown as a floating popup. */
  emoji?: string;
  /** Body animation motion key from LLM <anim> tags (e.g. 'greeting', 'clapping'). */
  motion?: string;
}

export interface QuestChoice {
  label: string;
  value: string;
  icon?: string;
}

export type CharacterState = 'idle' | 'thinking' | 'talking' | 'happy' | 'sad' | 'angry' | 'relaxed' | 'surprised';


export interface Agent {
  id: string;
  name: string;
  description: string;
  status: 'running' | 'stopped' | 'installing';
  capabilities: string[];
}

export interface VrmMetadata {
  title: string;
  author: string;
  license: string;
}

export interface DeviceInfo {
  device_id: string;
  public_key_b64: string;
  name: string;
}

export interface TrustedDevice {
  device_id: string;
  name: string;
  public_key_b64: string;
  paired_at: number;
}

export type LinkStatusValue = 'disconnected' | 'connecting' | 'connected' | 'reconnecting';

export interface LinkPeer {
  device_id: string;
  name: string;
  addr: string;
}

export interface LinkStatusResponse {
  status: LinkStatusValue;
  transport: string;
  peer: LinkPeer | null;
  server_port: number | null;
}

export interface SyncState {
  conversation_count: number;
  character_selection: string | null;
  agent_count: number;
  last_synced_at: number | null;
}

export type CommandStatusValue =
  | 'pending_approval'
  | 'executing'
  | 'completed'
  | 'denied'
  | 'failed';

export interface PendingCommand {
  command_id: string;
  origin_device: string;
  command_type: string;
  payload: unknown;
}

export interface CommandResultResponse {
  command_id: string;
  status: CommandStatusValue;
  payload: unknown;
}

export type InstallType = 'binary' | 'wasm' | 'sidecar';

export interface ManifestInfo {
  name: string;
  version: string;
  description: string;
  capabilities: string[];
  sensitive_capabilities: string[];
  install_type: InstallType;
  ipc_protocol_version: number;
  author: string | null;
  license: string | null;
  homepage: string | null;
}

export interface InstalledAgentInfo {
  name: string;
  version: string;
  description: string;
  install_path: string;
}

// ── Brain / Ollama ────────────────────────────────────────────────────────────

export interface SystemInfo {
  total_ram_mb: number;
  ram_tier_label: string;
  cpu_cores: number;
  cpu_name: string;
  os_name: string;
  arch: string;
  gpu_name?: string;
}

export interface ModelRecommendation {
  model_tag: string;
  display_name: string;
  description: string;
  required_ram_mb: number;
  is_top_pick: boolean;
  is_cloud?: boolean;
}

export interface OllamaStatus {
  running: boolean;
  model_count: number;
}

export interface OllamaModelEntry {
  name: string;
  size: number;
}

export interface LmStudioStatus {
  running: boolean;
  model_count: number;
  loaded_count: number;
}

export interface LmStudioLoadedInstance {
  id: string;
}

export interface LmStudioModelEntry {
  key: string;
  display_name: string;
  type: 'llm' | 'embedding' | string;
  publisher?: string | null;
  architecture?: string | null;
  size_bytes: number;
  params_string?: string | null;
  loaded_instances: LmStudioLoadedInstance[];
}

export interface LmStudioDownloadStatus {
  status: string;
  job_id?: string | null;
  total_size_bytes?: number | null;
  downloaded_size_bytes?: number | null;
  started_at?: string | null;
  completed_at?: string | null;
  error?: string | null;
}

export interface LmStudioLoadResult {
  type: 'llm' | 'embedding' | string;
  instance_id: string;
  status: string;
  load_time_seconds?: number | null;
}

export interface LmStudioUnloadResult {
  instance_id: string;
}

// ── Memory ────────────────────────────────────────────────────────────────────

export type MemoryType = 'fact' | 'preference' | 'context' | 'summary';

export type MemoryTier = 'short' | 'working' | 'long';

export interface MemoryEntry {
  id: number;
  content: string;
  tags: string;
  importance: number;
  memory_type: MemoryType;
  created_at: number;
  last_accessed: number | null;
  access_count: number;
  tier: MemoryTier;
  decay_score: number;
  session_id: string | null;
  parent_id: number | null;
  token_count: number;
}

export interface MemoryStats {
  total: number;
  short_count: number;
  working_count: number;
  long_count: number;
  total_tokens: number;
  avg_decay: number;
}

export interface NewMemory {
  content: string;
  tags: string;
  importance: number;
  memory_type: MemoryType;
}

// ── Entity-Relationship Graph (V5) ───────────────────────────────────────────

export type EdgeSource = 'user' | 'llm' | 'auto';
export type EdgeDirection = 'in' | 'out' | 'both';

/** Typed, directional edge between two memories. See `memory_edges` (V5). */
export interface MemoryEdge {
  id: number;
  src_id: number;
  dst_id: number;
  rel_type: string;
  /** LLM-reported confidence in [0, 1]. User edges are 1.0. */
  confidence: number;
  source: EdgeSource;
  created_at: number;
}

export interface EdgeStats {
  total_edges: number;
  by_rel_type: Array<[string, number]>;
  by_source: Array<[string, number]>;
  /** Number of memories with at least one incident edge. */
  connected_memories: number;
}

// ── Registry ──────────────────────────────────────────────────────────────────

export interface AgentSearchResult {
  name: string;
  version: string;
  description: string;
  capabilities: string[];
  homepage: string | null;
  /**
   * Kind of agent:
   * - `"package"` — installable via `install_agent` Tauri command
   * - `"local_llm"` — a local Ollama model installed via `pull_ollama_model`
   *                  + activated via `set_active_brain`
   *
   * Defaults to `"package"` for backwards-compatibility with older backends.
   */
  kind?: 'package' | 'local_llm';
  /** Ollama model tag (e.g. `"gemma3:4b"`) — only set when `kind === "local_llm"`. */
  model_tag?: string | null;
  /** Approximate minimum RAM required (MB) — only set for local-LLM agents. */
  required_ram_mb?: number | null;
  /** True for the top-recommended local model on this hardware tier. */
  is_top_pick?: boolean;
  /** True for cloud-routed Ollama models (no local RAM needed). */
  is_cloud?: boolean;
}

// ── Sandbox ───────────────────────────────────────────────────────────────────

export type CapabilityName =
  | 'file_read'
  | 'file_write'
  | 'clipboard'
  | 'network'
  | 'process_spawn';

export interface ConsentInfo {
  agent_name: string;
  capability: CapabilityName;
  granted: boolean;
}

// ── Messaging ─────────────────────────────────────────────────────────────────

export interface AgentMessageInfo {
  id: string;
  sender: string;
  topic: string;
  payload: unknown;
  timestamp: number;
}

// ── Window Mode ───────────────────────────────────────────────────────────────

export type WindowMode = 'window' | 'pet';

export interface MonitorInfo {
  name: string | null;
  x: number;
  y: number;
  width: number;
  height: number;
  scale_factor: number;
}

// ── Streaming / Emotion ───────────────────────────────────────────────────────

export type EmotionTag =
  | 'happy'
  | 'sad'
  | 'angry'
  | 'relaxed'
  | 'surprised'
  | 'neutral';

export interface ParsedLlmChunk {
  /** Display text with tags stripped. */
  text: string;
  /** Emotion tag found in this chunk, if any. */
  emotion: EmotionTag | null;
  /** Motion gesture tag found (e.g. 'wave', 'nod'), if any. */
  motion: string | null;
  /** Emoji extracted from JSON-wrapped response, if any. */
  emoji: string | null;
}

// ── Three-Tier Brain ──────────────────────────────────────────────────────────

/** Describes a free LLM API provider from the curated catalogue. */
export interface FreeProvider {
  id: string;
  display_name: string;
  base_url: string;
  model: string;
  rpm_limit: number;
  rpd_limit: number;
  requires_api_key: boolean;
  notes: string;
}

/** The three-tier brain mode configuration. */
export type BrainMode =
  | { mode: 'free_api'; provider_id: string; api_key: string | null }
  | { mode: 'paid_api'; provider: string; api_key: string; model: string; base_url: string }
  | { mode: 'local_ollama'; model: string }
  | {
      mode: 'local_lm_studio';
      model: string;
      base_url: string;
      api_key: string | null;
      embedding_model: string | null;
    };

// ── Coding LLM + Self-Improve (Phase 25) ─────────────────────────────────────

export type CodingLlmProvider = 'anthropic' | 'openai' | 'deepseek' | 'custom';

/** Persisted dedicated coding-LLM configuration. */
export interface CodingLlmConfig {
  provider: CodingLlmProvider;
  model: string;
  base_url: string;
  api_key: string;
}

/** Curated recommendation entry (Claude / OpenAI / DeepSeek / custom). */
export interface CodingLlmRecommendation {
  provider: CodingLlmProvider;
  display_name: string;
  default_model: string;
  base_url: string;
  requires_api_key: boolean;
  notes: string;
  is_top_pick: boolean;
}

/** Self-improve toggle + audit metadata. */
export interface SelfImproveSettings {
  enabled: boolean;
  updated_at: number;
  last_acknowledged_at: number;
  last_provider: string;
}

/** Aggregate observability stats for the self-improve loop. */
export interface SelfImproveMetrics {
  total_runs: number;
  successes: number;
  failures: number;
  success_rate: number;
  failure_rate: number;
  avg_duration_ms: number;
  last_error: string | null;
  last_error_chunk: string | null;
  last_error_at_ms: number;
}

/** One persisted run record from the self-improve JSONL log. */
export interface SelfImproveRun {
  started_at_ms: number;
  finished_at_ms: number;
  chunk_id: string;
  chunk_title: string;
  outcome: 'running' | 'success' | 'failure';
  duration_ms: number;
  provider: string;
  model: string;
  plan_chars: number;
  error: string | null;
}

/**
 * Configurable context-loading rules for every coding workflow
 * (Chunk 25.16). Mirrors the `CodingWorkflowConfig` Rust struct.
 */
export interface CodingWorkflowConfig {
  include_dirs: string[];
  include_files: string[];
  exclude_paths: string[];
  max_file_chars: number;
  max_total_chars: number;
}

/** One row in the coding-workflow live preview. */
export interface CodingWorkflowPreviewDoc {
  label: string;
  char_count: number;
}

/** Aggregate preview returned by `preview_coding_workflow_context`. */
export interface CodingWorkflowPreview {
  documents: CodingWorkflowPreviewDoc[];
  total_chars: number;
  file_count: number;
  repo_root: string;
}

// ── Voice ──────────────────────────────────────────────────────────────────────

/** Metadata describing an available voice provider. */
export interface VoiceProviderInfo {
  id: string;
  display_name: string;
  description: string;
  kind: 'local' | 'cloud';
  requires_api_key: boolean;
}

/** Persisted voice configuration. */
export interface VoiceConfig {
  asr_provider: string | null;
  tts_provider: string | null;
  api_key: string | null;
  endpoint_url: string | null;
}

// ── Provider Health / Rotation ────────────────────────────────────────────────

/** Health and rate-limit status for a single free provider. */
export interface ProviderHealthInfo {
  id: string;
  display_name: string;
  is_healthy: boolean;
  is_rate_limited: boolean;
  requests_sent: number;
  remaining_requests: number | null;
  remaining_tokens: number | null;
  latency_ms: number | null;
}

// ── Vision / Screen Capture ──────────────────────────────────────────────────

export interface ScreenFrame {
  image_b64: string;
  width: number;
  height: number;
  captured_at: number;
  active_window_title: string | null;
}

export interface VisionAnalysis {
  description: string;
  activity: string;
  confidence: number;
  analyzed_at: number;
}

// ── Translation ──────────────────────────────────────────────────────────────

export interface TranslationResult {
  original: string;
  source_lang: string;
  translated: string;
  target_lang: string;
  confidence: number | null;
}

