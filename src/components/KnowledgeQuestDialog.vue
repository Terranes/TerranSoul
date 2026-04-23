<template>
  <Teleport to="body">
    <Transition name="kq-enter">
      <div v-if="visible" class="kq-backdrop" @click.self="$emit('close')">
        <div class="kq-dialog" role="dialog" aria-labelledby="kq-title">
          <!-- ═══ Header ═══ -->
          <header class="kq-header">
            <div class="kq-header-icon">📚</div>
            <div class="kq-header-text">
              <span class="kq-label">SCHOLAR'S QUEST</span>
              <h2 id="kq-title" class="kq-title">{{ topic || 'Knowledge Acquisition' }}</h2>
            </div>
            <button class="kq-close" @click="$emit('close')" aria-label="Close">✕</button>
          </header>

          <!-- ═══ Step Tracker ═══ -->
          <nav class="kq-steps" aria-label="Quest progress">
            <div
              v-for="(step, i) in steps"
              :key="i"
              class="kq-step"
              :class="{
                'kq-step--done': step.status === 'done',
                'kq-step--active': step.status === 'active',
                'kq-step--pending': step.status === 'pending',
              }"
            >
              <span class="kq-step-num">{{ step.status === 'done' ? '✓' : i + 1 }}</span>
              <span class="kq-step-icon">{{ step.icon }}</span>
              <span class="kq-step-label">{{ step.label }}</span>
            </div>
          </nav>

          <!-- ═══ Step Content ═══ -->
          <div class="kq-body">

            <!-- Step 1: Verify Brain -->
            <section v-if="currentStep === 0" class="kq-section">
              <h3 class="kq-section-title">🧠 Verifying Brain</h3>
              <div class="kq-check-list">
                <div v-for="check in brainChecks" :key="check.label" class="kq-check">
                  <span class="kq-check-icon">{{ check.ok ? '✅' : check.checking ? '⏳' : '❌' }}</span>
                  <span class="kq-check-label">{{ check.label }}</span>
                  <span v-if="check.detail" class="kq-check-detail">{{ check.detail }}</span>
                </div>
              </div>
              <p v-if="brainReady" class="kq-success-text">All systems ready! Your brain can learn.</p>
              <p v-else-if="brainError" class="kq-error-text">{{ brainError }}</p>
            </section>

            <!-- Step 2: Gather Sources -->
            <section v-if="currentStep === 1" class="kq-section">
              <h3 class="kq-section-title">📖 Gather Sources</h3>
              <p class="kq-hint">Add URLs or attach files containing the knowledge you want me to learn.</p>

              <div class="kq-source-input">
                <div class="kq-url-row">
                  <input
                    v-model="urlInput"
                    type="url"
                    class="kq-url-field"
                    placeholder="https://example.com/document"
                    @keydown.enter.prevent="addUrl"
                  />
                  <button class="kq-url-add" :disabled="!urlInput.trim()" @click="addUrl">
                    ＋ Add URL
                  </button>
                </div>
                <div class="kq-file-row">
                  <button class="kq-file-btn" @click="openFilePicker">
                    📎 Attach File
                  </button>
                  <input
                    ref="fileInputRef"
                    type="file"
                    class="kq-file-hidden"
                    accept=".md,.txt,.csv,.json,.xml,.html,.htm,.log,.rst,.adoc,.pdf"
                    @change="handleFileSelected"
                  />
                </div>
              </div>

              <div v-if="sources.length > 0" class="kq-source-list">
                <h4 class="kq-source-list-title">Sources added:</h4>
                <div v-for="(src, i) in sources" :key="i" class="kq-source-item">
                  <span class="kq-source-icon">{{ src.type === 'url' ? '🔗' : '📄' }}</span>
                  <span class="kq-source-name">{{ src.name }}</span>
                  <button class="kq-source-remove" @click="removeSource(i)" aria-label="Remove">✕</button>
                </div>
              </div>
            </section>

            <!-- Step 3: Learning -->
            <section v-if="currentStep === 2" class="kq-section">
              <h3 class="kq-section-title">⚡ Learning in Progress</h3>
              <div class="kq-progress-area">
                <div v-for="task in activeTasks" :key="task.id" class="kq-task">
                  <div class="kq-task-header">
                    <span class="kq-task-desc">{{ task.description }}</span>
                    <span class="kq-task-pct">{{ task.progress }}%</span>
                  </div>
                  <div class="kq-progress-bar">
                    <div class="kq-progress-fill" :style="{ width: task.progress + '%' }" />
                  </div>
                  <p v-if="task.status === 'completed'" class="kq-task-done">✅ Complete — {{ task.processed_items }} chunks stored</p>
                  <p v-else-if="task.status === 'failed'" class="kq-task-fail">❌ {{ task.error }}</p>
                </div>
                <p v-if="allTasksDone" class="kq-success-text">
                  🎉 All sources ingested! {{ totalChunks }} knowledge chunks stored.
                </p>
              </div>
            </section>

            <!-- Step 4: Ready -->
            <section v-if="currentStep === 3" class="kq-section">
              <h3 class="kq-section-title">🎯 Knowledge Acquired!</h3>
              <div class="kq-complete-card">
                <span class="kq-complete-icon">🏆</span>
                <p class="kq-complete-text">
                  I've studied <strong>{{ totalChunks }} knowledge chunks</strong> about
                  <strong>{{ topic }}</strong>. Ask me anything — my answers will now draw
                  from the sources you provided!
                </p>
              </div>
              <div class="kq-reward-grid">
                <div class="kq-reward-card">
                  <span>📚</span><span>RAG-augmented answers</span>
                </div>
                <div class="kq-reward-card">
                  <span>🔍</span><span>Semantic search</span>
                </div>
                <div class="kq-reward-card">
                  <span>🧠</span><span>Persistent memory</span>
                </div>
                <div class="kq-reward-card">
                  <span>⚡</span><span>Source-grounded replies</span>
                </div>
              </div>
            </section>
          </div>

          <!-- ═══ Footer Actions ═══ -->
          <footer class="kq-footer">
            <button
              v-if="currentStep === 0 && brainReady"
              class="kq-btn kq-btn-primary"
              @click="advanceStep"
            >
              Continue ▸
            </button>
            <button
              v-if="currentStep === 1 && sources.length > 0"
              class="kq-btn kq-btn-primary"
              @click="startIngestion"
            >
              ⚡ Start Learning
            </button>
            <button
              v-if="currentStep === 2 && allTasksDone"
              class="kq-btn kq-btn-primary"
              @click="advanceStep"
            >
              Continue ▸
            </button>
            <button
              v-if="currentStep === 3"
              class="kq-btn kq-btn-primary kq-btn-glow"
              @click="finishQuest"
            >
              🗡️ Ask Questions
            </button>
          </footer>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue';
