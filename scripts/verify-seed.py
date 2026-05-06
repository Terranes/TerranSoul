import sqlite3
c = sqlite3.connect("mcp-data/memory.db")
rows = c.execute(
    "SELECT id, substr(content,1,90) FROM memories "
    "WHERE content LIKE 'CHUNK 38%' "
    "   OR content LIKE 'SCHEMA UPDATE (2026-05-07)%' "
    "   OR content LIKE 'UI/UX PREMIUM%' "
    "   OR content LIKE 'RULE (user expectation%' "
    "ORDER BY id DESC"
).fetchall()
for r in rows:
    print(r)
print(f"total memories: {c.execute('SELECT COUNT(*) FROM memories').fetchone()[0]}")
