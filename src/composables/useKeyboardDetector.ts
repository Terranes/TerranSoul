import { ref, onMounted, onUnmounted } from 'vue';
import { resetAllScroll, burstResetScroll } from '../utils/scroll-reset';

/**
 * Detects when the mobile virtual keyboard opens or closes using the
 * `visualViewport` API.
 *
 * When the keyboard is open:
 *  - `keyboardHeight` is the pixel height the keyboard occupies.
 *  - `keyboardOpen` is true.
 *
 * The `visualViewport` approach is reliable across Chrome/Safari on Android
 * and iOS 13+ (the latter ships it since iOS 13.0).  On desktop browsers
 * where the keyboard never appears, the values stay at 0/false.
 *
 * iOS Safari aggressively scrolls the page when an input is focused to keep
 * it visible above the keyboard.  We prevent this with a multi-layer strategy:
 *  1. `body { position: fixed }` in CSS (primary prevention)
 *  2. `burstResetScroll()` (covers race conditions)
 *  3. Continuous scroll listener that resets scroll while keyboard is open
 *  4. Direct scrollTop resets on html/body elements
 *
 * Reliability notes:
 *  - We cache the "full" viewport height (no keyboard) and only update it
 *    when the keyboard is confirmed closed.  This avoids miscalculations
 *    caused by iOS Safari changing `window.innerHeight` when the URL bar
 *    auto-hides/shows independently of the keyboard.
 *  - A focus-based polling fallback re-checks `visualViewport` after a
 *    short delay, catching cases where the initial resize event fires
 *    before the keyboard has fully expanded.
 */
export function useKeyboardDetector() {
  const keyboardHeight = ref(0);
  const keyboardOpen = ref(false);

  /**
   * Minimum shrink in px before we consider the keyboard open.
   * 80px is chosen to comfortably exceed address-bar-hide animations
   * (~20–40px on iOS/Android) without being so high that landscape
   * keyboards (~200px) are missed.
   */
  const KEYBOARD_THRESHOLD_PX = 80;

  /**
   * Cached "full" viewport height — the height when no keyboard is present.
   * Updated on mount and whenever the keyboard closes, so it tracks address-bar
   * changes without being corrupted by a partially-open keyboard.
   */
  let fullViewportHeight = 0;

  /** Timer for focus-based fallback polling. */
  let focusPollTimer: ReturnType<typeof setTimeout> | null = null;

  /** Compute keyboard offset using the cached baseline height. */
  function computeKeyboardState() {
    const vv = window.visualViewport;
    if (!vv) return;

    const shrink = fullViewportHeight - vv.height;
    if (shrink > KEYBOARD_THRESHOLD_PX) {
      keyboardHeight.value = shrink;
      keyboardOpen.value = true;
      burstResetScroll();
    } else {
      keyboardHeight.value = 0;
      keyboardOpen.value = false;
      // Update baseline when keyboard is confirmed closed — this tracks
      // address-bar height changes that happen while the keyboard was open.
      fullViewportHeight = Math.max(fullViewportHeight, vv.height);
      resetAllScroll();
    }
  }

  function onVisualViewportResize() {
    computeKeyboardState();
  }

  /**
   * Handle visualViewport scroll events.  Even with `position: fixed` on
   * body, iOS Safari can still shift the visual viewport.  If the viewport
   * has a non-zero offsetTop, immediately reset scroll to undo the shift.
   */
  function onVisualViewportScroll() {
    const vv = window.visualViewport;
    if (!vv) return;
    if (vv.offsetTop !== 0) {
      resetAllScroll();
    }
  }

  /**
   * Prevent iOS from auto-scrolling the page when the virtual keyboard
   * opens by resetting scroll on any scroll event that occurs while the
   * keyboard is active.
   */
  function onWindowScroll() {
    if (keyboardOpen.value) {
      resetAllScroll();
    }
  }

  /**
   * Called when a text input gains focus.  On iOS, the `visualViewport`
   * resize event can fire before the keyboard has fully expanded, producing
   * a partial or zero height.  We re-check after a short delay to pick up
   * the final keyboard size.
   */
  function onInputFocused() {
    if (focusPollTimer) clearTimeout(focusPollTimer);
    // Re-check after 300ms to catch the final keyboard height
    focusPollTimer = setTimeout(() => {
      computeKeyboardState();
      focusPollTimer = null;
    }, 300);
  }

  /**
   * Called when all text inputs have lost focus.  We clear any pending poll
   * since the keyboard should be closing.
   */
  function onInputBlurred() {
    if (focusPollTimer) {
      clearTimeout(focusPollTimer);
      focusPollTimer = null;
    }
  }

  onMounted(() => {
    // Capture the initial full viewport height before any keyboard appears.
    fullViewportHeight = window.visualViewport?.height ?? window.innerHeight;

    if (window.visualViewport) {
      window.visualViewport.addEventListener('resize', onVisualViewportResize);
      window.visualViewport.addEventListener('scroll', onVisualViewportScroll);
    }
    window.addEventListener('scroll', onWindowScroll);
  });

  onUnmounted(() => {
    if (window.visualViewport) {
      window.visualViewport.removeEventListener('resize', onVisualViewportResize);
      window.visualViewport.removeEventListener('scroll', onVisualViewportScroll);
    }
    window.removeEventListener('scroll', onWindowScroll);
    if (focusPollTimer) clearTimeout(focusPollTimer);
  });

  return { keyboardHeight, keyboardOpen, onInputFocused, onInputBlurred };
}
