<template>
  <Transition name="upgrade-slide">
    <div v-if="visible" class="upgrade-backdrop">
      <div class="upgrade-dialog">
        <!-- Game-style header -->
        <div class="upgrade-header">
          <div class="upgrade-icon">⚡</div>
          <h3 class="upgrade-title">Level Up Your Brain!</h3>
        </div>

        <p class="upgrade-desc">
          I'm having trouble with some of your questions. A more capable model would
          give you much better answers.
        </p>

        <!-- Current vs Upgrade comparison -->
        <div class="upgrade-compare">
          <div class="upgrade-tier current">
            <span class="tier-badge">Current</span>
            <strong>{{ currentModelName }}</strong>
            <small>{{ currentModelDesc }}</small>
          </div>
          <div class="upgrade-arrow">→</div>
          <div class="upgrade-tier recommended">
            <span class="tier-badge rec">⭐ Recommended</span>
            <strong>{{ recommendedName }}</strong>
            <small>{{ recommendedDesc }}</small>
          </div>
        </div>

        <!-- Upgrade options -->
        <div class="upgrade-options">
          <button
            v-for="opt in options"
            :key="opt.id"
            :class="['upgrade-opt-btn', { selected: selectedOption === opt.id, primary: opt.primary }]"
            @click="selectedOption = opt.id"
          >
            <span class="opt-icon">{{ opt.icon }}</span>
            <div class="opt-text">
              <strong>{{ opt.label }}</strong>
              <small>{{ opt.detail }}</small>
            </div>
          </button>
        </div>

        <!-- Action buttons -->
        <div class="upgrade-actions">
          <button class="btn-upgrade" :disabled="!selectedOption" @click="handleAccept">
            {{ acceptLabel }}
          </button>
          <button class="btn-dismiss" @click="$emit('dismiss')">
            Not now
          </button>
        </div>
      </div>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue';

export interface UpgradeOption {
  id: string;
  icon: string;
  label: string;
  detail: string;
  primary?: boolean;
}

const props = defineProps<{
  visible: boolean;
  currentModelName: string;
  currentModelDesc: string;
  recommendedName: string;
  recommendedDesc: string;
  options: UpgradeOption[];
}>();

const emit = defineEmits<{
  (e: 'accept', optionId: string): void;
  (e: 'dismiss'): void;
}>();

const selectedOption = ref<string | null>(props.options.find((o) => o.primary)?.id ?? null);

const acceptLabel = computed(() => {
  if (!selectedOption.value) return 'Select an option';
  const opt = props.options.find((o) => o.id === selectedOption.value);
  if (opt?.id === 'local') return '⬇ Install & Activate';
  if (opt?.id === 'free_upgrade') return '☁️ Switch Now';
  if (opt?.id === 'paid') return '🔑 Configure API Key';
  return '✓ Upgrade';
});

function handleAccept() {
  if (selectedOption.value) {
    emit('accept', selectedOption.value);
  }
}
</script>

<style scoped>
.upgrade-backdrop {
  position: fixed;
  inset: 0;
  z-index: 200;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(4px);
}

.upgrade-dialog {
  width: min(420px, 92vw);
  background: linear-gradient(135deg, #1e293b 0%, #0f172a 100%);
  border: 1px solid rgba(59, 130, 246, 0.3);
  border-radius: 16px;
  padding: 1.5rem;
  display: flex;
  flex-direction: column;
  gap: 1rem;
  box-shadow: 0 0 40px rgba(59, 130, 246, 0.15), 0 20px 60px rgba(0, 0, 0, 0.4);
  animation: dialogPulse 2s ease-in-out infinite alternate;
}

@keyframes dialogPulse {
  from { box-shadow: 0 0 40px rgba(59, 130, 246, 0.15), 0 20px 60px rgba(0, 0, 0, 0.4); }
  to   { box-shadow: 0 0 50px rgba(59, 130, 246, 0.25), 0 20px 60px rgba(0, 0, 0, 0.4); }
}

.upgrade-header {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.upgrade-icon {
  font-size: 1.8rem;
  animation: iconBounce 1s ease-in-out infinite alternate;
}

@keyframes iconBounce {
  from { transform: translateY(0); }
  to   { transform: translateY(-3px); }
}

.upgrade-title {
  margin: 0;
  font-size: 1.2rem;
  background: linear-gradient(90deg, #60a5fa, #a78bfa);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.upgrade-desc {
  margin: 0;
  font-size: 0.85rem;
  color: #94a3b8;
  line-height: 1.5;
}

/* Compare boxes */
.upgrade-compare {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.upgrade-tier {
  flex: 1;
  padding: 0.6rem;
  border-radius: 8px;
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
}

.upgrade-tier.current {
  background: rgba(100, 116, 139, 0.15);
  border: 1px solid rgba(100, 116, 139, 0.3);
}

.upgrade-tier.recommended {
  background: rgba(59, 130, 246, 0.1);
  border: 1px solid rgba(59, 130, 246, 0.4);
}

.tier-badge {
  font-size: 0.65rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: #64748b;
}

.tier-badge.rec {
  color: #60a5fa;
}

.upgrade-tier strong {
  font-size: 0.85rem;
}

.upgrade-tier small {
  color: #64748b;
  font-size: 0.72rem;
}

.upgrade-arrow {
  font-size: 1.2rem;
  color: #3b82f6;
  flex-shrink: 0;
}

/* Options */
.upgrade-options {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.upgrade-opt-btn {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  padding: 0.6rem 0.8rem;
  background: rgba(15, 23, 42, 0.8);
  border: 2px solid transparent;
  border-radius: 10px;
  cursor: pointer;
  color: #f1f5f9;
  text-align: left;
  transition: border-color 0.15s, background 0.15s;
}

.upgrade-opt-btn:hover { border-color: #334155; }
.upgrade-opt-btn.selected { border-color: #3b82f6; background: rgba(59, 130, 246, 0.08); }
.upgrade-opt-btn.primary { border-color: rgba(59, 130, 246, 0.3); }

.opt-icon { font-size: 1.2rem; flex-shrink: 0; }
.opt-text { display: flex; flex-direction: column; }
.opt-text strong { font-size: 0.8rem; }
.opt-text small { color: #64748b; font-size: 0.7rem; }

/* Action buttons */
.upgrade-actions {
  display: flex;
  gap: 0.5rem;
  justify-content: flex-end;
  margin-top: 0.25rem;
}

.btn-upgrade {
  padding: 0.5rem 1.25rem;
  background: linear-gradient(135deg, #3b82f6, #6366f1);
  color: #fff;
  border: none;
  border-radius: 8px;
  cursor: pointer;
  font-size: 0.85rem;
  font-weight: 600;
  transition: opacity 0.15s;
}

.btn-upgrade:disabled { opacity: 0.4; cursor: not-allowed; }
.btn-upgrade:hover:not(:disabled) { opacity: 0.9; }

.btn-dismiss {
  padding: 0.5rem 1rem;
  background: transparent;
  color: #64748b;
  border: 1px solid #334155;
  border-radius: 8px;
  cursor: pointer;
  font-size: 0.8rem;
}

.btn-dismiss:hover { color: #94a3b8; border-color: #475569; }

/* Transitions */
.upgrade-slide-enter-active { transition: all 0.3s ease-out; }
.upgrade-slide-leave-active { transition: all 0.2s ease-in; }
.upgrade-slide-enter-from { opacity: 0; transform: scale(0.95); }
.upgrade-slide-leave-to { opacity: 0; transform: scale(0.95); }
</style>
