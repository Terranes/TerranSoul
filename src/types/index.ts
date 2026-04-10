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
