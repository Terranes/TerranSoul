"""Apply the latest memory-seed.sql tail to the live MCP database.

Idempotent — uses INSERT OR IGNORE on content. Safe to run while MCP is
serving (SQLite handles concurrent readers/writers via WAL).
"""
import sqlite3
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DB = ROOT / "mcp-data" / "memory.db"
SEED = ROOT / "mcp-data" / "shared" / "memory-seed.sql"

MARKER = "-- Phase 38 (Million-memory MCP) — chunks 38.3, 38.4 + UI/UX redesign"


def main() -> int:
    if not DB.exists():
        print(f"DB not found: {DB}", file=sys.stderr)
        return 1
    text = SEED.read_text(encoding="utf-8")
    idx = text.find(MARKER)
    if idx < 0:
        print(f"marker not found in {SEED}", file=sys.stderr)
        return 2
    delta = text[idx:]
    conn = sqlite3.connect(str(DB))
    try:
        before = conn.execute("SELECT COUNT(*) FROM memories").fetchone()[0]
        conn.executescript(delta)
        conn.commit()
        after = conn.execute("SELECT COUNT(*) FROM memories").fetchone()[0]
        print(f"memories: {before} -> {after} (+{after - before})")
    finally:
        conn.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
