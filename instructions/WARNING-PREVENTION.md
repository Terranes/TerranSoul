# Warning & Error Prevention Guide

> **Rule:** Every commit must be free of console warnings and errors in both
> browser mode (no Tauri) and desktop mode (Tauri). E2E tests must actively
> check for console errors during test flows.

---

## Known Warning Categories & How to Prevent Them

### 1. `Command <name> not found` — Tauri IPC in Browser Mode

**Cause:** Calling `invoke('some_command')` when running in the browser (no Tauri backend).

**Fix:** Always check for Tauri availability before calling `invoke()`:

```typescript
function isTauriAvailable(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

// ✅ Correct — browser fallback
if (isTauriAvailable()) {
  const result = await invoke<string>('my_command', { args });
} else {
  // Use browser-side alternative (free API client, localStorage, etc.)
}

// ❌ Wrong — will warn in browser mode
const result = await invoke<string>('my_command', { args });
```

**Common offenders:**
- `brain.ts: processPromptSilently` — must use `streamChatCompletion` in browser mode
- Any new Tauri command added without a browser fallback path

### 2. `injection "Symbol(route location)" not found` — Vue Router

**Cause:** Components or libraries (e.g. `@vercel/analytics`, `@vercel/speed-insights`)
calling `useRoute()` or `useRouter()` without a router being installed.

**Fix:** A minimal vue-router is installed in `main.ts` with `createMemoryHistory()`.
Do not remove it. If adding new libraries that use vue-router, ensure they work with
the memory history router.

### 3. `Cannot read properties of undefined` — Optional Chaining

**Cause:** Accessing nested properties on potentially-undefined objects, especially
in stores that load data asynchronously.

**Fix:** Always use optional chaining (`?.`) when accessing store state that may
not be loaded yet:

```typescript
// ✅ Correct
const len = settings.settings?.bgm_custom_tracks?.length ?? 0;

// ❌ Wrong — crashes if settings not loaded
const len = settings.settings.bgm_custom_tracks.length;
```

**Common offenders:**
- `skill-tree.ts: checkActive()` — settings may not be loaded when skill status is checked
- Any Pinia store watcher that reads from another store

### 4. `Unhandled error during watcher getter` — Store Watchers

**Cause:** A `watch()` dependency throws during evaluation.

**Fix:** Wrap watch dependencies in try-catch or use optional chaining:

```typescript
// ✅ Correct — safe getter
watch(
  () => {
    try { return store.someProperty?.length ?? 0; }
    catch { return 0; }
  },
  (newVal) => { /* ... */ }
);
```

### 5. Provider Migration Notices (Pollinations, etc.)

**Cause:** Free API providers sometimes return migration/deprecation notices
instead of actual LLM responses.

**Fix:** The `extractWarning()` + `applyWarningAsQuest()` functions in
`conversation.ts` detect these and convert them to actionable quest choices
(Upgrade/Switch/Dismiss). If a new warning pattern appears, add it to
`WARNING_PATTERNS`.

---

## E2E Test Rules for Warnings

### Every E2E test file MUST include console error collection

```typescript
function collectConsoleErrors(page: Page): string[] {
  const errors: string[] = [];
  page.on('console', msg => {
    if (msg.type() === 'error') errors.push(msg.text());
  });
  return errors;
}
```

### Filter out expected warnings

Some warnings are acceptable (e.g. Tauri IPC errors in browser mode when
properly caught). Filter them:

```typescript
const criticalErrors = errors.filter(e =>
  !e.includes('window.__TAURI_INTERNALS__') &&
  !e.includes('Failed to load resource') &&
  !e.includes('net::ERR_')
);
expect(criticalErrors).toHaveLength(0);
```

### 3D Model must always load

Add this check to any test that depends on the 3D character:

```typescript
// Wait for VRM to load (exposed on window by CharacterViewport)
await expect(async () => {
  const hasVrm = await page.evaluate(() => !!(window as any).__terransoul_vrm__);
  expect(hasVrm).toBe(true);
}).toPass({ timeout: 30_000 });
```

---

## Pre-Commit Checklist

Before every commit that touches stores, components, or IPC:

1. **Run `npm run dev`** — launches full Tauri app (kills existing processes first); check browser console for warnings
2. **Run `npx vitest run`** — all unit tests must pass
3. **Run `npx playwright test`** — all E2E tests must pass
4. **Check browser console** for:
   - `Command <X> not found` → add `isTauriAvailable()` guard
   - `Cannot read properties of undefined` → add `?.` optional chaining
   - `injection "Symbol(...)" not found` → check router is installed
   - Any new `console.warn` → investigate and fix or suppress intentionally

---

## Architecture Rule: Browser-First

TerranSoul must work fully in the browser (no Tauri) for:
- E2E testing via Playwright
- Web deployment (Vercel)
- Development without Rust toolchain

Every Tauri IPC call must have a browser-side fallback. The pattern is:

```
if (isTauriAvailable()) {
  // Tauri IPC path
} else {
  // Browser fallback (free API, localStorage, stub, etc.)
}
```

This is enforced in `conversation.ts` (sendMessage), `brain.ts` (processPromptSilently),
and all settings persistence stores.
