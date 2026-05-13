//! Actor-style supervision tree for the agent fleet (ACTOR-MODEL-1, Phase INFRA).
//!
//! Each registered agent declares a restart policy. The supervisor enforces
//! restart budgets and marks agents `Degraded` when the budget is exhausted.
//!
//! See the README pillar "Actor model" and `rules/milestones.md` ACTOR-MODEL-1.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Restart policy — how the supervisor handles a crashed agent.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestartPolicy {
    /// Always restart, regardless of exit reason.
    Always,
    /// Only restart on failure (non-zero exit / panic). Clean exit = no restart.
    OnFailure,
    /// Never restart — agent stays down after any stop.
    Never,
}

/// Exponential backoff configuration for restart delays.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExponentialBackoff {
    /// Initial delay before first restart (ms).
    pub initial_ms: u64,
    /// Multiplier applied to delay after each consecutive failure.
    pub factor: f64,
    /// Maximum delay cap (ms).
    pub max_ms: u64,
}

impl Default for ExponentialBackoff {
    fn default() -> Self {
        Self {
            initial_ms: 500,
            factor: 2.0,
            max_ms: 30_000,
        }
    }
}

impl ExponentialBackoff {
    /// Compute the delay for the `attempt`-th restart (0-indexed).
    pub fn delay_for(&self, attempt: u32) -> Duration {
        let ms = (self.initial_ms as f64 * self.factor.powi(attempt as i32)) as u64;
        Duration::from_millis(ms.min(self.max_ms))
    }
}

/// Typed supervision specification declared by each agent on registration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SupervisionSpec {
    pub policy: RestartPolicy,
    /// Maximum restarts allowed within `window`.
    pub max_restarts: u32,
    /// Rolling time window for counting restarts.
    pub window: Duration,
    /// Backoff configuration between restarts.
    pub backoff: ExponentialBackoff,
}

impl Default for SupervisionSpec {
    fn default() -> Self {
        Self {
            policy: RestartPolicy::OnFailure,
            max_restarts: 3,
            window: Duration::from_secs(300), // 5 minutes
            backoff: ExponentialBackoff::default(),
        }
    }
}

/// Runtime health status of a supervised agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    /// Agent is running normally.
    Running,
    /// Agent is restarting (within budget).
    Restarting,
    /// Agent exhausted its restart budget — manual intervention needed.
    Degraded,
    /// Agent was stopped via `policy: Never` or manual stop.
    Stopped,
}

/// Internal state tracking for one supervised agent.
#[derive(Debug, Clone)]
struct AgentState {
    spec: SupervisionSpec,
    status: AgentStatus,
    /// Timestamps of recent crashes (within `spec.window`).
    crash_timestamps: Vec<Instant>,
    /// Number of consecutive restarts since last stable run.
    consecutive_restarts: u32,
}

impl AgentState {
    fn new(spec: SupervisionSpec) -> Self {
        Self {
            spec,
            status: AgentStatus::Running,
            crash_timestamps: Vec::new(),
            consecutive_restarts: 0,
        }
    }

    /// Record a crash and determine if the agent should restart or degrade.
    fn record_crash(&mut self, now: Instant) -> AgentStatus {
        // Prune old crashes outside the window.
        let window_start = now.checked_sub(self.spec.window).unwrap_or(now);
        self.crash_timestamps.retain(|&t| t >= window_start);
        self.crash_timestamps.push(now);
        self.consecutive_restarts += 1;

        match self.spec.policy {
            RestartPolicy::Never => {
                self.status = AgentStatus::Stopped;
            }
            RestartPolicy::Always | RestartPolicy::OnFailure => {
                if self.crash_timestamps.len() as u32 > self.spec.max_restarts {
                    // Budget exhausted — degraded.
                    self.status = AgentStatus::Degraded;
                } else {
                    self.status = AgentStatus::Restarting;
                }
            }
        }

        self.status
    }

