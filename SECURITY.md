# Security Policy

TerranSoul is a pre-release, local-first desktop companion. Security posture is
therefore documented by supported branches/features rather than by stable public
versions.

## Supported stage

| Stage | Status | Security support |
|---|---|---|
| `main` / active pre-release branch | Supported | Security fixes, dependency updates, and hardening changes are accepted before release. |
| Published stable releases | Not yet applicable | TerranSoul has not shipped a stable public release yet. |

## Current security model

- **Local-first by default.** Brain, memory, MCP, and gRPC services bind to
  loopback unless the user explicitly enables a LAN feature.
- **MCP HTTP auth.** MCP HTTP uses bearer-token authentication with a token stored
  in the app data directory; MCP stdio is trusted only as a child-process pipe.
- **gRPC transport foundation.** The `brain.v1` tonic transport supports
  rustls/mTLS configuration and refuses plaintext serving on non-loopback
  addresses. LAN activation remains gated behind the Phase 24 pairing/device
  registry work.
- **No silent LAN exposure.** Any future LAN bind must be opt-in, user-visible,
  and protected by per-device certificates or equivalent capability checks.
- **Plugin sandboxing.** Plugin execution must remain capability-gated; untrusted
  WASM hooks run through the sandbox and must not read arbitrary memories.
- **Secret handling.** Do not commit API keys, model tokens, bearer tokens,
  certificates, private keys, `.env` files, memory databases, or user VRM assets.

## Reporting a vulnerability

Please report vulnerabilities privately through GitHub Security Advisories for
`Terranes/TerranSoul` when available. If GitHub advisories are unavailable to
you, open a minimal public issue that does **not** include exploit details or
secrets, and request a private maintainer contact.

Include:

1. Affected commit, branch, or build.
2. Component (`src-tauri`, frontend, MCP, gRPC, plugin system, sync, memory DB,
   packaging, etc.).
3. Reproduction steps and impact.
4. Whether any secret, token, certificate, or personal data may have been exposed.

## Maintainer response targets

- Acknowledge: best effort within 7 days.
- Triage severity and affected surface: best effort within 14 days.
- Fix timeline: prioritized by exploitability and whether the vulnerable surface
  is loopback-only, LAN-exposed, or remotely reachable.

## Dependency and quality checks

Security-sensitive changes should run the repository validation gate where
practical:

```bash
npm run build
npm run test
cd src-tauri && cargo clippy --all-targets -- -D warnings
cd src-tauri && cargo test --all-targets
```

When adding dependencies, check the GitHub Advisory Database for the exact
package/version before committing. Rust networking, crypto, parser, plugin, and
IPC changes should also be reviewed for fail-closed defaults.
