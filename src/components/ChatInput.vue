<template>
  <form
    class="chat-input-bar"
    @submit.prevent="handleSubmit"
    @dragover.prevent="dragOver = true"
    @dragleave="dragOver = false"
    @drop.prevent="handleDrop"
  >
    <!-- Slash command suggestions dropdown -->
    <Transition name="fade-up">
      <div
        v-if="showSuggestions && filteredSuggestions.length > 0"
        class="slash-suggestions"
      >
        <button
          v-for="(cmd, idx) in filteredSuggestions"
          :key="cmd.name"
          type="button"
          class="suggestion-item"
          :class="{ active: idx === selectedSuggestionIdx }"
          @mousedown.prevent="selectSuggestion(cmd)"
        >
          <span class="suggestion-name">/{{ cmd.name }}</span>
          <span class="suggestion-desc">{{ cmd.description }}</span>
        </button>
      </div>
    </Transition>
    <div
      class="input-wrapper"
      :class="{ 'drag-over': dragOver }"
    >
      <button
        type="button"
        class="attach-btn"
        aria-label="Attach file"
        @click="openFilePicker"
      >
        <svg
          width="18"
          height="18"
          viewBox="0 0 24 24"
          fill="currentColor"
        >
          <path d="M16.5 6v11.5a4 4 0 0 1-8 0V5a2.5 2.5 0 0 1 5 0v10.5a1 1 0 0 1-2 0V6h-1v9.5a2 2 0 0 0 4 0V5a3.5 3.5 0 0 0-7 0v12.5a5 5 0 0 0 10 0V6h-1z" />
        </svg>
      </button>
      <textarea
        ref="inputRef"
        v-model="inputText"
        class="chat-input"
        :class="{ 'is-thinking-hint': thinking && !inputText }"
        :placeholder="activePlaceholder"
        rows="1"
        autocomplete="off"
        :style="{ height: textareaHeight }"
        @focus="handleFocus"
        @blur="handleBlur"
        @keydown="handleKeydown"
        @input="autoResize"
      />
      <div
        v-if="messageHistory.length > 0"
        class="history-nav"
      >
        <button
          type="button"
          class="history-btn"
          :disabled="historyIndex === 0 || (historyIndex === -1 && messageHistory.length === 0)"
          aria-label="Previous message"
          @click="navigateHistory(-1)"
        >
          <svg
            width="12"
            height="12"
            viewBox="0 0 24 24"
            fill="currentColor"
          ><path d="M7.41 15.41L12 10.83l4.59 4.58L18 14l-6-6-6 6z" /></svg>
        </button>
        <button
          type="button"
          class="history-btn"
          :disabled="historyIndex === -1"
          aria-label="Next message"
          @click="navigateHistory(1)"
        >
          <svg
            width="12"
            height="12"
            viewBox="0 0 24 24"
            fill="currentColor"
          ><path d="M7.41 8.59L12 13.17l4.59-4.58L18 10l-6 6-6-6z" /></svg>
        </button>
      </div>
      <button
        type="submit"
        class="send-btn"
        :disabled="disabled || !inputText.trim()"
        aria-label="Send"
      >
        <svg
          width="18"
          height="18"
          viewBox="0 0 24 24"
          fill="currentColor"
        >
          <path d="M2.01 21L23 12 2.01 3 2 10l15 2-15 2z" />
        </svg>
      </button>
    </div>
    <input
      ref="fileInputRef"
      type="file"
      class="hidden-file-input"
      accept=".md,.txt,.csv,.json,.xml,.html,.htm,.log,.rst,.adoc,.pdf"
      @change="handleFileSelected"
    >
  </form>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted, onUnmounted } from 'vue';
import { burstResetScroll } from '../utils/scroll-reset';
import { usePromptCommandsStore } from '../stores/prompt-commands';
import { useTaskStore } from '../stores/tasks';

