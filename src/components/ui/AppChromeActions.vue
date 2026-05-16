<template>
  <Teleport to="body">
    <div
      class="app-chrome-actions"
      data-testid="app-chrome-actions"
      role="group"
      aria-label="App chrome actions"
    >
      <button
        type="button"
        class="ac-btn ac-btn-settings"
        data-testid="app-chrome-settings"
        aria-label="Open settings"
        title="Settings"
        @click="emit('open-settings')"
      >
        <svg
          width="18"
          height="18"
          viewBox="0 0 24 24"
          fill="currentColor"
          aria-hidden="true"
        >
          <path d="M19.14,12.94c0.04-0.3,0.06-0.61,0.06-0.94c0-0.32-0.02-0.64-0.07-0.94l2.03-1.58c0.18-0.14,0.23-0.41,0.12-0.61 l-1.92-3.32c-0.12-0.22-0.37-0.29-0.59-0.22l-2.39,0.96c-0.5-0.38-1.03-0.7-1.62-0.94L14.4,2.81c-0.04-0.24-0.24-0.41-0.48-0.41 h-3.84c-0.24,0-0.43,0.17-0.47,0.41L9.25,5.35C8.66,5.59,8.12,5.92,7.63,6.29L5.24,5.33c-0.22-0.08-0.47,0-0.59,0.22L2.74,8.87 C2.62,9.08,2.66,9.34,2.86,9.48l2.03,1.58C4.84,11.36,4.8,11.69,4.8,12s0.02,0.64,0.07,0.94l-2.03,1.58 c-0.18,0.14-0.23,0.41-0.12,0.61l1.92,3.32c0.12,0.22,0.37,0.29,0.59,0.22l2.39-0.96c0.5,0.38,1.03,0.7,1.62,0.94l0.36,2.54 c0.05,0.24,0.24,0.41,0.48,0.41h3.84c0.24,0,0.44-0.17,0.47-0.41l0.36-2.54c0.59-0.24,1.13-0.56,1.62-0.94l2.39,0.96 c0.22,0.08,0.47,0,0.59-0.22l1.92-3.32c0.12-0.22,0.07-0.47-0.12-0.61L19.14,12.94z M12,15.6c-1.98,0-3.6-1.62-3.6-3.6 s1.62-3.6,3.6-3.6s3.6,1.62,3.6,3.6S13.98,15.6,12,15.6z" />
        </svg>
      </button>

      <button
        type="button"
        class="ac-btn ac-btn-notifications"
        :class="{ 'has-active': activeCount > 0, 'has-unread': unreadCount > 0 }"
        data-testid="app-chrome-notifications"
        :aria-label="notificationsAriaLabel"
        title="Notifications"
        @click="handleNotificationsClick"
      >
        <span
          class="ac-bell"
          aria-hidden="true"
        >🔔</span>
        <span
          v-if="activeCount > 0"
          class="ac-ring"
          aria-hidden="true"
        />
        <span
          v-if="unreadCount > 0"
          class="ac-badge"
          data-testid="app-chrome-notifications-badge"
        >{{ unreadCount > 99 ? '99+' : unreadCount }}</span>
      </button>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useNotificationsStore } from '../../stores/notifications';

const props = withDefaults(
  defineProps<{
    /**
     * Disable the notifications button. Useful for tests / contexts where
     * the notifications store is not provided.
     */
    notificationsDisabled?: boolean;
  }>(),
  { notificationsDisabled: false },
);

const emit = defineEmits<{
  (e: 'open-settings'): void;
  (e: 'open-notifications'): void;
}>();

const notificationsStore = props.notificationsDisabled
  ? null
  : useNotificationsStore();

const unreadCount = computed(() => notificationsStore?.unreadCount ?? 0);
const activeCount = computed(() => notificationsStore?.activeJobs.length ?? 0);

