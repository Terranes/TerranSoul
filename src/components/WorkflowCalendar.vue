<template>
  <div class="wfc-root">
    <header class="wfc-header">
      <div class="wfc-nav">
        <button class="wfc-nav-btn" @click="store.shiftCalendarWeek(-1)" aria-label="Previous week">‹</button>
        <button class="wfc-today-btn" @click="store.jumpCalendarToToday()">Today</button>
        <button class="wfc-nav-btn" @click="store.shiftCalendarWeek(1)" aria-label="Next week">›</button>
      </div>
      <h3 class="wfc-range">{{ rangeLabel }}</h3>
      <div class="wfc-nav-spacer" />
    </header>

    <div class="wfc-grid">
      <!-- Day headers -->
      <div class="wfc-day-headers">
        <div class="wfc-time-col-header"></div>
        <div
          v-for="(day, i) in weekDays"
          :key="i"
          :class="['wfc-day-header', { 'wfc-day-today': isToday(day) }]"
        >
          <div class="wfc-day-name">{{ dayName(day) }}</div>
          <div class="wfc-day-num">{{ day.getDate() }}</div>
        </div>
      </div>

      <!-- Time grid -->
      <div class="wfc-time-grid">
        <div class="wfc-time-col">
          <div
            v-for="hr in hours"
            :key="`hr-${hr}`"
            class="wfc-time-label"
          >
            {{ formatHour(hr) }}
          </div>
        </div>

        <div
          v-for="(day, dayIdx) in weekDays"
          :key="`col-${dayIdx}`"
          class="wfc-day-col"
        >
          <div
            v-for="hr in hours"
            :key="`cell-${dayIdx}-${hr}`"
            class="wfc-hour-cell"
          />
          <!-- Events overlaid on this column -->
          <div
            v-for="ev in eventsForDay(day)"
            :key="ev.workflow_id + ev.start_at"
            class="wfc-event"
            :class="`wfc-event-${ev.kind}`"
            :style="eventStyle(ev)"
            :title="`${ev.title} — ${formatTime(ev.start_at)}`"
            @click="onEventClick(ev.workflow_id)"
          >
            <span v-if="ev.recurring" class="wfc-event-recurring" aria-label="Recurring">↻</span>
            <span class="wfc-event-title">{{ ev.title }}</span>
            <span class="wfc-event-time">{{ formatTime(ev.start_at) }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted } from 'vue';
import {
  type CalendarEvent,
  isoDayKey,
  useWorkflowPlansStore,
} from '../stores/workflow-plans';

const store = useWorkflowPlansStore();

const HOUR_PX = 48;
const hours = Array.from({ length: 24 }, (_, i) => i);

const weekDays = computed<Date[]>(() => {
  const start = new Date(store.calendarRangeStart);
  return Array.from({ length: 7 }, (_, i) => {
    const d = new Date(start);
    d.setDate(start.getDate() + i);
    return d;
  });
});

const rangeLabel = computed(() => {
  const start = weekDays.value[0];
  const end = weekDays.value[6];
  const fmt = (d: Date) =>
    d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
  return `${fmt(start)} – ${fmt(end)}, ${end.getFullYear()}`;
});

onMounted(async () => {
  await store.loadCalendarEvents(store.calendarRangeStart, store.calendarRangeEnd);
});

function dayName(d: Date): string {
  return d.toLocaleDateString(undefined, { weekday: 'short' });
}

function isToday(d: Date): boolean {
  const today = new Date();
  return (
    d.getFullYear() === today.getFullYear() &&
    d.getMonth() === today.getMonth() &&
    d.getDate() === today.getDate()
  );
}

function formatHour(hr: number): string {
  if (hr === 0) return '12 AM';
  if (hr === 12) return '12 PM';
  return hr < 12 ? `${hr} AM` : `${hr - 12} PM`;
}

function formatTime(epochMs: number): string {
  return new Date(epochMs).toLocaleTimeString(undefined, {
    hour: 'numeric',
    minute: '2-digit',
  });
}

