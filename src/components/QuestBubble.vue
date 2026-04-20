<template>
  <div class="quest-hub" :style="questHubPosition">
    <!-- FF-style floating crystal orb -->
    <button
      class="ff-orb"
      :class="{ 'ff-orb--open': panelOpen }"
      :title="`Skill Progress: ${progressPercent}%`"
      @click="panelOpen = !panelOpen"
    >
      <svg class="ff-orb-ring" viewBox="0 0 52 52">
        <circle class="ff-orb-ring-bg" cx="26" cy="26" r="22" />
        <circle
          class="ff-orb-ring-fill"
          cx="26" cy="26" r="22"
          :stroke-dasharray="circumference"
          :stroke-dashoffset="circumference - (circumference * progressPercent / 100)"
        />
      </svg>
      <span class="ff-orb-crystal">✦</span>
      <span class="ff-orb-pct">{{ progressPercent }}%</span>
    </button>

    <!-- FF Skill Tree Panel -->
    <Transition name="quest-panel">
      <div v-if="panelOpen" class="ff-panel" @click.stop>
        <!-- Ornate header -->
        <div class="ff-header">
          <div class="ff-header-ornament">━━━ ✦ ━━━</div>
          <div class="ff-header-title">License Board</div>
          <div class="ff-header-sub">{{ activeCount }} / {{ totalNodes }} Licenses Obtained</div>
        </div>

        <!-- Crystal progress bar -->
        <div class="ff-xp-bar">
          <div class="ff-xp-fill" :style="{ width: progressPercent + '%' }" />
          <span class="ff-xp-text">{{ progressPercent }}%</span>
        </div>

        <!-- Tab row: Foundation / Advanced / Ultimate -->
        <div class="ff-tabs">
          <button
            v-for="tier in tiers"
            :key="tier.id"
            class="ff-tab"
            :class="{ 'ff-tab--active': activeTab === tier.id }"
            @click="activeTab = tier.id"
          >
            <span class="ff-tab-icon">{{ tier.icon }}</span>
            <span class="ff-tab-label">{{ tier.label }}</span>
          </button>
        </div>

        <!-- Skill node grid -->
        <div class="ff-grid">
          <div
            v-for="node in filteredNodes"
            :key="node.id"
            class="ff-node"
            :class="[
              'ff-node--' + getSkillStatus(node.id),
              { 'ff-node--recommended': isRecommended(node.id) }
            ]"
            @click="selectQuest(node.id)"
          >
            <!-- Connection lines (for chain deps) -->
            <div v-if="hasParentInTier(node)" class="ff-node-connector" />
            <!-- Node crystal -->
            <div class="ff-node-gem">
              <span class="ff-node-icon">{{ node.icon }}</span>
              <div v-if="getSkillStatus(node.id) === 'active'" class="ff-node-glow" />
            </div>
            <div class="ff-node-label">{{ node.name }}</div>
            <div class="ff-node-tier-badge">{{ tierBadge(node.tier) }}</div>
            <!-- Completion toggle -->
            <button
              v-if="getSkillStatus(node.id) !== 'locked'"
              class="ff-node-check"
              :class="{ 'ff-node-check--done': getSkillStatus(node.id) === 'active' }"
              :title="getSkillStatus(node.id) === 'active' ? 'Unmark completion' : 'Mark as completed'"
              @click.stop="toggleComplete(node.id)"
            >{{ getSkillStatus(node.id) === 'active' ? '◆' : '◇' }}</button>
            <!-- Recommended sparkle -->
            <span v-if="isRecommended(node.id)" class="ff-node-rec">★</span>
          </div>
        </div>

        <!-- Selected node detail overlay -->
        <Transition name="qp-detail">
          <div v-if="selectedNode" class="ff-detail">
            <div class="ff-detail-top">
              <span class="ff-detail-gem">{{ selectedNode.icon }}</span>
              <div class="ff-detail-title-area">
                <div class="ff-detail-name">{{ selectedNode.name }}</div>
                <div class="ff-detail-tagline">{{ selectedNode.tagline }}</div>
              </div>
              <button class="ff-detail-close" @click="selectedQuestId = null">✕</button>
            </div>

            <p class="ff-detail-desc">{{ selectedNode.description }}</p>

            <!-- Quest steps -->
            <div class="ff-detail-section">
              <div class="ff-detail-section-label">◆ Objectives</div>
              <div v-for="(step, i) in selectedNode.questSteps" :key="i" class="ff-step">
                <span class="ff-step-num">{{ ['Ⅰ','Ⅱ','Ⅲ','Ⅳ','Ⅴ'][i] || i + 1 }}</span>
                <span class="ff-step-text">{{ step.label }}</span>
                <button
                  v-if="step.target"
                  class="ff-step-go"
                  @click="$emit('navigate', step.target!)"
                >▸</button>
              </div>
            </div>

            <!-- Rewards -->
            <div v-if="selectedNode.rewards.length" class="ff-detail-section">
              <div class="ff-detail-section-label">◆ Rewards</div>
              <div class="ff-reward-list">
                <span
                  v-for="(reward, i) in selectedNode.rewards"
                  :key="i"
                  class="ff-reward"
                >{{ selectedNode.rewardIcons[i] || '🎁' }} {{ reward }}</span>
              </div>
            </div>

            <!-- Prerequisites -->
            <div v-if="selectedNode.requires.length" class="ff-detail-section">
              <div class="ff-detail-section-label">◆ Prerequisites</div>
              <div class="ff-prereq-list">
                <span
                  v-for="reqId in selectedNode.requires"
                  :key="reqId"
                  class="ff-prereq"
                  :class="{ 'ff-prereq--met': getSkillStatus(reqId) === 'active' }"
                >{{ getNodeIcon(reqId) }} {{ getNodeName(reqId) }} {{ getSkillStatus(reqId) === 'active' ? '◆' : '◇' }}</span>
              </div>
            </div>

            <!-- Actions -->
            <div class="ff-detail-actions">
              <button
                v-if="!isPinned(selectedNode.id)"
                class="ff-btn ff-btn--secondary"
                @click="skillTree.pinQuest(selectedNode!.id)"
              >📌 Pin</button>
              <button
                v-else
                class="ff-btn ff-btn--secondary"
                @click="skillTree.unpinQuest(selectedNode!.id)"
              >📌 Unpin</button>
              <button
                class="ff-btn ff-btn--primary"
                @click="startQuestChat(selectedNode!.id)"
              >⚔️ Begin Quest</button>
            </div>
          </div>
        </Transition>

        <!-- Empty state -->
        <div v-if="filteredNodes.length === 0" class="ff-empty">
          <span class="ff-empty-crystal">✦</span>
          <span class="ff-empty-text">All licenses in this tier obtained!</span>
        </div>

        <!-- Footer ornament -->
        <div class="ff-footer-ornament">━━━ ✦ ━━━</div>
      </div>
    </Transition>

    <!-- Quest Confirmation Dialog -->
    <QuestConfirmationDialog
      :visible="showConfirmDialog"
      :quest="questToConfirm"
      @accept="handleAcceptQuest"
      @cancel="showConfirmDialog = false"
    />

    <!-- Quest Reward Panel -->
    <QuestRewardPanel
      :visible="showRewardPanel"
      :quest="rewardPanelQuest"
      :show-choices="showRewardChoices"
      :choice-question="rewardChoiceQuestion"
      :choices="rewardChoices"
      @close="closeRewardPanel"
      @choice="handleRewardChoice"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';
