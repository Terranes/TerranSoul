<template>
  <Teleport to="body">
    <div
      class="quest-backdrop"
      @click.self="$emit('close')"
    >
      <div
        class="quest-dialog"
        :class="'quest-status-' + status"
      >
        <!-- Close button -->
        <button
          class="quest-close"
          @click="$emit('close')"
        >
          ✕
        </button>

        <!-- Header -->
        <div class="quest-header">
          <div
            class="quest-icon-wrap"
            :class="'quest-status-' + status"
          >
            <span class="quest-icon">{{ node.icon }}</span>
            <span
              v-if="status === 'active'"
              class="quest-icon-badge"
            >✓</span>
          </div>
          <div class="quest-header-text">
            <h3 class="quest-name">
              {{ node.name }}
            </h3>
            <span class="quest-tier-badge">{{ tierLabel }}</span>
            <span class="quest-cat-badge">{{ categoryLabel }}</span>
          </div>
        </div>

        <!-- Status banner -->
        <div
          class="quest-status-banner"
          :class="'quest-banner-' + status"
        >
          <span v-if="status === 'active'">✅ Skill Active</span>
          <span v-else-if="status === 'available'">⚡ Ready to Unlock!</span>
          <span v-else>🔒 Prerequisites Required</span>
          <span
            v-if="activationTime"
            class="quest-activation-time"
          >
            Activated {{ formatRelativeTime(activationTime) }}
          </span>
        </div>

        <!-- Description -->
        <p class="quest-description">
          {{ node.description }}
        </p>

        <!-- Prerequisites -->
        <section
          v-if="node.requires.length > 0"
          class="quest-section"
        >
          <h4 class="quest-section-title">
            📋 Prerequisites
          </h4>
          <ul class="quest-prereq-list">
            <li
              v-for="reqId in node.requires"
              :key="reqId"
              class="quest-prereq-item"
            >
              <span class="quest-prereq-check">{{ isPrereqMet(reqId) ? '✅' : '❌' }}</span>
              <span>{{ getNodeName(reqId) }}</span>
            </li>
          </ul>
        </section>

        <!-- Quest Steps -->
        <section
          v-if="status !== 'active'"
          class="quest-section"
        >
          <h4 class="quest-section-title">
            🗺️ Quest Steps
          </h4>
          <ol class="quest-steps">
            <li
              v-for="(step, i) in node.questSteps"
              :key="i"
              class="quest-step"
            >
              <span class="quest-step-num">{{ i + 1 }}</span>
              <span class="quest-step-label">{{ step.label }}</span>
              <button
                v-if="step.action === 'navigate' && step.target"
                class="quest-step-action"
                @click="$emit('navigate', step.target!)"
              >
                Go →
              </button>
              <a
                v-if="step.action === 'external' && step.target"
                class="quest-step-action"
                :href="step.target"
                target="_blank"
                rel="noopener noreferrer"
              >Open ↗</a>
            </li>
          </ol>
        </section>

        <!-- Rewards -->
        <section class="quest-section">
          <h4 class="quest-section-title">
            🎁 Rewards
          </h4>
          <ul class="quest-reward-list">
            <li
              v-for="(reward, i) in node.rewards"
              :key="i"
              class="quest-reward-item"
            >
              <span class="quest-reward-icon">{{ node.rewardIcons?.[i] || (status === 'active' ? '✨' : '🎯') }}</span>
              <span
                class="quest-reward-text"
                :class="status === 'active' ? 'reward-active' : ''"
              >{{ reward }}</span>
            </li>
          </ul>
        </section>

        <!-- Video Reference -->
        <section
          v-if="node.videoRef"
          class="quest-section"
        >
          <h4 class="quest-section-title">
            🎬 See it in action
          </h4>
          <div class="quest-video-wrapper">
            <iframe
              :src="videoEmbedUrl"
              class="quest-video-iframe"
              allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
              allowfullscreen
              loading="lazy"
              referrerpolicy="no-referrer"
              sandbox="allow-scripts allow-same-origin"
            />
          </div>
        </section>

        <!-- Combos -->
        <section
          v-if="node.combos.length > 0"
          class="quest-section"
        >
          <h4 class="quest-section-title">
            🔥 Combo Potential
          </h4>
          <div class="quest-combo-list">
            <div
              v-for="combo in node.combos"
              :key="combo.name"
              class="quest-combo-item"
            >
              <span class="quest-combo-icon">{{ combo.icon }}</span>
              <div class="quest-combo-body">
                <span class="quest-combo-name">{{ combo.name }}</span>
                <span class="quest-combo-desc">{{ combo.description }}</span>
                <span class="quest-combo-needs">Needs: {{ combo.withSkills.map(getNodeName).join(' + ') }}</span>
              </div>
            </div>
          </div>
        </section>

        <!-- Action buttons -->
        <div class="quest-actions">
          <button
            v-if="!isPinned && status !== 'active'"
            class="quest-btn quest-btn-pin"
            @click="$emit('pin')"
          >
            📌 Pin Quest
          </button>
          <button
            v-if="isPinned"
            class="quest-btn quest-btn-unpin"
            @click="$emit('unpin')"
          >
            Unpin
          </button>
          <button
            v-if="status === 'available' && !isDismissed"
            class="quest-btn quest-btn-dismiss"
            @click="$emit('dismiss')"
          >
            Skip for now
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useSkillTreeStore, type SkillNode, type SkillStatus } from '../stores/skill-tree';

