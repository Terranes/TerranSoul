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

  function onVisualViewportResize() {
    const vv = window.visualViewport;
    if (!vv) return;
    // `vv.height` is the visible height of the visual viewport — it shrinks
    // when the keyboard opens.  `window.innerHeight` stays fixed at the full
    // layout viewport height, so their difference is the keyboard height.
    const shrink = window.innerHeight - vv.height;
    if (shrink > KEYBOARD_THRESHOLD_PX) {
      keyboardHeight.value = shrink;
      keyboardOpen.value = true;
      burstResetScroll();
    } else {
      keyboardHeight.value = 0;
      keyboardOpen.value = false;
      // Also reset when keyboard closes in case iOS left residual scroll.
      resetAllScroll();
    }
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

  onMounted(() => {
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
  });

  return { keyboardHeight, keyboardOpen };
}
