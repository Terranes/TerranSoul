<template>
  <div class="quest-hub" :style="questHubPosition">
    <!-- Floating progress bubble -->
    <button
      class="quest-bubble"
      :class="{ 'quest-bubble-active': panelOpen }"
      :title="`Skill Progress: ${progressPercent}%`"
      @click="panelOpen = !panelOpen"
    >
      <svg class="quest-ring" viewBox="0 0 48 48">
        <circle class="quest-ring-bg" cx="24" cy="24" r="20" />
        <circle
          class="quest-ring-fill"
          cx="24" cy="24" r="20"
          :stroke-dasharray="circumference"
          :stroke-dashoffset="circumference - (circumference * progressPercent / 100)"
        />
      </svg>
      <span class="quest-bubble-pct">{{ progressPercent }}%</span>
    </button>

    <!-- Quest panel dropdown -->
    <Transition name="quest-panel">
      <div v-if="panelOpen" class="quest-panel" @click.stop>
        <!-- Header -->
        <div class="qp-header">
          <span class="qp-title">⚔️ Quest Tracker</span>
          <span class="qp-stats">{{ activeCount }}/{{ totalNodes }} unlocked</span>
        </div>

        <!-- Progress bar -->
        <div class="qp-progress-track">
          <div class="qp-progress-fill" :style="{ width: progressPercent + '%' }" />
        </div>

        <!-- Pinned quests -->
        <div v-if="pinnedQuests.length > 0" class="qp-section">
          <div class="qp-section-label">📌 Pinned</div>
          <div
            v-for="node in pinnedQuests"
            :key="node.id"
            class="qp-quest-row"
            :class="'qp-status-' + getSkillStatus(node.id)"
            @click="selectQuest(node.id)"
          >
            <span class="qp-quest-icon">{{ node.icon }}</span>
            <div class="qp-quest-info">
              <span class="qp-quest-name">{{ node.name }}</span>
              <span class="qp-quest-tagline">{{ node.tagline }}</span>
            </div>
            <span class="qp-quest-status">{{ statusIcon(node.id) }}</span>
          </div>
        </div>

        <!-- Available quests -->
        <div v-if="sortedAvailableQuests.length > 0" class="qp-section">
          <div class="qp-section-label">🟡 Available</div>
          <div
            v-for="questData in sortedAvailableQuests"
            :key="questData.node.id"
            class="qp-quest-row qp-status-available"
            :class="{ 'qp-quest-recommended': questData.isRecommended }"
            :data-quest-id="questData.node.id"
            @click="selectQuest(questData.node.id)"
          >
            <span class="qp-quest-icon">{{ questData.node.icon }}</span>
            <div class="qp-quest-info">
              <span class="qp-quest-name">
                {{ questData.node.name }}
                <span v-if="questData.isRecommended" class="qp-rec-badge" title="AI Recommended">✨</span>
              </span>
              <span class="qp-quest-tagline">{{ questData.node.tagline }}</span>
            </div>
            <span class="qp-quest-status">!</span>
          </div>
        </div>

        <!-- Selected quest detail -->
        <Transition name="qp-detail">
          <div v-if="selectedNode" class="qp-detail">
            <div class="qp-detail-header">
              <span class="qp-detail-icon">{{ selectedNode.icon }}</span>
              <div>
                <div class="qp-detail-name">{{ selectedNode.name }}</div>
                <div class="qp-detail-tagline">{{ selectedNode.tagline }}</div>
              </div>
              <button class="qp-detail-close" @click="selectedQuestId = null" title="Close">✕</button>
            </div>
            <p class="qp-detail-desc">{{ selectedNode.description }}</p>

            <!-- Steps / TODO -->
            <div class="qp-detail-steps">
              <div class="qp-section-label">📋 Steps</div>
              <div v-for="(step, i) in selectedNode.questSteps" :key="i" class="qp-step">
                <span class="qp-step-num">{{ i + 1 }}</span>
                <span class="qp-step-label">{{ step.label }}</span>
                <button
                  v-if="step.target"
                  class="qp-step-go"
                  @click="$emit('navigate', step.target!)"
                >Go →</button>
              </div>
            </div>

            <!-- Rewards -->
            <div v-if="selectedNode.rewards.length" class="qp-detail-rewards">
              <div class="qp-section-label">🎁 Rewards</div>
              <div class="qp-reward-grid">
                <span
                  v-for="(reward, i) in selectedNode.rewards"
                  :key="i"
                  class="qp-reward-chip"
                >{{ selectedNode.rewardIcons[i] || '🎁' }} {{ reward }}</span>
              </div>
            </div>

            <!-- Actions -->
            <div class="qp-detail-actions">
              <button
                v-if="!isPinned(selectedNode.id)"
                class="qp-action-btn qp-pin"
                @click="skillTree.pinQuest(selectedNode!.id)"
              >📌 Pin</button>
              <button
                v-else
                class="qp-action-btn qp-unpin"
                @click="skillTree.unpinQuest(selectedNode!.id)"
              >📌 Unpin</button>
              <button
                class="qp-action-btn qp-chat"
                @click="startQuestChat(selectedNode!.id)"
              >💬 Ask Guide</button>
            </div>
          </div>
        </Transition>

        <!-- Empty state -->
        <div v-if="pinnedQuests.length === 0 && availableQuests.length === 0" class="qp-empty">
          <span class="qp-empty-icon">🏆</span>
          <span class="qp-empty-text">All quests completed!</span>
        </div>
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
import type { SkillNode } from '../stores/skill-tree';
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
const { isChatExpanded } = useChatExpansion();
const panelOpen = ref(false);
const screenWidth = ref(window.innerWidth);
const selectedQuestId = ref<string | null>(null);
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
const SORT_CACHE_DURATION = 30000; // Cache AI sorting for 30 seconds