const props = defineProps<{
  disabled: boolean;
  /**
   * When true, replace the placeholder with an animated grey
   * "Thinking[dots] - You can expand chat history to see thinking details"
   * hint that cycles `.`/`..`/`...` every ~400 ms. The textarea stays
   * usable so the user can queue the next message.
   */
  thinking?: boolean;
}>();
const emit = defineEmits<{ submit: [message: string]; focus: []; blur: [] }>();

const inputText = ref('');
const inputRef = ref<HTMLTextAreaElement | null>(null);
const fileInputRef = ref<HTMLInputElement | null>(null);
const dragOver = ref(false);
const selectedSuggestionIdx = ref(0);
const textareaHeight = ref('auto');

/** ── Message history (up/down recall) ─────────────────────────────────── */
const messageHistory = ref<string[]>([]);
const historyIndex = ref(-1);
const savedDraft = ref('');
const MAX_HISTORY = 50;

/** Default placeholder shown when the assistant is idle. */
const IDLE_PLACEHOLDER = 'Type a message…  (Shift+Enter for newline)';

/** Animated dots for the thinking hint — cycles 0→1→2→3 every 400 ms. */
const dotCount = ref(0);
let dotTimer: ReturnType<typeof setInterval> | null = null;

watch(
  () => props.thinking,
  (active) => {
    if (active) {
      dotCount.value = 0;
      if (!dotTimer) {
        dotTimer = setInterval(() => {
          dotCount.value = (dotCount.value + 1) % 4;
        }, 400);
      }
    } else if (dotTimer) {
      clearInterval(dotTimer);
      dotTimer = null;
      dotCount.value = 0;
    }
  },
  { immediate: true },
);

onUnmounted(() => {
  if (dotTimer) {
    clearInterval(dotTimer);
    dotTimer = null;
  }
});

/**
 * The placeholder shown in the textarea. While the assistant is thinking
 * we surface a hint that the chat history can be expanded to inspect the
 * reasoning trace; otherwise we fall back to the standard prompt.
 */
const activePlaceholder = computed(() => {
  if (!props.thinking) return IDLE_PLACEHOLDER;
  const dots = '.'.repeat(dotCount.value);
  return `Thinking${dots} - Expand chat history for details`;
});

/** Default height and growth cap for the chat composer. */
const MIN_TEXTAREA_ROWS = 1;
const MAX_TEXTAREA_ROWS = 3;

const promptCommandsStore = usePromptCommandsStore();

/** Built-in commands to show in suggestions. */
const builtInCommands = [
  { name: 'commands', description: 'List all available commands' },
  { name: 'reflect', description: 'Reflect on the current session' },
];

/** Whether to show the slash command suggestion dropdown. */
const showSuggestions = computed(() => {
  const text = inputText.value;
  return text.startsWith('/') && !text.includes(' ');
});

/** Filter suggestions based on what the user has typed. */
const filteredSuggestions = computed(() => {
  const text = inputText.value.slice(1).toLowerCase();
  const allCommands = [
    ...builtInCommands,
    ...promptCommandsStore.activeCommands.map((c) => ({ name: c.name, description: c.description })),
  ];
  if (!text) return allCommands;
  return allCommands.filter((c) => c.name.toLowerCase().startsWith(text));
});

/** Reset selection index when suggestions change. */
watch(filteredSuggestions, () => {
  selectedSuggestionIdx.value = 0;
});

function selectSuggestion(cmd: { name: string }) {
  inputText.value = `/${cmd.name} `;
  inputRef.value?.focus();
}

