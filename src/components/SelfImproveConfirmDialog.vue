<template>
  <Teleport to="body">
    <Transition name="si-fade">
      <div
        v-if="visible"
        class="si-confirm-backdrop"
        role="dialog"
        aria-modal="true"
        aria-labelledby="si-confirm-title"
        @click.self="$emit('cancel')"
        @keydown.esc="$emit('cancel')"
      >
        <Transition name="si-pop">
          <div
            v-if="visible"
            class="si-confirm-card"
            tabindex="-1"
          >
            <header class="si-header">
              <span
                class="si-icon"
                aria-hidden="true"
              >⚠️</span>
              <h2
                id="si-confirm-title"
                class="si-title"
              >
                Enable Self-Improve?
              </h2>
            </header>

            <div class="si-body">
              <p class="si-lead">
                TerranSoul will become an <strong>autonomous coding system</strong>
                that modifies its own source code without per-change approval.
              </p>

              <ul class="si-bullets">
                <li>
                  <span class="si-bullet-icon">🤖</span>
                  Runs the configured <strong>Coding LLM</strong>
                  ({{ providerLabel }}) against
                  <code>rules/milestones.md</code>.
                </li>
                <li>
                  <span class="si-bullet-icon">🌿</span>
                  Commits to a <strong>feature branch</strong> and opens a PR.
                  <em>Never pushes to <code>master</code> directly.</em>
                </li>
                <li>
                  <span class="si-bullet-icon">♻️</span>
                  Survives <strong>app, terminal, and computer restart</strong>.
                  The only way to stop is to untick this option.
                </li>
                <li>
                  <span class="si-bullet-icon">🔧</span>
                  May install a <strong>system tray icon</strong> and
                  enable <strong>auto-start with Windows</strong>.
                </li>
              </ul>

              <div
                v-if="!hasCodingLlm"
                class="si-warn"
                role="alert"
              >
                ⚠ No Coding LLM configured. You'll be routed to
                <strong>Brain → Coding LLM</strong> to pick one
                (Claude recommended) before self-improve can start.
              </div>
            </div>

            <footer class="si-actions">
              <button
                ref="cancelBtnRef"
                class="si-btn si-btn-secondary"
                type="button"
                @click="$emit('cancel')"
              >
                No, cancel
              </button>
              <button
                class="si-btn si-btn-danger"
                type="button"
                @click="$emit('confirm')"
              >
                Yes, enable self-improve
              </button>
            </footer>
          </div>
        </Transition>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { nextTick, ref, watch } from 'vue';

const props = defineProps<{
  visible: boolean;
  hasCodingLlm: boolean;
  providerLabel: string;
}>();

defineEmits<{
  confirm: [];
  cancel: [];
}>();

const cancelBtnRef = ref<HTMLButtonElement | null>(null);

// Default-focus the safer "No" option when the dialog opens — accessibility
// best practice for destructive/dangerous confirm flows.
watch(
  () => props.visible,
  async (v) => {
    if (v) {
      await nextTick();
      cancelBtnRef.value?.focus();
    }
  },
);

</script>

<style scoped>
.si-confirm-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.78);
  backdrop-filter: blur(6px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 9500;
  padding: 16px;
}

.si-confirm-card {
  background: linear-gradient(135deg, rgba(20, 14, 32, 0.96), rgba(28, 18, 44, 0.96));
  border: 1px solid rgba(248, 113, 113, 0.45);
  border-radius: 14px;
  box-shadow:
    0 24px 80px rgba(0, 0, 0, 0.6),
    0 0 60px rgba(248, 113, 113, 0.15);
  width: min(520px, 92vw);
  color: var(--ts-text-primary, #eaecf4);
  outline: none;
}

.si-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 18px 20px 12px;
  border-bottom: 1px solid rgba(248, 113, 113, 0.2);
}

.si-icon {
  font-size: 1.8rem;
  filter: drop-shadow(0 2px 6px rgba(248, 113, 113, 0.5));
}

.si-title {
  margin: 0;
  font-size: 1.2rem;
  font-weight: 700;
  background: linear-gradient(135deg, #fca5a5, #f87171);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.si-body {
  padding: 16px 20px;
  font-size: 0.9rem;
  line-height: 1.5;
}

.si-lead {
  margin: 0 0 12px;
}

.si-bullets {
  list-style: none;
  padding: 0;
  margin: 0 0 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.si-bullets li {
  display: flex;
  gap: 10px;
  align-items: flex-start;
  background: rgba(255, 255, 255, 0.04);
  padding: 8px 10px;
  border-radius: 8px;
  border: 1px solid rgba(255, 255, 255, 0.06);
}

.si-bullet-icon {
  font-size: 1.05rem;
  flex-shrink: 0;
}

.si-bullets code {
  background: rgba(255, 255, 255, 0.08);
  padding: 1px 5px;
  border-radius: 4px;
  font-size: 0.82rem;
}

.si-warn {
  background: rgba(251, 191, 36, 0.12);
  border: 1px solid rgba(251, 191, 36, 0.35);
  color: #fde68a;
  padding: 10px 12px;
  border-radius: 8px;
  font-size: 0.85rem;
}

.si-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  padding: 14px 20px 18px;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
}

.si-btn {
  border: none;
  border-radius: 8px;
  padding: 10px 18px;
  font-size: 0.9rem;
  font-weight: 600;
  cursor: pointer;
  transition: transform 0.12s, box-shadow 0.12s, background 0.12s;
}
.si-btn:focus-visible {
  outline: 2px solid var(--ts-accent, #7c6fff);
  outline-offset: 2px;
}

.si-btn-secondary {
  background: rgba(255, 255, 255, 0.08);
  color: var(--ts-text-primary, #eaecf4);
}
.si-btn-secondary:hover {
  background: rgba(255, 255, 255, 0.14);
}

.si-btn-danger {
  background: linear-gradient(135deg, #ef4444, #b91c1c);
  color: white;
  box-shadow: 0 4px 14px rgba(239, 68, 68, 0.35);
}
.si-btn-danger:hover {
  transform: translateY(-1px);
  box-shadow: 0 6px 18px rgba(239, 68, 68, 0.45);
}

.si-fade-enter-active,
.si-fade-leave-active { transition: opacity 0.18s; }
.si-fade-enter-from,
.si-fade-leave-to { opacity: 0; }

.si-pop-enter-active { transition: transform 0.22s ease, opacity 0.22s; }
.si-pop-leave-active { transition: transform 0.14s ease, opacity 0.14s; }
.si-pop-enter-from { transform: translateY(8px) scale(0.97); opacity: 0; }
.si-pop-leave-to   { transform: translateY(4px) scale(0.99); opacity: 0; }
</style>
