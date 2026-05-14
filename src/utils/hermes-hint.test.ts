import { describe, it, expect } from 'vitest';
import {
  classifyHermesIntent,
  shouldShowHermesHint,
  HERMES_HINT_TOKEN_THRESHOLD,
} from './hermes-hint';

describe('hermes-hint', () => {
  describe('classifyHermesIntent', () => {
    it('returns null for short messages', () => {
      expect(classifyHermesIntent('hi')).toBeNull();
      expect(classifyHermesIntent('')).toBeNull();
    });

    it('detects deep_research intent', () => {
      expect(classifyHermesIntent('I need you to do deep research on quantum computing across all available papers')).toBe('deep_research');
      expect(classifyHermesIntent('Please do a comprehensive analysis of market trends')).toBe('deep_research');
      expect(classifyHermesIntent('Can you do a literature review of recent ML papers?')).toBe('deep_research');
    });

    it('detects long_running_workflow intent', () => {
      expect(classifyHermesIntent('Schedule a cron job that processes data overnight')).toBe('long_running_workflow');
      expect(classifyHermesIntent('I need a long-running task that monitors the API continuously')).toBe('long_running_workflow');
      expect(classifyHermesIntent('Set up a batch process to run nightly')).toBe('long_running_workflow');
    });

    it('detects full_ide_coding intent', () => {
      expect(classifyHermesIntent('Refactor the entire codebase to use TypeScript strict mode')).toBe('full_ide_coding');
      expect(classifyHermesIntent('I need to edit dozens of files to rename this module')).toBe('full_ide_coding');
      expect(classifyHermesIntent('Can you do a multi-file refactor across the whole repo?')).toBe('full_ide_coding');
    });

    it('returns null for regular chat messages', () => {
      expect(classifyHermesIntent('What is the weather like today in New York?')).toBeNull();
      expect(classifyHermesIntent('Tell me a joke about programming languages')).toBeNull();
      expect(classifyHermesIntent('How do I write a React component for a form?')).toBeNull();
    });
  });

  describe('shouldShowHermesHint', () => {
    const longResponse = 'a'.repeat(16000); // ~4000 tokens of response alone

    it('shows hint when all conditions met', () => {
      const result = shouldShowHermesHint(
        'I need comprehensive research across all available papers on quantum computing',
        longResponse,
        true,  // hermes_hint_enabled
        false, // hermes not already configured
      );
      expect(result.show).toBe(true);
      expect(result.intent).toBe('deep_research');
      expect(result.turnTokens).toBeGreaterThanOrEqual(HERMES_HINT_TOKEN_THRESHOLD);
    });

    it('does not show when setting is disabled', () => {
      const result = shouldShowHermesHint(
        'Do deep research on quantum computing for me please',
        longResponse,
        false, // disabled
        false,
      );
      expect(result.show).toBe(false);
    });

    it('does not show when Hermes already configured', () => {
      const result = shouldShowHermesHint(
        'Do deep research on quantum computing for me please',
        longResponse,
        true,
        true, // already configured
      );
      expect(result.show).toBe(false);
    });

    it('does not show when tokens below threshold', () => {
      const result = shouldShowHermesHint(
        'Do deep research on quantum computing for me please',
        'Short response.', // way below threshold
        true,
        false,
      );
      expect(result.show).toBe(false);
    });

    it('does not show when intent does not match', () => {
      const result = shouldShowHermesHint(
        'Tell me a joke please',
        longResponse,
        true,
        false,
      );
      expect(result.show).toBe(false);
    });

    it('respects custom threshold', () => {
      const result = shouldShowHermesHint(
        'Do deep research on quantum computing for me please',
        'x'.repeat(800), // ~200 tokens total with user msg
        true,
        false,
        100, // very low threshold
      );
      expect(result.show).toBe(true);
      expect(result.intent).toBe('deep_research');
    });
  });
});
