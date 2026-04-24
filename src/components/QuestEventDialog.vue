<template>
  <Teleport to="body">
    <div
      class="qe-backdrop"
      @click.self="$emit('close')"
    >
      <div class="qe-dialog">
        <button
          class="qe-close"
          @click="$emit('close')"
        >
          ✕
        </button>

        <!-- Quest Header -->
        <div class="qe-header">
          <div class="qe-icon-wrap">
            <span class="qe-icon">{{ node.icon }}</span>
          </div>
          <div class="qe-header-text">
            <span class="qe-label">🗺️ Quest Event</span>
            <h3 class="qe-name">
              {{ node.name }}
            </h3>
            <span class="qe-tagline">{{ node.tagline }}</span>
          </div>
        </div>

        <!-- Reward Preview (rich icons) -->
        <section class="qe-rewards">
          <h4 class="qe-section-title">
            🎁 Rewards you'll unlock
          </h4>
          <div class="qe-reward-grid">
            <div
              v-for="(reward, i) in node.rewards"
              :key="i"
              class="qe-reward-card"
            >
              <span class="qe-reward-icon">{{ node.rewardIcons[i] || '🎯' }}</span>
              <span class="qe-reward-text">{{ reward }}</span>
            </div>
          </div>
        </section>

        <!-- Video Reference -->
        <section
          v-if="node.videoRef"
          class="qe-video-section"
        >
          <h4 class="qe-section-title">
            🎬 See it in action
          </h4>
          <div class="qe-video-wrapper">
            <iframe
              :src="videoEmbedUrl"
              class="qe-video-iframe"
              allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
              allowfullscreen
              loading="lazy"
              referrerpolicy="no-referrer"
              sandbox="allow-scripts allow-same-origin"
            />
          </div>
        </section>

        <!-- AI Conversation -->
        <section class="qe-chat">
          <h4 class="qe-section-title">
            💬 Quest Guide
          </h4>
          <div
            ref="messagesEl"
            class="qe-messages"
          >
            <div
              v-for="(msg, i) in chatMessages"
              :key="i"
              class="qe-msg"
              :class="'qe-msg-' + msg.role"
            >
              <span class="qe-msg-avatar">{{ msg.role === 'assistant' ? '🧙' : '🗡️' }}</span>
              <div
                class="qe-msg-bubble"
                v-html="renderMarkdown(msg.content)"
              />
            </div>
            <div
              v-if="isStreaming"
              class="qe-msg qe-msg-assistant"
            >
              <span class="qe-msg-avatar">🧙</span>
              <div class="qe-msg-bubble qe-typing">
                <span class="qe-dot" /><span class="qe-dot" /><span class="qe-dot" />
              </div>
            </div>
          </div>
        </section>

        <!-- Yes / No buttons -->
        <div
          v-if="awaitingResponse"
          class="qe-choices"
        >
          <button
            class="qe-btn qe-btn-yes"
            @click="respond('yes')"
          >
            ✅ Yes
          </button>
          <button
            class="qe-btn qe-btn-no"
            @click="respond('no')"
          >
            ❌ No
          </button>
        </div>

        <!-- Navigate to quest button when done -->
        <div
          v-if="questComplete"
          class="qe-done"
        >
          <p class="qe-done-text">
            Ready to start?
          </p>
          <button
            class="qe-btn qe-btn-start"
            @click="$emit('navigate', node.questSteps[0]?.target ?? 'skills')"
          >
            🚀 Begin Quest
          </button>
          <button
            class="qe-btn qe-btn-pin"
            @click="handlePin"
          >
            📌 Pin & Decide Later
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed, nextTick, onMounted, watch } from 'vue';
import { useSkillTreeStore, type SkillNode } from '../stores/skill-tree';
import { useBrainStore } from '../stores/brain';
import { streamChatCompletion, type ChatMessage } from '../utils/free-api-client';
import { renderMarkdown } from '../utils/render-markdown';

const props = defineProps<{
  node: SkillNode;
}>();

const emit = defineEmits<{
  close: [];
  navigate: [target: string];
}>();

interface QeMessage {
  role: 'assistant' | 'user';
  content: string;
}

const skillTree = useSkillTreeStore();
const brain = useBrainStore();
const messagesEl = ref<HTMLElement | null>(null);
const chatMessages = ref<QeMessage[]>([]);
const isStreaming = ref(false);
const awaitingResponse = ref(false);
const questComplete = ref(false);
let questionCount = 0;

