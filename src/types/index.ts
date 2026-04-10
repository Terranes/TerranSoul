export interface Message {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  agentName?: string;
  sentiment?: 'happy' | 'sad' | 'neutral';
  timestamp: number;
}

export type CharacterState = 'idle' | 'thinking' | 'talking' | 'happy' | 'sad';

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