const circumference = 2 * Math.PI * 20; // r=20

const progressPercent = computed(() => skillTree.progressPercent);
const activeCount = computed(() => skillTree.activeCount);
const totalNodes = computed(() => skillTree.totalNodes);
const pinnedQuests = computed(() => skillTree.pinnedQuests);

const availableQuests = computed(() => {
  const pinned = new Set(skillTree.tracker.pinnedQuestIds);
  return skillTree.nodes.filter(n =>
    skillTree.getSkillStatus(n.id) === 'available' && !pinned.has(n.id),
  );
});

// Dynamic positioning based on chat expansion state
const questHubPosition = computed(() => {
  const isMobile = screenWidth.value <= 640;
  const baseBottom = isMobile ? 85 : 90; // Different base positions for mobile vs desktop
  const baseRight = isMobile ? 16 : 24; // Different right positions for mobile vs desktop
  const expandedOffset = isMobile ? 200 : 250; // Less offset needed on mobile
  
  return {
    bottom: isChatExpanded.value ? `${baseBottom + expandedOffset}px` : `${baseBottom}px`,
    right: `${baseRight}px`,
    position: 'fixed',
    'z-index': '150',
    transition: 'bottom 0.35s cubic-bezier(0.4, 0, 0.2, 1)'  // Smooth animation
  };
});

// AI-powered quest sorting with caching
const sortedAvailableQuests = computed(() => {
  const now = Date.now();
  
  // If we have fresh AI-sorted data, use it
  if (aiSortedQuests.value.length > 0 && (now - lastSortTime.value) < SORT_CACHE_DURATION) {
    return aiSortedQuests.value;
  }
  
  // Fallback to basic sorting while AI processes
  return availableQuests.value.map((node, index) => ({
    node,
    priority: 0.5,
    isRecommended: index === 0, // Mark first as recommended as fallback
  }));
});

const selectedNode = computed(() => {
  if (!selectedQuestId.value) return null;
  return skillTree.nodes.find(n => n.id === selectedQuestId.value) ?? null;
});

// Trigger AI sorting when component mounts or when quests change
onMounted(() => {
  sortQuestsWithAI();
  // Add resize listener to update positioning on screen size changes
  window.addEventListener('resize', handleResize);
});

onUnmounted(() => {
  // Clean up resize listener
  window.removeEventListener('resize', handleResize);
});