const videoEmbedUrl = computed(() => {
  if (!props.node.videoRef) return '';
  return `https://www.youtube-nocookie.com/embed/${encodeURIComponent(props.node.videoRef)}?rel=0&modestbranding=1`;
});

function scrollToBottom() {
  nextTick(() => {
    if (messagesEl.value) {
      messagesEl.value.scrollTop = messagesEl.value.scrollHeight;
    }
  });
}

watch(chatMessages, scrollToBottom, { deep: true });

async function askAI(userReply?: string) {
  isStreaming.value = true;
  awaitingResponse.value = false;

  const status = skillTree.getSkillStatus(props.node.id);
  const rewardsDesc = props.node.rewards
    .map((r, i) => `${props.node.rewardIcons[i] || '•'} ${r}`)
    .join('\n');

  // Build the system prompt for the quest guide conversation
  const systemPrompt = `You are a helpful quest guide for "${props.node.name}" in TerranSoul (a gaming-style AI companion app). 
Skill status: ${status}. Tier: ${props.node.tier}. Category: ${props.node.category}.
Description: ${props.node.description}
Rewards:
${rewardsDesc}
Quest steps: ${props.node.questSteps.map(s => s.label).join(' → ')}

Your job: Ask the user simple YES/NO questions to understand their setup and guide them toward unlocking this skill. Be encouraging, use gaming metaphors. Keep responses to 1-2 short sentences. After 2-3 questions, summarize what they need to do and encourage them to start the quest. Ask one question at a time.`;

  const history: ChatMessage[] = [
    { role: 'system', content: systemPrompt },
    ...chatMessages.value.map(m => ({ role: m.role as 'user' | 'assistant', content: m.content })),
  ];

  if (userReply) {
    history.push({ role: 'user', content: userReply });
  }

  // Try streaming from active brain
  if (brain.hasBrain && brain.brainMode) {
    const mode = brain.brainMode;
    let baseUrl: string;
    let model: string;
    let apiKey: string | null = null;

    if (mode.mode === 'free_api') {
      const provider = brain.freeProviders.find(p => p.id === mode.provider_id);
      if (!provider) { fallbackResponse(); return; }
      baseUrl = provider.base_url;
      model = provider.model;
      apiKey = mode.api_key;
    } else if (mode.mode === 'paid_api') {
      baseUrl = mode.base_url;
      model = mode.model;
      apiKey = mode.api_key;
    } else {
      baseUrl = 'http://localhost:11434';
      model = mode.model;
    }

    let fullText = '';
    try {
      await new Promise<void>((resolve, reject) => {
        streamChatCompletion(baseUrl, model, apiKey, history, {
          onChunk: (text) => { fullText += text; },
          onDone: () => resolve(),
          onError: (err) => reject(new Error(err)),
        });
      });
      if (fullText.trim()) {
        chatMessages.value.push({ role: 'assistant', content: fullText.trim() });
      } else {
        fallbackResponse();
      }
    } catch {
      fallbackResponse();
    }
  } else {
    fallbackResponse();
  }

  isStreaming.value = false;
  questionCount++;

  // After 3 exchanges, mark quest guide as complete
  if (questionCount >= 3) {
    questComplete.value = true;
  } else {
    awaitingResponse.value = true;
  }
}

function fallbackResponse() {
  const fallbacks = [
    [`Adventurer! 🗡️ "${props.node.name}" awaits! Do you have the prerequisites ready?`,
     `Great choice! ${props.node.description} Want to unlock ${props.node.rewards[0]}?`,
     `This skill is in the ${props.node.tier} tier. ${props.node.rewards.length} rewards await! Ready to start?`],
    [`Nice! The next step is: ${props.node.questSteps[0]?.label ?? 'Check the skill tree'}. Shall I guide you there?`,
     `You're making progress! This unlocks: ${props.node.rewards.slice(0, 2).join(' and ')}. Want to proceed?`,
     `Almost there! Just follow the quest steps and you'll unlock some powerful abilities!`],
    [`You're all set to begin! 🎉 Head to the quest steps and unlock "${props.node.name}". The rewards are worth it!`,
     `Quest guide complete! Go claim your rewards: ${props.node.rewards.join(', ')}. Good luck, adventurer! 🌟`,
     `That's all I needed to know! You're ready to tackle this quest. May your skills grow stronger! ⚔️`],
  ];
  const pool = fallbacks[Math.min(questionCount, fallbacks.length - 1)];
  chatMessages.value.push({
    role: 'assistant',
    content: pool[Math.floor(Math.random() * pool.length)],
  });
}