const props = defineProps<{
  node: SkillNode;
  status: SkillStatus;
  isPinned: boolean;
  isDismissed: boolean;
  activationTime: number | null;
}>();

defineEmits<{
  close: [];
  pin: [];
  unpin: [];
  dismiss: [];
  navigate: [target: string];
}>();

const skillTree = useSkillTreeStore();

const tierLabel = computed(() => {
  const labels = { foundation: '🏗️ Foundation', advanced: '⚡ Advanced', ultimate: '👑 Ultimate' };
  return labels[props.node.tier];
});

const categoryLabel = computed(() => {
  const labels = { brain: '🧠 Brain', voice: '🎤 Voice', avatar: '✨ Avatar', social: '🔗 Social', utility: '🔧 Utility' };
  return labels[props.node.category];
});

const videoEmbedUrl = computed(() => {
  if (!props.node.videoRef) return '';
  return `https://www.youtube-nocookie.com/embed/${encodeURIComponent(props.node.videoRef)}?rel=0&modestbranding=1`;
});

function getNodeName(id: string): string {
  return skillTree.nodes.find(n => n.id === id)?.name ?? id;
}

function isPrereqMet(reqId: string): boolean {
  return skillTree.getSkillStatus(reqId) === 'active';
}

function formatRelativeTime(ts: number): string {
  const diff = Date.now() - ts;
  const mins = Math.floor(diff / 60000);
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}
</script>

<style scoped>
.quest-backdrop {
  position: fixed; inset: 0; z-index: 200;
  background: rgba(0, 0, 0, 0.6); backdrop-filter: blur(6px);
  display: flex; align-items: center; justify-content: center;
  padding: var(--ts-space-lg);
}
.quest-dialog {
  width: 100%; max-width: 520px; max-height: 85vh; overflow-y: auto;
  background: var(--ts-bg-card);
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-xl);
  padding: var(--ts-space-xl);
  position: relative;
  box-shadow: var(--ts-shadow-lg);
  scrollbar-width: thin;
}
.quest-dialog.quest-status-active { border-color: rgba(52, 211, 153, 0.3); }
.quest-dialog.quest-status-available { border-color: rgba(251, 191, 36, 0.3); }
.quest-close {
  position: absolute; top: 12px; right: 12px;
  background: none; border: none; color: var(--ts-text-muted);
  font-size: 1.1rem; cursor: pointer; padding: 4px;
  transition: color var(--ts-transition-fast);
}
.quest-close:hover { color: var(--ts-text-primary); }

/* Header */
.quest-header { display: flex; gap: var(--ts-space-md); align-items: center; margin-bottom: var(--ts-space-md); }
.quest-icon-wrap {
  width: 56px; height: 56px; border-radius: 50%;
  display: flex; align-items: center; justify-content: center;
  background: var(--ts-bg-surface); border: 2px solid var(--ts-border);
  position: relative; flex-shrink: 0;
}
.quest-icon-wrap.quest-status-active { border-color: var(--ts-success); }
.quest-icon-wrap.quest-status-available { border-color: var(--ts-warning); animation: pulse-glow 2s infinite; }
.quest-icon { font-size: 1.8rem; }
.quest-icon-badge {
  position: absolute; bottom: -2px; right: -2px;
  width: 20px; height: 20px; border-radius: 50%;
  background: var(--ts-success); color: #fff; font-size: 0.7rem;
  display: flex; align-items: center; justify-content: center;
}
.quest-header-text { display: flex; flex-direction: column; gap: 4px; }
.quest-name { margin: 0; font-size: 1.15rem; color: var(--ts-text-primary); }
.quest-tier-badge, .quest-cat-badge {
  display: inline-block; font-size: 0.7rem; color: var(--ts-text-muted);
  padding: 1px 6px; border: 1px solid var(--ts-border-subtle); border-radius: var(--ts-radius-sm);
  margin-right: 4px;
}

/* Status banner */
.quest-status-banner {
  padding: var(--ts-space-sm) var(--ts-space-md);
  border-radius: var(--ts-radius-md);
  font-size: 0.85rem; font-weight: 600;
  margin-bottom: var(--ts-space-md);
  display: flex; align-items: center; justify-content: space-between;
}
.quest-banner-active { background: var(--ts-success-bg); color: var(--ts-success); }
.quest-banner-available { background: rgba(251, 191, 36, 0.1); color: var(--ts-warning); }
.quest-banner-locked { background: rgba(255, 255, 255, 0.04); color: var(--ts-text-muted); }
.quest-activation-time { font-size: 0.75rem; font-weight: 400; color: var(--ts-text-muted); }

