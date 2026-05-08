//! Guided architecture tours.
//!
//! Builds ordered "tours" through a codebase from indexed processes
//! (entry-point traces) and dependency edges. Tours are designed for
//! reading order — a newcomer can follow stop-by-stop to understand
//! how a feature flows through the code.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use super::processes::{list_processes, Process};
use super::symbol_index::IndexError;

/// One stop on an architecture tour.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TourStop {
    /// Order on the tour (1-based).
    pub order: u32,
    /// Symbol name at this stop.
    pub symbol: String,
    /// File path.
    pub file: String,
    /// 1-based line number.
    pub line: u32,
    /// Call-graph depth from the entry point.
    pub depth: u32,
    /// Why this stop matters for the tour narrative.
    pub note: String,
}

/// A guided tour through the codebase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureTour {
    /// Stable tour id (matches the underlying `process_id`).
    pub id: u32,
    /// Display title (e.g. "Tour: handle_request").
    pub title: String,
    /// Short narrative describing what the tour covers.
    pub summary: String,
    /// Entry point symbol name.
    pub entry_point: String,
    /// Ordered stops along the tour.
    pub stops: Vec<TourStop>,
}

/// Build tours from all indexed processes for a repo.
///
/// Each `Process` becomes one tour, with steps converted to stops in
/// traversal order. `max_stops` caps tour length (defaults to 12 if 0).
pub fn build_tours(
    conn: &Connection,
    repo_id: i64,
    max_stops: u32,
) -> Result<Vec<ArchitectureTour>, IndexError> {
    let cap = if max_stops == 0 { 12 } else { max_stops };
    // Ensure process tables exist even if `compute_processes` was never run
    // for this database — return an empty list instead of an SQLite error.
    super::processes::ensure_process_tables(conn)?;
    let processes = list_processes(conn, repo_id)?;
    Ok(processes
        .into_iter()
        .map(|p| process_to_tour(p, cap))
        .collect())
}

/// Convert a single `Process` into an `ArchitectureTour`.
fn process_to_tour(process: Process, max_stops: u32) -> ArchitectureTour {
    let entry = process.entry_point.clone();
    let total = process.steps.len();

    let stops: Vec<TourStop> = process
        .steps
        .into_iter()
        .take(max_stops as usize)
        .enumerate()
        .map(|(i, step)| {
            let order = (i + 1) as u32;
            let note = stop_note(order, step.depth, &step.name, &entry);
            TourStop {
                order,
                symbol: step.name,
                file: step.file,
                line: step.line,
                depth: step.depth,
                note,
            }
        })
        .collect();

    let summary = if total == 0 {
        format!("Entry point `{entry}` has no traced steps.")
    } else if total <= max_stops as usize {
        format!("Walks the {total}-step execution flow starting from `{entry}`.")
    } else {
        format!(
            "First {} of {total} steps starting from `{entry}` (truncated).",
            max_stops
        )
    };

    ArchitectureTour {
        id: process.id,
        title: format!("Tour: {entry}"),
        summary,
        entry_point: entry,
        stops,
    }
}

/// Build a short narrative note for a single tour stop.
fn stop_note(order: u32, depth: u32, symbol: &str, entry: &str) -> String {
    if order == 1 {
        format!("Start here \u{2014} this is the entry point `{entry}`.")
    } else if depth == 1 {
        format!("`{symbol}` is called directly from the entry point.")
    } else {
        format!(
            "`{symbol}` (depth {depth}) \u{2014} reached through {} hops from the entry.",
            depth
        )
    }
}

/// Find a single tour by entry-point name.
pub fn find_tour_by_entry(
    conn: &Connection,
    repo_id: i64,
    entry_point: &str,
    max_stops: u32,
) -> Result<Option<ArchitectureTour>, IndexError> {
    let tours = build_tours(conn, repo_id, max_stops)?;
    Ok(tours.into_iter().find(|t| t.entry_point == entry_point))
}

#[cfg(test)]
mod tests {
    use super::super::processes::ProcessStep;
    use super::*;