// Watch for new messages to auto-dismiss reward panel
watch(
  () => conversationStore.messages.length,
  (newLength, oldLength) => {
    // If a new message was added and reward panel is showing choices, dismiss it
    if (newLength > oldLength && showRewardPanel.value && !showRewardChoices.value) {
      // Only dismiss if it's been showing for at least 1 second (to avoid dismissing immediately after opening)
      setTimeout(() => {
        if (showRewardPanel.value) {
          closeRewardPanel();
        }
      }, 100);
    }
  }
);

// Watch for user typing to dismiss reward panel choices
watch(
  () => conversationStore.isThinking,
  (isThinking) => {
    // If user started typing (or AI is thinking after user sent message), dismiss choices
    if (isThinking && showRewardChoices.value) {
      closeRewardPanel();
    }
  }
);

function handleResize() {
  screenWidth.value = window.innerWidth;
}

async function sortQuestsWithAI() {
  if (!brain.hasBrain || availableQuests.value.length === 0) {
    return;
  }

  try {
    const questList = availableQuests.value.map(q => ({
      id: q.id,
      name: q.name,
      tagline: q.tagline,
      description: q.description,
      category: q.category || 'general',
      tier: q.tier || 'basic',
      rewards: q.rewards,
    }));

    const prompt = `Analyze these available quests and prioritize them based on what would benefit the user most right now. Consider factors like:

- Learning progression (foundational skills first)
- Current user needs and pain points
- Practical impact and immediate benefits
- Skill dependencies and natural flow
- Engagement and motivation potential

Available quests:
${questList.map(q => `- ${q.name}: ${q.description} (${q.category}, ${q.tier})`).join('\n')}

Respond with ONLY a JSON array of quest IDs in priority order (highest priority first), followed by the most recommended quest ID. Format:
{
  "priority_order": ["quest_id_1", "quest_id_2", ...],
  "top_recommendation": "quest_id_1"
}`;

    // Use brain directly for background quest analysis without showing in chat
    const brain = useBrainStore();
    let responseText = '';
    
    if (brain.hasBrain) {
      try {
        // Use brain's internal messaging without adding to conversation
        const result = await brain.processPromptSilently?.(prompt);
        responseText = result || '';
      } catch (error) {
        console.warn('Silent brain processing failed:', error);
        return;
      }
    } else {
      return; // No point in quest sorting without AI
    }
    
    // Process the response without it appearing in chat
    if (!responseText) {
      console.warn('No response from brain for quest analysis');
      return;
    }

    if (!responseText) return;
    try {
      const match = responseText.match(/\{[\s\S]*\}/);
      if (match) {
        const result = JSON.parse(match[0]);
        const prioritizedQuests: QuestWithPriority[] = [];
        
        // Create priority map
        const priorityMap = new Map<string, number>();
        result.priority_order.forEach((id: string, index: number) => {
          priorityMap.set(id, 1 - (index / result.priority_order.length));
        });
        
        // Build sorted quest list
        availableQuests.value.forEach(node => {
          const priority = priorityMap.get(node.id) ?? 0.1;
          const isRecommended = node.id === result.top_recommendation;
          prioritizedQuests.push({ node, priority, isRecommended });
        });
        
        // Sort by priority (highest first)
        prioritizedQuests.sort((a, b) => b.priority - a.priority);
        
        aiSortedQuests.value = prioritizedQuests;
        lastSortTime.value = Date.now();
      }
    } catch (parseError) {
      console.warn('Failed to parse AI quest sorting response:', parseError);
    }
  } catch (error) {
    console.warn('AI quest sorting failed, using fallback:', error);
  }
}

function getSkillStatus(id: string) {
  return skillTree.getSkillStatus(id);
}

function statusIcon(id: string): string {
  const s = skillTree.getSkillStatus(id);
  return s === 'active' ? '✅' : s === 'available' ? '🟡' : '🔒';
}

function isPinned(id: string): boolean {
  return skillTree.tracker.pinnedQuestIds.includes(id);
}

function selectQuest(id: string) {
  const quest = skillTree.nodes.find(n => n.id === id);
  if (!quest) return;
  
  // Close quest panel
  panelOpen.value = false;
  selectedQuestId.value = null;
  
  // Show reward panel
  rewardPanelQuest.value = quest;
  showRewardPanel.value = true;
  showRewardChoices.value = false;
  
  // Start AI explanation after a brief delay
  setTimeout(() => {
    startQuestExplanation(quest);
  }, 300);
}

