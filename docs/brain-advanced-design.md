# Brain & Memory — Advanced Architecture Design

> **TerranSoul v0.1** — Self-learning AI companion with persistent memory  
> Last updated: 2026-05-02
> **Audience**: Developers, contributors, and architects who need to understand the full memory/brain system.

---

## Table of Contents

1. [System Overview](#system-overview)
2. [Three-Tier Memory Model](#three-tier-memory-model)
   - [Short-Term Memory](#short-term-memory)
   - [Working Memory](#working-memory)
   - [Long-Term Memory](#long-term-memory)
   - [Tier Lifecycle & Promotion Chain](#tier-lifecycle--promotion-chain)
3. [Memory Categories (Ontology)](#memory-categories-ontology)
   - [Core Types](#core-types)
   - [Proposed Category Taxonomy](#proposed-category-taxonomy)
   - [Category × Tier Matrix](#category--tier-matrix)
   - [Cognitive Memory Axes (Episodic / Semantic / Procedural)](#35-cognitive-memory-axes-episodic--semantic--procedural)
4. [Hybrid RAG Pipeline](#hybrid-rag-pipeline)
   - [6-Signal Scoring Formula](#6-signal-scoring-formula)
   - [RAG Injection Flow](#rag-injection-flow)
  - [Tool Plugins And Brain Routing](#tool-plugins-and-brain-routing)
  - [Memory Plugin Hooks](#memory-plugin-hooks)
   - [Embedding & Vector Search](#embedding--vector-search)
5. [Decay & Garbage Collection](#decay--garbage-collection)
6. [Knowledge Graph Vision](#knowledge-graph-vision)
   - [Current: Tag-Based Graph](#current-tag-based-graph)
   - [Implemented (V5): Entity-Relationship Graph](#implemented-v5-entity-relationship-graph)
   - [Graph Traversal for Multi-Hop RAG](#graph-traversal-for-multi-hop-rag)
7. [Visualization Layers](#visualization-layers)
   - [Layer 1: In-App (Cytoscape.js)](#layer-1-in-app-cytoscapejs)
   - [Layer 2: Obsidian Vault Export](#layer-2-obsidian-vault-export)
   - [Layer 3: Debug SQL Console](#layer-3-debug-sql-console)
8. [SQLite Schema](#sqlite-schema)
9. [Why SQLite?](#why-sqlite)
10. [Brain Modes & Provider Architecture](#brain-modes--provider-architecture)
11. [LLM-Powered Memory Operations](#llm-powered-memory-operations)
12. [Multi-Source Knowledge Management](#multi-source-knowledge-management)
    - [Source Hash Change Detection](#1-source-hash-change-detection)
    - [TTL Expiry](#2-ttl-expiry)
    - [Access Count Decay](#3-access-count-decay)
    - [LLM-Powered Conflict Resolution](#4-llm-powered-conflict-resolution)
13. [Open-Source RAG Ecosystem Comparison](#open-source-rag-ecosystem-comparison)
14. [Debugging with SQLite](#debugging-with-sqlite)
15. [Hardware Scaling](#hardware-scaling)
16. [Scaling Roadmap](#scaling-roadmap)
17. [FAQ](#faq)
18. [Diagrams Index](#diagrams-index)
19. [April 2026 Research Survey — Modern RAG & Agent-Memory Techniques](#april-2026-research-survey--modern-rag--agent-memory-techniques)
20. [Brain Component Selection & Routing — How the LLM Knows What to Use](#brain-component-selection--routing--how-the-llm-knows-what-to-use)
21. [How Daily Conversation Updates the Brain — Write-Back / Learning Loop](#how-daily-conversation-updates-the-brain--write-back--learning-loop)
22. [Code-Intelligence Bridge — GitNexus Sidecar (Phase 13 Tier 1)](#code-intelligence-bridge--gitnexus-sidecar-phase-13-tier-1)
23. [Code-RAG Fusion in `rerank_search_memories` (Phase 13 Tier 2)](#code-rag-fusion-in-rerank_search_memories-phase-13-tier-2)
24. [MCP Server — External AI Coding Assistant Integration (Phase 15)](#mcp-server--external-ai-coding-assistant-integration-phase-15)
25. [Intent Classification](#25-intent-classification)

---

## 1. System Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          TerranSoul Desktop App                            │
│                                                                             │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                     FRONTEND (Vue 3 + TypeScript)                    │   │
│  │                                                                      │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌────────────────────────────┐ │   │
│  │  │  ChatView    │  │ MemoryView   │  │ SkillTreeView              │ │   │
│  │  │              │  │              │  │                            │ │   │
│  │  │ • Send msg   │  │ • List/Grid  │  │ • Quest-guided discovery  │ │   │
│  │  │ • Stream res │  │ • Graph viz  │  │ • "Sage's Library" quest  │ │   │
│  │  │ • Subtitles  │  │ • Tier chips │  │   unlocks RAG features    │ │   │
│  │  └──────┬───────┘  │ • Filters    │  └────────────────────────────┘ │   │
│  │         │          │ • Search     │                                  │   │
│  │         │          │ • Add/Edit   │                                  │   │
│  │         │          │ • Decay viz  │                                  │   │
│  │         │          └──────┬───────┘                                  │   │
│  │         │                 │                                          │   │
│  │  ┌──────▼─────────────────▼──────────────────────────────────────┐  │   │
│  │  │                    Pinia Stores                                │  │   │
│  │  │                                                                │  │   │
│  │  │  brain.ts ──── conversation.ts ──── memory.ts ──── voice.ts   │  │   │
│  │  │  (provider)    (chat + stream)      (CRUD + search) (TTS/ASR) │  │   │
│  │  └──────────────────────┬────────────────────────────────────────┘  │   │
│  └─────────────────────────┼────────────────────────────────────────────┘   │
│                            │ Tauri IPC (invoke / emit)                      │
│  ┌─────────────────────────▼────────────────────────────────────────────┐   │
│  │                     BACKEND (Rust + Tokio)                           │   │
│  │                                                                      │   │
│  │  ┌────────────────────────────────────────────────────────────────┐  │   │
│  │  │                   Commands Layer (60+)                         │  │   │
│  │  │  chat.rs • streaming.rs • memory.rs • brain.rs • voice.rs     │  │   │
│  │  └────────┬──────────────────────────────┬────────────────────────┘  │   │
│  │           │                              │                           │   │
│  │  ┌────────▼────────┐           ┌─────────▼─────────┐               │   │
│  │  │  Brain Module   │           │  Memory Module     │               │   │
│  │  │                 │           │                    │               │   │
│  │  │ • OllamaAgent  │◄─────────►│ • MemoryStore      │               │   │
│  │  │ • OpenAiClient │  RAG loop │ • brain_memory.rs   │               │   │
│  │  │ • FreeProvider  │           │ • hybrid_search()  │               │   │
│  │  │ • embed_text() │           │ • vector_search()  │               │   │
│  │  │ • ProviderRotat│           │ • decay / gc       │               │   │
│  │  └────────┬────────┘           └─────────┬──────────┘               │   │
│  │           │                              │                           │   │
│  │           │      ┌───────────────────────▼──────┐                   │   │
│  │           │      │  SQLite (WAL mode)           │                   │   │
│  │           │      │  memory.db                   │                   │   │
│  │           │      │                              │                   │   │
│  │           │      │  memories ─┬─ content        │                   │   │
│  │           │      │            ├─ embedding BLOB  │                   │   │
│  │           │      │            ├─ tier            │                   │   │
│  │           │      │            ├─ memory_type     │                   │   │
│  │           │      │            ├─ tags            │                   │   │
│  │           │      │            ├─ importance      │                   │   │
│  │           │      │            ├─ decay_score     │                   │   │
│  │           │      │            └─ source_*        │                   │   │
│  │           │      └──────────────────────────────┘                   │   │
│  │           │                                                          │   │
│  │  ┌────────▼──────────────────────────┐                              │   │
│  │  │  External LLM Providers          │                              │   │
│  │  │                                   │                              │   │
│  │  │  • Ollama (localhost:11434)       │                              │   │
│  │  │  • Pollinations (free API)       │                              │   │
│  │  │  • OpenAI / Anthropic / Groq     │                              │   │
│  │  └───────────────────────────────────┘                              │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Browser mode surface

The Vue bundle also supports a browser-only mode for the public TerranSoul
landing page and live model testing on Vercel. This web surface is not a Tauri
release package or installer flow: when `App.vue` cannot reach Tauri IPC, it
routes to a static product landing page, auto-configures the free cloud brain
path, and keeps the real Three.js/VRM character mounted as a forced pet-mode
preview. Opening "3D" or "Chat" from the landing page creates a compact
responsive in-page app window with dialog semantics and mobile-safe sizing; it
uses the same Pinia stores as desktop, but native-only commands fall back to
browser-native storage and direct provider calls unless a remote host is paired.

Browser mode therefore exercises the real frontend brain contract without
claiming local desktop capabilities: Free API chat can run directly in the web
client, the Vercel onboarding presents one-click "Authorize with Google",
"Authorize with ChatGPT", and instant free demo buttons for non-technical users,
and the selected web test session is remembered in the `brain` Pinia store plus
localStorage without manual API-key input. Local/remote brain paths still require
an explicit paired TerranSoul host. The browser transport resolver rejects local
Ollama/LM Studio as direct browser transports so the UI never implies Rust-backed
memory or localhost LLM access is available without RemoteHost pairing.

Browser RAG is implemented by `src/transport/browser-rag.ts` and backs the
`memory` Pinia store whenever Tauri IPC is unavailable. It persists memory
records to IndexedDB with a localStorage mirror, computes browser embeddings via
a deterministic Transformers.js-compatible seam, performs flat vector search
with keyword/freshness/importance/decay/tier scoring, fuses vector + keyword +
freshness rankings with RRF (`k=60`), exposes a HyDE prompt builder for direct
provider calls, applies a simplified local rerank through the same hybrid score,
and exports/imports a versioned sync payload that can be stored in the user's
Google Drive file flow. Browser chat injects top browser-RAG hits as the same
`[LONG-TERM MEMORY]` block used by desktop and writes completed browser chat
turns back into local memory. Browser-mode QA is covered by focused Vue tests for
landing anchors, one-click browser authorization, forced pet-preview wiring,
manga-style pet emotion bubbles, app-window launch events, browser memory
storage/search, browser sync payloads, and prompt injection.

---

## 2. Three-Tier Memory Model

TerranSoul's memory mirrors human cognition: **short-term** (seconds–minutes), **working** (session-scoped), and **long-term** (permanent knowledge base).

### Short-Term Memory

```
┌─────────────────────────────────────────────────────────────────┐
│                    SHORT-TERM MEMORY                             │
│                                                                  │
│  Storage:   Rust Vec<Message> in AppState (in-memory)           │
│  Capacity:  Last ~20 messages                                    │
│  Lifetime:  Current session only — lost on app close            │
│  Purpose:   Conversation continuity ("what did I just say?")    │
│  Injected:  Always — appended to LLM prompt as chat history    │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ [user]      "What are the filing rules?"                  │   │
│  │ [assistant] "Family law filings require 30-day notice..." │   │
│  │ [user]      "What about emergency motions?"               │   │
│  │ [assistant] "Emergency motions can be filed same-day..."  │   │
│  │ ... (last 20 messages, FIFO eviction)                     │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  Eviction:  When buffer exceeds 20, oldest messages are         │
│             candidates for extraction → working memory           │
└─────────────────────────────────────────────────────────────────┘
```

### Working Memory

```
┌─────────────────────────────────────────────────────────────────┐
│                    WORKING MEMORY                                │
│                                                                  │
│  Storage:   SQLite, tier='working'                              │
│  Capacity:  Unbounded (session-scoped)                          │
│  Lifetime:  Persists across restarts but scoped to session_id   │
│  Purpose:   Facts extracted from current conversation           │
│  Injected:  Via hybrid_search() when relevant to query          │
│                                                                  │
│  Created by:                                                     │
│  • extract_facts() — LLM extracts 5 key facts from chat        │
│  • summarize() — LLM creates 1-3 sentence recap                │
│  • User clicks "Extract from session" button                    │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ id=101  "User prefers dark mode"         tier=working    │   │
│  │ id=102  "User is studying family law"    tier=working    │   │
│  │ id=103  "Session about filing deadlines" tier=working    │   │
│  │         session_id="sess_2026-04-22_001"                 │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  Promotion:  Working → Long when importance ≥ 4 or user confirms│
│  Eviction:   Decays faster than long-term (tier_priority=lower) │
└─────────────────────────────────────────────────────────────────┘
```

### Long-Term Memory

```
┌─────────────────────────────────────────────────────────────────┐
│                    LONG-TERM MEMORY                              │
│                                                                  │
│  Storage:   SQLite, tier='long', vector-indexed                 │
│  Capacity:  100,000+ entries (tested to <50ms search)           │
│  Lifetime:  Permanent — subject to decay + GC                   │
│  Purpose:   Knowledge base for RAG injection                    │
│  Injected:  Top 5 via hybrid_search() into [LONG-TERM MEMORY]  │
│                                                                  │
│  Sources:                                                        │
│  • Manual entry (user types in Memory View)                     │
│  • Promoted from working memory                                 │
│  • Document ingestion (PDF/URL → chunked → embedded)            │
│  • LLM extraction ("Extract from session")                      │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ id=1  "Cook County Rule 14.3: 30 days to respond"        │   │
│  │       tier=long  type=fact  importance=5  decay=0.92     │   │
│  │       tags="law,family,deadline,cook-county"             │   │
│  │       embedding=[0.12, -0.34, 0.56, ...] (768-dim)      │   │
│  │       access_count=47  last_accessed=2026-04-22          │   │
│  │                                                           │   │
│  │ id=2  "User's name is Alex"                              │   │
│  │       tier=long  type=fact  importance=5  decay=0.99     │   │
│  │       tags="personal,identity"                           │   │
│  │       access_count=312                                   │   │
│  │                                                           │   │
│  │ id=3  "Alex prefers concise answers"                     │   │
│  │       tier=long  type=preference  importance=4  decay=0.87│   │
│  │       tags="personal,preference,style"                   │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  Decay:  decay_score = 1.0 × 0.95^(hours_since_access / 168)  │
│  GC:     Remove when decay < 0.05 AND importance ≤ 2           │
└─────────────────────────────────────────────────────────────────┘
```

### Tier Lifecycle & Promotion Chain

```
 ┌───────────────────────────────────────────────────────────────────────┐
 │                      MEMORY TIER LIFECYCLE                            │
 │                                                                       │
 │                                                                       │
 │   CONVERSATION                                                        │
 │   ┌─────────┐     evict (FIFO, >20)     ┌───────────┐               │
 │   │  SHORT  │ ──────────────────────────>│  WORKING  │               │
 │   │  TERM   │     extract_facts()        │  MEMORY   │               │
 │   │         │     summarize()            │           │               │
 │   └─────────┘                            └─────┬─────┘               │
 │        │                                       │                      │
 │   lost on close                          promote()                    │
 │                                          (importance ≥ 4              │
 │                                           or user action)             │
 │                                                │                      │
 │                                          ┌─────▼─────┐               │
 │   MANUAL ENTRY ─────────────────────────>│   LONG    │               │
 │   DOCUMENT INGESTION ───────────────────>│   TERM    │               │
 │   LLM EXTRACTION ──────────────────────>│  MEMORY   │               │
 │                                          └─────┬─────┘               │
 │                                                │                      │
 │                                          decay < 0.05                │
 │                                          AND importance ≤ 2          │
 │                                                │                      │
 │                                          ┌─────▼─────┐               │
 │                                          │  GARBAGE   │               │
 │                                          │ COLLECTED  │               │
 │                                          └───────────┘               │
 └───────────────────────────────────────────────────────────────────────┘
```

---

## 3. Memory Categories (Ontology)

### Core Types

The current `memory_type` column supports four values:

| Type | Description | Example |
|------|-------------|---------|
| `fact` | Objective knowledge, rules, data | "Cook County requires 30-day notice for filings" |
| `preference` | Subjective user preferences | "User prefers dark mode and concise answers" |
| `context` | Situational/environmental info | "User is on mobile during commute" |
| `summary` | LLM-generated session recaps | "Session covered family law deadlines and billing" |

### Proposed Category Taxonomy

The four core types are **structural** (how the memory was created). Categories are **semantic** (what the memory is about). Both axes are needed:

```
                          STRUCTURAL TYPE
                  fact    preference    context    summary
              ┌─────────┬────────────┬──────────┬─────────┐
  personal    │ name,   │ dark mode, │ "on      │ "User   │
  info        │ age,    │ language,  │ mobile"  │ intro    │
              │ location│ timezone   │          │ session" │
              ├─────────┼────────────┼──────────┼─────────┤
  friends &   │ "Mom is │ "Dad likes│ "Sister  │ "Talked  │
  relations   │ Sarah"  │ golf"     │ visiting"│ about    │
              │         │           │          │ family"  │
              ├─────────┼────────────┼──────────┼─────────┤
  habits &    │ "Runs   │ "Prefers  │ "Morning │ "Health  │
  routines    │ 5km/day"│ 6am alarm"│ workout" │ recap"   │
              ├─────────┼────────────┼──────────┼─────────┤
  domain      │ "Rule   │ "Cite     │ "Case    │ "Law     │
  knowledge   │ 14.3..."│ Bluebook" │ research"│ session" │
              ├─────────┼────────────┼──────────┼─────────┤
  skills &    │ "Knows  │ "Learning │ "Coding  │ "Skill   │
  projects    │ Python" │ Rust"     │ session" │ progress"│
              ├─────────┼────────────┼──────────┼─────────┤
  emotional   │ "Anxious│ "Likes    │ "Stressed│ "Mood    │
  state       │ about   │ encourage-│ about    │ trend    │
              │ exams"  │ ment"     │ deadline"│ recap"   │
              ├─────────┼────────────┼──────────┼─────────┤
  world       │ "Earth  │ —         │ "Election│ "News    │
  knowledge   │ is 93M  │           │ season"  │ digest"  │
              │ mi from │           │          │          │
              │ sun"    │           │          │          │
              └─────────┴────────────┴──────────┴─────────┘
```

**Proposed `category` values** (stored as a new column or as structured tags):

| Category | Tag Prefix | Description | Decay Behavior |
|----------|-----------|-------------|----------------|
| `personal` | `personal:*` | Identity, demographics, self-description | Very slow decay (core identity) |
| `relations` | `rel:*` | People the user knows, relationships | Slow decay |
| `habits` | `habit:*` | Routines, schedules, repeated behaviors | Medium decay (habits change) |
| `domain` | `domain:*` | Professional/academic knowledge | Configurable per domain |
| `skills` | `skill:*` | Abilities, learning progress, projects | Medium decay |
| `emotional` | `emotional:*` | Mood, feelings, mental state snapshots | Fast decay (emotions are ephemeral) |
| `world` | `world:*` | General knowledge, news, events | Slow decay (facts are stable) |
| `meta` | `meta:*` | Preferences about TerranSoul itself | Very slow decay |

### Category × Tier Matrix

Not all categories belong in all tiers:

```
                    SHORT        WORKING         LONG
  personal          rare         extracted       ✓ permanent
  relations         mentioned    extracted       ✓ permanent
  habits            —            observed        ✓ after confirmation
  domain            referenced   chunked/cited   ✓ ingested docs
  skills            mentioned    session notes   ✓ tracked progress
  emotional         ✓ current    session mood    ✓ only patterns
  world             referenced   —               ✓ verified facts
  meta              —            —               ✓ always
```

**Key insight**: Emotional memories should decay fast in long-term (you don't want "user was stressed on April 3rd" cluttering RAG forever), but personal identity ("user's name is Alex") should essentially never decay.

**Implementation path**: The canonical V13 schema includes an optional `category` column, while structured tag prefixes (`personal:name`, `rel:friend:sarah`, `domain:law:family`) remain the primary portable categorisation layer.

> **As-built (2026-04-24).** Tag-prefix approach implemented:
> - **Chunk 18.4** — `memory::tag_vocabulary` with `CURATED_PREFIXES` (`personal`, `domain`, `project`, `tool`, `code`, `external`, `session`, `quest`), `validate()` / `validate_csv()`, `LEGACY_ALLOW_LIST`.
> - **Chunk 18.2** — `category_decay_multiplier()` per-prefix decay rates (personal 0.5×, session/quest 2×).
> - **Chunk 18.1** — `memory::auto_tag` LLM auto-tagger: opt-in via `AppSettings.auto_tag`; dispatches to Ollama/FreeApi/PaidApi; merges ≤ 4 curated tags with user tags on `add_memory`.
> - **Chunk 18.3** — `MemoryView.vue` tag-prefix filter chip row with per-prefix counts.
> - **Chunk 18.5** (planned) — Obsidian vault export with tag metadata.

---

## 3.5 Cognitive Memory Axes (Episodic / Semantic / Procedural)

> **Status:** classifier landed at `src-tauri/src/memory/cognitive_kind.rs`
> alongside ontology tag prefixes; **no extra schema change**.

### 3.5.1 The question

Cognitive psychology splits long-term memory into three biologically distinct
systems:

| Cognitive kind | Description | Brain region | TerranSoul examples |
|----------------|-------------|--------------|---------------------|
| **Episodic** | Time- and place-anchored personal experiences. Each memory is a "scene" with a when/where. | Hippocampus | "On April 22nd Alex finished the rust refactor", "We met Sarah at the cafe yesterday" |
| **Semantic** | Time-independent general knowledge and stable preferences. The "what" without the "when". | Neocortex | "Rust uses ownership for memory safety", "Alex prefers dark mode", "Mars has two moons" |
| **Procedural** | How-to knowledge, motor skills, repeatable workflows. The "how". | Cerebellum / basal ganglia | "How to ship a release: bump → tag → push", "Morning routine: 6am alarm, run 5km, shower, breakfast" |

These overlap with — but are **orthogonal to** — TerranSoul's existing
`MemoryType` (`fact`/`preference`/`context`/`summary`) and `MemoryTier`
(`short`/`working`/`long`) axes:

```
                STRUCTURAL TYPE × COGNITIVE KIND  (✓ = common, ◇ = possible, — = rare)

                    episodic    semantic    procedural
   fact             ◇  "Q3      ✓  "Mars   ◇  "rustup
                       earnings    has two     installs
                       were $X"    moons"      to ~/.cargo"
   preference       —           ✓  "Dark     ◇  "Always
                                   mode +      run cargo
                                   serif"      fmt before
                                               push"
   context          ✓  "User    ◇  "User    ◇  "Use
                       just         is on a     conventional
                       unblocked    Mac"        commits in
                       a build"                 this repo"
   summary          ✓  "Today   —           ◇  "Recap of
                       we                       Q1 release
                       discussed                process"
                       …"
```

### 3.5.2 Do we **need** a third axis?

**Yes** — but a derived one, not a stored one. We need it for three concrete
RAG-quality reasons:

1. **Decay must be kind-aware.** Episodic memories should decay much faster
   than semantic ones — nobody benefits from "user mentioned the weather on
   April 3rd" haunting RAG forever, but "user's name is Alex" must never
   decay. Procedural memories should decay slowly *unless* they're explicitly
   superseded.
2. **Retrieval must be kind-prioritised by query intent.** A query like
   *"how do I deploy?"* wants procedural first; *"what did we decide
   yesterday?"* wants episodic first; *"what is X?"* wants semantic first. A
   light query-intent classifier + a kind-aware ranker boost is a meaningful
   precision improvement over pure vector similarity.
3. **Conflict resolution must be kind-aware.** Two semantic memories saying
   contradictory things ("Alex prefers dark mode" vs "Alex prefers light
   mode") must be reconciled — the newer wins. Two episodic memories of the
   same event can both be true and should be merged, not deduplicated.

We do **not** need a `cognitive_kind` SQL column today because:

- All three kinds can be derived from `(memory_type, tags, content)` with a
  pure-function classifier (no LLM needed for the common case).
- Tag prefixes (`episodic:*`, `semantic:*`, `procedural:*`) layer cleanly on
  the V4 schema without any migration. Power users can override the heuristic
  by tagging.
- If profiling later shows the classifier is too slow at retrieval time, a V6
  migration to add the column + an index is straightforward.

### 3.5.3 Classifier algorithm

Implemented as a pure function in
[`memory/cognitive_kind.rs`](../src-tauri/src/memory/cognitive_kind.rs):

```
fn classify(memory_type, tags, content) -> CognitiveKind:
    1. If `tags` contains an explicit cognitive tag
       (`episodic` | `semantic` | `procedural`, optionally with
       `:detail` suffix), use it. — power-user override.
    2. Else apply structural-type defaults:
         Summary    → Episodic   (recaps a session)
         Preference → Semantic   (stable user state)
         Fact, Context → fall through to step 3.
    3. Else apply lightweight content heuristics:
         "how to" / "step " / "first," / numbered-list shape
            → Procedural
         "yesterday" / "this morning" / weekday names / "ago"
            → Episodic
         else → Semantic        (safe default)
```

The classifier is exhaustively unit-tested (15 cases covering tag override,
structural-type defaults, content heuristics, and edge cases). It is
**deterministic and offline** — no LLM call. An optional LLM-based reclassifier
can be added later for the long-tail; the heuristic currently resolves the
~85% of memories where the kind is obvious from surface features.

### 3.5.4 How the three axes compose

```
            ┌──────────────────────┐
            │      Memory          │
            │ ──────────────────── │
            │ tier: short/working/long       (lifecycle)
            │ memory_type: fact/pref/...     (structural origin)
            │ cognitive_kind: ep/sem/proc    (cognitive function)  ← derived
            │ category: personal/world/...   (subject taxonomy)    ← tag prefix
            │ tags: free-form + structured                         (filtering)
            │ embedding, importance, decay_score, …
            └──────────────────────┘
```

`tier` answers *"How long do we keep this?"*
`memory_type` answers *"How was this born?"*
`cognitive_kind` answers *"What kind of cognition does this support?"*
`category` answers *"What is this about?"*

### 3.5.5 Decay tuning by cognitive kind

Recommended decay multipliers (multiply the existing `decay_score` step):

| Kind        | Half-life target | Multiplier | Rationale |
|-------------|------------------|------------|-----------|
| Episodic    | 30–90 days       | × 1.5      | Time-anchored; old episodes stop being useful |
| Semantic    | 365+ days        | × 0.5      | Stable knowledge; almost never wrong-by-staleness |
| Procedural  | 180+ days        | × 0.7      | Slow decay; bump back up to 1.0 on each successful execution |

Implementation hook: extend `apply_memory_decay` to read
`classify_cognitive_kind(...)` and apply the multiplier. The classifier is
already exposed from `crate::memory` so no new wiring is required.

### 3.5.6 Retrieval ranking by query intent

A 50-line classifier on the **query** side can detect intent from cue words:

| Query cue                                  | Boost                |
|--------------------------------------------|----------------------|
| `how do I`, `steps to`, `procedure for`    | + procedural × 1.4   |
| `when`, `what did we`, `last time`, `ago`  | + episodic × 1.4     |
| `what is`, `define`, `prefers?`            | + semantic × 1.2     |

This is additive on top of the existing 6-signal hybrid score — see §4 — so
we never trade off vector similarity for kind matching, we just break ties.

> ✅ **Shipped (Chunk 16.6b, 2026-04-30).** Implemented as
> `crate::memory::query_intent::classify_query` returning an
> `IntentClassification { intent, confidence, kind_boosts }`. Five intent
> labels (`Procedural`, `Episodic`, `Factual`, `Semantic`, `Unknown`),
> heuristic-based, no LLM call. Caller can multiply each candidate doc's
> RRF score by `boosts.for_kind(doc.cognitive_kind)` before final ranking.
> Wiring into the live RRF pipeline tracked separately (see Phase 16.6
> GraphRAG / 16.4b Self-RAG).
>
> ✅ **Wiring shipped (Chunk 16.6c, 2026-05-01).** Exposed as
> `MemoryStore::hybrid_search_rrf_with_intent(query, embedding?, limit)`.
> Runs the standard 3-signal RRF fusion, then multiplies each fused
> score by the per-kind boost from `IntentClassification.kind_boosts`
> using each doc's `cognitive_kind::classify` result, and re-sorts.
> When the classifier returns `Unknown`, the method is identical to
> plain `hybrid_search_rrf` (no perturbation). +5 tests.

### 3.5.7 Migration story (when we do want a column)

If we later add a `cognitive_kind` column in V6, the migration is:

1. `ALTER TABLE memories ADD COLUMN cognitive_kind TEXT NOT NULL DEFAULT 'semantic';`
2. `CREATE INDEX idx_memories_cognitive_kind ON memories(cognitive_kind);`
3. Backfill: `UPDATE memories SET cognitive_kind = classify(memory_type, tags, content)`
   via a one-shot Rust pass (the classifier is already a pure function).
4. Update `add_memory` / `update_memory` to compute and persist the kind.

Until then, the derived classifier is the source of truth.

### 3.5.8 What does **not** change

- No new Tauri command, no new schema, no new index.
- `MemoryEntry` payload is unchanged on the wire — the kind is computed
  client-side or by the Rust ranker as needed.
- Existing tests remain green; the classifier ships alongside `MemoryStore`,
  not inside it.

---

## 4. Hybrid RAG Pipeline

### 6-Signal Scoring Formula

Every query triggers a hybrid search that combines six signals into a single relevance score:

```
final_score =
    0.40 × vector_similarity    // Semantic meaning (cosine distance)
  + 0.20 × keyword_match        // Exact word overlap (BM25-like)
  + 0.15 × recency_bias         // How recently accessed
  + 0.10 × importance_score     // User-assigned priority (1–5)
  + 0.10 × decay_score          // Freshness multiplier (0.0–1.0)
  + 0.05 × tier_priority        // working(1.0) > long(0.7) > short(0.3)
```

**Signal breakdown:**

```
┌─────────────────────────────────────────────────────────────────────┐
│                    HYBRID SEARCH — 6 SIGNALS                        │
│                                                                     │
│  ┌─────────────────────────┐                                        │
│  │ 1. VECTOR SIMILARITY    │  Weight: 40%                           │
│  │    cosine(query_emb,    │  Range: 0.0 – 1.0                     │
│  │           memory_emb)   │  Source: nomic-embed-text (768-dim)    │
│  │                         │  Fallback: skip if no embeddings       │
│  └─────────────────────────┘                                        │
│                                                                     │
│  ┌─────────────────────────┐                                        │
│  │ 2. KEYWORD MATCH        │  Weight: 20%                           │
│  │    words_in_common /    │  Range: 0.0 – 1.0                     │
│  │    total_query_words    │  Case-insensitive, whitespace-split    │
│  │                         │  Searches: content + tags              │
│  └─────────────────────────┘                                        │
│                                                                     │
│  ┌─────────────────────────┐                                        │
│  │ 3. RECENCY BIAS         │  Weight: 15%                           │
│  │    e^(-hours / 24)      │  Half-life: 24 hours                  │
│  │                         │  Based on last_accessed timestamp      │
│  │                         │  Decays exponentially from 1.0 → 0.0  │
│  └─────────────────────────┘                                        │
│                                                                     │
│  ┌─────────────────────────┐                                        │
│  │ 4. IMPORTANCE           │  Weight: 10%                           │
│  │    importance / 5.0     │  Range: 0.2 – 1.0                     │
│  │                         │  User-assigned: 1=low, 5=critical     │
│  └─────────────────────────┘                                        │
│                                                                     │
│  ┌─────────────────────────┐                                        │
│  │ 5. DECAY SCORE          │  Weight: 10%                           │
│  │    stored decay_score   │  Range: 0.01 – 1.0                    │
│  │    (exponential forget) │  Updated by apply_memory_decay()       │
│  └─────────────────────────┘                                        │
│                                                                     │
│  ┌─────────────────────────┐                                        │
│  │ 6. TIER PRIORITY        │  Weight: 5%                            │
│  │    working=1.0          │  Working memory is most relevant       │
│  │    long=0.7             │  Long-term is base knowledge           │
│  │    short=0.3            │  Short-term rarely searched            │
│  └─────────────────────────┘                                        │
│                                                                     │
│  PERFORMANCE: O(n) linear scan, pure arithmetic                     │
│  • 100 entries:    <1ms                                             │
│  • 10,000 entries:  2ms                                             │
│  • 100,000 entries: 5ms                                             │
│  • 1,000,000 entries: ~50ms                                         │
└─────────────────────────────────────────────────────────────────────┘
```

### RAG Injection Flow

> **Note (2026-04-25):** The diagram below shows the foundational 4-step flow
> that is still the backbone of every retrieval. Since this was first drawn,
> the pipeline has been extended with:
> - **ANN index** (usearch HNSW, Chunk 16.10) — O(log n) vector search replaces brute-force scan
> - **Cloud embedding API** (Chunk 16.9) — vector RAG works in free/paid modes too
> - **RRF fusion** (Chunk 1.8) — multiple retrieval signals fused via Reciprocal Rank Fusion (k=60)
> - **HyDE** (Chunk 1.9) — LLM-hypothetical-document embedding for cold/abstract queries
> - **Cross-encoder rerank** (Chunk 1.10) — LLM-as-judge scores (query, doc) pairs 0–10
> - **Relevance threshold** (Chunk 16.1) — only entries above a configurable score are injected
> - **Semantic chunking** (Chunk 16.11) — `text-splitter` crate replaces naive word-count splitter
> - **Contextual Retrieval** (Chunk 16.2) — Anthropic 2024 approach prepends doc context to chunks
> - **Contradiction resolution** (Chunk 17.2) — LLM-powered conflict detection + soft-closure
> - **Temporal reasoning** (Chunk 17.3) — natural-language time-range queries
> - **Memory versioning** (Chunk 16.12) — non-destructive V8 edit history
> - **Plugin memory hooks** — sandboxed `pre_store` / `post_store` processors can normalize or index memory payloads without schema changes
>
> See the § 19.2 research survey rows and `rules/completion-log.md` for as-built details.

```
User types: "What are the filing deadlines?"
                │
                ▼
┌──────────────────────────────────────────────────────────────────────┐
│ Step 1: EMBED QUERY                                                  │
│                                                                      │
│ POST http://127.0.0.1:11434/api/embed                               │
│ { "model": "nomic-embed-text", "input": "filing deadlines" }       │
│ → query_embedding = [0.12, -0.34, ...] (768 floats, ~50ms)         │
│                                                                      │
│ Fallback (no Ollama): skip vector signal, keyword+temporal only     │
└──────────────────────────┬───────────────────────────────────────────┘
                           ▼
┌──────────────────────────────────────────────────────────────────────┐
│ Step 2: HYBRID SEARCH                                                │
│                                                                      │
│ hybrid_search(query="filing deadlines",                             │
│               embedding=Some([0.12, -0.34, ...]),                   │
│               limit=5)                                              │
│                                                                      │
│ Scans ALL memories in SQLite, scores each with 6 signals,          │
│ returns top 5 by final_score.                                       │
│                                                                      │
│ Results:                                                             │
│   #1  score=0.89  "Cook County Rule 14.3: 30-day notice"           │
│   #2  score=0.74  "Emergency motions: same-day filing allowed"     │
│   #3  score=0.61  "Standard civil filing: 21-day response"         │
│   #4  score=0.55  "Court hours: 8:30am–4:30pm for filings"        │
│   #5  score=0.41  "E-filing portal: odysseyfileandserve.com"       │
└──────────────────────────┬───────────────────────────────────────────┘
                           ▼
┌──────────────────────────────────────────────────────────────────────┐
│ Step 3: FORMAT MEMORY BLOCK                                          │
│                                                                      │
│ [LONG-TERM MEMORY]                                                  │
│ - [long] Cook County Rule 14.3: 30-day notice required              │
│ - [long] Emergency motions: same-day filing allowed                 │
│ - [long] Standard civil filing: 21-day response                    │
│ - [long] Court hours: 8:30am–4:30pm for filings                   │
│ - [long] E-filing portal: odysseyfileandserve.com                  │
│ [/LONG-TERM MEMORY]                                                 │
└──────────────────────────┬───────────────────────────────────────────┘
                           ▼
┌──────────────────────────────────────────────────────────────────────┐
│ Step 4: INJECT INTO SYSTEM PROMPT                                    │
│                                                                      │
│ system: "You are a helpful AI companion. {personality}               │
│                                                                      │
│          [LONG-TERM MEMORY]                                          │
│          - [long] Cook County Rule 14.3: 30-day notice...           │
│          ... (top 5 memories)                                       │
│          [/LONG-TERM MEMORY]                                         │
│                                                                      │
│          Use these memories to inform your response."                │
│                                                                      │
│ user: "What are the filing deadlines?"                               │
│                                                                      │
│ → LLM generates response grounded in retrieved memories             │
└──────────────────────────────────────────────────────────────────────┘
```

### Tool Plugins And Brain Routing

Tool integrations that do not replace the active brain should live in the
plugin host, not the Agent Marketplace or local-model selector. The
`openclaw-bridge` built-in plugin is the reference case: `/openclaw read`,
`/openclaw fetch`, and `/openclaw chat` are command/slash-command contributions
registered by `PluginHost::with_builtin_plugins()`. The active brain still owns
conversation planning and any RAG-grounded answer, while OpenClaw owns the
external tool runtime behind explicit plugin capability consent.

This split keeps the brain surface coherent:

- **Brain mode** chooses the LLM provider and embedding path.
- **Memory/RAG** decides which long-term context enters the prompt.
- **Plugin commands** perform bounded side effects or external tool calls.
- **CapabilityStore** gates sensitive plugin operations by plugin id.

The local Ollama system prompt may recommend OpenClaw as an extension, but it
must describe it as the `openclaw-bridge` plugin rather than a model tag or
installable agent package.

### Memory Plugin Hooks

TerranSoul plugins can contribute sandboxed memory processors through
`contributes.memory_hooks` in `terransoul-plugin.json`. The host keeps these
hooks lazy by default: a plugin with `activation_events: [{ "type":
"on_memory_tag", "tag": "project" }]` activates only when an incoming or
persisted memory has either the exact tag (`project`) or a prefix-qualified tag
(`project:terran-soul`).

Supported stages:

| Stage | Timing | Can mutate stored memory? | Intended use |
|---|---|---:|---|
| `pre_store` | Before `add_memory` parses `MemoryType`, embeds content, and writes to SQLite | Yes | Normalize tags, enrich content, adjust importance, classify type |
| `post_store` | After SQLite persistence and built-in auto-tagging | No | External indexing, notifications, lightweight audit side effects |

The hook payload is JSON:

```json
{
  "stage": "pre_store",
  "content": "User prefers local-first tools",
  "tags": "preferences,local",
  "importance": 3,
  "memory_type": "preference",
  "entry_id": null
}
```

WASM processors export `memory_hook(input_ptr: i32, input_len: i32) -> i64` and
an exported `memory`. TerranSoul writes the JSON payload into guest memory,
runs the hook inside `WasmRunner`, and reads a JSON patch from the returned
packed pointer/length (`ptr << 32 | len`). Missing fields are left unchanged.
Invalid JSON or sandbox failures are logged and the original payload continues
through the memory pipeline, preserving the local-first fallback contract.

```json
{
  "tags": "personal:preference, tool:local-first",
  "importance": 4
}
```

### Embedding & Vector Search

```
┌──────────────────────────────────────────────────────────────────┐
│                    EMBEDDING ARCHITECTURE                         │
│                                                                   │
│  Model:     nomic-embed-text (768-dimensional)                   │
│  Providers:                                                       │
│    Local:   Ollama (localhost:11434/api/embed)                   │
│    Cloud:   OpenAI-compatible /v1/embeddings (paid/free modes)   │
│             Dispatched by cloud_embeddings::embed_for_mode()     │
│  Fallback chain (Chunk 16.9b — local Ollama path):               │
│    1. nomic-embed-text  (preferred, 768d)                        │
│    2. mxbai-embed-large (1024d, strong general-purpose)          │
│    3. snowflake-arctic-embed (1024d / 768d)                      │
│    4. bge-m3 (1024d, multilingual)                               │
│    5. all-minilm (384d, tiny last-resort)                        │
│    6. active chat model (almost always rejects → keyword-only)   │
│  Storage:   BLOB column in SQLite (768 × 4 bytes = 3 KB each)   │
│  ANN:       usearch HNSW index (vectors.usearch file)            │
│             O(log n) search — scales to 1M+ entries              │
│                                                                   │
│  Memory budget:                                                   │
│    1,000 memories   ×  3 KB  =    3 MB                           │
│   10,000 memories   ×  3 KB  =   30 MB                           │
│  100,000 memories   ×  3 KB  =  300 MB                           │
│  1,000,000 memories ×  3 KB  = 3,000 MB (ANN index: ~100 MB)    │
│                                                                   │
│  Cosine Similarity:                                               │
│  sim(A, B) = (A · B) / (||A|| × ||B||)                          │
│  Range: -1.0 (opposite) to 1.0 (identical)                      │
│  Threshold: > 0.97 = duplicate detection                         │
│                                                                   │
│  Deduplication:                                                   │
│  Before insert → embed new text → cosine vs all existing         │
│  If max_similarity > 0.97 → skip insert, return existing id     │
│                                                                   │
│  Resilience (durable workflow contract):                         │
│  • The embed-model resolver walks the fallback chain above on    │
│    cache miss, picking the first model present in `/api/tags`.   │
│    Models the unsupported-cache has marked are skipped.          │
│  • If every dedicated embedder is missing AND the active chat    │
│    model returns 501/400 from /api/embed (Llama, Phi, Gemma, …), │
│    the model is added to a process-lifetime "unsupported" cache  │
│    and no further embed calls are made for it. Vector RAG        │
│    silently degrades to keyword + LLM-ranking. No log spam, no   │
│    chat pipeline stalls.                                         │
│  • The `/api/tags` probe that picks the embedding model is       │
│    cached for 60 s.                                              │
│  • `reset_embed_cache` Tauri command flushes both caches; called │
│    automatically on `set_brain_mode` so a brain switch can       │
│    re-discover a working embedding backend.                      │
└──────────────────────────────────────────────────────────────────┘
```

---

## 5. Decay & Garbage Collection

### Exponential Forgetting Curve

Memories naturally fade over time unless actively accessed:

```
decay_score(t) = 1.0 × 0.95 ^ (hours_since_last_access / 168)

Where:
  • 168 hours = 1 week
  • Half-life ≈ 2 weeks of non-access
  • Minimum floor: 0.01 (never fully zero)
```

```
Decay Score
  1.0 ┤ ●
      │  ●
  0.9 ┤   ●
      │     ●
  0.8 ┤       ●
      │         ●
  0.7 ┤           ●
      │              ●
  0.6 ┤                ●
      │                   ●
  0.5 ┤                      ●                    ← ~2 weeks
      │                         ●
  0.4 ┤                            ●
      │                               ●
  0.3 ┤                                  ●
      │                                     ●
  0.2 ┤                                       ●
      │                                         ●
  0.1 ┤                                          ●●
      │                                             ●●●
  0.05┤─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ●●●●●── GC threshold
  0.01┤                                                     ●●●●●●●●●
      └──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬
        0d  2d  4d  6d  1w      2w      3w      4w      5w
                    Days since last access

  • Accessing a memory resets its decay to 1.0
  • Important memories (≥3) survive GC even at low decay
  • GC removes: decay < 0.05 AND importance ≤ 2
```

### Category-Aware Decay (Proposed)

Different categories should decay at different rates:

```
┌──────────────┬──────────────┬──────────────┬───────────────────────┐
│ Category     │ Base Rate    │ Half-Life    │ Rationale             │
├──────────────┼──────────────┼──────────────┼───────────────────────┤
│ personal     │ 0.99         │ ~6 months    │ Identity is stable    │
│ relations    │ 0.98         │ ~3 months    │ People change slowly  │
│ habits       │ 0.96         │ ~1 month     │ Routines evolve       │
│ domain       │ 0.97         │ ~2 months    │ Knowledge is durable  │
│ skills       │ 0.96         │ ~1 month     │ Skills need practice  │
│ emotional    │ 0.90         │ ~1 week      │ Moods are transient   │
│ world        │ 0.97         │ ~2 months    │ Facts are stable      │
│ meta         │ 0.99         │ ~6 months    │ App prefs are sticky  │
└──────────────┴──────────────┴──────────────┴───────────────────────┘

Future formula:
  decay_score(t) = 1.0 × category_rate ^ (hours / 168)
```

---

## 6. Knowledge Graph Vision

### Current: Tag-Based Graph

The in-app MemoryGraph (Cytoscape.js) connects memories that share tags:

```
┌─────────────────────────────────────────────────────────────────────┐
│                  CURRENT GRAPH MODEL (TAG EDGES)                    │
│                                                                     │
│   "Rule 14.3"                     "User prefers email"              │
│   tags: law, family, deadline     tags: preference, communication   │
│        ●───────────────────────────────●                            │
│       /│\         shared tag:          │                            │
│      / │ \        (none — no edge)     │                            │
│     /  │  \                            │                            │
│    /   │   \                           │                            │
│   ●    │    ●                          ●                            │
│  "Emergency    "Court hours"     "Email template"                   │
│   motions"     tags: law,        tags: communication,               │
│   tags: law,    schedule          template                          │
│    family,                                                          │
│    emergency                                                        │
│                                                                     │
│  Nodes: Each memory entry                                           │
│  Edges: Shared tag between two memories                             │
│  Size:  Proportional to importance (20 + importance × 8 px)        │
│  Color: By memory_type (fact=blue, preference=green, etc.)         │
│  Layout: CoSE (force-directed)                                      │
└─────────────────────────────────────────────────────────────────────┘
```

**Limitations of tag-based edges:**
- No semantic relationships ("Rule 14.3 is an exception to Rule 14.1")
- No directional links ("Sarah is Alex's mother" ≠ "Alex is Sarah's child")
- Tags must be manually assigned or extracted — no automatic linking
- Clusters form around common tags, not around meaning

### Implemented (V5): Entity-Relationship Graph

A proper knowledge graph with typed, directional edges — shipped in the V5
schema (April 2026).

```
┌─────────────────────────────────────────────────────────────────────┐
│                    ENTITY-RELATIONSHIP GRAPH (V5)                    │
│                                                                     │
│              ┌──────────┐                                            │
│              │  Alex    │                                            │
│              │ (person) │                                            │
│              └────┬─────┘                                            │
│           ┌───────┼──────────────┐                                   │
│     mother_of  studies      prefers                                  │
│           │       │              │                                    │
│     ┌─────▼──┐ ┌──▼───────┐ ┌───▼────────┐                         │
│     │ Sarah  │ │ Family   │ │ Dark mode  │                          │
│     │(person)│ │ Law      │ │(preference)│                          │
│     └────────┘ │(domain)  │ └────────────┘                          │
│                └────┬─────┘                                          │
│              ┌──────┼──────────┐                                     │
│          contains  governs   cites                                   │
│              │       │         │                                      │
│     ┌────────▼──┐ ┌──▼─────┐ ┌▼──────────┐                         │
│     │ Rule 14.3 │ │ Filing │ │ Illinois  │                          │
│     │ (rule)    │ │Deadline│ │ Statute   │                          │
│     │           │ │ (fact) │ │ 750-5/602 │                          │
│     └───────────┘ └────────┘ └───────────┘                          │
│                                                                     │
│  SCHEMA:                                                             │
│  • Nodes: existing `memories` rows                                   │
│  • Edges: typed relationships with direction (`memory_edges`)        │
│  • Edge types: contains, cites, governs, related_to, mother_of,    │
│                studies, prefers, contradicts, supersedes, …          │
│  • Provenance: `source` ∈ {user, llm, auto}                         │
│  • Idempotency: UNIQUE(src_id, dst_id, rel_type)                    │
│  • Cascade: ON DELETE CASCADE keeps the graph consistent             │
│  • Traversal: cycle-safe BFS, optional rel_type filter               │
└─────────────────────────────────────────────────────────────────────┘
```

**Shipped `memory_edges` table (canonical schema):**

```sql
CREATE TABLE memory_edges (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    src_id     INTEGER NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    dst_id     INTEGER NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    rel_type   TEXT    NOT NULL,         -- 'contains', 'cites', 'related_to', …
    confidence REAL    NOT NULL DEFAULT 1.0,  -- LLM extraction confidence
    source     TEXT    NOT NULL DEFAULT 'user',  -- user | llm | auto
    created_at INTEGER NOT NULL,
    UNIQUE(src_id, dst_id, rel_type)
);

CREATE INDEX idx_edges_src  ON memory_edges(src_id);
CREATE INDEX idx_edges_dst  ON memory_edges(dst_id);
CREATE INDEX idx_edges_type ON memory_edges(rel_type);
-- PRAGMA foreign_keys=ON enforced at connection open.
```

**Code surface (Rust):**

| Symbol | Location |
|---|---|
| `MemoryEdge` / `NewMemoryEdge` / `EdgeSource` / `EdgeDirection` | `src-tauri/src/memory/edges.rs` |
| `MemoryStore::add_edge` / `add_edges_batch` / `list_edges` | `edges.rs` |
| `MemoryStore::get_edges_for(id, EdgeDirection)` | `edges.rs` |
| `MemoryStore::delete_edge` / `delete_edges_for_memory` | `edges.rs` |
| `MemoryStore::edge_stats() -> EdgeStats` | `edges.rs` |
| `MemoryStore::traverse_from(id, max_hops, rel_filter)` | `edges.rs` |
| `MemoryStore::hybrid_search_with_graph(query, emb, limit, hops)` | `edges.rs` |
| `OllamaAgent::propose_edges(memories_block) -> String` | `brain/ollama_agent.rs` |
| `parse_llm_edges(text, known_ids)` | `edges.rs` |

### Graph Traversal for Multi-Hop RAG

```
Query: "What rules apply to Alex's area of study?"

Step 1 — Hybrid search (vector + keyword + recency + importance):
  → "Alex studies Family Law"  (direct hit, score 1.0)

Step 2 — Graph traversal (1 hop from each direct hit):
  → "Rule 14.3: 30-day notice"      (Family Law --contains--> Rule 14.3)
  → "Filing deadline: 30 days"       (Family Law --governs--> Filing Deadline)
  → "Illinois Statute 750-5/602"     (Family Law --cites--> Statute)

Step 3 — Merge & re-rank:
  → Each graph hit scored as `seed_score / (hop + 1)`
  → De-duplicate by memory id, keeping the highest score
  → Sort by composite score, truncate to `limit`

This finds memories that are TOPICALLY connected even if they don't share
exact keywords or high vector similarity with the query.
```

Implemented as `MemoryStore::hybrid_search_with_graph` and exposed via the
`multi_hop_search_memories` Tauri command. `hops` is hard-capped at 3 by the
command layer to prevent runaway expansion.

---

## 7. Visualization Layers

The memory graph is hard to visualize in a single UI because it spans three tiers, multiple categories, thousands of entries, and complex relationships. The solution: **three complementary visualization layers**.

### Layer 1: In-App (Cytoscape.js)

The primary visualization, rendered inside TerranSoul's Memory tab:

```
┌─────────────────────────────────────────────────────────────────────┐
│                  IN-APP MEMORY GRAPH                                 │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  Filters: [Tier ▼] [Type ▼] [Category ▼] [Search...]      │   │
│  ├─────────────────────────────────────────────────────────────┤   │
│  │                                                             │   │
│  │        ● Rule 14.3                                         │   │
│  │       / \          ● Alex's name                           │   │
│  │      /   \        /                                        │   │
│  │     ●     ●      ●                                         │   │
│  │   Emergency  Court   Prefers                               │   │
│  │   motions    hours   email     ● Dark mode                 │   │
│  │                      \        /                             │   │
│  │                       ●──────●                              │   │
│  │                    Communication                            │   │
│  │                    preferences                              │   │
│  │                                                             │   │
│  ├─────────────────────────────────────────────────────────────┤   │
│  │  Legend:  ● fact  ● preference  ● context  ● summary       │   │
│  │  Size = importance │ Opacity = decay_score                 │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  Interactions:                                                      │
│  • Click node → detail panel (content, tags, decay, access_count)  │
│  • Hover → highlight connected nodes                                │
│  • Pinch/scroll → zoom                                              │
│  • Drag → pan                                                       │
│  • Filter toolbar → show/hide by tier, type, category              │
│                                                                     │
│  Pros: Integrated, always available, real-time                      │
│  Cons: Limited screen real estate, no advanced layout controls      │
└─────────────────────────────────────────────────────────────────────┘
```

### Layer 2: Obsidian Vault Export

For power users who want to explore their memory graph in a full-featured knowledge management tool:

```
┌─────────────────────────────────────────────────────────────────────┐
│                 OBSIDIAN VAULT EXPORT                                │
│                                                                     │
│  TerranSoul exports memories as an Obsidian-compatible vault:      │
│                                                                     │
│  📁 TerranSoul-Vault/                                               │
│  ├── 📁 personal/                                                   │
│  │   ├── Alex.md                                                    │
│  │   ├── preferences.md                                             │
│  │   └── identity.md                                                │
│  ├── 📁 relations/                                                  │
│  │   ├── Sarah (mother).md                                          │
│  │   ├── David (study partner).md                                   │
│  │   └── Professor Kim.md                                           │
│  ├── 📁 habits/                                                     │
│  │   ├── morning-routine.md                                         │
│  │   └── study-schedule.md                                          │
│  ├── 📁 domain/                                                     │
│  │   ├── 📁 family-law/                                             │
│  │   │   ├── Rule 14.3 — Filing Deadline.md                        │
│  │   │   ├── Emergency Motions.md                                   │
│  │   │   └── Illinois Statute 750-5-602.md                          │
│  │   └── 📁 civil-procedure/                                       │
│  │       └── Standard Filing — 21 Day Response.md                   │
│  ├── 📁 emotional/                                                  │
│  │   └── 2026-04-22 — exam stress.md                                │
│  ├── 📁 meta/                                                       │
│  │   ├── brain-mode.md                                              │
│  │   └── voice-settings.md                                          │
│  └── 📁 _session-summaries/                                        │
│      ├── 2026-04-20 — family law study.md                           │
│      ├── 2026-04-21 — filing deadlines.md                           │
│      └── 2026-04-22 — exam prep.md                                  │
│                                                                     │
│  Each .md file contains:                                            │
│  ┌──────────────────────────────────────────────────────────┐      │
│  │ ---                                                       │      │
│  │ id: 42                                                    │      │
│  │ tier: long                                                │      │
│  │ type: fact                                                │      │
│  │ category: domain                                          │      │
│  │ importance: 5                                             │      │
│  │ decay: 0.92                                               │      │
│  │ access_count: 47                                          │      │
│  │ created: 2026-03-15                                       │      │
│  │ last_accessed: 2026-04-22                                 │      │
│  │ tags: [law, family, deadline, cook-county]                │      │
│  │ ---                                                       │      │
│  │                                                           │      │
│  │ # Cook County Rule 14.3                                   │      │
│  │                                                           │      │
│  │ 30 days to respond to a family law motion in Cook County. │      │
│  │                                                           │      │
│  │ ## Related                                                │      │
│  │ - [[Emergency Motions]] — exception for same-day filing  │      │
│  │ - [[Illinois Statute 750-5-602]] — governing statute     │      │
│  │ - [[Standard Filing — 21 Day Response]] — civil default  │      │
│  │                                                           │      │
│  │ ## Source                                                 │      │
│  │ Ingested from: court-rules-2026.pdf (page 14)            │      │
│  └──────────────────────────────────────────────────────────┘      │
│                                                                     │
│  Obsidian features this enables:                                    │
│  • Graph View — full knowledge graph with category coloring        │
│  • Backlinks — see which memories reference each other             │
│  • Dataview — query memories by metadata (importance ≥ 4)          │
│  • Canvas — drag memories into spatial layouts                      │
│  • Daily Notes — session summaries linked by date                  │
│  • Search — full-text across all memories                          │
│  • Community plugins — timeline, kanban, excalidraw                │
│                                                                     │
│  Sync strategy:                                                     │
│  • Export: TerranSoul → Obsidian (one-way, on demand or scheduled) │
│  • Import: Obsidian → TerranSoul (future — parse [[wikilinks]])    │
│  • Bidirectional sync is a non-goal (too complex, conflict-prone)  │
│                                                                     │
│  Implementation:                                                     │
│  • Tauri command: export_obsidian_vault(path: String)              │
│  • Iterates all memories, groups by category, writes .md files     │
│  • Generates [[wikilinks]] from shared tags + memory_edges         │
│  • YAML frontmatter from memory metadata                           │
│  • Runs in background (async), shows progress bar                  │
└─────────────────────────────────────────────────────────────────────┘
```

**Why Obsidian?**
- Free, local-first, Markdown-based — aligns with TerranSoul's privacy philosophy
- Graph View is the best knowledge graph visualizer for personal data
- Massive plugin ecosystem (Dataview, Timeline, etc.) we don't need to build
- Users already familiar with it (50M+ downloads)
- No vendor lock-in — it's just Markdown files in folders

### Layer 3: Debug SQL Console

For developers and advanced users:

```
┌─────────────────────────────────────────────────────────────────────┐
│               DEBUG SQL CONSOLE (Ctrl+Shift+D)                      │
│                                                                     │
│  Direct SQLite queries against memory.db for debugging:            │
│                                                                     │
│  > SELECT tier, memory_type, COUNT(*), AVG(importance),            │
│    AVG(decay_score) FROM memories GROUP BY tier, memory_type;       │
│                                                                     │
│  tier     │ type       │ count │ avg_importance │ avg_decay         │
│  ─────────┼────────────┼───────┼────────────────┼──────────         │
│  long     │ fact       │  1247 │ 3.8            │ 0.72              │
│  long     │ preference │   89  │ 4.1            │ 0.85              │
│  long     │ summary    │   203 │ 3.0            │ 0.55              │
│  working  │ context    │   34  │ 2.5            │ 0.91              │
│  working  │ fact       │   12  │ 3.2            │ 0.95              │
│                                                                     │
│  Accessible via:                                                    │
│  • Tauri command: get_schema_info()                                │
│  • External: sqlite3 memory.db (direct access)                     │
│  • Dev overlay: Ctrl+D shows memory stats in-app                   │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 8. SQLite Schema

### Current Canonical Schema (V13)

Chunk 19.1 collapsed the pre-release migration runner into a single canonical
initializer at `src-tauri/src/memory/schema.rs`. Fresh SQLite databases are
created directly at V13, and `schema_version` records one canonical row rather
than a historical migration ledger.

```sql
CREATE TABLE schema_version (
  version     INTEGER PRIMARY KEY,
  applied_at  INTEGER NOT NULL,
  description TEXT    NOT NULL DEFAULT ''
);

CREATE TABLE memories (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    content       TEXT    NOT NULL,
    tags          TEXT    NOT NULL DEFAULT '',
    importance    INTEGER NOT NULL DEFAULT 3,    -- 1=low, 5=critical
    memory_type   TEXT    NOT NULL DEFAULT 'fact', -- fact|preference|context|summary
    created_at    INTEGER NOT NULL,              -- Unix timestamp (ms)
    last_accessed INTEGER,                       -- Last RAG retrieval
    access_count  INTEGER NOT NULL DEFAULT 0,    -- Times retrieved
    embedding     BLOB,                          -- 768-dim f32 (3 KB)
    source_url    TEXT,                          -- Origin URL for ingested docs
    source_hash   TEXT,                          -- SHA-256 for dedup/staleness
    expires_at    INTEGER,                       -- TTL for auto-expiry
    tier          TEXT    NOT NULL DEFAULT 'long',  -- short|working|long
    decay_score   REAL    NOT NULL DEFAULT 1.0,  -- 0.01–1.0 freshness
    session_id    TEXT,                          -- Links working memories to session
    parent_id     INTEGER,                       -- Summary → source memory link
    token_count   INTEGER,                       -- Content size in tokens
    valid_to      INTEGER,                       -- Soft-close timestamp
    obsidian_path TEXT,                          -- Vault-relative .md path
    last_exported INTEGER,                       -- Unix-ms export timestamp
    category      TEXT,                          -- Optional taxonomy category
    updated_at    INTEGER,                       -- CRDT LWW timestamp
    origin_device TEXT                           -- CRDT tiebreaker device id
);

The feature-gated distributed backends (`postgres.rs`, `mssql.rs`,
`cassandra.rs`) mirror the same lifecycle metadata for cross-device and vault
workflows: `valid_to` for non-destructive supersession, `obsidian_path` /
`last_exported` for Obsidian export state, and `updated_at` / `origin_device`
for LWW sync deltas. SQLite remains canonical, but `StorageBackend` implementors
must keep those `MemoryEntry` fields populated or explicitly `NULL` so
all-feature Rust validation covers the same shape.

CREATE INDEX idx_memories_importance ON memories(importance DESC);
CREATE INDEX idx_memories_created    ON memories(created_at DESC);
CREATE INDEX idx_memories_tier       ON memories(tier);
CREATE INDEX idx_memories_session    ON memories(session_id);
CREATE INDEX idx_memories_decay      ON memories(decay_score);
  CREATE INDEX idx_memories_source_hash ON memories(source_hash);
CREATE INDEX idx_memories_category ON memories(category);
  CREATE INDEX idx_memories_updated_at ON memories(updated_at);

CREATE TABLE memory_edges (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    src_id      INTEGER NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    dst_id      INTEGER NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    rel_type    TEXT    NOT NULL,
    confidence  REAL    NOT NULL DEFAULT 1.0,
    source      TEXT    NOT NULL DEFAULT 'user',  -- user | llm | auto
    created_at  INTEGER NOT NULL,
    valid_from  INTEGER,
    valid_to    INTEGER,
    edge_source TEXT,                             -- e.g. gitnexus:<scope>
    UNIQUE(src_id, dst_id, rel_type)
);

CREATE INDEX idx_edges_src  ON memory_edges(src_id);
CREATE INDEX idx_edges_dst  ON memory_edges(dst_id);
CREATE INDEX idx_edges_type ON memory_edges(rel_type);
  CREATE INDEX idx_edges_valid_to ON memory_edges(valid_to);
  CREATE INDEX idx_edges_edge_source ON memory_edges(edge_source);

  CREATE TABLE memory_versions (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    memory_id   INTEGER NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    version_num INTEGER NOT NULL,
    content     TEXT    NOT NULL,
    tags        TEXT    NOT NULL DEFAULT '',
    importance  INTEGER NOT NULL DEFAULT 3,
    memory_type TEXT    NOT NULL DEFAULT 'fact',
    created_at  INTEGER NOT NULL,
    UNIQUE(memory_id, version_num)
  );
  CREATE INDEX idx_versions_memory ON memory_versions(memory_id);

  CREATE TABLE memory_conflicts (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    entry_a_id  INTEGER NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    entry_b_id  INTEGER NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    status      TEXT    NOT NULL DEFAULT 'open',
    winner_id   INTEGER,
    created_at  INTEGER NOT NULL,
    resolved_at INTEGER,
    reason      TEXT    NOT NULL DEFAULT ''
  );
  CREATE INDEX idx_conflicts_status ON memory_conflicts(status);

  CREATE TABLE paired_devices (
    device_id        TEXT PRIMARY KEY,
    display_name     TEXT NOT NULL,
    cert_fingerprint TEXT NOT NULL,
    capabilities     TEXT NOT NULL DEFAULT '[]',
    paired_at        INTEGER NOT NULL,
    last_seen_at     INTEGER
  );

  CREATE TABLE sync_log (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    peer_device TEXT    NOT NULL,
    direction   TEXT    NOT NULL,
    entry_count INTEGER NOT NULL,
    timestamp   INTEGER NOT NULL
  );
  CREATE INDEX idx_sync_log_peer ON sync_log(peer_device);
-- PRAGMA foreign_keys=ON enforced at connection open.
```

  `edge_source` remains distinct from the `source` column: `source` records who
  asserted an edge (`user` / `llm` / `auto`), while `edge_source` records an
  external mirror scope such as `gitnexus:repo:owner/name@sha`. The Phase 13 Tier
  3 `gitnexus_sync` Tauri command populates it so `gitnexus_unmirror` can roll
  back exactly one sync without touching native or LLM-extracted edges.

---

## 9. Why SQLite?

TerranSoul is a **desktop app** (Tauri 2.x), not a web service. The database must satisfy very different constraints than a server-side application:

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

### Database Location by OS

| OS | Path |
|---|---|
| **Windows** | `%APPDATA%\com.terransoul.app\memory.db` |
| **macOS** | `~/Library/Application Support/com.terransoul.app/memory.db` |
| **Linux** | `~/.local/share/com.terransoul.app/memory.db` |

---

## 10. Brain Modes & Provider Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                    BRAIN MODE ARCHITECTURE                           │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │              Provider Selection (BrainMode)                  │   │
│  │                                                              │   │
│  │  ┌───────────┐  ┌───────────┐  ┌────────────────────────┐  │   │
│  │  │ free_api  │  │ paid_api  │  │ local_ollama           │  │   │
│  │  │           │  │           │  │                        │  │   │
│  │  │ Pollina-  │  │ OpenAI    │  │ Ollama server          │  │   │
│  │  │ tions AI  │  │ Anthropic │  │ localhost:11434         │  │   │
│  │  │           │  │ Groq      │  │                        │  │   │
│  │  │ No key    │  │ User key  │  │ Full privacy           │  │   │
│  │  │ No embed  │  │ No embed  │  │ Local embed            │  │   │
│  │  │           │  │           │  │ nomic-embed-text       │  │   │
│  │  └─────┬─────┘  └─────┬─────┘  └───────────┬────────────┘  │   │
│  │        │              │                     │               │   │
│  │        ▼              ▼                     ▼               │   │
│  │  ┌──────────────────────────────────────────────────────┐  │   │
│  │  │              Unified LLM Interface                    │  │   │
│  │  │                                                       │  │   │
│  │  │  call(prompt, system) → String                       │  │   │
│  │  │  call_streaming(prompt, system) → SSE events         │  │   │
│  │  │  embed(text) → Option<Vec<f32>>                      │  │   │
│  │  └──────────────────────────────────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  RAG capability by mode:                                            │
│                                                                     │
│  ┌──────────────┬──────────┬──────────┬──────────────────────┐     │
│  │ Signal       │ Free API │ Paid API │ Local Ollama         │     │
│  ├──────────────┼──────────┼──────────┼──────────────────────┤     │
│  │ Vector (40%) │    ◐*    │    ✓**   │ ✓ (nomic-embed-text)│     │
│  │ Keyword(20%) │    ✓     │    ✓     │ ✓                    │     │
│  │ Recency(15%) │    ✓     │    ✓     │ ✓                    │     │
│  │ Import.(10%) │    ✓     │    ✓     │ ✓                    │     │
│  │ Decay  (10%) │    ✓     │    ✓     │ ✓                    │     │
│  │ Tier    (5%) │    ✓     │    ✓     │ ✓                    │     │
│  ├──────────────┼──────────┼──────────┼──────────────────────┤     │
│  │ Effective    │ 60–100%  │ 100%     │ 100%                 │     │
│  │ RAG quality  │ (varies) │ (full)   │ (full hybrid)        │     │
│  └──────────────┴──────────┴──────────┴──────────────────────┘     │
│                                                                     │
│  * Free API vector: depends on provider (Mistral/GitHub Models     │
│    yes via cloud_embeddings; Pollinations/Groq no). Chunk 16.9.    │
│  ** Paid API: OpenAI-compatible /v1/embeddings via Chunk 16.9.     │
│                                                                     │
│  Model selection:                                                   │
│  • model_recommender.rs — RAM-based catalogue                      │
│  • Auto-selects best model for available hardware                   │
│  • Catalogue includes: Gemma 4, Phi-4, Qwen 3, Kimi K2.6 (cloud) │
│  • ProviderRotator — cycles through free providers on failure      │
└─────────────────────────────────────────────────────────────────────┘
```

#### Machine-readable model catalogue

> Parsed by `brain::doc_catalogue::parse_catalogue()` at runtime.
> Keep column order: model_tag | display_name | description | required_ram_mb | is_cloud.

<!-- BEGIN MODEL_CATALOGUE -->

| model_tag | display_name | description | required_ram_mb | is_cloud |
|---|---|---|---|---|
| gemma4:31b | Gemma 4 31B | Google's dense 30.7B flagship. State-of-the-art reasoning, coding, and 256K context. 20 GB download. | 24576 | false |
| gemma4:26b | Gemma 4 26B MoE | MoE with 25.2B total / 3.8B active params. Fast inference with 256K context. 18 GB download. | 22528 | false |
| gemma3:27b | Gemma 3 27B | Previous-gen flagship. Excellent reasoning, vision, and 128K context. 17 GB download. | 20480 | false |
| gemma4:e4b | Gemma 4 E4B | 4.5B effective params optimised for edge. 128K context, vision + audio. 9.6 GB download. | 12288 | false |
| gemma4:e2b | Gemma 4 E2B | 2.3B effective params for edge devices. 128K context, vision + audio. 7.2 GB download. | 8192 | false |
| gemma3:4b | Gemma 3 4B | Compact multimodal model. 128K context, great for everyday chat. 3.3 GB download. | 6144 | false |
| phi4-mini | Phi-4 Mini 3.8B | Microsoft's compact reasoner. 128K context, strong math/logic. 2.5 GB download. | 4096 | false |
| gemma3:1b | Gemma 3 1B | Ultra-lightweight. 32K context, text-only. 815 MB download. | 2048 | false |
| gemma2:2b | Gemma 2 2B | Compact Gemma 2. 8K context, solid for simple tasks. 1.6 GB download. | 4096 | false |
| tinyllama | TinyLlama 1.1B | Minimal 1.1B model. 2K context. Works on very limited hardware. 638 MB download. | 2048 | false |
| kimi-k2.6:cloud | Kimi K2.6 (Cloud) | Moonshot AI's 1T MoE (32B active). Vision, tool use, thinking. 256K context. No local GPU needed. | 0 | true |

<!-- END MODEL_CATALOGUE -->

<!-- BEGIN TOP_PICKS -->

| tier | model_tag |
|---|---|
| VeryHigh | gemma4:31b |
| High | gemma4:e4b |
| Medium | gemma4:e2b |
| Low | gemma3:1b |
| VeryLow | tinyllama |

<!-- END TOP_PICKS -->

### 10.1. External CLI backend (Chunk 1.5)

In addition to the three **native** brain modes above, TerranSoul
agents may be backed by an **external CLI worker** (`codex`, `claude`,
`gemini`, or a user-validated custom binary) bound to a working folder.
External CLI agents route chat turns through
[`cli_worker.rs`](../src-tauri/src/agents/cli_worker.rs) instead of the
unified LLM interface — stdout and stderr stream back as chat lines,
and progress is persisted to an append-only workflow history so a
killed app can resume the job. See
[`instructions/AGENT-ROSTER.md`](../instructions/AGENT-ROSTER.md) for
the full sandbox model, the RAM-aware concurrency cap, and the
durable-workflow replay semantics.

---

## 11. LLM-Powered Memory Operations

```
┌─────────────────────────────────────────────────────────────────────┐
│              LLM-POWERED MEMORY OPERATIONS                          │
│                                                                     │
│  These operations use the active LLM to enhance memory quality:    │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │ 1. EXTRACT FACTS                                            │   │
│  │                                                              │   │
│  │ Input:  Last N conversation messages                        │   │
│  │ Prompt: "Extract the 5 most important facts from this       │   │
│  │          conversation. Return as a JSON array of strings."  │   │
│  │ Output: ["User's name is Alex", "Studying family law", ...] │   │
│  │ Stored: tier=working, type=fact                             │   │
│  │ Trigger: User clicks "Extract from session" or auto at 20+ │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │ 2. SUMMARIZE SESSION                                        │   │
│  │                                                              │   │
│  │ Input:  Full conversation history                           │   │
│  │ Prompt: "Summarize this conversation in 1-3 sentences."     │   │
│  │ Output: "Discussed family law filing deadlines and..."      │   │
│  │ Stored: tier=working, type=summary, parent_id=session_id    │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │ 3. SEMANTIC SEARCH (legacy)                                 │   │
│  │                                                              │   │
│  │ Input:  Query + all memory entries (sent in one prompt!)    │   │
│  │ Prompt: "Rank these memories by relevance to the query."    │   │
│  │ Output: Ordered list of memory IDs                          │   │
│  │ Status: DEPRECATED — replaced by hybrid_search()            │   │
│  │ Limit:  ~500 entries before context overflow                │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │ 4. EMBED TEXT                                               │   │
│  │                                                              │   │
│  │ Input:  Any text string                                     │   │
│  │ Model:  nomic-embed-text (768-dim) via Ollama               │   │
│  │ Output: Option<Vec<f32>> (None if Ollama unavailable)       │   │
│  │ Used:   On memory insert, on query for hybrid search        │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │ 5. DUPLICATE CHECK                                          │   │
│  │                                                              │   │
│  │ Input:  New memory content                                  │   │
│  │ Method: Embed → cosine similarity vs all existing           │   │
│  │ Threshold: > 0.97 = duplicate                               │   │
│  │ Action: Skip insert, return existing memory ID              │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │ 6. BACKFILL EMBEDDINGS                                      │   │
│  │                                                              │   │
│  │ Input:  All memories with NULL embedding column             │   │
│  │ Method: Batch embed via Ollama, update BLOB column          │   │
│  │ Output: Count of newly embedded memories                    │   │
│  │ Trigger: Manual button or auto when Ollama first detected   │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │ 7. EXTRACT EDGES (V5 / Entity-Relationship Graph)           │   │
│  │                                                              │   │
│  │ Input:  All memories (chunked, default 25 per call)         │   │
│  │ Method: LLM proposes JSON-line edges with rel_type +        │   │
│  │         confidence; parser drops self-loops, unknown ids,   │   │
│  │         and clamps confidence to [0, 1].                     │   │
│  │ Output: Count of new edges inserted                         │   │
│  │ Trigger: 🔗 Extract edges button or `extract_edges_via_brain` │  │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │ 8. MULTI-HOP HYBRID SEARCH (V5)                             │   │
│  │                                                              │   │
│  │ Input:  Query text + optional embedding + hops (≤ 3)        │   │
│  │ Method: hybrid_search → BFS each hit `hops` deep →          │   │
│  │         re-rank with `seed_score / (hop + 1)`                │   │
│  │ Output: Top-N memories (vector hits + graph neighbours)     │   │
│  │ Trigger: `multi_hop_search_memories` Tauri command          │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  Future operations:                                                 │
│  • auto_categorize() — LLM assigns category from taxonomy         │
│  • extract_entities() — LLM identifies people, places, concepts   │
│  • detect_conflicts() — LLM finds contradicting memories          │
│  • merge_duplicates() — LLM combines near-duplicate content       │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 12. Multi-Source Knowledge Management

Real-world knowledge comes from many sources that overlap, conflict, and go stale. TerranSoul handles this with four mechanisms:

### 1. Source Hash Change Detection

Every ingested document is tracked by URL and SHA-256 content hash:

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

### 2. TTL Expiry

Some knowledge has a natural shelf life. The `expires_at` column allows auto-expiry:

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

### 3. Access Count Decay

Memories that RAG never retrieves are probably not useful:

```sql
-- Find memories older than 90 days that were never accessed by RAG
SELECT id, content, created_at, access_count
FROM memories
WHERE access_count = 0
  AND created_at < strftime('%s','now') * 1000 - 7776000000  -- 90 days
ORDER BY created_at ASC;
```

These zero-access memories are prime candidates for GC — they were stored but never contributed to any RAG response.

### 4. LLM-Powered Conflict Resolution

When new information semantically overlaps with existing knowledge but says something different, the LLM analyzes the conflict:

```
Existing memory (id=42):
  "Family law motion responses must be filed within 30 days"
  source: ilcourts.gov/rules/family/14.3
  stored: 2026-03-15

New incoming content:
  "Effective April 1, 2026: Family law motion responses now have a
   21-day filing deadline (reduced from 30 days)"
  source: ilcourts.gov/rules/family/14.3-amended

Conflict detection:
  1. Embed new content → cosine similarity with id=42 = 0.94
     (high similarity, but below 0.97 dedup threshold)
  2. Both reference "family law motion response deadline"
  3. But values differ: 30 days vs 21 days

LLM conflict analysis prompt:
  "Compare these two pieces of information and determine if the
   new one supersedes the old one:
   OLD: [30-day filing deadline]
   NEW: [21-day filing deadline, effective April 1]
   Which is current?"

LLM response:
  "The new information supersedes the old. The deadline was reduced
   from 30 to 21 days effective April 1, 2026."

Action:
  → Mark id=42 as expired (or delete)
  → Insert new memory with updated content
  → Log: "Conflict resolved: Rule 14.3 deadline updated 30d → 21d"
```

This is inspired by **Mem0's conflict resolution** approach — using the LLM itself to arbitrate when two memories say different things about the same topic.

---

## 13. Open-Source RAG Ecosystem Comparison

TerranSoul's RAG pipeline is purpose-built for a single-user desktop companion. Here's how it compares to the leading open-source RAG and memory frameworks:

### Cross-framework comparison at a glance

The table below contrasts TerranSoul against five widely-used RAG / knowledge
systems with very different design centres: **LangChain** (programmable RAG
framework), **Odyssey** ([odyssey-llm/odyssey](https://github.com/odyssey-llm/odyssey),
multi-agent orchestration with long-term memory), **RAGFlow** (enterprise
deep-document RAG), **SiYuan** ([siyuan-note/siyuan](https://github.com/siyuan-note/siyuan),
local-first block-based knowledge notebook with RAG plugins), and **GitNexus**
(repo-aware code-RAG for GitHub navigation). The goal is a single side-by-side
view rather than per-project deep-dives.

| Dimension | TerranSoul | LangChain | Odyssey | RAGFlow | SiYuan | GitNexus |
|---|---|---|---|---|---|---|
| Primary purpose | Personal AI companion w/ 3D avatar + RAG memory | General-purpose LLM/RAG framework (lib + LCEL) | Multi-agent task orchestration with persistent memory | Enterprise deep-document RAG | Local-first block-based note-taking + RAG plugin | Repo-aware code-RAG for GitHub exploration |
| Distribution | Single Tauri binary (Win/macOS/Linux/iOS/Android) | Python / JS package | Python framework | Docker Compose stack (server) | Electron desktop + optional self-host server | Hosted service / CLI |
| Storage backend | Embedded SQLite + BLOB embeddings | Bring-your-own (FAISS, Chroma, pgvector, …) | Bring-your-own vector DB | Elasticsearch + MinIO + MySQL + Redis | Local filesystem (Markdown/JSON blocks) + optional vector index | Repo-scoped index (cloud-hosted) |
| Embedding model | Ollama `nomic-embed-text` (local, default) | Any provider (OpenAI/HF/Ollama/…) | Any provider | Built-in + pluggable | Pluggable (BGE / OpenAI / local) | Provider-managed |
| Retrieval strategy | Hybrid: cosine ANN + keyword + tag/recency boost | Composable (vector / BM25 / hybrid / multi-query / reranker) | Memory hierarchy + agent-tool retrieval | Layout-aware chunking + reranking + citation | Tag/link graph + vector search inside notebooks | Code-symbol-aware retrieval over repo graph |
| Knowledge graph | Typed entity-relationship graph (V5: directional `memory_edges`, multi-hop traversal) | LangGraph (separate package) | First-class graph memory | Document → chunk → citation graph | Bidirectional block links + tag graph (native) | AST + import/call graph over repo |
| Memory model | Three tiers: short-term, working, long-term + decay/GC | Conversation/buffer/summary memories (opt-in) | Hierarchical episodic + semantic memory across agents | None — stateless retrieval per query | Notebook history (no LLM-managed memory) | Session-scoped repo cache |
| Multi-modal ingest | Text via external scripts; images via vision tool | Connectors for 100+ sources (community) | Tool-driven ingestion | Native PDF/DOCX/PPTX/image w/ layout parsing | Markdown / PDF / images attached to blocks | Source code (most languages) |
| Conflict / decay handling | Hash-based staleness + LLM conflict resolver + decay scoring | None (manual) | LLM-mediated reconciliation between agents | Document versioning | Manual edits; Git-style history per block | Re-index on commit |
| Offline / privacy | 100 % offline-capable; data never leaves device | Depends on chosen providers | Depends on chosen providers | Server-bound; data stays on infra you run | Fully local-first by default | Cloud-hosted (telemetry-bearing) |
| Multi-user / collab | Single user + CRDT sync across that user's own devices | N/A (library) | Multi-agent, single user | Multi-tenant with RBAC | Single user; optional sync | Multi-user (org/repo scoped) |
| Programming surface | Tauri commands consumed by Vue 3 | Python + JS APIs (LCEL, Runnable) | Python agent SDK | REST API + UI | JS plugin API + Lua-ish kernel | REST / CLI |
| Best fit | Always-on personal companion that knows your life | Building bespoke RAG/agent apps | Multi-agent assistants needing persistent memory | Teams indexing large heterogeneous corpora | Researchers / writers managing personal knowledge | Developers exploring large repos |
| License | MIT | MIT | Apache-2.0 (per repo) | Apache-2.0 | AGPL-3.0 | Mixed (server proprietary, clients OSS) |

> Note: the GitHub URLs vary per project; the sources above were chosen as the
> canonical "headline" repository for each. Project metadata (license, exact
> feature set) drifts over time — update this table together with section 13.x
> when you upgrade or replace a comparator.

### Mem0 (53.7k stars)

| Capability | Mem0 | TerranSoul |
|---|---|---|
| Memory storage | External vector DB (Qdrant/Chroma) | Embedded SQLite — zero infra |
| Entity extraction | Automatic via LLM | Manual tags + LLM-assisted extract |
| Memory levels | User / Session / Agent | Short / Working / Long |
| Graph relationships | Built-in | Typed entity-relationship graph (V5: directional edges, multi-hop search) |
| Conflict resolution | LLM-powered automatic | Hash-based staleness + LLM conflict |
| Deployment | Requires server + vector DB | Fully embedded, works offline |

**What TerranSoul borrows**: LLM-powered memory extraction and conflict detection. Mem0's graph memory layer is now mirrored in TerranSoul's V5 entity-relationship graph (`memory_edges` table + `multi_hop_search_memories` command).

### LlamaIndex (48.8k stars)

| Capability | LlamaIndex | TerranSoul |
|---|---|---|
| PDF ingestion | LlamaParse (cloud API) | External script + text extraction |
| Data connectors | 160+ built-in | Manual ingestion scripts per source |
| Query pipeline | Composable (tree/compact/refine) | Single-pass vector search + inject |
| Embedding | Any provider | Ollama nomic-embed-text (local) |
| Chunking | Sentence/token/semantic splitters | 500-word overlap chunking |
| Deployment | Python library | Rust native binary |

**What TerranSoul borrows**: The chunking strategy (500-word segments with 50-word overlap) is inspired by LlamaIndex's sentence window approach.

### ChromaDB (27.6k stars)

| Capability | ChromaDB | TerranSoul |
|---|---|---|
| Storage | Custom Rust engine | SQLite BLOB column |
| Distance function | Cosine / L2 / IP | Cosine only |
| Metadata filtering | Built-in | SQL WHERE clauses on tags/importance |
| Indexing | HNSW (approximate) | Brute-force linear scan |
| Scalability | Millions (ANN) | Millions (acceptable at <50ms) |
| Deployment | Separate server or embedded | Fully embedded in app binary |

**What TerranSoul borrows**: The philosophy of "embeddings in a single binary." ChromaDB proves that a Rust core + simple API can handle production workloads.

### RAGFlow (78.7k stars)

| Capability | RAGFlow | TerranSoul |
|---|---|---|
| Document parsing | Deep layout understanding | Plain text extraction |
| File formats | 30+ (PDF, DOCX, PPTX, images) | Text-based (via external scripts) |
| Chunk visualization | Built-in UI | Memory View + access_count tracking |
| Deployment | Docker (server-based) | Desktop app (no server) |
| Target user | Enterprise teams | Individual power users |

**What TerranSoul borrows**: The chunk visualization concept — tracking which memories are actually used by RAG via `access_count`.

### Cognee (16.6k stars)

| Capability | Cognee | TerranSoul |
|---|---|---|
| Knowledge representation | Graph + Vector | Vector only (tags for structure) |
| Multi-hop reasoning | Graph traversal | Not yet (single-hop vector search) |
| Entity extraction | Automatic | LLM-assisted ("Extract from session") |
| Deployment | Python library | Rust native binary |

**Status (V5, April 2026)**: Cognee's graph-based approach now lives in TerranSoul's V5 schema. The shipped `memory_edges` table + `multi_hop_search_memories` command answer exactly that class of multi-hop query: a vector hit on "Smith case" expands one hop along `mentions` / `cites` / `governs` edges to surface every connected client and their communication-preference memory.

### Why TerranSoul Doesn't Use an External RAG Framework

| Decision Factor | External Framework | TerranSoul Built-in |
|---|---|---|
| **Zero dependencies** | Requires Python/Docker/server | Just install the app |
| **Offline-first** | Most need network for vector DB | SQLite works offline always |
| **Privacy** | Data may leave the machine | Everything stays local |
| **Single binary** | Multiple processes to manage | One Tauri binary |
| **Desktop UX** | Built for servers/APIs | Built for desktop companion |
| **Performance** | Network overhead | In-process, <5ms search |
| **Maintenance** | Version compatibility issues | Self-contained canonical schema |

TerranSoul's approach: **take the best ideas** from these frameworks (Mem0's conflict detection, LlamaIndex's chunking, Chroma's Rust-native search, RAGFlow's access tracking, Cognee's entity extraction vision) and **implement them natively in Rust** as part of the Tauri binary.

---

## 14. Debugging with SQLite

### Recommended Tools

#### DB Browser for SQLite (GUI)

Download from https://sqlitebrowser.org/dl/ — open `memory.db` directly:

```
┌─ DB Browser for SQLite ────────────────────────────────┐
│ File  Edit  View  Tools  Help                           │
│                                                          │
│ Database: memory.db                                      │
│                                                          │
│ Tables:                                                  │
│  ├── memories (15,247 rows)                              │
│  ├── memory_edges                                        │
│  ├── memory_versions                                     │
│  ├── memory_conflicts                                    │
│  └── schema_version (1 canonical row)                    │
│                                                          │
│ ┌──────────────────────────────────────────────────────┐ │
│ │ Browse Data │ Execute SQL │ DB Structure │ Edit Prag │ │
│ ├──────────────────────────────────────────────────────┤ │
│ │ Table: memories ▾                                    │ │
│ ├────┬──────────────────────┬──────┬─────┬─────┬──────┤ │
│ │ id │ content              │ tags │ imp │ type│ tier │ │
│ ├────┼──────────────────────┼──────┼─────┼─────┼──────┤ │
│ │  1 │ Filing deadline: 30d │ law  │  5  │ fact│ long │ │
│ │  2 │ Client prefers email │ pref │  4  │ pref│ long │ │
│ │  3 │ Office hours M-F 9-5 │ info │  2  │ fact│ long │ │
│ └────┴──────────────────────┴──────┴─────┴─────┴──────┘ │
└─────────────────────────────────────────────────────────┘
```

#### sqlite3 CLI (Terminal)

```bash
sqlite3 "%APPDATA%/com.terransoul.app/memory.db"

.tables        -- → memories memory_edges memory_versions memory_conflicts paired_devices schema_version sync_log
.schema memories
```

#### VS Code Extension

Install "SQLite Viewer" (`qwtel.sqlite-viewer`) — open `memory.db` directly in VS Code.

### Useful Debug Queries

```sql
-- Embedding coverage
SELECT
    COUNT(*) AS total,
    COUNT(embedding) AS embedded,
    COUNT(*) - COUNT(embedding) AS unembedded
FROM memories;

-- Most-accessed memories (RAG hits)
SELECT id, content, access_count, last_accessed
FROM memories
ORDER BY access_count DESC
LIMIT 10;

-- Never-retrieved memories (candidates for GC)
SELECT id, content, created_at
FROM memories
WHERE access_count = 0
ORDER BY created_at DESC
LIMIT 20;

-- Embedding size validation (expect 3072 bytes = 768 dims × 4 bytes)
SELECT id, content, LENGTH(embedding) AS embed_bytes,
       LENGTH(embedding) / 4 AS dimensions
FROM memories
WHERE embedding IS NOT NULL
LIMIT 5;

-- Canonical schema marker
SELECT version, description,
       datetime(applied_at / 1000, 'unixepoch', 'localtime') AS applied
FROM schema_version
ORDER BY version;

-- Memory distribution by tier and type
SELECT tier, memory_type, COUNT(*), AVG(importance), AVG(decay_score)
FROM memories GROUP BY tier, memory_type;

-- Find exact duplicates
SELECT a.id, b.id AS dup_id, a.content
FROM memories a
JOIN memories b ON a.id < b.id AND a.content = b.content;

-- Database health check
PRAGMA integrity_check;   -- → ok
PRAGMA journal_mode;      -- → wal
PRAGMA page_count;
PRAGMA page_size;
```

### Common Debugging Scenarios

**"My memories aren't being found by RAG"**
```sql
-- Check if the memory has an embedding
SELECT id, content, embedding IS NOT NULL AS has_embedding
FROM memories WHERE content LIKE '%your search term%';
-- If has_embedding = 0, run backfill: invoke('backfill_embeddings')
```

**"RAG is returning irrelevant results"**
```sql
-- Check low-importance entries polluting results
SELECT id, content, importance
FROM memories WHERE importance <= 2 AND access_count > 10;
-- Consider increasing importance or deleting irrelevant entries
```

**"Database seems corrupted"**
```sql
PRAGMA integrity_check;
-- If not "ok": close app → copy memory.db.bak → memory.db → reopen
```

---

## 15. Hardware Scaling

### Memory Count → Hardware Requirements

| Memory Count | Embedding Storage | RAM Usage | Search Time | Recommended Hardware |
|---|---|---|---|---|
| 1,000 | 3 MB | ~50 MB | <1 ms | Any modern PC |
| 10,000 | 30 MB | ~100 MB | ~2 ms | 8 GB RAM |
| 100,000 | 300 MB | ~500 MB | ~5 ms | 16 GB RAM |
| 1,000,000 | 3 GB | ~4 GB | ~50 ms | 32 GB RAM |
| 10,000,000 | 30 GB | ~35 GB | ~500 ms | 64 GB RAM |

### Example: High-End Desktop (65 GB RAM, RTX 3080 Ti)

```
Capacity breakdown:
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

### Scaling Beyond Linear Scan

For datasets exceeding 1M entries where <50ms search is needed:
- **HNSW index** (via `usearch` crate): Approximate Nearest Neighbor — O(log n) instead of O(n)
- **Sharding**: Split memories across multiple SQLite files by date/topic
- **External vector DB**: Connect to Qdrant/Milvus as a Tauri sidecar

The current pure-cosine approach is intentionally simple and works for the vast majority of use cases.

---

## 16. Scaling Roadmap

### Current Limits

| Metric | Current | Target |
|--------|---------|--------|
| Total memories | ~500 (brute-force LLM search) | 100,000+ (hybrid search) |
| Search latency | <5ms (hybrid) | <10ms at 1M entries (ANN) |
| Embedding model | nomic-embed-text (768-dim) | Same (good quality/size ratio) |
| RAG quality | 60% (no embed) to 100% (Ollama) | 100% via cloud embed API |
| Visualization | Cytoscape.js with typed graph edges (V5) | + Obsidian vault export |
| Categories | 4 types (flat) | 8 categories (hierarchical) |
| Relationships | Typed entity-relationship graph (V5) | Conflict detection + temporal links |

### Phase Plan

```
┌─────────────────────────────────────────────────────────────────────┐
│                      SCALING ROADMAP                                │
│                                                                     │
│  PHASE 1 — Foundation (Current)                                     │
│  ├── ✓ Three-tier memory model (short/working/long)                │
│  ├── ✓ Hybrid 6-signal search                                      │
│  ├── ✓ Exponential decay + GC                                      │
│  ├── ✓ Cytoscape.js graph visualization                            │
│  ├── ✓ LLM extract/summarize/embed                                 │
│  └── ✓ Deduplication via cosine threshold                          │
│                                                                     │
│  PHASE 2 — Categories & Graph (✅ Shipped via tag-prefix convention) │
│  ├── ✓ Tag-prefix vocabulary as category surrogate                 │
│  │     (`memory::tag_vocabulary::CURATED_PREFIXES` —                │
│  │      `personal:`, `domain:`, `project:`, `event:`, `temporal:`,  │
│  │      `meta:`) — Chunk 18.4                                       │
│  ├── ✓ Auto-categorise via LLM on insert                           │
│  │     (LLM prompted with curated prefix list) — Chunk 18.1         │
│  ├── ✓ Category-aware decay rates                                  │
│  │     (`category_decay_multiplier(tags_csv)` →                     │
│  │      slowest-decaying prefix wins) — Chunk 18.2                  │
│  ├── ✓ Category filters in Memory View                             │
│  │     (multi-select chip row) — Chunk 18.3                         │
│  └── ✓ Obsidian vault export (one-way) — Chunk 18.5                │
│                                                                     │
│  PHASE 3 — Entity Graph (✅ Shipped — canonical schema)             │
│  ├── ✓ memory_edges table (FK cascade)                             │
│  ├── ✓ LLM-powered edge extraction (extract_edges_via_brain)       │
│  ├── ✓ Relationship type taxonomy (17 curated types + free-form)   │
│  ├── ✓ Multi-hop RAG via graph traversal (hybrid_search_with_graph)│
│  ├── ✓ Graph-enhanced Cytoscape visualization (typed/directional)  │
│  └── ✓ Conflict detection between connected memories               │
│        (`memory::edge_conflict_scan` — collect_scan_candidates +    │
│         record_contradiction, LLM-as-judge over positive-relation   │
│         edges, 3-phase lock-safe pattern) — Chunk 17.6              │
│                                                                     │
│  PHASE 4 — Scale                                                    │
│  ├── ✓ ANN index (usearch crate) for >1M memories                 │
│  │     (`memory::ann_index::AnnIndex` — HNSW via usearch 2.x,       │
│  │      lazy OnceCell init, auto-rebuild, periodic save)             │
│  │     — Chunk 16.10                                                │
│  ├── ✓ Cloud embedding API for free/paid modes                     │
│  │     (`brain::cloud_embeddings::embed_for_mode` dispatcher,       │
│  │      OpenAI-compat `/v1/embeddings`) — Chunk 16.9                │
│  ├── ✓ Chunking pipeline for large documents                       │
│  │     (`memory::chunking`, `text-splitter` crate, semantic         │
│  │      Markdown/text splitting, dedup, heading metadata)           │
│  │     — Chunk 16.11                                                │
│  │  ├── ✓ Relevance threshold (skip injection if score < 0.3,         │
│  │  │     user-tunable via `AppSettings.relevance_threshold`,         │
│  │  │     `MemoryStore::hybrid_search_with_threshold`) — Chunk 16.1   │
│  ├── ✓ One-way Obsidian vault export (`export_to_obsidian` command,  │
│  │     `memory::obsidian_export`) — Chunk 18.5                      │
│  ├── ○ Bidirectional Obsidian sync (extends 18.5)                  │
│  └── ✓ Memory versioning (`memory::versioning`, V8 schema,         │
│        `memory_versions` table, `get_memory_history` command)       │
│        — Chunk 16.12                                                │
│                                                                     │
│  PHASE 5 — Intelligence                                             │
│  ├── ✓ Auto-promotion based on access patterns                     │
│  │     (`MemoryStore::auto_promote_to_long`,                        │
│  │      command `auto_promote_memories`) — Chunk 17.1               │
│  ├── ✓ Contradiction resolution (LLM picks winner)                 │
│  │     (`memory::conflicts` — V9 schema, `MemoryConflict` CRUD,    │
│  │      losers soft-closed via `valid_to`) — Chunk 17.2             │
│  ├── ✓ Temporal reasoning (`memory::temporal::parse_time_range` +   │
│  │     `temporal_query` command) — Chunk 17.3                       │
│  ├── ✓ Memory importance auto-adjustment from access_count         │
│  │     (`MemoryStore::adjust_importance_by_access`,                  │
│  │      command `adjust_memory_importance`) — Chunk 17.4            │
│  └── ✓ Cross-device memory merge via CRDT sync                    │
│        (`memory::crdt_sync` LWW deltas + `link::handlers`         │
│         Soul Link `memory_sync` / `memory_sync_request`,           │
│         Unix-ms `sync_log` watermarks) — Chunks 17.5a/b            │
│                                                                     │
│  PHASE 6 — Modern RAG (April 2026 research absorption — see §19)   │
│  ├── ✓ Reciprocal Rank Fusion utility (memory/fusion.rs)           │
│  ├── ✓ RRF wired into hybrid_search (vector + keyword + freshness  │
│  │     retrievers fused with k = 60) — `hybrid_search_rrf`         │
│  ├── ✓ HyDE — Hypothetical Document Embeddings search command      │
│  │     (`memory/hyde.rs` + `hyde_search_memories` Tauri command)   │
│  ├── ✓ Cross-encoder reranking pass (LLM-as-judge style;           │
│  │     `memory/reranker.rs` + `rerank_search_memories` command)    │
│  ├── ✓ Contextual Retrieval (Anthropic 2024) — LLM-prepended chunk │
│  │     context before embedding (`memory::contextualize`,          │
│  │     `AppSettings.contextual_retrieval`) — Chunk 16.2            │
│  ├── ✓ Late chunking ingest integration — opt-in whole-document    │
│  │     token embeddings + pooled chunk vectors via `late_chunking` │
│  ├── ○ GraphRAG / LightRAG-style community summaries over          │
│  │     memory_edges (multi-hop + LLM cluster summary)              │
│  ├── ◐ Self-RAG controller shipped (`orchestrator::self_rag` —    │
│  │     reflection-token parser + 3-iteration decision SM,           │
│  │     Chunk 16.4a); orchestrator-loop integration is Chunk 16.4b   │
│  ├── ◐ CRAG retrieval evaluator shipped (`memory::crag` —        │
│  │     `parse_verdict` + `aggregate` over CORRECT/AMBIGUOUS/        │
│  │     INCORRECT, Chunk 16.5a); query-rewrite + web-search          │
│  │     fallback is Chunk 16.5b                                      │
│  ├── ✓ Sleep-time consolidation (Letta-style background job that    │
│  │     compresses/links short→working→long during idle)             │
│  │     (`memory::consolidation`, schedule via workflows) — Chunk 16.7│
│  ├── ✓ Temporal knowledge graph (Zep / Graphiti-style valid_from / │
│  │     valid_to columns on memory_edges, V6 schema)                │
│  └── ✓ Matryoshka embeddings (truncate to 256-dim fast pass +    │
│        full-dim re-rank; `memory::matryoshka` module +              │
│        `matryoshka_search_memories` Tauri command) — Chunk 16.8     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 17. FAQ

### "What if Ollama is not running?"

TerranSoul gracefully degrades:
- **Vector search**: Skipped (no embedding available for query)
- **Fallback**: Keyword + temporal signals only (60% RAG quality)
- **Chat**: Uses Free Cloud API or Paid API if configured as backup

### "Can I export/import memories?"

```sql
-- Export to CSV
.mode csv
.headers on
.output memories_backup.csv
SELECT id, content, tags, importance, memory_type, tier, created_at FROM memories;
.output stdout

-- Import from CSV
.mode csv
.import memories_backup.csv memories
```

For richer export, use the Obsidian vault export (§7 Layer 2) which preserves metadata, relationships, and category structure.

### "How do I add memories programmatically?"

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
// { schema_version: 13, total_memories: 15247, embedded_count: 15200, ... }
```

### "What's the difference between search and semantic search?"

| Feature | `search_memories` | `semantic_search_memories` | `hybrid_search_memories` |
|---|---|---|---|
| Method | SQL `LIKE '%keyword%'` | Cosine similarity | 6-signal scoring |
| Speed | <1ms (any size) | ~50ms embed + <5ms search | ~50ms embed + <5ms search |
| Accuracy | Exact match only | Understands meaning | Best of both worlds |
| Requires Brain | No | Yes (Ollama for embedding) | Partial (degrades gracefully) |
| Example | "deadline" finds "deadline" | "when to file" finds "30-day deadline" | "when to file" finds "30-day deadline" + recency/importance boost |

### "How does the memory graph connect to categories?"

Currently, the Cytoscape.js graph connects nodes (memories) via shared tags. With the proposed category taxonomy (§3), the graph gains a second axis:

- **Tags** create horizontal connections (memories about the same topic)
- **Categories** create vertical grouping (all personal info, all domain knowledge, etc.)
- **Obsidian export** (§7) provides the best visualization — category folders become Obsidian folders, `[[wikilinks]]` become graph edges, and Obsidian's Graph View renders the full knowledge topology

---

## 18. Diagrams Index

Quick reference for all diagrams in this document:

| Section | Diagram | Description |
|---------|---------|-------------|
| §1 | System Overview | Full stack: Vue → Tauri IPC → Rust → SQLite → LLM |
| §2 | Short/Working/Long boxes | Three-tier memory details |
| §2 | Tier Lifecycle | Promotion chain: short → working → long → GC |
| §3 | Category × Type matrix | 8 categories × 4 types grid |
| §4 | 6-Signal breakdown | Hybrid search weights and ranges |
| §4 | RAG Injection Flow | 4-step: embed → search → format → inject |
| §4 | Embedding Architecture | Model, storage, budget, dedup |
| §5 | Decay curve | Exponential forgetting over 5 weeks |
| §5 | Category decay table | Proposed per-category decay rates |
| §6 | Tag-based graph | Current Cytoscape model |
| §6 | Entity-relationship graph | Shipped V5 graph with typed edges |
| §6 | Multi-hop RAG | Graph traversal for related memories |
| §7 | In-app graph | Cytoscape.js with filters |
| §7 | Obsidian vault structure | Folder tree + Markdown format |
| §9 | Why SQLite | WAL mode, auto-backup, DB location |
| §10 | Brain modes | Provider architecture + RAG capability matrix |
| §11 | LLM operations | 6 current + future operations |
| §12 | Staleness detection | Source hash change flow |
| §12 | Conflict resolution | LLM-powered contradiction handling |
| §13 | RAG ecosystem | 5 framework comparison tables |
| §14 | DB Browser | SQLite debug tool UI |
| §15 | Hardware scaling | Memory count → RAM/speed table |
| §16 | Scaling roadmap | 6-phase plan from foundation to modern-RAG absorption |
| §19 | Research survey | April 2026 modern RAG / agent-memory technique map |
| §20 | Selection topology | How the LLM / orchestrator chooses each brain component |
| §21 | Write-back loop | How daily conversation updates the brain (auto-learn cadence, fact extraction, decay) |

---

## 19. April 2026 Research Survey — Modern RAG & Agent-Memory Techniques

> **Why this section exists**: the RAG and agent-memory landscape moved fast in 2024–2026. This section catalogs every major technique that emerged or matured during that window, maps each one to TerranSoul's current implementation status, and links each gap to a concrete Phase 6 roadmap item (§16). It is the canonical "what are we missing?" reference — consult it (alongside §10 and §13) before designing any new brain / memory work.

### 19.1 Status legend

| Symbol | Meaning |
|---|---|
| ✅ | Shipped in the current binary |
| 🟡 | Partial / foundations in place, full feature pending |
| 🔵 | Documented gap with concrete Phase 6 roadmap item |
| ⚪ | Intentionally rejected (does not fit single-user desktop companion) |

### 19.2 Technique → TerranSoul status map

| # | Technique (year, source) | What it is | TerranSoul status | Where / Roadmap |
|---|---|---|---|---|
| 1 | **Hybrid dense + sparse retrieval** (BM25 + vector, established) | Combine lexical and semantic signals | ✅ | §4 — 6-signal hybrid scoring |
| 2 | **Reciprocal Rank Fusion (RRF)** (Cormack 2009, ubiquitous in 2024+ stacks) | Rank-based fusion `Σ 1/(k + rank_i)` across multiple retrievers, robust to score-scale mismatch | ✅ | `src-tauri/src/memory/fusion.rs` (utility + tests). Wired into `hybrid_search_rrf` (`memory/store.rs`) — fuses vector + keyword + freshness rankings with `k = 60`; exposed as `hybrid_search_memories_rrf` Tauri command. |
| 3 | **Contextual Retrieval** ([Anthropic, Sep 2024](https://www.anthropic.com/news/contextual-retrieval)) | LLM prepends a 50–100 token chunk-specific context to each chunk *before* embedding, reduces failed retrievals by ~49 % | ✅ | `src-tauri/src/memory/contextualize.rs` — `contextualise_chunk(doc_summary, chunk, brain_mode)` + `generate_doc_summary()`. Opt-in via `AppSettings.contextual_retrieval`. Integrated into `run_ingest_task`. Chunk 16.2. |
| 4 | **HyDE — Hypothetical Document Embeddings** (Gao et al., 2022; mainstream 2024) | LLM generates a hypothetical answer; we embed *that* and search, much better recall on cold/abstract queries | ✅ | `src-tauri/src/memory/hyde.rs` (prompt + reply cleaner, 10 unit tests) + `OllamaAgent::hyde_complete` + `hyde_search_memories` Tauri command. Falls back to embedding the raw query if the brain is unreachable. |
| 5 | **Self-RAG** (Asai et al., 2023) | LLM emits reflection tokens (`Retrieve` / `Relevant` / `Supported` / `Useful`), iteratively decides when to retrieve and self-grades output | � | `src-tauri/src/orchestrator/self_rag.rs` ships the **pure controller** — reflection-token parser + 3-iteration decision SM (Chunk 16.4a). Orchestrator-loop integration that re-prompts the LLM until `Decision::Accept` is the follow-up Chunk 16.4b. |
| 6 | **Corrective RAG (CRAG)** (Yan et al., 2024) | Lightweight retrieval evaluator classifies hits as Correct / Ambiguous / Incorrect, triggers web search or rewrite on the latter two | � | `src-tauri/src/memory/crag.rs` ships the **pure evaluator** — `build_evaluator_prompts` + `parse_verdict` + `aggregate` over CORRECT/AMBIGUOUS/INCORRECT (Chunk 16.5a). Query-rewrite + web-search fallback is the follow-up Chunk 16.5b. |
| 7 | **GraphRAG** ([Microsoft, 2024](https://github.com/microsoft/graphrag)) | LLM extracts entities + relations into a KG, runs Leiden community detection, summarizes each community; queries hit community summaries first | 🟡 → 🔵 | Foundations: `memory_edges` V5 + `multi_hop_search_memories` (§6). Missing: community detection + LLM community-summary rollups. Phase 6. |
| 8 | **LightRAG** (HKU, 2024) | GraphRAG variant: dual-level retrieval (low-level entity + high-level theme) with incremental graph updates; cheaper than full GraphRAG | 🔵 | Phase 6 — natural follow-on once community summaries land |
| 9 | **Late Chunking** ([Jina AI, Sep 2024](https://jina.ai/news/late-chunking-in-long-context-embedding-models/)) | Embed the *whole* document with a long-context embedding model first, then mean-pool per-chunk token windows — preserves cross-chunk context | ✅ | `memory::late_chunking` now includes pooling plus chunk↔token alignment (`CharSpan`, `token_spans_for_char_spans`), and `run_ingest_task` uses it behind `AppSettings.late_chunking`. `OllamaAgent::embed_tokens` calls `/api/embed` with `truncate=false` and accepts token-vector response shapes with offsets or token text. If the local model returns standard pooled embeddings, ingestion falls back to the existing per-chunk embedding path. Chunk 16.3b. |
| 10 | **Cross-encoder reranking** (BGE-reranker-v2-m3, Cohere Rerank 3, etc.) | Second-pass scorer over top-k candidates with a query-doc joint encoder, much higher precision than bi-encoder cosine | ✅ | `src-tauri/src/memory/reranker.rs` (prompt + score parser + reorder logic, 14 unit tests) + `OllamaAgent::rerank_score` + `rerank_search_memories` Tauri command. Uses **LLM-as-judge** with the active brain (no extra model download). Unscored candidates are kept rather than dropped, preserving recall when the brain is flaky. Interface (`(query, document) -> Option<u8>`) is identical to a future BGE/mxbai backend so swapping is a one-line change. |
| 11 | **Matryoshka Representation Learning** (Kusupati et al., 2022; widely adopted 2024) | One embedding model, truncatable to 256 / 512 / 768 dim with graceful quality degradation — cheap fast first pass + full-dim re-rank | ✅ | `src-tauri/src/memory/matryoshka.rs` — `truncate_and_normalize` + `two_stage_search`, plus `matryoshka_search_memories` Tauri command. Default fast-dim 256 for `nomic-embed-text`. Chunk 16.8. |
| 12 | **Letta (formerly MemGPT) sleep-time memory** ([Letta, 2024](https://www.letta.com/blog/sleep-time-compute)) | Background "sleep" job during idle compresses, links, and consolidates short → working → long; writable structured memory blocks | ✅ | `src-tauri/src/memory/consolidation.rs` — `run_sleep_time_consolidation` job runs during idle, links short→working→long, surfaces stats via `ConsolidationResult`. Chunk 16.7. |
| 13 | **Zep / Graphiti temporal KG** ([getzep/graphiti, 2024](https://github.com/getzep/graphiti)) | Knowledge graph where every edge has `valid_from` / `valid_to` timestamps; supports point-in-time queries and contradicting-fact resolution | ✅ | The canonical schema includes two nullable Unix-ms columns on `memory_edges` (`valid_from` inclusive, `valid_to` exclusive — open ends mean "always"). `MemoryEdge::is_valid_at(t)` is the pure interval predicate; `MemoryStore::get_edges_for_at(memory, dir, valid_at)` is the point-in-time query (when `valid_at = None` it preserves the legacy "return all edges" behaviour for full back-compat). `MemoryStore::close_edge(id, t)` records supersession; pairing it with `add_edge { valid_from: Some(t) }` expresses "fact X changed value at time t" as two non-destructive rows. The `add_memory_edge` Tauri command gained `valid_from` / `valid_to` parameters; the `close_memory_edge` command exposes supersession to the frontend. Edge unit tests plus canonical schema tests cover the shape. |
| 14 | **Agentic RAG** (industry term, 2024–2026) | RAG embedded in an agent loop: plan → retrieve → reflect → re-retrieve → generate, with tool use | 🟡 | Foundations: roster + workflow engine (Chunk 1.5, `agents/roster.rs`). Phase 6: explicit retrieve-as-tool wiring. |
| 15 | **Context Engineering** (discipline, 2025) | Systematic management of *what* enters the context window: history, tool descriptions, retrieved chunks, structured instructions — beyond prompt engineering | 🟡 | Persona + `[LONG-TERM MEMORY]` block + animation tags is a starting point (§4 RAG injection flow). Phase 6: explicit context budgeter. |
| 16 | **Long-context vs RAG ("just stuff 1M tokens")** | Use 200K–2M token windows instead of retrieval | ⚪ | Rejected for personal companion: cost-prohibitive on local hardware, attention blind spots, privacy. RAG remains primary; long-context is a per-call tactical choice. |
| 17 | **ColBERT / late-interaction retrieval** (Khattab & Zaharia, 2020; ColBERTv2, 2022) | Token-level multi-vector retrieval with MaxSim, very high recall but storage-heavy | ⚪ | Rejected for desktop: ~10× embedding storage. Cross-encoder reranker (item 10) gives most of the quality at far lower cost. |
| 18 | **External vector DB (Qdrant, Weaviate, Milvus, pgvector)** | Dedicated vector database service | ⚪ | Rejected by design: TerranSoul ships as a single Tauri binary (§13 "Why TerranSoul Doesn't Use an External RAG Framework"). SQLite + optional ANN index (Phase 4) keeps the offline-first promise. |

### 19.3 Implementation already shipped from this survey

**Reciprocal Rank Fusion utility** — `src-tauri/src/memory/fusion.rs` ships `reciprocal_rank_fuse(rankings, k)`, a pure stable function that takes any number of ranked candidate lists (e.g. vector-rank, keyword-rank, graph-rank) and returns a fused ranking by `Σ 1/(k + rank_i)` with `k = 60` per the original Cormack et al. paper. It is intentionally decoupled from `MemoryStore` so Phase 6 work (cross-encoder reranking, multi-retriever fusion, GraphRAG community vs entity-level fusion) can plug into it without further refactoring. Unit tests cover: stable ordering, missing-from-some-rankings handling, single-list passthrough, and tie behaviour.

**RRF wired into hybrid_search** — `MemoryStore::hybrid_search_rrf(query, query_embedding, limit)` builds three independent rankings (vector cosine similarity over embeddings, keyword hit-count over content + tags, freshness composite of recency + importance + decay + tier) and fuses them with `reciprocal_rank_fuse` (`k = 60`). It is exposed as the Tauri command `hybrid_search_memories_rrf` alongside the legacy weighted-sum `hybrid_search_memories`. RRF is preferred when the underlying retrievers have incomparable score scales — the case for raw cosine, hit ratios and freshness composites — because it removes hand-tuned weight magic. The `StorageBackend` trait gains a default `hybrid_search_rrf` that delegates to `hybrid_search`, so non-default backends (Postgres / MSSQL / Cassandra) continue to compile and may opt into RRF natively later.

**HyDE — Hypothetical Document Embeddings** — `src-tauri/src/memory/hyde.rs` ships `build_hyde_prompt(query) -> (system, user)` and `clean_hyde_reply(reply) -> Option<String>`; both are pure functions with full unit-test coverage of preamble stripping, whitespace collapsing, and too-short-input rejection. `OllamaAgent::hyde_complete(query)` orchestrates the LLM call, returning the cleaned hypothetical or `None` if the brain is unreachable. The Tauri command `hyde_search_memories(query, limit)` chains HyDE → embed → `hybrid_search_rrf`, with a three-stage fallback: if HyDE expansion fails, embed the raw query; if embedding fails, fall back to keyword + freshness ranking. This makes HyDE a drop-in upgrade over `hybrid_search_memories_rrf` for cold-query retrieval without changing caller code.

**Cross-encoder reranker (LLM-as-judge style)** — `src-tauri/src/memory/reranker.rs` ships three pure functions: `build_rerank_prompt(query, doc) -> (system, user)` (with rubric + 1500-char document clipping), `parse_rerank_score(reply) -> Option<u8>` (robust to chat noise like `"**7**"`, `"Score: 7"`, `"7 out of 10"`), and `rerank_candidates(candidates, scores, limit) -> Vec<MemoryEntry>` (sorts by score descending, breaks ties by original bi-encoder rank, **keeps unscored candidates below scored ones rather than dropping them**). `OllamaAgent::rerank_score(query, doc) -> Option<u8>` is the brain wrapper. The Tauri command `rerank_search_memories(query, limit, candidates_k)` runs a two-stage pipeline: stage 1 calls `hybrid_search_rrf` with `candidates_k` (default 20, clamped `limit..=50`) for recall, stage 2 reranks down to `limit` for precision. Choosing LLM-as-judge over a dedicated BGE/mxbai reranker model avoids a second model download and works in all three brain modes (Free / Paid / Local Ollama); the `(query, document) -> Option<u8>` interface is identical to a future dedicated-reranker backend so swapping is a one-line change.

**Temporal knowledge graph (V6 schema)** — V6 migration adds two nullable Unix-ms columns to `memory_edges`: `valid_from` (inclusive lower bound, `NULL` ≡ "always has been valid") and `valid_to` (exclusive upper bound, `NULL` ≡ "still valid"). The right-exclusive convention makes supersession unambiguous: closing edge A at `t` and inserting edge B with `valid_from = Some(t)` produces exactly one valid edge for every timestamp. `MemoryEdge::is_valid_at(t)` is the pure interval predicate (10 unit tests across the 4 open/closed combinations + boundary inclusivity); `MemoryStore::get_edges_for_at(memory_id, direction, valid_at: Option<i64>)` is the point-in-time query — when `valid_at = None` it preserves the legacy "return every edge" behaviour, so every existing caller stays correct without modification. `MemoryStore::close_edge(id, t)` records supersession (idempotent, returns SQL row count). The `add_memory_edge` Tauri command gained `valid_from` / `valid_to` parameters with serde defaults of `None`; the new `close_memory_edge` command exposes supersession to the frontend. **Why the in-memory filter rather than a SQL `WHERE` clause?** The temporal predicate involves two `IS NULL OR …` branches per row, which costs more in query-planner complexity than it saves in I/O at the size of a personal memory graph; the new `idx_edges_valid_to` index leaves the door open to push the predicate into SQL once any user's graph grows past the working-set boundary.

### 19.4 How to use this section

1. **Before designing brain work**, scan the `Status` column for 🔵 items relevant to your goal — they already have a Phase 6 roadmap slot.
2. **When picking up a 🔵 item**, file a Chunk in `rules/milestones.md` referencing both its row in §19.2 and its Phase 6 entry in §16.
3. **When a new 2026+ technique emerges**, append a row to §19.2 with a `[citation](url)` and assign a status symbol — never silently absorb new work without updating this map.

### 19.5 Sources

- Anthropic — *Contextual Retrieval* (Sep 2024)
- Asai et al. — *Self-RAG: Learning to Retrieve, Generate, and Critique through Self-Reflection* (NeurIPS 2023)
- Cormack, Clarke, Büttcher — *Reciprocal Rank Fusion outperforms Condorcet and individual Rank Learning Methods* (SIGIR 2009)
- Edge et al. (Microsoft) — *From Local to Global: A GraphRAG Approach to Query-Focused Summarization* (2024)
- Gao et al. — *Precise Zero-Shot Dense Retrieval without Relevance Labels* (HyDE, 2022)
- Guo et al. — *LightRAG: Simple and Fast Retrieval-Augmented Generation* (HKU, 2024)
- Jina AI — *Late Chunking in Long-Context Embedding Models* (Sep 2024)
- Khattab & Zaharia — *ColBERT* (SIGIR 2020) and *ColBERTv2* (NAACL 2022)
- Kusupati et al. — *Matryoshka Representation Learning* (NeurIPS 2022)
- Letta — *Sleep-Time Compute for AI Agents* (2024)
- Packer et al. — *MemGPT: Towards LLMs as Operating Systems* (2023)
- Yan et al. — *Corrective Retrieval Augmented Generation (CRAG)* (2024)
- Zep / `getzep/graphiti` — Temporal Knowledge Graph for Agent Memory (2024)
- Industry surveys: Redis "Agentic RAG" (2025), Eden AI "2025 Guide to RAG", Microsoft Research GraphRAG releases through 2026Q1.

---

## 20. Brain Component Selection & Routing — How the LLM Knows What to Use

> **Why this section exists**: TerranSoul's brain is composed of **many independent components** (4 provider modes × N free providers, 2 embedding models, 3 memory tiers, 3 search methods, 4 storage backends, 17 edge relation types, agent roster, durable workflow engine, …). A frequent contributor question is *"how does the LLM know which one to pick for a given turn?"* The honest answer is: **most routing is deterministic and happens in Rust, not inside the LLM.** The LLM is invoked only at a few precise decision points. This section is the canonical decision matrix.

### 20.1 Selection topology — who decides what?

```
┌────────────────────────────────────────────────────────────────────────────┐
│                    BRAIN COMPONENT SELECTION TOPOLOGY                      │
│                                                                            │
│   USER (Setup wizard, Brain hub UI, chat command)                          │
│      │  sets:     active brain mode, paid model, Ollama model,             │
│      │            storage backend, RAG injection toggle                    │
│      ▼                                                                     │
│   PERSISTED CONFIG  (active_brain.txt · brain_mode.json · settings)        │
│      │                                                                     │
│      ▼                                                                     │
│   RUST DETERMINISTIC ROUTER  (no LLM in this layer)                       │
│   ├── streaming::stream_chat       → match BrainMode { Free/Paid/Local }   │
│   ├── phone_control::stream_chat   → same prompt assembly over gRPC-Web    │
│   ├── ProviderRotator              → fastest healthy free provider         │
│   ├── OllamaAgent::resolve_embed_model → nomic-embed-text → chat fallback │
│   ├── MemoryStore::hybrid_search   → score every memory, top-k            │
│   ├── StorageBackend trait         → SQLite | Postgres | MSSQL | Cassandra│
│   ├── cognitive_kind::classify     → episodic | semantic | procedural     │
│   ├── AgentOrchestrator::dispatch  → agent_id="auto" → default ("stub")   │
│   └── PermissionStore              → cross-device command gating          │
│      │                                                                     │
│      ▼                                                                     │
│   LLM-DRIVEN DECISION POINTS  (the few places the LLM actually chooses)   │
│   ├── semantic_search_entries      → LLM ranks memory relevance           │
│   ├── extract_facts / summarize    → LLM picks what is worth remembering   │
│   ├── extract_edges_via_brain      → LLM picks relation type from 17-list │
│   └── Free-text intent in chat     → "switch to groq" → conversation.ts   │
│      │                                                                     │
│      ▼                                                                     │
│   CHAT TURN  (provider answers user with retrieved context)                │
└────────────────────────────────────────────────────────────────────────────┘
```

**Design principle**: keep routing in Rust whenever it can be expressed as a pure function of state — the LLM is expensive, non-deterministic, and harder to test. The LLM is reserved for *content* decisions (what is relevant, what is a fact, what relation connects two facts), not for *plumbing* decisions (which provider, which model, which tier).

### 20.2 Decision matrix

Each row below is one selection point. The "Decided by" column tells you **whether the LLM, the user, or pure code makes the call**, and where the logic lives.

| # | Selection point | Decided by | Algorithm / signal | Source of truth | Fallback chain |
|---|---|---|---|---|---|
| 1 | **Brain mode** (Free / Paid / Local / Stub) | User (setup wizard, mode switcher, chat intent) | Persisted `BrainMode` enum | `brain/brain_config.rs::load_brain_mode` | `BrainMode::default()` → Free API (Groq) → Pollinations |
| 2 | **Free provider** (Groq, Pollinations, …) | Pure code | `ProviderRotator::next_healthy_provider` — fastest healthy non-rate-limited | `brain/provider_rotator.rs:161` | Configured provider → next in `sorted_ids` → Stub |
| 3 | **Paid model & endpoint** | User (paid setup) | Persisted in `BrainMode::PaidApi { model, base_url }` | `brain/brain_config.rs` | None — paid mode requires explicit config |
| 4 | **Local Ollama chat model** | User (model picker, hardware-adaptive recommender) | `model_recommender::recommend_for_ram(ram_mb)` | `brain/model_recommender.rs` | Default `gemma3:4b` |
| 5 | **Local Ollama embedding model** | Pure code with cache | `OllamaAgent::resolve_embed_model` — try `nomic-embed-text`, else fall back to active chat model; cache result for 60s; mark unsupported permanently | `brain/ollama_agent.rs` | `nomic-embed-text` → chat model → skip vector signal entirely |
| 6 | **Memory tier to write into** | User explicit + auto-promotion | `MemoryStore::add_memory(tier=Working/Long)`, `promote()` triggered by importance ≥ 4 | `memory/store.rs` | New entries default to Working |
| 7 | **Memory tier to search** | Pure code | `hybrid_search` scans **all tiers**, applies `tier_priority` weight (working 1.0 → long 0.5 → short 0.3) | `memory/store.rs:574` | All tiers always considered |
| 8 | **Search method** (`search` / `semantic_search` / `hybrid_search` / `multi_hop`) | Caller (frontend / streaming command / phone-control stream) | Frontend calls `hybrid_search_memories` for explicit search; chat streams call hybrid retrieval for RAG injection | `commands/memory.rs` + `commands/streaming.rs` + `ai_integrations/grpc/phone_control.rs` | `hybrid_search` → degrades to keyword if embedding fails |
| 9 | **Top-k for RAG injection** | Pure code + user threshold | Top 5 after hybrid scoring, filtered by `AppSettings.relevance_threshold` | `commands/streaming.rs` + `ai_integrations/grpc/phone_control.rs` | Empty block when nothing clears threshold |
| 10 | **Memory relevance ranking (LLM mode)** | **LLM** | `semantic_search_entries` sends all entries to LLM with a ranking prompt | `memory/brain_memory.rs` | Falls back to `hybrid_search` if Ollama unreachable |
| 11 | **Fact extraction from chat** | **LLM** | `extract_facts` prompts LLM for ≤5 atomic facts | `memory/brain_memory.rs` | None — feature unavailable without an LLM brain |
| 12 | **Cognitive kind** (episodic / semantic / procedural) | Pure code | `cognitive_kind::classify(memory_type, tags, content)` — tag prefix `episodic:* / semantic:* / procedural:*` overrides; otherwise tag → type → content order, verb/hint heuristics | `memory/cognitive_kind.rs` | Defaults to `Semantic` |
| 13 | **Knowledge-graph edge relation type** | **LLM** + normaliser | `extract_edges_via_brain` prompts LLM with the 17-type taxonomy; `edges::normalise_rel_type` snaps free-form types to canonical | `memory/edges.rs` | Free-form edges allowed (preserved as-is) |
| 14 | **Storage backend** | User (compile-time + config) | Cargo features `postgres` / `mssql` / `cassandra`; runtime `StorageConfig` selects which `StorageBackend` impl is bound | `memory/backend.rs` + `lib.rs` startup | SQLite (always available, default) |
| 15 | **Agent dispatch** | Caller / orchestrator | `AgentOrchestrator::dispatch(agent_id, msg)`; `agent_id="auto"` → `default_agent_id` ("stub") | `orchestrator/agent_orchestrator.rs:34` | Stub agent when no others registered |
| 16 | **Cross-device command permission** | Permission store + user prompt | `PermissionStore::check(origin_device)` → Allowed / Denied / Ask | `routing/router.rs:36` + `routing/permission.rs` | First-time → Ask |
| 17 | **Streaming timeout** | Pure code (constant) | 60s overall stream timeout, 30s fallback timeout | `commands/streaming.rs` | Emit completion sentinel and surface error |

### 20.3 Worked example — what happens on one chat turn

> User types: "What did the lawyer say about Cook County filings?"

1. **Provider selection (rule 1, 2)** — Desktop frontend calls the `send_message_stream` Tauri command; iOS calls `PhoneControl.StreamChatMessage` through `RemoteHost`. Backend reads `state.brain_mode`. If `FreeApi`, the `ProviderRotator` (rule 2) picks the fastest healthy provider; if `LocalOllama`, the user-configured model is used (rule 4).
2. **History assembly (rule 7)** — Last 20 messages from `state.conversation` (short-term memory) are loaded into the prompt verbatim — no LLM decision, just a FIFO slice.
3. **Embedding model selection (rule 5)** — Backend calls `OllamaAgent::embed_text(query)`. The cached resolver picks `nomic-embed-text` if installed; otherwise the chat model; otherwise returns `None` and the vector signal is skipped (degrades to 60% RAG quality, see §17 FAQ).
4. **Hybrid search (rule 7, 8, 9)** — `MemoryStore::hybrid_search(query, embedding, limit=5)` scans **every tier** of every stored memory, scoring each with the 6-signal formula (§4). The cognitive-kind classifier (rule 12) is **not** invoked at search time — it is computed at write time and stored derived.
5. **Top-5 injection (rule 9)** — Top 5 entries are formatted into the `[LONG-TERM MEMORY]` block. There is currently **no relevance threshold** — even a weakly-matching memory at rank 5 is injected. This is a documented Phase 4 gap (§16); the user can preview what would be injected via the Brain hub "Active Selection" panel (§20.5).
6. **Provider call (rule 1)** — The chosen provider streams tokens back via `llm-chunk` events; `<anim>` blocks are split off into `llm-animation` (per repo memory `streaming architecture`).
7. **Post-turn (rules 10, 11)** — The chat turn is *not* automatically extracted as facts. Extraction runs only when the user clicks "Extract from session" or when the session ends, at which point `extract_facts` (rule 11) and optionally `summarize` are called, producing new Working-tier memories that may later be promoted (rule 6).
8. **Edge extraction (rule 13)** — Optional, user-triggered. `extract_edges_via_brain` asks the LLM to propose typed edges between newly-added memories, using the 17-type taxonomy. Free-form types are accepted; `normalise_rel_type` snaps near-matches.

### 20.4 Failure / degradation contract

When a component is unavailable, the router **degrades silently** rather than erroring — every selection point has a documented fallback (rightmost column of §20.2). The user-visible effect is captured by the `effective_quality` percentage on the Brain hub:

| Failure mode | Effect | Effective RAG quality | UI signal |
|---|---|---|---|
| No brain configured | Persona-based stub responses, no RAG | 0 % | Brain hub shows ⚠ "Not configured" |
| Free API rate-limited | Rotator skips to next provider | 100 % (cloud quality) | Provider badge shows live status |
| Ollama down (Local mode) | Vector signal skipped, keyword + temporal only | 60 % | RAG capability strip greys out the Vector cell |
| Embedding model missing | Cached "unsupported" → no further calls | 60 % | RAG capability strip greys out the Vector cell |
| Hybrid search returns nothing | Empty `[LONG-TERM MEMORY]` block injected | n/a | "No relevant memories" subtitle |
| Cross-device command from new origin | Held in `pending_envelopes` | n/a | Toast prompts user to allow / block / ask-once |

### 20.5 What the user sees — Brain hub "Active Selection" panel

The Brain hub view (`src/views/BrainView.vue`) renders an **Active Selection** panel that mirrors §20.2 in plain English, fed by the typed `BrainSelection` snapshot returned by the new `get_brain_selection` Tauri command:

```
┌─ Active brain selection ───────────────────────────────────────┐
│  Provider     :  Free API → Groq (auto-rotated, healthy)       │
│  Chat model   :  llama-3.3-70b-versatile                       │
│  Embedding    :  ✗ unavailable (cloud mode — vector RAG off)   │
│  Memory       :  3 tiers active · 1,247 long · 18 working      │
│  Search       :  Hybrid 6-signal · top-5 injection · no threshold │
│  Storage      :  SQLite (WAL) · schema V6                      │
│  Agents       :  1 registered (stub) · default = "auto" → stub │
│  RAG quality  :  60 % (cloud APIs cannot compute embeddings)   │
└────────────────────────────────────────────────────────────────┘
                                                  [Configure ▸]
```

This panel is the operational answer to "how does the LLM know what to choose?" — the user can read each selection at a glance, click `Configure` to override any of them, and immediately see the effect on the RAG capability strip (§20.4 row 3 vs row 1).

### 20.6 Adding new components — required steps

Whenever a contributor adds a new brain component (new provider, new embedding model, new search method, new storage backend, new agent, new edge-extraction strategy), they **must**:

1. Add a row to the §20.2 decision matrix specifying who decides, what algorithm, source-of-truth file, and fallback.
2. Extend the `BrainSelection` snapshot struct (`src-tauri/src/brain/selection.rs`) so the Active Selection panel can report it.
3. Add a fallback row to §20.4 if the new component can fail or be unavailable.
4. Update the Brain hub UI panel (`src/views/BrainView.vue` "Active selection" section) to render the new field.
5. Update `README.md` per architecture rule 11 (Brain Documentation Sync).

This keeps the "how does the LLM choose what to use?" question answerable in one place forever.

---

## 21. How Daily Conversation Updates the Brain — Write-Back / Learning Loop

> **Why this section exists**: §20 explains how the brain *reads* memory on every chat turn. This section is the matching answer for the *write* side — *"how does daily conversation update the brain?"*. The honest summary is: every chat turn lands instantly in **short-term** memory, but promotion into **long-term** memory only happens when the auto-learner fires (or when the user clicks an explicit button). This section documents the full loop, the cadence policy, and the gaps.

### 21.1 The five-step write-back loop

```
┌────────────────────────────────────────────────────────────────────────────┐
│                CONVERSATION → BRAIN  WRITE-BACK LOOP                       │
│                                                                            │
│   Step 1  ── Live append                                                   │
│   Every chat turn pushes user + assistant messages into                    │
│     state.conversation : Mutex<Vec<Message>>   (short-term, in-memory)     │
│     ▼                                                                      │
│   Step 2  ── History window for next turn                                  │
│   The next chat turn reads the **last 20** messages from short-term        │
│   into the LLM prompt (commands/streaming.rs ~line 224).                   │
│     ▼                                                                      │
│   Step 3  ── Auto-learn evaluator (NEW — §21.2)                            │
│   After each assistant turn the frontend asks                              │
│     evaluate_auto_learn(total_turns, last_autolearn_turn)                  │
│   which returns Fire | SkipDisabled | SkipBelowThreshold | SkipCooldown.   │
│     ▼  (only on Fire)                                                      │
│   Step 4  ── Extraction & persistence                                      │
│   extract_memories_from_session  (commands/memory.rs:134)                  │
│     → brain_memory::extract_facts (LLM picks ≤5 atomic facts)              │
│     → brain_memory::save_facts    (writes Working-tier rows to SQLite)     │
│   Each new row gets:                                                       │
│     · cognitive_kind = classify(type, tags, content)   (pure fn, §3.5)     │
│     · embedding      = nomic-embed-text(content)        (if local Ollama)  │
│     · created_at, decay = 1.0                                              │
│     ▼                                                                      │
│   Step 5  ── Background maintenance                                        │
│   On its own cadence (currently user-triggered, daily-job target):         │
│     · apply_memory_decay  — exponential decay multiplier on all rows        │
│     · gc_memories         — drop rows below GC threshold                   │
│     · promote_memory      — Working → Long when importance ≥ 4             │
│     · extract_edges_via_brain — LLM proposes typed edges between new rows  │
└────────────────────────────────────────────────────────────────────────────┘
```

### 21.2 Auto-learn cadence policy (`memory::auto_learn`)

Steps 1–2 are unconditional and free. Step 4 calls an LLM (cost + latency), so it is gated by a **pure-function policy** evaluated after every assistant turn. The policy lives in `src-tauri/src/memory/auto_learn.rs` and is exposed via three Tauri commands:

| Command | Purpose |
|---|---|
| `get_auto_learn_policy` | Read the user-configured cadence (toggle + every-N-turns + cooldown) |
| `set_auto_learn_policy` | Persist a new cadence (Brain hub "Daily learning" card) |
| `evaluate_auto_learn(total_turns, last_autolearn_turn)` | Pure decision query — "should I fire right now?" |

**Default cadence**: enabled, fire every 10 turns, minimum cooldown 3 turns. With these defaults a typical 30-turn session yields three auto-extractions (~15 new memories), with no LLM cost during quiet periods.

**Decision values** (mirrored to the frontend as `AutoLearnDecisionDto`):

| Decision | UI signal |
|---|---|
| `Fire` | Toast: *"Brain is learning from this conversation…"*, then refresh memory list |
| `SkipDisabled` | "Daily learning" toggle visibly off in Brain hub |
| `SkipBelowThreshold { turns_until_next: N }` | Progress dial in Brain hub: "Next auto-learn in N turns" |
| `SkipCooldown { turns_remaining: N }` | Same dial, "Cooling down (N)" |

The policy is intentionally not negotiable by the LLM — it is **user-configurable state** so privacy-conscious users can disable automatic learning entirely (Step 4 still runs on demand from the Memory tab "Extract from session" button).

### 21.3 What gets written, and why

| Memory tier | Triggered by | Purpose | Lifetime |
|---|---|---|---|
| **Short-term** (in-memory `Vec<Message>`) | Every chat turn (Step 1) | LLM prompt history | Lost on app restart |
| **Working** (SQLite, `tier='working'`) | Auto-learn fire (Step 4) **or** explicit "Extract from session" / "Summarize" | Recently-learned facts pending consolidation | Survives restart; subject to decay & GC |
| **Long** (SQLite, `tier='long'`) | `promote_memory` (Step 5) when importance ≥ 4 | Durable knowledge, biased highest in hybrid scoring (`tier_priority` = 1.0) | Permanent until user deletes |

Notes:

- **Decay (Step 5)** is the slow forgetting curve; `apply_memory_decay` multiplies each row's `decay` field by an exponential factor based on age. This signal contributes 10 % to the hybrid RAG score (§4) — old memories quietly recede unless re-accessed.
- **GC (Step 5)** drops rows whose `decay × importance` falls below a threshold, so the database doesn't grow without bound.
- **Edge extraction (Step 5)** is *not* automatic today. It is a Phase 4 / 5 gap — the design doc carries it under §16 "Scaling Roadmap".

### 21.4 Failure / cost contract

| Failure mode | Effect on the loop |
|---|---|
| No brain configured | Steps 1–2 still work (history is just a prompt slice). Step 4 errors with `"No brain configured"`; the auto-learner silently skips. |
| LLM call fails during extraction | `extract_facts` returns `Vec::new()`; `save_facts` saves nothing; UI surfaces "extraction returned 0 facts" rather than aborting the chat. |
| Embedding model missing | New Working-tier rows still write — but with `embedding = NULL`. They participate in keyword + recency + importance + decay + tier scoring (60 % RAG quality, §17 FAQ); a future `backfill_embeddings` call adds vectors when an embedding model becomes available. |
| User disables auto-learn mid-session | Next `evaluate_auto_learn` returns `SkipDisabled`; existing memories are untouched; no chat impact. |
| User reduces `every_n_turns` mid-session | Cooldown clause prevents immediate re-fire (≥3 turns must elapse since the last run). |

### 21.5 Manual overrides (always available)

Even with auto-learn on, the following commands are always available from the Memory tab and via Tauri:

- `extract_memories_from_session` — force Step 4 now
- `summarize_session` — collapse the whole session into one summary memory
- `add_memory` / `update_memory` / `delete_memory` — direct CRUD
- `apply_memory_decay` — force decay tick
- `gc_memories` — force garbage collection
- `extract_edges_via_brain` — propose typed edges via LLM
- `backfill_embeddings` — compute missing vectors

This guarantees the user is never locked out of any maintenance step the auto-learner would have done.

### 21.6 Persona drift detection (Chunk 14.8) ✅

After every auto-learn extraction, the frontend accumulates a running count of saved facts. When the count crosses a configurable threshold (default **25 facts**), the `check_persona_drift` Tauri command fires — comparing the active `PersonaTraits` JSON against the latest `personal:*` long-tier memories via a lightweight LLM prompt. The result is a `DriftReport`:

| Field | Type | Description |
|---|---|---|
| `drift_detected` | `bool` | Whether a meaningful shift was found |
| `summary` | `String` | 1–2 sentence description of the shift (empty if none) |
| `suggested_changes` | `Vec<DriftSuggestion>` | 0–3 field/current/proposed triples |

**Design decisions:**
- **Piggybacks on auto-learn** — no new background loop or scheduler; fires only when the user is actively chatting and facts are accumulating.
- **Pure prompt + parse** — `persona::drift::build_drift_prompt` and `parse_drift_reply` are I/O-free and exhaustively unit-tested (14 tests).
- **Non-blocking** — drift check failure never breaks chat. If the brain can't parse a reply, a "no drift" report is returned.
- **Fact-count-based** — uses accumulated facts (not turns) as the trigger, so quiet sessions with few extractable facts don't waste LLM calls.

### 21.7 Roadmap gaps (already tracked in §16)

- **Background scheduler** — Step 5 maintenance jobs are currently user-triggered. A daily background scheduler (`tasks::manager::TaskManager`) is on the Phase 4 roadmap.
- **Conversation-aware extraction** — today extraction sees the whole session as one blob. The Phase 5 roadmap adds *segmented* extraction (e.g. one extraction per topic shift detected by embedding-distance peak).
- **Edge auto-extraction** — `extract_edges_via_brain` is manual; auto-firing it after each successful `extract_facts` is the next iteration of this loop.
- **Replay-from-history rebuild** — re-run extraction over old chat logs to backfill memories created before auto-learn existed (planned for Phase 5 alongside the export/import work).

### 21.8 Adding a new write path — required steps

When a contributor adds a new way for conversation to update the brain (new extractor, new edge proposer, new background job), they **must**:

1. Decide whether it belongs in Step 4 (per-turn, LLM-cost) or Step 5 (background, batched). Cost-sensitive paths must go in Step 5.
2. If it triggers per-turn, gate it behind the same `AutoLearnPolicy` (or a sibling policy with its own `enabled` flag) — never run an LLM after every turn unconditionally.
3. Update §21.1 diagram, §21.3 table, and §21.4 failure contract above.
4. Surface a "what just happened?" signal in the Brain hub UI so the user can see the brain learn in real time.
5. Update `README.md` per architecture rule 11 (Brain Documentation Sync).

This keeps the write-back side as understandable as the read side (§20).

---

## 22. Code-Intelligence Bridge — GitNexus Sidecar (Phase 13 Tier 1)

> **Implemented in Chunk 2.1 (2026-04-24).** Tier 1 of the four-tier
> GitNexus integration plan. See `rules/completion-log.md` for the
> per-file change manifest.

The brain reads structured **code** intelligence (symbol locations, call
graphs, blast-radius, change diffs) through a strict out-of-process
bridge to the upstream **GitNexus** project (`abhigyanpatwari/GitNexus`,
PolyForm-Noncommercial-1.0.0). The licence forbids bundling, so
TerranSoul **never ships GitNexus binaries**. Users install GitNexus
themselves under their own licence terms (most commonly
`npm i -g gitnexus`) and TerranSoul spawns `npx gitnexus mcp` over stdio
when the user grants the `code_intelligence` capability to the
`gitnexus-sidecar` agent.

### 22.1 Wire diagram

```
Frontend (BrainView · Code knowledge panel — `src/components/CodeKnowledgePanel.vue`, shipped 2026-04-24)
   │
   │  invoke('gitnexusQuery', { prompt })
   ▼
src-tauri/src/commands/gitnexus.rs  ← capability gate (CapabilityStore)
   │
   ▼
src-tauri/src/agent/gitnexus_sidecar.rs
   │
   │  JSON-RPC 2.0 over stdio (line-delimited JSON)
   ▼
$ npx gitnexus mcp           ← user-installed, out-of-process, kill_on_drop
   │
   ▼
GitNexus MCP server (TypeScript) — analyses the active repo
```

### 22.2 Tools exposed (Tier 1)

| Tauri command | MCP tool | Arguments | Returns |
|---|---|---|---|
| `gitnexus_query` | `query` | `query: string` | Free-form code-intelligence answer |
| `gitnexus_context` | `context` | `target: string`, `maxResults: u32 = 10` | Ranked code snippets relevant to a symbol/file |
| `gitnexus_impact` | `impact` | `symbol: string` | Blast-radius (callers / dependents) of changing a symbol |
| `gitnexus_detect_changes` | `detect_changes` | `from: string`, `to: string` | Diff-aware summary between two git refs |

The bridge returns the JSON-RPC `result` payload as `serde_json::Value`
verbatim — TerranSoul does not reshape GitNexus's response schema, so
upstream changes do not require a TerranSoul release.

### 22.3 Capability model

The bridge uses **two layers** of consent:

1. **Process spawn** — handled by the OS / Tauri sidecar config; the user
   chose to install GitNexus and configure the sidecar command.
2. **`code_intelligence` capability** — granted per-agent via the
   existing `CapabilityStore` consent dialog. Every Tauri command in
   `commands/gitnexus.rs` re-reads the consent on every call, so revoking
   consent immediately blocks subsequent tool invocations even if the
   sidecar is still running.

The bridge never has filesystem or network capabilities of its own — all
filesystem/network actions are GitNexus's responsibility, performed in
its own subprocess address space.

### 22.4 Reliability guarantees

- **Lazy initialisation.** The MCP `initialize` handshake (and the
  spec-mandated `notifications/initialized` follow-up) runs only on the
  first tool call, then is cached for the bridge's lifetime.
- **ID matching.** Every JSON-RPC request carries a strictly-increasing
  numeric `id`. The reader loop skips notifications and stale responses
  with non-matching ids.
- **Bounded skip.** The reader will skip at most `MAX_SKIPPED_LINES`
  (256) unrelated lines before returning `NoMatchingResponse`. This
  defends Tauri commands against runaway / chatty sidecars.
- **EOF / pipe closed.** Returns `GitNexusError::Io` so the frontend can
  show a clean error and offer to respawn the sidecar.
- **Reaping.** `tokio::process::Command::kill_on_drop(true)` ensures the
  child process is reaped when the bridge handle is dropped — including
  on `configure_gitnexus_sidecar`, which intentionally drops the cached
  bridge to force a respawn under the new config.

### 22.5 Roadmap (later tiers)

| Tier | Chunk | Status | Goal |
|---|---|---|---|
| 1 | 2.1 | ✅ done (2026-04-24) | Sidecar bridge + four read-only Tauri commands behind `code_intelligence` capability |
| 2 | 2.2 | ✅ done (2026-04-24) | Fuse `gitnexus_query` results into `rerank_search_memories` recall stage via existing `memory::fusion::reciprocal_rank_fuse` |
| 3 | 2.3 | ✅ done (2026-04-24) | The canonical SQLite schema includes the `edge_source` column on `memory_edges` (+ index). New `memory::gitnexus_mirror` module maps `CONTAINS`/`CALLS`/`IMPORTS`/`EXTENDS`/`HANDLES_ROUTE` into the existing 17-relation taxonomy and writes mirrored edges with `edge_source = 'gitnexus:<scope>'`. Tauri commands `gitnexus_sync` (opt-in; calls the sidecar's `graph` MCP tool) and `gitnexus_unmirror` (single-scope rollback). 11 unit tests + 4 extractor tests. |
| 4 | 2.4 | ✅ done (2026-04-24) | New `src/components/CodeKnowledgePanel.vue` (sync form + mirror list with last-sync time + edge counts + per-row Unmirror + blast-radius `gitnexus_impact` probe) wired into `BrainView.vue`. New Tauri command `gitnexus_list_mirrors` (powered by `MemoryStore::list_external_mirrors("gitnexus:%")`) returns one row per mirrored scope ordered by most-recent-sync first. 9 Vitest unit tests + 3 new Rust unit tests. |

---

## 23. Code-RAG Fusion in `rerank_search_memories` (Phase 13 Tier 2)

> **Implemented in Chunk 2.2 (2026-04-24).** Tier 2 of the four-tier
> GitNexus integration. Builds directly on §22's sidecar bridge and §19.2
> rows 2 (RRF) and 10 (cross-encoder reranker).

When a user invokes `rerank_search_memories` and **both** of the
following are true:

1. The `gitnexus-sidecar` agent has been granted the
   `code_intelligence` capability via `CapabilityStore`.
2. `AppState.gitnexus_sidecar` holds a live bridge handle (i.e. at least
   one prior tool call has lazily spawned the child process, or the user
   explicitly configured it via `configure_gitnexus_sidecar`).

…then the recall stage now **augments** its SQLite candidate set with
GitNexus snippets before handing off to the LLM-as-judge reranker:

```
Stage 1   — RRF recall over SQLite (vector ⊕ keyword ⊕ freshness)
Stage 1.5 — NEW: dispatch user query → GitNexus `query` tool
            → normalise JSON response → pseudo-MemoryEntries
            → RRF-fuse with Stage-1 candidates (k=60, DEFAULT_RRF_K)
            → truncate to candidates_k
Stage 2   — LLM-as-judge rerank (unchanged) → final top-N
```

### 23.1 Pseudo-`MemoryEntry` shape

GitNexus snippets are wrapped in `MemoryEntry` records that the existing
fusion + rerank code can consume without modification, but with two
discriminators that downstream code can rely on:

| Field             | Value                              | Why                                                           |
|-------------------|------------------------------------|---------------------------------------------------------------|
| `id`              | strictly **negative** (`-1, -2, …`) | Cannot collide with SQLite's positive `INTEGER PRIMARY KEY` |
| `tier`            | `MemoryTier::Working`              | Ephemeral, not persisted                                      |
| `memory_type`     | `MemoryType::Context`              | Transient retrieval context, not a personal fact              |
| `tags`            | `code:gitnexus[,code:<path>]`      | Greppable provenance                                          |
| `embedding`       | `None`                             | We never embed code snippets locally                          |
| `decay_score`     | `1.0`                              | Always fresh                                                  |

The pure helper `memory::code_rag::is_code_rag_entry(&entry)` is the
canonical check for "this entry came from GitNexus, do not write it
back to disk".

### 23.2 Response-shape tolerance

The normaliser `gitnexus_response_to_entries` accepts every published
GitNexus response shape (and a few defensive variants):

```text
{ "snippets": [ { "content": "...", "path": "..." }, ... ] }
{ "answer":   "...", "sources": [ { "content": "...", "path": "..." } ] }
{ "results":  [ { "content": "...", "path": "..." } ] }
[ { "content": "...", "path": "..." }, ... ]   // top-level array
{ "answer": "single sentence" }                // synthesised answer only
"plain string answer"                          // top-level scalar
```

Field aliases handled: `content` / `text` / `snippet` / `body` / `code`
for the body; `path` / `file` / `location` / `uri` / `source` for the
source link. Anything else is silently dropped.

A defensive cap (`MAX_CODE_RAG_ENTRIES = 16`) prevents a runaway
response from flooding the rerank stage and blowing up LLM token usage.

### 23.3 Failure modes — all degrade to DB-only recall

The bridge call is wrapped so that **none** of the following ever
fail the search; each silently returns the original SQLite candidate
set after an `eprintln!` warning:

| Failure                                | Behaviour                  |
|----------------------------------------|----------------------------|
| Capability not granted                 | Skip Stage 1.5 entirely    |
| Sidecar handle absent                  | Skip Stage 1.5 entirely    |
| Sidecar process crashed / pipe closed  | Warn + return DB results   |
| GitNexus returned RPC error            | Warn + return DB results   |
| GitNexus returned unrecognised shape   | Skip merge (no error)      |
| Empty snippets list                    | Skip merge (no error)      |

This mirrors the existing rerank fallback contract (§19.2 row 10): the
system must always serve **some** result, even if every advanced
component is unreachable.

### 23.4 What this does NOT do (scope guard)

- Does **not** mutate the SQLite store. Code-RAG entries are ephemeral.
- Does **not** persist GitNexus snippets — Tier 3 (Chunk 2.3, shipped
  2026-04-24) is the opt-in path that mirrors the GitNexus knowledge
  graph into the memory-graph V7 schema with an `edge_source` column.
- Does **not** rerank GitNexus snippets via the LLM-as-judge
  *separately* — they enter Stage 2 through the same `rerank_score`
  call as DB entries, so the rerank stage's existing `Option<u8>`
  "unscored kept below scored" contract applies uniformly.
- Does **not** affect any other RAG command (`hybrid_search_memories`,
  `hybrid_search_memories_rrf`, `hyde_search_memories`) — fusion lives
  inside the `rerank_search_memories` Tauri command only, so users
  who don't want code-RAG can opt out simply by calling a different
  command.

---

## 24. MCP Server — External AI Coding Assistant Integration (Phase 15)

> **Shipped as Chunk 15.1 (2026-04-25).** See `rules/completion-log.md`
> and `docs/AI-coding-integrations.md` for full details.

TerranSoul exposes its brain to **external AI coding assistants**
(Copilot, Cursor, Windsurf, Continue, etc.) via the
[Model Context Protocol (MCP)](https://modelcontextprotocol.io/).
The server runs as an in-process axum HTTP service on
`127.0.0.1:7421` — no sidecar, no external binary.

### 24.1 Architecture

```
External AI assistant
      │  HTTP POST (JSON-RPC 2.0, Bearer token)
      ▼
┌───────────────────────────────────────────┐
│  MCP Server (axum, 127.0.0.1:7421)       │
│  src-tauri/src/ai_integrations/mcp/      │
│  ├── mod.rs      — start/stop, McpServerHandle
│  ├── auth.rs     — SHA-256 bearer token (mcp-token.txt)
│  ├── router.rs   — JSON-RPC dispatch + auth middleware
│  └── tools.rs    — 8 tool definitions + dispatch
└──────────────────┬────────────────────────┘
                   │ dyn BrainGateway (8 ops)
                   ▼
┌───────────────────────────────────────────┐
│  BrainGateway trait                       │
│  src-tauri/src/ai_integrations/gateway.rs │
│  AppStateGateway adapter (holds AppState) │
└───────────────────────────────────────────┘
```

**`AppState(Arc<AppStateInner>)`** — The cheaply-clonable Arc newtype
lets the MCP server hold a reference without lifetime issues. Background
axum task receives `AppState` directly.

### 24.2 Exposed MCP tools

> Source of truth: `src-tauri/src/ai_integrations/mcp/tools.rs`. The
> 8-tool surface below mirrors that file's `tool_definitions()` /
> `dispatch_tool()` exactly.

| Tool | BrainGateway op | Description |
|---|---|---|
| `brain_search` | `search()` | Hybrid semantic + keyword memory search |
| `brain_get_entry` | `get_entry()` | Fetch a single memory by id |
| `brain_list_recent` | `list_recent()` | List the most recently created/touched memories |
| `brain_kg_neighbors` | `kg_neighbors()` | Knowledge-graph neighbours of a memory (typed edges) |
| `brain_summarize` | `summarize()` | LLM summary of a passage |
| `brain_suggest_context` | `suggest_context()` | Suggest relevant memories for an editor cursor / file |
| `brain_ingest_url` | `ingest_url()` | Crawl + chunk + embed a URL into the memory store |
| `brain_health` | `health()` | Provider status + model info |

### 24.3 Security

- **Bearer-token auth** — SHA-256 hash of a UUID v4 stored in
  `$APP_DATA/mcp-token.txt` with `0600` permissions on Unix.
- **Localhost-only** — binds to `127.0.0.1`, never `0.0.0.0`.
- **Regeneratable** — `mcp_regenerate_token` Tauri command.

### 24.4 Tauri commands

- `mcp_server_start` / `mcp_server_stop` / `mcp_server_status` / `mcp_regenerate_token`
- Lifecycle managed through `AppStateInner.mcp_server: TokioMutex<Option<McpServerHandle>>`

### 24.5 Test coverage

22 Rust tests: 4 auth, 6 router, 3 tools, 11 integration (ephemeral ports via `portpicker`).

### 24.6 gRPC-Web and RemoteHost (Phase 24)

The same brain surface now has a browser-safe transport for the mobile companion. The tonic gRPC server uses `tonic_web::GrpcWebLayer` with HTTP/1 enabled, so a WebView can call `terransoul.brain.v1.Brain` without an Envoy proxy while native gRPC clients still use the same service definitions.

Frontend callers do not talk directly to `invoke()` or Connect clients. They depend on `src/transport/remote-host.ts`, a shared `RemoteHost` contract with two implementations:

- **Local desktop:** `createLocalRemoteHost()` adapts existing Tauri commands (`hybrid_search_memories_rrf`, `send_message`, workflow status, paired devices) into the same DTOs.
- **Paired mobile/WebView:** `createGrpcWebRemoteHost()` uses `@bufbuild/connect` + `@bufbuild/connect-web` against hand-written protobuf-es descriptors in `src/transport/brain_pb.ts` and `src/transport/phone_control_pb.ts`.

The adapter currently exposes `Brain.Health`, unary `Brain.Search`, and server-streaming `Brain.StreamSearch`, plus the Phase 24 phone-control RPCs for system status, VS Code/Copilot session status, workflow progress/continue, chat, and paired devices. Search modes remain the backend modes (`rrf`, `hybrid`, `hyde`); the phone only selects a mode and streams results, while retrieval, ranking, persona context, and memory injection stay server-side.

Chunk 24.9 extends this same boundary to live chat. `PhoneControl.StreamChatMessage(ChatRequest) returns (stream ChatChunk)` runs on the desktop host and assembles the full system prompt there: `SYSTEM_PROMPT_FOR_STREAMING`, hybrid memory lookup above the relevance threshold, `[LONG-TERM MEMORY]`, persona block, and one-shot `[HANDOFF]` block. The stream reuses the Rust `StreamTagParser`, so `<anim>` / `<pose>` blocks are stripped before mobile receives text. The unary `SendChatMessage` remains as a fallback, but iOS chat uses `RemoteHost.streamChatMessage()` from `src/stores/remote-conversation.ts`.

Frontend routing is deliberately boring: `src/stores/chat-store-router.ts` selects the existing local `conversation.ts` store on desktop and `remote-conversation.ts` when `src/utils/runtime-target.ts` detects iOS (or an explicit `remoteConversation` test override). `ChatView.vue` binds to the shared store surface, so message lists, agent thread filtering, queue/stop controls, subtitles, and mobile breakpoints remain the same while the backing stream moves from in-process Tauri IPC to the paired desktop `RemoteHost`.

Chunk 24.10 adds the phone-control tool layer above `RemoteHost`. `src/transport/remote-tools.ts` defines MCP-style tool names — `describe_copilot_session`, `describe_workflow_progress`, and `continue_workflow` — with JSON-schema-shaped inputs and capability metadata. The tools call existing Phase 24 RPCs through `RemoteHost`: Copilot session probing, workflow run listing, workflow progress lookup, and workflow continue/heartbeat. `remote-conversation.ts` routes headline phone prompts such as “what's Copilot doing on my desktop?” and “continue the next chunk” through these tools before falling back to normal hosted chat streaming, so the phone remains a microphone/screen while the desktop host owns the reasoning, state, and side effects.

Chunk 24.11 adds local paired-mobile notifications for long-running desktop work without APNS or a cloud push relay. `src/stores/mobile-notifications.ts` starts only for the iOS/remote runtime, polls the paired desktop through `RemoteHost.listWorkflowRuns(true)` and `RemoteHost.getCopilotSessionStatus()`, observes local `task-progress` events when available, and sends native notifications through `tauri-plugin-notification` after the configured threshold (`AppSettings.mobile_notification_threshold_ms`, default 30 s). Workflow and task notifications fire only after a previously observed run reaches a terminal state; Copilot sessions notify once when active work crosses the threshold. The mobile shell stores enablement and poll timing in `AppSettings`, while the desktop host still owns workflow, Copilot, chat, RAG, and memory state.

---

## 25. Intent Classification

> Source: `src-tauri/src/brain/intent_classifier.rs` · Tauri command `classify_intent` · Frontend dispatch in `src/stores/conversation.ts` (`classifyIntent` + `sendMessage` switch)

Every chat turn is routed through a structured **LLM-powered intent
classifier** before being handed to the streaming chat path. This
replaces three brittle regex detectors that used to live in
`conversation.ts` (`detectLearnWithDocsIntent`, `detectTeachIntent`,
`detectGatedSetupCommand`) and that only matched exact English
phrasings — they're now kept as deprecated test fixtures only.

### 25.1 Why an LLM, not regex?

Regex routing is fundamentally English-only and brittle to typos,
paraphrases, compound requests, and other languages. A user who types
`học luật Việt Nam từ tài liệu của tôi`,
`can you study these PDFs and tell me about contract law`, or
`upgrde to gemni model` should reach the same destination as a user
who types the canonical English phrase. The brain already understands
all of these — so we let the brain decide.

This also matches the project's "brain decides everything" posture
(§ 10 Brain Modes, § 20 Brain Component Selection).

### 25.2 JSON schema

The classifier asks the LLM to reply with **exactly one** of:

```json
{ "kind": "chat" }
{ "kind": "learn_with_docs", "topic": "<short topic phrase>" }
{ "kind": "teach_ingest",   "topic": "<short topic phrase>" }
{ "kind": "gated_setup",    "setup": "upgrade_gemini" | "provide_context" }
```

Any malformed reply, unknown `kind`, or unknown `setup` value is
mapped to `IntentDecision::Unknown` and the frontend triggers the
install-all fallback. The system prompt is ~150 tokens so the call is
cheap on free providers (Pollinations / Groq).

### 25.3 Provider rotation & timeouts

The classifier reuses the same `ProviderRotator` the chat path uses
(`src-tauri/src/brain/provider_rotator.rs`):

1. **Free** provider first — cheapest, default.
2. **Paid** provider when configured.
3. **Local Ollama** when installed and reachable.
4. **Local LM Studio** when configured.

A hard **3-second timeout** (`CLASSIFY_TIMEOUT`) bounds the call so a
slow free provider can never block the user's chat turn. Any timeout,
network failure, HTTP non-2xx, or schema-violating reply maps to
`IntentDecision::Unknown`.

### 25.4 Caching

Identical *trimmed lowercase* inputs hit a process-global LRU
(`CACHE_MAX_ENTRIES = 256`, `CACHE_TTL = 30 s`). This stops the user
double-classifying when they retry, and avoids re-asking the LLM if
the conversation store is recreated mid-session. The cache is cleared
automatically by `set_brain_mode` because a different model may
classify differently.

### 25.5 The "unknown ⇒ trigger local install" guarantee

The frontend `sendMessage` switch maps `IntentDecision::Unknown` to
`startLearnDocsFlow(content)` — the same install-all overlay the user
gets when they type the canonical English phrase. Pressing "Auto
install all" runs the existing prereq chain
(`ollama-installed → free-llm → rag-knowledge → scholar-quest`),
which installs a local Ollama brain. From that turn onward the
classifier always has a working local provider, so it works offline
forever.

This means the worst case (no network, no local LLM, free LLM down)
is **the same UX the user gets today** when they type the magic
phrase — not a silent failure.

### 25.6 Test surface

- `intent_classifier.rs` — 20 Rust unit tests covering every
  `IntentDecision` variant, malformed JSON, unknown kinds, prose /
  markdown-fence tolerance, nested-brace JSON extraction (including
  unbalanced-brace guard), cache round-trip + TTL eviction + capacity
  eviction, empty input, and no-brain-mode short-circuit.
- `conversation.test.ts` — every sendMessage flow that used to rely
  on a regex detector now mocks `invoke('classify_intent', …)` to
  return the intended decision; one new integration test verifies
  that returning `{kind:'unknown'}` enters the install-all flow with
  the original user input as the topic.
- The three legacy `detect*Intent` helpers retain their unit tests
  as deterministic golden cases; they are no longer called from the
  live message path.

### 25.7 User controls — Brain panel "🧭 AI decision-making"

Every "LLM decides" surface exposed by TerranSoul is opt-out from a
single panel in `src/views/BrainView.vue`. Settings live in the
frontend-only Pinia store `src/stores/ai-decision-policy.ts` and
persist via `localStorage` under the key
`terransoul.ai-decision-policy.v1`. No DB schema change, no Tauri
round-trip — toggling a setting takes effect on the very next message.

| Setting | Default | What it controls |
|---|---|---|
| `intentClassifierEnabled` | on | Run every chat turn through `classify_intent`. Off = skip the IPC entirely; every message goes straight to streaming. |
| `unknownFallbackToInstall` | on | When the classifier returns `Unknown`, open the Auto-Install-All overlay. Off = silently fall through to streaming. |
| `dontKnowGateEnabled` | on | Watch assistant replies for hedging language ("I don't know", …) and push the Gemini-search / context-upload gate. Off = no follow-up prompt. |
| `questSuggestionsEnabled` | on | Auto-open quest overlay when a reply or the user's input mentions getting-started keywords. Off = quests only launch from the Skill Tree. |
| `chatBasedLlmSwitchEnabled` | on | Recognise commands like `switch to groq` / `use my openai api key sk-…` and reconfigure the brain in-place. Off = those messages reach the LLM unchanged. |
| `quickRepliesEnabled` | on | Show one-tap "Yes / No" buttons under the latest reply when its trailing sentence matches a yes/no question pattern (`shall we…?`, `would you like…?`, etc.). Off = always type your full reply. |
| `capacityDetectionEnabled` | on | Watch free-API replies for incapability phrasings (`I can't / cannot / am only an AI / beyond my capabilities`); after a few low-quality replies in a sliding window, pop the in-chat upgrade dialog. Off = no auto-prompts. |

All gates in `conversation.ts` early-return when their corresponding
field is `false`, with a `try { … } catch` wrapper so legacy unit
tests that don't set up Pinia retain default-on behaviour.

`src/stores/ai-decision-policy.test.ts` covers defaults, persistence,
rehydration, corrupt-JSON recovery, sanitisation of non-boolean
values, and `reset()`. `conversation.test.ts` adds three integration
tests asserting that each toggle actually short-circuits its gate.

---

## Related Documents

- [AI-coding-integrations.md](../docs/AI-coding-integrations.md) — Full MCP / gRPC / A2A integration design
- [BRAIN-COMPLEX-EXAMPLE.md](../instructions/BRAIN-COMPLEX-EXAMPLE.md) — Quest-guided setup walkthrough (Free API, with screenshots)
- [BRAIN-COMPLEX-EXAMPLE-LOCAL-LM.md](../instructions/BRAIN-COMPLEX-EXAMPLE-LOCAL-LM.md) — Local LM Studio variant walkthrough (with screenshots)
- [BRAIN-COMPLEX-EXAMPLE-EXPLAIN.md](../instructions/BRAIN-COMPLEX-EXAMPLE-EXPLAIN.md) — Technical reference (schema, RAG pipeline, comparisons)
- [architecture-rules.md](../rules/architecture-rules.md) — Project architecture constraints
- [coding-standards.md](../rules/coding-standards.md) — Code style and library policy
- [backlog.md](../rules/backlog.md) — Feature backlog with memory items
