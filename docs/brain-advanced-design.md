# Brain & Memory — Advanced Architecture Design

> **TerranSoul v0.1** — Self-learning AI companion with persistent memory  
> Last updated: 2026-04-22  
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

**Implementation path**: Add a `category` column in a V5 migration, or use structured tag prefixes (`personal:name`, `rel:friend:sarah`, `domain:law:family`) to avoid schema changes. Tag prefixes are recommended first — they work with the existing search infrastructure and don't require a migration.

---

## 3.5 Cognitive Memory Axes (Episodic / Semantic / Procedural)

> **Status:** classifier landed at `src-tauri/src/memory/cognitive_kind.rs`
> alongside ontology tag prefixes; **no V6 schema migration**.

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

### Embedding & Vector Search

```
┌──────────────────────────────────────────────────────────────────┐
│                    EMBEDDING ARCHITECTURE                         │
│                                                                   │
│  Model:     nomic-embed-text (768-dimensional)                   │
│  Provider:  Ollama (localhost:11434/api/embed)                   │
│  Fallback:  Active chat model (lower quality but works)          │
│  Storage:   BLOB column in SQLite (768 × 4 bytes = 3 KB each)   │
│                                                                   │
│  Memory budget:                                                   │
│    1,000 memories   ×  3 KB  =    3 MB                           │
│   10,000 memories   ×  3 KB  =   30 MB                           │
│  100,000 memories   ×  3 KB  =  300 MB                           │
│  1,000,000 memories ×  3 KB  = 3,000 MB (needs ANN index)       │
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
│  • If `nomic-embed-text` is missing AND the active chat model    │
│    returns 501/400 from /api/embed (Llama, Phi, Gemma, …), the   │
│    model is added to a process-lifetime "unsupported" cache and  │
│    no further embed calls are made for it. Vector RAG silently   │
│    degrades to keyword + LLM-ranking. No log spam, no chat       │
│    pipeline stalls.                                              │
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

**Shipped `memory_edges` table (V5 migration):**

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

### Current Schema (V4)

```sql
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
    session_id    TEXT,                          -- Links working memories to session
    parent_id     INTEGER,                       -- Summary → source memory link
    token_count   INTEGER,                       -- Content size in tokens
    decay_score   REAL    NOT NULL DEFAULT 1.0   -- 0.01–1.0 freshness
);

CREATE INDEX idx_memories_importance ON memories(importance DESC);
CREATE INDEX idx_memories_created    ON memories(created_at DESC);
CREATE INDEX idx_memories_tier       ON memories(tier);
CREATE INDEX idx_memories_session    ON memories(session_id);
CREATE INDEX idx_memories_decay      ON memories(decay_score);
CREATE INDEX idx_memories_source     ON memories(source_hash);

CREATE TABLE schema_version (
    version     INTEGER PRIMARY KEY,
    applied_at  INTEGER NOT NULL,
    description TEXT    NOT NULL DEFAULT ''
);
```

### Proposed Schema Changes

**V6 — Category column (proposed):**
```sql
ALTER TABLE memories ADD COLUMN category TEXT NOT NULL DEFAULT 'general';
CREATE INDEX idx_memories_category ON memories(category);
```

### Shipped V5 — Entity-relationship edges

```sql
CREATE TABLE memory_edges (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    src_id     INTEGER NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    dst_id     INTEGER NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    rel_type   TEXT    NOT NULL,
    confidence REAL    NOT NULL DEFAULT 1.0,
    source     TEXT    NOT NULL DEFAULT 'user',  -- user | llm | auto
    created_at INTEGER NOT NULL,
    UNIQUE(src_id, dst_id, rel_type)
);