function respond(answer: string) {
  chatMessages.value.push({ role: 'user', content: answer === 'yes' ? 'Yes!' : 'No' });
  askAI(answer === 'yes' ? 'Yes!' : 'No');
}

function handlePin() {
  skillTree.pinQuest(props.node.id);
  emit('close');
}

onMounted(() => {
  askAI();
});
</script>

<style scoped>
.qe-backdrop {
  position: fixed; inset: 0; z-index: 210;
  background: rgba(0, 0, 0, 0.7); backdrop-filter: blur(8px);
  display: flex; align-items: center; justify-content: center;
  padding: var(--ts-space-lg);
  animation: qe-fade-in 0.3s ease;
}
@keyframes qe-fade-in { from { opacity: 0; } to { opacity: 1; } }

.qe-dialog {
  width: 100%; max-width: 540px; max-height: 88vh; overflow-y: auto;
  background: var(--ts-bg-card);
  border: 1px solid var(--ts-warning);
  border-radius: var(--ts-radius-xl);
  padding: var(--ts-space-xl);
  position: relative;
  box-shadow: 0 0 40px rgba(251, 191, 36, 0.15), var(--ts-shadow-lg);
  scrollbar-width: thin;
}
.qe-close {
  position: absolute; top: 12px; right: 12px;
  background: none; border: none; color: var(--ts-text-muted);
  font-size: 1.1rem; cursor: pointer; padding: 4px;
  transition: color var(--ts-transition-fast);
}
.qe-close:hover { color: var(--ts-text-primary); }

/* Header */
.qe-header { display: flex; gap: var(--ts-space-md); align-items: center; margin-bottom: var(--ts-space-lg); }
.qe-icon-wrap {
  width: 60px; height: 60px; border-radius: 50%;
  display: flex; align-items: center; justify-content: center;
  background: rgba(251, 191, 36, 0.1); border: 2px solid var(--ts-warning);
  animation: qe-icon-pulse 2s ease-in-out infinite;
  flex-shrink: 0;
}
@keyframes qe-icon-pulse {
  0%, 100% { box-shadow: 0 0 0 0 rgba(251, 191, 36, 0); }
  50% { box-shadow: 0 0 16px 4px rgba(251, 191, 36, 0.25); }
}
.qe-icon { font-size: 2rem; }
.qe-header-text { display: flex; flex-direction: column; gap: 2px; }
.qe-label { font-size: 0.7rem; text-transform: uppercase; letter-spacing: 1.5px; color: var(--ts-warning); font-weight: 700; }
.qe-name { margin: 0; font-size: 1.2rem; color: var(--ts-text-primary); }
.qe-tagline { font-size: 0.82rem; color: var(--ts-text-muted); }

/* Sections */
.qe-section-title { margin: 0 0 var(--ts-space-sm); font-size: 0.85rem; color: var(--ts-text-primary); }

/* Rewards */
.qe-rewards { margin-bottom: var(--ts-space-lg); }
.qe-reward-grid {
  display: grid; grid-template-columns: 1fr 1fr;
  gap: var(--ts-space-xs);
}
.qe-reward-card {
  display: flex; align-items: center; gap: 8px;
  padding: 8px 12px;
  background: var(--ts-bg-surface);
  border: 1px solid var(--ts-border-subtle);
  border-radius: var(--ts-radius-md);
  transition: border-color var(--ts-transition-fast), background var(--ts-transition-fast);
}
.qe-reward-card:hover {
  border-color: var(--ts-warning);
  background: rgba(251, 191, 36, 0.05);
}
.qe-reward-icon { font-size: 1.2rem; flex-shrink: 0; }
.qe-reward-text { font-size: 0.78rem; color: var(--ts-text-secondary); line-height: 1.3; }

/* Video */
.qe-video-section { margin-bottom: var(--ts-space-lg); }
.qe-video-wrapper {
  position: relative; padding-bottom: 56.25%; height: 0; overflow: hidden;
  border-radius: var(--ts-radius-md); border: 1px solid var(--ts-border-subtle);
}
.qe-video-iframe {
  position: absolute; top: 0; left: 0; width: 100%; height: 100%;
  border: none;
}