import { useSkillTreeStore } from '../stores/skill-tree';
import { useBrainStore } from '../stores/brain';
import { useConversationStore } from '../stores/conversation';
import { useChatExpansion } from '../composables/useChatExpansion';
import type { SkillNode, SkillTier } from '../stores/skill-tree';
import QuestConfirmationDialog from './QuestConfirmationDialog.vue';
import QuestRewardPanel from './QuestRewardPanel.vue';
import type { RewardChoice } from './QuestRewardPanel.vue';

const emit = defineEmits<{
  navigate: [target: string];
  trigger: [];
}>();

interface QuestWithPriority {
  node: SkillNode;
  priority: number;
  isRecommended: boolean;
}

const skillTree = useSkillTreeStore();
const brain = useBrainStore();
const conversationStore = useConversationStore();
useChatExpansion();
const panelOpen = ref(false);
const screenWidth = ref(window.innerWidth);
const selectedQuestId = ref<string | null>(null);
const activeTab = ref<SkillTier>('foundation');
// Reward panel state
const showRewardPanel = ref(false);
const rewardPanelQuest = ref<SkillNode | null>(null);
const showRewardChoices = ref(false);
const rewardChoiceQuestion = ref('');
const rewardChoices = ref<RewardChoice[]>([]);
const showConfirmDialog = ref(false);
const questToConfirm = ref<SkillNode | null>(null);
const aiSortedQuests = ref<QuestWithPriority[]>([]);
const lastSortTime = ref(0);
const SORT_CACHE_DURATION = 30000;