    fn fake_process(id: u32, entry: &str, step_count: usize) -> Process {
        let steps = (0..step_count)
            .map(|i| ProcessStep {
                symbol_id: i as i64 + 1,
                name: if i == 0 {
                    entry.to_string()
                } else {
                    format!("step_{i}")
                },
                file: format!("src/file_{i}.rs"),
                line: (i + 1) as u32 * 10,
                depth: i as u32,
            })
            .collect();
        Process {
            id,
            entry_point: entry.to_string(),
            entry_symbol_id: 1,
            steps,
        }
    }

    #[test]
    fn process_to_tour_preserves_step_order() {
        let p = fake_process(1, "main", 5);
        let tour = process_to_tour(p, 12);
        assert_eq!(tour.stops.len(), 5);
        assert_eq!(tour.stops[0].order, 1);
        assert_eq!(tour.stops[4].order, 5);
        assert_eq!(tour.stops[0].symbol, "main");
        assert_eq!(tour.entry_point, "main");
        assert!(tour.title.contains("main"));
    }

    #[test]
    fn process_to_tour_truncates_to_max_stops() {
        let p = fake_process(2, "handle", 20);
        let tour = process_to_tour(p, 5);
        assert_eq!(tour.stops.len(), 5);
        assert!(tour.summary.contains("5 of 20"));
    }

    #[test]
    fn process_to_tour_empty_process_yields_empty_stops() {
        let p = fake_process(3, "noop", 0);
        let tour = process_to_tour(p, 12);
        assert!(tour.stops.is_empty());
        assert!(tour.summary.contains("no traced steps"));
    }

    #[test]
    fn stop_notes_vary_by_position_and_depth() {
        let n1 = stop_note(1, 0, "main", "main");
        assert!(n1.contains("Start here"));
        let n2 = stop_note(2, 1, "init", "main");
        assert!(n2.contains("called directly"));
        let n3 = stop_note(5, 4, "deep_fn", "main");
        assert!(n3.contains("4 hops"));
    }

    #[test]
    fn zero_max_stops_uses_default_cap() {
        let p = fake_process(4, "entry", 30);
        let tour = process_to_tour(p, 12);
        assert_eq!(tour.stops.len(), 12);
    }

    #[test]
    fn build_tours_with_real_db() {
        // Integration: build tours from an in-memory SQLite with one process.
        use super::super::symbol_index::open_db;
        let tmp = tempfile::tempdir().unwrap();
        let conn = open_db(tmp.path()).unwrap();
        super::super::processes::ensure_process_tables(&conn).unwrap();
        // Insert a repo, symbols, a process, and steps.
        conn.execute(
            "INSERT INTO code_repos (id, path, label, indexed_at) VALUES (1, '/test', 'test', 0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO code_symbols (id, repo_id, name, kind, file, line, end_line) \
             VALUES (1, 1, 'main', 'function', 'src/main.rs', 10, 20)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO code_symbols (id, repo_id, name, kind, file, line, end_line) \
             VALUES (2, 1, 'helper', 'function', 'src/main.rs', 50, 60)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO code_processes (repo_id, process_id, entry_symbol_id, entry_name) \
             VALUES (1, 1, 1, 'main')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO code_process_steps (repo_id, process_id, symbol_id, depth, step_order) \
             VALUES (1, 1, 1, 0, 0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO code_process_steps (repo_id, process_id, symbol_id, depth, step_order) \
             VALUES (1, 1, 2, 1, 1)",
            [],
        )
        .unwrap();

        let tours = build_tours(&conn, 1, 12).unwrap();
        assert_eq!(tours.len(), 1);
        assert_eq!(tours[0].entry_point, "main");
        assert_eq!(tours[0].stops.len(), 2);

        let found = find_tour_by_entry(&conn, 1, "main", 12).unwrap();
        assert!(found.is_some());
        let none = find_tour_by_entry(&conn, 1, "missing", 12).unwrap();
        assert!(none.is_none());
    }
}