function startQuestChat(questId: string) {
  // Show confirmation dialog instead of immediately triggering
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
    
    // Trigger quest event in chat for this specific quest - AI will handle everything
    skillTree.triggerQuestEvent(questToConfirm.value.id);
    emit('trigger');
    
    questToConfirm.value = null;
    
    // Re-sort quests after accepting one
    setTimeout(() => sortQuestsWithAI(), 1000);
  }
}

async function startQuestExplanation(quest: SkillNode) {
  if (!brain.hasBrain) {
    // Fallback for when no AI is available
    setTimeout(() => showQuestChoices(quest), 500);
    return;
  }

  try {
    // Build a comprehensive quest description
    const questDetails = [];
    if (quest.description) questDetails.push(`Description: ${quest.description}`);
    if (quest.rewards?.length) questDetails.push(`Rewards: ${quest.rewards.join(', ')}`);
    if (quest.questSteps?.length) questDetails.push(`Key steps: ${quest.questSteps.slice(0, 3).map(s => s.label).join(', ')}`);
    if (quest.difficulty) questDetails.push(`Difficulty: ${quest.difficulty}`);
    
    const prompt = `A user clicked on the quest "${quest.name}" with tagline "${quest.tagline}". 

${questDetails.join('\n')}

Please explain this quest in an engaging way (2-3 sentences max), focusing on:
- What they'll learn or achieve
- Why it's valuable for their journey
- How it fits their current progress

Keep it concise and motivating. Then naturally ask if they'd like to start this quest.`;

    // Start the AI explanation in chat
    emit('trigger'); // This will focus the chat
    await conversationStore.sendMessage(prompt);
    
    // Show choices after a reasonable delay for reading
    setTimeout(() => {
      if (showRewardPanel.value && rewardPanelQuest.value?.id === quest.id) {
        showQuestChoices(quest);
      }
    }, 2500);
    
  } catch (error) {
    console.warn('Quest explanation failed:', error);
    // Fallback to choices without explanation
    setTimeout(() => showQuestChoices(quest), 500);
  }
}

function showQuestChoices(quest: SkillNode) {
  rewardChoiceQuestion.value = `Ready to start "${quest.name}"?`;
  rewardChoices.value = [
    { label: '✅ Yes, let\'s do this!', value: 'accept', primary: true },
    { label: '💭 Maybe later', value: 'decline', primary: false }
  ];
  showRewardChoices.value = true;
}

