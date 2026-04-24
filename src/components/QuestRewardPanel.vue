<template>
  <Transition name="reward-panel">
    <div
      v-if="visible && quest"
      class="reward-panel"
      @click.stop
    >
      <!-- Header -->
      <div class="rp-header">
        <div class="rp-icon">
          {{ quest.icon }}
        </div>
        <div class="rp-title-section">
          <h3 class="rp-title">
            {{ quest.name }}
          </h3>
          <p class="rp-tagline">
            {{ quest.tagline }}
          </p>
        </div>
        <button
          class="rp-close"
          @click="$emit('close')"
        >
          ✕
        </button>
      </div>

      <!-- Rewards Section -->
      <div
        v-if="quest.rewards && quest.rewards.length > 0"
        class="rp-rewards"
      >
        <div class="rp-rewards-label">
          🎁 Quest Rewards
        </div>
        <div class="rp-reward-grid">
          <div 
            v-for="(reward, index) in quest.rewards"
            :key="index"
            class="rp-reward-item"
          >
            <span class="rp-reward-icon">{{ quest.rewardIcons?.[index] || '🎁' }}</span>
            <span class="rp-reward-text">{{ reward }}</span>
          </div>
        </div>
      </div>

      <!-- Quest Steps Preview -->
      <div
        v-if="quest.questSteps && quest.questSteps.length > 0"
        class="rp-steps"
      >
        <div class="rp-steps-label">
          📋 What you'll learn
        </div>
        <div class="rp-step-list">
          <div 
            v-for="(step, index) in quest.questSteps.slice(0, 3)"
            :key="index"
            class="rp-step-item"
          >
            <span class="rp-step-number">{{ index + 1 }}</span>
            <span class="rp-step-text">{{ step.label }}</span>
          </div>
          <div
            v-if="quest.questSteps.length > 3"
            class="rp-step-more"
          >
            +{{ quest.questSteps.length - 3 }} more steps
          </div>
        </div>
      </div>

      <!-- Multiple Choice Section -->
      <div
        v-if="showChoices"
        class="rp-choices"
      >
        <div class="rp-choice-question">
          {{ choiceQuestion }}
        </div>
        <div class="rp-choice-buttons">
          <button 
            v-for="choice in choices"
            :key="choice.value"
            class="rp-choice-btn"
            :class="choice.primary ? 'primary' : 'secondary'"
            @click="handleChoice(choice.value)"
          >
            {{ choice.label }}
          </button>
        </div>
      </div>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue';
import type { SkillNode } from '../stores/skill-tree';

export interface RewardChoice {
  label: string;
  value: string;
  primary?: boolean;
}

const props = defineProps<{
  visible: boolean;
  quest: SkillNode | null;
  showChoices?: boolean;
  choiceQuestion?: string;
  choices?: RewardChoice[];
}>();

const emit = defineEmits<{
  close: [];
  choice: [value: string];
}>();

function handleChoice(value: string) {
  emit('choice', value);
}

function handleEscape(event: KeyboardEvent) {
  if (event.key === 'Escape' && props.visible) {
    emit('close');
  }
}

// Add keyboard event listener when visible
onMounted(() => {
  document.addEventListener('keydown', handleEscape);
});

onUnmounted(() => {
  document.removeEventListener('keydown', handleEscape);
});
</script>

<style scoped>
.reward-panel {
  position: fixed;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  z-index: 200;
  width: 400px;
  max-width: 90vw;
  max-height: 80vh;
  background: rgba(15, 23, 42, 0.95);
  border: 1px solid rgba(139, 92, 246, 0.3);
  border-radius: 16px;
  backdrop-filter: blur(16px);
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.6);
  overflow-y: auto;
}

/* Header */
.rp-header {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 20px 20px 16px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
}

.rp-icon {
  font-size: 2.5rem;
  flex-shrink: 0;
  margin-top: 4px;
}

.rp-title-section {
  flex: 1;
  min-width: 0;
}

