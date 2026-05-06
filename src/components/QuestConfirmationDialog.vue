<template>
  <Transition name="quest-confirm-backdrop">
    <div
      v-if="visible"
      class="quest-confirm-backdrop"
      @click.self="$emit('cancel')"
    >
      <Transition name="quest-confirm-dialog">
        <div
          v-if="visible"
          class="quest-confirm-dialog"
          @click.stop
        >
          <!-- Header -->
          <div class="qcd-header">
            <span class="qcd-icon">{{ quest?.icon || '⚔️' }}</span>
            <div class="qcd-title-area">
              <h3 class="qcd-title">
                {{ quest?.name || 'Quest Available' }}
              </h3>
              <p class="qcd-tagline">
                {{ quest?.tagline || 'Ready to embark on this journey?' }}
              </p>
            </div>
            <button
              class="qcd-close"
              title="Cancel"
              @click="$emit('cancel')"
            >
              ✕
            </button>
          </div>

          <!-- Quest Details -->
          <div class="qcd-content">
            <p class="qcd-description">
              {{ quest?.description || 'Accept this quest to continue your journey.' }}
            </p>

            <!-- Steps Preview -->
            <div
              v-if="quest?.questSteps?.length"
              class="qcd-steps"
            >
              <div class="qcd-section-label">
                📋 What you'll do:
              </div>
              <div class="qcd-step-list">
                <div 
                  v-for="(step, i) in quest.questSteps.slice(0, 3)" 
                  :key="i" 
                  class="qcd-step-preview"
                >
                  <span class="qcd-step-num">{{ i + 1 }}</span>
                  <span class="qcd-step-text">{{ step.label }}</span>
                </div>
                <div
                  v-if="quest.questSteps.length > 3"
                  class="qcd-step-more"
                >
                  <span class="qcd-step-num">…</span>
                  <span class="qcd-step-text">{{ quest.questSteps.length - 3 }} more steps</span>
                </div>
              </div>
            </div>

            <!-- Rewards Preview -->
            <div
              v-if="quest?.rewards?.length"
              class="qcd-rewards"
            >
              <div class="qcd-section-label">
                🎁 Rewards:
              </div>
              <div class="qcd-reward-grid">
                <span
                  v-for="(reward, i) in quest.rewards"
                  :key="i"
                  class="qcd-reward-chip"
                >{{ quest.rewardIcons?.[i] || '🎁' }} {{ reward }}</span>
              </div>
            </div>

            <!-- AI Guidance Notice -->
            <div class="qcd-ai-notice">
              <span class="qcd-ai-icon">🤖</span>
              <div class="qcd-ai-text">
                <strong>AI-Guided Quest</strong>
                <p>I'll provide step-by-step guidance with multiple-choice questions. No manual configuration needed - I'll handle all the setup automatically.</p>
              </div>
            </div>
          </div>

          <!-- Action Buttons -->
          <div class="qcd-actions">
            <button
              class="qcd-btn-secondary"
              @click="$emit('cancel')"
            >
              💤 Not Now
            </button>
            <button
              class="qcd-btn-primary"
              @click="$emit('accept')"
            >
              ⚔️ Accept Quest
            </button>
          </div>
        </div>
      </Transition>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import type { SkillNode } from '../stores/skill-tree';

defineProps<{
  visible: boolean;
  quest: SkillNode | null;
}>();

defineEmits<{
  accept: [];
  cancel: [];
}>();
</script>

<style scoped>
/* Backdrop */
.quest-confirm-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.75);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 200;
  padding: 16px;
}

/* Dialog */
.quest-confirm-dialog {
  background: linear-gradient(135deg, rgba(15, 20, 35, 0.95) 0%, rgba(20, 25, 40, 0.95) 100%);
  backdrop-filter: blur(20px);
  border: 1px solid rgba(124, 111, 255, 0.25);
  border-radius: var(--ts-radius-lg, 16px);
  box-shadow: 
    0 20px 80px rgba(0, 0, 0, 0.6),
    0 0 60px rgba(124, 111, 255, 0.15);
  width: min(480px, 90vw);
  max-height: 85vh;
  overflow-y: auto;
  position: relative;
  animation: questDialogGlow 3s ease-in-out infinite alternate;
}

@keyframes questDialogGlow {
  0% { box-shadow: 0 20px 80px rgba(0, 0, 0, 0.6), 0 0 60px rgba(124, 111, 255, 0.15); }
  100% { box-shadow: 0 20px 80px rgba(0, 0, 0, 0.6), 0 0 80px rgba(124, 111, 255, 0.25); }
}

/* Header */
.qcd-header {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 20px 20px 16px;
  border-bottom: 1px solid rgba(124, 111, 255, 0.15);
}

.qcd-icon {
  font-size: 2.2rem;
  flex-shrink: 0;
  filter: drop-shadow(0 2px 8px rgba(124, 111, 255, 0.3));
}

.qcd-title-area { flex: 1; }