function handleKeydown(e: KeyboardEvent) {
  // Submit on Enter (without Shift). Shift+Enter inserts a newline.
  if (e.key === 'Enter' && !e.shiftKey && !showSuggestions.value) {
    e.preventDefault();
    handleSubmit();
    return;
  }

  // ── Message history navigation (up/down) ──
  if (!showSuggestions.value && messageHistory.value.length > 0) {
    if (e.key === 'ArrowUp') {
      const el = inputRef.value;
      // Only navigate history when cursor is on the first line
      if (el && el.selectionStart === el.selectionEnd) {
        const textBefore = el.value.slice(0, el.selectionStart);
        if (!textBefore.includes('\n')) {
          e.preventDefault();
          navigateHistory(-1);
          return;
        }
      }
    }
    if (e.key === 'ArrowDown') {
      const el = inputRef.value;
      // Only navigate history when cursor is on the last line
      if (el && el.selectionStart === el.selectionEnd) {
        const textAfter = el.value.slice(el.selectionStart);
        if (!textAfter.includes('\n')) {
          e.preventDefault();
          navigateHistory(1);
          return;
        }
      }
    }
  }

  if (!showSuggestions.value || filteredSuggestions.value.length === 0) return;

  if (e.key === 'ArrowDown') {
    e.preventDefault();
    selectedSuggestionIdx.value =
      (selectedSuggestionIdx.value + 1) % filteredSuggestions.value.length;
  } else if (e.key === 'ArrowUp') {
    e.preventDefault();
    selectedSuggestionIdx.value =
      (selectedSuggestionIdx.value - 1 + filteredSuggestions.value.length) %
      filteredSuggestions.value.length;
  } else if (e.key === 'Tab' || (e.key === 'Enter' && showSuggestions.value)) {
    if (inputText.value.trim() !== `/${filteredSuggestions.value[selectedSuggestionIdx.value]?.name}`) {
      e.preventDefault();
      const cmd = filteredSuggestions.value[selectedSuggestionIdx.value];
      if (cmd) selectSuggestion(cmd);
    }
  }
}

/**
 * Auto-resize the textarea to fit its content, up to MAX_TEXTAREA_ROWS rows.
 * Beyond that, it scrolls vertically.
 */
function autoResize() {
  const el = inputRef.value;
  if (!el) return;
  // Reset to auto so scrollHeight reflects the natural content height.
  el.style.height = 'auto';
  const styles = window.getComputedStyle(el);
  const lineHeight = parseFloat(styles.lineHeight) || 20;
  const paddingTop = parseFloat(styles.paddingTop) || 0;
  const paddingBottom = parseFloat(styles.paddingBottom) || 0;
  const minHeight = lineHeight * MIN_TEXTAREA_ROWS + paddingTop + paddingBottom;
  const maxHeight = lineHeight * MAX_TEXTAREA_ROWS + paddingTop + paddingBottom;
  const next = Math.max(minHeight, Math.min(el.scrollHeight, maxHeight));
  textareaHeight.value = `${next}px`;
}

onMounted(() => {
  void nextTick(autoResize);
});

watch(inputText, () => {
  void nextTick(autoResize);
});

function handleSubmit() {
  const text = inputText.value.trim();
  if (!text || props.disabled) return;
  // Push to history (avoid duplicating the last entry)
  if (messageHistory.value[messageHistory.value.length - 1] !== text) {
    messageHistory.value.push(text);
    if (messageHistory.value.length > MAX_HISTORY) messageHistory.value.shift();
  }
  historyIndex.value = -1;
  savedDraft.value = '';
  emit('submit', text);
  inputText.value = '';
}

/** Navigate through message history. dir=-1 goes older, dir=+1 goes newer. */
function navigateHistory(dir: number) {
  const len = messageHistory.value.length;
  if (len === 0) return;

  if (historyIndex.value === -1) {
    // Entering history — save current draft
    if (dir > 0) return; // already at newest
    savedDraft.value = inputText.value;
    historyIndex.value = len - 1;
  } else {
    const next = historyIndex.value + dir;
    if (next < 0) return; // already at oldest
    if (next >= len) {
      // Back to draft
      historyIndex.value = -1;
      inputText.value = savedDraft.value;
      void nextTick(autoResize);
      return;
    }
    historyIndex.value = next;
  }
  inputText.value = messageHistory.value[historyIndex.value];
  void nextTick(autoResize);
}

function handleFocus() {
  burstResetScroll();
  emit('focus');
}

function handleBlur() {
  emit('blur');
}

function openFilePicker() {
  fileInputRef.value?.click();
}