import { useBrainStore } from '../stores/brain';
import { useTaskStore, type TaskInfo } from '../stores/tasks';
import { useMemoryStore } from '../stores/memory';

const props = defineProps<{
  visible: boolean;
  topic: string;
}>();

const emit = defineEmits<{
  close: [];
  finish: [];
}>();

interface QuestSource {
  type: 'url' | 'file';
  name: string;
  path: string; // URL or file path
}

interface BrainCheck {
  label: string;
  ok: boolean;
  checking: boolean;
  detail?: string;
}

const brain = useBrainStore();
const taskStore = useTaskStore();
const memoryStore = useMemoryStore();

// ── Step management ──
const currentStep = ref(0);
const steps = computed(() => [
  { icon: '🧠', label: 'Verify Brain', status: currentStep.value > 0 ? 'done' : currentStep.value === 0 ? 'active' : 'pending' },
  { icon: '📖', label: 'Gather Sources', status: currentStep.value > 1 ? 'done' : currentStep.value === 1 ? 'active' : 'pending' },
  { icon: '⚡', label: 'Learn', status: currentStep.value > 2 ? 'done' : currentStep.value === 2 ? 'active' : 'pending' },
  { icon: '🎯', label: 'Ready', status: currentStep.value === 3 ? 'active' : 'pending' },
]);

// ── Step 1: Brain verification ──
const brainChecks = ref<BrainCheck[]>([
  { label: 'Brain configured', ok: false, checking: true },
  { label: 'LLM model loaded', ok: false, checking: true },
  { label: 'Memory system ready', ok: false, checking: true },
  { label: 'Ingestion engine online', ok: false, checking: true },
]);
const brainReady = computed(() => brainChecks.value.every(c => c.ok));
const brainError = ref<string | null>(null);

