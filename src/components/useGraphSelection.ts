/**
 * Shared search + persistent-selection state used by both the 2D MemoryGraph
 * and the 3D BrainGraphViewport so their side panels behave identically.
 *
 * The contract:
 *  - `searchText` + `searchMode` + `searchFields` drive a live label filter.
 *  - When `highlightFilterActive` is true, current matches are added to the
 *    `highlightedIds` set in real time.
 *  - When `highlightFilterActive` is false, only the persistent
 *    `selectedIds` set contributes — so previously highlighted nodes stay
 *    highlighted after the user turns the filter off.
 *  - `selectMatches` / `addMatches` / `removeMatches` are the three explicit
 *    verbs the panel exposes; they map to Enter, Shift+Enter, Alt+Enter on
 *    the search input.
 */
import { computed, reactive, ref, type ComputedRef, type Ref } from 'vue';

export type GraphSearchMode = 'contains' | 'starts' | 'ends';

/** Which of a node's text fields participate in the search query. */
export interface GraphSearchFields {
  label: boolean;
  tags: boolean;
  body: boolean;
  community: boolean;
}

export interface SearchableNode {
  id: number;
  /** Short label / truncated content shown on the node. */
  label: string;
  /** Comma-separated tag list. */
  tags?: string;
  /** Full memory body / long content. */
  body?: string;
  /** Cluster / community / memory-type label. */
  community?: string;
}

export interface GraphSelectionState<TNode extends SearchableNode> {
  searchText: Ref<string>;
  searchMode: Ref<GraphSearchMode>;
  searchFields: GraphSearchFields;
  highlightFilterActive: Ref<boolean>;
  selectedIds: Ref<Set<number>>;
  matchedIds: ComputedRef<Set<number>>;
  highlightedIds: ComputedRef<Set<number>>;
  matchCount: ComputedRef<number>;
  selectedCount: ComputedRef<number>;
  /** Replace selection with the current matches. (Enter) */
  selectMatches(): void;
  /** Add current matches to the selection. (Shift+Enter, or button) */
  addMatches(): void;
  /** Remove current matches from the selection. (Alt+Enter, or button) */
  removeMatches(): void;
  /** Add every node in the source list to the selection. */
  selectAll(nodes: readonly TNode[]): void;
  /** Drop the entire persistent selection. */
  clearSelection(): void;
  /** Flip a single id in/out of the selection. */
  toggleSelected(id: number): void;
  /** Add every id in `ids` to the selection. */
  addIds(ids: Iterable<number>): void;
  /** Remove every id in `ids` from the selection. */
  removeIds(ids: Iterable<number>): void;
  /** True if a node matches the current query under the current scope. */
  matches(node: TNode): boolean;
}

function fieldHaystacks(node: SearchableNode, fields: GraphSearchFields): string[] {
  const out: string[] = [];
  if (fields.label && node.label) out.push(node.label.toLowerCase());
  if (fields.tags && node.tags) out.push(node.tags.toLowerCase());
  if (fields.body && node.body) out.push(node.body.toLowerCase());
  if (fields.community && node.community) out.push(node.community.toLowerCase());
  return out;
}

function nodeMatchesQuery(
  node: SearchableNode,
  query: string,
  mode: GraphSearchMode,
  fields: GraphSearchFields,
): boolean {
  if (!query) return false;
  const needle = query.toLowerCase();
  const candidates = fieldHaystacks(node, fields);
  if (candidates.length === 0) return false;
  if (mode === 'contains') return candidates.some((c) => c.includes(needle));
  if (mode === 'starts') return candidates.some((c) => c.startsWith(needle));
  return candidates.some((c) => c.endsWith(needle));
}

export function createGraphSelection<TNode extends SearchableNode>(
  nodesSource: () => readonly TNode[],
): GraphSelectionState<TNode> {
  const searchText = ref('');
  const searchMode = ref<GraphSearchMode>('contains');
  // Defaults: label + tags + community on; body off (body is slow to scan on
  // big graphs and rarely what users want when fuzzy-finding by name).
  const searchFields = reactive<GraphSearchFields>({
    label: true,
    tags: true,
    body: false,
    community: true,
  });
  const highlightFilterActive = ref(true);
  const selectedIds = ref<Set<number>>(new Set());

  const matches = (node: TNode) =>
    nodeMatchesQuery(node, searchText.value.trim(), searchMode.value, searchFields);

  const matchedIds = computed<Set<number>>(() => {
    const q = searchText.value.trim();
    if (!q) return new Set();
    const out = new Set<number>();
    for (const n of nodesSource()) {
      if (nodeMatchesQuery(n, q, searchMode.value, searchFields)) out.add(n.id);
    }
    return out;
  });

  const highlightedIds = computed<Set<number>>(() => {
    const out = new Set<number>(selectedIds.value);
    if (highlightFilterActive.value) {
      for (const id of matchedIds.value) out.add(id);
    }
    return out;
  });

  function setSelection(next: Set<number>) {
    selectedIds.value = next;
  }

  return {
    searchText,
    searchMode,
    searchFields,
    highlightFilterActive,
    selectedIds,
    matchedIds,
    highlightedIds,
    matchCount: computed(() => matchedIds.value.size),
    selectedCount: computed(() => selectedIds.value.size),
    matches,
    selectMatches() {
      setSelection(new Set(matchedIds.value));
    },
    addMatches() {
      if (matchedIds.value.size === 0) return;
      const next = new Set(selectedIds.value);
      for (const id of matchedIds.value) next.add(id);
      setSelection(next);
    },
    removeMatches() {
      if (matchedIds.value.size === 0 || selectedIds.value.size === 0) return;
      const next = new Set(selectedIds.value);
      for (const id of matchedIds.value) next.delete(id);
      setSelection(next);
    },
    selectAll(nodes) {
      if (nodes.length === 0) return;
      const next = new Set(selectedIds.value);
      for (const n of nodes) next.add(n.id);
      setSelection(next);
    },
    clearSelection() {
      if (selectedIds.value.size === 0) return;
      setSelection(new Set());
    },
    toggleSelected(id) {
      const next = new Set(selectedIds.value);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      setSelection(next);
    },
    addIds(ids) {
      const next = new Set(selectedIds.value);
      for (const id of ids) next.add(id);
      setSelection(next);
    },
    removeIds(ids) {
      if (selectedIds.value.size === 0) return;
      const next = new Set(selectedIds.value);
      for (const id of ids) next.delete(id);
      setSelection(next);
    },
  };
}
