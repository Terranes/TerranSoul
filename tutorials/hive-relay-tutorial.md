# Hive Relay — Self-Hosting & Development Tutorial

> **TerranSoul v0.1** · Last updated: 2026-05-07
>
> Related: [Device Sync & Hive](device-sync-hive-tutorial.md) ·
> [LAN Brain Sharing](lan-mcp-sharing-tutorial.md) ·
> [Architecture: `docs/hive-protocol.md`](../docs/hive-protocol.md)

Deploy, operate, and extend the TerranSoul Hive Relay — an opt-in
gRPC federation server for sharing knowledge between TerranSoul
instances across the internet.

---

## Table of Contents

1. [What Is the Hive Relay?](#1-what-is-the-hive-relay)
2. [Architecture Overview](#2-architecture-overview)
3. [Quick Start (Docker)](#3-quick-start-docker)
4. [Building from Source](#4-building-from-source)
5. [Configuration](#5-configuration)
6. [Database Schema](#6-database-schema)
7. [gRPC API Reference](#7-grpc-api-reference)
8. [Security Model](#8-security-model)
9. [Production Deployment](#9-production-deployment)
10. [Distributed Jobs](#10-distributed-jobs)
11. [Development & Testing](#11-development--testing)
12. [Extending the Relay](#12-extending-the-relay)
13. [Troubleshooting](#13-troubleshooting)

---

## 1. What Is the Hive Relay?

![Hive Relay link panel](screenshots/hive-relay/01-link-panel.png)

The Hive Relay is a standalone Rust server (`crates/hive-relay/`) that
routes Ed25519-signed envelopes between TerranSoul devices. It handles:

- **Bundles** — batches of memories + knowledge-graph edges
- **Ops** — real-time CRDT operations (ephemeral)
- **Jobs** — distributed AI work (embedding, summarisation, etc.)

The relay never sees private memories (client-side filtering). It only
stores `hive`-scoped data that devices explicitly publish.

---

## 2. Architecture Overview

```
┌──────────────┐         gRPC (TLS)         ┌───────────────────┐
│  TerranSoul  │ ──── Submit / Subscribe ──→ │   Hive Relay      │
│  (Device A)  │                             │                   │
└──────────────┘                             │  ┌─────────────┐  │
                                             │  │ PostgreSQL  │  │
┌──────────────┐         gRPC (TLS)         │  │  • bundles  │  │
│  TerranSoul  │ ←── Subscribe stream ───── │  │  • jobs     │  │
│  (Device B)  │                             │  │  • hlc_wm   │  │
└──────────────┘                             │  └─────────────┘  │
                                             └───────────────────┘
```

| Component | Port | Purpose |
|-----------|------|---------|
| Hive Relay | 50051 | gRPC server |
| PostgreSQL | 5433 (host) / 5432 (container) | Persistent storage |

---

## 3. Quick Start (Docker)

### Prerequisites

- Docker + Docker Compose v2 (`docker compose version`)
- 512 MB RAM minimum for Postgres + relay

### Launch

```bash
cd crates/hive-relay
docker compose up -d
```

### Verify

```bash
# Check services are healthy
docker compose ps

# Health check (with grpcurl)
grpcurl -plaintext localhost:50051 hive.HiveRelay/Health

# Expected:
# { "version": "0.1.0", "connectedDevices": "0", "pendingJobs": "0" }
```

### Connect TerranSoul

1. Open **Settings → Network** in the TerranSoul app.
2. Enter Hive URL: `http://localhost:50051` (or your server's IP).
3. Click **Connect** → status shows "Connected ✓".

---

## 4. Building from Source

### Prerequisites

- Rust 1.80+ (`rustup update`)
- `protoc` (Protocol Buffers compiler)
- PostgreSQL 14+ (or use the Docker Compose Postgres)

### Build

```bash
cd crates/hive-relay
cargo build --release
```

The binary is at `target/release/hive-relay`.

### Run Locally

```bash
# Start just Postgres from Docker Compose
docker compose up -d postgres

# Run the relay natively
DATABASE_URL="postgres://hive:hive_dev_password@localhost:5433/hive_relay" \
LISTEN_ADDR="0.0.0.0:50051" \
RUST_LOG="hive_relay=info" \
cargo run --release
```

---

## 5. Configuration

The relay is configured via CLI args or environment variables:

| Env / Flag | Default | Description |
|---|---|---|
| `DATABASE_URL` / `--database-url` | (required) | PostgreSQL connection URL |
| `LISTEN_ADDR` / `--listen` | `0.0.0.0:50051` | gRPC bind address |
| `RUST_LOG` | `hive_relay=info` | Tracing filter (use `debug` for development) |

### Example `.env`

```env
DATABASE_URL=postgres://hive:secret@db.example.com:5432/hive_relay
LISTEN_ADDR=0.0.0.0:50051
RUST_LOG=hive_relay=info
```

Place this in `crates/hive-relay/.env` — it's loaded automatically via `dotenvy`.

---

## 6. Database Schema

The relay auto-migrates on startup. Three core tables:

```sql
-- Persisted bundles (signed memory batches)
CREATE TABLE IF NOT EXISTS hive_bundles (
    id          BIGSERIAL PRIMARY KEY,
    sender_id   TEXT NOT NULL,
    bundle_id   TEXT NOT NULL UNIQUE,
    hlc_counter BIGINT NOT NULL,
    payload     BYTEA NOT NULL,
    signature   BYTEA NOT NULL,
    received_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Job queue (distributed AI work)
CREATE TABLE IF NOT EXISTS hive_jobs (
    id          BIGSERIAL PRIMARY KEY,
    job_id      TEXT NOT NULL UNIQUE,
    sender_id   TEXT NOT NULL,
    payload     BYTEA NOT NULL,
    signature   BYTEA NOT NULL,
    status      TEXT NOT NULL DEFAULT 'pending',
    worker_id   TEXT,
    enqueued_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    claimed_at  TIMESTAMPTZ,
    completed_at TIMESTAMPTZ
);

-- HLC watermarks for replay prevention
CREATE TABLE IF NOT EXISTS hlc_watermarks (
    sender_id   TEXT PRIMARY KEY,
    hlc_counter BIGINT NOT NULL DEFAULT 0,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

---

## 7. gRPC API Reference

The relay exposes 5 RPCs defined in `proto/hive.proto`:

### `Submit(HiveEnvelope) → SubmitResponse`

Submit a signed envelope. The relay:
1. Verifies the Ed25519 signature
2. Checks replay protection (HLC watermark)
3. Routes by `msg_type`:
   - `BUNDLE` → stored in Postgres + broadcast to subscribers
   - `OP` → broadcast only (ephemeral)
   - `JOB` → enqueued in `hive_jobs`

### `Subscribe(SubscribeRequest) → stream HiveEnvelope`

Open a server-streaming connection. Receives:
1. Historical bundles since `since_hlc` (catch-up)
2. Live broadcasts of new envelopes

### `ClaimJob(ClaimJobRequest) → ClaimJobResponse`

Pull the next pending job matching your capabilities. Uses `FOR UPDATE SKIP LOCKED` for fairness — multiple workers can poll without conflicts.

### `CompleteJob(CompleteJobRequest) → SubmitResponse`

Submit results for a claimed job. The result bundle is broadcast to the original requester.

### `Health(Empty) → HealthResponse`

Returns version, connected device count, and pending job count.

---

## 8. Security Model

| Layer | Mechanism |
|---|---|
| **Authentication** | Ed25519 signature on every envelope (public key in `sender_pubkey`) |
| **Replay prevention** | Per-sender HLC watermark — relay rejects `hlc ≤ last_seen` |
| **Privacy** | Client-side filtering — relay never receives `private` or `paired` memories |
| **Transport** | TLS recommended for production (configure in Tonic server builder) |
| **Wire format** | MessagePack payload + optional LZ4 compression (`compressed` flag) |

### Signature Verification

```
sign_input = version(1) ∥ msg_type(1) ∥ sender_id(UTF-8) ∥
             timestamp(8 LE) ∥ hlc_counter(8 LE) ∥ payload(raw)

signature = Ed25519_sign(device_signing_key, sign_input)
```

The relay verifies before storing/broadcasting. Invalid signatures are rejected with a `SubmitResponse { accepted: false, error: "..." }`.

---

## 9. Production Deployment

### Docker (recommended)

```bash
# Build the production image
cd crates/hive-relay
docker build -t terransoul-hive-relay:latest .

# Run with your own Postgres
docker run -d \
  --name hive-relay \
  -p 50051:50051 \
  -e DATABASE_URL="postgres://user:pass@your-db:5432/hive_relay" \
  -e LISTEN_ADDR="0.0.0.0:50051" \
  -e RUST_LOG="hive_relay=info" \
  terransoul-hive-relay:latest
```

### Enabling TLS

For production, add TLS to the Tonic server. Edit `main.rs`:

```rust
use tonic::transport::{Identity, ServerTlsConfig};

let cert = std::fs::read("server.pem")?;
let key = std::fs::read("server.key")?;

Server::builder()
    .tls_config(ServerTlsConfig::new().identity(Identity::from_pem(&cert, &key)))?
    .add_service(HiveRelayServer::new(service))
    .serve(addr)
    .await?;
```

Clients then connect with `https://relay.example.com:50051`.

### Monitoring

- **Health endpoint:** `grpcurl relay:50051 hive.HiveRelay/Health`
- **Postgres metrics:** standard `pg_stat_*` views
- **Tracing:** set `RUST_LOG=hive_relay=debug` for detailed logs
- **Docker health:** add a health check script that calls `Health` RPC

### Resource Sizing

| Scale | Postgres | Relay RAM | Bandwidth |
|---|---|---|---|
| 1–10 devices | 256 MB | 64 MB | Minimal |
| 10–100 devices | 1 GB | 256 MB | ~1 Mbps |
| 100–1000 devices | 4 GB + replicas | 512 MB | ~10 Mbps |

---

## 10. Distributed Jobs

The job system lets devices offload AI work to capable peers.

### Job Flow

```
Device A (no GPU)              Relay                Device B (has GPU)
─────────────────              ─────                ─────────────────
Create JOB envelope    →    enqueue_job()
  capabilities:                  │
  ["embedding:nomic"]            │
                                 │         ClaimJob(capabilities) ←
                                 │         Match? → lock row
                                 │         Return job envelope   →
                                 │
                                 │         Execute locally
                                 │         (run Ollama embed)
                                 │
                                 │         CompleteJob(results)  →
                              broadcast
        ← result bundle arrives
```

### Capability Format

Capabilities are `kind:value` pairs matched with AND logic:

```
Required: ["embedding:nomic-embed-text", "gpu:8gb"]
Worker:   ["embedding:nomic-embed-text", "gpu:rtx4090", "brain:ollama"]
→ Match ✓ (superset)
```

### Job Status Lifecycle

```
pending → claimed → completed
                 ↘ timeout → pending (re-queued)
```

---

## 11. Development & Testing

### Run Tests

```bash
cd crates/hive-relay
cargo test
```

Tests use in-memory state where possible. Integration tests that need
Postgres require the Docker Compose stack running.

### Manual Testing with grpcurl

```bash
# Health check
grpcurl -plaintext localhost:50051 hive.HiveRelay/Health

# Subscribe (streams until cancelled)
grpcurl -plaintext -d '{"device_id":"dev-test","since_hlc":"0"}' \
  localhost:50051 hive.HiveRelay/Subscribe
```

### Adding a New RPC

1. Edit `proto/hive.proto` — add message types and RPC definition.
2. Run `cargo build` — `tonic-build` regenerates the Rust code.
3. Implement the handler in `src/relay.rs`.
4. Add persistence logic in `src/db.rs` if needed.
5. Add tests.

---

## 12. Extending the Relay

### Custom Message Types

To add a new `MsgType`:

1. Add the variant to `MsgType` enum in `hive.proto`.
2. Handle it in the `match msg_type { ... }` block in `relay.rs`.
3. Decide: persist (like BUNDLE) or broadcast-only (like OP)?

### Filtering / Moderation

The relay is neutral by default — it forwards any valid signed
envelope. To add filtering:

```rust
// In relay.rs, after verify_envelope():
if should_reject(&envelope) {
    return Ok(Response::new(SubmitResponse {
        accepted: false,
        error: "content policy".into(),
    }));
}
```

### Multi-Relay Federation

Future: relays can peer with each other by subscribing to one another's
streams. This is not yet implemented but the protocol supports it —
each relay is just another "device" with its own Ed25519 identity.

---

## 13. Troubleshooting

| Problem | Solution |
|---|---|
| `connection refused` on 50051 | Is the relay running? Check `docker compose ps` or process list |
| `Signature verification failed` | Device identity may have been regenerated. Re-pair. |
| `Replay rejected` | Device's HLC is behind the relay's watermark. Usually means a restart reset the local clock — wait for it to advance. |
| Postgres connection error | Verify `DATABASE_URL`. Check Postgres logs: `docker compose logs postgres` |
| `protoc not found` during build | Install: `apt install protobuf-compiler` or `brew install protobuf` |
| Job never claimed | No connected worker has the required capabilities. Check with `Health` RPC. |

---

## Related Tutorials

- **[Device Sync & Hive Federation](device-sync-hive-tutorial.md)** — User-facing pairing, sync, and privacy controls
- **[LAN Brain Sharing](lan-mcp-sharing-tutorial.md)** — Local network MCP sharing (no relay needed)
- **[Advanced Memory & RAG](advanced-memory-rag-tutorial.md)** — Understanding the memory system that Hive syncs

---

## File Reference

| Path | Purpose |
|---|---|
| `crates/hive-relay/src/main.rs` | CLI entrypoint, Tonic server bootstrap |
| `crates/hive-relay/src/relay.rs` | gRPC service implementation (5 RPCs) |
| `crates/hive-relay/src/db.rs` | PostgreSQL persistence + job queue |
| `crates/hive-relay/src/verify.rs` | Ed25519 signature verification |
| `crates/hive-relay/src/lib.rs` | Crate root + protobuf include |
| `crates/hive-relay/proto/hive.proto` | Protocol Buffers service definition |
| `crates/hive-relay/docker-compose.yml` | Docker Compose (Postgres + relay) |
| `crates/hive-relay/Dockerfile` | Multi-stage Rust build → slim runtime |
| `docs/hive-protocol.md` | Full protocol specification |
