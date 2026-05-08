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
        :disabled="disabled"
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
      <input
        ref="inputRef"
        v-model="inputText"
        type="text"
        class="chat-input"
        placeholder="Type a message…"
        :disabled="disabled"
        autocomplete="off"
        @focus="handleFocus"
        @blur="handleBlur"
        @keydown="handleKeydown"
      >
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
import { ref, computed, watch } from 'vue';
import { burstResetScroll } from '../utils/scroll-reset';
import { usePromptCommandsStore } from '../stores/prompt-commands';

const props = defineProps<{ disabled: boolean }>();
const emit = defineEmits<{ submit: [message: string]; focus: []; blur: [] }>();

const inputText = ref('');
const inputRef = ref<HTMLInputElement | null>(null);
const fileInputRef = ref<HTMLInputElement | null>(null);
const dragOver = ref(false);
const selectedSuggestionIdx = ref(0);

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

function handleSubmit() {
  const text = inputText.value.trim();
  if (!text || props.disabled) return;
  emit('submit', text);
  inputText.value = '';
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
    const { useTaskStore } = await import('../stores/tasks');
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
            const { useTaskStore } = await import('../stores/tasks');
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
  align-items: center;
  flex: 1;
  background: var(--ts-glass-bg, rgba(15, 23, 42, 0.72));
  border: 1px solid var(--ts-glass-border, rgba(255, 255, 255, 0.08));
  border-radius: var(--ts-radius-pill);
  padding: 5px 6px 5px 10px;
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
  box-shadow: 0 0 0 3px rgba(124, 111, 255, 0.15),
              0 4px 20px rgba(124, 111, 255, 0.12),
              inset 0 1px 0 rgba(255, 255, 255, 0.06);
  background: rgba(15, 23, 42, 0.88);
  transform: translateY(-1px);
}

.input-wrapper.drag-over {
  border-color: var(--ts-accent);
  background: rgba(124, 111, 255, 0.08);
  box-shadow: 0 0 0 3px rgba(124, 111, 255, 0.2),
              0 4px 20px rgba(124, 111, 255, 0.15);
  transform: scale(1.01);
}

.attach-btn {
  width: 34px;
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
  transition: color var(--ts-transition-normal),
              background var(--ts-transition-normal),
              transform 0.2s cubic-bezier(0.34, 1.56, 0.64, 1);
}

.attach-btn:hover:not(:disabled) {
  color: var(--ts-accent);
  background: rgba(124, 111, 255, 0.12);
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
  outline: none;
  min-width: 0;
}

.chat-input::placeholder {
  color: var(--ts-text-dim);
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
    padding: 3px 3px 3px 12px;
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
</style>
