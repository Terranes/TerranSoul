<template>
  <component
    :is="rootTag"
    class="panel-shell"
    :class="[`panel-shell--${variant}`, { 'panel-shell--no-backdrop': noBackdrop }]"
    :data-testid="testId"
    @click.self="onBackdropClick"
  >
    <div
      class="panel-shell__card"
      :class="cardClass"
      role="dialog"
      :aria-label="title ?? undefined"
      @click.stop
    >
      <header
        v-if="hasHeader"
        class="panel-shell__header"
      >
        <slot name="header">
          <h3
            v-if="title"
            class="panel-shell__title"
          >
            {{ title }}
          </h3>
        </slot>
        <div
          v-if="$slots.actions || showCloseButton"
          class="panel-shell__actions"
        >
          <slot name="actions" />
          <button
            v-if="showCloseButton"
            type="button"
            class="panel-shell__close"
            aria-label="Close"
            data-testid="panel-shell-close"
            @click="handleClose"
          >
            &times;
          </button>
        </div>
      </header>

      <div class="panel-shell__body">
        <slot />
        <slot name="body" />
      </div>

      <footer
        v-if="$slots.footer"
        class="panel-shell__footer"
      >
        <slot name="footer" />
      </footer>
    </div>
  </component>
</template>

<script setup lang="ts">
import { computed, useSlots } from 'vue';

export type PanelShellVariant = 'overlay-fixed' | 'overlay-absolute' | 'embedded';

const props = withDefaults(
  defineProps<{
    /** Layout mode. `overlay-fixed` = full-viewport modal; `overlay-absolute` = parent-scoped overlay; `embedded` = inline section. */
    variant?: PanelShellVariant;
    /** Default header title text. Ignored if the `header` slot is used. */
    title?: string;
    /** `data-testid` applied to the root element. */
    testId?: string;
    /** Optional close callback. When provided, the × button renders in the header and clicking the backdrop also closes. Consumers must ALSO attach `@close` to re-emit upward (Vue does not auto-forward declared `on*` props). */
    onClose?: () => void;
    /** Disable the dim/blur backdrop for overlay variants. */
    noBackdrop?: boolean;
    /** Allow closing by clicking the backdrop. Defaults to true when `onClose` is provided. */
    closeOnBackdrop?: boolean;
    /** Extra class names applied to the card container. */
    cardClass?: string | string[] | Record<string, boolean>;
    /** Render the root as a specific tag. Defaults to `div` (overlay) or `section` (embedded). */
    as?: 'div' | 'section' | 'aside';
  }>(),
  {
    variant: 'overlay-fixed',
    title: undefined,
    testId: undefined,
    onClose: undefined,
    noBackdrop: false,
    closeOnBackdrop: undefined,
    cardClass: undefined,
    as: undefined,
  },
);

const emit = defineEmits<{
  (e: 'close'): void;
}>();

const slots = useSlots();

const rootTag = computed(() => {
  if (props.as) return props.as;
  return props.variant === 'embedded' ? 'section' : 'div';
});

const showCloseButton = computed(() => typeof props.onClose === 'function');

const hasHeader = computed(
  () => Boolean(slots.header) || Boolean(props.title) || Boolean(slots.actions) || showCloseButton.value,
);

const backdropEnabled = computed(() => {
  if (props.variant === 'embedded') return false;
  if (props.closeOnBackdrop !== undefined) return props.closeOnBackdrop;
  return showCloseButton.value;
});

function handleClose() {
  // Vue's runtime auto-invokes the `onClose` prop when we emit 'close',
  // so this single emit covers both: it triggers `props.onClose` AND
  // bubbles up as a 'close' event for `@close` listeners.
  emit('close');
}

function onBackdropClick() {
  if (backdropEnabled.value) handleClose();
}
</script>

<style scoped>
.panel-shell {
  box-sizing: border-box;
}

.panel-shell--overlay-fixed,
.panel-shell--overlay-absolute {
  inset: 0;
  z-index: 50;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 16px;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(4px);
}

.panel-shell--overlay-fixed {
  position: fixed;
}

.panel-shell--overlay-absolute {
  position: absolute;
}

.panel-shell--no-backdrop {
  background: transparent;
  backdrop-filter: none;
}

.panel-shell--embedded {
  display: block;
}

.panel-shell__card {
  display: flex;
  flex-direction: column;
  background: var(--ts-bg-panel);
  border: 1px solid var(--ts-border);
  border-radius: 12px;
  box-shadow: var(--ts-shadow-lg);
  overflow: hidden;
  width: min(480px, 100%);
  max-height: min(84vh, calc(100dvh - 32px));
  backdrop-filter: blur(20px);
}

.panel-shell--overlay-fixed[data-fullscreen="true"] .panel-shell__card,
.panel-shell--overlay-fixed.panel-shell--fullscreen .panel-shell__card {
  width: 100%;
  max-width: 100%;
  height: 100%;
  max-height: 100%;
  border-radius: 0;
}

.panel-shell--embedded .panel-shell__card {
  width: 100%;
  max-height: none;
  box-shadow: none;
  background: transparent;
  border: none;
  border-radius: 0;
  overflow: visible;
  backdrop-filter: none;
}

.panel-shell__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 16px 20px;
  border-bottom: 1px solid var(--ts-border);
  flex-shrink: 0;
}

.panel-shell--embedded .panel-shell__header {
  padding: 12px 0;
  border-bottom-color: var(--ts-border-subtle, var(--ts-border));
}

.panel-shell__title {
  margin: 0;
  font-size: 1.1rem;
  font-weight: 700;
  color: var(--ts-text-primary);
}

.panel-shell__actions {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-left: auto;
}

.panel-shell__close {
  background: none;
  border: none;
  color: var(--ts-text-secondary);
  cursor: pointer;
  font-size: 1.5rem;
  line-height: 1;
  padding: 4px 8px;
  border-radius: 6px;
  transition: color 0.2s ease, background 0.2s ease;
}

.panel-shell__close:hover {
  color: var(--ts-text-primary);
  background: var(--ts-bg-hover, rgba(255, 255, 255, 0.08));
}

.panel-shell__close:focus-visible {
  outline: 2px solid var(--ts-accent);
  outline-offset: 2px;
}

.panel-shell__body {
  padding: 20px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 16px;
  flex: 1 1 auto;
  min-height: 0;
}

.panel-shell--embedded .panel-shell__body {
  padding: 12px 0;
  overflow: visible;
}

.panel-shell__footer {
  padding: 12px 20px;
  border-top: 1px solid var(--ts-border);
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  flex-shrink: 0;
}

.panel-shell--embedded .panel-shell__footer {
  padding: 12px 0 0;
  border-top-color: var(--ts-border-subtle, var(--ts-border));
}

@media (max-width: 640px) {
  .panel-shell--overlay-fixed,
  .panel-shell--overlay-absolute {
    padding: 8px;
  }

  .panel-shell__card {
    width: 100%;
    max-height: calc(100dvh - 16px);
  }

  .panel-shell__header,
  .panel-shell__body {
    padding-left: 16px;
    padding-right: 16px;
  }
}
</style>
