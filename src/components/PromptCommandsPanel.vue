<template>
  <div
    class="prompt-commands-panel"
    data-testid="prompt-commands-panel"
  >
    <header class="pcp-header">
      <span class="pcp-title">⚡ Prompt Commands</span>
      <button
        class="pcp-add-btn"
        type="button"
        data-testid="pcp-add-btn"
        @click="startCreate"
      >
        + New
      </button>
    </header>
    <p class="pcp-desc">
      Extensible slash commands loaded from <code>.terransoul/prompts/</code>.
      Type <code>/command-name</code> in chat to use. Supports <code v-pre>{{input}}</code> and <code v-pre>{{date}}</code> template variables.
    </p>

    <!-- Command list -->
    <div
      v-if="store.commands.length > 0"
      class="pcp-list"
    >
      <div
        v-for="cmd in store.commands"
        :key="cmd.name"
        class="pcp-item"
        :class="{ editing: editingName === cmd.name }"
        :data-testid="`pcp-item-${cmd.name}`"
      >
        <div class="pcp-item-header">
          <span class="pcp-item-name">/{{ cmd.name }}</span>
          <span class="pcp-item-desc">{{ cmd.description }}</span>
          <div class="pcp-item-actions">
            <button
              type="button"
              class="pcp-icon-btn"
              title="Edit"
              :data-testid="`pcp-edit-${cmd.name}`"
              @click="startEdit(cmd)"
            >
              ✏️
            </button>
            <button
              type="button"
              class="pcp-icon-btn pcp-danger"
              title="Delete"
              :data-testid="`pcp-delete-${cmd.name}`"
              @click="confirmDelete(cmd.name)"
            >
              🗑️
            </button>
          </div>
        </div>
        <!-- Inline editor for this command -->
        <div
          v-if="editingName === cmd.name"
          class="pcp-editor"
        >
          <textarea
            v-model="editContent"
            class="pcp-textarea"
            rows="8"
            placeholder="Prompt content (markdown)…"
            :data-testid="`pcp-textarea-${cmd.name}`"
          />
          <div class="pcp-editor-actions">
            <button
              type="button"
              class="pcp-save-btn"
              :disabled="saving"
              :data-testid="`pcp-save-${cmd.name}`"
              @click="save"
            >
              {{ saving ? 'Saving…' : 'Save' }}
            </button>
            <button
              type="button"
              class="pcp-cancel-btn"
              @click="cancelEdit"
            >
              Cancel
            </button>
          </div>
        </div>
      </div>
    </div>
    <p
      v-else
      class="pcp-empty"
    >
      No prompt commands loaded. Click <strong>+ New</strong> to create one, or add
      <code>.md</code> files to <code>.terransoul/prompts/</code>.
    </p>

    <!-- Create new command form -->
    <div
      v-if="creating"
      class="pcp-create-form"
      data-testid="pcp-create-form"
    >
      <label class="pcp-label">
        Command name
        <input
          v-model="newName"
          class="pcp-input"
          type="text"
          placeholder="my-command"
          pattern="[a-zA-Z0-9_-]+"
          data-testid="pcp-new-name"
          @keydown.enter="save"
        >
      </label>
      <label class="pcp-label">
        Prompt content
        <textarea
          v-model="editContent"
          class="pcp-textarea"
          rows="8"
          placeholder="# My Command&#10;&#10;Your prompt here. Use {{input}} for user arguments."
          data-testid="pcp-new-content"
        />
      </label>
      <div class="pcp-editor-actions">
        <button
          type="button"
          class="pcp-save-btn"
          :disabled="saving || !newName.trim()"
          data-testid="pcp-create-save"
          @click="save"
        >
          {{ saving ? 'Creating…' : 'Create' }}
        </button>
        <button
          type="button"
          class="pcp-cancel-btn"
          @click="cancelEdit"
        >
          Cancel
        </button>
      </div>
      <p
        v-if="error"
        class="pcp-error"
      >
        {{ error }}
      </p>
    </div>

    <!-- Delete confirmation -->
    <div
      v-if="deletingName"
      class="pcp-delete-confirm"
      data-testid="pcp-delete-confirm"
    >
      <p>Delete <strong>/{{ deletingName }}</strong>?</p>
      <div class="pcp-editor-actions">
        <button
          type="button"
          class="pcp-danger-btn"
          data-testid="pcp-confirm-delete"
          @click="doDelete"
        >
          Delete
        </button>
        <button
          type="button"
          class="pcp-cancel-btn"
          @click="deletingName = ''"
        >
          Cancel
        </button>
      </div>
    </div>

    <!-- Error display -->
    <p
      v-if="error && !creating"
      class="pcp-error"
    >
      {{ error }}
    </p>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { usePromptCommandsStore, type PromptCommand } from '../stores/prompt-commands';

