<template>
  <div
    class="quest-hub"
    :style="questHubPosition"
  >
    <!-- Floating crystal orb -->
    <button
      class="ff-orb"
      :class="{ 'ff-orb--open': constellationOpen }"
      :title="`Skill Progress: ${progressPercent}%`"
      @click="toggleConstellation"
    >
      <svg
        class="ff-orb-ring"
        viewBox="0 0 52 52"
      >
        <circle
          class="ff-orb-ring-bg"
          cx="26"
          cy="26"
          r="22"
        />
        <circle
          class="ff-orb-ring-fill"
          cx="26"
          cy="26"
          r="22"
          :stroke-dasharray="circumference"
          :stroke-dashoffset="circumference - (circumference * progressPercent / 100)"
        />
      </svg>
      <span class="ff-orb-crystal">✦</span>
      <span class="ff-orb-pct">{{ progressPercent }}%</span>
    </button>

    <!-- Full-screen constellation map -->
    <SkillConstellation
      :visible="constellationOpen"
      @close="constellationOpen = false"
      @navigate="onConstellationNavigate"
      @begin="onConstellationBegin"
    />

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
import SkillConstellation from './SkillConstellation.vue';
import type { RewardChoice } from './QuestRewardPanel.vue';

const emit = defineEmits<{
  navigate: [target: string];
  trigger: [];
  'update:constellationOpen': [value: boolean];
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
const constellationOpen = ref(false);
watch(constellationOpen, (val) => emit('update:constellationOpen', val));
const screenWidth = ref(window.innerWidth);
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

const circumference = 2 * Math.PI * 22; // r=22

const progressPercent = computed(() => skillTree.progressPercent);
const availableQuests = computed(() => {
  const pinned = new Set(skillTree.tracker.pinnedQuestIds);
  return skillTree.nodes.filter(n =>
    skillTree.getSkillStatus(n.id) === 'available' && !pinned.has(n.id),
  );
});

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
  },
);

watch(
  () => conversationStore.isThinking,
  (isThinking) => {
    if (isThinking && showRewardChoices.value) closeRewardPanel();
  },
);

function handleResize() {
  screenWidth.value = window.innerWidth;
}

function toggleConstellation() {
  constellationOpen.value = !constellationOpen.value;
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

function onConstellationNavigate(target: string) {
  constellationOpen.value = false;
  emit('navigate', target);
}

function onConstellationBegin(questId: string) {
  const quest = skillTree.nodes.find(n => n.id === questId);
  if (quest) {
    questToConfirm.value = quest;
    showConfirmDialog.value = true;
  }
}

function handleAcceptQuest() {
  if (questToConfirm.value) {
    const questId = questToConfirm.value.id;
    showConfirmDialog.value = false;
    constellationOpen.value = false;
    // The user already confirmed via the dialog — don't re-prompt them with
    // the "🗡️ A New Quest Appears!" Accept/Tell-me-more/Maybe-later buttons.
    // Jump straight to the "Quest Accepted!" follow-up so the next thing the
    // adventurer sees is the actual first step + navigation choice.
    skillTree.handleQuestChoice(questId, 'accept');
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
   Floating Crystal Orb — opens the Skill Constellation
   ═══════════════════════════════════════════════════════════════════════════ */

.ff-orb {
  width: 56px;
  height: 56px;
  border-radius: 50%;
  border: 2px solid var(--ts-quest-gold-glow);
  background: var(--ts-quest-bg, radial-gradient(ellipse at 40% 35%, rgba(30, 28, 50, 0.95), rgba(10, 8, 25, 0.98)));
  backdrop-filter: blur(12px);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  position: relative;
  box-shadow:
    0 0 16px var(--ts-quest-gold-dim),
    inset 0 0 20px var(--ts-accent-glow);
  transition: transform 0.2s ease, box-shadow 0.25s ease;
  flex-direction: column;
  gap: 0;
}
.ff-orb:hover {
  transform: scale(1.1);
  box-shadow:
    0 0 24px var(--ts-quest-gold-glow),
    inset 0 0 24px var(--ts-accent-glow);
}
.ff-orb:active { transform: scale(0.95); }
.ff-orb--open {
  border-color: var(--ts-quest-gold);
  box-shadow:
    0 0 28px var(--ts-quest-gold-glow),
    inset 0 0 20px var(--ts-accent-glow);
}

.ff-orb-ring {
  position: absolute;
  width: 100%;
  height: 100%;
  transform: rotate(-90deg);
}
.ff-orb-ring-bg {
  fill: none;
  stroke: var(--ts-quest-gold-dim);
  stroke-width: 2.5;
}
.ff-orb-ring-fill {
  fill: none;
  stroke: var(--ts-quest-gold);
  stroke-width: 2.5;
  stroke-linecap: round;
  transition: stroke-dashoffset 0.8s ease;
  filter: drop-shadow(0 0 3px var(--ts-quest-gold-dim));
}

.ff-orb-crystal {
  font-size: 1rem;
  color: var(--ts-text-link, #8ec8f6);
  z-index: 1;
  text-shadow: 0 0 8px var(--ts-accent-glow);
  animation: crystal-pulse 3s ease-in-out infinite;
}
@keyframes crystal-pulse {
  0%, 100% { opacity: 0.8; transform: scale(1); }
  50% { opacity: 1; transform: scale(1.08); }
}

.ff-orb-pct {
  font-size: 0.58rem;
  font-weight: 700;
  color: var(--ts-quest-gold);
  z-index: 1;
  letter-spacing: 0.04em;
  margin-top: -2px;
}

/* ── Mobile ── */
@media (max-width: 640px) {
  .ff-orb { width: 48px; height: 48px; }
  .ff-orb-crystal { font-size: 0.85rem; }
  .ff-orb-pct { font-size: 0.52rem; }
}
</style>
