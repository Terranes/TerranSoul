<template>
  <div ref="container" class="memory-graph" />
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue';
import cytoscape, { type Core, type ElementDefinition } from 'cytoscape';
import type { MemoryEntry } from '../types';

const props = defineProps<{ memories: MemoryEntry[] }>();
const emit = defineEmits<{ (e: 'select', id: number): void }>();

const container = ref<HTMLDivElement | null>(null);
let cy: Core | null = null;

const TYPE_COLOURS: Record<string, string> = {
  fact: '#60a5fa',
  preference: '#34d399',
  context: '#fbbf24',
  summary: '#c084fc',
};

function buildElements(memories: MemoryEntry[]): ElementDefinition[] {
  const nodes: ElementDefinition[] = memories.map((m) => ({
    data: {
      id: String(m.id),
      label: m.content.length > 40 ? m.content.slice(0, 40) + '…' : m.content,
      fullContent: m.content,
      importance: m.importance,
      memoryType: m.memory_type,
      colour: TYPE_COLOURS[m.memory_type] ?? '#94a3b8',
    },
  }));

  // Edges between memories sharing at least one tag.
  const edges: ElementDefinition[] = [];
  for (let i = 0; i < memories.length; i++) {
    for (let j = i + 1; j < memories.length; j++) {
      const tagsA = memories[i].tags.split(',').map((t) => t.trim()).filter(Boolean);
      const tagsB = memories[j].tags.split(',').map((t) => t.trim()).filter(Boolean);
      const shared = tagsA.filter((t) => tagsB.includes(t));
      if (shared.length > 0) {
        edges.push({
          data: {
            id: `e-${memories[i].id}-${memories[j].id}`,
            source: String(memories[i].id),
            target: String(memories[j].id),
            label: shared.join(', '),
          },
        });
      }
    }
  }

  return [...nodes, ...edges];
}

function init() {
  if (!container.value) return;
  cy?.destroy();

  cy = cytoscape({
    container: container.value,
    elements: buildElements(props.memories),
    style: [
      {
        selector: 'node',
        style: {
          label: 'data(label)',
          'background-color': 'data(colour)',
          width: (el) => 20 + (el.data('importance') as number) * 8,
          height: (el) => 20 + (el.data('importance') as number) * 8,
          color: '#f1f5f9',
          'font-size': '11px',
          'text-valign': 'bottom',
          'text-margin-y': 4,
          'text-wrap': 'ellipsis',
          'text-max-width': '120px',
          'border-width': 2,
          'border-color': '#1e293b',
        },
      },
      {
        selector: 'node:selected',
        style: {
          'border-color': '#f97316',
          'border-width': 3,
        },
      },
      {
        selector: 'edge',
        style: {
          width: 1.5,
          'line-color': '#475569',
          'curve-style': 'bezier',
          opacity: 0.6,
        },
      },
    ],
    layout: { name: 'cose', padding: 40, animate: false },
  });

  cy.on('tap', 'node', (evt) => {
    emit('select', Number(evt.target.id()));
  });
}

onMounted(() => init());
onUnmounted(() => cy?.destroy());
watch(() => props.memories, () => init(), { deep: true });
</script>

<style scoped>
.memory-graph {
  width: 100%;
  height: 100%;
  background: #0f172a;
  border-radius: 8px;
}
</style>