.qcd-title {
  margin: 0 0 4px;
  font-size: 1.3rem;
  font-weight: 700;
  color: var(--ts-text-primary, #eaecf4);
  background: linear-gradient(135deg, var(--ts-text-primary), var(--ts-accent-violet));
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.qcd-tagline {
  margin: 0;
  font-size: 0.9rem;
  color: var(--ts-text-muted, #6b7280);
  line-height: 1.4;
}

.qcd-close {
  background: none;
  border: none;
  color: var(--ts-text-dim, #4b5563);
  cursor: pointer;
  font-size: 1.1rem;
  width: 32px;
  height: 32px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s ease;
  flex-shrink: 0;
}

.qcd-close:hover {
  background: rgba(239, 68, 68, 0.15);
  color: var(--ts-error);
}

/* Content */
.qcd-content {
  padding: 0 20px 20px;
}

.qcd-description {
  font-size: 0.95rem;
  color: var(--ts-text-secondary, #9ca3af);
  line-height: 1.6;
  margin: 0 0 20px;
}

.qcd-section-label {
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--ts-accent, #7c6fff);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin: 16px 0 8px;
}

/* Steps */
.qcd-steps { margin-bottom: 20px; }

.qcd-step-list { display: flex; flex-direction: column; gap: 8px; }

.qcd-step-preview {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  background: rgba(124, 111, 255, 0.05);
  border-radius: var(--ts-radius-sm, 8px);
  border-left: 3px solid rgba(124, 111, 255, 0.3);
}

.qcd-step-more {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  background: rgba(124, 111, 255, 0.03);
  border-radius: var(--ts-radius-sm, 8px);
  border-left: 3px solid rgba(124, 111, 255, 0.15);
  font-style: italic;
}

.qcd-step-num {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  background: rgba(124, 111, 255, 0.2);
  color: var(--ts-accent, #7c6fff);
  font-size: 0.75rem;
  font-weight: 700;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.qcd-step-text {
  font-size: 0.85rem;
  color: var(--ts-text-secondary, #9ca3af);
  line-height: 1.4;
}

/* Rewards */
.qcd-rewards { margin-bottom: 20px; }

.qcd-reward-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.qcd-reward-chip {
  font-size: 0.8rem;
  padding: 4px 10px;
  border-radius: var(--ts-radius-pill, 999px);
  background: linear-gradient(135deg, rgba(255, 215, 0, 0.15) 0%, rgba(255, 165, 0, 0.1) 100%);
  color: var(--ts-warning);
  border: 1px solid rgba(255, 215, 0, 0.25);
  font-weight: 500;
}

/* AI Notice */
.qcd-ai-notice {
  display: flex;
  gap: 12px;
  padding: 12px;
  background: rgba(59, 130, 246, 0.08);
  border: 1px solid rgba(59, 130, 246, 0.2);
  border-radius: var(--ts-radius-md, 10px);
  margin-bottom: 20px;
}

.qcd-ai-icon {
  font-size: 1.4rem;
  flex-shrink: 0;
}

.qcd-ai-text strong {
  display: block;
  font-size: 0.85rem;
  color: var(--ts-accent-blue);
  font-weight: 600;
  margin-bottom: 4px;
}

.qcd-ai-text p {
  margin: 0;
  font-size: 0.8rem;
  color: var(--ts-text-secondary, #9ca3af);
  line-height: 1.4;
}

/* Actions */
.qcd-actions {
  display: flex;
  gap: 12px;
  padding: 16px 20px;
  border-top: 1px solid rgba(124, 111, 255, 0.15);
  background: rgba(15, 20, 35, 0.3);
}

.qcd-btn-secondary,
.qcd-btn-primary {
  flex: 1;
  padding: 12px 20px;
  border-radius: var(--ts-radius-md, 10px);
  font-size: 0.9rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s ease;
  border: none;
  position: relative;
  overflow: hidden;
}

.qcd-btn-secondary {
  background: rgba(71, 85, 105, 0.3);
  color: var(--ts-text-secondary, #9ca3af);
  border: 1px solid rgba(71, 85, 105, 0.5);
}

.qcd-btn-secondary:hover {
  background: rgba(71, 85, 105, 0.5);
  color: var(--ts-text-primary, #eaecf4);
  transform: translateY(-1px);
}

.qcd-btn-primary {
  background: linear-gradient(135deg, var(--ts-accent, #7c6fff) 0%, var(--ts-accent-violet, #a78bfa) 100%);
  color: var(--ts-text-on-accent);
  border: 1px solid rgba(124, 111, 255, 0.3);
  box-shadow: 0 4px 16px rgba(124, 111, 255, 0.3);
}

.qcd-btn-primary:hover {
  background: var(--ts-gradient-accent);
  transform: translateY(-2px);
  box-shadow: 0 8px 24px rgba(124, 111, 255, 0.4);
}

.qcd-btn-primary:active {
  transform: translateY(-1px);
}

/* Transitions */
.quest-confirm-backdrop-enter-active { animation: backdropFadeIn 0.3s ease-out; }
.quest-confirm-backdrop-leave-active { animation: backdropFadeIn 0.25s ease-in reverse; }

.quest-confirm-dialog-enter-active { animation: dialogSlideIn 0.4s cubic-bezier(0.34, 1.56, 0.64, 1); }
.quest-confirm-dialog-leave-active { animation: dialogSlideIn 0.3s ease-in reverse; }

@keyframes backdropFadeIn {
  0% { opacity: 0; }
  100% { opacity: 1; }
}

@keyframes dialogSlideIn {
  0% { 
    opacity: 0; 
    transform: translateY(-20px) scale(0.9); 
  }
  100% { 
    opacity: 1; 
    transform: translateY(0) scale(1); 
  }
}

/* Mobile */
@media (max-width: 640px) {
  .quest-confirm-dialog {
    width: calc(100vw - 24px);
    margin: 12px;
  }
  
  .qcd-header,
  .qcd-content,
  .qcd-actions {
    padding-left: 16px;
    padding-right: 16px;
  }
  
  .qcd-title { font-size: 1.2rem; }
  .qcd-icon { font-size: 2rem; }
  
  .qcd-actions {
    flex-direction: column;
  }
}
</style>