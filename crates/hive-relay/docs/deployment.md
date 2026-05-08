# Hive Relay — Deployment Guide

## Quick Start (Docker)

The fastest way to run a Hive Relay:

```bash
cd crates/hive-relay
docker compose up -d
```

This starts:
- **PostgreSQL 16** (with pgvector) on port 5433
- **Hive Relay** (gRPC) on port 50051

Verify:

```bash
# Using grpcurl (install: brew install grpcurl)
grpcurl -plaintext localhost:50051 hive.HiveRelay/Health
```

---

## Docker Compose (Production)

### `docker-compose.yml`

```yaml
services:
  postgres:
    image: pgvector/pgvector:pg16
    environment:
      POSTGRES_DB: hive_relay
      POSTGRES_USER: hive
      POSTGRES_PASSWORD: ${HIVE_DB_PASSWORD:-change_me_in_production}
    ports:
      - "5433:5432"
    volumes:
      - hive_pg_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U hive -d hive_relay"]
      interval: 5s
      timeout: 3s
      retries: 5

  relay:
    build:
      context: .
      dockerfile: Dockerfile
    environment:
      DATABASE_URL: postgres://hive:${HIVE_DB_PASSWORD:-change_me_in_production}@postgres:5432/hive_relay
      LISTEN_ADDR: 0.0.0.0:50051
      RUST_LOG: hive_relay=info
    ports:
      - "50051:50051"
    depends_on:
      postgres:
        condition: service_healthy

volumes:
  hive_pg_data:
```

### Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `DATABASE_URL` | Yes | — | PostgreSQL connection URL |
| `LISTEN_ADDR` | No | `0.0.0.0:50051` | gRPC bind address |
| `RUST_LOG` | No | `hive_relay=info` | Tracing filter directive |
| `HIVE_DB_PASSWORD` | No | `change_me_in_production` | Postgres password |

---

## Bare-Metal Deployment

### Prerequisites

- Rust toolchain (1.80+)
- PostgreSQL 16+ with `pgvector` extension
- protoc (Protocol Buffers compiler)

### Build

```bash
cd crates/hive-relay
cargo build --release
```

The binary is at `target/release/hive-relay`.

### Run

```bash
export DATABASE_URL="postgres://hive:password@localhost:5432/hive_relay"
export LISTEN_ADDR="0.0.0.0:50051"
export RUST_LOG="hive_relay=info"

./target/release/hive-relay
```

### Systemd Service (Linux)

```ini
[Unit]
Description=TerranSoul Hive Relay
After=postgresql.service
Requires=postgresql.service

[Service]
Type=simple
User=hive
Environment=DATABASE_URL=postgres://hive:password@localhost:5432/hive_relay
Environment=LISTEN_ADDR=0.0.0.0:50051
Environment=RUST_LOG=hive_relay=info
ExecStart=/opt/hive-relay/hive-relay
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

---

## TLS Configuration

For production, always use TLS. Options:

### Option A: Reverse Proxy (Recommended)

Use nginx, Caddy, or Traefik as a TLS-terminating reverse proxy:

```nginx
# nginx config
server {
    listen 443 ssl http2;
    server_name hive.yourdomain.com;

    ssl_certificate /etc/letsencrypt/live/hive.yourdomain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/hive.yourdomain.com/privkey.pem;

    location / {
        grpc_pass grpc://127.0.0.1:50051;
    }
}
```

### Option B: Native TLS (Tonic)

Set environment variables for Tonic's built-in TLS:

```bash
export TLS_CERT=/path/to/cert.pem
export TLS_KEY=/path/to/key.pem
```

(Requires code changes to enable `tonic::transport::ServerTlsConfig` — 
currently the relay runs plain gRPC and expects a reverse proxy for TLS.)

---

## Database Setup (Manual)

If not using Docker for Postgres:

```sql
-- Create database and user
CREATE USER hive WITH PASSWORD 'your_secure_password';
CREATE DATABASE hive_relay OWNER hive;

-- Connect to hive_relay and enable pgvector
\c hive_relay
CREATE EXTENSION IF NOT EXISTS vector;
```

The relay runs schema migrations automatically on startup (`db.migrate()`).

---

## Monitoring

### Health Check

```bash
# gRPC health check
grpcurl -plaintext localhost:50051 hive.HiveRelay/Health

# Response:
# {
#   "version": "0.1.0",
#   "uptimeSecs": "3600",
#   "pendingJobs": "2",
#   "connectedPeers": "5"
# }
```

### Logs

The relay uses `tracing` with structured JSON-compatible output:

```bash
# Show all relay logs
RUST_LOG=hive_relay=debug ./hive-relay

# Only show warnings
RUST_LOG=hive_relay=warn ./hive-relay
```

### Database Metrics

```sql
-- Bundle count
SELECT COUNT(*) FROM hive_bundles;

-- Active devices (by watermark)
SELECT device_id, highest_hlc FROM hive_hlc_watermarks ORDER BY highest_hlc DESC;

-- Job queue status
SELECT status, COUNT(*) FROM hive_jobs GROUP BY status;

-- Storage usage
SELECT pg_size_pretty(pg_database_size('hive_relay'));
```

---

## Scaling Considerations

### Single Relay (Recommended for most deployments)

The relay is lightweight — a single instance handles thousands of devices:
- gRPC + tokio async handles high concurrency
- PostgreSQL `SKIP LOCKED` ensures fair job distribution
- Broadcast channel delivers real-time OPs with minimal overhead

### Multi-Relay (Future)

For very large deployments (10k+ devices), horizontal scaling options:
- **Stateless relay + shared Postgres** — multiple relay instances pointing
  at the same DB (works today with external load balancer)
- **Partitioned by hive group** — route different groups to different relays
- **Redis pub/sub** — replace in-memory broadcast with Redis for cross-relay fanout

---

## Backup & Recovery

### Database Backup

```bash
# Full logical backup
pg_dump -U hive hive_relay > hive_backup_$(date +%Y%m%d).sql

# Restore
psql -U hive hive_relay < hive_backup_20260507.sql
```

### Data Retention

Bundles accumulate over time. Implement a retention policy:

```sql
-- Delete bundles older than 90 days
DELETE FROM hive_bundles WHERE received_at < NOW() - INTERVAL '90 days';

-- Delete completed jobs older than 30 days
DELETE FROM hive_jobs WHERE status = 'completed' AND completed_at < NOW() - INTERVAL '30 days';
```

---

## Troubleshooting

| Issue | Diagnosis | Fix |
|-------|-----------|-----|
| Relay won't start | Check `DATABASE_URL` is reachable | Verify Postgres is running; test with `psql` |
| "Migration failed" | Schema conflict | Drop and recreate the database (dev only) |
| Signature rejected | Client/relay version mismatch | Ensure both use protocol version 1 |
| No jobs claimed | Capability mismatch | Check worker advertises required capabilities |
| High memory usage | Too many subscribers | Increase broadcast channel capacity or add rate limits |
| Slow job claims | Table bloat | Run `VACUUM ANALYZE hive_jobs;` |