.rp-title {
  font-size: 1.1rem;
  font-weight: 700;
  color: var(--ts-text-primary, #eaecf4);
  margin: 0 0 4px 0;
  line-height: 1.3;
}

.rp-tagline {
  font-size: 0.85rem;
  color: var(--ts-text-muted, #9ca3af);
  margin: 0;
  line-height: 1.4;
}

.rp-close {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  border: none;
  background: rgba(255, 255, 255, 0.1);
  color: var(--ts-text-muted, #9ca3af);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 0.9rem;
  flex-shrink: 0;
  transition: all 0.2s ease;
}

.rp-close:hover {
  background: rgba(255, 255, 255, 0.2);
  color: var(--ts-text-primary, #eaecf4);
}

/* Rewards */
.rp-rewards {
  padding: 16px 20px;
}

.rp-rewards-label {
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--ts-accent, #8b5cf6);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin-bottom: 12px;
}

.rp-reward-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.rp-reward-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  background: rgba(255, 215, 0, 0.1);
  border: 1px solid rgba(255, 215, 0, 0.2);
  border-radius: 20px;
  font-size: 0.8rem;
  color: #ffd700;
}

.rp-reward-icon {
  font-size: 0.9rem;
}

.rp-reward-text {
  font-weight: 500;
}

/* Steps */
.rp-steps {
  padding: 16px 20px;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
}

.rp-steps-label {
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--ts-text-secondary, #d1d5db);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin-bottom: 12px;
}

.rp-step-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.rp-step-item {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 0.85rem;
}

.rp-step-number {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: rgba(139, 92, 246, 0.15);
  color: var(--ts-accent, #8b5cf6);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 0.7rem;
  font-weight: 700;
  flex-shrink: 0;
}

.rp-step-text {
  color: var(--ts-text-secondary, #d1d5db);
  line-height: 1.4;
}

.rp-step-more {
  font-size: 0.8rem;
  color: var(--ts-text-muted, #9ca3af);
  font-style: italic;
  margin-left: 30px;
}

/* Multiple Choice */
.rp-choices {
  padding: 16px 20px;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
}

.rp-choice-question {
  font-size: 0.9rem;
  color: var(--ts-text-primary, #eaecf4);
  margin-bottom: 12px;
  line-height: 1.4;
  font-weight: 500;
}

.rp-choice-buttons {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.rp-choice-btn {
  flex: 1;
  min-width: 80px;
  padding: 10px 16px;
  border-radius: 8px;
  border: none;
  font-size: 0.85rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s ease;
}

.rp-choice-btn.primary {
  background: var(--ts-accent, #8b5cf6);
  color: white;
}

.rp-choice-btn.primary:hover {
  background: var(--ts-accent-hover, #7c3aed);
  transform: translateY(-1px);
}

.rp-choice-btn.secondary {
  background: rgba(255, 255, 255, 0.1);
  color: var(--ts-text-secondary, #d1d5db);
  border: 1px solid rgba(255, 255, 255, 0.2);
}

.rp-choice-btn.secondary:hover {
  background: rgba(255, 255, 255, 0.15);
  color: var(--ts-text-primary, #eaecf4);
}

/* Transitions */
.reward-panel-enter-active {
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.reward-panel-leave-active {
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}

.reward-panel-enter-from {
  opacity: 0;
  transform: translate(-50%, -50%) scale(0.9);
}

.reward-panel-leave-to {
  opacity: 0;
  transform: translate(-50%, -50%) scale(0.95);
}

/* Backdrop overlay for focus isolation */
.reward-panel::before {
  content: '';
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.4);
  z-index: -1;
  backdrop-filter: blur(2px);
}

/* Mobile adjustments */
@media (max-width: 640px) {
  .reward-panel {
    width: calc(100vw - 32px);
    max-height: calc(100vh - 100px);
  }
  
  .rp-header {
    padding: 16px 16px 14px;
  }
  
  .rp-rewards, .rp-steps, .rp-choices {
    padding: 14px 16px;
  }
  
  .rp-choice-buttons {
    flex-direction: column;
  }
  
  .rp-choice-btn {
    min-width: unset;
  }
}
</style>