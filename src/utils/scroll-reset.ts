/**
 * iOS Safari scroll-reset utilities.
 *
 * iOS aggressively scrolls the page when an input is focused to keep it
 * visible above the virtual keyboard.  Even with `body { position: fixed }`
 * and `overflow: hidden`, the browser can still shift the visual viewport.
 *
 * These helpers forcefully reset scroll at multiple timing points to cover
 * the various moments iOS may apply its auto-scroll.
 */

/** Force all known scroll positions back to zero. */
export function resetAllScroll(): void {
  window.scrollTo(0, 0);
  document.documentElement.scrollTop = 0;
  document.body.scrollTop = 0;
}

/**
 * Fire a burst of scroll resets across multiple frames / timers.
 * iOS Safari can re-apply its scroll-to-input behavior *after* our
 * first reset, so we need multiple attempts at different timings.
 * The 300ms final reset covers the widest observed iOS timing window.
 */
export function burstResetScroll(): void {
  resetAllScroll();
  requestAnimationFrame(resetAllScroll);
  setTimeout(resetAllScroll, 50);
  setTimeout(resetAllScroll, 150);
  setTimeout(resetAllScroll, 300);
}
