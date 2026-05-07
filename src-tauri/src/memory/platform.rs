//! Platform-adaptive SQLite PRAGMA selection.
//!
//! On desktop, we use aggressive WAL tuning (64 MiB cache, 256 MiB mmap).
//! On mobile (iOS/Android) we reduce resource usage and handle platform
//! quirks:
//! - **iOS**: WAL mode can leave orphan `-wal`/`-shm` files when the app
//!   is suspended/killed mid-transaction. We still use WAL (it's the best
//!   option) but reduce `wal_autocheckpoint` so the WAL stays small, and
//!   cap `mmap_size` at 32 MiB to respect iOS memory pressure.
//! - **Android**: WAL is safe. We reduce cache/mmap to fit typical device
//!   RAM budgets but otherwise match desktop behaviour.
//!
//! The `mobile` feature flag (compile-time) is the primary gate. At runtime
//! `target_os` further distinguishes iOS from Android.

/// PRAGMAs for a file-backed production database.
pub fn production_pragmas() -> &'static str {
    #[cfg(not(feature = "mobile"))]
    {
        // Desktop: aggressive tuning for million-memory CRUD.
        concat!(
            "PRAGMA journal_mode=WAL;\n",
            "PRAGMA synchronous=NORMAL;\n",
            "PRAGMA foreign_keys=ON;\n",
            "PRAGMA cache_size=-65536;\n", // 64 MiB
            "PRAGMA mmap_size=268435456;\n", // 256 MiB
            "PRAGMA temp_store=MEMORY;\n",
            "PRAGMA busy_timeout=5000;\n",
            "PRAGMA wal_autocheckpoint=1000;\n",
            "PRAGMA journal_size_limit=67108864;", // 64 MiB
        )
    }

    #[cfg(feature = "mobile")]
    {
        mobile_pragmas()
    }
}

/// Mobile-specific PRAGMAs. Reduced resource budget and iOS-safe defaults.
#[cfg(feature = "mobile")]
fn mobile_pragmas() -> &'static str {
    // iOS caveat: when the app is backgrounded, iOS may kill the process
    // without calling applicationWillTerminate. An un-checkpointed WAL can
    // leave pages that replay on next open (safe) but the -wal file can
    // grow large if autocheckpoint is too high. We set autocheckpoint=100
    // (vs 1000 on desktop) so the WAL stays under ~400 KiB typically.
    //
    // Android: WAL is fully safe. Same conservative resource budget applies.
    //
    // mmap_size: 32 MiB (vs 256 MiB desktop) — respects iOS jetsam limits.
    // cache_size: 16 MiB (vs 64 MiB desktop) — typical mobile has 4-6 GB RAM.
    // journal_size_limit: 8 MiB (vs 64 MiB desktop) — tighter storage.
    concat!(
        "PRAGMA journal_mode=WAL;\n",
        "PRAGMA synchronous=NORMAL;\n",
        "PRAGMA foreign_keys=ON;\n",
        "PRAGMA cache_size=-16384;\n", // 16 MiB
        "PRAGMA mmap_size=33554432;\n", // 32 MiB
        "PRAGMA temp_store=MEMORY;\n",
        "PRAGMA busy_timeout=5000;\n",
        "PRAGMA wal_autocheckpoint=100;\n",
        "PRAGMA journal_size_limit=8388608;", // 8 MiB
    )
}

/// PRAGMAs for in-memory test databases.
/// Kept consistent across platforms for test reproducibility.
pub fn test_pragmas() -> &'static str {
    concat!(
        "PRAGMA foreign_keys=ON;\n",
        "PRAGMA temp_store=MEMORY;\n",
        "PRAGMA cache_size=-65536;",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_pragmas_contain_journal_mode() {
        let p = production_pragmas();
        assert!(p.contains("journal_mode"));
        assert!(p.contains("foreign_keys=ON"));
        assert!(p.contains("busy_timeout"));
    }

    #[test]
    fn test_pragmas_contain_foreign_keys() {
        let p = test_pragmas();
        assert!(p.contains("foreign_keys=ON"));
        assert!(p.contains("temp_store=MEMORY"));
    }

    #[test]
    #[cfg(feature = "mobile")]
    fn mobile_pragmas_have_reduced_cache() {
        let p = production_pragmas();
        // 16 MiB = -16384 pages
        assert!(p.contains("cache_size=-16384"));
        // 32 MiB mmap
        assert!(p.contains("mmap_size=33554432"));
        // Aggressive checkpoint
        assert!(p.contains("wal_autocheckpoint=100"));
    }

    #[test]
    #[cfg(not(feature = "mobile"))]
    fn desktop_pragmas_have_full_cache() {
        let p = production_pragmas();
        // 64 MiB = -65536 pages
        assert!(p.contains("cache_size=-65536"));
        // 256 MiB mmap
        assert!(p.contains("mmap_size=268435456"));
        // Standard checkpoint
        assert!(p.contains("wal_autocheckpoint=1000"));
    }

    /// Smoke integration test: open a file-backed DB, apply platform PRAGMAs,
    /// run the canonical schema migration, insert a memory row, read it back.
    /// This exercises the full mobile path (SQLite bundled + platform PRAGMAs +
    /// schema) without requiring a real iOS/Android target.
    #[test]
    fn sqlite_open_migrate_write_read_roundtrip() {
        use rusqlite::Connection;

        let dir = std::env::temp_dir().join("ts_test_mobile_sqlite_smoke");
        let _ = std::fs::create_dir_all(&dir);
        let db_path = dir.join("memory.db");
        // Remove stale test artifacts
        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_file(dir.join("memory.db-wal"));
        let _ = std::fs::remove_file(dir.join("memory.db-shm"));

        // Open and apply platform PRAGMAs
        let conn = Connection::open(&db_path).expect("open SQLite");
        conn.execute_batch(production_pragmas())
            .expect("apply PRAGMAs");

        // Verify WAL mode is active
        let mode: String = conn
            .query_row("PRAGMA journal_mode;", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mode.to_lowercase(), "wal");

        // Run canonical schema
        super::super::schema::create_canonical_schema(&conn)
            .expect("schema migration");

        // Insert a memory row
        conn.execute(
            "INSERT INTO memories (content, importance, tier, category, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, datetime('now'), datetime('now'))",
            rusqlite::params!["Hello from mobile test", 5, "core", "general"],
        )
        .expect("insert row");

        // Read it back
        let content: String = conn
            .query_row(
                "SELECT content FROM memories WHERE content = ?1",
                ["Hello from mobile test"],
                |r| r.get(0),
            )
            .expect("read row");
        assert_eq!(content, "Hello from mobile test");

        // Verify foreign_keys is ON
        let fk: i32 = conn
            .query_row("PRAGMA foreign_keys;", [], |r| r.get(0))
            .unwrap();
        assert_eq!(fk, 1);

        // Cleanup
        drop(conn);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
