# Using TerranSoul as an MCP Server

> How external LLM agents, AI coding assistants, and custom apps connect to
> TerranSoul's brain via the Model Context Protocol (MCP).

TerranSoul exposes its entire brain — semantic search, knowledge graph,
RAG pipeline, memory store, and LLM summarization — as an MCP server.
Any MCP-compatible client can query it without re-scanning the codebase.

---

## Quick Reference

| Mode | Port | Token location | When to use |
|---|---|---|---|
| **Release** (desktop app) | `7421` | `%APPDATA%/com.terranes.terransoul/mcp-token.txt` | Production use — running TerranSoul app |
| **Dev** (`cargo tauri dev`) | `7422` | `%APPDATA%/com.terranes.terransoul/dev/mcp-token.txt` | Developing TerranSoul itself |
| **Headless / Pet Mode** (`npm run mcp`) | `7423` | `<repo>/mcp-data/mcp-token.txt` | AI coding agents, CI, headless use |

All three can run simultaneously without port conflicts.

---

## 1. Starting the MCP Server

### Release Mode (Desktop App)

Launch the TerranSoul desktop app. The MCP server starts automatically
on `127.0.0.1:7421`. No extra steps needed.

The bearer token is auto-generated at:
- **Windows:** `%APPDATA%\com.terranes.terransoul\mcp-token.txt`
- **macOS/Linux:** `~/.local/share/com.terranes.terransoul/mcp-token.txt`

### Dev Mode (`cargo tauri dev`)

```bash
cd <terransoul-repo>
npm run dev
```

The MCP server starts on `127.0.0.1:7422`. Token is at:
- **Windows:** `%APPDATA%\com.terranes.terransoul\dev\mcp-token.txt`

### Headless / Pet Mode (No GUI)

Best for AI coding agents and automation — runs the brain without a window:

```bash
cd <terransoul-repo>
npm run mcp
```

Or use the auto-start script (builds from source if needed):

```bash
node scripts/copilot-start-mcp.mjs
```

Starts on `127.0.0.1:7423`. Token is at `<repo>/mcp-data/mcp-token.txt`.

Environment variables:
- `TERRANSOUL_MCP_PORT` — override the default port (default: `7423`)
- `TERRANSOUL_MCP_IDLE_TIMEOUT` — seconds before auto-shutdown (default: `300`, `0` = never)
- `TERRANSOUL_MCP_SKIP_BUILD=1` — skip cargo build (use existing binary)

---

## 2. Verifying the Server

### Health Check (No Auth Required)

```bash
curl http://127.0.0.1:7421/health
```

Response:
```json
{
  "status": "ok",
  "port": 7421,
  "brain_provider": "ollama",
  "brain_model": "gemma3:4b",
  "rag_quality_pct": 85,
  "memory_total": 1053,
  "descriptions": {
    "rag_quality": "Percentage of retrieval quality based on ...",
    "memory": "Total memories stored in the brain"
  }
}
```

### MCP Initialize (Auth Required)

```bash
curl -X POST http://127.0.0.1:7421/mcp \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}'
```

Response:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": "2024-11-05",
    "capabilities": { "tools": {}, "resources": {}, "prompts": {} },
    "serverInfo": {
      "name": "terransoul-brain",
      "version": "0.1.0",
      "buildMode": "release"
    }
  }
}
```

The `serverInfo.name` varies by mode:
- `terransoul-brain` — release app
- `terransoul-brain-dev` — dev build
- `terransoul-brain-mcp` — headless pet mode

---

## 3. Transport Options

### HTTP Transport (Streamable HTTP)

The primary transport. All tool calls go through `POST /mcp` as JSON-RPC 2.0.

- **Endpoint:** `http://127.0.0.1:<port>/mcp`
- **Auth:** `Authorization: Bearer <token>` header on every request
- **Content-Type:** `application/json`

### Stdio Transport

For editors that spawn the server as a subprocess (Claude Desktop, Cursor,
VS Code MCP extension):

