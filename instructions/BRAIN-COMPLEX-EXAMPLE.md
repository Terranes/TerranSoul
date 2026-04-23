# Brain + Knowledge Quest + RAG — Walkthrough

> **TerranSoul v0.1** · Last verified: 2026-04-23
>
> Technical reference: [`BRAIN-COMPLEX-EXAMPLE-EXPLAIN.md`](BRAIN-COMPLEX-EXAMPLE-EXPLAIN.md)

The "Alice learns Vietnamese law" demo — 16 steps from Docker pre-flight
through Scholar's Quest ingestion to RAG-augmented chat. Driven by
[`scripts/verify-brain-flow.mjs`](../scripts/verify-brain-flow.mjs)
connecting to the running Tauri app via CDP.

```
107 passed · 0 failed · 0 skipped
```

### How to run

```powershell
docker start ollama                       # Ollama container with gemma3:4b
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=9222"
npm run tauri dev                         # Terminal 1
node scripts/verify-brain-flow.mjs        # Terminal 2
```

---

## Step 0 — Pre-flight (7 checks)

| Check | Assertion |
|---|---|
| Docker CLI | `docker --version` → `Docker version 28.3.2` |
| Ollama container | `docker ps` status starts with `"Up"` |
| Ollama API | `GET /api/tags` → 200 |
| Model installed | ≥ 1 model |
| Model tag | `gemma3:4b` |
| Model responds | `POST /api/chat` returns content |
| Tauri CDP | `GET :9222/json/version` → 200 |

---

## Step 1 — Fresh Launch (12 checks)

![Fresh launch](screenshots/01-fresh-launch.png)

420×700 Tauri window with mobile bottom nav (`<640px` breakpoint).

| Check | Selector | Expected |
|---|---|---|
| Chat view | `.chat-view` | visible |
| 3D viewport | `.viewport-layer` | visible |
| Input footer | `.input-footer` | visible |
| Navigation | `.mobile-bottom-nav` | visible |
| Nav labels | `.mobile-tab-label` | `["Chat","Quests","Memory","Market","Voice"]` |
| AI state pill | `.ai-state-pill` | visible |
| Quest orb | `.ff-orb` | visible |
| Mode toggle | `.mode-toggle-pill` | visible |
| Toggle label | `.mode-toggle-label` | `"Desktop"` |
| Chat input | `input.chat-input` | visible |
| Placeholder | `input.chat-input` | `"Type a message…"` |
| Send button | `button.send-btn` | visible |

---

## Step 2 — Brain → Local Ollama (8 checks)

![Brain configured](screenshots/02-brain-configured.png)

Auto-configures or Pinia-injects `local_ollama` mode. Warms up Ollama with a
direct `/api/chat` call.

| Check | Expected |
|---|---|
| Brain Pinia store | not null |
| `brainMode.mode` | `"local_ollama"` |
| `brainMode.model` | `"gemma3:4b"` |
| `ollamaStatus.running` | `true` |
| `ollamaStatus.model_count` | `1` |
| Brain status pill | visible |
| Pill text | `"Ollama · gemma3:4b"` |
| Brain overlay | hidden |

---

## Step 3 — Brain Component Verification (9 checks)

![Brain components](screenshots/03-brain-components.png)

Cross-checks Docker, Ollama `/api/show` metadata, Pinia state, and Tauri IPC.

| Check | Expected |
|---|---|
| Docker version | starts with `"Docker version"` |
| Container status | starts with `"Up"` |
| Model family | `"gemma3"` |
| Model params | `4.3B` |
| Model quantization | `Q4_K_M` |
| Pinia brainMode.mode | `"local_ollama"` |
| Pinia brainMode.model | `"gemma3:4b"` |
| Brain pill text | `"Ollama · gemma3:4b"` |
| Tauri IPC | `__TAURI_INTERNALS__` present |

---

## Step 4 — Alice Asks About Vietnamese Law (5 checks)

![Alice asks law](screenshots/04-alice-asks-law.png)

*"I want to learn about Vietnamese civil law on contract liability"* →
local Ollama responds (120 s timeout).

| Check | Expected |
|---|---|
| Input filled | contains `"Vietnamese civil law"` |
| Message sent | ✅ |
| User message row | `.message-row.user` ≥ 1 |
| Assistant response | `.message-row.assistant` ≥ 1 |
| Response content | length > 20 |

---

## Step 5 — Scholar's Quest Suggestion (5 checks)

![Quest suggestion](screenshots/05-quest-suggestion.png)

`maybeShowKnowledgeQuest()` detects `"learn about"` → injects quest message.

| Check | Expected |
|---|---|
| Quest suggestion | `questId === "scholar-quest"` |
| Quest ID | `"scholar-quest"` |
| Quest choices | ≥ 2 |
| Choice 1 | `"Start Knowledge Quest"` |
| Choice 2 | `"No thanks"` |

---

## Step 6 — Quest Choice Overlay (5 checks)

![Quest choice](screenshots/06-quest-choice-overlay.png)

| Check | Expected |
|---|---|
| Hotseat strip | visible |
| Question text | length > 5 |
| Tile labels | ≥ 2 |
| Start button | visible |
| Clicked | ✅ |

---

## Step 7 — Knowledge Quest Dialog (8 checks)

![KQ dialog](screenshots/07-knowledge-quest-dialog.png)

FF-style 4-step quest chain. Topic extracted from Alice's message.

