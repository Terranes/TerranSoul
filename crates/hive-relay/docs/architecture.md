# Hive Relay — Architecture

## Overview

The Hive Relay is a lightweight gRPC server that routes signed envelopes
between TerranSoul instances. It serves as:

1. **Message router** — Accepts and broadcasts signed knowledge bundles
2. **Job queue** — Dispatches distributed AI work to capable peers
3. **Consistency anchor** — Enforces HLC-based replay protection

The relay is **not** required for device-to-device sync (that uses direct
QUIC/WebSocket via `LinkManager`). It's needed only for:
- Multi-user knowledge sharing (teams, communities)
- Internet-distance sync when LAN is unavailable
- Distributed job offloading (e.g., embedding on a GPU peer)

```
┌──────────────┐     gRPC/TLS      ┌─────────────┐     gRPC/TLS      ┌──────────────┐
│  Device A    │ ─────Submit()────→ │  Hive Relay │ ←───Subscribe()── │  Device B    │
│  (desktop)   │ ←──Subscribe()─── │  (Postgres) │ ────Submit()────→ │  (mobile)    │
└──────────────┘                    └─────────────┘                    └──────────────┘
       │                                   │                                  │
       │    Direct QUIC (LAN pairing)      │      Broadcast channel           │
       └───────────────────────────────────┘──────────────────────────────────┘
```

## Tech Stack

| Component | Technology | Purpose |
|-----------|-----------|---------|
| Server | Rust + Tonic (gRPC) | High-performance async service |
| Database | PostgreSQL 16 + pgvector | Bundle storage, job queue, watermarks |
| Crypto | ed25519-dalek | Envelope signature verification |
| Serialization | MessagePack (rmp-serde) | Compact binary payload encoding |
| Compression | LZ4 (optional) | Large bundle compression |
| Container | Docker + docker-compose | Easy deployment |

## Message Types

### BUNDLE (type=0)

A signed batch of memory and edge CRDT deltas. Used for periodic sync and
knowledge sharing.

- **Persisted** in PostgreSQL (`hive_bundles` table)
- **Broadcast** to all connected subscribers
- **Size limit:** 64 MiB per bundle, 10,000 memories, 50,000 edges

### OP (type=1)

A single CRDT operation for low-latency real-time sync.

- **Ephemeral** — not persisted to DB
- **Broadcast** to subscribers immediately
- **Subsumed** by the next BUNDLE exchange (so missed OPs aren't lost)

### JOB (type=2)

A distributed work item with capability requirements.

- **Queued** in PostgreSQL (`hive_jobs` table)
- **Claimed** by workers via `SKIP LOCKED` (fair, non-blocking)
- **Timeout/retry** — unclaimed jobs return to queue

## Data Flow

### Bundle Submission

```
Client                           Relay                              Postgres
  │                                │                                  │
  │── Submit(HiveEnvelope) ──────→│                                  │
  │                                │── verify_envelope() ────────────│
  │                                │── check HLC > watermark ───────→│
  │                                │── store_bundle() ──────────────→│
  │                                │── update_hlc_watermark() ──────→│
  │                                │── broadcast to subscribers ─────│
  │←─ SubmitResponse(accepted) ───│                                  │
```

### Job Lifecycle

```
Originator                    Relay                         Worker
    │                           │                             │
    │── Submit(JOB) ──────────→│                             │
    │                           │── enqueue_job() ──────────→│ (Postgres)
    │                           │                             │
    │                           │←── ClaimJob() ─────────────│
    │                           │── claim_job() ────────────→│ (SKIP LOCKED)
    │                           │── ClaimJobResponse ───────→│
    │                           │                             │
    │                           │←── CompleteJob(result) ────│
    │                           │── complete_job() ─────────→│ (Postgres)
    │                           │── broadcast result ────────│
    │←── result bundle ────────│                             │
```

## Security Model

### Envelope Signing

Every message is Ed25519-signed before leaving the client:

```
sign_input = version (1 byte)
           ∥ msg_type (1 byte)
           ∥ sender_id (UTF-8)
           ∥ timestamp (8 bytes LE)
           ∥ hlc_counter (8 bytes LE)
           ∥ payload (raw bytes)

signature = Ed25519_sign(device_signing_key, sign_input)
```

The relay **rejects** any envelope with an invalid signature.

### Replay Protection

- Each device maintains a monotonic HLC counter.
- The relay tracks the **highest HLC** per device (`hive_hlc_watermarks`).
- Envelopes with `hlc_counter <= last_seen` are rejected.
- Subscribers specify a `since_hlc` cursor to resume from.

### Privacy Enforcement

Privacy is enforced **at the sender** (not the relay):

1. Memories with `share_scope = private` are NEVER included in outbound bundles.
2. Edges referencing any private endpoint are excluded.
3. The relay cannot inspect payload content (opaque MessagePack).
4. The relay only verifies signatures and routes.

### Threat Model

| Threat | Mitigation |
|--------|------------|
| Relay compromise | Cannot forge signatures; payloads opaque |
| Replay attack | HLC monotonicity enforcement |
| Impersonation | Ed25519 signature verification |
| Data exfiltration | Client-side `share_scope` filtering |
| DoS | Rate limiting, 64 MiB bundle cap |
| Privilege escalation | No admin API; relay is a dumb router |

## Database Schema

```sql
CREATE TABLE IF NOT EXISTS hive_bundles (
    bundle_id     TEXT PRIMARY KEY,
    sender_id     TEXT NOT NULL,
    hlc_counter   BIGINT NOT NULL,
    payload       BYTEA NOT NULL,
    signature     BYTEA NOT NULL,
    received_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS hive_jobs (
    job_id        TEXT PRIMARY KEY,
    sender_id     TEXT NOT NULL,
    payload       BYTEA NOT NULL,
    signature     BYTEA NOT NULL,
    status        TEXT NOT NULL DEFAULT 'pending',
    worker_id     TEXT,
    enqueued_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    claimed_at    TIMESTAMPTZ,
    completed_at  TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS hive_hlc_watermarks (
    device_id     TEXT PRIMARY KEY,
    highest_hlc   BIGINT NOT NULL DEFAULT 0
);

CREATE INDEX idx_bundles_hlc ON hive_bundles(hlc_counter);
CREATE INDEX idx_jobs_status ON hive_jobs(status, enqueued_at);
```

## Configuration

| Environment Variable | Default | Description |
|---------------------|---------|-------------|
| `DATABASE_URL` | (required) | PostgreSQL connection URL |
| `LISTEN_ADDR` | `0.0.0.0:50051` | gRPC listen address |
| `RUST_LOG` | `hive_relay=info` | Tracing log level |

## Source Layout

```
crates/hive-relay/
├── proto/hive.proto       — Protobuf service definition
├── src/
│   ├── main.rs            — CLI entrypoint (clap + tokio)
│   ├── lib.rs             — Module declarations + proto include
│   ├── relay.rs           — gRPC service impl (Submit, Subscribe, ClaimJob, etc.)
│   ├── verify.rs          — Ed25519 signature verification
│   └── db.rs              — PostgreSQL persistence (SQLx)
├── build.rs               — tonic-build protoc compilation
├── Cargo.toml             — Dependencies
├── Dockerfile             — Multi-stage Rust build
└── docker-compose.yml     — Postgres + relay stack
```