async function handleFileSelected() {
  const file = fileInputRef.value?.files?.[0];
  if (!file) return;
  try {
    const taskStore = useTaskStore();
    const name = file.name;
    await taskStore.ingestDocument(name);
  } catch {
    emit('submit', `[Attach: ${file.name}] — Please type the full file path to import.`);
  }
  if (fileInputRef.value) fileInputRef.value.value = '';
}

async function handleDrop(e: DragEvent) {
  dragOver.value = false;
  const files = e.dataTransfer?.files;
  if (!files?.length) return;
  const items = e.dataTransfer?.items;
  if (items) {
    for (let i = 0; i < items.length; i++) {
      const item = items[i];
      if (item.kind === 'file') {
        const file = item.getAsFile();
        if (file) {
          try {
            const taskStore = useTaskStore();
            const path = (file as unknown as { path?: string }).path ?? file.name;
            await taskStore.ingestDocument(path);
          } catch {
            emit('submit', `[Attach: ${file.name}]`);
          }
        }
      }
    }
  }
}</script>

<style scoped>
.chat-input-bar {
  display: flex;
  align-items: center;
  width: 100%;
}

.input-wrapper {
  display: flex;
  align-items: flex-end;
  flex: 1;
  background: var(--ts-glass-bg, rgba(15, 23, 42, 0.72));
  border: 1px solid var(--ts-glass-border, rgba(255, 255, 255, 0.08));
  border-radius: var(--ts-radius-lg, 16px);
  padding: 5px 6px 5px 6px;
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  box-shadow: var(--ts-shadow-sm),
              inset 0 1px 0 rgba(255, 255, 255, 0.04);
  transition: border-color var(--ts-transition-normal),
              box-shadow var(--ts-transition-normal),
              background var(--ts-transition-normal),
              transform var(--ts-transition-normal);
}

.input-wrapper:focus-within {
  border-color: var(--ts-accent);
  box-shadow: 0 0 0 3px var(--ts-accent-glow, rgba(124, 111, 255, 0.15)),
              0 4px 20px var(--ts-accent-glow, rgba(124, 111, 255, 0.12)),
              inset 0 1px 0 rgba(255, 255, 255, 0.06);
  background: var(--ts-glass-bg-focus, var(--ts-glass-bg, rgba(15, 23, 42, 0.88)));
  transform: translateY(-1px);
}

.input-wrapper.drag-over {
  border-color: var(--ts-accent);
  background: var(--ts-accent-glow, rgba(124, 111, 255, 0.08));
  box-shadow: 0 0 0 3px var(--ts-accent-glow, rgba(124, 111, 255, 0.2)),
              0 4px 20px var(--ts-accent-glow, rgba(124, 111, 255, 0.15));
  transform: scale(1.01);
}