async function verifyBrain() {
  // Check 1: Brain configured
  const hasBrain = brain.hasBrain && brain.brainMode !== null;
  brainChecks.value[0] = {
    label: 'Brain configured',
    ok: hasBrain,
    checking: false,
    detail: hasBrain
      ? `${brain.brainMode!.mode === 'local_ollama' ? 'Ollama' : brain.brainMode!.mode} · ${(brain.brainMode as any).model ?? ''}`
      : 'No brain configured',
  };

  // Check 2: LLM model loaded
  const isLocal = brain.brainMode?.mode === 'local_ollama';
  let modelOk = hasBrain;
  let modelDetail = '';
  if (isLocal) {
    try {
      const resp = await fetch('http://localhost:11434/api/tags');
      if (resp.ok) {
        const data = await resp.json();
        modelOk = data.models && data.models.length > 0;
        modelDetail = modelOk ? `${data.models[0].name} loaded` : 'No models found';
      }
    } catch {
      modelOk = false;
      modelDetail = 'Ollama not reachable';
    }
  } else if (hasBrain) {
    modelDetail = 'Cloud provider active';
  }
  brainChecks.value[1] = { label: 'LLM model loaded', ok: modelOk, checking: false, detail: modelDetail };

  // Check 3: Memory system
  let memOk = false;
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('get_memories');
    memOk = true;
  } catch {
    memOk = false;
  }
  brainChecks.value[2] = {
    label: 'Memory system ready',
    ok: memOk,
    checking: false,
    detail: memOk ? 'SQLite connected' : 'Tauri IPC required',
  };

  // Check 4: Ingestion engine
  let ingestOk = false;
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('get_all_tasks');
    ingestOk = true;
  } catch {
    ingestOk = false;
  }
  brainChecks.value[3] = {
    label: 'Ingestion engine online',
    ok: ingestOk,
    checking: false,
    detail: ingestOk ? 'Ready to learn' : 'Tauri IPC required',
  };

  if (!brainReady.value) {
    brainError.value = 'Some systems are not available. Ensure the Tauri desktop app is running.';
  }
}

// ── Step 2: Sources ──
const urlInput = ref('');
const sources = ref<QuestSource[]>([]);
const fileInputRef = ref<HTMLInputElement | null>(null);

function addUrl() {
  const url = urlInput.value.trim();
  if (!url) return;
  sources.value.push({ type: 'url', name: url, path: url });
  urlInput.value = '';
}

function openFilePicker() {
  fileInputRef.value?.click();
}

function handleFileSelected() {
  const file = fileInputRef.value?.files?.[0];
  if (!file) return;
  sources.value.push({ type: 'file', name: file.name, path: file.name });
  if (fileInputRef.value) fileInputRef.value.value = '';
}

function removeSource(index: number) {
  sources.value.splice(index, 1);
}

// ── Step 3: Ingestion ──
const taskIds = ref<string[]>([]);
const activeTasks = computed(() => {
  const all: TaskInfo[] = [];
  for (const id of taskIds.value) {
    const t = taskStore.tasks.get(id);
    if (t) all.push(t);
  }
  return all;
});
const allTasksDone = computed(() =>
  taskIds.value.length > 0 &&
  activeTasks.value.every(t => t.status === 'completed' || t.status === 'failed')
);
const totalChunks = computed(() =>
  activeTasks.value.reduce((sum, t) => sum + (t.processed_items ?? 0), 0)
);

async function startIngestion() {
  currentStep.value = 2;
  for (const src of sources.value) {
    try {
      const result = await taskStore.ingestDocument(
        src.type === 'url' ? src.path : src.path,
        `knowledge,${props.topic.toLowerCase().replace(/\s+/g, '-')}`,
        5,
      );
      taskIds.value.push(result.task_id);
    } catch (err) {
      console.error('Ingestion failed:', err);
    }
  }
}

// Watch for all tasks completing → auto-advance
watch(allTasksDone, (done) => {
  if (done && currentStep.value === 2) {
    // Small delay for visual feedback
    setTimeout(() => advanceStep(), 1500);
  }
});

