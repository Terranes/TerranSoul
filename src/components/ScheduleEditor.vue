<template>
  <div class="se-root">
    <label class="se-field">
      <span>Pattern</span>
      <select v-model="kind">
        <option value="none">No schedule</option>
        <option value="once">Once</option>
        <option value="daily">Daily</option>
        <option value="weekly">Weekly</option>
        <option value="monthly">Monthly</option>
      </select>
    </label>

    <template v-if="kind !== 'none'">
      <label class="se-field">
        <span>Start at</span>
        <input v-model="startAtLocal" type="datetime-local" />
      </label>

      <label class="se-field">
        <span>Duration (minutes)</span>
        <input v-model.number="durationMinutes" type="number" min="5" max="1440" />
      </label>

      <template v-if="kind !== 'once'">
        <label class="se-field">
          <span>Repeat every</span>
          <div class="se-interval-row">
            <input v-model.number="interval" type="number" min="1" max="365" />
            <span>{{ intervalUnit }}</span>
          </div>
        </label>

        <div v-if="kind === 'weekly'" class="se-field">
          <span>On these days</span>
          <div class="se-weekdays">
            <label v-for="d in weekdayOptions" :key="d.value" class="se-weekday">
              <input
                type="checkbox"
                :checked="selectedWeekdays.has(d.value)"
                @change="toggleWeekday(d.value)"
              />
              <span>{{ d.label }}</span>
            </label>
          </div>
        </div>

        <label v-if="kind === 'monthly'" class="se-field">
          <span>Day of month</span>
          <input v-model.number="dayOfMonth" type="number" min="1" max="31" />
        </label>

        <label class="se-field">
          <span>End on (optional)</span>
          <input v-model="endByLocal" type="date" />
        </label>
      </template>

      <p class="se-preview">{{ previewText }}</p>
    </template>

    <div class="se-actions">
      <button class="se-btn se-btn-primary" @click="onSave">Save schedule</button>
      <button v-if="props.plan.schedule" class="se-btn" @click="onClear">Remove schedule</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import {
  type RecurrencePattern,
  type Weekday,
  type WorkflowPlan,
  type WorkflowSchedule,
  formatRecurrence,
} from '../stores/workflow-plans';

const props = defineProps<{ plan: WorkflowPlan }>();
const emit = defineEmits<{ save: [schedule: WorkflowSchedule | null] }>();

type Kind = 'none' | 'once' | 'daily' | 'weekly' | 'monthly';

const weekdayOptions: { value: Weekday; label: string }[] = [
  { value: 'sunday', label: 'Sun' },
  { value: 'monday', label: 'Mon' },
  { value: 'tuesday', label: 'Tue' },
  { value: 'wednesday', label: 'Wed' },
  { value: 'thursday', label: 'Thu' },
  { value: 'friday', label: 'Fri' },
  { value: 'saturday', label: 'Sat' },
];

const kind = ref<Kind>(props.plan.schedule?.recurrence.kind ?? 'none');
const startAtLocal = ref(toLocalInput(props.plan.schedule?.start_at ?? Date.now()));
const interval = ref(getInterval(props.plan.schedule?.recurrence) ?? 1);
const dayOfMonth = ref(getDayOfMonth(props.plan.schedule?.recurrence) ?? 1);
const selectedWeekdays = ref(new Set<Weekday>(getWeekdays(props.plan.schedule?.recurrence)));
const endByLocal = ref(
  props.plan.schedule?.end_at ? toLocalDate(props.plan.schedule.end_at) : '',
);
const durationMinutes = ref(props.plan.schedule?.duration_minutes ?? 30);

watch(
  () => props.plan,
  (p) => {
    kind.value = p.schedule?.recurrence.kind ?? 'none';
    startAtLocal.value = toLocalInput(p.schedule?.start_at ?? Date.now());
    interval.value = getInterval(p.schedule?.recurrence) ?? 1;
    dayOfMonth.value = getDayOfMonth(p.schedule?.recurrence) ?? 1;
    selectedWeekdays.value = new Set(getWeekdays(p.schedule?.recurrence));
    endByLocal.value = p.schedule?.end_at ? toLocalDate(p.schedule.end_at) : '';
    durationMinutes.value = p.schedule?.duration_minutes ?? 30;
  },
);

