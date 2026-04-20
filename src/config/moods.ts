/**
 * Shared mood/pose catalogue consumed by both the pet-mode context menu
 * and the desktop-mode settings dropdown so every surface offers the same
 * configurable poses.
 */
import type { CharacterState } from '../types';
import type { useCharacterStore } from '../stores/character';

export type MoodKey = CharacterState;

export interface MoodEntry {
  key: MoodKey;
  label: string;
  emoji: string;
}

export const MOOD_ENTRIES: MoodEntry[] = [
  { key: 'idle',      label: 'Idle',       emoji: '🙂' },
  { key: 'thinking',  label: 'Thinking',   emoji: '🤔' },
  { key: 'talking',   label: 'Talking',    emoji: '💬' },
  { key: 'happy',     label: 'Happy',      emoji: '😊' },
  { key: 'sad',       label: 'Sad',        emoji: '😢' },
  { key: 'angry',     label: 'Angry',      emoji: '😠' },
  { key: 'relaxed',   label: 'Relaxed',    emoji: '😌' },
  { key: 'surprised', label: 'Surprised',  emoji: '😮' },
];

/** Returns true if the given mood is the one currently applied to the character. */
export function isMoodActive(
  mood: MoodEntry,
  characterStore: ReturnType<typeof useCharacterStore>,
): boolean {
  return characterStore.state === mood.key;
}

/** Apply the given mood to the character store. */
export function applyMood(
  mood: MoodEntry,
  characterStore: ReturnType<typeof useCharacterStore>,
): void {
  characterStore.setState(mood.key as CharacterState);
}
