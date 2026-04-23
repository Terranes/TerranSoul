/**
 * Format helpers shared across views.
 */

/**
 * Format a megabyte amount for display.
 * Values ≥ 1024 MB switch to GB with one decimal place.
 *
 * @example
 *   formatRam(512)  // "512 MB"
 *   formatRam(2048) // "2.0 GB"
 *   formatRam(0)    // "0 MB"
 */
export function formatRam(mb: number): string {
  if (mb >= 1024) return `${(mb / 1024).toFixed(1)} GB`;
  return `${mb} MB`;
}
