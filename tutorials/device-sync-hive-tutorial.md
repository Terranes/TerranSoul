# Device Sync & Hive Federation — Cross-Device Memory, Privacy ACLs & Distributed Jobs

> **TerranSoul v0.1** · Last updated: 2026-05-07
>
> Related: [Hive Relay Self-Hosting](hive-relay-tutorial.md) ·
> [LAN Brain Sharing](lan-mcp-sharing-tutorial.md) ·
> [Advanced Memory & RAG](advanced-memory-rag-tutorial.md)

TerranSoul can sync memories between your own devices via peer-to-peer
CRDT replication, and optionally participate in a federated “Hive” for
shared knowledge and distributed AI jobs. This tutorial covers device
pairing, memory sync, the Hive relay, privacy controls, and job
distribution.

---

## Table of Contents

1. [Device Pairing (Soul Link)](#1-device-pairing-soul-link)
2. [Memory Sync (CRDT Replication)](#2-memory-sync-crdt-replication)
3. [Privacy Controls (Share Scope ACL)](#3-privacy-controls-share-scope-acl)
4. [Hive Federation (Opt-in Relay)](#4-hive-federation-opt-in-relay)
5. [Distributed Jobs](#5-distributed-jobs)
6. [Security Model](#6-security-model)
7. [Troubleshooting](#7-troubleshooting)

---

## Requirements

| Requirement | Notes |
|---|---|
| **Two+ devices** | Both running TerranSoul (desktop, mobile, or browser) |
| **Same network** | For peer-to-peer sync (LAN); or a Hive relay for internet sync |
| **Hive relay** (optional) | Self-hosted via `docker-compose` or a community relay URL |

---

## 1. Device Pairing (Soul Link)

![Settings → Devices panel showing QR code for pairing](screenshots/device-sync/01-pairing-qr.png)

### Step 1: Enable LAN on the Host Device

1. Open **Settings → Network**.
2. Enable **"LAN Sharing"** toggle (`lan_enabled: true`).
3. TerranSoul binds a QUIC server and displays your local IP.

### Step 2: Generate a Pairing QR Code

1. On the host device, navigate to **Settings → Devices** (or the Mobile Pairing view).
2. Click **"Pair New Device"**.
3. A QR code appears containing a `terransoul://pair?...` URI with:
   - Host address and port
   - One-time pairing token
   - Device identity public key

### Step 3: Scan on the Second Device

1. On the second device (phone or desktop), open **Settings → Devices → "Scan QR"**.
2. Scan the QR code (or manually enter the pairing URI).
3. The devices exchange Ed25519 public keys and establish a secure channel.
4. Both devices show "Paired ✓" in the device list.

### Step 4: Verify Pairing

- The Link status shows `connected` with the peer's device name.
- Transport: QUIC (primary) with WebSocket fallback.
- You can now sync memories, conversations, and settings.

---

## 2. Memory Sync (CRDT Replication)

![Two devices showing synced memory entries with connected status](screenshots/device-sync/02-memory-sync.png)

Once paired, memories sync automatically:

### How It Works

1. **LWW (Last-Writer-Wins) for memory rows** — Each memory carries a Hybrid Logical Clock (HLC) timestamp + `origin_device` ID.
2. **2P-Set for knowledge graph edges** — Edges use add/remove sets with `valid_to` as tombstone.
3. **Op-log replication** — Changes are sent as operations over the QUIC/WebSocket link.
4. **Conflict handling** — When the same memory is edited on two devices:
   - The HLC determines the winner (higher timestamp wins).
   - The loser is archived in `memory_versions` (never lost).
   - Conflicts are surfaced in the Brain View for manual review.

### What Syncs

| Data | Sync Method |
|------|-------------|
| Long-term memories | LWW CRDT (automatic) |
| Knowledge graph edges | 2P-Set CRDT (automatic) |
| Conversations | Full sync after reconnect |
| Settings | Last-writer-wins |
| Persona traits | Merged (union of expressions/motions) |

### What Doesn't Sync

- Working/short-tier memories (session-scoped)
- Embeddings (regenerated locally per device)
- Plugin state (device-specific)

---

## 3. Privacy Controls (Share Scope ACL)

![Memory detail panel showing Share Scope dropdown — Private / Paired / Hive](screenshots/device-sync/03-privacy-controls.png)

Every memory has a `share_scope` that controls where it can travel:

| Scope | Meaning | Leaves Device? | Syncs to Paired? | Goes to Hive? |
|-------|---------|---------------|------------------|---------------|
| **Private** | Never leaves this device | ❌ | ❌ | ❌ |
| **Paired** | Syncs to your own devices only | ✓ | ✓ | ❌ |
| **Hive** | Can be shared with the relay | ✓ | ✓ | ✓ |

### Default Scopes by Cognitive Kind

| Cognitive Kind | Default Scope |
|---|---|
| Episodic (personal events) | Private |
| Semantic (factual knowledge) | Paired |
| Procedural (how-to) | Paired |
| Judgment (rules/policies) | Private |

### Changing a Memory's Scope

1. Open the **Memory** tab.
2. Click a memory entry.
3. In the detail panel, change the **Share Scope** dropdown.
4. Changes take effect immediately — on the next sync cycle, the privacy filter applies.

### Privacy Guarantee

The privacy engine (`hive/privacy.rs`) enforces:

- **Private memories NEVER appear in outbound bundles** — even if a bug in sync code tries to include them.
- **Edges are excluded if either endpoint is private** — no information leakage through graph structure.
- **Empty bundles after filtering return `None`** — no accidental empty submissions to the relay.

---

## 4. Hive Federation (Opt-in Relay)

![Settings → Network showing Hive URL field and connection status](screenshots/device-sync/04-hive-connect.png)

The Hive is a **fully optional** federation layer for sharing knowledge across users or teams.

### Step 1: Set Up a Hive Relay

**Option A — Self-host with Docker:**

```bash
cd crates/hive-relay
docker-compose up -d
```

This starts:
- PostgreSQL (data storage)
- Hive Relay server (gRPC on port 50051)

**Option B — Use a community relay:**

Enter the relay URL in Settings (provided by the relay operator).

### Step 2: Connect Your App

1. Open **Settings → Network**.
2. Enter the **Hive URL** (e.g., `http://relay.example.com:50051`).
3. TerranSoul will:
   - Sign bundles with your Ed25519 device identity
   - Submit `hive`-scoped memories to the relay
   - Subscribe to incoming bundles from other participants
   - Pull available jobs

### Step 3: Understand Hive Messages

| Message Type | Purpose |
|---|---|
| **BUNDLE** | Signed batch of memories + edges (up to 10,000 memories, 64 MiB max) |
| **OP** | Single CRDT operation (real-time edge add/remove) |
| **JOB** | Work request + required capabilities |

All messages are **Ed25519 signed** by the originating device. The relay verifies signatures before accepting or forwarding.

---

## 5. Distributed Jobs

![Job queue showing submitted work and capability matching](screenshots/device-sync/05-distributed-jobs.png)

Hive supports distributing AI work across participants:

### How It Works

1. A device submits a **JOB** with required capabilities (e.g., `["brain:local_ollama", "gpu:8gb", "model:nomic-embed-text"]`).
2. The relay holds the job in a queue.
3. Another device with matching capabilities **claims** the job (using SQL `SKIP LOCKED` for fairness).
4. The worker executes locally and returns results as a **BUNDLE**.
5. The originator receives the completed bundle.

### Capability Matching

Jobs require ALL listed capabilities (AND logic):

```
Job requires: ["brain:local_ollama", "embedding:nomic-embed-text"]
Worker has:   ["brain:local_ollama", "embedding:nomic-embed-text", "gpu:rtx4090"]
→ Match ✓ (worker has a superset)
```

### Self-Jobs (Local Fallback)

If your device has the required capabilities, the job executes locally without touching the relay. This is the default for most operations — the relay is only used when local execution isn't possible.

---

## 6. Security Model

![Architecture diagram showing Ed25519 signing flow between devices and relay](screenshots/device-sync/06-security-model.png)

| Layer | Protection |
|-------|-----------|
| **Bundle signing** | Ed25519 (same keys used for device identity) |
| **Replay prevention** | HLC timestamps + sender dedup on relay |
| **Wire format** | MessagePack + optional LZ4 compression |
| **Transport** | gRPC (TLS recommended for production relays) |
| **Privacy** | Client-side filtering — relay never sees private memories |

### Sign Input Format

```
version(1 byte) ∥ msg_type(1 byte) ∥ sender_id(variable) ∥ 
timestamp(8 bytes LE) ∥ hlc_counter(8 bytes LE) ∥ payload(variable)
```

---

## 7. Troubleshooting

| Problem | Solution |
|---------|----------|
| Devices not discovering each other | Ensure both are on the same LAN subnet. Check firewall allows UDP broadcast. |
| Sync not happening | Verify Link status shows "connected". Check if memories are `paired` or `hive` scope (not `private`). |
| Hive relay unreachable | Test: `curl http://relay:50051/health`. Check Docker logs if self-hosting. |
| Signature rejected by relay | Device identity may have been regenerated. Re-pair with the relay. |
| Private memory appeared remotely | This should never happen — file a bug. The privacy filter has a hard guarantee. |

---

## Where to Go Next

- **[LAN Brain Sharing](lan-mcp-sharing-tutorial.md)** — Share your brain with coding agents on the local network
- **[Advanced Memory & RAG](advanced-memory-rag-tutorial.md)** — Understand the memory system being synced
- **[MCP for Coding Agents](mcp-coding-agents-tutorial.md)** — Connect coding assistants to your synced brain