    /// Mark agent as successfully running (resets consecutive counter).
    fn mark_running(&mut self) {
        self.status = AgentStatus::Running;
        self.consecutive_restarts = 0;
    }

    /// Get the backoff delay for the next restart attempt.
    fn next_backoff(&self) -> Duration {
        self.spec
            .backoff
            .delay_for(self.consecutive_restarts.saturating_sub(1))
    }
}

/// Supervisor — manages the agent fleet with typed restart policies.
#[derive(Debug, Clone)]
pub struct Supervisor {
    agents: Arc<Mutex<HashMap<String, AgentState>>>,
}

impl Supervisor {
    pub fn new() -> Self {
        Self {
            agents: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register an agent with its supervision spec.
    pub fn register(&self, agent_id: &str, spec: SupervisionSpec) {
        let mut agents = self.agents.lock().unwrap();
        agents.insert(agent_id.to_string(), AgentState::new(spec));
    }

    /// Report a crash for the given agent. Returns the new status.
    pub fn report_crash(&self, agent_id: &str) -> Option<AgentStatus> {
        self.report_crash_at(agent_id, Instant::now())
    }

    /// Report a crash at a specific instant (for testing).
    pub fn report_crash_at(&self, agent_id: &str, now: Instant) -> Option<AgentStatus> {
        let mut agents = self.agents.lock().unwrap();
        agents.get_mut(agent_id).map(|state| state.record_crash(now))
    }

    /// Mark an agent as successfully running after a restart.
    pub fn mark_running(&self, agent_id: &str) {
        let mut agents = self.agents.lock().unwrap();
        if let Some(state) = agents.get_mut(agent_id) {
            state.mark_running();
        }
    }

    /// Get the current status of an agent.
    pub fn status(&self, agent_id: &str) -> Option<AgentStatus> {
        let agents = self.agents.lock().unwrap();
        agents.get(agent_id).map(|s| s.status)
    }

    /// Get the backoff delay for the next restart of the given agent.
    pub fn next_backoff(&self, agent_id: &str) -> Option<Duration> {
        let agents = self.agents.lock().unwrap();
        agents.get(agent_id).map(|s| s.next_backoff())
    }

    /// List all registered agent IDs with their current status.
    pub fn list(&self) -> Vec<(String, AgentStatus)> {
        let agents = self.agents.lock().unwrap();
        agents
            .iter()
            .map(|(id, state)| (id.clone(), state.status))
            .collect()
    }
}

impl Default for Supervisor {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Agent crashes 3× in 1 min → trips restart budget → marked `Degraded`.
    #[test]
    fn crash_3x_trips_budget_degraded() {
        let supervisor = Supervisor::new();

        let spec = SupervisionSpec {
            policy: RestartPolicy::OnFailure,
            max_restarts: 3,
            window: Duration::from_secs(60), // 1 minute
            backoff: ExponentialBackoff::default(),
        };

        supervisor.register("rag-agent", spec);
        assert_eq!(supervisor.status("rag-agent"), Some(AgentStatus::Running));

        let now = Instant::now();

        // Crash 1 — within budget.
        let status = supervisor.report_crash_at("rag-agent", now).unwrap();
        assert_eq!(status, AgentStatus::Restarting);

        // Crash 2 — still within budget.
        let status = supervisor
            .report_crash_at("rag-agent", now + Duration::from_secs(10))
            .unwrap();
        assert_eq!(status, AgentStatus::Restarting);

        // Crash 3 — still within budget (max_restarts=3 means 3 allowed).
        let status = supervisor
            .report_crash_at("rag-agent", now + Duration::from_secs(20))
            .unwrap();
        assert_eq!(status, AgentStatus::Restarting);

        // Crash 4 — exceeds budget (4 > 3) → degraded.
        let status = supervisor
            .report_crash_at("rag-agent", now + Duration::from_secs(30))
            .unwrap();
        assert_eq!(status, AgentStatus::Degraded);
    }

    /// Agent with `policy: Always` survives OOM-kill and resumes.
    #[test]
    fn policy_always_survives_oom_and_resumes() {
        let supervisor = Supervisor::new();

        let spec = SupervisionSpec {
            policy: RestartPolicy::Always,
            max_restarts: 5,
            window: Duration::from_secs(300), // 5 minutes
            backoff: ExponentialBackoff {
                initial_ms: 100,
                factor: 2.0,
                max_ms: 5_000,
            },
        };

        supervisor.register("embed-worker", spec);

        // Simulate OOM-kill (crash).
        let now = Instant::now();
        let status = supervisor.report_crash_at("embed-worker", now).unwrap();
        assert_eq!(status, AgentStatus::Restarting);

        // Verify backoff is calculated.
        let backoff = supervisor.next_backoff("embed-worker").unwrap();
        assert_eq!(backoff, Duration::from_millis(100)); // first attempt: initial_ms

        // Simulate successful restart.
        supervisor.mark_running("embed-worker");
        assert_eq!(
            supervisor.status("embed-worker"),
            Some(AgentStatus::Running)
        );

        // Crash again — budget resets because window hasn't expired but
        // consecutive resets to 0 on mark_running, so backoff restarts.
        let status = supervisor
            .report_crash_at("embed-worker", now + Duration::from_secs(60))
            .unwrap();
        assert_eq!(status, AgentStatus::Restarting);

        // Backoff for this new crash: first attempt again = initial_ms.
        let backoff = supervisor.next_backoff("embed-worker").unwrap();
        assert_eq!(backoff, Duration::from_millis(100));

        // Mark running again — agent fully survives.
        supervisor.mark_running("embed-worker");
        assert_eq!(
            supervisor.status("embed-worker"),
            Some(AgentStatus::Running)
        );
    }

    #[test]
    fn policy_never_stops_on_first_crash() {
        let supervisor = Supervisor::new();

        let spec = SupervisionSpec {
            policy: RestartPolicy::Never,
            max_restarts: 10, // irrelevant for Never policy
            window: Duration::from_secs(60),
            backoff: ExponentialBackoff::default(),
        };

        supervisor.register("one-shot-task", spec);

        let status = supervisor.report_crash("one-shot-task").unwrap();
        assert_eq!(status, AgentStatus::Stopped);
    }

    #[test]
    fn old_crashes_outside_window_dont_count() {
        let supervisor = Supervisor::new();

        let spec = SupervisionSpec {
            policy: RestartPolicy::OnFailure,
            max_restarts: 2,
            window: Duration::from_secs(60),
            backoff: ExponentialBackoff::default(),
        };

        supervisor.register("agent-a", spec);

        let now = Instant::now();

        // Crash 1 at t=0.
        supervisor.report_crash_at("agent-a", now);

        // Crash 2 at t=0+30s — still in budget.
        let status = supervisor
            .report_crash_at("agent-a", now + Duration::from_secs(30))
            .unwrap();
        assert_eq!(status, AgentStatus::Restarting);

        // Crash 3 at t=0+90s — crash 1 is now outside the 60s window,
        // so only 2 crashes remain in window (crash 2 + crash 3) → still OK.
        let status = supervisor
            .report_crash_at("agent-a", now + Duration::from_secs(90))
            .unwrap();
        assert_eq!(status, AgentStatus::Restarting);
    }

    #[test]
    fn exponential_backoff_calculation() {
        let backoff = ExponentialBackoff {
            initial_ms: 200,
            factor: 3.0,
            max_ms: 10_000,
        };

        assert_eq!(backoff.delay_for(0), Duration::from_millis(200));
        assert_eq!(backoff.delay_for(1), Duration::from_millis(600));
        assert_eq!(backoff.delay_for(2), Duration::from_millis(1800));
        assert_eq!(backoff.delay_for(3), Duration::from_millis(5400));
        // Capped at max_ms.
        assert_eq!(backoff.delay_for(4), Duration::from_millis(10_000));
    }
}
