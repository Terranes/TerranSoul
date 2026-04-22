# Brain + RAG Complex Setup Guide — Fresh User Walkthrough

> **TerranSoul v0.1** — Self-learning AI companion with persistent memory  
> Last updated: 2026-04-22  
> **See also**: [BRAIN-COMPLEX-EXAMPLE-EXPLAIN.md](BRAIN-COMPLEX-EXAMPLE-EXPLAIN.md) for architecture, schema, RAG pipeline, debugging, and scaling details.

---

## Table of Contents

1. [Quest-Guided Setup: Fresh User Journey](#quest-guided-setup-fresh-user-journey)
   - [Step 1: Fresh Launch — No Brain Yet](#step-1-fresh-launch--no-brain-yet)
   - [Step 2: Alex Asks About Law PDFs](#step-2-alex-asks-about-law-pdfs)
   - [Step 3: Quest System Activates](#step-3-quest-system-activates)
   - [Step 4: Brain Auto-Setup via Quest](#step-4-brain-auto-setup-via-quest)
   - [Step 5: First Chat — Works But Vague](#step-5-first-chat--works-but-vague)
   - [Step 6: Long-Term Memory Quest Unlocks](#step-6-long-term-memory-quest-unlocks)
   - [Step 7: Alex Adds PDF Knowledge](#step-7-alex-adds-pdf-knowledge)
   - [Step 8: Sage's Library Quest Auto-Activates RAG](#step-8-sages-library-quest-auto-activates-rag)
   - [Step 9: Same Question — Now with Precise Answers](#step-9-same-question--now-with-precise-answers)
   - [Step 10: Multi-Source Updates with LLM Conflict Detection](#step-10-multi-source-updates-with-llm-conflict-detection)
   - [Complete Quest Progression](#complete-quest-progression)
2. [Setup Walkthrough (Screenshots)](#setup-walkthrough-screenshots)
3. [Real-World Example: Law Firm Knowledge Base](#real-world-example-law-firm-knowledge-base)
   - [The Scenario](#the-scenario)
   - [Step-by-Step: First Day Setup](#step-by-step-first-day-setup)
   - [Daily Sync: Credential-Based Source Ingestion](#daily-sync-credential-based-source-ingestion)
   - [Handling Duplicates Across Sources](#handling-duplicates-across-sources)
   - [Detecting and Removing Stale/Out-of-Date Knowledge](#detecting-and-removing-staleout-of-date-knowledge)
   - [Day-in-the-Life: Attorney Sarah Uses TerranSoul](#day-in-the-life-attorney-sarah-uses-terransoul)

---

## Quest-Guided Setup: Fresh User Journey

> **Meet Alex** — a 2nd-year law student who just downloaded TerranSoul.
> Alex has hundreds of pages of family law PDFs and wants an AI companion
> that can answer questions from those documents. Alex has never configured
> an LLM or heard of "RAG."
>
> TerranSoul's quest system will guide Alex step-by-step — no manual
> configuration needed.

### Step 1: Fresh Launch — No Brain Yet

Alex opens TerranSoul for the first time. The 3D character appears with
an idle animation, but no brain is configured. The chat input is disabled.

```
┌─────────────────────────────────────────────────────────┐
│                    TerranSoul                            │
│                                                          │
│         [ 3D VRM Character — Idle Animation ]            │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │                                                    │  │
│  │  🤖  Hello! I'm your AI companion.                 │  │
│  │      I need a brain before I can chat!             │  │
│  │                                                    │  │
│  │      Check out the ✨ Quest panel to get started.  │  │
│  │                                                    │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │ Type a message…  (disabled — no brain)   [Send ➤]  │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  ┌──────────────────────────┐                            │
│  │ ✨ Quest Available!       │ ← pulsing badge           │
│  │ "Awaken the Mind"        │                            │
│  └──────────────────────────┘                            │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

> **What Alex sees**: A friendly character that can't chat yet, and a
> glowing quest badge that says "Awaken the Mind." No confusing settings,
> no API key forms — just a quest to follow.

### Step 2: Alex Asks About Law PDFs

Alex tries to type a message but the input is disabled. A tooltip says
"Complete the 'Awaken the Mind' quest to enable chat." Alex clicks the
quest badge instead.

```
┌─────────────────────────────────────────────────────────┐
│                    TerranSoul                            │
│                                                          │
│         [ 3D VRM Character — Pointing Animation ]        │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │                                                    │  │
│  │  🤖  I see you want to chat! Let me help you       │  │
│  │      set up my brain first.                        │  │
│  │                                                    │  │
│  │      I can learn from PDFs, websites, and your     │  │
│  │      conversations — but first I need an AI        │  │
│  │      engine to think with.                         │  │
│  │                                                    │  │
│  │      Let's complete the "Awaken the Mind" quest!   │  │
│  │                                                    │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │ ✨ QUEST: Awaken the Mind                          │  │
│  │                                                    │  │
│  │ Your companion needs a brain to think.             │  │
│  │ Choose how to power it:                            │  │
│  │                                                    │  │
│  │ ☁️  Free Cloud API (instant, no setup)             │  │
│  │ 💳 Paid Cloud API (OpenAI, Anthropic)              │  │
│  │ 🖥  Local Ollama (private, offline)                │  │
│  │                                                    │  │
│  │           [ Start Quest → ]                        │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

> **Design insight**: Alex never sees "API endpoint", "model tag", or
> "provider configuration." The quest system translates technical choices
> into a simple 3-option menu. The character guides them with personality.

### Step 3: Quest System Activates

Alex clicks "Start Quest" and chooses "Free Cloud API" (the easiest path).
The quest system auto-detects that no brain is configured and presents the
"Awaken the Mind" quest from the skill tree.

```
┌─────────────────────────────────────────────────────────┐
│                    TerranSoul                            │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │ ✨ QUEST: Awaken the Mind                          │  │
│  │                                                    │  │
│  │ Step 1 of 2: Choose your brain type                │  │
│  │                                                    │  │
│  │  ┌──────────────────────────────────────────────┐  │  │
│  │  │ ☁️  Free Cloud API              ← SELECTED   │  │  │
│  │  │ Use Pollinations AI — no API key needed      │  │  │
│  │  │ Instant setup · Rate-limited · Good quality  │  │  │
│  │  └──────────────────────────────────────────────┘  │  │
│  │                                                    │  │
│  │  ┌──────────────────────────────────────────────┐  │  │
│  │  │ 💳 Paid Cloud API                            │  │  │
│  │  │ OpenAI / Anthropic / Groq with your API key  │  │  │
│  │  └──────────────────────────────────────────────┘  │  │
│  │                                                    │  │
│  │  ┌──────────────────────────────────────────────┐  │  │
│  │  │ 🖥  Local Ollama                              │  │  │
│  │  │ Fully private, runs on your hardware         │  │  │
│  │  └──────────────────────────────────────────────┘  │  │
│  │                                                    │  │
│  │ Progress: [████░░░░░░] 50%                         │  │
│  │                                      [ Next → ]    │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

> **Behind the scenes**: The quest system calls `useBrainStore().setProvider('free')`
> and auto-configures the Pollinations endpoint. Alex never sees a URL or config file.

### Step 4: Brain Auto-Setup via Quest

Alex clicks "Next" and the quest auto-configures the free cloud API.
The quest completes with a celebration animation.

```
┌─────────────────────────────────────────────────────────┐
│                    TerranSoul                            │
│                                                          │
│         [ 3D VRM Character — Celebration Animation ]     │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │                                                    │  │
│  │                    🎉                               │  │
│  │          Quest Complete!                            │  │
│  │          "Awaken the Mind"                          │  │
│  │                                                    │  │
│  │  Your companion now has a brain!                   │  │
│  │  Using: Free Cloud API (Pollinations)              │  │
│  │                                                    │  │
│  │  ┌──────────────────────────────────────────────┐  │  │
│  │  │ 🏆 Reward: +50 XP                           │  │  │
│  │  │ 🔓 Unlocked: Chat capability                 │  │  │
│  │  │ 🔓 Unlocked: "Long-Term Memory" quest        │  │  │
│  │  └──────────────────────────────────────────────┘  │  │
│  │                                                    │  │
│  │              [ Start chatting → ]                   │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  ┌──────────────────────────┐                            │
│  │ ✨ New Quest Available!   │ ← pulsing badge           │
│  │ "Long-Term Memory"       │                            │
│  └──────────────────────────┘                            │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

> **Skill tree auto-detection**: The `brain-online` skill in `skill-tree.ts`
> checks `brainStore.isConfigured` — once the quest sets the provider, the
> skill activates automatically and unlocks the next quest chain.

### Step 5: First Chat — Works But Vague

Alex can now chat! They ask about family law filing deadlines from their
PDF materials. The brain responds, but the answer is generic — it has
no knowledge of Alex's specific documents.

```
┌─────────────────────────────────────────────────────────┐
│                    TerranSoul                            │
│                                                          │
│         [ 3D VRM Character — Thinking Animation ]        │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │                                                    │  │
│  │  You:  What's the deadline for responding to a     │  │
│  │        family law motion in Cook County?            │  │
│  │                                                    │  │
│  │  ────────────────────────────────────────────────  │  │
│  │                                                    │  │
│  │  🤖:  Generally, in most jurisdictions, you have   │  │
│  │       about 30 days to respond to a motion, but    │  │
│  │       specific deadlines vary by local rules.      │  │
│  │       I'd recommend checking your local court      │  │
│  │       rules for exact deadlines.                   │  │
│  │                                                    │  │
│  │       💡 Tip: I can give much better answers if    │  │
│  │       you teach me your specific knowledge!        │  │
│  │       Check the "Long-Term Memory" quest.          │  │
│  │                                                    │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │ Type a message…                          [Send ➤]  │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

> **The problem**: The LLM gives a generic answer because it has no
> memories. It doesn't know about Cook County's specific Rule 14.3
> or Alex's PDF materials. This is the "no RAG" experience.

### Step 6: Long-Term Memory Quest Unlocks

Alex notices the new quest badge and clicks it. The "Long-Term Memory"
quest teaches Alex how to add knowledge that persists between sessions.

```
┌─────────────────────────────────────────────────────────┐
│                    TerranSoul                            │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │ ✨ QUEST: Long-Term Memory                         │  │
│  │                                                    │  │
│  │ Your companion can remember facts permanently!     │  │
│  │                                                    │  │
│  │ Right now, everything is forgotten when you close  │  │
│  │ the app. Let's fix that.                           │  │
│  │                                                    │  │
│  │ Step 1: Go to the Memory tab (🧠 icon)            │  │
│  │ Step 2: Click "＋ Add memory"                       │  │
│  │ Step 3: Type a fact you want remembered            │  │
│  │                                                    │  │
│  │ Try adding something from your study materials!    │  │
│  │                                                    │  │
│  │ Progress: [░░░░░░░░░░] 0%                          │  │
│  │                                                    │  │
│  │                              [ Go to Memory → ]    │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

### Step 7: Alex Adds PDF Knowledge

Alex navigates to the Memory tab and starts adding facts from their
family law PDFs. Each memory is stored in SQLite and auto-embedded.

```
┌─────────────────────────────────────────────────────────┐
│  🧠 Memory            [⬇ Extract] [📄 Sum] [＋ Add]     │
│                                                          │
│  [Graph]  [List] ← active  [Session]                     │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │ ＋ Add New Memory                                    │  │
│  │                                                    │  │
│  │ Content:                                           │  │
│  │ ┌──────────────────────────────────────────────┐   │  │
│  │ │ Cook County Family Law Rule 14.3: Responses  │   │  │
│  │ │ to motions must be filed within 30 days of   │   │  │
│  │ │ service. Filing must include proof of service │   │  │
│  │ │ and use the e-filing system.                  │   │  │
│  │ └──────────────────────────────────────────────┘   │  │
│  │                                                    │  │
│  │ Tags: [ court-rules ] [ cook-county ] [ family ]   │  │
│  │ Importance: ★★★★★ (5)                              │  │
│  │ Type: [fact ▾]                                     │  │
│  │                                                    │  │
│  │                             [ Save Memory ✓ ]      │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │ fact          ★★★★★                 (just added!)  │  │
│  │ Cook County Family Law Rule 14.3: Responses to     │  │
│  │ motions must be filed within 30 days of service    │  │
│  │ [ court-rules ] [ cook-county ] [ family ]         │  │
│  │ 2026-04-22                              [✏] [🗑]  │  │
│  ├────────────────────────────────────────────────────┤  │
│  │ fact          ★★★★☆                                │  │
│  │ Section 7.2: All motions require a certificate     │  │
│  │ of service filed simultaneously                    │  │
│  │ [ procedure ] [ motions ] [ service ]              │  │
│  │ 2026-04-22                              [✏] [🗑]  │  │
│  ├────────────────────────────────────────────────────┤  │
│  │ fact          ★★★★☆                                │  │
│  │ Parenting time modifications: must show material   │  │
│  │ change in circumstances (In re Marriage of...)     │  │
│  │ [ family-law ] [ parenting ] [ modification ]      │  │
│  │ 2026-04-22                              [✏] [🗑]  │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  ✨ Quest progress: 3 memories added [████████░░] 80%    │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

> **Behind the scenes**: Each memory is auto-embedded via the active LLM
> provider (or Ollama if available). The 768-dim vector is stored as a BLOB
> in SQLite alongside the text. See [BRAIN-COMPLEX-EXAMPLE-EXPLAIN.md](BRAIN-COMPLEX-EXAMPLE-EXPLAIN.md#embedding-model) for details.

### Step 8: Sage's Library Quest Auto-Activates RAG

After Alex adds 5+ memories, the "Long-Term Memory" quest completes
and the "Sage's Library" skill auto-activates. This means RAG (memory
retrieval) is now active for every chat message.

```
┌─────────────────────────────────────────────────────────┐
│                    TerranSoul                            │
│                                                          │
│         [ 3D VRM Character — Enlightened Animation ]     │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │                                                    │  │
│  │                    🎉                               │  │
│  │          Quest Complete!                            │  │
│  │          "Long-Term Memory"                         │  │
│  │                                                    │  │
│  │  Your companion now remembers between sessions!    │  │
│  │                                                    │  │
│  │  ┌──────────────────────────────────────────────┐  │  │
│  │  │ 🏆 Reward: +75 XP                           │  │  │
│  │  │ 🔓 Unlocked: RAG memory retrieval            │  │  │
│  │  │ 🧠 Auto-activated: "Sage's Library" skill    │  │  │
│  │  │                                              │  │  │
│  │  │ Your companion now searches memories before  │  │  │
│  │  │ every response. Ask your law questions again! │  │  │
│  │  └──────────────────────────────────────────────┘  │  │
│  │                                                    │  │
│  │              [ Try it out → ]                       │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  ┌──────────────────────────────────────────────────────┐│
│  │ SKILL TREE                                           ││
│  │                                                      ││
│  │  [Awaken Mind ✓]──>[Long-Term Memory ✓]              ││
│  │                          │                           ││
│  │                    [Sage's Library ✓] ← AUTO         ││
│  │                                                      ││
│  │  "Sage's Library" activates when brain is online     ││
│  │  AND memories exist. RAG now runs on every message.  ││
│  └──────────────────────────────────────────────────────┘│
│                                                          │
└─────────────────────────────────────────────────────────┘
```

> **Skill auto-detection**: The `rag-knowledge` skill in `skill-tree.ts`
> checks `brainStore.isConfigured && memoryStore.entries.length > 0`.
> Once both conditions are true, the skill activates automatically —
> Alex never has to toggle a "RAG" switch or understand what it means.

### Step 9: Same Question — Now with Precise Answers

Alex asks the same question as Step 5. This time, TerranSoul searches
memories before responding and gives a precise, sourced answer.

```
┌─────────────────────────────────────────────────────────┐
│                    TerranSoul                            │
│                                                          │
│         [ 3D VRM Character — Confident Animation ]       │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │                                                    │  │
│  │  You:  What's the deadline for responding to a     │  │
│  │        family law motion in Cook County?            │  │
│  │                                                    │  │
│  │  ────────────────────────────────────────────────  │  │
│  │                                                    │  │
│  │  🤖:  Per Cook County Family Law Rule 14.3, you    │  │
│  │       have **30 days from service** to file your   │  │
│  │       response.                                    │  │
│  │                                                    │  │
│  │       Requirements:                                │  │
│  │       • Filing must include proof of service       │  │
│  │       • Must use the court's e-filing system       │  │
│  │       • Section 7.2: All motions require a         │  │
│  │         certificate of service filed simultaneously│  │
│  │                                                    │  │
│  │       📚 Sources: 3 memories used                  │  │
│  │                                                    │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │ Type a message…                          [Send ➤]  │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

**What happened behind the scenes:**

```
1. User's message embedded: embed_text("What's the deadline...") → 768-dim vector
2. vector_search() scanned all memory embeddings in <5ms (pure cosine similarity)
3. Top 3 most relevant memories injected into system prompt:

   [LONG-TERM MEMORY]
   - Cook County Family Law Rule 14.3: Responses to motions
     must be filed within 30 days of service...
   - Section 7.2: All motions require a certificate of service...
   - Parenting time modifications: must show material change...
   [/LONG-TERM MEMORY]

4. LLM generated the response with full context — specific and accurate
```

> **The transformation**: Compare Step 5 ("check your local court rules")
> vs Step 9 ("Per Cook County Rule 14.3, 30 days from service"). Same
> question, completely different quality — all because of the memories
> Alex added during the quest.

### Step 10: Multi-Source Updates with LLM Conflict Detection

Weeks later, Alex learns that Rule 14.3 has been amended — the deadline
changed from 30 to 21 days. Alex adds the new information:

```
┌─────────────────────────────────────────────────────────┐
│                    TerranSoul                            │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │                                                    │  │
│  │  You:  Remember this: Cook County amended Rule     │  │
│  │        14.3 effective April 2026 — the deadline    │  │
│  │        for responding to family law motions is     │  │
│  │        now 21 days (reduced from 30).              │  │
│  │                                                    │  │
│  │  ────────────────────────────────────────────────  │  │
│  │                                                    │  │
│  │  🤖:  Got it! I noticed this conflicts with an    │  │
│  │       existing memory:                             │  │
│  │                                                    │  │
│  │       ⚠️ CONFLICT DETECTED:                        │  │
│  │       OLD: "...filed within 30 days of service"   │  │
│  │       NEW: "...deadline is now 21 days"            │  │
│  │                                                    │  │
│  │       The new information supersedes the old rule. │  │
│  │       I've updated my knowledge:                   │  │
│  │       ✓ Marked old Rule 14.3 (30 days) as expired │  │
│  │       ✓ Stored new Rule 14.3 (21 days) as active  │  │
│  │                                                    │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

> **How conflict detection works**: When the new memory's embedding has
> high cosine similarity (0.90–0.97) with an existing memory but the
> content differs, TerranSoul uses the LLM to analyze which version is
> current. See [Multi-Source Knowledge Management](BRAIN-COMPLEX-EXAMPLE-EXPLAIN.md#multi-source-knowledge-management)
> in the technical reference.

### Complete Quest Progression

```
Alex's Journey Through the Quest System:
─────────────────────────────────────────

Day 1, Minute 0:
  [Awaken the Mind]          Status: AVAILABLE
  [Long-Term Memory]         Status: LOCKED
  [Sage's Library]           Status: LOCKED

Day 1, Minute 2:  (chose Free Cloud API)
  [Awaken the Mind]     ✅   Status: COMPLETED (+50 XP)
  [Long-Term Memory]         Status: AVAILABLE ← unlocked!
  [Sage's Library]           Status: LOCKED

Day 1, Minute 10:  (added 5 memories from PDFs)
  [Awaken the Mind]     ✅   Status: COMPLETED
  [Long-Term Memory]    ✅   Status: COMPLETED (+75 XP)
  [Sage's Library]      ✅   Status: AUTO-ACTIVATED ← RAG enabled!

Day 1, Minute 11:  (asks same question — gets precise answer)
  Alex: "Wow, that's exactly the rule from my PDF!"

Week 3:  (rule amendment detected)
  Conflict detected → LLM resolves → knowledge updated automatically

SKILL TREE VISUALIZATION:

  ┌─────────────┐     ┌───────────────────┐     ┌──────────────┐
  │ Awaken the  │────>│ Long-Term Memory  │────>│ Sage's       │
  │ Mind     ✅ │     │               ✅  │     │ Library   ✅ │
  │             │     │                   │     │              │
  │ Brain setup │     │ Add 5+ memories   │     │ AUTO: brain  │
  │ via quest   │     │ from PDFs/notes   │     │ + memories   │
  └─────────────┘     └───────────────────┘     └──────────────┘
        │                                              │
        │         ┌───────────────────┐                │
        └────────>│ Combo: Scholar 📚 │<───────────────┘
                  │ Brain + Memory +  │
                  │ RAG all active    │
                  │ Bonus: +100 XP    │
                  └───────────────────┘
```

> **Total time from download to RAG-powered answers: ~10 minutes.**
> Zero configuration files edited. Zero API keys entered. Zero
> understanding of embeddings, vectors, or cosine similarity required.

---

## Setup Walkthrough (Screenshots)

> For users who prefer the manual setup path instead of quests,
> or want to use Local Ollama for full privacy.

### Screenshot 1: First Launch — Brain Setup Overlay (ChatView)

When you first open TerranSoul, the 3D character is visible but a Brain Setup
card floats on top. This is the quick-start path from `ChatView.vue`:

```
┌─────────────────────────────────────────────────────────┐
│                    TerranSoul                            │
│                                                          │
│         [ 3D VRM Character — Idle Animation ]            │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │  🧠  Set up your Brain                             │  │
│  │                                                    │  │
│  │  Start chatting instantly with a free cloud LLM:   │  │
│  │                                                    │  │
│  │        [ ☁️ Use Free Cloud API (no setup) ]        │  │
│  │                                                    │  │
│  │  Intel i7 · 65 GB RAM                              │  │
│  │  Or run locally: gemma3:12b-it-qat                 │  │
│  │    Fast, private, runs on your hardware            │  │
│  │                                                    │  │
│  │  ┌──────────────┐ ┌──────────────┐ ┌───────────┐  │  │
│  │  │gemma3:12b ⭐ │ │ phi4:14b     │ │llama3.3   │  │  │
│  │  └──────────────┘ └──────────────┘ └───────────┘  │  │
│  │                                                    │  │
│  │  ❌ Ollama not running — start it first            │  │
│  │     (ollama serve)               [ 🔄 Retry ]     │  │
│  │                                                    │  │
│  │       [ ⬇ Install & activate gemma3:12b ]          │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

> **For privacy-sensitive use cases (law firms, medical, finance)**: Choose the
> local Ollama path. Zero data ever leaves the machine.

### Screenshot 2: Brain Setup Wizard (BrainSetupView — Full Setup)

For more control, navigate to Settings → Brain Setup. This is a multi-step wizard:

```
┌─────────────────────────────────────────────────────────┐
│  Brain Setup                                             │
│                                                          │
│  Step: (●)─────(○)─────(○)─────(○)─────(○)              │
│        Tier   Hardware  Model  Ollama  Download          │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │         🧠 Choose how to power your Brain          │  │
│  │                                                    │  │
│  │  TerranSoul needs an AI brain for conversations.   │  │
│  │  Choose how you'd like to connect it.              │  │
│  │                                                    │  │
│  │  ┌──────────────────────────────────────────────┐  │  │
│  │  │ ☁️  Free Cloud API                           │  │  │
│  │  │ Use free LLM providers (Groq, Cerebras)      │  │  │
│  │  │ No API key needed · Rate-limited              │  │  │
│  │  │                           [ Instant — no setup]│  │  │
│  │  └──────────────────────────────────────────────┘  │  │
│  │                                                    │  │
│  │  ┌──────────────────────────────────────────────┐  │  │
│  │  │ 💳 Paid Cloud API                            │  │  │
│  │  │ OpenAI, Anthropic, or custom endpoint        │  │  │
│  │  │ Best quality · Requires API key               │  │  │
│  │  └──────────────────────────────────────────────┘  │  │
│  │                                                    │  │
│  │  ┌──────────────────────────────────────────────┐  │  │
│  │  │ 🖥  Local LLM (Ollama)           ← SELECTED  │  │  │
│  │  │ Run locally, fully private, no internet       │  │  │
│  │  │ Requires Ollama installed · Best for privacy  │  │  │
│  │  └──────────────────────────────────────────────┘  │  │
│  │                                                    │  │
│  │                                      [ Next → ]    │  │
│  └────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### Screenshot 3: Hardware Analysis (Step 2)

```
┌─────────────────────────────────────────────────────────┐
│  Brain Setup                                             │
│                                                          │
│  Step: (✓)─────(●)─────(○)─────(○)─────(○)              │
│        Tier   Hardware  Model  Ollama  Download          │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │              🖥 Local LLM Setup                     │  │
│  │                                                    │  │
│  │  We'll analyse your hardware and recommend the     │  │
│  │  best model for your machine.                      │  │
│  │                                                    │  │
│  │  💾 RAM    65 GB (High-End)                        │  │
│  │  🖥 CPU    Intel i7-12700K · 12 cores              │  │
│  │  🗂 OS     Windows 11 (x86_64)                     │  │
│  │                                                    │  │
│  │               [ ← Back ]  [ Next → ]               │  │
│  └────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### Screenshot 4: Choose Model (Step 3)

```
┌─────────────────────────────────────────────────────────┐
│  Brain Setup                                             │
│                                                          │
│  Step: (✓)─────(✓)─────(●)─────(○)─────(○)              │
│        Tier   Hardware  Model  Ollama  Download          │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │          Choose your Brain                          │  │
│  │  Based on your 65 GB of RAM, we recommend:         │  │
│  │                                                    │  │
│  │  ┌──────────────────────────────────────────────┐  │  │
│  │  │ ⭐ gemma3:12b-it-qat            ← SELECTED  │  │  │
│  │  │    Fast and accurate for conversations       │  │  │
│  │  │    Requires 8 GB RAM · tag: gemma3:12b-it-qat│  │  │
│  │  └──────────────────────────────────────────────┘  │  │
│  │  ┌──────────────────────────────────────────────┐  │  │
│  │  │   phi4:14b                                   │  │  │
│  │  │   Strong reasoning, good for analysis        │  │  │
│  │  │   Requires 10 GB RAM · tag: phi4:14b         │  │  │
│  │  └──────────────────────────────────────────────┘  │  │
│  │  ┌──────────────────────────────────────────────┐  │  │
│  │  │   ☁️ kimi-k2.6  (Cloud)                      │  │  │
│  │  │   Cloud-routed · no local RAM needed         │  │  │
│  │  └──────────────────────────────────────────────┘  │  │
│  │                                                    │  │
│  │               [ ← Back ]  [ Next → ]               │  │
│  └────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### Screenshot 5: Ollama Status Check (Step 4)

```
┌─────────────────────────────────────────────────────────┐
│  Brain Setup                                             │
│                                                          │
│  Step: (✓)─────(✓)─────(✓)─────(●)─────(○)              │
│        Tier   Hardware  Model  Ollama  Download          │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │              Check Ollama                           │  │
│  │                                                    │  │
│  │  TerranSoul uses Ollama to run models locally.     │  │
│  │  It must be running before we can download.        │  │
│  │                                                    │  │
│  │           ✅ Ollama is running                      │  │
│  │                                                    │  │
│  │          [ ← Back ] [ 🔄 Retry ] [ Next → ]        │  │
│  └────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### Screenshot 6: Download & Activate (Step 5)

```
┌─────────────────────────────────────────────────────────┐
│  Brain Setup                                             │
│                                                          │
│  Step: (✓)─────(✓)─────(✓)─────(✓)─────(●)              │
│        Tier   Hardware  Model  Ollama  Download          │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │        Download gemma3:12b-it-qat                   │  │
│  │                                                    │  │
│  │  This will download the model via Ollama.          │  │
│  │  It may take several minutes.                      │  │
│  │                                                    │  │
│  │           🔄 Downloading… this may take             │  │
│  │              a few minutes.                         │  │
│  │                                                    │  │
│  │                          [ ← Back ]                 │  │
│  └────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### Screenshot 7: Brain Connected! (Done)

```
┌─────────────────────────────────────────────────────────┐
│  Brain Setup                                             │
│                                                          │
│  Step: (✓)─────(✓)─────(✓)─────(✓)─────(✓)              │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │                    🎉                               │  │
│  │             Brain connected!                        │  │
│  │                                                    │  │
│  │  Using gemma3:12b-it-qat via local Ollama.         │  │
│  │  Fully private — no data leaves your machine.      │  │
│  │                                                    │  │
│  │              [ Start chatting → ]                   │  │
│  └────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### Screenshot 8: Memory View — Managing Knowledge

Navigate to the Memory tab (🧠 icon in nav) to see all stored memories:

```
┌─────────────────────────────────────────────────────────┐
│  🧠 Memory            [⬇ Extract from session]          │
│                        [📄 Summarize session]            │
│                        [＋ Add memory]                    │
│                                                          │
│  [Graph]  [List]  [Session]                              │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │ 🔍 Search memories…          [🔍 Search] [🤖 Sem] │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  Filter: [fact] [preference] [context] [summary]         │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │ fact          ★★★★★                                │  │
│  │ Family law filings: 30-day deadline from service   │  │
│  │ [ law ] [ family ] [ deadline ]                    │  │
│  │ 2026-04-22                              [✏] [🗑]  │  │
│  ├────────────────────────────────────────────────────┤  │
│  │ preference    ★★★★☆                                │  │
│  │ Client Smith prefers email communication           │  │
│  │ [ client ] [ preference ]                          │  │
│  │ 2026-04-21                              [✏] [🗑]  │  │
│  ├────────────────────────────────────────────────────┤  │
│  │ fact          ★★★★★                                │  │
│  │ Section 14.3: Motion response window is 30 days   │  │
│  │ [ law ] [ procedure ] [ motions ]                  │  │
│  │ 2026-04-20                              [✏] [🗑]  │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

### Screenshot 9: Session Tab — Short-Term Memory

```
┌─────────────────────────────────────────────────────────┐
│  🧠 Memory            [⬇ Extract] [📄 Sum] [＋ Add]     │
│                                                          │
│  [Graph]  [List]  [Session] ← active                     │
│                                                          │
│  Short-term memory — the last 20 messages of the         │
│  current session that the brain reads before every reply. │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │ You:  What are the rules for filing in family court│  │
│  │ 🤖:   Based on Section 14.3, family law filings   │  │
│  │       require a 30-day notice period from service. │  │
│  │ You:  What about the Smith case specifically?      │  │
│  │ 🤖:   Client Smith prefers email communication.    │  │
│  │       I'll format the response accordingly.        │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

### Screenshot 10: Chat with RAG Context

```
┌─────────────────────────────────────────────────────────┐
│                                                          │
│         [ 3D VRM Character — Thinking Animation ]        │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │                                                    │  │
│  │  You:  What's the deadline for filing a response   │  │
│  │        to a family law motion?                     │  │
│  │                                                    │  │
│  │  ────────────────────────────────────────────────  │  │
│  │                                                    │  │
│  │  🤖:  Based on the firm's procedures, Section      │  │
│  │       14.3 requires responses to family law        │  │
│  │       motions within 30 days of service. The       │  │
│  │       filing must include a proof of service and   │  │
│  │       be submitted through the court's electronic  │  │
│  │       filing system.                               │  │
│  │                                                    │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │ Type a message…                          [Send ➤]  │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

**What happened behind the scenes:**

1. User's message was embedded: `embed_text("What's the deadline...")` → 768-dim vector (50ms)
2. `vector_search()` scanned all memory embeddings in **<5ms** (pure cosine similarity)
3. Top 5 most relevant memories injected into system prompt as `[LONG-TERM MEMORY]`
4. LLM generated the response with full context — accurate and fast

---

## Real-World Example: Law Firm Knowledge Base

### The Scenario

**Morrison & Clark LLP** — a 50-attorney law firm in Chicago.

Their knowledge lives in multiple disconnected systems, all behind credentials:

| Source | URL | Auth | Update Frequency |
|---|---|---|---|
| Firm Intranet Wiki | `https://wiki.morrisonclark.com` | LDAP (Active Directory) | Daily edits |
| Document Management | `https://dms.morrisonclark.com` | OAuth2 (Azure AD) | Hourly uploads |
| Court Rules PDF Library | `https://ilcourts.gov/rules/` | Public (no auth) | Quarterly revisions |
| Client Portal | `https://portal.morrisonclark.com` | Client-specific API keys | Real-time |
| Training Materials | `\\fileserver\training\` | Windows network share | Monthly |
| Slack/Teams | Microsoft Graph API | OAuth2 | Real-time |

**The problem**: Attorney Sarah needs to ask "What's the deadline for filing a response to a family law motion in Cook County?" — and the answer is spread across 3 different systems, some with outdated versions.

**The goal**: TerranSoul learns all of this daily, deduplicates overlapping info, detects when rules change, and gives instant answers with zero internet calls.

---

### Step-by-Step: First Day Setup

#### Step 1: Install TerranSoul + Choose Local Brain

Sarah installs TerranSoul on her work laptop (Windows 11, 32 GB RAM, no GPU).

```
┌─────────────────────────────────────────────────────────┐
│                    TerranSoul                            │
│                                                          │
│         [ 3D VRM Character — Idle Animation ]            │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │  🧠  Set up your Brain                             │  │
│  │                                                    │  │
│  │  Start chatting instantly with a free cloud LLM:   │  │
│  │                                                    │  │
│  │        [ ☁️ Use Free Cloud API (no setup) ]        │  │
│  │                                                    │  │
│  │  Intel i7-12700 · 32 GB RAM                        │  │
│  │  Or run locally: phi4:14b                          │  │
│  │    Strong reasoning, good for analysis             │  │
│  │                                                    │  │
│  │  ┌──────────────┐ ┌──────────────┐ ┌───────────┐  │  │
│  │  │ phi4:14b  ⭐ │ │ gemma3:4b    │ │ ☁️ kimi   │  │  │
│  │  └──────────────┘ └──────────────┘ └───────────┘  │  │
│  │                                                    │  │
│  │       [ ⬇ Install & activate phi4:14b ]            │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

> Sarah clicks **"🖥 Local LLM"** because firm policy requires all data to
> stay on-premise. No document content ever leaves her machine.

#### Step 2: Bulk Load Firm Knowledge

Sarah's IT admin provides a one-time ingestion script that reads from the firm's
sources with proper credentials. Here's what happens for each source type:

**Source Type A: Wiki Pages (LDAP Auth)**

```
Ingestion Script (runs on Sarah's machine)
──────────────────────────────────────────
1. Authenticate to wiki.morrisonclark.com
   → POST /api/login { user: "sarah", pass: "***" }
   → Receives session cookie

2. Fetch page list
   → GET /api/pages?space=legal-rules
   → Returns 847 wiki pages

3. For each page:
   a. Download content → GET /api/pages/{id}/content
   b. Chunk into ~500-word segments (overlap 50 words)
   c. For each chunk:
      → invoke('add_memory', {
          content: chunk_text,
          tags: "wiki,legal-rules,family-law",
          importance: 4,
          memoryType: "fact"
        })
      → TerranSoul auto-embeds via Ollama (768-dim vector)
      → Stored in SQLite with source_url + source_hash
```

**Source Type B: PDF Documents (Public / File Share)**

```
Ingestion Script
──────────────────
1. Scan \\fileserver\training\*.pdf
   → Found 120 PDF files

2. For each PDF:
   a. Extract text (via pdf-extract or similar)
   b. Split into chunks by section headings
   c. SHA-256 hash the content
   d. Check for existing hash:
      → SELECT id FROM memories WHERE source_hash = ?
      → If exists: SKIP (already ingested)
      → If new: invoke('add_memory', { ... })

3. Track source metadata:
   → source_url = "file:///fileserver/training/handbook-v12.pdf"
   → source_hash = "a1b2c3d4e5f6..."
   → expires_at = NULL (permanent)
```

**Source Type C: Court Rules Website (Public)**

```
Ingestion Script
──────────────────
1. Scrape https://ilcourts.gov/rules/family/
   → Download all rule pages

2. Each rule becomes a memory:
   → content: "Cook County Family Law Rule 14.3: ..."
   → tags: "court-rules,cook-county,family-law"
   → importance: 5  (critical — these are binding rules)
   → source_url: "https://ilcourts.gov/rules/family/14.3"
   → source_hash: SHA-256 of page content
   → expires_at: 1735689600000  (next quarterly review date)
```

#### Step 3: Verify Ingestion in Memory View

After bulk load completes, Sarah opens the Memory tab:

```
┌─────────────────────────────────────────────────────────┐
│  🧠 Memory            [⬇ Extract from session]          │
│                        [📄 Summarize session]            │
│                        [＋ Add memory]                    │
│                                                          │
│  [Graph]  [List] ← active  [Session]                     │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │ 🔍 Search memories…          [🔍 Search] [🤖 Sem] │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  Filter: [fact ●] [preference] [context] [summary]       │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │ fact          ★★★★★                                │  │
│  │ Cook County Family Law Rule 14.3: Responses to     │  │
│  │ motions must be filed within 30 days of service    │  │
│  │ [ court-rules ] [ cook-county ] [ family-law ]     │  │
│  │ 2026-04-22                              [✏] [🗑]  │  │
│  ├────────────────────────────────────────────────────┤  │
│  │ fact          ★★★★☆                                │  │
│  │ Firm Policy 7.2: All filings must be reviewed by   │  │
│  │ a senior partner before submission                 │  │
│  │ [ wiki ] [ firm-policy ] [ filing ]                │  │
│  │ 2026-04-22                              [✏] [🗑]  │  │
│  ├────────────────────────────────────────────────────┤  │
│  │ fact          ★★★★☆                                │  │
│  │ Client Smith preferences: email only, no calls     │  │
│  │ after 6pm, billing contact: jane@smith.com         │  │
│  │ [ client ] [ smith ] [ preference ]                │  │
│  │ 2026-04-22                              [✏] [🗑]  │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  Showing 3 of 12,453 memories                            │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

#### Step 4: Verify Schema Info

Sarah can check database health via the Tauri console or by asking TerranSoul:

```typescript
// From browser dev tools or a custom admin panel:
const info = await invoke('get_schema_info');
console.log(info);
```

```json
{
  "schema_version": 3,
  "target_version": 3,
  "total_memories": 12453,
  "unembedded_count": 0,
  "embedded_count": 12453,
  "db_engine": "SQLite (WAL mode)",
  "columns": {
    "id": "INTEGER PRIMARY KEY AUTOINCREMENT",
    "content": "TEXT NOT NULL — the memory text",
    "tags": "TEXT — comma-separated tags",
    "importance": "INTEGER 1-5 — priority ranking",
    "memory_type": "TEXT — fact|preference|context|summary",
    "created_at": "INTEGER — Unix timestamp (ms)",
    "last_accessed": "INTEGER — last RAG hit timestamp",
    "access_count": "INTEGER — times retrieved by RAG",
    "embedding": "BLOB — 768-dim f32 vector (little-endian)",
    "source_url": "TEXT — origin URL for ingested documents",
    "source_hash": "TEXT — content hash for dedup/staleness",
    "expires_at": "INTEGER — TTL for auto-expiry"
  }
}
```

---

### Daily Sync: Credential-Based Source Ingestion

Every morning at 6:00 AM, a scheduled task runs the sync script on Sarah's machine.
The script authenticates to each source, downloads changes, and feeds them to TerranSoul.

```
┌─────────────────────────────────────────────────────────────────┐
│                     DAILY SYNC WORKFLOW                          │
│                     (runs at 06:00 AM)                           │
│                                                                  │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐       │
│  │ Wiki (LDAP) │     │ DMS (OAuth) │     │ Court Rules │       │
│  │ 847 pages   │     │ 3,200 docs  │     │ 500 rules   │       │
│  └──────┬──────┘     └──────┬──────┘     └──────┬──────┘       │
│         │                   │                   │               │
│         ▼                   ▼                   ▼               │
│  ┌──────────────────────────────────────────────────────┐       │
│  │              Credential Manager                       │       │
│  │                                                       │       │
│  │  Wiki: LDAP bind → session cookie                     │       │
│  │  DMS:  OAuth2 client_credentials → bearer token       │       │
│  │  Court: No auth (public)                              │       │
│  │  Portal: API key from environment variable            │       │
│  │  FileShare: Windows integrated auth (Kerberos)        │       │
│  └──────────────────────┬────────────────────────────────┘       │
│                         │                                        │
│                         ▼                                        │
│  ┌──────────────────────────────────────────────────────┐       │
│  │              Change Detection                         │       │
│  │                                                       │       │
│  │  For each source document:                            │       │
│  │  1. Download content with auth token                  │       │
│  │  2. SHA-256 hash the content                          │       │
│  │  3. Compare with stored source_hash in SQLite         │       │
│  │                                                       │       │
│  │  ┌─────────────┐  ┌──────────────┐  ┌────────────┐  │       │
│  │  │ UNCHANGED   │  │ MODIFIED     │  │ NEW        │  │       │
│  │  │ hash match  │  │ hash differs │  │ no record  │  │       │
│  │  │ → SKIP      │  │ → UPDATE +   │  │ → INSERT + │  │       │
│  │  │             │  │   re-embed   │  │   embed    │  │       │
│  │  └─────────────┘  └──────────────┘  └────────────┘  │       │
│  └──────────────────────┬────────────────────────────────┘       │
│                         │                                        │
│                         ▼                                        │
│  ┌──────────────────────────────────────────────────────┐       │
│  │              Dedup Check                              │       │
│  │                                                       │       │
│  │  Before inserting new content:                        │       │
│  │  1. Embed the new text                                │       │
│  │  2. find_duplicate(embedding, threshold=0.97)         │       │
│  │  3. Cosine > 0.97? → SKIP (near-duplicate)           │       │
│  │     Cosine < 0.97? → INSERT as new memory             │       │
│  └──────────────────────┬────────────────────────────────┘       │
│                         │                                        │
│                         ▼                                        │
│  ┌──────────────────────────────────────────────────────┐       │
│  │              Expiry Cleanup                           │       │
│  │                                                       │       │
│  │  DELETE FROM memories                                 │       │
│  │  WHERE expires_at IS NOT NULL                         │       │
│  │    AND expires_at < current_timestamp                  │       │
│  │                                                       │       │
│  │  → Removes: old court calendars, temp announcements   │       │
│  └──────────────────────────────────────────────────────┘       │
│                                                                  │
│  SYNC REPORT (emailed to admin):                                │
│  ─────────────────────────────────────                          │
│  Sources checked:     5                                          │
│  Documents scanned:   4,547                                      │
│  Unchanged (skipped): 4,489                                      │
│  Modified (updated):  23                                         │
│  New (inserted):      35                                         │
│  Duplicates (skipped):7                                          │
│  Expired (deleted):   12                                         │
│  Embeddings created:  58                                         │
│  Total sync time:     4m 32s                                     │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

#### Example Sync Script (TypeScript — runs as a scheduled Tauri sidecar)

```typescript
// sync-firm-knowledge.ts — Daily knowledge sync for Morrison & Clark LLP
import { invoke } from '@tauri-apps/api/core';
import { createHash } from 'crypto';

interface SyncSource {
  name: string;
  type: 'wiki' | 'dms' | 'web' | 'fileshare';
  baseUrl: string;
  auth: AuthConfig;
  tags: string;
  importance: number;
  expiresInDays?: number;
}

interface AuthConfig {
  method: 'ldap' | 'oauth2' | 'apikey' | 'kerberos' | 'none';
  credentials?: string;  // env var name, NEVER hardcoded
}

const SOURCES: SyncSource[] = [
  {
    name: 'Firm Wiki',
    type: 'wiki',
    baseUrl: 'https://wiki.morrisonclark.com/api',
    auth: { method: 'ldap', credentials: 'WIKI_SESSION_TOKEN' },
    tags: 'wiki,firm-rules',
    importance: 4,
  },
  {
    name: 'Court Rules',
    type: 'web',
    baseUrl: 'https://ilcourts.gov/rules',
    auth: { method: 'none' },
    tags: 'court-rules,illinois',
    importance: 5,
    expiresInDays: 90,  // re-check quarterly
  },
  {
    name: 'Client Portal',
    type: 'dms',
    baseUrl: 'https://portal.morrisonclark.com/api/v2',
    auth: { method: 'oauth2', credentials: 'PORTAL_OAUTH_TOKEN' },
    tags: 'client,portal',
    importance: 3,
  },
];

async function syncSource(source: SyncSource): Promise<SyncStats> {
  const stats = { scanned: 0, unchanged: 0, modified: 0, inserted: 0, duplicates: 0 };

  // 1. Authenticate (credentials from environment, NEVER hardcoded)
  const token = await authenticate(source.auth);

  // 2. Fetch document list from source
  const documents = await fetchDocumentList(source.baseUrl, token);

  for (const doc of documents) {
    stats.scanned++;

    // 3. Download content
    const content = await fetchDocument(doc.url, token);
    const hash = createHash('sha256').update(content).digest('hex');

    // 4. Check if this URL already exists with same hash
    const existing = await invoke<any[]>('search_memories', {
      query: doc.url,
    });

    const match = existing.find(m => m.source_url === doc.url);

    if (match && match.source_hash === hash) {
      // Content unchanged — skip
      stats.unchanged++;
      continue;
    }

    // 5. Chunk the content (for long documents)
    const chunks = chunkText(content, 500, 50);  // 500 words, 50 overlap

    for (const chunk of chunks) {
      // 6. Check for semantic duplicates
      const isDuplicate = await invoke<number | null>('check_duplicate', {
        content: chunk,
      });

      if (isDuplicate) {
        stats.duplicates++;
        continue;
      }

      // 7. Insert new memory (auto-embedded by TerranSoul)
      const expiresAt = source.expiresInDays
        ? Date.now() + source.expiresInDays * 86400000
        : undefined;

      await invoke('add_memory', {
        content: chunk,
        tags: source.tags,
        importance: source.importance,
        memoryType: 'fact',
        // V3 columns:
        sourceUrl: doc.url,
        sourceHash: hash,
        expiresAt,
      });

      if (match) {
        stats.modified++;
      } else {
        stats.inserted++;
      }
    }
  }

  return stats;
}

function chunkText(text: string, maxWords: number, overlap: number): string[] {
  const words = text.split(/\s+/);
  const chunks: string[] = [];
  for (let i = 0; i < words.length; i += maxWords - overlap) {
    chunks.push(words.slice(i, i + maxWords).join(' '));
    if (i + maxWords >= words.length) break;
  }
  return chunks;
}
```

---

### Handling Duplicates Across Sources

The same fact often appears in multiple places:

```
EXAMPLE: "Family law responses must be filed within 30 days"

Source 1: Firm Wiki → /rules/family-law/deadlines
  "A response to a family law motion must be filed
   within thirty (30) days of service."

Source 2: Court Rules → ilcourts.gov/rules/family/14.3
  "Responses to motions in family law matters shall
   be filed within 30 days after service."

Source 3: Training Manual → handbook-v12.pdf, page 47
  "Filing deadline: 30 days from service date for
   all family law motion responses."
```

Without dedup, TerranSoul would store **3 separate memories** saying the same thing,
wasting space and potentially confusing RAG ranking.

#### How TerranSoul Handles This

```
Step 1: Source 1 arrives first
  → embed_text("A response to a family law motion...")
  → embedding = [0.82, -0.31, 0.56, ...]
  → No existing duplicate found (empty DB)
  → INSERT → id=1

Step 2: Source 2 arrives
  → embed_text("Responses to motions in family law...")
  → embedding = [0.83, -0.30, 0.55, ...]
  → find_duplicate(embedding, threshold=0.97)
  → cosine_similarity with id=1 = 0.98  ← ABOVE THRESHOLD
  → SKIP — this is a near-duplicate of id=1

Step 3: Source 3 arrives
  → embed_text("Filing deadline: 30 days from service...")
  → embedding = [0.81, -0.32, 0.57, ...]
  → find_duplicate(embedding, threshold=0.97)
  → cosine_similarity with id=1 = 0.97  ← AT THRESHOLD
  → SKIP — still a near-duplicate

Result: Only 1 memory stored instead of 3
  → id=1 "A response to a family law motion must be filed
           within thirty (30) days of service."
  → source_url = wiki (first source wins)
```

#### Dedup in the SQLite Database

```sql
-- Find all memories from the same source that might be redundant
SELECT a.id, b.id AS similar_to, a.content, a.source_url
FROM memories a
JOIN memories b ON a.id > b.id
WHERE a.source_url != b.source_url
  AND a.embedding IS NOT NULL
  AND b.embedding IS NOT NULL;
-- Note: Actual cosine comparison happens in Rust, not SQL

-- Admin query: find exact text duplicates (not just semantic)
SELECT content, COUNT(*) AS copies, GROUP_CONCAT(id) AS ids
FROM memories
GROUP BY content
HAVING copies > 1;
```

---

### Detecting and Removing Stale/Out-of-Date Knowledge

Legal rules change. Court deadlines shift. Firm policies get revised.
TerranSoul uses three mechanisms to keep knowledge current.
See [Multi-Source Knowledge Management](BRAIN-COMPLEX-EXAMPLE-EXPLAIN.md#multi-source-knowledge-management)
in the technical reference for full details.

**Quick summary:**

1. **Source hash change detection**: Daily sync re-downloads each source, SHA-256 hashes the content, and compares with the stored `source_hash`. Different hash → delete old, insert new, re-embed.

2. **TTL expiry**: Time-sensitive content gets an `expires_at` timestamp. Daily cleanup removes expired memories automatically.

3. **Access count decay**: Memories that RAG never retrieves (0 accesses after 90 days) are candidates for pruning.

4. **LLM conflict resolution**: When new information overlaps semantically but differs in content, the LLM analyzes which version is current and resolves the conflict.

---

### Day-in-the-Life: Attorney Sarah Uses TerranSoul

#### 8:30 AM — Sarah Opens TerranSoul

The daily sync already ran at 6:00 AM. TerranSoul has 12,500+ memories
from all firm sources, fully embedded and searchable.

```
┌─────────────────────────────────────────────────────────┐
│                                                          │
│         [ 3D VRM Character — Good morning wave ]         │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │                                                    │  │
│  │  You:  Good morning! I need to prepare a response  │  │
│  │        to the Smith family law motion. What are     │  │
│  │        the deadlines and filing requirements?       │  │
│  │                                                    │  │
│  │  ────────────────────────────────────────────────  │  │
│  │                                                    │  │
│  │  🤖:  Good morning! For the Smith case, here's     │  │
│  │       what I found:                                │  │
│  │                                                    │  │
│  │       **Deadline**: Per Cook County Family Law      │  │
│  │       Rule 14.3, you have 30 days from service     │  │
│  │       to file your response.                       │  │
│  │                                                    │  │
│  │       **Firm Policy**: Policy 7.2 requires         │  │
│  │       senior partner review before submission.     │  │
│  │                                                    │  │
│  │       **Client Preference**: Mr. Smith prefers     │  │
│  │       email communication only, no calls after     │  │
│  │       6pm. Billing contact: jane@smith.com         │  │
│  │                                                    │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │ Type a message…                          [Send ➤]  │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

**Behind the scenes** — the RAG pipeline found 3 relevant memories in **<5ms**:

```
[LONG-TERM MEMORY]
- Cook County Family Law Rule 14.3: Responses to motions must be
  filed within 30 days of service.
- Firm Policy 7.2: All filings must be reviewed by a senior partner
  before submission.
- Client Smith preferences: email only, no calls after 6pm, billing
  contact: jane@smith.com
[/LONG-TERM MEMORY]
```

#### 10:00 AM — Sarah Learns Something New in a Meeting

Sarah: "I just learned that Judge Martinez's courtroom has a new
local rule — all motions must include a certificate of compliance
starting May 1st."

```
┌────────────────────────────────────────────────────────┐
│                                                         │
│  You:  Remember this: Judge Martinez's courtroom now    │
│        requires a certificate of compliance for all     │
│        motions starting May 1st, 2026.                  │
│                                                         │
│  🤖:   Got it! I've stored that as a high-importance    │
│        fact. I'll remember this for future filings      │
│        in Judge Martinez's courtroom.                   │
│                                                         │
└────────────────────────────────────────────────────────┘
```

TerranSoul auto-extracts this as a memory:

```sql
INSERT INTO memories
  (content, tags, importance, memory_type, created_at, access_count)
VALUES
  ('Judge Martinez courtroom: certificate of compliance
   required for all motions starting May 1st, 2026',
   'judge-martinez,compliance,motions,new-rule',
   5,
   'fact',
   1713780000000,
   0);
-- Auto-embedded via Ollama → 768-dim vector stored as BLOB
```

#### 2:00 PM — Sarah Asks About a Rule That Changed Overnight

The daily sync detected that Court Rule 22.1 was amended:

```
┌────────────────────────────────────────────────────────┐
│                                                         │
│  You:  What's the current procedure for filing a        │
│        civil motion in Cook County?                     │
│                                                         │
│  🤖:   Based on the updated Cook County Civil Rule      │
│        22.1 (amended April 21, 2026):                   │
│                                                         │
│        1. Electronic filing is now MANDATORY (was        │
│           optional before the amendment)                 │
│        2. Paper copies are no longer accepted            │
│        3. Filing fee increased to $350                   │
│                                                         │
│        Note: This rule was updated yesterday. The        │
│        previous version required only e-filing for       │
│        cases over $50,000.                               │
│                                                         │
└────────────────────────────────────────────────────────┘
```

**This works because** the daily sync at 6:00 AM:
1. Downloaded the new version of Rule 22.1
2. Detected the hash mismatch (`old_hash ≠ new_hash`)
3. Deleted the old memory
4. Inserted the new text
5. Auto-embedded the new version
6. Sarah gets the updated rule — automatically, no manual intervention

#### 5:00 PM — End of Day: Extract Session Memories

Sarah clicks "⬇ Extract from session" in the Memory tab:

```
┌─────────────────────────────────────────────────────────┐
│  🧠 Memory            [⬇ Extract from session]          │
│                        Working… extracting facts         │
│                                                          │
│  Extracted 4 new memories from today's session:          │
│                                                          │
│  ✓ "Judge Martinez requires certificate of compliance"   │
│  ✓ "Smith case response deadline is April 30th"          │
│  ✓ "New e-filing mandate for civil motions"              │
│  ✓ "Sarah prefers to prepare filings in the morning"     │
│                                                          │
│  Total memories: 12,457 (was 12,453 this morning)        │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

These session facts are now part of the permanent long-term memory,
available for RAG in all future conversations.

---

> **For architecture diagrams, SQLite schema details, RAG pipeline internals,
> debugging queries, hardware scaling tables, and FAQ** — see
> [BRAIN-COMPLEX-EXAMPLE-EXPLAIN.md](BRAIN-COMPLEX-EXAMPLE-EXPLAIN.md).
# Brain + RAG Complex Setup Guide

> **TerranSoul v0.1** — Self-learning AI companion with persistent memory  
> Last updated: 2026-04-22

---

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Why SQLite?](#why-sqlite)
3. [Schema & Migrations](#schema--migrations)
4. [RAG Pipeline: How Memory Retrieval Works](#rag-pipeline-how-memory-retrieval-works)
5. [Setup Walkthrough (Screenshots)](#setup-walkthrough-screenshots)
6. [Real-World Example: Law Firm Knowledge Base](#real-world-example-law-firm-knowledge-base)
   - [The Scenario](#the-scenario)
   - [Step-by-Step: First Day Setup](#step-by-step-first-day-setup)
   - [Daily Sync: Credential-Based Source Ingestion](#daily-sync-credential-based-source-ingestion)
   - [Handling Duplicates Across Sources](#handling-duplicates-across-sources)
   - [Detecting and Removing Stale/Out-of-Date Knowledge](#detecting-and-removing-staleout-of-date-knowledge)
   - [Day-in-the-Life: Attorney Sarah Uses TerranSoul](#day-in-the-life-attorney-sarah-uses-terransoul)
7. [Debugging with SQLite](#debugging-with-sqlite)
8. [Hardware Scaling](#hardware-scaling)
9. [FAQ](#faq)

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        TerranSoul Desktop App                          │
│                                                                         │
│  ┌───────────────┐    ┌──────────────┐    ┌───────────────────────┐    │
│  │  Vue 3 UI     │───>│ Tauri IPC    │───>│  Rust Core Engine     │    │
│  │               │    │ (invoke)     │    │                       │    │
│  │ • ChatView    │    └──────────────┘    │  ┌─────────────────┐  │    │
│  │ • MemoryView  │                        │  │ Brain Module    │  │    │
│  │ • BrainSetup  │                        │  │ • OllamaAgent  │  │    │
│  └───────────────┘                        │  │ • embed_text() │  │    │
│                                           │  │ • call() (LLM) │  │    │
│                                           │  └────────┬────────┘  │    │
│                                           │           │           │    │
│                                           │  ┌────────▼────────┐  │    │
│                                           │  │ Memory Module   │  │    │
│                                           │  │ • MemoryStore   │  │    │
│                                           │  │ • vector_search │  │    │
│                                           │  │ • brain_memory  │  │    │
│                                           │  └────────┬────────┘  │    │
│                                           │           │           │    │
│                                           │  ┌────────▼────────┐  │    │
│                                           │  │ SQLite (WAL)    │  │    │
│                                           │  │ memory.db       │  │    │
│                                           │  └─────────────────┘  │    │
│                                           └───────────────────────┘    │
│                                                                         │
│                          ┌──────────────────┐                          │
│                          │ Ollama Server     │ (localhost:11434)        │
│                          │ • Chat API        │                          │
│                          │ • Embed API       │                          │
│                          │ • nomic-embed-text│                          │
│                          └──────────────────┘                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Data Flow: User Message → RAG-Enhanced Response

```
User types: "What are the filing rules for family law cases?"
                │
                ▼
┌─────────────────────────────┐
│ 1. Embed query via Ollama   │  POST /api/embed
│    → 768-dim float vector   │  {"model":"nomic-embed-text","input":"..."}
└──────────────┬──────────────┘
               ▼
┌─────────────────────────────┐
│ 2. Cosine similarity search │  Pure Rust arithmetic
│    vs all stored embeddings │  <5ms for 100k entries
│    → Top 5 memories ranked  │  No LLM call needed
└──────────────┬──────────────┘
               ▼
┌─────────────────────────────┐
│ 3. Inject into system prompt│  [LONG-TERM MEMORY]
│    as context for the LLM   │  - Family law filings require...
│                              │  - Section 14.3 states that...
└──────────────┬──────────────┘
               ▼
┌─────────────────────────────┐
│ 4. LLM generates response   │  Informed by relevant memories
│    with full RAG context     │  "Based on the firm's filing rules..."
└─────────────────────────────┘
```

---

## Why SQLite?

TerranSoul is a **desktop app** (Tauri 2.x), not a web service. The database must:

| Requirement | SQLite ✓ | PostgreSQL ✗ | Why SQLite wins |
|---|---|---|---|
| **Zero config** | Embedded, no server | Needs install + config | Users just open the app |
| **Single file** | `memory.db` | Data directory cluster | Easy backup, easy sync |
| **Crash-safe** | WAL mode = ACID | Needs `pg_dump` setup | Auto-backup on startup |
| **Portable** | Works everywhere | OS-specific packages | Windows/Mac/Linux/mobile |
| **Performance** | <5ms for 100k rows | Overkill for single-user | Desktop app, not a cluster |
| **Offline** | Always works | Needs running service | Companion works without internet |

### WAL Mode (Write-Ahead Logging)

TerranSoul enables WAL mode on every startup:

```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
```

**What this means for your data:**
- Writes go to a WAL file first, then get checkpointed to the main DB
- If the app crashes mid-write, the WAL replays on next open — **zero data loss**
- Concurrent reads while writing (important for RAG search during chat)

### Auto-Backup

Every time TerranSoul starts, it copies `memory.db` → `memory.db.bak`:

```
%APPDATA%/com.terransoul.app/
├── memory.db         ← Live database
├── memory.db.bak     ← Auto-backup from last startup
├── memory.db-wal     ← Write-ahead log (may exist during use)
└── memory.db-shm     ← Shared memory (may exist during use)
```

---

## Schema & Migrations

TerranSoul uses a **versioned migration system** — schema changes are tracked,
applied incrementally, and can be rolled back without losing customer data.

### Current Schema (Version 3)

```sql
-- Tracked in schema_version table
-- V1: Initial table
-- V2: Vector embeddings for fast RAG
-- V3: Source tracking for document ingestion

CREATE TABLE memories (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    content       TEXT    NOT NULL,        -- The memory text
    tags          TEXT    NOT NULL DEFAULT '',  -- Comma-separated tags
    importance    INTEGER NOT NULL DEFAULT 3,   -- 1-5 priority ranking
    memory_type   TEXT    NOT NULL DEFAULT 'fact',  -- fact|preference|context|summary
    created_at    INTEGER NOT NULL,        -- Unix timestamp (ms)
    last_accessed INTEGER,                 -- Last RAG hit timestamp
    access_count  INTEGER NOT NULL DEFAULT 0,  -- Times retrieved by RAG
    embedding     BLOB,                    -- 768-dim f32 vector (V2+)
    source_url    TEXT,                    -- Origin URL for ingested docs (V3+)
    source_hash   TEXT,                    -- Content hash for dedup (V3+)
    expires_at    INTEGER                  -- TTL for auto-expiry (V3+)
);

CREATE TABLE schema_version (
    version     INTEGER PRIMARY KEY,
    applied_at  INTEGER NOT NULL,
    description TEXT    NOT NULL DEFAULT ''
);
```

### How Migrations Work

```
App starts → Check schema_version table → Apply missing migrations
                                                │
    ┌──────────────────────┐    ┌───────────────▼───────────────┐
    │ schema_version table │    │ Migration runner               │
    │                      │    │                                │
    │ version │ applied_at │    │ current=2, target=3            │
    │ ────────┼────────────│    │ → Apply V3 up SQL              │
    │    1    │ 1713744000 │    │ → Record version 3             │
    │    2    │ 1713744001 │    │ → Done                         │
    └──────────────────────┘    └────────────────────────────────┘
```

**Upgrade**: Migrations run automatically on startup — users never notice.  
**Downgrade**: Available via `downgrade_to(conn, target_version)` for CI rollbacks.  
**Idempotent**: Running migrations twice is safe — already-applied versions are skipped.  
**Tolerant**: Old databases with ad-hoc `ALTER TABLE` are handled gracefully.

### Adding a New Migration

Append to `src-tauri/src/memory/migrations.rs`:

```rust
Migration {
    version: 4,
    description: "add your new feature",
    up: "ALTER TABLE memories ADD COLUMN new_col TEXT;",
    down: r#"
        CREATE TABLE memories_backup AS SELECT ... FROM memories;
        DROP TABLE memories;
        ALTER TABLE memories_backup RENAME TO memories;
    "#,
},
```

Then run `cargo test` — the migration tests verify up/down roundtrips automatically.

---

## RAG Pipeline: How Memory Retrieval Works

### Short-Term Memory

The last ~20 conversation messages are held in-memory (Rust `Vec<Message>`):

```
┌─────────────────────────────────────────────┐
│ Short-Term Memory (conversation buffer)      │
│                                               │
│ [user] "What are the rules for filing?"       │
│ [assistant] "Family law filings require..."   │
│ [user] "What about the deadline?"             │
│ [assistant] "Section 14.3 states 30 days..."  │
│ ...last 20 messages                           │
│                                               │
│ Stored in: Rust AppState.conversation Mutex   │
│ Lifetime: Current session only                │
│ Used for: Conversation continuity             │
└─────────────────────────────────────────────┘
```

### Long-Term Memory

Persistent facts stored in SQLite, retrieved by vector similarity:

```
┌─────────────────────────────────────────────┐
│ Long-Term Memory (SQLite + embeddings)       │
│                                               │
│ id=1  "Client prefers email communication"    │
│       embedding=[0.12, -0.34, 0.56, ...]     │
│       tags="preference,client"                │
│       importance=4                            │
│                                               │
│ id=2  "Family law filings: 30-day deadline"   │
│       embedding=[0.78, 0.11, -0.45, ...]     │
│       tags="law,family,deadline"              │
│       importance=5                            │
│                                               │
│ id=3  "Office closes at 5pm on Fridays"       │
│       embedding=[0.01, 0.99, -0.02, ...]     │
│       tags="office,schedule"                  │
│       importance=2                            │
│                                               │
│ Stored in: memory.db (SQLite WAL)             │
│ Lifetime: Permanent until deleted             │
│ Used for: RAG context injection               │
└─────────────────────────────────────────────┘
```

### Vector Search Performance

The key innovation: **pure-Rust cosine similarity** — no external vector DB needed.

```
Traditional (SLOW — old approach):
  Load ALL memories → Send to LLM → "Which are relevant?" → Wait 2-5 seconds
  Scales to ~500 entries, then LLM context window overflows

TerranSoul Vector RAG (FAST — current approach):
  Embed query (50ms) → Cosine search all vectors (<5ms) → Inject top 5 → Done
  Scales to 100,000+ entries with no degradation

┌──────────────────────────────────────────────────────┐
│ Benchmark: vector_search() on 100,000 entries        │
│                                                       │
│  Entries    │  Search Time  │  Method                 │
│ ────────────┼───────────────┼──────────────────────── │
│       100   │     <1 ms     │  Pure cosine similarity │
│     1,000   │     <1 ms     │  Pure cosine similarity │
│    10,000   │      2 ms     │  Pure cosine similarity │
│   100,000   │      5 ms     │  Pure cosine similarity │
│ 1,000,000   │     ~50 ms    │  Pure cosine similarity │
│                                                       │
│ * All times exclude the one-time 50ms embed call      │
│ * Measured on RTX 3080 Ti / 65GB RAM / Windows 11     │
└──────────────────────────────────────────────────────┘
```

### Embedding Model

TerranSoul uses `nomic-embed-text` (768-dimensional) via Ollama:

```
POST http://127.0.0.1:11434/api/embed
{
    "model": "nomic-embed-text",
    "input": "Family law filings require a 30-day notice period"
}

Response:
{
    "embeddings": [[0.0123, -0.0456, 0.0789, ...]]  // 768 floats
}
```

- **Auto-installed**: If `nomic-embed-text` isn't available, falls back to the active chat model
- **Storage**: Each embedding = 768 × 4 bytes = **3 KB** per memory
- **100k memories** = ~300 MB of embeddings in SQLite — easily fits in RAM

### Deduplication

Before storing a new memory, TerranSoul checks for near-duplicates:

```
New text: "Family law filings must be submitted within 30 days"
                │
                ▼
Embed → cosine similarity vs all existing embeddings
                │
                ▼
Existing id=2: "Family law filings: 30-day deadline"
Cosine similarity = 0.98 (threshold: 0.97)
                │
                ▼
DUPLICATE DETECTED → Skip insert, return existing entry
```

---

## Setup Walkthrough (Screenshots)

### Screenshot 1: First Launch — Brain Setup Overlay (ChatView)

When you first open TerranSoul, the 3D character is visible but a Brain Setup
card floats on top. This is the quick-start path from `ChatView.vue`:

```
┌─────────────────────────────────────────────────────────┐
│                    TerranSoul                            │
│                                                          │
│         [ 3D VRM Character — Idle Animation ]            │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │  🧠  Set up your Brain                             │  │
│  │                                                    │  │
│  │  Start chatting instantly with a free cloud LLM:   │  │
│  │                                                    │  │
│  │        [ ☁️ Use Free Cloud API (no setup) ]        │  │
│  │                                                    │  │
│  │  Intel i7 · 65 GB RAM                              │  │
│  │  Or run locally: gemma3:12b-it-qat                 │  │
│  │    Fast, private, runs on your hardware            │  │
│  │                                                    │  │
│  │  ┌──────────────┐ ┌──────────────┐ ┌───────────┐  │  │
│  │  │gemma3:12b ⭐ │ │ phi4:14b     │ │llama3.3   │  │  │
│  │  └──────────────┘ └──────────────┘ └───────────┘  │  │
│  │                                                    │  │
│  │  ❌ Ollama not running — start it first            │  │
│  │     (ollama serve)               [ 🔄 Retry ]     │  │
│  │                                                    │  │
│  │       [ ⬇ Install & activate gemma3:12b ]          │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

> **For privacy-sensitive use cases (law firms, medical, finance)**: Choose the
> local Ollama path. Zero data ever leaves the machine.

### Screenshot 2: Brain Setup Wizard (BrainSetupView — Full Setup)

For more control, navigate to Settings → Brain Setup. This is a multi-step wizard:

```
┌─────────────────────────────────────────────────────────┐
│  Brain Setup                                             │
│                                                          │
│  Step: (●)─────(○)─────(○)─────(○)─────(○)              │
│        Tier   Hardware  Model  Ollama  Download          │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │         🧠 Choose how to power your Brain          │  │
│  │                                                    │  │
│  │  TerranSoul needs an AI brain for conversations.   │  │
│  │  Choose how you'd like to connect it.              │  │
│  │                                                    │  │
│  │  ┌──────────────────────────────────────────────┐  │  │
│  │  │ ☁️  Free Cloud API                           │  │  │
│  │  │ Use free LLM providers (Groq, Cerebras)      │  │  │
│  │  │ No API key needed · Rate-limited              │  │  │
│  │  │                           [ Instant — no setup]│  │  │
│  │  └──────────────────────────────────────────────┘  │  │
│  │                                                    │  │
│  │  ┌──────────────────────────────────────────────┐  │  │
│  │  │ 💳 Paid Cloud API                            │  │  │
│  │  │ OpenAI, Anthropic, or custom endpoint        │  │  │
│  │  │ Best quality · Requires API key               │  │  │
│  │  └──────────────────────────────────────────────┘  │  │
│  │                                                    │  │
│  │  ┌──────────────────────────────────────────────┐  │  │
│  │  │ 🖥  Local LLM (Ollama)           ← SELECTED  │  │  │
│  │  │ Run locally, fully private, no internet       │  │  │
│  │  │ Requires Ollama installed · Best for privacy  │  │  │
│  │  └──────────────────────────────────────────────┘  │  │
│  │                                                    │  │
│  │                                      [ Next → ]    │  │
│  └────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### Screenshot 3: Hardware Analysis (Step 2)

```
┌─────────────────────────────────────────────────────────┐
│  Brain Setup                                             │
│                                                          │
│  Step: (✓)─────(●)─────(○)─────(○)─────(○)              │
│        Tier   Hardware  Model  Ollama  Download          │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │              🖥 Local LLM Setup                     │  │
│  │                                                    │  │
│  │  We'll analyse your hardware and recommend the     │  │
│  │  best model for your machine.                      │  │
│  │                                                    │  │
│  │  💾 RAM    65 GB (High-End)                        │  │
│  │  🖥 CPU    Intel i7-12700K · 12 cores              │  │
│  │  🗂 OS     Windows 11 (x86_64)                     │  │
│  │                                                    │  │
│  │               [ ← Back ]  [ Next → ]               │  │
│  └────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### Screenshot 4: Choose Model (Step 3)

```
┌─────────────────────────────────────────────────────────┐
│  Brain Setup                                             │
│                                                          │
│  Step: (✓)─────(✓)─────(●)─────(○)─────(○)              │
│        Tier   Hardware  Model  Ollama  Download          │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │          Choose your Brain                          │  │
│  │  Based on your 65 GB of RAM, we recommend:         │  │
│  │                                                    │  │
│  │  ┌──────────────────────────────────────────────┐  │  │
│  │  │ ⭐ gemma3:12b-it-qat            ← SELECTED  │  │  │
│  │  │    Fast and accurate for conversations       │  │  │
│  │  │    Requires 8 GB RAM · tag: gemma3:12b-it-qat│  │  │
│  │  └──────────────────────────────────────────────┘  │  │
│  │  ┌──────────────────────────────────────────────┐  │  │
│  │  │   phi4:14b                                   │  │  │
│  │  │   Strong reasoning, good for analysis        │  │  │
│  │  │   Requires 10 GB RAM · tag: phi4:14b         │  │  │
│  │  └──────────────────────────────────────────────┘  │  │
│  │  ┌──────────────────────────────────────────────┐  │  │
│  │  │   ☁️ kimi-k2.6  (Cloud)                      │  │  │
│  │  │   Cloud-routed · no local RAM needed         │  │  │
│  │  └──────────────────────────────────────────────┘  │  │
│  │                                                    │  │
│  │               [ ← Back ]  [ Next → ]               │  │
│  └────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### Screenshot 5: Ollama Status Check (Step 4)

```
┌─────────────────────────────────────────────────────────┐
│  Brain Setup                                             │
│                                                          │
│  Step: (✓)─────(✓)─────(✓)─────(●)─────(○)              │
│        Tier   Hardware  Model  Ollama  Download          │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │              Check Ollama                           │  │
│  │                                                    │  │
│  │  TerranSoul uses Ollama to run models locally.     │  │
│  │  It must be running before we can download.        │  │
│  │                                                    │  │
│  │           ✅ Ollama is running                      │  │
│  │                                                    │  │
│  │          [ ← Back ] [ 🔄 Retry ] [ Next → ]        │  │
│  └────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### Screenshot 6: Download & Activate (Step 5)

```
┌─────────────────────────────────────────────────────────┐
│  Brain Setup                                             │
│                                                          │
│  Step: (✓)─────(✓)─────(✓)─────(✓)─────(●)              │
│        Tier   Hardware  Model  Ollama  Download          │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │        Download gemma3:12b-it-qat                   │  │
│  │                                                    │  │
│  │  This will download the model via Ollama.          │  │
│  │  It may take several minutes.                      │  │
│  │                                                    │  │
│  │           🔄 Downloading… this may take             │  │
│  │              a few minutes.                         │  │
│  │                                                    │  │
│  │                          [ ← Back ]                 │  │
│  └────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### Screenshot 7: Brain Connected! (Done)

```
┌─────────────────────────────────────────────────────────┐
│  Brain Setup                                             │
│                                                          │
│  Step: (✓)─────(✓)─────(✓)─────(✓)─────(✓)              │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │                    🎉                               │  │
│  │             Brain connected!                        │  │
│  │                                                    │  │
│  │  Using gemma3:12b-it-qat via local Ollama.         │  │
│  │  Fully private — no data leaves your machine.      │  │
│  │                                                    │  │
│  │              [ Start chatting → ]                   │  │
│  └────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### Screenshot 8: Memory View — Managing Knowledge

Navigate to the Memory tab (🧠 icon in nav) to see all stored memories:

```
┌─────────────────────────────────────────────────────────┐
│  🧠 Memory            [⬇ Extract from session]          │
│                        [📄 Summarize session]            │
│                        [＋ Add memory]                    │
│                                                          │
│  [Graph]  [List]  [Session]                              │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │ 🔍 Search memories…          [🔍 Search] [🤖 Sem] │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  Filter: [fact] [preference] [context] [summary]         │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │ fact          ★★★★★                                │  │
│  │ Family law filings: 30-day deadline from service   │  │
│  │ [ law ] [ family ] [ deadline ]                    │  │
│  │ 2026-04-22                              [✏] [🗑]  │  │
│  ├────────────────────────────────────────────────────┤  │
│  │ preference    ★★★★☆                                │  │
│  │ Client Smith prefers email communication           │  │
│  │ [ client ] [ preference ]                          │  │
│  │ 2026-04-21                              [✏] [🗑]  │  │
│  ├────────────────────────────────────────────────────┤  │
│  │ fact          ★★★★★                                │  │
│  │ Section 14.3: Motion response window is 30 days   │  │
│  │ [ law ] [ procedure ] [ motions ]                  │  │
│  │ 2026-04-20                              [✏] [🗑]  │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

### Screenshot 9: Session Tab — Short-Term Memory

```
┌─────────────────────────────────────────────────────────┐
│  🧠 Memory            [⬇ Extract] [📄 Sum] [＋ Add]     │
│                                                          │
│  [Graph]  [List]  [Session] ← active                     │
│                                                          │
│  Short-term memory — the last 20 messages of the         │
│  current session that the brain reads before every reply. │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │ You:  What are the rules for filing in family court│  │
│  │ 🤖:   Based on Section 14.3, family law filings   │  │
│  │       require a 30-day notice period from service. │  │
│  │ You:  What about the Smith case specifically?      │  │
│  │ 🤖:   Client Smith prefers email communication.    │  │
│  │       I'll format the response accordingly.        │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

### Screenshot 10: Chat with RAG Context

```
┌─────────────────────────────────────────────────────────┐
│                                                          │
│         [ 3D VRM Character — Thinking Animation ]        │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │                                                    │  │
│  │  You:  What's the deadline for filing a response   │  │
│  │        to a family law motion?                     │  │
│  │                                                    │  │
│  │  ────────────────────────────────────────────────  │  │
│  │                                                    │  │
│  │  🤖:  Based on the firm's procedures, Section      │  │
│  │       14.3 requires responses to family law        │  │
│  │       motions within 30 days of service. The       │  │
│  │       filing must include a proof of service and   │  │
│  │       be submitted through the court's electronic  │  │
│  │       filing system.                               │  │
│  │                                                    │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │ Type a message…                          [Send ➤]  │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

**What happened behind the scenes:**

1. User's message was embedded: `embed_text("What's the deadline...")` → 768-dim vector (50ms)
2. `vector_search()` scanned all memory embeddings in **<5ms** (pure cosine similarity)
3. Top 5 most relevant memories injected into system prompt as `[LONG-TERM MEMORY]`
4. LLM generated the response with full context — accurate and fast

---

## Real-World Example: Law Firm Knowledge Base

### The Scenario

**Morrison & Clark LLP** — a 50-attorney law firm in Chicago.

Their knowledge lives in multiple disconnected systems, all behind credentials:

| Source | URL | Auth | Update Frequency |
|---|---|---|---|
| Firm Intranet Wiki | `https://wiki.morrisonclark.com` | LDAP (Active Directory) | Daily edits |
| Document Management | `https://dms.morrisonclark.com` | OAuth2 (Azure AD) | Hourly uploads |
| Court Rules PDF Library | `https://ilcourts.gov/rules/` | Public (no auth) | Quarterly revisions |
| Client Portal | `https://portal.morrisonclark.com` | Client-specific API keys | Real-time |
| Training Materials | `\\fileserver\training\` | Windows network share | Monthly |
| Slack/Teams | Microsoft Graph API | OAuth2 | Real-time |

**The problem**: Attorney Sarah needs to ask "What's the deadline for filing a response to a family law motion in Cook County?" — and the answer is spread across 3 different systems, some with outdated versions.

**The goal**: TerranSoul learns all of this daily, deduplicates overlapping info, detects when rules change, and gives instant answers with zero internet calls.

---

### Step-by-Step: First Day Setup

#### Step 1: Install TerranSoul + Choose Local Brain

Sarah installs TerranSoul on her work laptop (Windows 11, 32 GB RAM, no GPU).

```
┌─────────────────────────────────────────────────────────┐
│                    TerranSoul                            │
│                                                          │
│         [ 3D VRM Character — Idle Animation ]            │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │  🧠  Set up your Brain                             │  │
│  │                                                    │  │
│  │  Start chatting instantly with a free cloud LLM:   │  │
│  │                                                    │  │
│  │        [ ☁️ Use Free Cloud API (no setup) ]        │  │
│  │                                                    │  │
│  │  Intel i7-12700 · 32 GB RAM                        │  │
│  │  Or run locally: phi4:14b                          │  │
│  │    Strong reasoning, good for analysis             │  │
│  │                                                    │  │
│  │  ┌──────────────┐ ┌──────────────┐ ┌───────────┐  │  │
│  │  │ phi4:14b  ⭐ │ │ gemma3:4b    │ │ ☁️ kimi   │  │  │
│  │  └──────────────┘ └──────────────┘ └───────────┘  │  │
│  │                                                    │  │
│  │       [ ⬇ Install & activate phi4:14b ]            │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

> Sarah clicks **"🖥 Local LLM"** because firm policy requires all data to
> stay on-premise. No document content ever leaves her machine.

#### Step 2: Bulk Load Firm Knowledge

Sarah's IT admin provides a one-time ingestion script that reads from the firm's
sources with proper credentials. Here's what happens for each source type:

**Source Type A: Wiki Pages (LDAP Auth)**

```
Ingestion Script (runs on Sarah's machine)
──────────────────────────────────────────
1. Authenticate to wiki.morrisonclark.com
   → POST /api/login { user: "sarah", pass: "***" }
   → Receives session cookie

2. Fetch page list
   → GET /api/pages?space=legal-rules
   → Returns 847 wiki pages

3. For each page:
   a. Download content → GET /api/pages/{id}/content
   b. Chunk into ~500-word segments (overlap 50 words)
   c. For each chunk:
      → invoke('add_memory', {
          content: chunk_text,
          tags: "wiki,legal-rules,family-law",
          importance: 4,
          memoryType: "fact"
        })
      → TerranSoul auto-embeds via Ollama (768-dim vector)
      → Stored in SQLite with source_url + source_hash
```

**Source Type B: PDF Documents (Public / File Share)**

```
Ingestion Script
──────────────────
1. Scan \\fileserver\training\*.pdf
   → Found 120 PDF files

2. For each PDF:
   a. Extract text (via pdf-extract or similar)
   b. Split into chunks by section headings
   c. SHA-256 hash the content
   d. Check for existing hash:
      → SELECT id FROM memories WHERE source_hash = ?
      → If exists: SKIP (already ingested)
      → If new: invoke('add_memory', { ... })

3. Track source metadata:
   → source_url = "file:///fileserver/training/handbook-v12.pdf"
   → source_hash = "a1b2c3d4e5f6..."
   → expires_at = NULL (permanent)
```

**Source Type C: Court Rules Website (Public)**

```
Ingestion Script
──────────────────
1. Scrape https://ilcourts.gov/rules/family/
   → Download all rule pages

2. Each rule becomes a memory:
   → content: "Cook County Family Law Rule 14.3: ..."
   → tags: "court-rules,cook-county,family-law"
   → importance: 5  (critical — these are binding rules)
   → source_url: "https://ilcourts.gov/rules/family/14.3"
   → source_hash: SHA-256 of page content
   → expires_at: 1735689600000  (next quarterly review date)
```

#### Step 3: Verify Ingestion in Memory View

After bulk load completes, Sarah opens the Memory tab:

```
┌─────────────────────────────────────────────────────────┐
│  🧠 Memory            [⬇ Extract from session]          │
│                        [📄 Summarize session]            │
│                        [＋ Add memory]                    │
│                                                          │
│  [Graph]  [List] ← active  [Session]                     │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │ 🔍 Search memories…          [🔍 Search] [🤖 Sem] │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  Filter: [fact ●] [preference] [context] [summary]       │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │ fact          ★★★★★                                │  │
│  │ Cook County Family Law Rule 14.3: Responses to     │  │
│  │ motions must be filed within 30 days of service    │  │
│  │ [ court-rules ] [ cook-county ] [ family-law ]     │  │
│  │ 2026-04-22                              [✏] [🗑]  │  │
│  ├────────────────────────────────────────────────────┤  │
│  │ fact          ★★★★☆                                │  │
│  │ Firm Policy 7.2: All filings must be reviewed by   │  │
│  │ a senior partner before submission                 │  │
│  │ [ wiki ] [ firm-policy ] [ filing ]                │  │
│  │ 2026-04-22                              [✏] [🗑]  │  │
│  ├────────────────────────────────────────────────────┤  │
│  │ fact          ★★★★☆                                │  │
│  │ Client Smith preferences: email only, no calls     │  │
│  │ after 6pm, billing contact: jane@smith.com         │  │
│  │ [ client ] [ smith ] [ preference ]                │  │
│  │ 2026-04-22                              [✏] [🗑]  │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  Showing 3 of 12,453 memories                            │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

#### Step 4: Verify Schema Info

Sarah can check database health via the Tauri console or by asking TerranSoul:

```typescript
// From browser dev tools or a custom admin panel:
const info = await invoke('get_schema_info');
console.log(info);
```

```json
{
  "schema_version": 3,
  "target_version": 3,
  "total_memories": 12453,
  "unembedded_count": 0,
  "embedded_count": 12453,
  "db_engine": "SQLite (WAL mode)",
  "columns": {
    "id": "INTEGER PRIMARY KEY AUTOINCREMENT",
    "content": "TEXT NOT NULL — the memory text",
    "tags": "TEXT — comma-separated tags",
    "importance": "INTEGER 1-5 — priority ranking",
    "memory_type": "TEXT — fact|preference|context|summary",
    "created_at": "INTEGER — Unix timestamp (ms)",
    "last_accessed": "INTEGER — last RAG hit timestamp",
    "access_count": "INTEGER — times retrieved by RAG",
    "embedding": "BLOB — 768-dim f32 vector (little-endian)",
    "source_url": "TEXT — origin URL for ingested documents",
    "source_hash": "TEXT — content hash for dedup/staleness",
    "expires_at": "INTEGER — TTL for auto-expiry"
  }
}
```

---

### Daily Sync: Credential-Based Source Ingestion

Every morning at 6:00 AM, a scheduled task runs the sync script on Sarah's machine.
The script authenticates to each source, downloads changes, and feeds them to TerranSoul.

```
┌─────────────────────────────────────────────────────────────────┐
│                     DAILY SYNC WORKFLOW                          │
│                     (runs at 06:00 AM)                           │
│                                                                  │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐       │
│  │ Wiki (LDAP) │     │ DMS (OAuth) │     │ Court Rules │       │
│  │ 847 pages   │     │ 3,200 docs  │     │ 500 rules   │       │
│  └──────┬──────┘     └──────┬──────┘     └──────┬──────┘       │
│         │                   │                   │               │
│         ▼                   ▼                   ▼               │
│  ┌──────────────────────────────────────────────────────┐       │
│  │              Credential Manager                       │       │
│  │                                                       │       │
│  │  Wiki: LDAP bind → session cookie                     │       │
│  │  DMS:  OAuth2 client_credentials → bearer token       │       │
│  │  Court: No auth (public)                              │       │
│  │  Portal: API key from environment variable            │       │
│  │  FileShare: Windows integrated auth (Kerberos)        │       │
│  └──────────────────────┬────────────────────────────────┘       │
│                         │                                        │
│                         ▼                                        │
│  ┌──────────────────────────────────────────────────────┐       │
│  │              Change Detection                         │       │
│  │                                                       │       │
│  │  For each source document:                            │       │
│  │  1. Download content with auth token                  │       │
│  │  2. SHA-256 hash the content                          │       │
│  │  3. Compare with stored source_hash in SQLite         │       │
│  │                                                       │       │
│  │  ┌─────────────┐  ┌──────────────┐  ┌────────────┐  │       │
│  │  │ UNCHANGED   │  │ MODIFIED     │  │ NEW        │  │       │
│  │  │ hash match  │  │ hash differs │  │ no record  │  │       │
│  │  │ → SKIP      │  │ → UPDATE +   │  │ → INSERT + │  │       │
│  │  │             │  │   re-embed   │  │   embed    │  │       │
│  │  └─────────────┘  └──────────────┘  └────────────┘  │       │
│  └──────────────────────┬────────────────────────────────┘       │
│                         │                                        │
│                         ▼                                        │
│  ┌──────────────────────────────────────────────────────┐       │
│  │              Dedup Check                              │       │
│  │                                                       │       │
│  │  Before inserting new content:                        │       │
│  │  1. Embed the new text                                │       │
│  │  2. find_duplicate(embedding, threshold=0.97)         │       │
│  │  3. Cosine > 0.97? → SKIP (near-duplicate)           │       │
│  │     Cosine < 0.97? → INSERT as new memory             │       │
│  └──────────────────────┬────────────────────────────────┘       │
│                         │                                        │
│                         ▼                                        │
│  ┌──────────────────────────────────────────────────────┐       │
│  │              Expiry Cleanup                           │       │
│  │                                                       │       │
│  │  DELETE FROM memories                                 │       │
│  │  WHERE expires_at IS NOT NULL                         │       │
│  │    AND expires_at < current_timestamp                  │       │
│  │                                                       │       │
│  │  → Removes: old court calendars, temp announcements   │       │
│  └──────────────────────────────────────────────────────┘       │
│                                                                  │
│  SYNC REPORT (emailed to admin):                                │
│  ─────────────────────────────────────                          │
│  Sources checked:     5                                          │
│  Documents scanned:   4,547                                      │
│  Unchanged (skipped): 4,489                                      │
│  Modified (updated):  23                                         │
│  New (inserted):      35                                         │
│  Duplicates (skipped):7                                          │
│  Expired (deleted):   12                                         │
│  Embeddings created:  58                                         │
│  Total sync time:     4m 32s                                     │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

#### Example Sync Script (TypeScript — runs as a scheduled Tauri sidecar)

```typescript
// sync-firm-knowledge.ts — Daily knowledge sync for Morrison & Clark LLP
import { invoke } from '@tauri-apps/api/core';
import { createHash } from 'crypto';

interface SyncSource {
  name: string;
  type: 'wiki' | 'dms' | 'web' | 'fileshare';
  baseUrl: string;
  auth: AuthConfig;
  tags: string;
  importance: number;
  expiresInDays?: number;
}

interface AuthConfig {
  method: 'ldap' | 'oauth2' | 'apikey' | 'kerberos' | 'none';
  credentials?: string;  // env var name, NEVER hardcoded
}

const SOURCES: SyncSource[] = [
  {
    name: 'Firm Wiki',
    type: 'wiki',
    baseUrl: 'https://wiki.morrisonclark.com/api',
    auth: { method: 'ldap', credentials: 'WIKI_SESSION_TOKEN' },
    tags: 'wiki,firm-rules',
    importance: 4,
  },
  {
    name: 'Court Rules',
    type: 'web',
    baseUrl: 'https://ilcourts.gov/rules',
    auth: { method: 'none' },
    tags: 'court-rules,illinois',
    importance: 5,
    expiresInDays: 90,  // re-check quarterly
  },
  {
    name: 'Client Portal',
    type: 'dms',
    baseUrl: 'https://portal.morrisonclark.com/api/v2',
    auth: { method: 'oauth2', credentials: 'PORTAL_OAUTH_TOKEN' },
    tags: 'client,portal',
    importance: 3,
  },
];

async function syncSource(source: SyncSource): Promise<SyncStats> {
  const stats = { scanned: 0, unchanged: 0, modified: 0, inserted: 0, duplicates: 0 };

  // 1. Authenticate (credentials from environment, NEVER hardcoded)
  const token = await authenticate(source.auth);

  // 2. Fetch document list from source
  const documents = await fetchDocumentList(source.baseUrl, token);

  for (const doc of documents) {
    stats.scanned++;

    // 3. Download content
    const content = await fetchDocument(doc.url, token);
    const hash = createHash('sha256').update(content).digest('hex');

    // 4. Check if this URL already exists with same hash
    const existing = await invoke<any[]>('search_memories', {
      query: doc.url,
    });

    const match = existing.find(m => m.source_url === doc.url);

    if (match && match.source_hash === hash) {
      // Content unchanged — skip
      stats.unchanged++;
      continue;
    }

    // 5. Chunk the content (for long documents)
    const chunks = chunkText(content, 500, 50);  // 500 words, 50 overlap

    for (const chunk of chunks) {
      // 6. Check for semantic duplicates
      const isDuplicate = await invoke<number | null>('check_duplicate', {
        content: chunk,
      });

      if (isDuplicate) {
        stats.duplicates++;
        continue;
      }

      // 7. Insert new memory (auto-embedded by TerranSoul)
      const expiresAt = source.expiresInDays
        ? Date.now() + source.expiresInDays * 86400000
        : undefined;

      await invoke('add_memory', {
        content: chunk,
        tags: source.tags,
        importance: source.importance,
        memoryType: 'fact',
        // V3 columns:
        sourceUrl: doc.url,
        sourceHash: hash,
        expiresAt,
      });

      if (match) {
        stats.modified++;
      } else {
        stats.inserted++;
      }
    }
  }

  return stats;
}

function chunkText(text: string, maxWords: number, overlap: number): string[] {
  const words = text.split(/\s+/);
  const chunks: string[] = [];
  for (let i = 0; i < words.length; i += maxWords - overlap) {
    chunks.push(words.slice(i, i + maxWords).join(' '));
    if (i + maxWords >= words.length) break;
  }
  return chunks;
}
```

---

### Handling Duplicates Across Sources

The same fact often appears in multiple places:

```
EXAMPLE: "Family law responses must be filed within 30 days"

Source 1: Firm Wiki → /rules/family-law/deadlines
  "A response to a family law motion must be filed
   within thirty (30) days of service."

Source 2: Court Rules → ilcourts.gov/rules/family/14.3
  "Responses to motions in family law matters shall
   be filed within 30 days after service."

Source 3: Training Manual → handbook-v12.pdf, page 47
  "Filing deadline: 30 days from service date for
   all family law motion responses."
```

Without dedup, TerranSoul would store **3 separate memories** saying the same thing,
wasting space and potentially confusing RAG ranking.

#### How TerranSoul Handles This

```
Step 1: Source 1 arrives first
  → embed_text("A response to a family law motion...")
  → embedding = [0.82, -0.31, 0.56, ...]
  → No existing duplicate found (empty DB)
  → INSERT → id=1

Step 2: Source 2 arrives
  → embed_text("Responses to motions in family law...")
  → embedding = [0.83, -0.30, 0.55, ...]
  → find_duplicate(embedding, threshold=0.97)
  → cosine_similarity with id=1 = 0.98  ← ABOVE THRESHOLD
  → SKIP — this is a near-duplicate of id=1

Step 3: Source 3 arrives
  → embed_text("Filing deadline: 30 days from service...")
  → embedding = [0.81, -0.32, 0.57, ...]
  → find_duplicate(embedding, threshold=0.97)
  → cosine_similarity with id=1 = 0.97  ← AT THRESHOLD
  → SKIP — still a near-duplicate

Result: Only 1 memory stored instead of 3
  → id=1 "A response to a family law motion must be filed
           within thirty (30) days of service."
  → source_url = wiki (first source wins)
```

#### Dedup in the SQLite Database

```sql
-- Find all memories from the same source that might be redundant
SELECT a.id, b.id AS similar_to, a.content, a.source_url
FROM memories a
JOIN memories b ON a.id > b.id
WHERE a.source_url != b.source_url
  AND a.embedding IS NOT NULL
  AND b.embedding IS NOT NULL;
-- Note: Actual cosine comparison happens in Rust, not SQL

-- Admin query: find exact text duplicates (not just semantic)
SELECT content, COUNT(*) AS copies, GROUP_CONCAT(id) AS ids
FROM memories
GROUP BY content
HAVING copies > 1;
```

---

### Detecting and Removing Stale/Out-of-Date Knowledge

Legal rules change. Court deadlines shift. Firm policies get revised.
TerranSoul uses three mechanisms to keep knowledge current:

#### Mechanism 1: Source Hash Change Detection

```
Monday (initial sync):
  Court Rule 14.3 → hash = "a1b2c3d4"
  → Stored: id=42, source_hash="a1b2c3d4"

Tuesday (daily sync):
  Court Rule 14.3 → hash = "a1b2c3d4"  (same)
  → SKIP — content unchanged

Wednesday (rule amended):
  Court Rule 14.3 → hash = "e5f6g7h8"  (DIFFERENT!)
  → Detected: source_hash mismatch for source_url
  → Action:
    1. DELETE old memory (id=42)
    2. INSERT new content with new hash
    3. Auto-embed the new content
    4. Log: "Updated: Court Rule 14.3 — content changed"
```

```
┌──────────────────────────────────────────────────────────┐
│                  STALENESS DETECTION                      │
│                                                           │
│   Source URL             Stored Hash    Current Hash      │
│   ──────────────────     ───────────    ────────────      │
│   /rules/family/14.3    a1b2c3d4       e5f6g7h8   ← STALE│
│   /rules/family/14.4    f9g0h1i2       f9g0h1i2   ✓ OK   │
│   /rules/civil/22.1     j3k4l5m6       j3k4l5m6   ✓ OK   │
│   /policies/billing     n7o8p9q0       r1s2t3u4   ← STALE│
│                                                           │
│   Action: 2 memories updated, 2 re-embedded               │
└──────────────────────────────────────────────────────────┘
```

#### Mechanism 2: TTL Expiry (`expires_at` Column)

Some knowledge has a natural shelf life:

```sql
-- Set expiry when ingesting time-sensitive content
INSERT INTO memories (content, tags, importance, memory_type,
                      created_at, source_url, expires_at)
VALUES (
  'Court closed Dec 25, 2026 – Jan 2, 2027 for holiday recess',
  'court-calendar,holiday',
  3,
  'fact',
  1713744000000,
  'https://ilcourts.gov/calendar/2026',
  1735776000000   -- expires Jan 2, 2027
);

-- Daily cleanup: remove expired memories
DELETE FROM memories
WHERE expires_at IS NOT NULL AND expires_at < strftime('%s','now') * 1000;
```

| Content Type | Typical TTL | Example |
|---|---|---|
| Court calendar events | Until event date + 1 day | "Hearing on May 15" |
| Holiday schedules | Until end of holiday | "Closed Dec 25-Jan 2" |
| Temporary policies | Duration of policy | "COVID masking required" |
| Client case deadlines | Until deadline + 7 days | "Smith motion due Apr 30" |
| Permanent rules | No expiry (`NULL`) | "30-day filing deadline" |

#### Mechanism 3: Access Count Decay (Unused Knowledge)

Memories that RAG never retrieves are probably not useful:

```sql
-- Find memories older than 90 days that were never accessed by RAG
SELECT id, content, created_at, access_count
FROM memories
WHERE access_count = 0
  AND created_at < strftime('%s','now') * 1000 - 7776000000  -- 90 days
ORDER BY created_at ASC;

-- Example output:
-- id   | content                              | access_count
-- 847  | 2025 holiday party menu choices       | 0
-- 1203 | Old phone extension list (pre-move)   | 0
-- 1456 | Draft policy that was never adopted   | 0
```

Admin can periodically review and prune these:

```typescript
// Cleanup script: archive zero-access memories older than 90 days
const stale = await invoke<MemoryEntry[]>('search_memories', {
  query: '',  // get all, then filter
});

const ninetyDaysAgo = Date.now() - 90 * 24 * 60 * 60 * 1000;
const unused = stale.filter(m =>
  m.access_count === 0 && m.created_at < ninetyDaysAgo
);

for (const m of unused) {
  // Archive to CSV before deleting
  appendToArchive(m);
  await invoke('delete_memory', { id: m.id });
}
console.log(`Archived and deleted ${unused.length} stale memories`);
```

---

### Day-in-the-Life: Attorney Sarah Uses TerranSoul

#### 8:30 AM — Sarah Opens TerranSoul

The daily sync already ran at 6:00 AM. TerranSoul has 12,500+ memories
from all firm sources, fully embedded and searchable.

```
┌─────────────────────────────────────────────────────────┐
│                                                          │
│         [ 3D VRM Character — Good morning wave ]         │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │                                                    │  │
│  │  You:  Good morning! I need to prepare a response  │  │
│  │        to the Smith family law motion. What are     │  │
│  │        the deadlines and filing requirements?       │  │
│  │                                                    │  │
│  │  ────────────────────────────────────────────────  │  │
│  │                                                    │  │
│  │  🤖:  Good morning! For the Smith case, here's     │  │
│  │       what I found:                                │  │
│  │                                                    │  │
│  │       **Deadline**: Per Cook County Family Law      │  │
│  │       Rule 14.3, you have 30 days from service     │  │
│  │       to file your response.                       │  │
│  │                                                    │  │
│  │       **Firm Policy**: Policy 7.2 requires         │  │
│  │       senior partner review before submission.     │  │
│  │                                                    │  │
│  │       **Client Preference**: Mr. Smith prefers     │  │
│  │       email communication only, no calls after     │  │
│  │       6pm. Billing contact: jane@smith.com         │  │
│  │                                                    │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │ Type a message…                          [Send ➤]  │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

**Behind the scenes** — the RAG pipeline found 3 relevant memories in **<5ms**:

```
[LONG-TERM MEMORY]
- Cook County Family Law Rule 14.3: Responses to motions must be
  filed within 30 days of service.
- Firm Policy 7.2: All filings must be reviewed by a senior partner
  before submission.
- Client Smith preferences: email only, no calls after 6pm, billing
  contact: jane@smith.com
[/LONG-TERM MEMORY]
```

#### 10:00 AM — Sarah Learns Something New in a Meeting

Sarah: "I just learned that Judge Martinez's courtroom has a new
local rule — all motions must include a certificate of compliance
starting May 1st."

```
┌────────────────────────────────────────────────────────┐
│                                                         │
│  You:  Remember this: Judge Martinez's courtroom now    │
│        requires a certificate of compliance for all     │
│        motions starting May 1st, 2026.                  │
│                                                         │
│  🤖:   Got it! I've stored that as a high-importance    │
│        fact. I'll remember this for future filings      │
│        in Judge Martinez's courtroom.                   │
│                                                         │
└────────────────────────────────────────────────────────┘
```

TerranSoul auto-extracts this as a memory:

```sql
INSERT INTO memories
  (content, tags, importance, memory_type, created_at, access_count)
VALUES
  ('Judge Martinez courtroom: certificate of compliance
   required for all motions starting May 1st, 2026',
   'judge-martinez,compliance,motions,new-rule',
   5,
   'fact',
   1713780000000,
   0);
-- Auto-embedded via Ollama → 768-dim vector stored as BLOB
```

#### 2:00 PM — Sarah Asks About a Rule That Changed Overnight

The daily sync detected that Court Rule 22.1 was amended:

```
┌────────────────────────────────────────────────────────┐
│                                                         │
│  You:  What's the current procedure for filing a        │
│        civil motion in Cook County?                     │
│                                                         │
│  🤖:   Based on the updated Cook County Civil Rule      │
│        22.1 (amended April 21, 2026):                   │
│                                                         │
│        1. Electronic filing is now MANDATORY (was        │
│           optional before the amendment)                 │
│        2. Paper copies are no longer accepted            │
│        3. Filing fee increased to $350                   │
│                                                         │
│        Note: This rule was updated yesterday. The        │
│        previous version required only e-filing for       │
│        cases over $50,000.                               │
│                                                         │
└────────────────────────────────────────────────────────┘
```

**This works because** the daily sync at 6:00 AM:
1. Downloaded the new version of Rule 22.1
2. Detected the hash mismatch (`old_hash ≠ new_hash`)
3. Deleted the old memory
4. Inserted the new text
5. Auto-embedded the new version
6. Sarah gets the updated rule — automatically, no manual intervention

#### 5:00 PM — End of Day: Extract Session Memories

Sarah clicks "⬇ Extract from session" in the Memory tab:

```
┌─────────────────────────────────────────────────────────┐
│  🧠 Memory            [⬇ Extract from session]          │
│                        Working… extracting facts         │
│                                                          │
│  Extracted 4 new memories from today's session:          │
│                                                          │
│  ✓ "Judge Martinez requires certificate of compliance"   │
│  ✓ "Smith case response deadline is April 30th"          │
│  ✓ "New e-filing mandate for civil motions"              │
│  ✓ "Sarah prefers to prepare filings in the morning"     │
│                                                          │
│  Total memories: 12,457 (was 12,453 this morning)        │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

These session facts are now part of the permanent long-term memory,
available for RAG in all future conversations.

---

## Debugging with SQLite

### Where is the Database?

The database file lives in the Tauri app data directory:

| OS | Path |
|---|---|
| **Windows** | `%APPDATA%\com.terransoul.app\memory.db` |
| **macOS** | `~/Library/Application Support/com.terransoul.app/memory.db` |
| **Linux** | `~/.local/share/com.terransoul.app/memory.db` |

### Opening SQLite: Recommended Tools

#### Option 1: DB Browser for SQLite (GUI — Recommended for beginners)

1. **Download**: https://sqlitebrowser.org/dl/
2. **Install**: Run the installer (Windows: `.msi`, macOS: `.dmg`)
3. **Open**: File → Open Database → Navigate to `memory.db`

```
┌─ DB Browser for SQLite ────────────────────────────────┐
│ File  Edit  View  Tools  Help                           │
│                                                          │
│ Database: memory.db                                      │
│                                                          │
│ Tables:                                                  │
│  ├── memories (15,247 rows)                              │
│  └── schema_version (3 rows)                             │
│                                                          │
│ ┌──────────────────────────────────────────────────────┐ │
│ │ Browse Data │ Execute SQL │ DB Structure │ Edit Prag │ │
│ ├──────────────────────────────────────────────────────┤ │
│ │ Table: memories ▾                                    │ │
│ ├────┬──────────────────────┬──────┬─────┬─────┬──────┤ │
│ │ id │ content              │ tags │ imp │ type│ embed│ │
│ ├────┼──────────────────────┼──────┼─────┼─────┼──────┤ │
│ │  1 │ Filing deadline: 30d │ law  │  5  │ fact│ BLOB │ │
│ │  2 │ Client prefers email │ pref │  4  │ pref│ BLOB │ │
│ │  3 │ Office hours M-F 9-5 │ info │  2  │ fact│ BLOB │ │
│ │ .. │ ...                  │      │     │     │      │ │
│ └────┴──────────────────────┴──────┴─────┴─────┴──────┘ │
└─────────────────────────────────────────────────────────┘
```

#### Option 2: sqlite3 CLI (Terminal — for power users)

Pre-installed on macOS/Linux. Windows: download from https://sqlite.org/download.html

```bash
# Open the database
sqlite3 "%APPDATA%/com.terransoul.app/memory.db"

# Show tables
.tables
# → memories  schema_version

# Show schema
.schema memories

# Check schema version
SELECT * FROM schema_version;
# → 1|1713744000|initial memories table
# → 2|1713744001|add embedding column for vector search
# → 3|1713744002|add source metadata columns for document ingestion
```

#### Option 3: VS Code Extension

Install "SQLite Viewer" (id: `qwtel.sqlite-viewer`) in VS Code, then open `memory.db` directly.

### Useful Debug Queries

#### Check how many memories have embeddings

```sql
SELECT
    COUNT(*) AS total,
    COUNT(embedding) AS embedded,
    COUNT(*) - COUNT(embedding) AS unembedded
FROM memories;

-- Example output:
-- total | embedded | unembedded
-- 15247 |    15200 |         47
```

#### Find the most-accessed memories (RAG hits)

```sql
SELECT id, content, access_count, last_accessed
FROM memories
ORDER BY access_count DESC
LIMIT 10;

-- Example output:
-- id | content                        | access_count | last_accessed
--  2 | Family law filings: 30-day...  |          147 | 1713800000000
--  5 | Section 14.3: Motion response  |          133 | 1713799500000
```

#### Find memories that RAG never retrieved

```sql
SELECT id, content, created_at
FROM memories
WHERE access_count = 0
ORDER BY created_at DESC
LIMIT 20;
```

#### Check for orphaned embeddings (size validation)

```sql
SELECT id, content, LENGTH(embedding) AS embed_bytes,
       LENGTH(embedding) / 4 AS dimensions
FROM memories
WHERE embedding IS NOT NULL
LIMIT 5;

-- Expected: 3072 bytes = 768 dimensions × 4 bytes/float
-- id | content     | embed_bytes | dimensions
--  1 | Filing...   |        3072 |        768
```

#### View schema migration history

```sql
SELECT version, description,
       datetime(applied_at / 1000, 'unixepoch', 'localtime') AS applied
FROM schema_version
ORDER BY version;

-- version | description                                  | applied
--       1 | initial memories table                       | 2026-04-20 10:00:00
--       2 | add embedding column for vector search       | 2026-04-20 10:00:01
--       3 | add source metadata columns for doc ingest   | 2026-04-22 14:30:00
```

#### Find duplicate content

```sql
SELECT a.id, b.id AS dup_id, a.content
FROM memories a
JOIN memories b ON a.id < b.id AND a.content = b.content;
```

#### Check database health

```sql
-- File size and page count
PRAGMA page_count;
PRAGMA page_size;

-- WAL mode verification
PRAGMA journal_mode;
-- → wal

-- Integrity check
PRAGMA integrity_check;
-- → ok

-- Database statistics
SELECT
    (SELECT COUNT(*) FROM memories) AS total_memories,
    (SELECT COUNT(*) FROM schema_version) AS schema_versions,
    (SELECT MAX(version) FROM schema_version) AS current_version;
```

### Common Debugging Scenarios

#### "My memories aren't being found by RAG"

```sql
-- Check if the memory has an embedding
SELECT id, content, embedding IS NOT NULL AS has_embedding
FROM memories
WHERE content LIKE '%your search term%';

-- If has_embedding = 0, run backfill:
-- In TerranSoul: invoke('backfill_embeddings')
```

#### "RAG is returning irrelevant results"

```sql
-- Check what's being accessed most recently
SELECT id, content, access_count,
       datetime(last_accessed / 1000, 'unixepoch', 'localtime') AS last_hit
FROM memories
WHERE last_accessed IS NOT NULL
ORDER BY last_accessed DESC
LIMIT 10;

-- Look for low-importance entries polluting results
SELECT id, content, importance
FROM memories
WHERE importance <= 2 AND access_count > 10;
-- Consider increasing importance or deleting irrelevant entries
```

#### "Database seems corrupted"

```sql
-- Run integrity check
PRAGMA integrity_check;

-- If not "ok", restore from backup:
-- 1. Close TerranSoul
-- 2. Copy memory.db.bak → memory.db
-- 3. Reopen TerranSoul (migrations will re-apply if needed)
```

#### "I want to reset everything"

```sql
-- Delete all memories (keeps schema)
DELETE FROM memories;

-- Or delete the file entirely:
-- Close TerranSoul → delete memory.db → reopen (fresh database created)
```

---

## Hardware Scaling

### How Memory Count Maps to Hardware

| Memory Count | Embedding Storage | RAM Usage | Search Time | Recommended Hardware |
|---|---|---|---|---|
| 1,000 | 3 MB | ~50 MB | <1 ms | Any modern PC |
| 10,000 | 30 MB | ~100 MB | ~2 ms | 8 GB RAM |
| 100,000 | 300 MB | ~500 MB | ~5 ms | 16 GB RAM |
| 1,000,000 | 3 GB | ~4 GB | ~50 ms | 32 GB RAM |
| 10,000,000 | 30 GB | ~35 GB | ~500 ms | 64 GB RAM |

### Your Hardware (65 GB RAM, RTX 3080 Ti)

With 65 GB RAM, you can comfortably hold **~10 million memory entries** in the vector index:

```
Your capacity:
├── Chat model (e.g., gemma3:12b):  ~8 GB VRAM
├── Embedding model (nomic-embed):  ~300 MB VRAM
├── OS + Apps:                      ~8 GB RAM
├── Available for memory index:     ~49 GB RAM
│
├── At 3 KB per embedding:
│   49 GB / 3 KB = ~16 million entries
│
└── Practical limit: ~10 million entries
    (leaves headroom for SQLite, OS cache, etc.)
```

### Scaling Beyond 10 Million

For datasets exceeding 10M entries (unlikely for a desktop companion), the next step would be:
- **HNSW index**: Approximate Nearest Neighbor search — O(log n) instead of O(n)
- **Sharding**: Split memories across multiple SQLite files by date/topic
- **External vector DB**: Connect to Qdrant/Milvus as a Tauri sidecar

The current pure-cosine approach is intentionally simple and works for the vast majority of use cases.

---

## FAQ

### "How do I ask TerranSoul about SQLite?"

Just ask in the chat! If TerranSoul has memories about SQLite (or you add them), it will use RAG to answer:

> **You**: "How do I check which schema version the database is on?"  
> **TerranSoul**: "You can check the schema version by querying `SELECT * FROM schema_version ORDER BY version DESC LIMIT 1;` in DB Browser for SQLite. The current target version is 3, which includes the source tracking columns for document ingestion."

### "What if Ollama is not running?"

TerranSoul gracefully degrades:
- **Vector search**: Skipped (no embedding available for query)
- **Fallback**: Keyword search across `content` and `tags` columns
- **Chat**: Uses Free Cloud API if configured as backup

### "Can I export/import memories?"

```sql
-- Export to CSV
.mode csv
.headers on
.output memories_backup.csv
SELECT id, content, tags, importance, memory_type, created_at FROM memories;
.output stdout

-- Import from CSV
.mode csv
.import memories_backup.csv memories
```

### "How do I add memories programmatically?"

Via Tauri IPC from the frontend:

```typescript
import { invoke } from '@tauri-apps/api/core';

// Add a single memory (auto-embedded if brain is configured)
await invoke('add_memory', {
  content: 'Court filing deadline is 30 days from service',
  tags: 'law,deadline,filing',
  importance: 5,
  memoryType: 'fact',
});

// Backfill embeddings for all un-embedded entries
const count = await invoke<number>('backfill_embeddings');
console.log(`Embedded ${count} new entries`);

// Check database schema info
const info = await invoke('get_schema_info');
console.log(info);
// {
//   schema_version: 3,
//   target_version: 3,
//   total_memories: 15247,
//   unembedded_count: 47,
//   embedded_count: 15200,
//   db_engine: "SQLite (WAL mode)",
//   columns: { ... }
// }
```

### "How do I check the database columns?"

Call `get_schema_info` from the UI or chat. It returns all column definitions:

| Column | Type | Description |
|---|---|---|
| `id` | INTEGER PRIMARY KEY | Auto-incrementing unique ID |
| `content` | TEXT NOT NULL | The memory text |
| `tags` | TEXT | Comma-separated tags for categorization |
| `importance` | INTEGER (1-5) | Priority ranking (5 = most important) |
| `memory_type` | TEXT | `fact`, `preference`, `context`, or `summary` |
| `created_at` | INTEGER | Unix timestamp in milliseconds |
| `last_accessed` | INTEGER | Last time RAG retrieved this memory |
| `access_count` | INTEGER | Number of times retrieved by RAG |
| `embedding` | BLOB | 768-dim f32 vector (3,072 bytes) |
| `source_url` | TEXT | Origin URL for ingested documents |
| `source_hash` | TEXT | SHA-256 hash for dedup/staleness |
| `expires_at` | INTEGER | TTL timestamp for auto-expiry |

### "What's the difference between search and semantic search?"

| Feature | `search_memories` | `semantic_search_memories` |
|---|---|---|
| Method | SQL `LIKE '%keyword%'` | Cosine similarity on embeddings |
| Speed | <1ms (any size) | ~50ms embed + <5ms search |
| Accuracy | Exact match only | Understands meaning |
| Requires Brain | No | Yes (Ollama for embedding) |
| Example | "deadline" finds "deadline" | "when to file" finds "30-day deadline" |
