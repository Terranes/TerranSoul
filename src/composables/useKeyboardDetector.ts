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
 */
export function useKeyboardDetector() {
  const keyboardHeight = ref(0);
  const keyboardOpen = ref(false);

  /** Minimum shrink in px before we consider the keyboard open. */
  const KEYBOARD_THRESHOLD_PX = 80;

  function onVisualViewportResize() {
    const vv = window.visualViewport;
    if (!vv) return;
    const shrink = window.innerHeight - (vv.height + vv.offsetTop);
    if (shrink > KEYBOARD_THRESHOLD_PX) {
      keyboardHeight.value = shrink;
      keyboardOpen.value = true;
    } else {
      keyboardHeight.value = 0;
      keyboardOpen.value = false;
    }
  }

  onMounted(() => {
    if (window.visualViewport) {
      window.visualViewport.addEventListener('resize', onVisualViewportResize);
      window.visualViewport.addEventListener('scroll', onVisualViewportResize);
    }
  });

  onUnmounted(() => {
    if (window.visualViewport) {
      window.visualViewport.removeEventListener('resize', onVisualViewportResize);
      window.visualViewport.removeEventListener('scroll', onVisualViewportResize);
    }
  });

  return { keyboardHeight, keyboardOpen };
}
