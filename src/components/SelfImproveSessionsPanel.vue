<template>
  <section
    class="si-sessions"
    aria-labelledby="si-sessions-title"
  >
    <header class="si-sessions-header">
      <h3
        id="si-sessions-title"
        class="si-sessions-title"
      >
        🗂️ Coding sessions
      </h3>
      <button
        type="button"
        class="si-sessions-refresh"
        :disabled="store.sessionsLoading"
        @click="refresh"
      >
        {{ store.sessionsLoading ? 'Loading…' : 'Refresh' }}
      </button>
    </header>

    <p class="si-sessions-desc">
      Resumable named transcripts modelled on Claude Code <code>--resume</code> /
      <code>--fork-session</code> and claw-code's per-project sessions sidebar.
      Every user/assistant turn is appended to an append-only JSONL alongside
      the existing handoff snapshot.
    </p>

    <div class="si-sessions-body">
      <!-- Session sidebar -->
      <ul class="si-session-list">
        <li
          v-if="store.sessions.length === 0"
          class="si-session-empty"
        >
          No saved sessions yet — start the autonomous loop or send a message
          below to create one.
        </li>
        <li
          v-for="entry in store.sessions"
          :key="entry.session_id"
          class="si-session-item"
          :class="{
            'si-session-item--active': entry.session_id === store.activeSessionId,
          }"
        >
          <button
            type="button"
            class="si-session-pick"
            @click="selectSession(entry.session_id)"
          >
            <span class="si-session-id">{{ entry.session_id }}</span>
            <span class="si-session-meta">
              <span>chunk {{ entry.chunk_id || '—' }}</span>
              <span>{{ entry.chat.message_count }} msg</span>
            </span>
            <span
              v-if="entry.chat.last_user_preview"
              class="si-session-preview"
              :title="entry.chat.last_user_preview"
            >
              {{ entry.chat.last_user_preview }}
            </span>
            <span
              v-else-if="entry.last_action"
              class="si-session-preview si-session-preview--meta"
              :title="entry.last_action"
            >
              {{ entry.last_action }}
            </span>
          </button>
          <div class="si-session-actions">
            <button
              type="button"
              class="si-session-action"
              title="Rename"
              @click="onRename(entry.session_id)"
            >
              ✎
            </button>
            <button
              type="button"
              class="si-session-action"
              title="Fork into a new session"
              @click="onFork(entry.session_id)"
            >
              ⑂
            </button>
            <button
              type="button"
              class="si-session-action si-session-action--danger"
              title="Delete session and transcript"
              @click="onPurge(entry.session_id)"
            >
              ✕
            </button>
          </div>
        </li>
      </ul>

      <!-- Chat scrollback + slash-command input -->
      <div class="si-session-chat">
        <header class="si-chat-header">
          <span class="si-chat-active">
            {{ store.activeSessionId ?? 'No active session' }}
          </span>
          <button
            v-if="store.activeSessionId"
            type="button"
            class="si-chat-clear"
            @click="onClearChat"
          >
            Clear transcript
          </button>
        </header>

        <div
          ref="scrollback"
          class="si-chat-scrollback"
        >
          <p
            v-if="!store.activeSessionId"
            class="si-chat-empty"
          >
            Pick a session from the list, or start typing below to create one.
          </p>
          <p
            v-else-if="store.activeChat.length === 0"
            class="si-chat-empty"
          >
            No messages yet in <code>{{ store.activeSessionId }}</code>.
          </p>
          <article
            v-for="(msg, idx) in store.activeChat"
            :key="idx"
            class="si-chat-msg"
            :class="`si-chat-msg--${msg.role}`"
          >
            <header class="si-chat-msg-head">
              <span class="si-chat-role">{{ msg.role }}</span>
              <span class="si-chat-time">{{ formatTime(msg.ts_ms) }}</span>
            </header>
            <p class="si-chat-body">
              {{ msg.content }}
            </p>
          </article>
        </div>

        <form
          class="si-chat-form"
          @submit.prevent="onSubmit"
        >
          <input
            v-model="draft"
            class="si-chat-input"
            type="text"
            placeholder="Message or /help for slash commands"
            aria-label="Self-improve session message"
            autocomplete="off"
          >
          <button
            type="submit"
            class="si-chat-send"
            :disabled="!draft.trim() || sending"
          >
            {{ sending ? '…' : 'Send' }}
          </button>
        </form>
        <p
          v-if="status"
          class="si-chat-status"
          :class="`si-chat-status--${status.kind}`"
        >
          {{ status.message }}
        </p>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useSelfImproveStore } from '../stores/self-improve';
