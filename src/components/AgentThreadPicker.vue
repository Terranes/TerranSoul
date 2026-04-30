<template>
  <div
    v-if="hasAnyAgent"
    class="agent-thread-picker"
    role="tablist"
    aria-label="Filter chat by agent"
  >
    <button
      type="button"
      role="tab"
      class="agent-thread-pill"
      :class="{ active: isAllActive }"
      :aria-selected="isAllActive"
      @click="selectAll"
    >
      All
    </button>
    <button
      v-for="agent in agentPills"
      :key="agent.id"
      type="button"
      role="tab"
      class="agent-thread-pill"
      :class="{ active: agent.id === currentAgent }"
      :aria-selected="agent.id === currentAgent"
      :title="`Filter to ${agent.label}'s thread (${agent.count} message${agent.count === 1 ? '' : 's'})`"
      @click="selectAgent(agent.id)"
    >
      {{ agent.label }}
      <span class="agent-thread-count">{{ agent.count }}</span>
    </button>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import type { Message } from '../types';
import { useAgentRosterStore } from '../stores/agent-roster';

/**
 * Per-agent chat thread picker (Chunk 23.3).
 *
 * Surfaces the per-agent threads scaffolded in Chunk 23.0
 * (`conversation.ts` → `agentMessages` / `setAgent` / `currentAgent`).
 * The user picks an agent pill to scope the chat view to that agent's
 * messages, or "All" to see the unified history. Pills only render
 * agents that have actually produced at least one message in the
 * current session — so a fresh chat starts blank rather than dumping
 * the entire roster on screen.
 */

const props = defineProps<{
  /** All conversation messages, unfiltered. The picker derives unique
   *  agent IDs from these messages. */
  messages: Message[];
  /** The currently selected agent ID, or `'auto'` for the unfiltered
   *  "All" view. Mirrors `conversation.ts` → `currentAgent.value`. */
  currentAgent: string;
}>();

const emit = defineEmits<{
  /** User picked an agent pill or "All". Pass the new agent ID
   *  (or `'auto'` for "All") so the parent can call
   *  `conversationStore.setAgent(...)`. */
  pick: [agentId: string];
}>();

const rosterStore = useAgentRosterStore();

interface AgentPill {
  id: string;
  label: string;
  count: number;
}

/** Unique agent IDs present in `messages`, with their counts. */
const agentPills = computed<AgentPill[]>(() => {
  const counts = new Map<string, number>();
  for (const msg of props.messages) {
    if (!msg.agentId) continue;
    counts.set(msg.agentId, (counts.get(msg.agentId) ?? 0) + 1);
  }
  return Array.from(counts.entries())
    .map(([id, count]) => ({
      id,
      label: rosterStore.agents.find((a) => a.id === id)?.display_name ?? id,
      count,
    }))
    .sort((a, b) => a.label.localeCompare(b.label));
});

const hasAnyAgent = computed(() => agentPills.value.length > 0);
const isAllActive = computed(() => props.currentAgent === 'auto');

function selectAll(): void {
  emit('pick', 'auto');
}

function selectAgent(id: string): void {
  emit('pick', id);
}
</script>

<style scoped>
.agent-thread-picker {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  padding: 6px 12px;
  border-bottom: 1px solid var(--ts-border-soft, rgba(255, 255, 255, 0.06));
  background: var(--ts-surface-soft, rgba(0, 0, 0, 0.18));
}

.agent-thread-pill {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 12px;
  border: 1px solid var(--ts-border-soft, rgba(255, 255, 255, 0.12));
  border-radius: 999px;
  background: transparent;
  color: var(--ts-text-secondary, rgba(255, 255, 255, 0.65));
  font-size: 0.72rem;
  font-weight: 600;
  letter-spacing: 0.04em;
  cursor: pointer;
  transition: background-color 120ms ease, color 120ms ease, border-color 120ms ease;
}

.agent-thread-pill:hover {
  color: var(--ts-text-primary, #fff);
  border-color: var(--ts-accent-violet, #a78bfa);
}

.agent-thread-pill.active {
  background: var(--ts-accent-violet, #a78bfa);
  border-color: var(--ts-accent-violet, #a78bfa);
  color: #fff;
}

.agent-thread-count {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 18px;
  height: 16px;
  padding: 0 5px;
  border-radius: 999px;
  background: rgba(0, 0, 0, 0.25);
  font-size: 0.65rem;
  font-weight: 700;
}

.agent-thread-pill.active .agent-thread-count {
  background: rgba(255, 255, 255, 0.22);
}
</style>