const notificationsAriaLabel = computed(() => {
  const parts: string[] = [];
  if (activeCount.value > 0) parts.push(`${activeCount.value} active job(s)`);
  if (unreadCount.value > 0) parts.push(`${unreadCount.value} unread`);
  return parts.length === 0
    ? 'Open notifications'
    : `Open notifications — ${parts.join(', ')}`;
});

function handleNotificationsClick() {
  if (notificationsStore) {
    notificationsStore.togglePanel();
  }
  emit('open-notifications');
}
</script>

<style scoped>
/* ── Global chrome actions cluster ────────────────────────────────────────
   A fixed-position cluster pinned to the top-right of every view (chat,
   brain, memory, voice, settings, link, …). Bundles two buttons:

     [Settings ⚙] [Notifications 🔔]

   Both buttons match the historical NotificationBubble size (36×36 desktop,
   32×32 mobile) so existing pages keep the same visual rhythm. The cluster
   has `pointer-events: none` so the gap between the buttons does not block
   clicks on content underneath; each child button re-enables pointer events.

   Z-index 1500 — same as the legacy NotificationBubble — so the cluster
   always sits above panel content but below modal overlays (PanelShell
   uses 2000+). bp-shell padding-top is bumped to clear the cluster
   vertically; see `src/styles/brain-panel.css`. */
.app-chrome-actions {
  position: fixed;
  top: 18px;
  right: 16px;
  z-index: 1500;
  display: flex;
  align-items: center;
  gap: var(--ts-space-sm, 8px);
  pointer-events: none;
}

.ac-btn {
  pointer-events: auto;
  position: relative;
  width: 36px;
  height: 36px;
  border-radius: 50%;
  border: 1px solid var(--ts-border);
  background: var(--ts-bg-card);
  color: var(--ts-text-primary);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  font-size: 1rem;
  padding: 0;
  transition: transform 0.15s ease, border-color 0.15s ease,
    box-shadow 0.15s ease, color 0.15s ease;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.35);
}

.ac-btn:hover {
  transform: translateY(-1px);
  border-color: var(--ts-border-strong, var(--ts-accent));
  box-shadow: 0 6px 18px rgba(0, 0, 0, 0.45);
}

.ac-btn:focus-visible {
  outline: 2px solid var(--ts-accent);
  outline-offset: 2px;
}

.ac-btn-settings:hover {
  color: var(--ts-accent);
}

.ac-btn-notifications.has-active {
  border-color: var(--ts-accent);
}

.ac-btn-notifications.has-unread {
  border-color: var(--ts-accent-pink, var(--ts-accent));
}

.ac-bell {
  filter: drop-shadow(0 0 4px var(--ts-accent-glow, rgba(108, 99, 255, 0.5)));
}

.ac-ring {
  position: absolute;
  inset: -3px;
  border-radius: 50%;
  border: 2px solid var(--ts-accent);
  opacity: 0.5;
  animation: ac-pulse 1.6s ease-out infinite;
  pointer-events: none;
}

@keyframes ac-pulse {
  0%   { transform: scale(0.92); opacity: 0.55; }
  70%  { transform: scale(1.15); opacity: 0; }
  100% { transform: scale(1.15); opacity: 0; }
}

.ac-badge {
  position: absolute;
  top: -4px;
  right: -4px;
  min-width: 18px;
  height: 18px;
  padding: 0 5px;
  border-radius: 9px;
  background: var(--ts-accent-pink, var(--ts-accent));
  color: var(--ts-text-on-accent, #fff);
  font-size: 0.65rem;
  font-weight: 700;
  display: flex;
  align-items: center;
  justify-content: center;
  pointer-events: none;
}

@media (max-width: 640px) {
  .app-chrome-actions {
    top: 6px;
    right: 10px;
    gap: var(--ts-space-xs, 4px);
  }

  .ac-btn {
    width: 32px;
    height: 32px;
    font-size: 0.9rem;
  }
}
</style>