```bash
terransoul --mcp-stdio
```

Or with a custom data directory:

```bash
TERRANSOUL_MCP_DATA_DIR=/path/to/data terransoul --mcp-stdio
```

Stdio uses newline-delimited JSON-RPC 2.0 on stdin/stdout. **No bearer token
is needed** — the parent-child process relationship is the trust boundary.
Diagnostic output goes to stderr.

---

## 4. Available Tools

List all tools via `tools/list`:

```bash
curl -X POST http://127.0.0.1:7421/mcp \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}'
```

### Brain Tools (Always Available)

| Tool | Description | Write? |
|---|---|---|
| `brain_search` | Hybrid + RRF + optional HyDE search over memories | No |
| `brain_get_entry` | Get a full memory entry by ID | No |
| `brain_list_recent` | List recent memories (filter by kind, tag, since) | No |
| `brain_kg_neighbors` | Knowledge-graph one-hop traversal around an entry | No |
| `brain_summarize` | LLM-summarize text, memory IDs, or a search query | No |
| `brain_suggest_context` | Flagship: curated context pack (memories + KG + summary) | No |
| `brain_ingest_url` | Fetch, chunk, embed a URL into the brain | **Yes** |
| `brain_health` | Server status, provider, model, RAG quality, memory count | No |
| `brain_failover_status` | Provider failover health and recent events | No |
| `brain_wiki_audit` | Knowledge wiki audit (conflicts, orphans, stale entries) | No |
| `brain_wiki_spotlight` | Most-connected memories by graph edge degree | No |
| `brain_wiki_serendipity` | Cross-community links bridging memory clusters | No |
| `brain_wiki_revisit` | Review queue of memories needing attention | No |
| `brain_wiki_digest_text` | Digest pasted text into the wiki (with dedup) | **Yes** |
| `brain_review_gaps` | Queries where retrieval found no good match | No |
| `brain_session_checklist` | MCP session compliance status | No |

### Code Intelligence Tools (When Available)

These require the code index to be built and `code_read` capability:

| Tool | Description |
|---|---|
| `code_query` | Search the code symbol index |
| `code_context` | Get context around a code location |
| `code_impact` | Analyze impact of changes to a symbol |
| `code_rename` | Rename a symbol across the codebase |
| `code_branch_diff` | Diff between branches |
| `code_branch_sync` | Sync branch index |
| And more... | See `tools/list` for the full set |

---

## 5. Calling Tools

### Example: Search Memories

```bash
curl -X POST http://127.0.0.1:7421/mcp \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/call",
    "params": {
      "name": "brain_search",
      "arguments": {
        "query": "how does the RAG pipeline work",
        "limit": 5,
        "mode": "rrf"
      }
    }
  }'
```

Response:
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "content": [{ "type": "text", "text": "[{\"id\":42,\"content\":\"...\",\"score\":0.87,...}]" }],
    "isError": false
  }
}
```

### Example: Get Curated Context

The flagship call — returns top memories, KG neighborhood, LLM summary,
and a delta-stable fingerprint for caching:

```bash
curl -X POST http://127.0.0.1:7421/mcp \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 3,
    "method": "tools/call",
    "params": {
      "name": "brain_suggest_context",
      "arguments": {
        "query": "streaming architecture for LLM responses",
        "file_path": "src-tauri/src/commands/streaming.rs",
        "limit": 5
      }
    }
  }'
```

### Example: Ingest a URL

```bash
curl -X POST http://127.0.0.1:7421/mcp \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 4,
    "method": "tools/call",
    "params": {
      "name": "brain_ingest_url",
      "arguments": {
        "url": "https://example.com/article",
        "tags": "research,architecture",
        "importance": 4
      }
    }
  }'