const circumference = 2 * Math.PI * 22; // r=22

const tiers = [
  { id: 'foundation' as SkillTier, label: 'Foundation', icon: '🔮' },
  { id: 'advanced' as SkillTier, label: 'Advanced', icon: '⚔️' },
  { id: 'ultimate' as SkillTier, label: 'Ultimate', icon: '👑' },
];

const progressPercent = computed(() => skillTree.progressPercent);
const activeCount = computed(() => skillTree.activeCount);
const totalNodes = computed(() => skillTree.totalNodes);
const availableQuests = computed(() => {
  const pinned = new Set(skillTree.tracker.pinnedQuestIds);
  return skillTree.nodes.filter(n =>
    skillTree.getSkillStatus(n.id) === 'available' && !pinned.has(n.id),
  );
});

const filteredNodes = computed(() =>
  skillTree.nodes.filter(n => n.tier === activeTab.value),
);

// Dynamic positioning
const questHubPosition = computed(() => {
  const isMobile = screenWidth.value <= 640;
  const baseTop = isMobile ? 6 : 44;
  const baseRight = isMobile ? 52 : 16;
  return {
    top: `${baseTop}px`,
    right: `${baseRight}px`,
    position: 'fixed' as const,
    zIndex: '19',
  };
});

// AI-powered quest sorting with caching
const sortedAvailableQuests = computed(() => {
  const now = Date.now();
  if (aiSortedQuests.value.length > 0 && (now - lastSortTime.value) < SORT_CACHE_DURATION) {
    return aiSortedQuests.value;
  }
  return availableQuests.value.map((node, index) => ({
    node,
    priority: 0.5,
    isRecommended: index === 0,
  }));
});

const selectedNode = computed(() => {
  if (!selectedQuestId.value) return null;
  return skillTree.nodes.find(n => n.id === selectedQuestId.value) ?? null;
});

onMounted(() => {
  sortQuestsWithAI();
  window.addEventListener('resize', handleResize);
});

onUnmounted(() => {
  window.removeEventListener('resize', handleResize);
});

watch(
  () => conversationStore.messages.length,
  (newLength, oldLength) => {
    if (newLength > oldLength && showRewardPanel.value && !showRewardChoices.value) {
      setTimeout(() => { if (showRewardPanel.value) closeRewardPanel(); }, 100);
    }
  }
);

watch(
  () => conversationStore.isThinking,
  (isThinking) => {
    if (isThinking && showRewardChoices.value) closeRewardPanel();
  }
);

function handleResize() {
  screenWidth.value = window.innerWidth;
}

function isRecommended(nodeId: string): boolean {
  return sortedAvailableQuests.value.some(q => q.node.id === nodeId && q.isRecommended);
}

function hasParentInTier(node: SkillNode): boolean {
  return node.requires.some(reqId => {
    const parent = skillTree.nodes.find(n => n.id === reqId);
    return parent && parent.tier === node.tier;
  });
}

function tierBadge(tier: SkillTier): string {
  return tier === 'foundation' ? 'Ⅰ' : tier === 'advanced' ? 'Ⅱ' : 'Ⅲ';
}

function getNodeName(id: string): string {
  return skillTree.nodes.find(n => n.id === id)?.name ?? id;
}

function getNodeIcon(id: string): string {
  return skillTree.nodes.find(n => n.id === id)?.icon ?? '?';
}

function toggleComplete(id: string) {
  if (skillTree.getSkillStatus(id) === 'active') {
    skillTree.unmarkComplete(id);
  } else {
    skillTree.markComplete(id);
  }
}