import { parseSlashCommand, SLASH_HELP_TEXT } from '../utils/slash-commands';

const store = useSelfImproveStore();
const draft = ref('');
const sending = ref(false);
const scrollback = ref<HTMLDivElement | null>(null);
const status = ref<{ kind: 'info' | 'error' | 'success'; message: string } | null>(null);

function formatTime(ts: number): string {
  if (!ts) return '';
  try {
    return new Date(ts).toLocaleTimeString();
  } catch {
    return '';
  }
}

function setStatus(kind: 'info' | 'error' | 'success', message: string): void {
  status.value = { kind, message };
}

async function refresh(): Promise<void> {
  await store.loadSessions();
}

async function selectSession(id: string): Promise<void> {
  try {
    await store.loadSessionChat(id);
    setStatus('info', `Loaded ${store.activeChat.length} message(s) for ${id}.`);
  } catch (e) {
    setStatus('error', String(e));
  }
}

function promptForId(prompt: string, fallback?: string): string | null {
  const value = window.prompt(prompt, fallback ?? '');
  if (!value) return null;
  return value.trim();
}

async function onRename(id: string): Promise<void> {
  const next = promptForId(`Rename session "${id}" to:`, id);
  if (!next || next === id) return;
  try {
    await store.renameSession(id, next);
    setStatus('success', `Renamed ${id} → ${next}.`);
  } catch (e) {
    setStatus('error', String(e));
  }
}

async function onFork(id: string): Promise<void> {
  const next = promptForId(`Fork "${id}" into new session id:`, `${id}-fork`);
  if (!next || next === id) return;
  try {
    const copied = await store.forkSession(id, next);
    setStatus('success', `Forked ${id} → ${next} (${copied} message(s)).`);
  } catch (e) {
    setStatus('error', String(e));
  }
}

async function onPurge(id: string): Promise<void> {
  if (!window.confirm(`Delete session "${id}" and its transcript? This cannot be undone.`)) {
    return;
  }
  try {
    await store.purgeSession(id);
    setStatus('success', `Deleted ${id}.`);
  } catch (e) {
    setStatus('error', String(e));
  }
}

async function onClearChat(): Promise<void> {
  const id = store.activeSessionId;
  if (!id) return;
  if (!window.confirm(`Clear the transcript for "${id}"?`)) return;
  try {
    await store.clearSessionChat(id);
    setStatus('success', 'Transcript cleared.');
  } catch (e) {
    setStatus('error', String(e));
  }
}

async function ensureActiveSession(): Promise<string | null> {
  if (store.activeSessionId) return store.activeSessionId;
  const next = promptForId('Name a new session to start:', `session-${Date.now().toString(36)}`);
  if (!next) return null;
  store.activeSessionId = next;
  store.activeChat = [];
  return next;
}

async function handleSlashCommand(parsed: ReturnType<typeof parseSlashCommand>): Promise<void> {
  switch (parsed.kind) {
    case 'help': {
      setStatus('info', SLASH_HELP_TEXT);
      return;
    }
    case 'list': {
      await refresh();
      setStatus('info', `${store.sessions.length} session(s) on disk.`);
      return;
    }
    case 'clear': {
      await onClearChat();
      return;
    }
    case 'rename': {
      const id = store.activeSessionId;
      if (!id) {
        setStatus('error', 'No active session to rename.');
        return;
      }
      if (!parsed.arg) {
        setStatus('error', 'Usage: /rename <new-name>');
        return;
      }
      try {
        await store.renameSession(id, parsed.arg);
        setStatus('success', `Renamed ${id} → ${parsed.arg}.`);
      } catch (e) {
        setStatus('error', String(e));
      }
      return;
    }
    case 'fork': {
      const id = store.activeSessionId;
      if (!id) {
        setStatus('error', 'No active session to fork.');
        return;
      }
      const target = parsed.arg || `${id}-fork-${Date.now().toString(36)}`;
      try {
        const copied = await store.forkSession(id, target);
        setStatus('success', `Forked ${id} → ${target} (${copied} message(s)).`);
      } catch (e) {
        setStatus('error', String(e));
      }
      return;
    }
    case 'resume': {
      if (!parsed.arg) {
        setStatus('error', 'Usage: /resume <session-id>');
        return;
      }
      try {
        await store.loadSessionChat(parsed.arg);
        setStatus('success', `Resumed ${parsed.arg}.`);
      } catch (e) {
        setStatus('error', String(e));
      }
      return;
    }
    case 'unknown': {
      setStatus('error', `Unknown command: /${parsed.command ?? ''}. Type /help for the list.`);
      return;
    }
    default:
      return;
  }
}