.quest-description { font-size: 0.88rem; color: var(--ts-text-secondary); line-height: 1.5; margin: 0 0 var(--ts-space-md); }

/* Sections */
.quest-section { margin-bottom: var(--ts-space-md); }
.quest-section-title { margin: 0 0 var(--ts-space-sm); font-size: 0.85rem; color: var(--ts-text-primary); }

.quest-prereq-list { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 4px; }
.quest-prereq-item { display: flex; align-items: center; gap: 6px; font-size: 0.82rem; color: var(--ts-text-secondary); }
.quest-prereq-check { font-size: 0.85rem; }

.quest-steps { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: var(--ts-space-sm); }
.quest-step {
  display: flex; align-items: center; gap: var(--ts-space-sm);
  padding: var(--ts-space-sm) var(--ts-space-md);
  background: var(--ts-bg-surface); border-radius: var(--ts-radius-md);
}
.quest-step-num {
  width: 22px; height: 22px; border-radius: 50%;
  background: var(--ts-accent-glow); color: var(--ts-accent);
  font-size: 0.75rem; font-weight: 700;
  display: flex; align-items: center; justify-content: center;
  flex-shrink: 0;
}
.quest-step-label { flex: 1; font-size: 0.82rem; color: var(--ts-text-secondary); }
.quest-step-action {
  padding: 3px 10px; border: 1px solid var(--ts-accent);
  border-radius: var(--ts-radius-sm); background: transparent;
  color: var(--ts-accent); font-size: 0.75rem; font-weight: 600;
  cursor: pointer; text-decoration: none;
  transition: background var(--ts-transition-fast);
}
.quest-step-action:hover { background: var(--ts-accent-glow); }

.quest-reward-list { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 6px; }
.quest-reward-item { font-size: 0.82rem; color: var(--ts-text-secondary); display: flex; align-items: center; gap: 8px; }
.quest-reward-icon { font-size: 1.1rem; flex-shrink: 0; }
.quest-reward-text { line-height: 1.3; }
.reward-active { color: var(--ts-success); font-weight: 600; }

.quest-video-wrapper {
  position: relative; padding-bottom: 56.25%; height: 0; overflow: hidden;
  border-radius: var(--ts-radius-md); border: 1px solid var(--ts-border-subtle);
  margin-top: var(--ts-space-xs);
}
.quest-video-iframe {
  position: absolute; top: 0; left: 0; width: 100%; height: 100%;
  border: none;
}

.quest-combo-list { display: flex; flex-direction: column; gap: var(--ts-space-sm); }
.quest-combo-item {
  display: flex; gap: var(--ts-space-sm); align-items: flex-start;
  padding: var(--ts-space-sm) var(--ts-space-md);
  border-radius: var(--ts-radius-md);
  background: linear-gradient(135deg, rgba(124, 111, 255, 0.06), rgba(52, 211, 153, 0.06));
  border: 1px solid var(--ts-border-subtle);
}
.quest-combo-icon { font-size: 1.2rem; flex-shrink: 0; }
.quest-combo-body { display: flex; flex-direction: column; gap: 2px; }
.quest-combo-name { font-size: 0.82rem; font-weight: 600; color: var(--ts-accent-violet); }
.quest-combo-desc { font-size: 0.75rem; color: var(--ts-text-muted); }
.quest-combo-needs { font-size: 0.7rem; color: var(--ts-text-dim); }

/* Actions */
.quest-actions { display: flex; gap: var(--ts-space-sm); margin-top: var(--ts-space-md); }
.quest-btn {
  padding: 6px 14px; border-radius: var(--ts-radius-md);
  font-size: 0.8rem; font-weight: 600; cursor: pointer;
  transition: background var(--ts-transition-fast), border-color var(--ts-transition-fast);
}
.quest-btn-pin { background: var(--ts-accent-glow); border: 1px solid var(--ts-accent); color: var(--ts-accent); }
.quest-btn-pin:hover { background: var(--ts-accent); color: var(--ts-text-on-accent); }
.quest-btn-unpin { background: transparent; border: 1px solid var(--ts-border); color: var(--ts-text-secondary); }
.quest-btn-unpin:hover { background: var(--ts-bg-hover); }
.quest-btn-dismiss { background: transparent; border: 1px solid var(--ts-border-subtle); color: var(--ts-text-muted); }
.quest-btn-dismiss:hover { background: var(--ts-bg-hover); }

@keyframes pulse-glow {
  0%, 100% { box-shadow: 0 0 0 0 rgba(251, 191, 36, 0); }
  50% { box-shadow: 0 0 12px 2px rgba(251, 191, 36, 0.3); }
}

@media (max-width: 640px) {
  .quest-dialog { max-width: 100%; max-height: 90vh; padding: var(--ts-space-lg); }
}
</style>