async function sortQuestsWithAI() {
  if (!brain.hasBrain || availableQuests.value.length === 0) return;

  try {
    const questList = availableQuests.value.map(q => ({
      id: q.id, name: q.name, tagline: q.tagline,
      description: q.description, category: q.category || 'general',
      tier: q.tier || 'basic', rewards: q.rewards,
    }));

    const prompt = `Analyze these available quests and prioritize them based on what would benefit the user most right now. Consider factors like:
- Learning progression (foundational skills first)
- Current user needs and pain points
- Practical impact and immediate benefits
- Skill dependencies and natural flow

Available quests:
${questList.map(q => `- ${q.name}: ${q.description} (${q.category}, ${q.tier})`).join('\n')}

Respond with ONLY valid JSON:
{"priority_order":["quest_id_1","quest_id_2"],"top_recommendation":"quest_id_1"}`;

    const brainStore = useBrainStore();
    let responseText = '';
    if (brainStore.hasBrain) {
      try {
        const result = await brainStore.processPromptSilently?.(prompt);
        responseText = result || '';
      } catch { return; }
    } else { return; }

    if (!responseText) return;
    try {
      const match = responseText.match(/\{[\s\S]*\}/);
      if (match) {
        const result = JSON.parse(match[0]);
        const prioritizedQuests: QuestWithPriority[] = [];
        const priorityMap = new Map<string, number>();
        result.priority_order.forEach((id: string, index: number) => {
          priorityMap.set(id, 1 - (index / result.priority_order.length));
        });
        availableQuests.value.forEach(node => {
          const priority = priorityMap.get(node.id) ?? 0.1;
          const isRec = node.id === result.top_recommendation;
          prioritizedQuests.push({ node, priority, isRecommended: isRec });
        });
        prioritizedQuests.sort((a, b) => b.priority - a.priority);
        aiSortedQuests.value = prioritizedQuests;
        lastSortTime.value = Date.now();
      }
    } catch { /* parse fail */ }
  } catch { /* sort fail */ }
}

function getSkillStatus(id: string) {
  return skillTree.getSkillStatus(id);
}

function isPinned(id: string): boolean {
  return skillTree.tracker.pinnedQuestIds.includes(id);
}

function selectQuest(id: string) {
  const quest = skillTree.nodes.find(n => n.id === id);
  if (!quest) return;

  panelOpen.value = false;
  selectedQuestId.value = null;

  const parts: string[] = [];
  parts.push(`**${quest.name}** — ${quest.tagline}`);
  if (quest.description) parts.push(quest.description);
  if (quest.rewards?.length) parts.push(`**Rewards:** ${quest.rewards.join(', ')}`);
  if (quest.questSteps?.length) {
    const steps = quest.questSteps.slice(0, 3).map((s, i) => `${i + 1}. ${s.label}`).join('\n');
    parts.push(`**Steps:**\n${steps}`);
  }
  parts.push('Would you like to start this quest?');

  conversationStore.addMessage({
    id: crypto.randomUUID(),
    role: 'assistant',
    content: parts.join('\n\n'),
    agentName: 'TerranSoul',
    sentiment: 'happy',
    timestamp: Date.now(),
    questChoices: [
      { label: 'Yes, let\'s do this!', value: `auto-config:${quest.id}`, icon: '✅' },
      { label: 'Maybe later', value: 'dismiss', icon: '💭' },
    ],
    questId: quest.id,
  });

  emit('trigger');
}

function startQuestChat(questId: string) {
  const quest = skillTree.nodes.find(n => n.id === questId);
  if (quest) {
    questToConfirm.value = quest;
    showConfirmDialog.value = true;
  }
}

function handleAcceptQuest() {
  if (questToConfirm.value) {
    showConfirmDialog.value = false;
    panelOpen.value = false;
    selectedQuestId.value = null;
    skillTree.triggerQuestEvent(questToConfirm.value.id);
    emit('trigger');
    questToConfirm.value = null;
    setTimeout(() => sortQuestsWithAI(), 1000);
  }
}

function handleRewardChoice(choice: string) {
  const quest = rewardPanelQuest.value;
  if (!quest) return;
  if (choice === 'accept') {
    closeRewardPanel();
    skillTree.triggerQuestEvent(quest.id);
    emit('trigger');
    setTimeout(() => sortQuestsWithAI(), 1000);
  } else {
    closeRewardPanel();
  }
}

