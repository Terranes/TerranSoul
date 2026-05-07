"""Apply pending numbered MCP seed migrations to the live MCP database.

Safe to run while MCP is serving (SQLite handles concurrent readers/writers
via WAL). This mirrors the Rust seed_migrations runner closely enough for
coding-agent sessions that need live MCP knowledge updated without restarting
the MCP terminal.
"""

from __future__ import annotations

import argparse
import re
import sqlite3
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DB = ROOT / "mcp-data" / "memory.db"
MIGRATIONS = ROOT / "mcp-data" / "shared" / "migrations"

MIGRATION_RE = re.compile(r"^(\d{3})_(.+)\.sql$")


def fnv1a_hex(data: bytes) -> str:
    hash_value = 0xCBF29CE484222325
    for byte in data:
        hash_value ^= byte
        hash_value = (hash_value * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"{hash_value:016x}"


def sql_quote(value: str) -> str:
    return "'" + value.replace("'", "''") + "'"


def discover_migrations() -> list[tuple[int, str, str, str]]:
    migrations: list[tuple[int, str, str, str]] = []
    for path in MIGRATIONS.glob("*.sql"):
        match = MIGRATION_RE.match(path.name)
        if not match:
            continue
        version = int(match.group(1))
        name = match.group(2)
        sql = path.read_text(encoding="utf-8")
        checksum = fnv1a_hex(sql.encode("utf-8"))
        migrations.append((version, name, sql, checksum))
    migrations.sort(key=lambda item: item[0])
    return migrations


def ensure_migration_table(conn: sqlite3.Connection) -> None:
    conn.execute(
        """
        CREATE TABLE IF NOT EXISTS seed_migrations (
            version     INTEGER PRIMARY KEY,
            name        TEXT    NOT NULL,
            applied_at  INTEGER NOT NULL,
            checksum    TEXT    NOT NULL
        )
        """
    )


def current_version(conn: sqlite3.Connection) -> int:
    ensure_migration_table(conn)
    row = conn.execute("SELECT COALESCE(MAX(version), 0) FROM seed_migrations").fetchone()
    return int(row[0]) if row else 0


def memory_count(conn: sqlite3.Connection) -> int:
    row = conn.execute("SELECT COUNT(*) FROM memories").fetchone()
    return int(row[0]) if row else 0


def apply_migration(
    conn: sqlite3.Connection, version: int, name: str, sql: str, checksum: str
) -> None:
    now_ms = int(time.time() * 1000)
    record_sql = (
        "INSERT OR REPLACE INTO seed_migrations (version, name, applied_at, checksum) "
        f"VALUES ({version}, {sql_quote(name)}, {now_ms}, {sql_quote(checksum)});"
    )
    script = f"BEGIN IMMEDIATE;\n{sql}\n{record_sql}\nCOMMIT;"
    try:
        conn.executescript(script)
    except sqlite3.Error:
        try:
            conn.executescript("ROLLBACK;")
        except sqlite3.Error:
            pass
        raise


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--db", type=Path, default=DB, help="Path to live memory.db")
    parser.add_argument("--dry-run", action="store_true", help="List pending migrations only")
    parser.add_argument(
        "--allow-replay-from-zero",
        action="store_true",
        help="Allow applying migrations 001+ to a populated DB with no tracking rows",
    )
    args = parser.parse_args()

    if not args.db.exists():
        print(f"DB not found: {args.db}", file=sys.stderr)
        return 1

    migrations = discover_migrations()
    if not migrations:
        print(f"No migrations found in {MIGRATIONS}", file=sys.stderr)
        return 2

    conn = sqlite3.connect(str(args.db))
    try:
        conn.execute("PRAGMA busy_timeout=5000")
        initial_version = current_version(conn)
        count = memory_count(conn)
        if initial_version == 0 and count > 0 and not args.allow_replay_from_zero:
            print(
                "Refusing to replay migrations from v000 against a populated DB. "
                "Start MCP once to bootstrap tracking, or pass --allow-replay-from-zero if intentional.",
                file=sys.stderr,
            )
            return 3

        pending = [migration for migration in migrations if migration[0] > initial_version]
        print(f"current_version={initial_version:03}; pending={len(pending)}; memories={count}")

        if args.dry_run:
            for version, name, _sql, checksum in pending:
                print(f"would apply v{version:03}_{name} ({checksum})")
            return 0

        for version, name, sql, checksum in pending:
            apply_migration(conn, version, name, sql, checksum)
            print(f"applied v{version:03}_{name} ({checksum})")

        final_version = current_version(conn)
        final_count = memory_count(conn)
        print(
            f"done: version {initial_version:03} -> {final_version:03}; "
            f"memories {count} -> {final_count}"
        )
    finally:
        conn.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
