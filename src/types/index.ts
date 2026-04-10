export interface Message {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  agentName?: string;
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
