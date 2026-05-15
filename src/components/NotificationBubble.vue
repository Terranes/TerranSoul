<template>
  <Teleport to="body">
    <button
      type="button"
      class="ts-notif-bubble"
      :class="{ 'has-active': active > 0, 'has-unread': unread > 0 }"
      :aria-label="ariaLabel"
      :data-testid="'notification-bubble'"
      @click="store.togglePanel()"
    >
      <span
        class="bell"
        aria-hidden="true"
      >🔔</span>
      <span
        v-if="active > 0"
        class="ring"
        aria-hidden="true"
      />
      <span
        v-if="unread > 0"
        class="badge"
        :data-testid="'notification-badge'"
      >{{ unread > 99 ? '99+' : unread }}</span>
    </button>
  </Teleport>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useNotificationsStore } from '../stores/notifications';

const store = useNotificationsStore();
const unread = computed(() => store.unreadCount);
const active = computed(() => store.activeJobs.length);
const ariaLabel = computed(() => {
  const parts = [];
  if (active.value > 0) parts.push(`${active.value} active job(s)`);
  if (unread.value > 0) parts.push(`${unread.value} unread`);
  return parts.length === 0
    ? 'Open notifications'
    : `Open notifications — ${parts.join(', ')}`;
});
</script>

<style scoped>
.ts-notif-bubble {
  position: fixed;
  /* Match the Settings gear in CharacterViewport so the two buttons sit
     side-by-side on the same row at the same size.
     Default (>640px): .ts-floating-chip is 36px tall, .corner-cluster at
       top:18px right:72px → bell at top:18px right:16px, 36×36.
     Mobile (≤640px): .settings-toggle becomes 32×32 at top:6px right:62px
       → bell tracks the same dimensions and offset. */
  top: 18px;
  right: 16px;
  z-index: 1500;
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
  transition: transform 0.15s ease, border-color 0.15s ease,
    box-shadow 0.15s ease;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.35);
}

@media (max-width: 640px) {
  .ts-notif-bubble {
    top: 6px;
    width: 32px;
    height: 32px;
    font-size: 0.9rem;
  }
}

.ts-notif-bubble:hover {
  transform: translateY(-1px);
  border-color: var(--ts-border-strong);
  box-shadow: 0 6px 18px rgba(0, 0, 0, 0.45);
}

.ts-notif-bubble.has-active {
  border-color: var(--ts-accent);
}

.ts-notif-bubble.has-unread {
  border-color: var(--ts-accent-pink);
}

.ts-notif-bubble .bell {
  filter: drop-shadow(0 0 4px var(--ts-accent-glow));
}

.ts-notif-bubble .ring {
  position: absolute;
  inset: -3px;
  border-radius: 50%;
  border: 2px solid var(--ts-accent);
  opacity: 0.5;
  animation: ts-pulse 1.6s ease-out infinite;
}

@keyframes ts-pulse {
  0%   { transform: scale(0.92); opacity: 0.55; }
  70%  { transform: scale(1.15); opacity: 0;    }
  100% { transform: scale(1.15); opacity: 0;    }
}

.ts-notif-bubble .badge {
  position: absolute;
  top: -4px;
  right: -4px;
  min-width: 18px;
  height: 18px;
  padding: 0 5px;
  border-radius: 9px;
  background: var(--ts-accent-pink);
  color: var(--ts-text-on-accent);
  font-size: 0.65rem;
  font-weight: 700;
  display: flex;
  align-items: center;
  justify-content: center;
  line-height: 1;
}
</style>
