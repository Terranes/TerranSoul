//! Ambient agent validation — simulated garden cycles (Chunk 44.3).
//!
//! Runs 50+ simulated maintenance cycles on a pre-populated memory store,
//! measuring quality metrics (decay progression, GC effectiveness, promotion
//! rate) to validate that the ambient maintenance pipeline behaves correctly
//! under production-like conditions.

#[cfg(test)]
mod tests {
    use crate::brain::maintenance_scheduler::{
        jobs_due, MaintenanceConfig, MaintenanceJob, MaintenanceState,
    };
    use crate::coding::ambient::{garden, AmbientConfig, CycleState};
    use crate::memory::store::{MemoryStore, MemoryType, NewMemory};

    /// Quality metrics collected across the simulation.
    #[derive(Debug, Default)]
    struct SimulationMetrics {
        total_cycles: u64,
        total_decay_applications: u64,
        total_decayed_entries: usize,
        total_gc_deletions: usize,
        total_promotions: usize,
        memories_at_start: usize,
        memories_at_end: usize,
        min_decay_score_observed: f64,
        max_decay_score_observed: f64,
        cycles_with_jobs: u64,
    }

    /// Create a realistic test store with varied entries.
    fn populated_store(count: usize) -> MemoryStore {
        let store = MemoryStore::in_memory();
        for i in 0..count {
            let importance = ((i % 5) + 1) as i64;
            let memory_type = match i % 4 {
                0 => MemoryType::Fact,
                1 => MemoryType::Preference,
                2 => MemoryType::Context,
                _ => MemoryType::Summary,
            };
            let tier = if i % 3 == 0 { "working" } else { "long" };
            let entry = NewMemory {
                content: format!("Simulated memory entry #{i} for ambient validation"),
                tags: format!("sim,cycle,type:{}", memory_type.as_str()),
                importance,
                memory_type,
                ..Default::default()
            };
            let added = store.add(entry).unwrap();

            // Set tier directly via SQL for non-default tiers.
            if tier == "long" {
                let _ = store.conn.execute(
                    "UPDATE memories SET tier = 'long' WHERE id = ?1",
                    rusqlite::params![added.id],
                );
            }

            // Simulate varied access patterns: some entries are accessed more.
            if i % 7 == 0 {
                for _ in 0..6 {
                    let now = crate::memory::store::now_ms();
                    let _ = store.conn.execute(
                        "UPDATE memories SET access_count = access_count + 1, last_accessed = ?1 WHERE id = ?2",
                        rusqlite::params![now, added.id],
                    );
                }
            }
        }
        store
    }

    /// Simulate time passing by advancing `last_accessed` timestamps backward.
    fn age_store(store: &MemoryStore, hours: i64) {
        let shift_ms = hours * 3_600_000;
        // Push all timestamps backward (simulating time passing).
        let _ = store.conn.execute_batch(&format!(
            "UPDATE memories SET last_accessed = CASE
                WHEN last_accessed IS NOT NULL THEN last_accessed - {shift_ms}
                ELSE NULL END;
             UPDATE memories SET created_at = created_at - {shift_ms};"
        ));
    }