// ── Navigation ──
function advanceStep() {
  if (currentStep.value < 3) {
    currentStep.value++;
  }
}

async function finishQuest() {
  // Refresh memories so RAG can use the new knowledge
  try {
    await memoryStore.fetchAll();
  } catch { /* browser mode fallback */ }
  emit('finish');
}

// ── Lifecycle ──
onMounted(() => {
  if (props.visible) {
    verifyBrain();
  }
});

watch(() => props.visible, (v) => {
  if (v) {
    currentStep.value = 0;
    brainError.value = null;
    taskIds.value = [];
    sources.value = [];
    urlInput.value = '';
    verifyBrain();
  }
});
</script>

<style scoped>
/* ═══ Backdrop ═══ */
.kq-backdrop {
  position: fixed;
  inset: 0;
  z-index: 250;
  background: rgba(0, 0, 0, 0.85);
  backdrop-filter: blur(12px);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 16px;
}

/* ═══ Dialog ═══ */
.kq-dialog {
  width: 100%;
  max-width: 560px;
  max-height: 90vh;
  overflow-y: auto;
  background: linear-gradient(180deg, #1a1a2e 0%, #16213e 40%, #0f3460 100%);
  border: 2px solid #ffd700;
  border-radius: 16px;
  padding: 0;
  box-shadow:
    0 0 60px rgba(255, 215, 0, 0.15),
    0 0 120px rgba(255, 215, 0, 0.05),
    inset 0 1px 0 rgba(255, 255, 255, 0.05);
  scrollbar-width: thin;
  scrollbar-color: rgba(255, 215, 0, 0.3) transparent;
}

/* ═══ Header ═══ */
.kq-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 20px 24px 16px;
  border-bottom: 1px solid rgba(255, 215, 0, 0.2);
  background: linear-gradient(180deg, rgba(255, 215, 0, 0.08) 0%, transparent 100%);
}
.kq-header-icon {
  font-size: 2.2rem;
  width: 56px;
  height: 56px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  border: 2px solid #ffd700;
  background: rgba(255, 215, 0, 0.1);
  animation: kq-icon-glow 2.5s ease-in-out infinite;
  flex-shrink: 0;
}
@keyframes kq-icon-glow {
  0%, 100% { box-shadow: 0 0 0 0 rgba(255, 215, 0, 0); }
  50% { box-shadow: 0 0 20px 6px rgba(255, 215, 0, 0.25); }
}
.kq-header-text { flex: 1; }
.kq-label {
  display: block;
  font-size: 0.65rem;
  text-transform: uppercase;
  letter-spacing: 3px;
  color: #ffd700;
  font-weight: 700;
  margin-bottom: 2px;
}
.kq-title {
  margin: 0;
  font-size: 1.15rem;
  color: #fff;
  font-weight: 600;
}
.kq-close {
  background: none;
  border: none;
  color: rgba(255, 255, 255, 0.4);
  font-size: 1.1rem;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 4px;
  transition: color 0.2s, background 0.2s;
}
.kq-close:hover {
  color: #fff;
  background: rgba(255, 255, 255, 0.1);
}

