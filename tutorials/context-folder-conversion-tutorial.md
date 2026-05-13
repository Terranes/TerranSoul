# Context Folder Conversion Tutorial

> **Difficulty:** Beginner → Intermediate  
> **Time:** 5–10 minutes  
> **Prerequisites:** TerranSoul installed, brain configured (any mode)

## What Are Context Folders?

Context Folders let you point TerranSoul at an entire directory on your
computer and ingest every supported file as knowledge. This is the fastest
way to give your companion bulk context — project docs, code, research
papers, legal filings, or any reference material.

**⚠️ Brute-force warning:** Context-folder ingestion reads *every*
supported file in the directory. Large folders will produce many memory
entries and consume embedding resources. Start small and use the
conversion tools described below to optimise.

---

## Step 1: Add a Context Folder

1. Open **Brain View** (click the brain icon in the sidebar)
2. Scroll to the **📂 Context Folders** section
3. Paste or type an absolute directory path (e.g. `D:\Projects\MyDocs`)
4. Click **➕ Add**

The folder appears in the list with its auto-derived label (the last
path component, e.g. `MyDocs`).

### Supported File Types

`.txt` `.md` `.json` `.rs` `.ts` `.vue` `.css` `.html` `.toml` `.yaml`
`.yml` `.xml` `.csv` `.py` `.js` `.jsx` `.tsx` `.sql` `.sh` `.bat`
`.ps1` `.cfg` `.ini` `.log` `.c` `.cpp` `.h` `.hpp` `.java` `.kt`
`.go` `.rb` `.php` `.swift` `.r`

Binary files, images, and unsupported extensions are silently skipped.

---

## Step 2: Sync the Folder

Click **🔄 Sync All** to scan and ingest all enabled folders.

During sync, TerranSoul:
- Reads each supported file
- Computes a SHA-256 hash for change detection
- Splits the content into semantic chunks
- Stores each chunk as a memory entry (importance 2, tagged
  `context-folder,<label>`)

You'll see a result like:
> Synced 1 folder(s), 47 file(s) ingested.

**Re-syncing** is safe: unchanged files are skipped (hash match), changed
files are re-ingested, and deleted files' memories remain (manual cleanup
possible via the memory panel).

---

## Step 3: View Context Memory Stats

After syncing, the **🔄 Context ↔ Knowledge Conversion** panel shows:
- Total context memories and approximate token count
- Breakdown by folder label

This helps you gauge the ingestion cost before converting.

---

## Step 4: Convert to Knowledge

Raw context-folder chunks are numerous and low-importance (2). They work
for RAG retrieval but can be noisy. Converting consolidates them into
fewer, higher-quality knowledge entries.

1. In the conversion panel, click **📚 Convert to Knowledge**
2. TerranSoul groups chunks by source file, concatenates them in order,
   and creates consolidated `Fact` entries at importance 4

**Before conversion:**
```
file_a.md → [chunk_1 (imp=2), chunk_2 (imp=2), chunk_3 (imp=2)]
```

**After conversion:**
```
file_a.md → [chunk_1, chunk_2, chunk_3]  (unchanged, still available)
          + [knowledge_entry (imp=4)]    (consolidated, higher RAG score)
```

The original chunks remain as raw reference. The converted entries are
the optimised form that scores higher in retrieval.

---

## Step 5: Export Knowledge to Files

You can reverse the flow — export brain memories back to portable
Markdown files:

1. Enter an output directory path in the export input field
2. Click **💾 Export to Files**
3. Each matching memory becomes a `.md` file with YAML frontmatter
   (id, tags, importance, created_at) and body content

### Use Cases

- **Backup** — version-control your knowledge in a git repo
- **Sharing** — send knowledge files to another TerranSoul instance
- **Editing** — modify exported files, then re-add the folder as a
  context folder and sync to update the brain

---

## Recommended Workflow

```
1. Add folder    → Brain View → 📂 Context Folders → paste path → ➕ Add
2. Sync          → 🔄 Sync All → raw chunks created (importance 2)
3. Review stats  → conversion panel shows counts + tokens
4. Convert       → 📚 Convert to Knowledge → consolidated entries (importance 4)
5. Export        → 💾 Export to Files → portable Markdown backup
6. Re-sync       → periodically re-sync to catch file changes
```

---

## Tips

- **Start small.** Add a focused documentation folder first, not your
  entire home directory.
- **Convert early.** Raw chunks are noisy in RAG. Converting produces
  cleaner retrieval results.
- **Use labels.** Each folder gets a label tag. You can filter by label
  when converting or exporting.
- **SHA-256 dedup.** Re-syncing is cheap — unchanged files are skipped
  automatically.
- **Combine with wiki.** After converting, your knowledge entries
  participate in the Knowledge Wiki graph for connected exploration.

---

## Knowledge Graph Import & Export

Beyond flat context-folder ingestion, you can work directly with the
knowledge graph — importing files as structured subgraphs and exporting
graph subtrees back to files.

### Import a File to the Knowledge Graph

1. In the **🕸️ Knowledge Graph ↔ Files** panel, enter a file path
2. Click **📥 Import to KG**

This creates a structured subgraph:

```
               root (Summary node)
              ╱      │       ╲
         contains contains  contains
           ╱         │         ╲
      chunk_1 ─follows→ chunk_2 ─follows→ chunk_3
```

**Why use this instead of context folders?**
- **Graph structure**: chunks are connected by `follows` edges preserving
  document order, plus a root `contains` hub node
- **Higher importance**: chunks are importance 3 (vs context-folder's 2)
- **Navigable**: the root appears in WikiPanel spotlight; you can traverse
  the document graph from any chunk
- **Provenance**: all edges carry `edge_source` for batch management

### Export a KG Subtree to Files

1. Find the memory IDs you want to export (visible in the memory panel
   or WikiPanel spotlight)
2. Enter comma-separated root IDs and an output directory
3. Click **📤 Export KG Subtree**

This performs a BFS walk (up to 2 hops by default) from each root and
writes:
- One `.md` file per node with YAML frontmatter and a `## Graph Edges`
  section listing connections
- A `_graph.json` manifest with the complete node+edge structure

The `_graph.json` enables re-import on another device or instance.

### When to Use Which

| Scenario | Tool |
|---|---|
| Bulk folder of docs (quick & dirty) | Context Folders → Sync |
| Single important document (structured) | Import to KG |
| Consolidate raw chunks | Convert to Knowledge |
| Backup/share flat memories | Export to Files |
| Backup/share graph structure | Export KG Subtree |

---

## Troubleshooting

| Issue | Solution |
|---|---|
| Sync shows 0 files | Check the path exists and contains supported file types |
| Too many memories | Convert to knowledge to consolidate, or remove the folder and re-add a smaller subdirectory |
| Export fails | Ensure the output directory path is valid and writable |
| Changes not detected | Re-sync — SHA-256 hashing detects content changes automatically |
| KG import shows "File not found" | Use an absolute file path |
| KG export shows "root_ids must not be empty" | Enter at least one valid memory ID |

---

## Related

- [Brain RAG Setup Tutorial](brain-rag-setup-tutorial.md)
- [Advanced Memory & RAG Tutorial](advanced-memory-rag-tutorial.md)
- [Knowledge Wiki Tutorial](knowledge-wiki-tutorial.md)
- [Folder to Knowledge Graph Tutorial](folder-to-knowledge-graph-tutorial.md)
- [Brain Advanced Design — § 12.5 Context Folders](../docs/brain-advanced-design.md)
