/**
 * Memory View E2E — comprehensive test for the tiered memory UI.
 *
 * Runs against the Vite dev server (browser-only, no Tauri backend).
 * Memory CRUD operations (add/edit/delete) require Tauri IPC and are
 * exercised only when the backend is detected. UI rendering, navigation,
 * modal interactions, filter chips, and tab switching are tested regardless.
 *
 * Sections:
 *  1.  Memory view renders (header, tabs, filters, search)
 *  2.  Tab switching: List → Graph → Session → List
 *  3.  Type filter chips toggle
 *  4.  Tier filter chips toggle
 *  5.  Search input + Hybrid button present
 *  6.  Add memory modal opens/closes
 *  7.  Header action buttons visible (Extract, Summarize, Decay, GC)
 *  8.  Stats dashboard (conditional on Tauri)
 *  9.  Memory CRUD flow (conditional on Tauri)
 * 10.  No critical console errors
 */
import { test, expect } from '@playwright/test';
import {
  collectConsoleErrors,
  assertNoCrashErrors,
  waitForAppReady,
  getPiniaState,
  navigateToTab,
  TIMEOUTS,
} from './helpers';

test('memory: UI rendering and interaction flow', { timeout: 120_000 }, async ({ page }) => {
  const errors = collectConsoleErrors(page);
  await page.goto('/');
  await waitForAppReady(page);

  // Navigate to Memory tab
  await navigateToTab(page, 'Memory');
  const mv = page.locator('.memory-view');
  await expect(mv).toBeVisible({ timeout: TIMEOUTS.panel });

  // ── 1. Memory view renders ────────────────────────────────────────────
  // Header with title
  await expect(mv.locator('.mv-header h2')).toContainText('Memory');

  // Three tabs: List, Graph, Session
  const tabBtns = mv.locator('.mv-tab');
  await expect(tabBtns).toHaveCount(3);

  // List panel active by default
  await expect(mv.locator('.mv-list-panel')).toBeVisible();

  // ── 2. Tab switching ──────────────────────────────────────────────────
  // Switch to Graph tab
  await mv.locator('.mv-tab', { hasText: 'Graph' }).click();
  await expect(mv.locator('.mv-graph-panel')).toBeVisible({ timeout: TIMEOUTS.panel });
  // Graph container rendered
  await expect(mv.locator('.memory-graph')).toBeVisible();

  // Switch to Session tab
  await mv.locator('.mv-tab', { hasText: 'Session' }).click();
  await expect(mv.locator('.mv-session-panel')).toBeVisible({ timeout: TIMEOUTS.panel });
  await expect(mv.locator('.mv-session-hint')).toBeVisible();

  // Back to List tab
  await mv.locator('.mv-tab', { hasText: 'List' }).click();
  await expect(mv.locator('.mv-list-panel')).toBeVisible({ timeout: TIMEOUTS.panel });

  // ── 3. Type filter chips toggle ───────────────────────────────────────
  const filterRow = mv.locator('.mv-filter-row');
  const typeChips = filterRow.locator('.mv-type-chip');
  await expect(typeChips).toHaveCount(4);

  // Click 'fact' → active
  const factChip = filterRow.locator('.mv-type-chip', { hasText: 'fact' });
  await factChip.click();
  await expect(factChip).toHaveClass(/active/);

  // Click again → deactivated
  await factChip.click();
  await expect(factChip).not.toHaveClass(/active/);

  // ── 4. Tier filter chips toggle ───────────────────────────────────────
  const tierChips = filterRow.locator('.mv-tier-chip');
  await expect(tierChips).toHaveCount(3);

  // Click 'long' → active
  const longChip = filterRow.locator('.mv-tier-chip', { hasText: 'long' });
  await longChip.click();
  await expect(longChip).toHaveClass(/active/);

  // Click 'short' → switches (long deactivates, short activates)
  const shortChip = filterRow.locator('.mv-tier-chip', { hasText: 'short' });
  await shortChip.click();
  await expect(shortChip).toHaveClass(/active/);

  // Deselect all
  await shortChip.click();
  await expect(shortChip).not.toHaveClass(/active/);

  // ── 5. Search input + action buttons present ──────────────────────────
  const searchRow = mv.locator('.mv-search-row');
  const searchInput = searchRow.locator('.mv-search');
  await expect(searchInput).toBeVisible();
  await expect(searchInput).toHaveAttribute('placeholder', 'Search memories…');

  // Three search buttons: keyword, semantic, hybrid
  await expect(searchRow.locator('button', { hasText: '🔍 Search' })).toBeVisible();
  await expect(searchRow.locator('button', { hasText: '🤖 Semantic' })).toBeVisible();
  await expect(searchRow.locator('button', { hasText: '⚡ Hybrid' })).toBeVisible();

  // Type a query and click search — should not crash even without backend
  await searchInput.fill('test query');
  await searchRow.locator('button', { hasText: '🔍 Search' }).click();
  await page.waitForTimeout(300);

  // Hybrid search button should not crash
  await searchRow.locator('button', { hasText: '⚡ Hybrid' }).click();
  await page.waitForTimeout(300);

  // Clear search
  await searchInput.fill('');
  await searchInput.press('Enter');

  // ── 6. Add memory modal opens/closes ──────────────────────────────────
  const addBtn = mv.locator('button', { hasText: 'Add memory' });
  await expect(addBtn).toBeVisible();
  await addBtn.click();

  // Modal opens with correct title
  const modal = page.locator('.mv-modal');
  await expect(modal).toBeVisible({ timeout: TIMEOUTS.panel });
  await expect(modal.locator('h3')).toContainText('Add memory');

  // Modal has all form fields
  await expect(modal.locator('textarea')).toBeVisible();        // Content
  await expect(modal.locator('input').first()).toBeVisible();    // Tags
  await expect(modal.locator('select')).toBeVisible();           // Type dropdown
  await expect(modal.locator('input[type="range"]')).toBeVisible(); // Importance slider

  // Type dropdown has 4 options
  const options = modal.locator('select option');
  await expect(options).toHaveCount(4);

  // Save and Cancel buttons
  await expect(modal.locator('button', { hasText: 'Save' })).toBeVisible();
  await expect(modal.locator('button', { hasText: 'Cancel' })).toBeVisible();

  // Fill form (validates form interaction works)
  await modal.locator('textarea').fill('E2E test memory content');
  await modal.locator('input').first().fill('test, e2e');
  await modal.locator('select').selectOption('preference');
  await modal.locator('input[type="range"]').fill('4');

  // Cancel to close
  await modal.locator('button', { hasText: 'Cancel' }).click();
  await expect(modal).not.toBeVisible({ timeout: TIMEOUTS.panel });

  // Clicking backdrop closes modal too
  await addBtn.click();
  await expect(modal).toBeVisible({ timeout: TIMEOUTS.panel });
  await page.locator('.mv-modal-backdrop').click({ position: { x: 5, y: 5 } });
  await expect(modal).not.toBeVisible({ timeout: TIMEOUTS.panel });

  // ── 7. Header action buttons visible ──────────────────────────────────
  await expect(mv.locator('button', { hasText: 'Extract from session' })).toBeVisible();
  await expect(mv.locator('button', { hasText: 'Summarize session' })).toBeVisible();
  await expect(mv.locator('button', { hasText: 'Decay' })).toBeVisible();
  await expect(mv.locator('button', { hasText: 'GC' })).toBeVisible();

  // Decay and GC should not crash even without backend
  await mv.locator('button', { hasText: 'Decay' }).click();
  await page.waitForTimeout(500);
  await mv.locator('button', { hasText: 'GC' }).click();
  await page.waitForTimeout(500);

  // ── 8. Stats dashboard (conditional on Tauri backend) ─────────────────
  const hasTauri = await page.evaluate(() => '__TAURI_INTERNALS__' in window);
  if (hasTauri) {
    const statsPanel = mv.locator('.mv-stats');
    await expect(statsPanel).toBeVisible({ timeout: 5_000 });
    await expect(statsPanel.locator('.mv-stat')).toHaveCount(6);
    await expect(statsPanel.locator('.mv-stat-label', { hasText: 'Total' })).toBeVisible();
    await expect(statsPanel.locator('.mv-stat-label', { hasText: 'Short' })).toBeVisible();
    await expect(statsPanel.locator('.mv-stat-label', { hasText: 'Working' })).toBeVisible();
    await expect(statsPanel.locator('.mv-stat-label', { hasText: 'Long' })).toBeVisible();
    await expect(statsPanel.locator('.mv-stat-label', { hasText: 'Tokens' })).toBeVisible();
    await expect(statsPanel.locator('.mv-stat-label', { hasText: 'Avg Decay' })).toBeVisible();
  }

  // ── 9. Memory CRUD flow (conditional on Tauri backend) ────────────────
  if (hasTauri) {
    // Add a memory
    await addBtn.click();
    await expect(modal).toBeVisible({ timeout: TIMEOUTS.panel });
    await modal.locator('textarea').fill('E2E test: user loves TypeScript');
    await modal.locator('input').first().fill('test, e2e');
    await modal.locator('select').selectOption('preference');
    await modal.locator('input[type="range"]').fill('4');
    await modal.locator('button', { hasText: 'Save' }).click();
    await expect(modal).not.toBeVisible({ timeout: TIMEOUTS.panel });

    // Memory card appears
    const testCard = mv.locator('.mv-card', { hasText: 'E2E test' });
    await expect(testCard).toBeVisible({ timeout: 5_000 });

    // Card has expected elements
    await expect(testCard.locator('.mv-chip')).toContainText('preference');
    await expect(testCard.locator('.mv-tier-badge')).toContainText('long');
    const starsText = await testCard.locator('.mv-stars').textContent();
    expect(starsText).toBe('★★★★');
    await expect(testCard.locator('.mv-decay-bar')).toBeVisible();
    await expect(testCard.locator('.mv-tag', { hasText: 'test' })).toBeVisible();

    // Edit the memory
    await testCard.locator('button', { hasText: '✏' }).click();
    const editModal = page.locator('.mv-modal');
    await expect(editModal).toBeVisible({ timeout: TIMEOUTS.panel });
    await expect(editModal.locator('h3')).toContainText('Edit memory');
    await editModal.locator('textarea').fill('E2E EDITED: user loves Rust');
    await editModal.locator('button', { hasText: 'Save' }).click();
    await expect(editModal).not.toBeVisible({ timeout: TIMEOUTS.panel });
    await expect(mv.locator('.mv-card', { hasText: 'EDITED' })).toBeVisible({ timeout: 5_000 });

    // Search for the edited memory
    await searchInput.fill('Rust');
    await searchRow.locator('button', { hasText: '🔍 Search' }).click();
    await expect(mv.locator('.mv-card', { hasText: 'EDITED' })).toBeVisible({ timeout: 5_000 });
    await searchInput.fill('');
    await searchInput.press('Enter');

    // Type filter hides the memory
    await factChip.click();
    await expect(mv.locator('.mv-card', { hasText: 'EDITED' })).not.toBeVisible();
    await factChip.click(); // deselect

    // Tier filter shows the memory (tier=long)
    await longChip.click();
    await expect(mv.locator('.mv-card', { hasText: 'EDITED' })).toBeVisible();
    await longChip.click(); // deselect

    // Delete the memory
    page.on('dialog', (d) => d.accept());
    await mv.locator('.mv-card', { hasText: 'EDITED' }).locator('button', { hasText: '🗑' }).click();
    await expect(mv.locator('.mv-card', { hasText: 'EDITED' })).not.toBeVisible({ timeout: 5_000 });
  }

  // ── 10. No critical console errors ────────────────────────────────────
  assertNoCrashErrors(errors);
});