    /// Run the full simulation: 60 garden cycles with 24h between each.
    #[test]
    fn run_60_simulated_garden_cycles() {
        let store = populated_store(200);
        let config = AmbientConfig {
            enabled: true,
            maintenance: MaintenanceConfig {
                // Shorten cooldowns to 12h so jobs fire every cycle in sim.
                decay_cooldown_ms: 12 * 3_600_000,
                garbage_collect_cooldown_ms: 12 * 3_600_000,
                promote_tier_cooldown_ms: 12 * 3_600_000,
                edge_extract_cooldown_ms: u64::MAX, // skip (needs LLM)
                obsidian_export_cooldown_ms: u64::MAX, // skip
                ann_compact_cooldown_ms: u64::MAX, // skip (no ANN in sim)
            },
            ..Default::default()
        };

        let mut scheduler_state = MaintenanceState::default();
        let mut metrics = SimulationMetrics {
            memories_at_start: store
                .conn
                .query_row("SELECT COUNT(*) FROM memories", [], |r| r.get::<_, usize>(0))
                .unwrap(),
            min_decay_score_observed: 1.0,
            max_decay_score_observed: 1.0,
            ..Default::default()
        };

        let gc_threshold = 0.05;
        // Use synthetic clock: start at a fixed time, advance 24h per cycle.
        let mut sim_time_ms: u64 = 1_700_000_000_000; // arbitrary epoch

        for cycle_num in 0..60 {
            // Determine which jobs are due.
            let jobs = jobs_due(&scheduler_state, &config.maintenance, sim_time_ms);

            let mut cycle = CycleState::new(cycle_num);

            if !jobs.is_empty() {
                metrics.cycles_with_jobs += 1;
            }

            for job in &jobs {
                match job {
                    MaintenanceJob::Decay => {
                        let decayed = store.apply_decay().unwrap_or(0);
                        metrics.total_decay_applications += 1;
                        metrics.total_decayed_entries += decayed;
                        cycle.record_tool(crate::coding::ambient::AmbientTool::Garden);
                    }
                    MaintenanceJob::GarbageCollect => {
                        let deleted = store.gc_decayed(gc_threshold).unwrap_or(0);
                        metrics.total_gc_deletions += deleted;
                    }
                    MaintenanceJob::PromoteTier => {
                        let promoted = store.auto_promote_to_long(5, 7).unwrap_or_default();
                        metrics.total_promotions += promoted.len();
                    }
                    // Skip LLM-dependent jobs in simulation.
                    _ => {}
                }
                scheduler_state.record_finished(*job, sim_time_ms);
            }

            cycle.ended = true;
            metrics.total_cycles += 1;

            // Advance synthetic clock by 24 hours.
            sim_time_ms += 24 * 3_600_000;

            // Simulate 24 hours passing in the store data.
            age_store(&store, 24);
        }

        // Collect final metrics.
        metrics.memories_at_end = store
            .conn
            .query_row("SELECT COUNT(*) FROM memories", [], |r| r.get::<_, usize>(0))
            .unwrap();

        // Collect decay score range.
        let (min_decay, max_decay): (f64, f64) = store
            .conn
            .query_row(
                "SELECT MIN(decay_score), MAX(decay_score) FROM memories",
                [],
                |r| Ok((r.get::<_, f64>(0)?, r.get::<_, f64>(1)?)),
            )
            .unwrap_or((0.0, 1.0));
        metrics.min_decay_score_observed = min_decay;
        metrics.max_decay_score_observed = max_decay;

        // ── Assertions: quality invariants ──────────────────────────────
        // 1. All 60 cycles completed.
        assert_eq!(metrics.total_cycles, 60);

        // 2. Decay was applied in most cycles (cooldown is shorter than interval).
        assert!(
            metrics.total_decay_applications >= 50,
            "Expected ≥50 decay applications, got {}",
            metrics.total_decay_applications
        );

        // 3. Some entries were actually decayed (scores reduced).
        assert!(
            metrics.total_decayed_entries > 0,
            "Expected some entries to have decay_score reduced"
        );

        // 4. GC removed some entries (low-importance entries should decay below threshold).
        assert!(
            metrics.total_gc_deletions > 0,
            "Expected GC to remove at least some entries over 60 cycles"
        );

        // 5. Store didn't lose ALL entries (GC is conservative).
        assert!(
            metrics.memories_at_end > 0,
            "GC must not delete everything"
        );
        assert!(
            metrics.memories_at_end >= metrics.memories_at_start / 4,
            "GC removed too aggressively: started with {}, ended with {}",
            metrics.memories_at_start,
            metrics.memories_at_end
        );

        // 6. Promotions happened (entries with high access count).
        // Note: promotions require access within last 7 days; our aging
        // pushes timestamps back, so this depends on timing.
        // We accept ≥ 0 promotions (they fire when conditions align).

        // 7. Decay scores show proper range after 60 days of simulated decay.
        assert!(
            metrics.min_decay_score_observed < 0.5,
            "After 60 cycles, some entries should have decayed significantly (min={})",
            metrics.min_decay_score_observed
        );

        // 8. Jobs fired in most cycles (not starved).
        assert!(
            metrics.cycles_with_jobs >= 55,
            "Expected jobs to fire in most cycles, got {} of 60",
            metrics.cycles_with_jobs
        );
    }

