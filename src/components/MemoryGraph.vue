<template>
  <div
    ref="container"
    class="memory-graph"
  />
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue';
import cytoscape, { type Core, type ElementDefinition, type NodeSingular } from 'cytoscape';
import type { MemoryEdge, MemoryEntry } from '../types';

const props = withDefaults(
  defineProps<{
    memories: MemoryEntry[];
    /** Typed edges from the V5 entity-relationship graph. When non-empty
     *  these replace the implicit tag-overlap edges. */
    edges?: MemoryEdge[];
    /** Edge mode: 'typed' uses props.edges, 'tag' computes shared-tag edges,
     *  'both' overlays them (tag edges shown faded). */
    edgeMode?: 'typed' | 'tag' | 'both';
  }>(),
  { edges: () => [], edgeMode: 'typed' },
);
const emit = defineEmits<{
  (e: 'select', id: number): void;
  (e: 'select-edge', id: number): void;
}>();

const container = ref<HTMLDivElement | null>(null);
let cy: Core | null = null;

const TYPE_COLOURS: Record<string, string> = {
  fact: '#60a5fa',
  preference: '#34d399',
  context: '#fbbf24',
  summary: '#c084fc',
};

/** Stable colour per relation type using a small hash. Keeps the same edge
 *  type the same colour across renders for visual continuity. */
function relTypeColour(rel: string): string {
  const palette = [
    '#f97316', '#22d3ee', '#a3e635', '#f472b6',
    '#fb7185', '#facc15', '#38bdf8', '#a78bfa',
  ];
  let h = 0;
  for (let i = 0; i < rel.length; i++) h = (h * 31 + rel.charCodeAt(i)) >>> 0;
  return palette[h % palette.length];
}

function buildElements(memories: MemoryEntry[], edges: MemoryEdge[]): ElementDefinition[] {
  const knownIds = new Set(memories.map((m) => m.id));
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

  const useTyped = (props.edgeMode === 'typed' || props.edgeMode === 'both') && edges.length > 0;
  const useTag = props.edgeMode === 'tag' || props.edgeMode === 'both' || (!useTyped && edges.length === 0);

  const edgeEls: ElementDefinition[] = [];

  if (useTyped) {
    for (const e of edges) {
      // Drop edges whose endpoints aren't in the visible memories slice.
      if (!knownIds.has(e.src_id) || !knownIds.has(e.dst_id)) continue;
      edgeEls.push({
        data: {
          id: `te-${e.id}`,
          source: String(e.src_id),
          target: String(e.dst_id),
          label: e.rel_type,
          relType: e.rel_type,
          confidence: e.confidence,
          edgeSource: e.source,
          colour: relTypeColour(e.rel_type),
          edgeKind: 'typed',
        },
      });
    }
  }

  if (useTag) {
    // Implicit edges between memories sharing at least one tag.
    for (let i = 0; i < memories.length; i++) {
      for (let j = i + 1; j < memories.length; j++) {
        const tagsA = memories[i].tags.split(',').map((t) => t.trim()).filter(Boolean);
        const tagsB = memories[j].tags.split(',').map((t) => t.trim()).filter(Boolean);
        const shared = tagsA.filter((t) => tagsB.includes(t));
        if (shared.length > 0) {
          edgeEls.push({
            data: {
              id: `e-${memories[i].id}-${memories[j].id}`,
              source: String(memories[i].id),
              target: String(memories[j].id),
              label: shared.join(', '),
              colour: '#475569',
              edgeKind: 'tag',
            },
          });
        }
      }
    }
  }

  return [...nodes, ...edgeEls];
}

function init() {
  if (!container.value) return;
  cy?.destroy();

  cy = cytoscape({
    container: container.value,
    elements: buildElements(props.memories, props.edges ?? []),
    style: [
      {
        selector: 'node',
        style: {
          label: 'data(label)',
          'background-color': 'data(colour)',
          width: (el: NodeSingular) => 20 + (el.data('importance') as number) * 8,
          height: (el: NodeSingular) => 20 + (el.data('importance') as number) * 8,
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
      // Tag-overlap edges — undirected, faint.
      {
        selector: 'edge[edgeKind = "tag"]',
        style: {
          width: 1,
          'line-color': 'data(colour)',
          'curve-style': 'bezier',
          opacity: 0.35,
        },
      },
      // Typed entity-relationship edges — directed, coloured, labelled.
      {
        selector: 'edge[edgeKind = "typed"]',
        style: {
          width: 2,
          'line-color': 'data(colour)',
          'target-arrow-color': 'data(colour)',
          'target-arrow-shape': 'triangle',
          'curve-style': 'bezier',
          opacity: 0.85,
          label: 'data(label)',
          color: '#cbd5f5',
          'font-size': '9px',
          'text-background-color': '#0f172a',
          'text-background-opacity': 0.7,
          'text-background-padding': '2px',
          'text-rotation': 'autorotate',
        },
      },
      {
        selector: 'edge:selected',
        style: {
          width: 4,
          opacity: 1,
        },
      },
    ],
    layout: { name: 'cose', padding: 40, animate: false },
  });

  cy.on('tap', 'node', (evt) => {
    emit('select', Number(evt.target.id()));
  });
  cy.on('tap', 'edge[edgeKind = "typed"]', (evt) => {
    const raw = String(evt.target.id());
    if (raw.startsWith('te-')) emit('select-edge', Number(raw.slice(3)));
  });
}

onMounted(() => init());
onUnmounted(() => cy?.destroy());
watch(() => props.memories, () => init(), { deep: true });
watch(() => props.edges, () => init(), { deep: true });
watch(() => props.edgeMode, () => init());
</script>

<style scoped>
.memory-graph {
  width: 100%;
  height: 100%;
  background: #0f172a;
  border-radius: 8px;
}
</style>
