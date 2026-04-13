export interface Message {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  agentName?: string;
  sentiment?: 'happy' | 'sad' | 'angry' | 'relaxed' | 'surprised' | 'neutral';
  timestamp: number;
}

export type CharacterState = 'idle' | 'thinking' | 'talking' | 'happy' | 'sad' | 'angry' | 'relaxed' | 'surprised';

export type AnimationPersona = 'witch' | 'idol';

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
}

export interface ModelRecommendation {
  model_tag: string;
  display_name: string;
  description: string;
  required_ram_mb: number;
  is_top_pick: boolean;
}

export interface OllamaStatus {
  running: boolean;
  model_count: number;
}

export interface OllamaModelEntry {
  name: string;
  size: number;
}

// ── Memory ────────────────────────────────────────────────────────────────────

export type MemoryType = 'fact' | 'preference' | 'context' | 'summary';

export interface MemoryEntry {
  id: number;
  content: string;
  tags: string;
  importance: number;
  memory_type: MemoryType;
  created_at: number;
  last_accessed: number | null;
  access_count: number;
}

export interface NewMemory {
  content: string;
  tags: string;
  importance: number;
  memory_type: MemoryType;
}

// ── Registry ──────────────────────────────────────────────────────────────────

export interface AgentSearchResult {
  name: string;
  version: string;
  description: string;
  capabilities: string[];
  homepage: string | null;
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

export type MotionTag = string; // e.g. 'wave', 'nod', 'bow'

export interface ParsedLlmChunk {
  /** Display text with tags stripped. */
  text: string;
  /** Emotion tag found in this chunk, if any. */
  emotion: EmotionTag | null;
  /** Motion tag found in this chunk, if any. */
  motion: MotionTag | null;
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
  | { mode: 'local_ollama'; model: string };