.attach-btn {
  width: 30px;
  height: 34px;
  border-radius: 50%;
  border: none;
  background: transparent;
  color: var(--ts-text-dim, #888);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  margin-left: -2px;
  transition: color var(--ts-transition-normal),
              background var(--ts-transition-normal),
              transform 0.2s cubic-bezier(0.34, 1.56, 0.64, 1);
}

.attach-btn:hover:not(:disabled) {
  color: var(--ts-accent);
  background: var(--ts-accent-glow, rgba(124, 111, 255, 0.12));
  transform: scale(1.1);
}

.attach-btn:disabled {
  opacity: 0.35;
  cursor: not-allowed;
}

.hidden-file-input {
  display: none;
}

.chat-input {
  flex: 1;
  padding: 9px 6px;
  border: none;
  background: transparent;
  color: var(--ts-text-primary);
  font-size: var(--ts-text-base);
  font-family: inherit;
  line-height: 1.4;
  outline: none;
  min-width: 0;
  resize: none;
  overflow-y: auto;
  min-height: calc(1.4em * 1 + 18px);
  max-height: calc(1.4em * 3 + 18px);
  scrollbar-width: thin;
  scrollbar-color: var(--ts-text-dim) transparent;
}

.chat-input::placeholder {
  color: var(--ts-text-dim);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* Greyed, italic placeholder while the assistant is thinking — gives the
   user a visible cue without blocking input. The text content itself
   ("Thinking. / Thinking.. / Thinking…") is animated in JS. */
.chat-input.is-thinking-hint::placeholder {
  color: rgba(148, 163, 184, 0.75); /* slate-400 @ 75% — neutral grey */
  font-style: italic;
  font-size: clamp(0.66rem, 1.5vw, 0.84rem);
}

.chat-input:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

.send-btn {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  border: none;
  background: linear-gradient(135deg, var(--ts-accent), var(--ts-accent-violet));
  color: var(--ts-text-on-accent);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: background var(--ts-transition-normal),
              opacity var(--ts-transition-normal),
              transform 0.2s cubic-bezier(0.34, 1.56, 0.64, 1),
              box-shadow var(--ts-transition-normal);
  box-shadow: 0 2px 12px rgba(124, 111, 255, 0.3);
}

.send-btn:hover:not(:disabled) {
  background: linear-gradient(135deg, var(--ts-accent-hover), var(--ts-accent-violet-hover));
  transform: scale(1.1);
  box-shadow: 0 4px 20px rgba(124, 111, 255, 0.4);
}

.send-btn:active:not(:disabled) {
  transform: scale(0.95);
}

.send-btn:disabled {
  opacity: 0.35;
  cursor: not-allowed;
}

/* Mobile refinements */
@media (max-width: 640px) {
  .input-wrapper {
    padding: 3px 3px 3px 8px;
  }

  .chat-input {
    font-size: 0.88rem;
    padding: 7px 4px;
  }

  .send-btn {
    width: 34px;
    height: 34px;
  }

  .send-btn svg {
    width: 16px;
    height: 16px;
  }
}

/* Slash command suggestion dropdown */
.slash-suggestions {
  position: absolute;
  bottom: calc(100% + 6px);
  left: 0;
  right: 0;
  background: var(--ts-glass-bg, rgba(15, 23, 42, 0.92));
  border: 1px solid var(--ts-glass-border, rgba(255, 255, 255, 0.1));
  border-radius: var(--ts-radius-md, 8px);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
  box-shadow: var(--ts-shadow-lg, 0 8px 32px rgba(0, 0, 0, 0.4));
  max-height: 200px;
  overflow-y: auto;
  z-index: 100;
  padding: 4px;
}

.suggestion-item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 8px 10px;
  border: none;
  background: transparent;
  color: var(--ts-text-primary, #e2e8f0);
  font-size: var(--ts-text-sm, 0.85rem);
  cursor: pointer;
  border-radius: var(--ts-radius-sm, 4px);
  text-align: left;
  transition: background 0.15s;
}

.suggestion-item:hover,
.suggestion-item.active {
  background: rgba(124, 111, 255, 0.12);
}

.suggestion-name {
  font-weight: 600;
  color: var(--ts-accent, #7c6fff);
  white-space: nowrap;
}

.suggestion-desc {
  color: var(--ts-text-dim, #888);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* Transition */
.fade-up-enter-active,
.fade-up-leave-active {
  transition: opacity 0.15s, transform 0.15s;
}
.fade-up-enter-from,
.fade-up-leave-to {
  opacity: 0;
  transform: translateY(4px);
}

.chat-input-bar {
  position: relative;
}

/* ── History navigation arrows ── */
.history-nav {
  display: flex;
  flex-direction: column;
  gap: 1px;
  flex-shrink: 0;
  margin-right: 2px;
}

.history-btn {
  width: 22px;
  height: 16px;
  border: none;
  background: transparent;
  color: var(--ts-text-dim);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--ts-radius-sm, 4px);
  padding: 0;
  transition: color var(--ts-transition-fast),
              background var(--ts-transition-fast);
}

.history-btn:hover:not(:disabled) {
  color: var(--ts-text-primary);
  background: var(--ts-bg-hover, rgba(124, 111, 255, 0.1));
}

.history-btn:disabled {
  opacity: 0.25;
  cursor: default;
}
</style>
