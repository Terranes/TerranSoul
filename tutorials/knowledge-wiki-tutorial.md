# Knowledge Wiki — Graph Curation with Slash Commands

> TerranSoul's Knowledge Wiki brings graph-based memory curation to chat.
> Audit your brain's health, discover hidden connections, spotlight hub
> memories, and manage a review queue — all through slash commands and
> the Wiki Panel UI.

---

## Requirements

| Requirement | Notes |
|---|---|
| **Brain configured** | Any mode (free, paid, or local) |
| **Memories stored** | At least 10+ for meaningful results; 100+ for best graph insights |

---

## Step 1 — Open the Wiki Panel

Navigate to **Brain View** (sidebar → 🧠 Brain icon) → click the **Wiki** tab.

The panel has three tabs:
- **Audit** — Health metrics and issues
- **Spotlight** — Most-connected hub memories
- **Serendipity** — Surprising cross-community connections

---

## Step 2 — Audit Your Brain (`/ponder`)

Type `/ponder` in chat (or click the Audit tab):

The audit report shows:

| Metric | Meaning |
|--------|---------|
| **Total Memories** | All stored entries across tiers |
| **Live Edges** | Active knowledge graph connections |
| **Conflicts** | Open contradictions awaiting resolution |
| **Orphans** | Memories with zero edges (isolated, un-linked knowledge) |
| **Stale Candidates** | Memories not accessed beyond the staleness threshold |
| **Embedding Queue** | Entries waiting for vector embedding |

### What to Do With Audit Results

- **High orphan count:** Many memories aren't connected to anything. Consider using `/digest` to add related content, or manually adding tags that group them.
- **Open conflicts:** Navigate to Memory tab → Conflicts to resolve contradictions.
- **Large embedding queue:** Wait for the background worker to process, or check that your brain provider is reachable.
- **Stale memories:** These might be outdated. Review them with `/revisit`.

---

## Step 3 — Spotlight Hub Memories (`/spotlight`)

Type `/spotlight` in chat (or click the Spotlight tab):

Shows the **most-connected memories** in your knowledge graph — the "god nodes" that link to many other entries.

| Column | Meaning |
|--------|---------|
| **Memory** | Content preview |
| **Degree** | Number of incoming + outgoing edges |

### Why This Matters

- Hub memories are your brain's **backbone** — they connect clusters of knowledge.
- If a hub memory is wrong or outdated, it affects many related memories.
- Consider protecting critical hubs (set importance ≥ 4 so they're never evicted).

---

## Step 4 — Serendipity: Cross-Community Connections (`/serendipity`)

Type `/serendipity` in chat (or click the Serendipity tab):

Discovers **surprising high-confidence edges between otherwise unrelated memory clusters**.

**Example:** A memory about "Rust ownership" connecting to "team code review process" — because your procedural review memory references ownership concepts. The system surfaces these non-obvious bridges.

### Use Cases

- **Discover hidden relationships** in your knowledge base.
- **Find synthesis opportunities** — memories that could be combined into a wiki page.
- **Identify cross-domain insights** you might not have noticed.

---

## Step 5 — Review Queue (`/revisit`)

Type `/revisit` in chat:

Returns memories that are **most ready for review** using the append-and-review pattern:

- Oldest-accessed memories that have accumulated new related content since last review.
- Memories with pending edges that need confirmation.
- High-importance entries that haven't been verified recently.

### Review Workflow

1. Run `/revisit` to get candidates.
2. For each candidate:
   - **Still accurate?** → No action needed (access refreshes decay).
   - **Outdated?** → Edit the memory content in the Memory tab.
   - **Contradicted?** → The conflict system will catch it on edit.
   - **No longer relevant?** → Lower importance or let decay handle it.

---

## Step 6 — Digest New Content (`/digest`)

Type `/digest <text>` in chat to store a deduplicated source note:

```
/digest The deployment pipeline uses GitHub Actions with three stages: build, test, deploy.
```

### How `/digest` Works

1. Content is hashed (SHA-256 source dedup).
2. If the exact content already exists → skipped (no duplicate).
3. If new → stored as a long-term memory with source tracking.
4. Auto-tagged and classified by cognitive kind.

### Use Cases

- **Meeting notes:** `/digest We decided to use Postgres for the hive layer.`
- **Quick facts:** `/digest Project deadline is June 15, 2026.`
- **Research snippets:** `/digest Matryoshka embeddings allow truncating dimensions without retraining.`

> **Tip:** For large documents, use the document ingestion flow (attach files in chat) instead of `/digest`. `/digest` is for single facts and short notes.

---

## Step 7 — Planned Commands (Coming Soon)

| Command | Purpose |
|---------|---------|
| `/weave <topic>` | Synthesize a protected wiki page from related memories |
| `/trace <a> <b>` | Find the shortest path between two memories in the knowledge graph |
| `/why <id>` | Explain the provenance and rationale behind a memory |

---

## Step 8 — Confidence Rubric (How Edges Are Scored)

The knowledge graph uses a confidence rubric for edges:

| Level | Source | Confidence | Meaning |
|-------|--------|-----------|---------|
| **Extracted** | User or Auto-created | Any | Definitive — user stated or system extracted |
| **Inferred Strong** | LLM | ≥ 0.85 | High-confidence LLM inference |
| **Inferred Weak** | LLM | 0.65–0.85 | Moderate confidence — may need verification |
| **Ambiguous** | LLM | < 0.65 | Low confidence — treat as tentative |

The Wiki Panel uses these levels to flag which edges might need human verification.

---

## Troubleshooting

| Problem | Solution |
|---------|----------|
| `/ponder` returns all zeros | You need memories first. Ingest a document or have some conversations. |
| `/spotlight` shows nothing | Need enough edges (graph connections). KG builds automatically over time from ingested content. |
| `/digest` says "already exists" | The exact same content was previously stored. This is dedup working correctly. |
| Commands not recognized | Ensure you type the `/` prefix. Slash commands are case-sensitive. |

---

## Where to Go Next

- **[Folder to Knowledge Graph](folder-to-knowledge-graph-tutorial.md)** — Bulk-import an entire codebase or document folder
- **[Advanced Memory & RAG](advanced-memory-rag-tutorial.md)** — Understand the full retrieval pipeline that queries these memories
- **[Device Sync & Hive](device-sync-hive-tutorial.md)** — Share curated knowledge across devices