function closeRewardPanel() {
  showRewardPanel.value = false;
  showRewardChoices.value = false;
  rewardPanelQuest.value = null;
  rewardChoiceQuestion.value = '';
  rewardChoices.value = [];
}
</script>

<style scoped>
/* ═══════════════════════════════════════════════════════════════════════════
   Final Fantasy "License Board" Skill Tree — Crystal / Gold / Dark Blue
   ═══════════════════════════════════════════════════════════════════════════ */

/* ── Floating Crystal Orb ── */
.ff-orb {
  width: 56px;
  height: 56px;
  border-radius: 50%;
  border: 2px solid rgba(180, 160, 100, 0.5);
  background: radial-gradient(ellipse at 40% 35%, rgba(30, 28, 50, 0.95), rgba(10, 8, 25, 0.98));
  backdrop-filter: blur(12px);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  position: relative;
  box-shadow:
    0 0 16px rgba(180, 160, 100, 0.15),
    inset 0 0 20px rgba(100, 140, 220, 0.08);
  transition: transform 0.2s ease, box-shadow 0.25s ease;
  flex-direction: column;
  gap: 0;
}
.ff-orb:hover {
  transform: scale(1.1);
  box-shadow:
    0 0 24px rgba(180, 160, 100, 0.3),
    inset 0 0 24px rgba(100, 140, 220, 0.12);
}
.ff-orb:active { transform: scale(0.95); }
.ff-orb--open {
  border-color: rgba(220, 195, 110, 0.7);
  box-shadow:
    0 0 28px rgba(220, 195, 110, 0.35),
    inset 0 0 20px rgba(100, 140, 220, 0.15);
}

.ff-orb-ring {
  position: absolute;
  width: 100%;
  height: 100%;
  transform: rotate(-90deg);
}
.ff-orb-ring-bg {
  fill: none;
  stroke: rgba(180, 160, 100, 0.12);
  stroke-width: 2.5;
}
.ff-orb-ring-fill {
  fill: none;
  stroke: #dcc36e;
  stroke-width: 2.5;
  stroke-linecap: round;
  transition: stroke-dashoffset 0.8s ease;
  filter: drop-shadow(0 0 3px rgba(220, 195, 110, 0.4));
}

.ff-orb-crystal {
  font-size: 1rem;
  color: #8ec8f6;
  z-index: 1;
  text-shadow: 0 0 8px rgba(142, 200, 246, 0.5);
  animation: crystal-pulse 3s ease-in-out infinite;
}
@keyframes crystal-pulse {
  0%, 100% { opacity: 0.8; transform: scale(1); }
  50% { opacity: 1; transform: scale(1.08); }
}

.ff-orb-pct {
  font-size: 0.58rem;
  font-weight: 700;
  color: #dcc36e;
  z-index: 1;
  letter-spacing: 0.04em;
  margin-top: -2px;
}

/* ── Panel ── */
.ff-panel {
  position: absolute;
  top: 64px;
  right: 0;
  width: 360px;
  max-height: 540px;
  overflow-y: auto;
  background:
    linear-gradient(170deg, rgba(16, 14, 36, 0.98) 0%, rgba(8, 6, 22, 0.99) 100%);
  border: 1px solid rgba(180, 160, 100, 0.25);
  border-radius: 4px;
  box-shadow:
    0 8px 48px rgba(0, 0, 0, 0.6),
    inset 0 1px 0 rgba(180, 160, 100, 0.1);
  padding: 0;
  scrollbar-width: thin;
  scrollbar-color: rgba(180, 160, 100, 0.25) transparent;
}

/* ── Header ── */
.ff-header {
  text-align: center;
  padding: 14px 16px 8px;
}
.ff-header-ornament {
  font-size: 0.65rem;
  color: rgba(180, 160, 100, 0.4);
  letter-spacing: 0.3em;
  margin-bottom: 4px;
}
.ff-header-title {
  font-size: 1rem;
  font-weight: 700;
  color: #dcc36e;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  text-shadow: 0 0 12px rgba(220, 195, 110, 0.25);
}
.ff-header-sub {
  font-size: 0.68rem;
  color: rgba(180, 180, 200, 0.55);
  margin-top: 2px;
  letter-spacing: 0.06em;
}

