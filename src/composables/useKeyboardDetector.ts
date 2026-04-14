import { ref, onMounted, onUnmounted } from 'vue';

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
 * iOS Safari may still scroll the page upward to reveal the focused input
 * even with `interactive-widget=overlays-content`.  We counteract this by
 * forcing `window.scrollTo(0, 0)` whenever the keyboard is detected and
 * accounting for `visualViewport.offsetTop` in the height calculation.
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

      // iOS Safari scrolls the page upward to keep the focused element
      // visible when the keyboard opens.  This shifts the entire view
      // (including the 3D model) which we don't want — only the bottom
      // input panel should move.  Force scroll back to origin so the
      // viewport stays pinned and our translateY-based offset handles
      // the repositioning instead.
      window.scrollTo(0, 0);
    } else {
      keyboardHeight.value = 0;
      keyboardOpen.value = false;
    }
  }

  /**
   * Prevent iOS from auto-scrolling the page when the virtual keyboard
   * opens by resetting scroll on any scroll event that occurs while the
   * keyboard is active.
   */
  function onWindowScroll() {
    if (keyboardOpen.value) {
      window.scrollTo(0, 0);
    }
  }

  onMounted(() => {
    if (window.visualViewport) {
      window.visualViewport.addEventListener('resize', onVisualViewportResize);
      window.visualViewport.addEventListener('scroll', onVisualViewportResize);
    }
    window.addEventListener('scroll', onWindowScroll);
  });

  onUnmounted(() => {
    if (window.visualViewport) {
      window.visualViewport.removeEventListener('resize', onVisualViewportResize);
      window.visualViewport.removeEventListener('scroll', onVisualViewportResize);
    }
    window.removeEventListener('scroll', onWindowScroll);
  });

  return { keyboardHeight, keyboardOpen };
}