/* Chat */
.qe-chat { margin-bottom: var(--ts-space-md); }
.qe-messages {
  max-height: 200px; overflow-y: auto; scrollbar-width: thin;
  display: flex; flex-direction: column; gap: var(--ts-space-sm);
  padding: var(--ts-space-sm);
  background: var(--ts-bg-surface);
  border-radius: var(--ts-radius-md);
  border: 1px solid var(--ts-border-subtle);
}
.qe-msg { display: flex; gap: 8px; align-items: flex-start; }
.qe-msg-user { flex-direction: row-reverse; }
.qe-msg-avatar { font-size: 1.1rem; flex-shrink: 0; margin-top: 2px; }
.qe-msg-bubble {
  padding: 8px 12px;
  border-radius: var(--ts-radius-md);
  font-size: 0.82rem; line-height: 1.4;
  max-width: 80%;
}
.qe-msg-assistant .qe-msg-bubble {
  background: var(--ts-bg-card);
  border: 1px solid var(--ts-border-subtle);
  color: var(--ts-text-secondary);
}
.qe-msg-user .qe-msg-bubble {
  background: var(--ts-accent-glow);
  border: 1px solid var(--ts-accent);
  color: var(--ts-accent);
}
.qe-typing { display: flex; gap: 4px; align-items: center; padding: 10px 16px; }
.qe-dot {
  width: 6px; height: 6px; border-radius: 50%; background: var(--ts-text-muted);
  animation: qe-dot-bounce 1.2s ease-in-out infinite;
}
.qe-dot:nth-child(2) { animation-delay: 0.2s; }
.qe-dot:nth-child(3) { animation-delay: 0.4s; }
@keyframes qe-dot-bounce {
  0%, 80%, 100% { transform: translateY(0); opacity: 0.4; }
  40% { transform: translateY(-6px); opacity: 1; }
}

/* Choices */
.qe-choices {
  display: flex; gap: var(--ts-space-sm); justify-content: center;
  margin-top: var(--ts-space-sm);
}
.qe-btn {
  padding: 8px 20px; border-radius: var(--ts-radius-md);
  font-size: 0.85rem; font-weight: 600; cursor: pointer;
  transition: background var(--ts-transition-fast), transform 0.15s ease, box-shadow 0.15s ease;
  border: 1px solid transparent;
}
.qe-btn:active { transform: scale(0.96); }
.qe-btn-yes {
  background: rgba(52, 211, 153, 0.15); border-color: var(--ts-success); color: var(--ts-success);
}
.qe-btn-yes:hover { background: rgba(52, 211, 153, 0.25); box-shadow: 0 0 12px rgba(52, 211, 153, 0.2); }
.qe-btn-no {
  background: rgba(239, 68, 68, 0.1); border-color: var(--ts-error, #ef4444); color: var(--ts-error, #ef4444);
}
.qe-btn-no:hover { background: rgba(239, 68, 68, 0.2); box-shadow: 0 0 12px rgba(239, 68, 68, 0.2); }

/* Done state */
.qe-done {
  display: flex; flex-direction: column; align-items: center; gap: var(--ts-space-sm);
  margin-top: var(--ts-space-md);
  padding: var(--ts-space-md);
  background: rgba(52, 211, 153, 0.05);
  border: 1px solid rgba(52, 211, 153, 0.2);
  border-radius: var(--ts-radius-md);
}
.qe-done-text { margin: 0; font-size: 0.88rem; color: var(--ts-text-secondary); font-weight: 600; }
.qe-btn-start {
  background: var(--ts-success); border-color: var(--ts-success); color: #fff;
  font-size: 0.9rem; padding: 10px 28px;
}
.qe-btn-start:hover { box-shadow: 0 0 16px rgba(52, 211, 153, 0.4); }
.qe-btn-pin {
  background: transparent; border-color: var(--ts-border); color: var(--ts-text-secondary);
}
.qe-btn-pin:hover { background: var(--ts-bg-hover); }

@media (max-width: 640px) {
  .qe-dialog { max-width: 100%; max-height: 92vh; padding: var(--ts-space-lg); }
  .qe-reward-grid { grid-template-columns: 1fr; }
}
</style>