const store = usePromptCommandsStore();

const editingName = ref('');
const editContent = ref('');
const creating = ref(false);
const newName = ref('');
const saving = ref(false);
const error = ref('');
const deletingName = ref('');

onMounted(async () => {
  if (!store.loaded) {
    await store.loadCommands();
  }
});

function startCreate() {
  editingName.value = '';
  creating.value = true;
  newName.value = '';
  editContent.value = '';
  error.value = '';
}

function startEdit(cmd: PromptCommand) {
  creating.value = false;
  editingName.value = cmd.name;
  editContent.value = cmd.content;
  error.value = '';
}

function cancelEdit() {
  editingName.value = '';
  creating.value = false;
  newName.value = '';
  editContent.value = '';
  error.value = '';
}

async function save() {
  const name = creating.value ? newName.value.trim() : editingName.value;
  if (!name) return;

  saving.value = true;
  error.value = '';
  try {
    await store.saveCommand(name, editContent.value);
    cancelEdit();
  } catch (e) {
    error.value = String(e);
  } finally {
    saving.value = false;
  }
}

function confirmDelete(name: string) {
  deletingName.value = name;
}

async function doDelete() {
  const name = deletingName.value;
  if (!name) return;
  error.value = '';
  try {
    await store.deleteCommand(name);
    deletingName.value = '';
    if (editingName.value === name) cancelEdit();
  } catch (e) {
    error.value = String(e);
  }
}
</script>

<style scoped>
.prompt-commands-panel {
  display: flex;
  flex-direction: column;
  gap: var(--ts-space-sm, 8px);
}

.pcp-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--ts-space-md, 12px);
}