function eventsForDay(day: Date): CalendarEvent[] {
  const key = isoDayKey(day.getTime());
  return store.eventsByDay[key] ?? [];
}

function eventStyle(ev: CalendarEvent): Record<string, string> {
  const start = new Date(ev.start_at);
  const startMins = start.getHours() * 60 + start.getMinutes();
  const top = (startMins / 60) * HOUR_PX;
  const durationMs = Math.max(15 * 60 * 1000, ev.end_at - ev.start_at);
  const height = (durationMs / 60_000 / 60) * HOUR_PX;
  return {
    top: `${top}px`,
    height: `${height}px`,
  };
}

function onEventClick(planId: string): void {
  void store.loadPlan(planId);
}
</script>

<style scoped>
.wfc-root {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  height: 600px;
}
.wfc-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.5rem;
}
.wfc-nav { display: flex; gap: 0.25rem; align-items: center; }
.wfc-nav-spacer { width: 100px; }
.wfc-nav-btn,
.wfc-today-btn {
  padding: 0.375rem 0.75rem;
  background: transparent;
  border: 1px solid var(--ts-border, #333);
  border-radius: 0.375rem;
  color: var(--ts-text);
  cursor: pointer;
  font-size: 0.875rem;
}
.wfc-nav-btn:hover, .wfc-today-btn:hover { background: var(--ts-surface-hover, #222); }
.wfc-range { margin: 0; font-size: 1rem; font-weight: 600; }

.wfc-grid {
  flex: 1;
  display: flex;
  flex-direction: column;
  border: 1px solid var(--ts-border, #333);
  border-radius: 0.5rem;
  overflow: hidden;
  background: var(--ts-surface, #1a1a1a);
}
.wfc-day-headers {
  display: grid;
  grid-template-columns: 60px repeat(7, 1fr);
  border-bottom: 1px solid var(--ts-border, #333);
  background: var(--ts-surface-2, #222);
}
.wfc-time-col-header { border-right: 1px solid var(--ts-border, #333); }
.wfc-day-header {
  padding: 0.5rem;
  text-align: center;
  border-right: 1px solid var(--ts-border, #333);
}
.wfc-day-header:last-child { border-right: none; }
.wfc-day-name {
  font-size: 0.6875rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--ts-text-muted);
}
.wfc-day-num {
  font-size: 1.125rem;
  font-weight: 600;
  margin-top: 0.125rem;
}
.wfc-day-today .wfc-day-num {
  color: var(--ts-accent);
}

.wfc-time-grid {
  flex: 1;
  display: grid;
  grid-template-columns: 60px repeat(7, 1fr);
  overflow-y: auto;
  position: relative;
}
.wfc-time-col {
  border-right: 1px solid var(--ts-border, #333);
}
.wfc-time-label {
  height: 48px;
  padding: 0.125rem 0.375rem;
  font-size: 0.6875rem;
  color: var(--ts-text-muted);
  border-bottom: 1px solid var(--ts-border, #333);
  text-align: right;
}
.wfc-day-col {
  position: relative;
  border-right: 1px solid var(--ts-border, #333);
}
.wfc-day-col:last-child { border-right: none; }
.wfc-hour-cell {
  height: 48px;
  border-bottom: 1px solid var(--ts-border, #333);
}

.wfc-event {
  position: absolute;
  left: 4px;
  right: 4px;
  padding: 0.25rem 0.375rem;
  border-radius: 0.25rem;
  background: var(--ts-accent);
  color: white;
  font-size: 0.75rem;
  cursor: pointer;
  overflow: hidden;
  z-index: 2;
  border-left: 3px solid rgba(255, 255, 255, 0.5);
  transition: filter 0.15s;
}
.wfc-event:hover { filter: brightness(1.15); }
.wfc-event-coding { background: var(--ts-accent-blue); }
.wfc-event-daily { background: var(--ts-success); }
.wfc-event-one_time { background: var(--ts-accent-violet); }
.wfc-event-recurring { margin-right: 0.25rem; }
.wfc-event-title {
  display: block;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.wfc-event-time {
  display: block;
  font-size: 0.6875rem;
  opacity: 0.85;
}
</style>