/* ── XP Bar ── */
.ff-xp-bar {
  height: 6px;
  margin: 6px 16px 10px;
  border-radius: 3px;
  background: rgba(180, 160, 100, 0.08);
  border: 1px solid rgba(180, 160, 100, 0.12);
  position: relative;
  overflow: hidden;
}
.ff-xp-fill {
  height: 100%;
  border-radius: 2px;
  background: linear-gradient(90deg, #b8860b, #dcc36e, #f0e68c);
  transition: width 0.8s ease;
  box-shadow: 0 0 6px rgba(220, 195, 110, 0.3);
}
.ff-xp-text {
  position: absolute;
  right: 4px;
  top: -1px;
  font-size: 0.5rem;
  font-weight: 700;
  color: rgba(220, 195, 110, 0.7);
  line-height: 6px;
}

/* ── Tier Tabs ── */
.ff-tabs {
  display: flex;
  padding: 0 12px;
  gap: 4px;
  border-bottom: 1px solid rgba(180, 160, 100, 0.1);
}
.ff-tab {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
  padding: 8px 4px;
  border: none;
  background: transparent;
  color: rgba(180, 180, 200, 0.45);
  font-size: 0.72rem;
  font-weight: 600;
  cursor: pointer;
  border-bottom: 2px solid transparent;
  transition: all 0.2s ease;
  letter-spacing: 0.04em;
}
.ff-tab:hover {
  color: rgba(220, 195, 110, 0.7);
}
.ff-tab--active {
  color: #dcc36e;
  border-bottom-color: #dcc36e;
  text-shadow: 0 0 8px rgba(220, 195, 110, 0.2);
}
.ff-tab-icon { font-size: 0.85rem; }
.ff-tab-label { text-transform: uppercase; }

/* ── Skill Node Grid ── */
.ff-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 6px;
  padding: 12px;
}

.ff-node {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 10px 4px 8px;
  border-radius: 4px;
  border: 1px solid rgba(180, 160, 100, 0.08);
  background: rgba(20, 18, 40, 0.5);
  cursor: pointer;
  transition: all 0.2s ease;
}
.ff-node:hover {
  border-color: rgba(180, 160, 100, 0.25);
  background: rgba(30, 28, 55, 0.7);
}

/* ── Node States ── */
.ff-node--locked {
  opacity: 0.35;
  filter: saturate(0.3);
}
.ff-node--locked .ff-node-gem {
  border-color: rgba(80, 80, 100, 0.3);
}

.ff-node--available {
  border-color: rgba(180, 160, 100, 0.15);
}
.ff-node--available .ff-node-gem {
  border-color: rgba(220, 195, 110, 0.5);
  animation: node-breathe 2.5s ease-in-out infinite;
}
@keyframes node-breathe {
  0%, 100% { box-shadow: 0 0 4px rgba(220, 195, 110, 0.15); }
  50% { box-shadow: 0 0 12px rgba(220, 195, 110, 0.3); }
}

.ff-node--active {
  border-color: rgba(142, 200, 246, 0.2);
  background: rgba(20, 30, 55, 0.6);
}
.ff-node--active .ff-node-gem {
  border-color: rgba(142, 200, 246, 0.6);
  box-shadow: 0 0 12px rgba(142, 200, 246, 0.2);
}

.ff-node--recommended {
  border-color: rgba(220, 195, 110, 0.3);
  background:
    linear-gradient(135deg, rgba(220, 195, 110, 0.04) 0%, rgba(20, 18, 40, 0.5) 100%);
}

/* ── Node Gem (icon container) ── */
.ff-node-gem {
  width: 40px;
  height: 40px;
  border-radius: 50%;
  border: 2px solid rgba(100, 100, 130, 0.25);
  background: radial-gradient(circle at 40% 35%, rgba(40, 38, 65, 0.9), rgba(15, 13, 30, 0.95));
  display: flex;
  align-items: center;
  justify-content: center;
  position: relative;
  transition: all 0.25s ease;
}
.ff-node-icon { font-size: 1.1rem; z-index: 1; }
.ff-node-glow {
  position: absolute;
  inset: -3px;
  border-radius: 50%;
  background: radial-gradient(circle, rgba(142, 200, 246, 0.15) 0%, transparent 70%);
  animation: glow-rotate 4s linear infinite;
}
@keyframes glow-rotate {
  0% { transform: rotate(0deg); }
  100% { transform: rotate(360deg); }
}

