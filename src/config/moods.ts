/**
 * Shared mood/pose catalogue consumed by both the pet-mode context menu
 * and the desktop-mode settings dropdown so every surface offers the same
 * configurable poses.
 *
 * 'sitting' is a UI sentinel — it is NOT a `CharacterState`.  Applying it
 * sets `state='idle'` and pins the animator's idle-pose rotation to the
 * seated pose via `characterStore.sittingPinned`.
 */
import type { CharacterState } from '../types';
import type { useCharacterStore } from '../stores/character';

export type MoodKey = CharacterState | 'sitting';

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
  { key: 'sitting',   label: 'Sitting',    emoji: '🛋' },
];

/** Returns true if the given mood is the one currently applied to the character. */
export function isMoodActive(
  mood: MoodEntry,
  characterStore: ReturnType<typeof useCharacterStore>,
): boolean {
  if (mood.key === 'sitting') return characterStore.sittingPinned === true;
  return !characterStore.sittingPinned && characterStore.state === mood.key;
}

/** Apply the given mood to the character store — handles the 'sitting' sentinel. */
export function applyMood(
  mood: MoodEntry,
  characterStore: ReturnType<typeof useCharacterStore>,
): void {
  if (mood.key === 'sitting') {
    characterStore.setSittingPinned(true);
    characterStore.setState('idle');
  } else {
    characterStore.setSittingPinned(false);
    characterStore.setState(mood.key as CharacterState);
  }
}