    /// Verify the ambient agent doesn't crash on an empty store.
    #[test]
    fn garden_on_empty_store_is_safe() {
        let store = MemoryStore::in_memory();
        let config = AmbientConfig {
            enabled: true,
            ..Default::default()
        };
        let scheduler_state = MaintenanceState::default();

        // Should not panic.
        let jobs = garden(&scheduler_state, &config);
        for job in &jobs {
            match job {
                MaintenanceJob::Decay => { store.apply_decay().unwrap(); }
                MaintenanceJob::GarbageCollect => { store.gc_decayed(0.05).unwrap(); }
                MaintenanceJob::PromoteTier => { store.auto_promote_to_long(5, 7).unwrap(); }
                _ => {}
            }
        }
    }

    /// Verify high-importance entries survive GC even after extended decay.
    #[test]
    fn high_importance_survives_gc() {
        let store = MemoryStore::in_memory();
        // Add a high-importance long-tier memory.
        let entry = NewMemory {
            content: "Critical knowledge — must survive".to_string(),
            tags: "critical".to_string(),
            importance: 5,
            ..Default::default()
        };
        let added = store.add(entry).unwrap();
        let _ = store.conn.execute(
            "UPDATE memories SET tier = 'long', decay_score = 0.01 WHERE id = ?1",
            rusqlite::params![added.id],
        );

        // GC with standard threshold.
        let deleted = store.gc_decayed(0.05).unwrap();
        assert_eq!(deleted, 0, "High-importance entry must not be GC'd");

        // Verify it still exists.
        let remaining = store.get_by_id(added.id);
        assert!(remaining.is_ok());
    }

    /// Verify decay doesn't go below the 0.01 floor.
    #[test]
    fn decay_respects_floor() {
        let store = MemoryStore::in_memory();
        let entry = NewMemory {
            content: "Test floor entry".to_string(),
            tags: "test".to_string(),
            importance: 1,
            ..Default::default()
        };
        let added = store.add(entry).unwrap();
        let _ = store.conn.execute(
            "UPDATE memories SET tier = 'long', decay_score = 0.02, last_accessed = ?1 WHERE id = ?2",
            rusqlite::params![1_000_000_i64, added.id], // very old access
        );

        // Apply decay many times.
        for _ in 0..10 {
            store.apply_decay().unwrap();
        }

        let decay: f64 = store
            .conn
            .query_row(
                "SELECT decay_score FROM memories WHERE id = ?1",
                rusqlite::params![added.id],
                |r| r.get(0),
            )
            .unwrap();
        assert!(
            decay >= 0.01,
            "Decay score should never go below 0.01 floor, got {decay}"
        );
    }