.ff-node-label {
  font-size: 0.62rem;
  font-weight: 600;
  color: rgba(200, 200, 220, 0.8);
  text-align: center;
  line-height: 1.2;
  max-width: 90px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.ff-node--active .ff-node-label { color: #8ec8f6; }

.ff-node-tier-badge {
  position: absolute;
  top: 2px;
  left: 4px;
  font-size: 0.5rem;
  color: rgba(180, 160, 100, 0.35);
  font-weight: 700;
}

/* ── Node Completion Check ── */
.ff-node-check {
  position: absolute;
  top: 2px;
  right: 4px;
  background: none;
  border: none;
  font-size: 0.65rem;
  color: rgba(180, 160, 100, 0.4);
  cursor: pointer;
  padding: 2px;
  transition: all 0.15s ease;
  line-height: 1;
}
.ff-node-check:hover {
  color: #dcc36e;
  transform: scale(1.2);
}
.ff-node-check--done {
  color: #8ec8f6;
  text-shadow: 0 0 6px rgba(142, 200, 246, 0.4);
}

.ff-node-rec {
  position: absolute;
  bottom: 2px;
  right: 4px;
  font-size: 0.55rem;
  color: #dcc36e;
  animation: sparkle 2s ease-in-out infinite alternate;
}
@keyframes sparkle {
  0% { opacity: 0.6; }
  100% { opacity: 1; }
}

/* ── Connector Lines ── */
.ff-node-connector {
  position: absolute;
  top: -6px;
  width: 1px;
  height: 6px;
  background: rgba(180, 160, 100, 0.2);
}

/* ── Detail Overlay ── */
.ff-detail {
  border-top: 1px solid rgba(180, 160, 100, 0.12);
  padding: 14px 16px;
  background: rgba(12, 10, 28, 0.5);
}
.ff-detail-top {
  display: flex;
  align-items: flex-start;
  gap: 10px;
}
.ff-detail-gem {
  font-size: 1.5rem;
  flex-shrink: 0;
}
.ff-detail-title-area { flex: 1; }
.ff-detail-name {
  font-size: 0.88rem;
  font-weight: 700;
  color: #dcc36e;
  letter-spacing: 0.04em;
}
.ff-detail-tagline {
  font-size: 0.7rem;
  color: rgba(180, 180, 200, 0.5);
  margin-top: 1px;
}
.ff-detail-close {
  background: none;
  border: 1px solid rgba(180, 160, 100, 0.15);
  color: rgba(180, 180, 200, 0.4);
  cursor: pointer;
  font-size: 0.75rem;
  padding: 2px 6px;
  border-radius: 2px;
  transition: all 0.15s ease;
}
.ff-detail-close:hover {
  border-color: rgba(180, 160, 100, 0.4);
  color: #dcc36e;
}

.ff-detail-desc {
  font-size: 0.75rem;
  color: rgba(200, 200, 220, 0.65);
  line-height: 1.5;
  margin: 10px 0;
}

/* ── Detail Sections ── */
.ff-detail-section { margin-top: 10px; }
.ff-detail-section-label {
  font-size: 0.65rem;
  font-weight: 700;
  color: #dcc36e;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  margin-bottom: 6px;
}

/* Steps */
.ff-step {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 3px 0;
}
.ff-step-num {
  width: 18px;
  font-size: 0.65rem;
  font-weight: 700;
  color: rgba(180, 160, 100, 0.5);
  text-align: center;
  flex-shrink: 0;
}
.ff-step-text {
  font-size: 0.73rem;
  color: rgba(200, 200, 220, 0.7);
  flex: 1;
}
.ff-step-go {
  font-size: 0.7rem;
  color: #dcc36e;
  background: none;
  border: 1px solid rgba(180, 160, 100, 0.2);
  border-radius: 2px;
  padding: 1px 8px;
  cursor: pointer;
  transition: all 0.15s ease;
}
.ff-step-go:hover {
  background: rgba(180, 160, 100, 0.08);
  border-color: rgba(180, 160, 100, 0.4);
}

/* Rewards */
.ff-reward-list { display: flex; flex-wrap: wrap; gap: 4px; }
.ff-reward {
  font-size: 0.68rem;
  padding: 3px 8px;
  border-radius: 2px;
  background: rgba(180, 160, 100, 0.06);
  color: #dcc36e;
  border: 1px solid rgba(180, 160, 100, 0.12);
}

/* Prerequisites */
.ff-prereq-list { display: flex; flex-wrap: wrap; gap: 4px; }
.ff-prereq {
  font-size: 0.68rem;
  padding: 3px 8px;
  border-radius: 2px;
  color: rgba(200, 200, 220, 0.5);
  border: 1px solid rgba(100, 100, 130, 0.15);
}
.ff-prereq--met {
  color: #8ec8f6;
  border-color: rgba(142, 200, 246, 0.2);
}

/* Actions */
.ff-detail-actions {
  display: flex;
  gap: 6px;
  margin-top: 12px;
}
.ff-btn {
  flex: 1;
  padding: 7px 0;
  font-size: 0.72rem;
  font-weight: 700;
  cursor: pointer;
  border-radius: 2px;
  letter-spacing: 0.04em;
  transition: all 0.15s ease;
  text-transform: uppercase;
}
.ff-btn--secondary {
  background: transparent;
  border: 1px solid rgba(180, 160, 100, 0.2);
  color: rgba(200, 200, 220, 0.6);
}
.ff-btn--secondary:hover {
  border-color: rgba(180, 160, 100, 0.4);
  color: #dcc36e;
}
.ff-btn--primary {
  background: linear-gradient(180deg, rgba(180, 160, 100, 0.15) 0%, rgba(180, 160, 100, 0.06) 100%);
  border: 1px solid rgba(220, 195, 110, 0.35);
  color: #dcc36e;
  text-shadow: 0 0 8px rgba(220, 195, 110, 0.2);
}
.ff-btn--primary:hover {
  background: linear-gradient(180deg, rgba(180, 160, 100, 0.22) 0%, rgba(180, 160, 100, 0.1) 100%);
  border-color: rgba(220, 195, 110, 0.5);
  box-shadow: 0 0 12px rgba(220, 195, 110, 0.1);
}

/* ── Empty State ── */
.ff-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 32px 16px;
}
.ff-empty-crystal {
  font-size: 1.5rem;
  color: #8ec8f6;
  text-shadow: 0 0 12px rgba(142, 200, 246, 0.3);
}
.ff-empty-text {
  font-size: 0.75rem;
  color: rgba(180, 180, 200, 0.4);
  letter-spacing: 0.06em;
}

/* ── Footer Ornament ── */
.ff-footer-ornament {
  text-align: center;
  font-size: 0.6rem;
  color: rgba(180, 160, 100, 0.2);
  letter-spacing: 0.3em;
  padding: 8px 0 12px;
}

/* ── Transitions ── */
.quest-panel-enter-active { animation: panel-slide-in 0.3s ease-out; }
.quest-panel-leave-active { animation: panel-slide-in 0.2s ease-in reverse; }
@keyframes panel-slide-in {
  0% { opacity: 0; transform: translateY(-8px) scale(0.97); }
  100% { opacity: 1; transform: translateY(0) scale(1); }
}

.qp-detail-enter-active { animation: detail-fade 0.25s ease-out; }
.qp-detail-leave-active { animation: detail-fade 0.15s ease-in reverse; }
@keyframes detail-fade {
  0% { opacity: 0; transform: translateY(4px); }
  100% { opacity: 1; transform: translateY(0); }
}

/* ── Mobile ── */
@media (max-width: 640px) {
  .ff-orb { width: 48px; height: 48px; }
  .ff-orb-crystal { font-size: 0.85rem; }
  .ff-orb-pct { font-size: 0.52rem; }
  .ff-panel {
    width: calc(100vw - 24px);
    right: 0;
    top: 56px;
    max-height: 440px;
  }
  .ff-grid {
    grid-template-columns: repeat(2, 1fr);
    gap: 4px;
    padding: 8px;
  }
}
</style>
