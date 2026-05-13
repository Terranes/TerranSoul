import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export interface TaskInfo {
  id: string;
  kind: 'ingest' | 'crawl' | 'quest' | 'extract' | 'model_pull' | 'custom';
  status: 'running' | 'paused' | 'completed' | 'failed' | 'cancelled';
  progress: number;
  description: string;
  processed_items: number;
  total_items: number;
  error: string | null;
}

export interface CrawlIngestOptions {
  crawlDepth?: number;
  crawlMaxPages?: number;
}

export const useTaskStore = defineStore('tasks', () => {
  const tasks = ref<Map<string, TaskInfo>>(new Map());
  let unlistenProgress: (() => void) | null = null;

  const activeTasks = computed(() =>
    [...tasks.value.values()].filter(t => t.status === 'running' || t.status === 'paused')
  );

  const isAgentBusy = computed(() =>
    [...tasks.value.values()].some(t => t.status === 'running')
  );

  const runningTask = computed(() =>
    [...tasks.value.values()].find(t => t.status === 'running') ?? null
  );

  async function init() {
    try {
      const unlisten = await listen<TaskInfo>('task-progress', (event) => {
        const t = event.payload;
        tasks.value = new Map(tasks.value).set(t.id, t);
      });
      unlistenProgress = unlisten;
    } catch {
      // Browser mode — no Tauri events
    }
    await loadTasks();
  }

  async function loadTasks() {
    try {
      const all = await invoke<TaskInfo[]>('get_all_tasks');
      const map = new Map<string, TaskInfo>();
      for (const t of all) map.set(t.id, t);
      tasks.value = map;
    } catch {
      // Backend unavailable
    }
  }

  async function ingestDocument(source: string, tags?: string, importance?: number, options?: CrawlIngestOptions) {
    try {
      const payload: Record<string, unknown> = { source, tags, importance };
      if (options?.crawlDepth != null) payload.crawlDepth = options.crawlDepth;
      if (options?.crawlMaxPages != null) payload.crawlMaxPages = options.crawlMaxPages;
      const result = await invoke<{ task_id: string; source: string; source_type: string }>(
        'ingest_document', payload,
      );
      return result;
    } catch (e) {
      throw new Error(String(e));
    }
  }

  async function cancelTask(taskId: string) {
    await invoke('cancel_ingest_task', { taskId });
  }

  async function resumeTask(taskId: string) {
    await invoke('resume_ingest_task', { taskId });
  }

  function cleanup() {
    if (unlistenProgress) {
      unlistenProgress();
      unlistenProgress = null;
    }
  }

  return {
    tasks, activeTasks, isAgentBusy, runningTask,
    init, loadTasks, ingestDocument, cancelTask, resumeTask, cleanup,
  };
});