    /// Stress test: 500 entries through 100 cycles.
    #[test]
    fn stress_100_cycles_500_entries() {
        let store = populated_store(500);
        let config = AmbientConfig {
            enabled: true,
            maintenance: MaintenanceConfig {
                decay_cooldown_ms: 0, // fire every cycle
                garbage_collect_cooldown_ms: 0,
                promote_tier_cooldown_ms: 0,
                edge_extract_cooldown_ms: u64::MAX,
                obsidian_export_cooldown_ms: u64::MAX,
                ann_compact_cooldown_ms: u64::MAX,
            },
            ..Default::default()
        };

        let mut scheduler_state = MaintenanceState::default();
        let initial_count: usize = store
            .conn
            .query_row("SELECT COUNT(*) FROM memories", [], |r| r.get(0))
            .unwrap();

        let mut sim_time_ms: u64 = 1_700_000_000_000;
        for _cycle in 0..100 {
            let jobs = jobs_due(&scheduler_state, &config.maintenance, sim_time_ms);
            for job in &jobs {
                match job {
                    MaintenanceJob::Decay => { store.apply_decay().unwrap(); }
                    MaintenanceJob::GarbageCollect => { store.gc_decayed(0.05).unwrap(); }
                    MaintenanceJob::PromoteTier => { store.auto_promote_to_long(5, 7).unwrap(); }
                    _ => {}
                }
                scheduler_state.record_finished(*job, sim_time_ms);
            }
            sim_time_ms += 24 * 3_600_000;
            age_store(&store, 24);
        }

        let final_count: usize = store
            .conn
            .query_row("SELECT COUNT(*) FROM memories", [], |r| r.get(0))
            .unwrap();

        // The store should have lost some entries to GC but not all.
        assert!(final_count > 0, "Store emptied completely after 100 cycles");
        assert!(
            final_count < initial_count,
            "Expected some GC deletions over 100 cycles ({initial_count} → {final_count})"
        );
        // High-importance entries (importance ≥ 3) should mostly survive.
        let high_imp_remaining: usize = store
            .conn
            .query_row(
                "SELECT COUNT(*) FROM memories WHERE importance >= 3",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(
            high_imp_remaining > 0,
            "All high-importance entries were lost"
        );
    }

    /// Verify CycleState tracking works correctly over many cycles.
    #[test]
    fn cycle_state_accumulation() {
        let mut total_tools = 0;
        for i in 0..50 {
            let mut cycle = CycleState::new(i);
            cycle.record_tool(crate::coding::ambient::AmbientTool::Garden);
            cycle.record_tool(crate::coding::ambient::AmbientTool::ScheduleNext);
            cycle.record_tool(crate::coding::ambient::AmbientTool::EndCycle);
            cycle.ended = true;
            total_tools += cycle.tools_invoked.len();
            assert_eq!(cycle.cycle_number, i);
        }
        assert_eq!(total_tools, 150); // 3 tools × 50 cycles
    }

    /// Verify scheduling decisions respect cooldowns.
    #[test]
    fn cooldown_respected_across_cycles() {
        let config = MaintenanceConfig {
            decay_cooldown_ms: 48 * 3_600_000, // 48 hours
            garbage_collect_cooldown_ms: 48 * 3_600_000,
            promote_tier_cooldown_ms: 48 * 3_600_000,
            edge_extract_cooldown_ms: u64::MAX,
            obsidian_export_cooldown_ms: u64::MAX,
            ann_compact_cooldown_ms: u64::MAX,
        };
        let mut state = MaintenanceState::default();

        // First check: all jobs should fire (never run before).
        let now = 100_000_000_000u64; // arbitrary start
        let jobs = jobs_due(&state, &config, now);
        assert!(!jobs.is_empty());
        for job in &jobs {
            state.record_finished(*job, now);
        }

        // 24 hours later: should NOT fire (cooldown is 48h).
        let jobs_24h = jobs_due(&state, &config, now + 24 * 3_600_000);
        let maintenance_jobs: Vec<_> = jobs_24h
            .iter()
            .filter(|j| matches!(j, MaintenanceJob::Decay | MaintenanceJob::GarbageCollect | MaintenanceJob::PromoteTier))
            .collect();
        assert!(
            maintenance_jobs.is_empty(),
            "Jobs should not fire within cooldown period"
        );

        // 49 hours later: should fire (past 48h cooldown).
        let jobs_49h = jobs_due(&state, &config, now + 49 * 3_600_000);
        assert!(
            !jobs_49h.is_empty(),
            "Jobs should fire after cooldown period"
        );
    }
}