const intervalUnit = computed(() => {
  if (kind.value === 'daily') return interval.value === 1 ? 'day' : 'days';
  if (kind.value === 'weekly') return interval.value === 1 ? 'week' : 'weeks';
  if (kind.value === 'monthly') return interval.value === 1 ? 'month' : 'months';
  return '';
});

const previewText = computed(() => {
  const pattern = buildPattern();
  if (!pattern) return '';
  return formatRecurrence(pattern);
});

function toggleWeekday(day: Weekday): void {
  if (selectedWeekdays.value.has(day)) selectedWeekdays.value.delete(day);
  else selectedWeekdays.value.add(day);
}

function buildPattern(): RecurrencePattern | null {
  if (kind.value === 'none') return null;
  if (kind.value === 'once') return { kind: 'once' };
  if (kind.value === 'daily') return { kind: 'daily', interval: interval.value };
  if (kind.value === 'weekly') {
    return {
      kind: 'weekly',
      interval: interval.value,
      weekdays: weekdayOptions.filter((w) => selectedWeekdays.value.has(w.value)).map((w) => w.value),
    };
  }
  if (kind.value === 'monthly') {
    return {
      kind: 'monthly',
      interval: interval.value,
      day_of_month: dayOfMonth.value,
    };
  }
  return null;
}

function onSave(): void {
  const pattern = buildPattern();
  if (!pattern) {
    emit('save', null);
    return;
  }
  const schedule: WorkflowSchedule = {
    recurrence: pattern,
    start_at: new Date(startAtLocal.value).getTime(),
    end_at: endByLocal.value ? new Date(endByLocal.value).getTime() : undefined,
    duration_minutes: durationMinutes.value,
    timezone: Intl.DateTimeFormat().resolvedOptions().timeZone ?? 'UTC',
    last_fired_at: props.plan.schedule?.last_fired_at,
  };
  emit('save', schedule);
}

function onClear(): void {
  kind.value = 'none';
  emit('save', null);
}

function toLocalInput(epochMs: number): string {
  const d = new Date(epochMs);
  const pad = (n: number): string => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

function toLocalDate(epochMs: number): string {
  const d = new Date(epochMs);
  const pad = (n: number): string => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

function getInterval(rec?: RecurrencePattern): number | null {
  if (!rec || rec.kind === 'once') return null;
  return rec.interval;
}

function getDayOfMonth(rec?: RecurrencePattern): number | null {
  if (rec?.kind === 'monthly') return rec.day_of_month;
  return null;
}

function getWeekdays(rec?: RecurrencePattern): Weekday[] {
  if (rec?.kind === 'weekly') return rec.weekdays;
  return [];
}
</script>

<style scoped>
.se-root {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding: 0.5rem 0;
}
.se-field {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  font-size: 0.875rem;
}
.se-field input,
.se-field select {
  padding: 0.375rem 0.5rem;
  background: var(--ts-surface, #1a1a1a);
  border: 1px solid var(--ts-border, #333);
  border-radius: 0.25rem;
  color: var(--ts-text);
  font-family: inherit;
}
.se-interval-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}
.se-interval-row input { width: 80px; }
.se-weekdays {
  display: flex;
  gap: 0.5rem;
  flex-wrap: wrap;
  margin-top: 0.25rem;
}
.se-weekday {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  font-size: 0.8125rem;
}
.se-preview {
  margin: 0.5rem 0;
  padding: 0.5rem;
  background: var(--ts-accent-glow);
  border-radius: 0.25rem;
  color: var(--ts-accent);
  font-size: 0.875rem;
  font-style: italic;
}
.se-actions {
  display: flex;
  gap: 0.5rem;
  margin-top: 0.5rem;
}
.se-btn {
  padding: 0.375rem 0.75rem;
  background: transparent;
  border: 1px solid var(--ts-border, #333);
  border-radius: 0.375rem;
  color: var(--ts-text);
  cursor: pointer;
  font-size: 0.875rem;
}
.se-btn-primary {
  background: var(--ts-accent);
  border-color: var(--ts-accent);
  color: white;
}
</style>