function handleRewardChoice(choice: string) {
  const quest = rewardPanelQuest.value;
  if (!quest) return;
  
  if (choice === 'accept') {
    // Close reward panel and start quest
    closeRewardPanel();
    skillTree.triggerQuestEvent(quest.id);
    emit('trigger');
    // Re-sort quests after accepting one
    setTimeout(() => sortQuestsWithAI(), 1000);
  } else {
    // Just close the panel
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
.quest-hub {
  /* Position properties are now handled by dynamic :style binding */
}

/* ── Progress Bubble ── */
.quest-bubble {
  width: 56px;
  height: 56px;
  border-radius: 50%;
  border: none;
  background: rgba(15, 20, 35, 0.85);
  backdrop-filter: blur(10px);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  position: relative;
  box-shadow: 0 4px 20px rgba(124, 111, 255, 0.25);
  transition: transform 0.2s ease, box-shadow 0.2s ease;
}
.quest-bubble:hover {
  transform: scale(1.08);
  box-shadow: 0 6px 28px rgba(124, 111, 255, 0.4);
}
.quest-bubble:active { transform: scale(0.95); }
.quest-bubble-active {
  box-shadow: 0 6px 28px rgba(124, 111, 255, 0.5);
}

.quest-ring {
  position: absolute;
  width: 100%;
  height: 100%;
  transform: rotate(-90deg);
}
.quest-ring-bg {
  fill: none;
  stroke: rgba(124, 111, 255, 0.15);
  stroke-width: 3;
}
.quest-ring-fill {
  fill: none;
  stroke: var(--ts-accent, #7c6fff);
  stroke-width: 3;
  stroke-linecap: round;
  transition: stroke-dashoffset 0.6s ease;
}
.quest-bubble-pct {
  font-size: 0.72rem;
  font-weight: 700;
  color: var(--ts-accent, #7c6fff);
  z-index: 1;
  text-shadow: 0 1px 3px rgba(0, 0, 0, 0.5);
}

/* ── Quest Panel ── */
.quest-panel {
  position: absolute;
  bottom: 64px;
  right: 0;
  width: 320px;
  max-height: 480px;
  overflow-y: auto;
  background: rgba(15, 20, 35, 0.95);
  backdrop-filter: blur(16px);
  border: 1px solid rgba(124, 111, 255, 0.2);
  border-radius: var(--ts-radius-lg, 12px);
  box-shadow: 0 8px 40px rgba(0, 0, 0, 0.5);
  padding: 0;
  scrollbar-width: thin;
  scrollbar-color: rgba(124, 111, 255, 0.3) transparent;
}

.qp-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 14px 8px;
}
.qp-title { font-weight: 700; font-size: 0.9rem; color: var(--ts-text-primary, #eaecf4); }
.qp-stats { font-size: 0.72rem; color: var(--ts-text-muted, #6b7280); }

.qp-progress-track {
  height: 4px;
  margin: 0 14px 8px;
  border-radius: 2px;
  background: rgba(124, 111, 255, 0.12);
  overflow: hidden;
}
.qp-progress-fill {
  height: 100%;
  border-radius: 2px;
  background: linear-gradient(90deg, var(--ts-accent, #7c6fff), var(--ts-accent-violet, #a78bfa));
  transition: width 0.6s ease;
}

/* ── Sections ── */
.qp-section { padding: 4px 14px 8px; }
.qp-section-label {
  font-size: 0.7rem;
  font-weight: 600;
  color: var(--ts-text-dim, #4b5563);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin-bottom: 4px;
}

.qp-quest-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border-radius: var(--ts-radius-sm, 6px);
  cursor: pointer;
  transition: background 0.15s ease;
}
.qp-quest-row:hover { background: rgba(124, 111, 255, 0.08); }
.qp-quest-recommended { 
  background: linear-gradient(90deg, rgba(255, 215, 0, 0.08) 0%, rgba(124, 111, 255, 0.05) 100%);
  border: 1px solid rgba(255, 215, 0, 0.15);
  border-radius: var(--ts-radius-sm, 6px);
}
.qp-quest-recommended:hover { 
  background: linear-gradient(90deg, rgba(255, 215, 0, 0.12) 0%, rgba(124, 111, 255, 0.08) 100%);
  border-color: rgba(255, 215, 0, 0.25);
}
.qp-quest-icon { font-size: 1.1rem; flex-shrink: 0; }
.qp-quest-info { display: flex; flex-direction: column; flex: 1; min-width: 0; }
.qp-quest-name { 
  font-size: 0.82rem; 
  font-weight: 600; 
  color: var(--ts-text-primary, #eaecf4);
  display: flex;
  align-items: center;
  gap: 6px;
}
.qp-rec-badge {
  font-size: 0.75rem;
  background: linear-gradient(135deg, #ffd700, #ffb700);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
  animation: sparkle 2s ease-in-out infinite alternate;
}
@keyframes sparkle {
  0% { opacity: 0.8; transform: scale(1); }
  100% { opacity: 1; transform: scale(1.1); }
}
.qp-quest-tagline { font-size: 0.7rem; color: var(--ts-text-muted, #6b7280); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.qp-quest-status { font-size: 0.8rem; flex-shrink: 0; }
.qp-status-available .qp-quest-status { color: var(--ts-warning, #fbbf24); font-weight: 700; }
.qp-status-active .qp-quest-status { color: var(--ts-success, #22c55e); }
.qp-status-locked { opacity: 0.45; }

/* ── Detail pane ── */
.qp-detail {
  border-top: 1px solid rgba(124, 111, 255, 0.12);
  padding: 12px 14px;
}
.qp-detail-header {
  display: flex;
  align-items: flex-start;
  gap: 8px;
}
.qp-detail-icon { font-size: 1.6rem; flex-shrink: 0; }
.qp-detail-name { font-size: 0.88rem; font-weight: 700; color: var(--ts-text-primary, #eaecf4); }
.qp-detail-tagline { font-size: 0.72rem; color: var(--ts-text-muted, #6b7280); }
.qp-detail-close {
  margin-left: auto;
  background: none;
  border: none;
  color: var(--ts-text-dim, #4b5563);
  cursor: pointer;
  font-size: 0.9rem;
  padding: 2px 4px;
}
.qp-detail-close:hover { color: var(--ts-text-primary, #eaecf4); }
.qp-detail-desc {
  font-size: 0.78rem;
  color: var(--ts-text-secondary, #9ca3af);
  line-height: 1.5;
  margin: 8px 0;
}

/* Steps */
.qp-detail-steps { margin-top: 8px; }
.qp-step {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 0;
}
.qp-step-num {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: rgba(124, 111, 255, 0.15);
  color: var(--ts-accent, #7c6fff);
  font-size: 0.68rem;
  font-weight: 700;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.qp-step-label {
  font-size: 0.78rem;
  color: var(--ts-text-secondary, #9ca3af);
  flex: 1;
}
.qp-step-go {
  font-size: 0.7rem;
  color: var(--ts-accent, #7c6fff);
  background: none;
  border: 1px solid rgba(124, 111, 255, 0.3);
  border-radius: var(--ts-radius-pill, 999px);
  padding: 2px 10px;
  cursor: pointer;
  flex-shrink: 0;
  transition: background 0.15s ease;
}
.qp-step-go:hover { background: rgba(124, 111, 255, 0.12); }

/* Rewards */
.qp-detail-rewards { margin-top: 8px; }
.qp-reward-grid { display: flex; flex-wrap: wrap; gap: 4px; }
.qp-reward-chip {
  font-size: 0.72rem;
  padding: 3px 8px;
  border-radius: var(--ts-radius-pill, 999px);
  background: rgba(255, 215, 0, 0.1);
  color: #ffd700;
  border: 1px solid rgba(255, 215, 0, 0.2);
}

/* Actions */
.qp-detail-actions {
  display: flex;
  gap: 6px;
  margin-top: 10px;
}
.qp-action-btn {
  flex: 1;
  padding: 6px 0;
  border-radius: var(--ts-radius-sm, 6px);
  font-size: 0.75rem;
  font-weight: 600;
  cursor: pointer;
  border: 1px solid rgba(124, 111, 255, 0.25);
  background: rgba(124, 111, 255, 0.08);
  color: var(--ts-accent, #7c6fff);
  transition: background 0.15s ease;
}
.qp-action-btn:hover { background: rgba(124, 111, 255, 0.18); }
.qp-chat {
  background: rgba(124, 111, 255, 0.15);
  border-color: rgba(124, 111, 255, 0.4);
}

/* Empty state */
.qp-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 24px 14px;
}
.qp-empty-icon { font-size: 2rem; }
.qp-empty-text { font-size: 0.82rem; color: var(--ts-text-muted, #6b7280); }

/* ── Transitions ── */
.quest-panel-enter-active { animation: panel-slide-up 0.25s ease-out; }
.quest-panel-leave-active { animation: panel-slide-up 0.2s ease-in reverse; }
@keyframes panel-slide-up {
  0% { opacity: 0; transform: translateY(10px) scale(0.95); }
  100% { opacity: 1; transform: translateY(0) scale(1); }
}

.qp-detail-enter-active { animation: detail-fade 0.2s ease-out; }
.qp-detail-leave-active { animation: detail-fade 0.15s ease-in reverse; }
@keyframes detail-fade {
  0% { opacity: 0; }
  100% { opacity: 1; }
}

/* Mobile */
@media (max-width: 640px) {
  /* Mobile positioning adjustments are handled by the dynamic :style binding */
  .quest-bubble { width: 48px; height: 48px; }
  .quest-bubble-pct { font-size: 0.65rem; }
  .quest-panel {
    width: calc(100vw - 32px);
    right: 0;
    max-height: 400px;
  }
}
</style>
