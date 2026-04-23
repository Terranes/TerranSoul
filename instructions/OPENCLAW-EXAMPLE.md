# OpenClaw Example — Using OpenClaw Functions with the TerranSoul AI

This walkthrough shows how to use the **OpenClaw bridge agent** that ships in
TerranSoul's Agent Marketplace. The bridge demonstrates the canonical shape of
a TerranSoul ↔ external-platform integration: capability gating, tool-call
dispatch, and sentiment passthrough into the VRM character.

> **Source:** `src-tauri/src/agent/openclaw_agent.rs`
> **Manifest:** `src-tauri/src/registry_server/catalog.rs` (`openclaw-bridge`)
> **Audience:** Anyone integrating an external AI platform (OpenClaw, LangChain,
> Open Interpreter, custom JSON-RPC services) with TerranSoul.

---

## 1. What OpenClaw is

[OpenClaw](https://openclaw.dev) is an open-source AI tool platform that exposes
filesystem, network, and chat tools over a JSON-RPC interface. The
`openclaw-bridge` agent is TerranSoul's first-party adapter for that interface.
It registers itself as an [`AgentProvider`](../src-tauri/src/agent/mod.rs) so the
orchestrator can route messages to it the same way it routes to the built-in
stub agent or any other installed agent.

The bridge defines three tools:

| Directive                              | Capability  | What real OpenClaw does                |
|----------------------------------------|-------------|----------------------------------------|
| `/openclaw read <relative-path>`       | `file_read` | JSON-RPC `fs.read` against the runtime |
| `/openclaw fetch <url>`                | `network`   | JSON-RPC `net.fetch` against the runtime |
| `/openclaw chat <prompt>`              | `chat`      | Forwards the prompt to OpenClaw's chat tool |

In this repository the dispatch handlers return descriptive placeholder
responses instead of opening sockets — the boundary is intentional so the
integration is fully testable without an OpenClaw runtime in CI. Replacing the
match arms in `OpenClawAgent::handle_command` is all that's needed to wire the
real JSON-RPC calls.

---

## 2. End-to-end usage

### 2.1 Install via Marketplace

1. Launch TerranSoul (`npm run tauri dev`).
2. Open the **🏪 Marketplace** tab.
3. Find the **openclaw-bridge** card (it appears immediately because the
   default registry is now the in-process [`CatalogRegistry`]
   (../src-tauri/src/registry_server/catalog_registry.rs) — no need to start
   the registry HTTP server first).
4. Click **⬇ Install**. The capability-consent dialog lists the sensitive
   capabilities the bridge requires:
   - `filesystem` (mapped to sandbox `file_read` + `file_write`)
   - `network`
5. Approve the capabilities you want to grant. The bridge stores its grant set
   internally — directives whose capability you didn't grant are rejected with
   a clear error and the character expresses sadness (`Sentiment::Sad`).

### 2.2 Use OpenClaw tools from chat

Once installed and granted, type any of these into the main chat:

```text
/openclaw read README.md
/openclaw fetch https://example.com
/openclaw chat Summarise the last paragraph of my open file
```

The bridge will respond with the result of the tool call. Successful tool runs
trigger `Sentiment::Happy`, which the VRM facial expression pipeline picks up
just like any first-party agent response.

A plain message (no `/openclaw` prefix) returns a help string explaining the
available directives.

### 2.3 Swap in your own JSON-RPC client

`OpenClawAgent::handle_command` is the only place that needs to change to talk
to a real OpenClaw runtime. It already receives the parsed `OpenClawTool` and
its argument; replace the descriptive placeholder with an
[`reqwest`](https://docs.rs/reqwest)-backed JSON-RPC call, e.g.:

```rust
let body = serde_json::json!({
    "jsonrpc": "2.0",
    "id": 1,
    "method": match tool {
        OpenClawTool::Read  => "fs.read",
        OpenClawTool::Fetch => "net.fetch",
        OpenClawTool::Chat  => "chat.complete",
    },
    "params": { "arg": argument },
});
let response = reqwest::Client::new()
    .post("http://127.0.0.1:8732/rpc")
    .json(&body)
    .send().await
    .map_err(|e| e.to_string())?
    .text().await
    .map_err(|e| e.to_string())?;
Ok((response, Sentiment::Happy))
```

Capability gating in `handle_command` runs before the network call, so an
ungranted tool never hits the network — that's the architectural guarantee
the example provides.

---

## 3. Why this is the canonical integration shape

The OpenClaw bridge is intentionally small (~250 LoC including tests) and
exemplifies the four invariants every external-platform integration must keep:

1. **One `AgentProvider` impl per platform.** No cross-provider coupling — the
   orchestrator depends on the trait, not on `OpenClawAgent` (rule:
   `architecture-rules.md` §Module Dependency Rules).
2. **Capabilities authoritative inside the agent.** The bridge holds its own
   `granted_capabilities` set so a misconfigured orchestrator can't bypass
   consent.
3. **Pure-function parser.** `parse(message)` is a free function, exhaustively
   unit-tested. UI layers can reuse it without instantiating an agent.
4. **Sentiment is part of the contract.** Tool successes return `Happy`,
   failures return `Sad`, plain chat returns `Neutral`. The VRM pipeline
   already maps these — no extra wiring needed.

## 4. Local LLM models are also Marketplace agents

The Marketplace surfaces local Ollama models alongside packaged agents — they
appear with the `🖥` icon, the `local_llm` capability badge, and a RAM
estimate. Their **Install & Activate** button calls `pull_ollama_model` to
pull (if needed) and then `set_active_brain` + `set_brain_mode` to switch the
active brain — see [`MarketplaceView.vue`](../src/views/MarketplaceView.vue)
`handleLocalLlmAction()`. This unifies "install an agent" and "switch the
brain to a local model" under the same browse / install UX.

## 5. Tests

- Rust unit tests live alongside the agent: `cargo test -p terransoul --lib openclaw_agent`
- Integration tests for the marketplace registry: `cargo test -p terransoul --lib catalog_registry`
- Frontend tests: `npm run test` (existing `MarketplaceView.test.ts` continues to pass).
