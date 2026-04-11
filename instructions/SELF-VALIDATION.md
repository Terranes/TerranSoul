# Self-Validation & Video Recording

TerranSoul includes an automated self-validation system that exercises every
major feature and records a video proving the app works as expected. The video
is committed to the repository so anyone can review it without running the app.

---

## How It Works

1. A dedicated Playwright E2E test (`e2e/validate-and-record.spec.ts`) opens the
   app in a headless Chromium browser with **video recording enabled**.
2. The test walks through every core feature:
   - App layout loads (viewport + chat sections)
   - 3D canvas renders with real dimensions
   - Character state badge shows "idle"
   - Chat input is visible, enabled, and has the correct placeholder
   - Sending a message shows a user bubble and an assistant response
   - The Model Panel opens and closes correctly
   - A second message round-trip works
3. A helper script (`scripts/copy-validation-video.mjs`) copies the recorded
   video to `recording/validation.webm`, **replacing** any previous recording.
4. In CI, the video is committed back to the repository automatically.

---

## Where to Find the Video

The latest validation video is always at:

```
recording/validation.webm
```

Only one video file is kept — each new validation run replaces it.

---

## Running Locally

```bash
# Install Playwright browsers (first time only)
npx playwright install chromium --with-deps

# Run the self-validation with video recording
npm run test:validate
```

After the test completes, the video will be at `recording/validation.webm`.

---

## Running in CI

The `TerranSoul CI` workflow includes a **validation-recording** job that:

1. Runs after the `build-and-test` job passes.
2. Executes `npm run test:validate` which runs the Playwright validation test
   with video recording.
3. Copies the video to `recording/validation.webm`.
4. Commits and pushes the updated video back to the branch.

This happens automatically on every push to `main` or `copilot/**` branches and
on pull requests targeting `main`.

---

## What the Video Validates

| Step | What It Checks |
|------|---------------|
| 1 | App loads — `.chat-view`, `.viewport-section`, `.chat-section` visible |
| 2 | 3D viewport — `<canvas>` renders with width/height > 100px |
| 3 | State badge — Shows "idle" on startup |
| 4 | Chat input — Visible, enabled, correct placeholder, send button disabled when empty |
| 5 | Send message — User message appears, assistant responds, input clears |
| 6 | Model panel — Toggle opens/closes panel, header shows "3D Models" |
| 7 | Second message — Proves ongoing chat capability works |

---

## Verifying a Recording

Open `recording/validation.webm` in any browser or media player. You should see:

1. The TerranSoul app loading with a 3D viewport and chat area
2. The "idle" badge in the viewport
3. A message being typed and sent
4. User and assistant chat bubbles appearing
5. The Model Panel opening to show "3D Models" header, then closing
6. A second message exchange completing successfully

If any of these steps are missing or show errors, the validation test would have
failed — so a video's mere existence confirms the app passed all checks.