CREATE INDEX idx_edges_src  ON memory_edges(src_id);
CREATE INDEX idx_edges_dst  ON memory_edges(dst_id);
CREATE INDEX idx_edges_type ON memory_edges(rel_type);
-- PRAGMA foreign_keys=ON enforced at connection open.
```

**V7 — Obsidian sync metadata (proposed):**
```sql
ALTER TABLE memories ADD COLUMN obsidian_path TEXT;      -- vault-relative .md path
ALTER TABLE memories ADD COLUMN last_exported INTEGER;   -- Unix timestamp
```

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
│  │ Vector (40%) │    ✗     │    ✗     │ ✓ (nomic-embed-text)│     │
│  │ Keyword(20%) │    ✓     │    ✓     │ ✓                    │     │
│  │ Recency(15%) │    ✓     │    ✓     │ ✓                    │     │
│  │ Import.(10%) │    ✓     │    ✓     │ ✓                    │     │
│  │ Decay  (10%) │    ✓     │    ✓     │ ✓                    │     │
│  │ Tier    (5%) │    ✓     │    ✓     │ ✓                    │     │
│  ├──────────────┼──────────┼──────────┼──────────────────────┤     │
│  │ Effective    │ 60%      │ 60%      │ 100%                 │     │
│  │ RAG quality  │ (no vec) │ (no vec) │ (full hybrid)        │     │
│  └──────────────┴──────────┴──────────┴──────────────────────┘     │
│                                                                     │
│  Model selection:                                                   │
│  • model_recommender.rs — RAM-based catalogue                      │
│  • Auto-selects best model for available hardware                   │
│  • Catalogue includes: Gemma 4, Phi-4, Qwen 3, Kimi K2.6 (cloud) │
│  • ProviderRotator — cycles through free providers on failure      │
└─────────────────────────────────────────────────────────────────────┘
```

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
| **Maintenance** | Version compatibility issues | Self-contained, auto-migrating |

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
│  └── schema_version (4 rows)                             │
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

.tables        -- → memories  schema_version
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