/* ═══ Step Tracker ═══ */
.kq-steps {
  display: flex;
  gap: 0;
  padding: 0 24px;
  margin: 16px 0;
}
.kq-step {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 8px 4px;
  position: relative;
  opacity: 0.4;
  transition: opacity 0.3s;
}
.kq-step--active { opacity: 1; }
.kq-step--done { opacity: 0.8; }
.kq-step::after {
  content: '';
  position: absolute;
  top: 20px;
  right: -50%;
  width: 100%;
  height: 2px;
  background: rgba(255, 215, 0, 0.15);
}
.kq-step:last-child::after { display: none; }
.kq-step--done::after { background: #ffd700; }
.kq-step-num {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 0.75rem;
  font-weight: 700;
  border: 2px solid rgba(255, 215, 0, 0.3);
  color: rgba(255, 255, 255, 0.6);
  background: rgba(0, 0, 0, 0.3);
  z-index: 1;
}
.kq-step--active .kq-step-num {
  border-color: #ffd700;
  color: #ffd700;
  background: rgba(255, 215, 0, 0.15);
  box-shadow: 0 0 12px rgba(255, 215, 0, 0.3);
}
.kq-step--done .kq-step-num {
  border-color: #4ade80;
  color: #4ade80;
  background: rgba(74, 222, 128, 0.15);
}
.kq-step-icon { font-size: 0.9rem; }
.kq-step-label {
  font-size: 0.68rem;
  color: rgba(255, 255, 255, 0.6);
  text-align: center;
  white-space: nowrap;
}
.kq-step--active .kq-step-label { color: #ffd700; font-weight: 600; }

/* ═══ Body ═══ */
.kq-body {
  padding: 0 24px 16px;
}
.kq-section {
  animation: kq-section-in 0.3s ease;
}
@keyframes kq-section-in {
  from { opacity: 0; transform: translateY(8px); }
  to { opacity: 1; transform: translateY(0); }
}
.kq-section-title {
  margin: 0 0 12px;
  font-size: 0.95rem;
  color: #fff;
  font-weight: 600;
}
.kq-hint {
  font-size: 0.82rem;
  color: rgba(255, 255, 255, 0.6);
  margin: 0 0 16px;
  line-height: 1.4;
}

/* ── Brain checks ── */
.kq-check-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.kq-check {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 14px;
  background: rgba(0, 0, 0, 0.2);
  border-radius: 8px;
  border: 1px solid rgba(255, 255, 255, 0.06);
}
.kq-check-icon { font-size: 1rem; flex-shrink: 0; }
.kq-check-label { font-size: 0.82rem; color: #fff; flex: 1; }
.kq-check-detail { font-size: 0.72rem; color: rgba(255, 255, 255, 0.45); }

.kq-success-text {
  margin: 14px 0 0;
  font-size: 0.85rem;
  color: #4ade80;
  font-weight: 600;
}
.kq-error-text {
  margin: 14px 0 0;
  font-size: 0.82rem;
  color: #f87171;
}

/* ── Source input ── */
.kq-source-input {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.kq-url-row {
  display: flex;
  gap: 8px;
}
.kq-url-field {
  flex: 1;
  padding: 10px 14px;
  background: rgba(0, 0, 0, 0.3);
  border: 1px solid rgba(255, 215, 0, 0.2);
  border-radius: 8px;
  color: #fff;
  font-size: 0.82rem;
  outline: none;
  transition: border-color 0.2s;
}
.kq-url-field::placeholder { color: rgba(255, 255, 255, 0.3); }
.kq-url-field:focus { border-color: #ffd700; }
.kq-url-add {
  padding: 10px 16px;
  background: linear-gradient(135deg, rgba(255, 215, 0, 0.2) 0%, rgba(255, 165, 0, 0.15) 100%);
  border: 1px solid rgba(255, 215, 0, 0.4);
  border-radius: 8px;
  color: #ffd700;
  font-size: 0.82rem;
  font-weight: 600;
  cursor: pointer;
  white-space: nowrap;
  transition: border-color 0.2s, background 0.2s;
}
.kq-url-add:hover:not(:disabled) { border-color: #ffd700; background: rgba(255, 215, 0, 0.25); }
.kq-url-add:disabled { opacity: 0.4; cursor: default; }
.kq-file-row {
  display: flex;
}
.kq-file-btn {
  padding: 10px 16px;
  background: rgba(56, 189, 248, 0.12);
  border: 1px solid rgba(56, 189, 248, 0.3);
  border-radius: 8px;
  color: #38bdf8;
  font-size: 0.82rem;
  font-weight: 600;
  cursor: pointer;
  transition: border-color 0.2s, background 0.2s;
}
.kq-file-btn:hover { border-color: #38bdf8; background: rgba(56, 189, 248, 0.2); }
.kq-file-hidden { display: none; }

/* ── Source list ── */
.kq-source-list { margin-top: 14px; }
.kq-source-list-title {
  font-size: 0.75rem;
  color: rgba(255, 255, 255, 0.5);
  text-transform: uppercase;
  letter-spacing: 1px;
  margin: 0 0 8px;
}
.kq-source-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: rgba(0, 0, 0, 0.2);
  border-radius: 6px;
  margin-bottom: 4px;
  border: 1px solid rgba(255, 255, 255, 0.06);
}
.kq-source-icon { font-size: 0.9rem; }
.kq-source-name {
  flex: 1;
  font-size: 0.78rem;
  color: rgba(255, 255, 255, 0.8);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.kq-source-remove {
  background: none;
  border: none;
  color: rgba(255, 255, 255, 0.3);
  cursor: pointer;
  padding: 2px 6px;
  font-size: 0.75rem;
}
.kq-source-remove:hover { color: #f87171; }

/* ── Progress ── */
.kq-progress-area {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.kq-task {
  padding: 12px 14px;
  background: rgba(0, 0, 0, 0.2);
  border-radius: 8px;
  border: 1px solid rgba(255, 255, 255, 0.06);
}
.kq-task-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}
.kq-task-desc { font-size: 0.8rem; color: rgba(255, 255, 255, 0.8); }
.kq-task-pct { font-size: 0.85rem; color: #ffd700; font-weight: 700; }
.kq-progress-bar {
  height: 6px;
  background: rgba(255, 255, 255, 0.08);
  border-radius: 3px;
  overflow: hidden;
}
.kq-progress-fill {
  height: 100%;
  background: linear-gradient(90deg, #ffd700, #ff8c00);
  border-radius: 3px;
  transition: width 0.4s ease;
}
.kq-task-done {
  margin: 8px 0 0;
  font-size: 0.78rem;
  color: #4ade80;
}
.kq-task-fail {
  margin: 8px 0 0;
  font-size: 0.78rem;
  color: #f87171;
}

/* ── Complete ── */
.kq-complete-card {
  text-align: center;
  padding: 24px 16px;
  background: radial-gradient(ellipse at center, rgba(255, 215, 0, 0.08) 0%, transparent 70%);
  border-radius: 12px;
  margin-bottom: 16px;
}
.kq-complete-icon {
  font-size: 3rem;
  display: block;
  margin-bottom: 12px;
  animation: kq-trophy-bounce 1.5s ease-in-out infinite;
}
@keyframes kq-trophy-bounce {
  0%, 100% { transform: translateY(0); }
  50% { transform: translateY(-6px); }
}
.kq-complete-text {
  font-size: 0.88rem;
  color: rgba(255, 255, 255, 0.85);
  line-height: 1.5;
  margin: 0;
}
.kq-reward-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}
.kq-reward-card {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px;
  background: rgba(255, 215, 0, 0.05);
  border: 1px solid rgba(255, 215, 0, 0.15);
  border-radius: 8px;
  font-size: 0.78rem;
  color: rgba(255, 255, 255, 0.7);
}
.kq-reward-card span:first-child { font-size: 1.1rem; }

/* ═══ Footer ═══ */
.kq-footer {
  padding: 16px 24px 20px;
  border-top: 1px solid rgba(255, 215, 0, 0.1);
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
.kq-btn {
  padding: 12px 24px;
  border-radius: 10px;
  font-size: 0.88rem;
  font-weight: 700;
  cursor: pointer;
  transition: transform 0.15s, box-shadow 0.15s;
}
.kq-btn:hover { transform: scale(1.02); }
.kq-btn:active { transform: scale(0.98); }
.kq-btn-primary {
  background: linear-gradient(135deg, #ffd700 0%, #ff8c00 100%);
  border: none;
  color: #1a1a2e;
  box-shadow: 0 4px 16px rgba(255, 215, 0, 0.25);
}
.kq-btn-primary:hover {
  box-shadow: 0 6px 24px rgba(255, 215, 0, 0.35);
}
.kq-btn-glow {
  animation: kq-btn-pulse 2s ease-in-out infinite;
}
@keyframes kq-btn-pulse {
  0%, 100% { box-shadow: 0 4px 16px rgba(255, 215, 0, 0.25); }
  50% { box-shadow: 0 4px 30px rgba(255, 215, 0, 0.5); }
}

/* ═══ Transitions ═══ */
.kq-enter-enter-active { transition: opacity 0.35s ease, transform 0.35s ease; }
.kq-enter-leave-active { transition: opacity 0.25s ease, transform 0.25s ease; }
.kq-enter-enter-from { opacity: 0; }
.kq-enter-enter-from .kq-dialog { transform: scale(0.92) translateY(20px); }
.kq-enter-leave-to { opacity: 0; }
.kq-enter-leave-to .kq-dialog { transform: scale(0.95) translateY(10px); }
</style>