async function onSubmit(): Promise<void> {
  const raw = draft.value.trim();
  if (!raw) return;
  const parsed = parseSlashCommand(raw);
  draft.value = '';
  status.value = null;
  sending.value = true;
  try {
    if (parsed.kind === 'chat') {
      const id = await ensureActiveSession();
      if (!id) {
        setStatus('error', 'No session selected.');
        return;
      }
      await store.appendSessionMessage(id, 'user', parsed.arg, 'chat');
      setStatus('info', 'Message appended to transcript. Connect the autonomous loop to drive a reply.');
      await store.loadSessions();
    } else {
      await handleSlashCommand(parsed);
    }
  } finally {
    sending.value = false;
    // Best-effort scroll-to-bottom after the next paint.
    requestAnimationFrame(() => {
      if (scrollback.value) {
        scrollback.value.scrollTop = scrollback.value.scrollHeight;
      }
    });
  }
}

onMounted(() => {
  void store.loadSessions();
});
</script>

<style scoped>
.si-sessions {
  margin-top: var(--ts-space-lg, 1.5rem);
  padding: var(--ts-space-md, 1rem);
  border: 1px solid var(--ts-border-soft, rgba(255, 255, 255, 0.08));
  border-radius: var(--ts-radius-md, 12px);
  background: var(--ts-surface-1, rgba(255, 255, 255, 0.03));
  display: flex;
  flex-direction: column;
  gap: var(--ts-space-md, 1rem);
}

.si-sessions-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--ts-space-md, 1rem);
}

.si-sessions-title {
  margin: 0;
  font-size: var(--ts-font-md, 1rem);
}

.si-sessions-refresh {
  padding: 0.25rem 0.75rem;
  border-radius: var(--ts-radius-sm, 6px);
  border: 1px solid var(--ts-border-soft, rgba(255, 255, 255, 0.12));
  background: transparent;
  color: var(--ts-text-primary, inherit);
  cursor: pointer;
}

.si-sessions-refresh:disabled {
  opacity: 0.5;
  cursor: progress;
}

.si-sessions-desc {
  margin: 0;
  color: var(--ts-text-muted, rgba(255, 255, 255, 0.6));
  font-size: var(--ts-font-sm, 0.875rem);
  line-height: 1.4;
}

.si-sessions-body {
  display: grid;
  grid-template-columns: minmax(220px, 1fr) 2fr;
  gap: var(--ts-space-md, 1rem);
  min-height: 320px;
}

@media (max-width: 640px) {
  .si-sessions-body {
    grid-template-columns: 1fr;
  }
}

.si-session-list {
  list-style: none;
  margin: 0;
  padding: 0;
  border: 1px solid var(--ts-border-soft, rgba(255, 255, 255, 0.08));
  border-radius: var(--ts-radius-sm, 8px);
  overflow: hidden auto;
  max-height: 360px;
}

.si-session-empty {
  padding: var(--ts-space-md, 1rem);
  color: var(--ts-text-muted, rgba(255, 255, 255, 0.6));
  font-size: var(--ts-font-sm, 0.875rem);
}

.si-session-item {
  display: flex;
  align-items: stretch;
  border-bottom: 1px solid var(--ts-border-soft, rgba(255, 255, 255, 0.06));
}

.si-session-item:last-child {
  border-bottom: none;
}

.si-session-item--active {
  background: var(--ts-accent-soft, rgba(103, 168, 255, 0.12));
}

.si-session-pick {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.25rem;
  padding: 0.5rem 0.75rem;
  border: none;
  background: transparent;
  color: inherit;
  text-align: left;
  cursor: pointer;
}

.si-session-id {
  font-family: var(--ts-font-mono, ui-monospace, monospace);
  font-size: var(--ts-font-sm, 0.875rem);
}

.si-session-meta {
  display: flex;
  gap: 0.75rem;
  font-size: var(--ts-font-xs, 0.75rem);
  color: var(--ts-text-muted, rgba(255, 255, 255, 0.55));
}

.si-session-preview {
  font-size: var(--ts-font-xs, 0.75rem);
  color: var(--ts-text-muted, rgba(255, 255, 255, 0.7));
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 100%;
}

