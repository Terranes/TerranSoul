# OpenClaw Plugin Example - Using OpenClaw Effectively With TerranSoul

This walkthrough shows how to use the built-in **OpenClaw Bridge** plugin that
ships with TerranSoul. It follows the same PluginHost path as Translator Mode:
the plugin contributes commands, a slash command, activation events, and
capability-gated execution through the shared plugin command dispatcher.

> Source: `src-tauri/src/plugins/host.rs` (`openclaw-bridge` built-in plugin)
> Parser compatibility: `src-tauri/src/agent/openclaw_agent.rs`
> Related example: `docs/plugin-development.md` (`terransoul-translator`)

OpenClaw is no longer surfaced as an Agent Marketplace entry. The Marketplace
catalog stays focused on installable agents and sidecars; OpenClaw is a plugin
tool bridge because its natural shape is explicit command execution rather than
general chat routing.

---

## 1. Integration model

OpenClaw-style runtimes are strongest when they own external tool execution:
filesystem reads, network fetches, and runtime-specific tool calls. TerranSoul is
strongest when it owns context, memory, personality, device trust, and user
consent. The clean boundary is:

| TerranSoul owns | OpenClaw owns |
|---|---|
| Brain mode selection and RAG context | JSON-RPC tool execution |
| Long-term memory and learned preferences | Runtime-specific filesystem/network tools |
| Persona, VRM expression, and chat UX | External automation workflows |
| Capability consent and auditability | Tool result payloads |
| Cross-device pairing and LAN trust | Optional local/remote OpenClaw service health |

That means the best user experience is not "route all chat to OpenClaw". It is
"ask TerranSoul to think with its memory, then invoke OpenClaw deliberately when
a tool action is needed." Explicit `/openclaw ...` commands keep the trust
boundary visible and make file/network access auditable.

---

## 2. What the plugin contributes

The built-in plugin id is `openclaw-bridge`. It contributes these commands:

| Command | Slash/direct usage | Capability |
|---|---|---|
| `openclaw-bridge.dispatch` | `/openclaw read README.md` | Depends on parsed tool |
| `openclaw-bridge.read` | Read a relative path | `file_read` |
| `openclaw-bridge.fetch` | Fetch a URL | `network` |
| `openclaw-bridge.chat` | Forward a prompt to OpenClaw chat | none sensitive in the stub path |
| `openclaw-bridge.status` | Show help/status | none |

The plugin also contributes the slash command `/openclaw`, which dispatches to
`openclaw-bridge.dispatch`.

Current CI-safe behavior returns descriptive placeholder output, for example
`[openclaw/read] would read README.md`. That is intentional: the plugin boundary,
parser, command routing, slash command, and capability checks can be tested
without requiring an OpenClaw runtime in CI.

---

## 3. Capability grants

Sensitive OpenClaw tools are denied unless the persisted capability store has a
grant for plugin id `openclaw-bridge`.

Grant strings exposed by the sandbox command layer:

```text
file_read
network
```

Useful examples:

```text
/openclaw read README.md
/openclaw fetch https://example.com
/openclaw chat Summarise the latest design tradeoff
```

If `file_read` is missing, reads fail with a clear `FileRead` capability error.
If `network` is missing, fetches fail with a clear `Network` capability error.
Chat remains available because it does not exercise filesystem or network inside
the built-in placeholder path.

---

## 4. Wiring a real OpenClaw runtime

The real-runtime seam is `invoke_openclaw_tool()` in
`src-tauri/src/plugins/host.rs`. Replace the placeholder output with a thin
JSON-RPC client after capability checks pass.

Recommended JSON-RPC mapping:

| Tool | JSON-RPC method | Params |
|---|---|---|
| `read` | `fs.read` | `{ "path": argument }` |
| `fetch` | `net.fetch` | `{ "url": argument }` |
| `chat` | `chat.complete` | `{ "prompt": argument }` |

Sketch:

```rust
let method = match tool {
    OpenClawTool::Read => "fs.read",
    OpenClawTool::Fetch => "net.fetch",
    OpenClawTool::Chat => "chat.complete",
};

let body = serde_json::json!({
    "jsonrpc": "2.0",
    "id": uuid::Uuid::new_v4().to_string(),
    "method": method,
    "params": { "argument": argument },
});

let response = reqwest::Client::new()
    .post("http://127.0.0.1:8732/rpc")
    .json(&body)
    .send()
    .await
    .map_err(|e| e.to_string())?
    .text()
    .await
    .map_err(|e| e.to_string())?;

Ok(CommandResult::success(response))
```

Keep the client thin. TerranSoul should not fork or embed OpenClaw. It should
send typed requests to a user-controlled runtime, then return the result through
the normal plugin command result path.

---

## 5. How TerranSoul and OpenClaw work best together

Use TerranSoul for planning, memory, and coordination:

- Ask TerranSoul to recall relevant project history before invoking a tool.
- Let TerranSoul summarize OpenClaw results into durable memory only after the
  user confirms the result matters.
- Keep persona, expression, and chat narration in TerranSoul so the companion
  remains consistent across local, cloud, and mobile workflows.
- Use TerranSoul Link when a phone should monitor progress or continue a
  workflow on the desktop.

Use OpenClaw for bounded tool work:

- Read a specific file when the user names it.
- Fetch a specific URL when network access has been granted.
- Run an OpenClaw-side workflow only after the user has explicitly requested the
  tool action.
- Return structured results that TerranSoul can summarize, cite, or store.

Avoid hidden coupling:

- Do not let ordinary chat silently read files or fetch URLs.
- Do not store raw OpenClaw tool output as memory without summarization or user
  review.
- Do not grant `file_write`, `process_spawn`, or remote-exec style capabilities
  until a specific future plugin command needs them.

---

## 6. Chunk plan for deeper integration

| Chunk | Goal | Status |
|---|---|---|
| OpenClaw Plugin Bridge | Register OpenClaw as a built-in PluginHost plugin like Translator, with commands, `/openclaw`, and capability checks. | Complete |
| Runtime Client | Add configurable JSON-RPC endpoint, health check, timeout, typed error handling, and tests with a mock OpenClaw server. | Future |
| Structured Results | Normalize OpenClaw responses into `CommandResult` metadata and optional memory summaries. | Future |
| Workflow Handoff | Let TerranSoul create explicit OpenClaw work orders from chat plans while still requiring user confirmation before tool execution. | Future |
| Mobile Observation | Expose OpenClaw plugin status and long-running workflow progress through the existing phone-control surface. | Future |

These chunks keep the integration incremental. Each one can be validated without
changing the core chat orchestrator or weakening plugin capability consent.

---

## 7. Tests

Focused Rust tests live in `src-tauri/src/plugins/host.rs`:

```text
production_host_registers_openclaw_plugin
openclaw_read_requires_file_read_capability
openclaw_slash_dispatch_runs_with_file_read_grant
openclaw_fetch_requires_network_capability
openclaw_chat_command_uses_no_sensitive_capability
```

The legacy parser and compatibility provider tests remain in
`src-tauri/src/agent/openclaw_agent.rs`. Keep those parser tests because the
plugin reuses the same directive parser.
