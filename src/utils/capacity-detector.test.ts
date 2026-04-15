import { describe, it, expect, beforeEach } from 'vitest';
import { assessCapacity, resetCapacityTracking } from './capacity-detector';

describe('capacity-detector', () => {
  beforeEach(() => {
    resetCapacityTracking();
  });

  it('marks a normal response as good quality', () => {
    const signal = assessCapacity(
      'Here is a detailed explanation of how photosynthesis works in plants.',
      'How does photosynthesis work?',
    );
    expect(signal.isLowQuality).toBe(false);
    expect(signal.shouldSuggestUpgrade).toBe(false);
  });

  it('marks a very short response as low quality', () => {
    const signal = assessCapacity('I don\'t know.', 'Explain quantum physics');
    expect(signal.isLowQuality).toBe(true);
  });

  it('detects incapability patterns', () => {
    const signal = assessCapacity(
      'I\'m sorry, I can\'t help you with that particular request at this time.',
      'Write me a complex algorithm',
    );
    expect(signal.isLowQuality).toBe(true);
  });

  it('detects "beyond my capabilities" pattern', () => {
    const signal = assessCapacity(
      'That is beyond my capabilities. You might want to use a more advanced model.',
      'Debug this Rust code',
    );
    expect(signal.isLowQuality).toBe(true);
  });

  it('does not suggest upgrade after one low-quality response', () => {
    const signal = assessCapacity('Sorry, I cannot do that.', 'Complex task');
    expect(signal.isLowQuality).toBe(true);
    expect(signal.shouldSuggestUpgrade).toBe(false);
  });

  it('suggests upgrade after multiple low-quality responses', () => {
    assessCapacity('I can\'t help with that.', 'Task 1');
    const signal = assessCapacity('Unfortunately, I cannot assist.', 'Task 2');
    expect(signal.shouldSuggestUpgrade).toBe(true);
    expect(signal.recentLowCount).toBe(2);
  });

  it('mixes good and bad without triggering upgrade', () => {
    assessCapacity('I can\'t help with that.', 'Hard task');
    assessCapacity(
      'Photosynthesis is the process by which plants convert sunlight into energy using chlorophyll.',
      'Easy task',
    );
    const signal = assessCapacity(
      'The capital of France is Paris, a beautiful city known for the Eiffel Tower.',
      'Another easy task',
    );
    expect(signal.shouldSuggestUpgrade).toBe(false);
  });

  it('resets tracking window', () => {
    assessCapacity('I can\'t help.', 'Task 1');
    assessCapacity('Sorry, I cannot.', 'Task 2');
    resetCapacityTracking();
    const signal = assessCapacity('I can\'t help.', 'Task 3');
    expect(signal.recentLowCount).toBe(1);
    expect(signal.shouldSuggestUpgrade).toBe(false);
  });

  it('sliding window drops old entries', () => {
    // Fill window with 5 good responses
    for (let i = 0; i < 5; i++) {
      assessCapacity(
        'Here is a detailed and helpful answer to your question about the topic.',
        `Good question ${i}`,
      );
    }
    // Add 2 bad ones (pushes out old good ones)
    assessCapacity('I can\'t do that.', 'Bad 1');
    const signal = assessCapacity('Sorry, I cannot help.', 'Bad 2');
    expect(signal.shouldSuggestUpgrade).toBe(true);
  });
});