.si-session-preview--meta {
  font-style: italic;
}

.si-session-actions {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 0.25rem;
}

.si-session-action {
  border: none;
  background: transparent;
  color: var(--ts-text-muted, rgba(255, 255, 255, 0.6));
  cursor: pointer;
  padding: 0.15rem 0.4rem;
  border-radius: var(--ts-radius-sm, 4px);
}

.si-session-action:hover {
  background: var(--ts-surface-2, rgba(255, 255, 255, 0.08));
  color: var(--ts-text-primary, inherit);
}

.si-session-action--danger:hover {
  color: var(--ts-danger, #ff7676);
}

.si-session-chat {
  display: flex;
  flex-direction: column;
  gap: var(--ts-space-sm, 0.5rem);
  border: 1px solid var(--ts-border-soft, rgba(255, 255, 255, 0.08));
  border-radius: var(--ts-radius-sm, 8px);
  padding: var(--ts-space-sm, 0.5rem);
  min-height: 320px;
}

.si-chat-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.si-chat-active {
  font-family: var(--ts-font-mono, ui-monospace, monospace);
  font-size: var(--ts-font-sm, 0.875rem);
  color: var(--ts-text-muted, rgba(255, 255, 255, 0.7));
}

.si-chat-clear {
  border: none;
  background: transparent;
  color: var(--ts-text-muted, rgba(255, 255, 255, 0.6));
  cursor: pointer;
  font-size: var(--ts-font-xs, 0.75rem);
}

.si-chat-clear:hover {
  color: var(--ts-danger, #ff7676);
}

.si-chat-scrollback {
  flex: 1;
  overflow-y: auto;
  border: 1px solid var(--ts-border-soft, rgba(255, 255, 255, 0.06));
  border-radius: var(--ts-radius-sm, 6px);
  padding: var(--ts-space-sm, 0.5rem);
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  min-height: 200px;
}

.si-chat-empty {
  color: var(--ts-text-muted, rgba(255, 255, 255, 0.5));
  font-size: var(--ts-font-sm, 0.875rem);
  margin: 0;
}

.si-chat-msg {
  border-left: 3px solid var(--ts-border-soft, rgba(255, 255, 255, 0.15));
  padding: 0.25rem 0.5rem;
  background: var(--ts-surface-2, rgba(255, 255, 255, 0.04));
  border-radius: 0 var(--ts-radius-sm, 6px) var(--ts-radius-sm, 6px) 0;
}

.si-chat-msg--user {
  border-left-color: var(--ts-accent, #67a8ff);
}

.si-chat-msg--assistant {
  border-left-color: var(--ts-success, #76d495);
}

.si-chat-msg--system {
  border-left-color: var(--ts-warning, #f6b96e);
}

.si-chat-msg-head {
  display: flex;
  justify-content: space-between;
  font-size: var(--ts-font-xs, 0.75rem);
  color: var(--ts-text-muted, rgba(255, 255, 255, 0.55));
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.si-chat-body {
  margin: 0.25rem 0 0;
  font-size: var(--ts-font-sm, 0.9rem);
  white-space: pre-wrap;
  word-break: break-word;
}

.si-chat-form {
  display: flex;
  gap: 0.5rem;
}

.si-chat-input {
  flex: 1;
  padding: 0.5rem 0.75rem;
  border-radius: var(--ts-radius-sm, 6px);
  border: 1px solid var(--ts-border-soft, rgba(255, 255, 255, 0.12));
  background: var(--ts-surface-1, rgba(255, 255, 255, 0.04));
  color: inherit;
  font-size: var(--ts-font-sm, 0.9rem);
}

.si-chat-send {
  padding: 0.5rem 1rem;
  border: 1px solid var(--ts-accent, #67a8ff);
  background: var(--ts-accent-soft, rgba(103, 168, 255, 0.18));
  color: inherit;
  border-radius: var(--ts-radius-sm, 6px);
  cursor: pointer;
}

.si-chat-send:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.si-chat-status {
  margin: 0;
  font-size: var(--ts-font-xs, 0.75rem);
  white-space: pre-wrap;
}

.si-chat-status--info {
  color: var(--ts-text-muted, rgba(255, 255, 255, 0.7));
}

.si-chat-status--success {
  color: var(--ts-success, #76d495);
}

.si-chat-status--error {
  color: var(--ts-danger, #ff7676);
}
</style>
