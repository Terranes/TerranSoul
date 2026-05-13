/**
 * Pure utility helpers used by BrainGraphViewport. Extracted to keep the
 * Vue SFC under the project's max-lines budget.
 */
import type { MemoryEntry } from '../types';
import { classifyCognitiveKind, type CognitiveKind } from '../utils/cognitive-kind';

export function classifyMemoryKind(memory: MemoryEntry): CognitiveKind {
  return classifyCognitiveKind(memory.memory_type, memory.tags ?? '', memory.content);
}

export const COMMUNITY_PALETTE = [
  '#a78bfa',
  '#60a5fa',
  '#2dd4bf',
  '#4ade80',
  '#fbbf24',
  '#fb923c',
  '#f87171',
  '#f472b6',
  '#c084fc',
  '#22d3ee',
  '#bef264',
  '#fde047',
] as const;

export const EDGE_COLOURS = [
  '#93a4bb',
  '#f59e0b',
  '#38bdf8',
  '#fb7185',
  '#34d399',
  '#8b5cf6',
  '#22c55e',
  '#c084fc',
] as const;

export function relTypeColour(rel: string): string {
  let h = 0;
  for (let i = 0; i < rel.length; i++) h = ((h << 5) - h + rel.charCodeAt(i)) | 0;
  return EDGE_COLOURS[Math.abs(h) % EDGE_COLOURS.length];
}

export function safeCoord(v: number | undefined | null): number {
  return (v != null && Number.isFinite(v)) ? v : 0;
}

export function truncate(text: string, max: number): string {
  if (!text) return '';
  return text.length <= max ? text : text.slice(0, max - 1) + '...';
}

export function dominantTag(tagsCsv: string | null | undefined): string {
  const list = (tagsCsv ?? '').split(',').map((tag) => tag.trim()).filter(Boolean);
  const tagged = list.find((tag) => tag.includes(':')) ?? list[0] ?? '';
  return tagged.includes(':') ? tagged.slice(0, tagged.indexOf(':')) : tagged;
}

export function hash(value: string): number {
  let h = 0;
  for (let i = 0; i < value.length; i++) h = (h * 31 + value.charCodeAt(i)) >>> 0;
  return h;
}

export function communityColour(label: string, index = 0): string {
  if (!label) return '#94a3b8';
  return COMMUNITY_PALETTE[(hash(label) + index) % COMMUNITY_PALETTE.length];
}
