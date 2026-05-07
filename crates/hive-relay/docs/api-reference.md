# Hive Relay — API Reference

> **Transport:** gRPC (Tonic)
> **Proto file:** [`proto/hive.proto`](../proto/hive.proto)
> **Default port:** 50051

---

## Service: `HiveRelay`

### `Submit`

Submit a signed envelope to the relay (BUNDLE, OP, or JOB).

```protobuf
rpc Submit(HiveEnvelope) returns (SubmitResponse);
```

**Request:** `HiveEnvelope`

| Field | Type | Description |
|-------|------|-------------|
| `version` | uint32 | Protocol version (must be `1`) |
| `msg_type` | MsgType | `MSG_TYPE_BUNDLE` (0), `MSG_TYPE_OP` (1), or `MSG_TYPE_JOB` (2) |
| `sender_id` | string | Device UUID |
| `sender_pubkey` | bytes | 32-byte Ed25519 verifying key |
| `timestamp` | uint64 | Unix milliseconds |
| `hlc_counter` | uint64 | Hybrid Logical Clock counter (monotonic per device) |
| `payload` | bytes | MessagePack-encoded body |
| `signature` | bytes | 64-byte Ed25519 signature over canonical sign-input |
| `compressed` | bool | Whether payload is LZ4-compressed |

**Response:** `SubmitResponse`

| Field | Type | Description |
|-------|------|-------------|
| `accepted` | bool | `true` if envelope was accepted |
| `error` | string | Error reason if rejected (empty on success) |

**Error conditions:**
- Invalid signature → `UNAUTHENTICATED`
- HLC replay (counter ≤ watermark) → `accepted: false`, error explains
- Unknown msg_type → `accepted: false`
- Internal DB failure → `INTERNAL`

---

### `Subscribe`

Open a server-streaming connection to receive envelopes destined for this device.

```protobuf
rpc Subscribe(SubscribeRequest) returns (stream HiveEnvelope);
```

**Request:** `SubscribeRequest`

| Field | Type | Description |
|-------|------|-------------|
| `device_id` | string | Subscriber's device UUID |
| `public_key` | bytes | 32-byte verifying key (for identity) |
| `capabilities` | repeated Capability | What this device can do (for job matching) |
| `since_hlc` | uint64 | Resume cursor — only deliver envelopes with `hlc > since_hlc` |

**Response stream:** `HiveEnvelope` (continuous)

The relay delivers:
1. All persisted bundles with `hlc_counter > since_hlc` (catch-up)
2. All real-time envelopes submitted after subscription (live stream)

**Stream lifecycle:**
- The stream stays open until the client disconnects.
- Reconnect with the latest `hlc_counter` seen to resume without duplicates.

---

### `ClaimJob`

Claim the next available job from the relay queue.

```protobuf
rpc ClaimJob(ClaimJobRequest) returns (ClaimJobResponse);
```

**Request:** `ClaimJobRequest`

| Field | Type | Description |
|-------|------|-------------|
| `worker_id` | string | Worker's device UUID |
| `capabilities` | repeated Capability | Worker's advertised capabilities |

**Response:** `ClaimJobResponse`

| Field | Type | Description |
|-------|------|-------------|
| `job_id` | string | Job UUID (empty if no job available) |
| `sender_id` | string | Originator's device ID |
| `payload` | bytes | Serialized job input |
| `signature` | bytes | Original envelope signature |

**Behavior:**
- Uses `SELECT ... FOR UPDATE SKIP LOCKED` for fair, non-blocking claims.
- Returns empty response if no pending jobs match capabilities.
- Claimed jobs that aren't completed within timeout return to the queue.

---

### `CompleteJob`

Submit the result of a completed job.

```protobuf
rpc CompleteJob(CompleteJobRequest) returns (SubmitResponse);
```

**Request:** `CompleteJobRequest`

| Field | Type | Description |
|-------|------|-------------|
| `job_id` | string | The job being completed |
| `worker_id` | string | Must match the claimer |
| `result_envelope` | HiveEnvelope | Signed result bundle |

**Response:** `SubmitResponse`

Returns `accepted: true` on success. The result bundle is broadcast to the
job originator via the Subscribe stream.

---

### `Health`

Health check endpoint.

```protobuf
rpc Health(Empty) returns (HealthResponse);
```

**Response:** `HealthResponse`

| Field | Type | Description |
|-------|------|-------------|
| `version` | string | Server version (e.g. "0.1.0") |
| `uptime_secs` | uint64 | Seconds since server start |
| `pending_jobs` | uint64 | Count of unclaimed pending jobs |
| `connected_peers` | uint64 | Active Subscribe streams |

---

## Common Types

### `MsgType` (enum)

| Value | Name | Description |
|-------|------|-------------|
| 0 | `MSG_TYPE_BUNDLE` | Signed batch of CRDT deltas |
| 1 | `MSG_TYPE_OP` | Single real-time CRDT operation |
| 2 | `MSG_TYPE_JOB` | Distributed work item |

### `Capability`

```protobuf
message Capability {
    string kind = 1;   // e.g. "brain_mode", "gpu", "embedding_model"
    string value = 2;  // e.g. "local_ollama", "cuda", "nomic-embed-text"
}
```

A job requires ALL its listed capabilities (AND logic). A worker matches
if its capability set is a superset of the job's requirements.

---

## Payload Formats (MessagePack)

The `payload` field in `HiveEnvelope` contains MessagePack-encoded data.
The schema depends on `msg_type`:

### Bundle Payload

```rust
struct BundlePayload {
    bundle_id: String,
    hlc_from: u64,
    hlc_to: u64,
    memory_deltas: Vec<MemoryDelta>,
    edge_deltas: Vec<EdgeDelta>,
    kind_summary: Option<HashMap<String, u32>>,
}
```

### OP Payload

```rust
enum OpPayload {
    Memory(MemoryDelta),
    Edge(EdgeDelta),
}
```

### JOB Payload

```rust
struct JobPayload {
    job_type: String,          // "embed_chunks", "summarize", "extract_edges"
    capabilities: Vec<Capability>,
    input: Vec<u8>,            // Opaque to relay
    timeout_ms: u64,
    max_retries: u8,
}
```

---

## Error Codes

| gRPC Code | When |
|-----------|------|
| `UNAUTHENTICATED` | Signature verification fails |
| `INTERNAL` | Database/server error |
| `INVALID_ARGUMENT` | Malformed envelope (wrong version, bad pubkey length) |
| `NOT_FOUND` | Job not found or not in claimable state |

---

## Rate Limits

The relay enforces per-device rate limits (configurable):

| Limit | Default | Description |
|-------|---------|-------------|
| Submissions/minute | 60 | Max Submit() calls per device per minute |
| Bundle size | 64 MiB | Max payload size |
| OP queue depth | 1,000 | Max buffered OPs per device |
| Job timeout | 300,000 ms | Default job claim timeout (5 min) |