-- Migration history
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
│  PHASE 2 — Categories & Graph                                       │
│  ├── ○ Add category column (V5 migration)                          │
│  ├── ○ Auto-categorize via LLM on insert                           │
│  ├── ○ Category-aware decay rates                                  │
│  ├── ○ Category filters in Memory View                             │
│  ├── ○ Tag prefix convention (personal:*, domain:*, etc.)          │
│  └── ○ Obsidian vault export (one-way)                             │
│                                                                     │
│  PHASE 3 — Entity Graph (✅ Shipped — V5 schema)                    │
│  ├── ✓ memory_edges table (V5 migration, FK cascade)               │
│  ├── ✓ LLM-powered edge extraction (extract_edges_via_brain)       │
│  ├── ✓ Relationship type taxonomy (17 curated types + free-form)   │
│  ├── ✓ Multi-hop RAG via graph traversal (hybrid_search_with_graph)│
│  ├── ✓ Graph-enhanced Cytoscape visualization (typed/directional)  │
│  └── ○ Conflict detection between connected memories               │
│                                                                     │
│  PHASE 4 — Scale                                                    │
│  ├── ○ ANN index (usearch crate) for >1M memories                 │
│  ├── ○ Cloud embedding API for free/paid modes                     │
│  ├── ○ Chunking pipeline for large documents                       │
│  ├── ○ Relevance threshold (skip injection if score < 0.3)        │
│  ├── ○ Bidirectional Obsidian sync                                 │
│  └── ○ Memory versioning (track edits, not just overwrites)        │
│                                                                     │
│  PHASE 5 — Intelligence                                             │
│  ├── ○ Auto-promotion based on access patterns                     │
│  ├── ○ Contradiction resolution (LLM picks winner)                 │
│  ├── ○ Temporal reasoning ("last month you said...")               │
│  ├── ○ Memory importance auto-adjustment from access_count         │
│  └── ○ Cross-device memory merge via CRDT sync                    │
│                                                                     │
│  PHASE 6 — Modern RAG (April 2026 research absorption — see §19)   │
│  ├── ✓ Reciprocal Rank Fusion utility (memory/fusion.rs)           │
│  ├── ○ Contextual Retrieval (Anthropic 2024) — LLM-prepended chunk │
│  │     context before embedding                                    │
│  ├── ○ HyDE (Hypothetical Document Embeddings) for cold queries   │
│  ├── ○ Cross-encoder reranking pass (BGE-reranker-v2-m3 via Ollama)│
│  ├── ○ Late chunking (embed full doc, pool per-chunk windows)      │
│  ├── ○ GraphRAG / LightRAG-style community summaries over          │
│  │     memory_edges (multi-hop + LLM cluster summary)              │
│  ├── ○ Self-RAG / Corrective RAG (CRAG) iterative refinement loop  │
│  ├── ○ Sleep-time consolidation (Letta-style background job that   │
│  │     compresses/links short→working→long during idle)            │
│  ├── ○ Temporal knowledge graph (Zep / Graphiti-style valid_from / │
│  │     valid_to edges on memory_edges)                             │
│  └── ○ Matryoshka embeddings (variable-dim 256/512/768 truncation) │
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
// { schema_version: 4, total_memories: 15247, embedded_count: 15200, ... }
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
| 2 | **Reciprocal Rank Fusion (RRF)** (Cormack 2009, ubiquitous in 2024+ stacks) | Rank-based fusion `Σ 1/(k + rank_i)` across multiple retrievers, robust to score-scale mismatch | ✅ | `src-tauri/src/memory/fusion.rs` (utility + tests). Wire-in to `hybrid_search` is Phase 6. |
| 3 | **Contextual Retrieval** ([Anthropic, Sep 2024](https://www.anthropic.com/news/contextual-retrieval)) | LLM prepends a 50–100 token chunk-specific context to each chunk *before* embedding, reduces failed retrievals by ~49 % | 🔵 | Phase 6 — chunking pipeline (§16) |
| 4 | **HyDE — Hypothetical Document Embeddings** (Gao et al., 2022; mainstream 2024) | LLM generates a hypothetical answer; we embed *that* and search, much better recall on cold/abstract queries | 🔵 | Phase 6 — Brain has `chat_completion` + `embed_text` already, ~50 LOC to add `hyde_search_memories` command |
| 5 | **Self-RAG** (Asai et al., 2023) | LLM emits reflection tokens (`Retrieve` / `Relevant` / `Supported` / `Useful`), iteratively decides when to retrieve and self-grades output | 🔵 | Phase 6 — orchestrator-level loop (`src-tauri/src/orchestrator/`) |
| 6 | **Corrective RAG (CRAG)** (Yan et al., 2024) | Lightweight retrieval evaluator classifies hits as Correct / Ambiguous / Incorrect, triggers web search or rewrite on the latter two | 🔵 | Phase 6 — pairs naturally with our `relevance_threshold` Phase 4 item |
| 7 | **GraphRAG** ([Microsoft, 2024](https://github.com/microsoft/graphrag)) | LLM extracts entities + relations into a KG, runs Leiden community detection, summarizes each community; queries hit community summaries first | 🟡 → 🔵 | Foundations: `memory_edges` V5 + `multi_hop_search_memories` (§6). Missing: community detection + LLM community-summary rollups. Phase 6. |
| 8 | **LightRAG** (HKU, 2024) | GraphRAG variant: dual-level retrieval (low-level entity + high-level theme) with incremental graph updates; cheaper than full GraphRAG | 🔵 | Phase 6 — natural follow-on once community summaries land |
| 9 | **Late Chunking** ([Jina AI, Sep 2024](https://jina.ai/news/late-chunking-in-long-context-embedding-models/)) | Embed the *whole* document with a long-context embedding model first, then mean-pool per-chunk token windows — preserves cross-chunk context | 🔵 | Phase 6 — requires long-context embedding model (e.g. `jina-embeddings-v3`) selectable via Ollama |
| 10 | **Cross-encoder reranking** (BGE-reranker-v2-m3, Cohere Rerank 3, etc.) | Second-pass scorer over top-k candidates with a query-doc joint encoder, much higher precision than bi-encoder cosine | 🔵 | Phase 6 — slot a reranker between `hybrid_search` and prompt formatting; RRF utility (item 2) is the fusion primitive |
| 11 | **Matryoshka Representation Learning** (Kusupati et al., 2022; widely adopted 2024) | One embedding model, truncatable to 256 / 512 / 768 dim with graceful quality degradation — cheap fast first pass + full-dim re-rank | 🔵 | Phase 6 — pairs with ANN index (Phase 4); current `nomic-embed-text` is fixed-dim |
| 12 | **Letta (formerly MemGPT) sleep-time memory** ([Letta, 2024](https://www.letta.com/blog/sleep-time-compute)) | Background "sleep" job during idle compresses, links, and consolidates short → working → long; writable structured memory blocks | 🔵 | Phase 6 — fits TerranSoul's tier model (§2); reuses durable workflow engine (`workflows/engine.rs`) for the idle-time job |
| 13 | **Zep / Graphiti temporal KG** ([getzep/graphiti, 2024](https://github.com/getzep/graphiti)) | Knowledge graph where every edge has `valid_from` / `valid_to` timestamps; supports point-in-time queries and contradicting-fact resolution | 🔵 | Phase 6 — additive columns on `memory_edges`; complements Phase 5 "Temporal reasoning" |
| 14 | **Agentic RAG** (industry term, 2024–2026) | RAG embedded in an agent loop: plan → retrieve → reflect → re-retrieve → generate, with tool use | 🟡 | Foundations: roster + workflow engine (Chunk 1.5, `agents/roster.rs`). Phase 6: explicit retrieve-as-tool wiring. |
| 15 | **Context Engineering** (discipline, 2025) | Systematic management of *what* enters the context window: history, tool descriptions, retrieved chunks, structured instructions — beyond prompt engineering | 🟡 | Persona + `[LONG-TERM MEMORY]` block + animation tags is a starting point (§4 RAG injection flow). Phase 6: explicit context budgeter. |
| 16 | **Long-context vs RAG ("just stuff 1M tokens")** | Use 200K–2M token windows instead of retrieval | ⚪ | Rejected for personal companion: cost-prohibitive on local hardware, attention blind spots, privacy. RAG remains primary; long-context is a per-call tactical choice. |
| 17 | **ColBERT / late-interaction retrieval** (Khattab & Zaharia, 2020; ColBERTv2, 2022) | Token-level multi-vector retrieval with MaxSim, very high recall but storage-heavy | ⚪ | Rejected for desktop: ~10× embedding storage. Cross-encoder reranker (item 10) gives most of the quality at far lower cost. |
| 18 | **External vector DB (Qdrant, Weaviate, Milvus, pgvector)** | Dedicated vector database service | ⚪ | Rejected by design: TerranSoul ships as a single Tauri binary (§13 "Why TerranSoul Doesn't Use an External RAG Framework"). SQLite + optional ANN index (Phase 4) keeps the offline-first promise. |

### 19.3 Implementation already shipped from this survey

**Reciprocal Rank Fusion utility** — `src-tauri/src/memory/fusion.rs` ships `reciprocal_rank_fuse(rankings, k)`, a pure stable function that takes any number of ranked candidate lists (e.g. vector-rank, keyword-rank, graph-rank) and returns a fused ranking by `Σ 1/(k + rank_i)` with `k = 60` per the original Cormack et al. paper. It is intentionally decoupled from `MemoryStore` so Phase 6 work (cross-encoder reranking, multi-retriever fusion, GraphRAG community vs entity-level fusion) can plug into it without further refactoring. Unit tests cover: stable ordering, missing-from-some-rankings handling, single-list passthrough, and tie behaviour.

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

## Related Documents

- [BRAIN-COMPLEX-EXAMPLE.md](../instructions/BRAIN-COMPLEX-EXAMPLE.md) — Quest-guided setup walkthrough
- [BRAIN-COMPLEX-EXAMPLE-EXPLAIN.md](../instructions/BRAIN-COMPLEX-EXAMPLE-EXPLAIN.md) — Technical reference (schema, RAG pipeline, comparisons)
- [architecture-rules.md](../rules/architecture-rules.md) — Project architecture constraints
- [coding-standards.md](../rules/coding-standards.md) — Code style and library policy
- [backlog.md](../rules/backlog.md) — Feature backlog with memory items
