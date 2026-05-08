# Hive Relay — Getting Started Tutorial

> Set up your own Hive Relay and connect two TerranSoul instances in
> under 10 minutes.

---

## What You'll Build

By the end of this tutorial you'll have:
- A running Hive Relay server (Docker)
- Two TerranSoul instances connected to it
- A shared memory flowing from Device A → Relay → Device B
- A distributed embedding job dispatched and completed

---

## Prerequisites

| Tool | Version | Check |
|------|---------|-------|
| Docker + Docker Compose | 24+ | `docker compose version` |
| TerranSoul app | Latest build | Running on two machines or two instances |
| (Optional) grpcurl | Any | `grpcurl --version` — for manual API testing |

---

## Step 1: Start the Relay

```bash
# From the TerranSoul repo root:
cd crates/hive-relay
docker compose up -d
```

Wait for both services to be healthy:

```bash
docker compose ps
# NAME        SERVICE    STATUS
# postgres    postgres   healthy
# relay       relay      running (port 50051)
```

Verify the relay is responding:

```bash
# If you have grpcurl:
grpcurl -plaintext localhost:50051 hive.HiveRelay/Health

# Expected output:
# {
#   "version": "0.1.0",
#   "pendingJobs": "0",
#   "connectedPeers": "0"
# }
```

---

## Step 2: Connect Device A

1. Open TerranSoul on your first device (or first instance).
2. Go to **Settings → Network**.
3. In the **Hive Relay** section, enter:
   - **URL:** `http://localhost:50051` (or your server IP)
4. Click **Connect**.
5. The status should change to **"Connected ✓"**.

Behind the scenes, TerranSoul:
- Sends a `Subscribe()` stream to the relay
- Advertises this device's capabilities (brain mode, embedding model, etc.)
- Starts receiving any bundles submitted by other devices

---

## Step 3: Connect Device B

Repeat the same on your second device/instance, using the same relay URL.

Both devices are now connected to the relay and will receive each other's
hive-scoped bundles.

---

## Step 4: Share a Memory

On **Device A**:

1. Go to the **Memory** tab.
2. Create a new memory (or select an existing one).
3. Set its **Share Scope** to **"Hive"** (dropdown in the detail panel).
4. Wait for the next sync cycle (automatic, ~30 seconds) or trigger manually
   via **Settings → Network → Sync Now**.

What happens:
1. TerranSoul builds a `Bundle` containing the hive-scoped memory.
2. Signs it with Device A's Ed25519 key.
3. Submits via `Submit()` gRPC call to the relay.
4. Relay verifies signature → stores in Postgres → broadcasts.
5. Device B receives the bundle via its `Subscribe()` stream.
6. Device B verifies signature → applies CRDT delta → memory appears locally.

On **Device B**:
- The memory should appear in the Memory tab within seconds.
- It will show the originator device name.
- It's tagged with `share_scope = hive`.

---

## Step 5: Dispatch a Distributed Job

This example offloads embedding work from a device without Ollama to one
that has it.

### On Device B (no local Ollama):

1. Go to **Settings → Brain** and confirm no local model is configured.
2. Create several memories without embeddings.
3. TerranSoul will automatically create a JOB:
   - Type: `embed_chunks`
   - Required capability: `embedding_model:nomic-embed-text`
4. The job is submitted to the relay.

### On Device A (has Ollama + nomic-embed-text):

1. Device A advertised `embedding_model:nomic-embed-text` on connect.
2. It periodically calls `ClaimJob()` and picks up the embedding task.
3. Runs embeddings locally using Ollama.
4. Returns results via `CompleteJob()`.
5. Device B receives the embedding vectors and applies them.

You can verify job activity in the relay:

```bash
# Check pending jobs
grpcurl -plaintext localhost:50051 hive.HiveRelay/Health
# "pendingJobs": "0" (if claimed already)
```

---

## Step 6: Verify Privacy

Test that private memories are never shared:

1. On Device A, create a memory with **Share Scope = "Private"**.
2. Wait for a sync cycle.
3. On Device B, search for that memory — **it should NOT appear**.
4. Check the relay database directly:

```bash
docker compose exec postgres psql -U hive hive_relay -c \
  "SELECT bundle_id, sender_id FROM hive_bundles ORDER BY received_at DESC LIMIT 5;"
```

The private memory will never appear in any bundle payload.

---

## Understanding the Data Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                        HIVE DATA FLOW                            │
│                                                                  │
│  Device A                  Relay                   Device B      │
│  ─────────                 ─────                   ─────────     │
│                                                                  │
│  Memory created            ┌─────────────┐                      │
│  (scope=hive)              │             │                       │
│       │                    │  PostgreSQL  │                       │
│       ▼                    │  ┌─────────┐│                       │
│  Build Bundle              │  │ bundles ││                       │
│       │                    │  │ jobs    ││                       │
│       ▼                    │  │ hlc_wm  ││                       │
│  Sign (Ed25519)            │  └─────────┘│                       │
│       │                    │             │                       │
│       ▼                    └──────┬──────┘                       │
│  Submit() ─────────────────────→  │                              │
│                    Verify sig ←───┘                              │
│                    Store bundle                                   │
│                    Broadcast  ─────────────────→  Receive         │
│                                                     │            │
│                                                     ▼            │
│                                                 Verify sig       │
│                                                 Apply CRDT       │
│                                                 Memory appears!  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Cleanup

Stop the relay:

```bash
cd crates/hive-relay
docker compose down
```

Remove data (fresh start):

```bash
docker compose down -v   # -v removes the postgres volume
```

---

## Next Steps

- **[Architecture](architecture.md)** — Deep dive into the protocol design
- **[API Reference](api-reference.md)** — Full gRPC endpoint documentation
- **[Deployment Guide](deployment.md)** — Production setup with TLS, backups, monitoring
- **[Device Sync Tutorial](../../../tutorials/device-sync-hive-tutorial.md)** — Full user-facing guide including LAN pairing
- **[Hive Protocol Spec](../../../docs/hive-protocol.md)** — Formal protocol specification

---

## FAQ

**Q: Can I run the relay without Docker?**
Yes — see the [Deployment Guide](deployment.md) for bare-metal instructions.

**Q: Is the relay required for device-to-device sync?**
No. Paired devices on the same LAN sync directly via QUIC/WebSocket (Soul Link).
The relay is only needed for internet-distance sync or multi-user sharing.

**Q: What if the relay goes down?**
Devices continue working independently. When the relay comes back, they
reconnect with their `since_hlc` cursor and catch up on missed bundles.
No data is lost.

**Q: Can I run multiple relays?**
Yes — multiple relay instances can point at the same PostgreSQL database.
Use a load balancer to distribute connections.

**Q: How much storage does the relay use?**
Roughly 1 KB per memory delta in a bundle. 10,000 memories ≈ 10 MB per bundle.
With retention policies, a relay serving a small team uses < 1 GB/month.