| Check | Expected |
|---|---|
| `.kq-dialog` | visible |
| Header label | `"SCHOLAR'S QUEST"` |
| Title | contains topic |
| Step 1 | `"Verify Brain"` |
| Step 2 | `"Gather Sources"` |
| Step 3 | `"Learn"` |
| Step 4 | `"Ready"` |
| Active step count | 1 |

---

## Step 8 — Brain Verification (9 checks)

![Brain verification](screenshots/08-brain-verification.png)

Four animated checks; "Continue" activates when all pass.

| Check | Expected |
|---|---|
| Section title | `"🧠 Verifying Brain"` |
| Check count | 4 |
| Label 1 | `"Brain configured"` |
| Label 2 | `"LLM model loaded"` |
| Label 3 | `"Memory system ready"` |
| Label 4 | `"Ingestion engine online"` |
| All passed | ✅ icons ≥ 3 |
| Continue button | visible |
| Advanced to step 2 | ✅ |

---

## Step 9 — Gather Sources (10 checks)

![Gather sources](screenshots/09-gather-sources.png)

Two sources added:
- **URL**: `localhost:1420/demo/vietnamese-civil-code.html` (Articles 351–468)
- **File**: `article-429-commentary.txt`

| Check | Expected |
|---|---|
| Section title | `"📖 Gather Sources"` |
| URL input | visible |
| URL placeholder | `"https://example.com/document"` |
| Add URL button | visible |
| URL source added | `.kq-source-item` ≥ 1 |
| Source name | contains `"vietnamese-civil-code"` |
| File button | visible |
| File button text | `"📎 Attach File"` |
| File source added | `.kq-source-item` ≥ 2 |
| Start Learning button | visible |

---

## Step 10 — Learning in Progress (5 checks)

![Learning progress](screenshots/10-learning-progress.png)

Ingestion pipeline: URL fetch → HTML extraction → chunking (800/100) →
SHA256 dedup → Ollama embedding → SQLite storage.

| Check | Expected |
|---|---|
| Clicked "Start Learning" | ✅ |
| Section title | `"⚡ Learning in Progress"` |
| Task progress | visible |
| Progress bar | visible |
| Ingestion completed | ✅ |

---

## Step 11 — Knowledge Acquired (6 checks)

![Knowledge acquired](screenshots/11-knowledge-acquired.png)

| Check | Expected |
|---|---|
| Complete card | visible |
| Trophy icon | `"🏆"` |
| Section title | `"🎯 Knowledge Acquired!"` |
| Reward cards | 4 |
| Ask Questions button | visible |
| KQ dialog closed | not visible after click |

---

## Step 12 — RAG: Statute of Limitations (3 checks)

![RAG law answer](screenshots/12-alice-asks-law.png)

*"What is the statute of limitations for contract disputes under Vietnamese law?"*

| Check | Expected |
|---|---|
| Completion message | contains `"Scholar's Quest Complete"` |
| RAG response | length > 50 |
| References law content | mentions statute/limitation/contract/civil |

---

## Step 13 — RAG: Exemptions from Liability (4 checks)

![More law answers](screenshots/13-more-law-answers.png)

*"What are the exemptions from liability for breach of contract under Vietnamese civil code?"*

| Check | Expected |
|---|---|
| Second RAG response | length > 50 |
| References exemptions | mentions exemptions/force majeure/liability |
| Brain mode | still `"local_ollama"` |
| Brain model | still `"gemma3:4b"` |

---

## Step 14 — Skill Tree (7 checks)

![Skill tree](screenshots/14-skill-tree.png)

Navigate to Quests tab via `navTo(page, 'Quests')`.

| Check | Expected |
|---|---|
| Skill tree view | visible |
| Title | `"⚔️ Skill Tree"` |
| Brain Stat Sheet | visible |
| Sheet title | `"⚔ Brain Stat Sheet"` |
| Stat abbreviations | `["INT","WIS","CHA","PER","DEX","END"]` |
| Level badge | matches `/^Lv\. \d+$/` |
| Daily section | visible |

---

## Step 15 — Pet Mode (4 checks)

![Pet mode](screenshots/15-pet-mode.png)

Toggle pet mode → dismiss onboarding → click character → chat panel → Escape.

| Check | Expected |
|---|---|
| Pet overlay | visible |
| App shell `.pet-mode` | visible |
| Pet chat panel | visible |
| Exited pet mode | overlay gone after Escape |

---

## Summary

| Step | Checks | Description |
|---:|---:|---|
| 0 | 7 | Docker, Ollama, model, CDP |
| 1 | 12 | Fresh launch — viewport, nav, controls |
| 2 | 8 | Brain → local_ollama / gemma3:4b |
| 3 | 9 | Docker + model details + Tauri IPC |
| 4 | 5 | Alice asks about Vietnamese law |
| 5 | 5 | Scholar's Quest suggestion |
| 6 | 5 | Hotseat overlay → start quest |
| 7 | 8 | KQ dialog — header, steps |
| 8 | 9 | Brain verification — 4 checks |
| 9 | 10 | URL + file sources |
| 10 | 5 | Ingestion pipeline |
| 11 | 6 | Knowledge acquired — trophy |
| 12 | 3 | RAG: statute of limitations |
| 13 | 4 | RAG: exemptions from liability |
| 14 | 7 | Skill tree stats |
| 15 | 4 | Pet mode with chat |
| **Total** | **107** | **0 failed · 0 skipped** |