.pcp-title {
  font-size: var(--ts-text-lg, 1.1rem);
  font-weight: 600;
  color: var(--ts-text-primary, #e2e8f0);
}

.pcp-desc {
  font-size: var(--ts-text-sm, 0.85rem);
  color: var(--ts-text-dim, #888);
  margin: 0;
  line-height: 1.5;
}

.pcp-desc code {
  background: rgba(124, 111, 255, 0.1);
  padding: 1px 4px;
  border-radius: 3px;
  font-size: 0.9em;
}

.pcp-add-btn {
  padding: 6px 14px;
  border: 1px solid var(--ts-accent, #7c6fff);
  border-radius: var(--ts-radius-sm, 6px);
  background: transparent;
  color: var(--ts-accent, #7c6fff);
  font-size: var(--ts-text-sm, 0.85rem);
  font-weight: 600;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}

.pcp-add-btn:hover {
  background: var(--ts-accent, #7c6fff);
  color: var(--ts-text-on-accent, #fff);
}

.pcp-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.pcp-item {
  border: 1px solid var(--ts-glass-border, rgba(255, 255, 255, 0.08));
  border-radius: var(--ts-radius-sm, 6px);
  padding: 10px 12px;
  background: var(--ts-glass-bg, rgba(15, 23, 42, 0.5));
  transition: border-color 0.15s;
}

.pcp-item.editing {
  border-color: var(--ts-accent, #7c6fff);
}

.pcp-item-header {
  display: flex;
  align-items: center;
  gap: 10px;
}

.pcp-item-name {
  font-weight: 600;
  color: var(--ts-accent, #7c6fff);
  white-space: nowrap;
  font-family: monospace;
}

.pcp-item-desc {
  flex: 1;
  color: var(--ts-text-dim, #888);
  font-size: var(--ts-text-sm, 0.85rem);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.pcp-item-actions {
  display: flex;
  gap: 4px;
  flex-shrink: 0;
}

.pcp-icon-btn {
  width: 28px;
  height: 28px;
  border: none;
  background: transparent;
  border-radius: var(--ts-radius-sm, 4px);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
  transition: background 0.15s;
}

.pcp-icon-btn:hover {
  background: rgba(255, 255, 255, 0.08);
}

.pcp-icon-btn.pcp-danger:hover {
  background: rgba(239, 68, 68, 0.15);
}

.pcp-editor {
  margin-top: 8px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.pcp-textarea {
  width: 100%;
  min-height: 120px;
  padding: 10px 12px;
  border: 1px solid var(--ts-glass-border, rgba(255, 255, 255, 0.12));
  border-radius: var(--ts-radius-sm, 6px);
  background: var(--ts-bg-input, rgba(15, 23, 42, 0.7));
  color: var(--ts-text-primary, #e2e8f0);
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: var(--ts-text-sm, 0.85rem);
  line-height: 1.5;
  resize: vertical;
}

.pcp-textarea:focus {
  outline: none;
  border-color: var(--ts-accent, #7c6fff);
}

.pcp-editor-actions {
  display: flex;
  gap: 8px;
}

.pcp-save-btn {
  padding: 6px 16px;
  border: none;
  border-radius: var(--ts-radius-sm, 6px);
  background: var(--ts-accent, #7c6fff);
  color: var(--ts-text-on-accent, #fff);
  font-weight: 600;
  font-size: var(--ts-text-sm, 0.85rem);
  cursor: pointer;
  transition: opacity 0.15s;
}

.pcp-save-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.pcp-cancel-btn {
  padding: 6px 16px;
  border: 1px solid var(--ts-glass-border, rgba(255, 255, 255, 0.15));
  border-radius: var(--ts-radius-sm, 6px);
  background: transparent;
  color: var(--ts-text-dim, #888);
  font-size: var(--ts-text-sm, 0.85rem);
  cursor: pointer;
  transition: color 0.15s;
}

.pcp-cancel-btn:hover {
  color: var(--ts-text-primary, #e2e8f0);
}

.pcp-danger-btn {
  padding: 6px 16px;
  border: none;
  border-radius: var(--ts-radius-sm, 6px);
  background: var(--ts-danger, #ef4444);
  color: #fff;
  font-weight: 600;
  font-size: var(--ts-text-sm, 0.85rem);
  cursor: pointer;
}

.pcp-create-form {
  margin-top: 8px;
  padding: 12px;
  border: 1px solid var(--ts-accent, #7c6fff);
  border-radius: var(--ts-radius-sm, 6px);
  background: rgba(124, 111, 255, 0.04);
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.pcp-label {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: var(--ts-text-sm, 0.85rem);
  color: var(--ts-text-dim, #888);
}

.pcp-input {
  padding: 8px 12px;
  border: 1px solid var(--ts-glass-border, rgba(255, 255, 255, 0.12));
  border-radius: var(--ts-radius-sm, 6px);
  background: var(--ts-bg-input, rgba(15, 23, 42, 0.7));
  color: var(--ts-text-primary, #e2e8f0);
  font-family: monospace;
  font-size: var(--ts-text-base, 0.95rem);
}

.pcp-input:focus {
  outline: none;
  border-color: var(--ts-accent, #7c6fff);
}

.pcp-empty {
  color: var(--ts-text-dim, #888);
  font-size: var(--ts-text-sm, 0.85rem);
  padding: 12px 0;
}

.pcp-error {
  color: var(--ts-danger, #ef4444);
  font-size: var(--ts-text-sm, 0.85rem);
  margin: 0;
}

.pcp-delete-confirm {
  padding: 10px 12px;
  border: 1px solid var(--ts-danger, #ef4444);
  border-radius: var(--ts-radius-sm, 6px);
  background: rgba(239, 68, 68, 0.05);
}

.pcp-delete-confirm p {
  margin: 0 0 8px;
  font-size: var(--ts-text-sm, 0.85rem);
}
</style>
