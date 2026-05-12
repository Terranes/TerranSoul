/**
 * Tests for KnowledgeQuestDialog step navigation and error-stop behavior.
 *
 * These test the pure logic functions mirrored from the component:
 * - allTasksDone, hasFailures, allSucceeded computeds
 * - Step navigation (goBack, goToStep)
 * - Error-stop: auto-advance only when all tasks succeed
 */
import { describe, it, expect } from 'vitest';

// ── Mirror of component task-status logic ────────────────────────────────────

interface MockTask {
  id: string;
  status: 'pending' | 'running' | 'completed' | 'failed';
  processed_items?: number;
  error?: string;
}

function computeAllTasksDone(tasks: MockTask[]): boolean {
  return tasks.length > 0 && tasks.every(t => t.status === 'completed' || t.status === 'failed');
}

function computeFailedTasks(tasks: MockTask[]): MockTask[] {
  return tasks.filter(t => t.status === 'failed');
}

function computeHasFailures(tasks: MockTask[]): boolean {
  return computeFailedTasks(tasks).length > 0;
}

function computeAllSucceeded(tasks: MockTask[]): boolean {
  return computeAllTasksDone(tasks) && !computeHasFailures(tasks);
}

function computeTotalChunks(tasks: MockTask[]): number {
  return tasks.reduce((sum, t) => sum + (t.processed_items ?? 0), 0);
}

// ── Mirror of step navigation logic ─────────────────────────────────────────

function advanceStep(currentStep: number): number {
  return currentStep < 2 ? currentStep + 1 : currentStep;
}

function goBack(currentStep: number): number {
  return currentStep > 0 ? currentStep - 1 : currentStep;
}

function goToStep(currentStep: number, targetIndex: number): number {
  return targetIndex < currentStep ? targetIndex : currentStep;
}

// ── Tests ────────────────────────────────────────────────────────────────────

describe('KnowledgeQuestDialog task status computeds', () => {
  it('allTasksDone is false when no tasks exist', () => {
    expect(computeAllTasksDone([])).toBe(false);
  });

  it('allTasksDone is false when tasks are still running', () => {
    const tasks: MockTask[] = [
      { id: '1', status: 'completed', processed_items: 10 },
      { id: '2', status: 'running' },
    ];
    expect(computeAllTasksDone(tasks)).toBe(false);
  });

  it('allTasksDone is true when all tasks are completed or failed', () => {
    const tasks: MockTask[] = [
      { id: '1', status: 'completed', processed_items: 10 },
      { id: '2', status: 'failed', error: 'file not found' },
    ];
    expect(computeAllTasksDone(tasks)).toBe(true);
  });

  it('hasFailures detects failed tasks', () => {
    const tasks: MockTask[] = [
      { id: '1', status: 'completed', processed_items: 10 },
      { id: '2', status: 'failed', error: 'file not found' },
    ];
    expect(computeHasFailures(tasks)).toBe(true);
  });

  it('hasFailures is false when all tasks succeed', () => {
    const tasks: MockTask[] = [
      { id: '1', status: 'completed', processed_items: 10 },
      { id: '2', status: 'completed', processed_items: 5 },
    ];
    expect(computeHasFailures(tasks)).toBe(false);
  });

  it('allSucceeded is true only when all tasks complete without failures', () => {
    const allGood: MockTask[] = [
      { id: '1', status: 'completed', processed_items: 10 },
      { id: '2', status: 'completed', processed_items: 5 },
    ];
    expect(computeAllSucceeded(allGood)).toBe(true);

    const withFailure: MockTask[] = [
      { id: '1', status: 'completed', processed_items: 10 },
      { id: '2', status: 'failed', error: 'not found' },
    ];
    expect(computeAllSucceeded(withFailure)).toBe(false);
  });

  it('allSucceeded is false when tasks are still running', () => {
    const tasks: MockTask[] = [
      { id: '1', status: 'completed', processed_items: 10 },
      { id: '2', status: 'running' },
    ];
    expect(computeAllSucceeded(tasks)).toBe(false);
  });

  it('totalChunks sums processed_items from all tasks', () => {
    const tasks: MockTask[] = [
      { id: '1', status: 'completed', processed_items: 100 },
      { id: '2', status: 'failed', error: 'err', processed_items: 0 },
      { id: '3', status: 'completed', processed_items: 50 },
    ];
    expect(computeTotalChunks(tasks)).toBe(150);
  });

  it('failedTasks returns only failed tasks', () => {
    const tasks: MockTask[] = [
      { id: '1', status: 'completed', processed_items: 10 },
      { id: '2', status: 'failed', error: 'file not found' },
      { id: '3', status: 'failed', error: 'network error' },
    ];
    const failed = computeFailedTasks(tasks);
    expect(failed).toHaveLength(2);
    expect(failed.map(t => t.id)).toEqual(['2', '3']);
  });
});

describe('KnowledgeQuestDialog step navigation', () => {
  it('advanceStep increments from 0 to 1', () => {
    expect(advanceStep(0)).toBe(1);
  });

  it('advanceStep increments from 1 to 2', () => {
    expect(advanceStep(1)).toBe(2);
  });

  it('advanceStep does not go beyond 2', () => {
    expect(advanceStep(2)).toBe(2);
  });

  it('goBack decrements from 2 to 1', () => {
    expect(goBack(2)).toBe(1);
  });

  it('goBack decrements from 1 to 0', () => {
    expect(goBack(1)).toBe(0);
  });

  it('goBack does not go below 0', () => {
    expect(goBack(0)).toBe(0);
  });

  it('goToStep navigates to a completed step', () => {
    expect(goToStep(2, 0)).toBe(0);
    expect(goToStep(2, 1)).toBe(1);
  });

  it('goToStep does not navigate forward', () => {
    expect(goToStep(1, 2)).toBe(1);
    expect(goToStep(0, 1)).toBe(0);
  });

  it('goToStep to same step stays put', () => {
    expect(goToStep(1, 1)).toBe(1);
  });
});

describe('KnowledgeQuestDialog error-stop auto-advance rules', () => {
  it('should auto-advance when allSucceeded is true at step 1', () => {
    const tasks: MockTask[] = [
      { id: '1', status: 'completed', processed_items: 100 },
    ];
    const done = computeAllTasksDone(tasks);
    const succeeded = computeAllSucceeded(tasks);
    const currentStep = 1;

    // Auto-advance condition
    const shouldAutoAdvance = done && currentStep === 1 && succeeded;
    expect(shouldAutoAdvance).toBe(true);
  });

  it('should NOT auto-advance when hasFailures at step 1', () => {
    const tasks: MockTask[] = [
      { id: '1', status: 'completed', processed_items: 100 },
      { id: '2', status: 'failed', error: 'file not found' },
    ];
    const done = computeAllTasksDone(tasks);
    const succeeded = computeAllSucceeded(tasks);
    const currentStep = 1;

    const shouldAutoAdvance = done && currentStep === 1 && succeeded;
    expect(shouldAutoAdvance).toBe(false);
  });

  it('should NOT auto-advance when all tasks failed', () => {
    const tasks: MockTask[] = [
      { id: '1', status: 'failed', error: 'file not found' },
      { id: '2', status: 'failed', error: 'network error' },
    ];
    const done = computeAllTasksDone(tasks);
    const succeeded = computeAllSucceeded(tasks);

    expect(done).toBe(true);
    expect(succeeded).toBe(false);
  });
});