```

---

## 6. Client Configuration

### VS Code (GitHub Copilot)

TerranSoul auto-generates `.vscode/mcp.json` in the workspace. You can
also set it up manually:

**.vscode/mcp.json:**
```json
{
  "servers": {
    "terransoul-brain": {
      "type": "http",
      "url": "http://127.0.0.1:7421/mcp",
      "headers": {
        "Authorization": "Bearer ${env:TERRANSOUL_MCP_TOKEN}"
      }
    }
  }
}
```

Set the environment variable before launching VS Code:

**Windows (PowerShell):**
```powershell
$env:TERRANSOUL_MCP_TOKEN = Get-Content "$env:APPDATA\com.terranes.terransoul\mcp-token.txt"
```

**macOS/Linux:**
```bash
export TERRANSOUL_MCP_TOKEN=$(cat ~/.local/share/com.terranes.terransoul/mcp-token.txt)
```

For the headless runner (port 7423):
```json
{
  "servers": {
    "terransoul-brain-mcp": {
      "type": "http",
      "url": "http://127.0.0.1:7423/mcp",
      "headers": {
        "Authorization": "Bearer ${env:TERRANSOUL_MCP_TOKEN_MCP}"
      }
    }
  }
}
```

### VS Code (Stdio — No Token Needed)

```json
{
  "servers": {
    "terransoul-brain-stdio": {
      "type": "stdio",
      "command": "/path/to/terransoul",
      "args": ["--mcp-stdio"],
      "env": {
        "TERRANSOUL_MCP_DATA_DIR": "/path/to/mcp-data"
      }
    }
  }
}
```

### Claude Desktop

**`~/.config/Claude/claude_desktop_config.json`** (Linux/macOS) or
**`%APPDATA%\Claude\claude_desktop_config.json`** (Windows):

```json
{
  "mcpServers": {
    "terransoul-brain": {
      "command": "/path/to/terransoul",
      "args": ["--mcp-stdio"]
    }
  }
}
```

Claude Desktop uses stdio transport — no token needed.

### Cursor

**`~/.cursor/mcp.json`:**
```json
{
  "mcpServers": {
    "terransoul-brain": {
      "command": "/path/to/terransoul",
      "args": ["--mcp-stdio"]
    }
  }
}
```

### OpenAI Codex CLI

**`~/.codex/config.json`:**
```json
{
  "mcpServers": {
    "terransoul-brain": {
      "command": "/path/to/terransoul",
      "args": ["--mcp-stdio"],
      "env": {
        "TERRANSOUL_MCP_DATA_DIR": "/path/to/data"
      }
    }
  }
}
```

### Custom App (HTTP)

Any application can connect via HTTP. Here's a minimal example in Python:

```python
import requests
import json

MCP_URL = "http://127.0.0.1:7421/mcp"
TOKEN = open("mcp-token.txt").read().strip()
HEADERS = {
    "Authorization": f"Bearer {TOKEN}",
    "Content-Type": "application/json",
}

def mcp_call(method, params=None, call_id=1):
    payload = {
        "jsonrpc": "2.0",
        "id": call_id,
        "method": method,
        "params": params or {},
    }
    resp = requests.post(MCP_URL, headers=HEADERS, json=payload)
    return resp.json()

# Initialize the session
init = mcp_call("initialize")
print("Server:", init["result"]["serverInfo"]["name"])

# Search the brain
results = mcp_call("tools/call", {
    "name": "brain_search",
    "arguments": {"query": "memory architecture", "limit": 5}
}, call_id=2)
print("Results:", results["result"]["content"][0]["text"])
```

Here's a minimal example in Node.js:

```javascript
const MCP_URL = "http://127.0.0.1:7421/mcp";
const TOKEN = require("fs").readFileSync("mcp-token.txt", "utf8").trim();

async function mcpCall(method, params = {}, id = 1) {
  const resp = await fetch(MCP_URL, {
    method: "POST",
    headers: {
      "Authorization": `Bearer ${TOKEN}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ jsonrpc: "2.0", id, method, params }),
  });
  return resp.json();
}

// Initialize, then search
const init = await mcpCall("initialize");
console.log("Server:", init.result.serverInfo.name);

const results = await mcpCall("tools/call", {
  name: "brain_search",
  arguments: { query: "RAG pipeline", limit: 5 },
}, 2);
console.log("Results:", JSON.parse(results.result.content[0].text));
```

---

## 7. Capability Gating

Tools are gated by capability. The default profile is **read-only**:

| Capability | Default | Required for |
|---|---|---|
| `brain_read` | **on** | All read tools (search, get_entry, summarize, etc.) |
| `brain_write` | **off** | `brain_ingest_url`, `brain_wiki_digest_text` |
| `code_read` | **off** | Code intelligence tools |
| `code_write` | **off** | `code_branch_sync`, `code_index_commit` |

The MCP HTTP and stdio transports grant **full read-write** by default
(since they are bearer-token authenticated or trusted parent-child).

To restrict a client, configure capabilities in the TerranSoul Control
Panel (Brain tab → AI Integrations).

---

## 8. Notifications

MCP notifications (requests without an `id` field) are acknowledged with
`202 Accepted`. TerranSoul currently does not emit server-initiated
notifications.

```bash
curl -X POST http://127.0.0.1:7421/mcp \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}'
# Returns: 202 Accepted
```

---

## 9. Security

- **Loopback only** by default — binds to `127.0.0.1`, not `0.0.0.0`
- **Bearer token auth** on HTTP transport — auto-generated SHA-256 hex token
- **No auth on stdio** — trusted parent-child process (standard MCP behavior)
- **Token file permissions** — `0600` on Unix (owner-only read)
- **LAN exposure** is opt-in via the app's settings (`lan_enabled: true`);
  when enabled, public-read-only mode restricts remote clients to read ops
- Token regeneration: use the `mcp_regenerate_token` Tauri command or the
  Control Panel UI

---

## 10. Troubleshooting

| Problem | Solution |
|---|---|
| `connection refused` on port 7421 | Start the TerranSoul app or run `npm run mcp` |
| `401 Unauthorized` | Check your bearer token matches the one in `mcp-token.txt` |
| `health` returns `200` but `tools/call` fails | Verify `Authorization` header is set (health is unauthenticated) |
| Headless mode won't start | Run `node scripts/copilot-start-mcp.mjs` — it builds from source if needed |
| Token env var not picked up | Restart VS Code / your editor after setting the env var |
| Port already in use | Another TerranSoul instance may be running; check with `netstat -an \| findstr 7421` |
| `brain_ingest_url` returns permission error | Write capability may not be granted; MCP transport grants it by default |

### Checking Server Logs

- **Release app:** Check the app's log output in the system tray / console
- **Headless mode:** `mcp-data/self_improve_mcp_process.log`

---

## Appendix: Full Protocol Flow

A typical MCP session:

```
Client                              Server (TerranSoul)
  │                                      │
  ├─── POST /mcp ────────────────────────►│
  │    initialize                        │
  │◄──────────────────── 200 ────────────┤
  │    { protocolVersion, capabilities } │
  │                                      │
  ├─── POST /mcp ────────────────────────►│
  │    notifications/initialized         │
  │◄──────────────────── 202 ────────────┤
  │                                      │
  ├─── POST /mcp ────────────────────────►│
  │    tools/list                        │
  │◄──────────────────── 200 ────────────┤
  │    { tools: [...] }                  │
  │                                      │
  ├─── POST /mcp ────────────────────────►│
  │    tools/call: brain_health          │
  │◄──────────────────── 200 ────────────┤
  │    { content: [...], isError: false }│
  │                                      │
  ├─── POST /mcp ────────────────────────►│
  │    tools/call: brain_search          │
  │◄──────────────────── 200 ────────────┤
  │    { content: [...], isError: false }│
  │                                      │
  └──────────────────────────────────────┘
```

All requests use `Authorization: Bearer <token>` (except `/health`).
