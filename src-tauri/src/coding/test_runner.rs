//! Sandboxed pre-commit test runner for the self-improve loop.
//!
//! **Chunk 28.4** — `docs/coding-workflow-design.md` §5 item 5
//! ("Sandboxed test runs"). Before the orchestrator commits a generated
//! diff, we want to run `cargo test` and `vitest` in an *isolated child
//! process* (separate stdout/stderr, separate working directory if needed,
//! separate signal group on Unix) and **gate the commit on a green run**.
//!
//! This module is intentionally *pure* in the same sense as
//! `brain/maintenance_scheduler.rs` — it does no I/O against `AppState`
//! and exposes a single async entry point that takes a config and
//! returns a structured result. The orchestrator (chunk 28.5+) will wire
//! it into the workflow engine alongside the reviewer sub-agent.
//!
//! Design choices
//! --------------
//! - **Use `tokio::process::Command`** rather than blocking
//!   `std::process::Command`. We're already in an async runtime and we
//!   need `tokio::time::timeout` to abort runaway test runs.
//! - **Retry-on-fail once** to detect flaky tests (a test that fails the
//!   first time but passes on retry → status `Flaky`, not `Fail`). This
//!   matches the project's stance that the
//!   `circuit_breaker_reopens_on_failed_probe` test in
//!   `workflows::resilience` is timing-dependent and should be re-run
//!   rather than investigated.
//! - **Truncate captured output** to the last 4 KiB per stream. Test
//!   suites can produce hundreds of KB of stdout; the orchestrator only
//!   needs the tail (final summary lines, panic backtraces).
//! - **No environment leakage** — we strip `RUST_LOG`, `CARGO_TARGET_DIR`,
//!   `NODE_OPTIONS` from the child env so the test run uses the project's
//!   defaults regardless of what the parent shell exported.
//! - **No global side effects** — every function takes its inputs as
//!   parameters; tests verify behaviour by spawning trivial commands
//!   like `cmd /c exit 0` (Windows) or `sh -c 'exit 0'` (Unix).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Default per-suite timeout. `cargo test` on a cold target/ on this repo
/// takes ~90s; `vitest run` takes ~15s. Five minutes is generous.
pub const DEFAULT_PER_SUITE_TIMEOUT: Duration = Duration::from_secs(300);

/// Default tail length captured from each stream (bytes). 4 KiB is enough
/// for the final summary lines of `cargo test` + a short panic
/// backtrace, but keeps `RunRecord` payloads small.
pub const DEFAULT_OUTPUT_TAIL_BYTES: usize = 4096;

/// One test suite to run. The two built-in variants cover the project's
/// CI gate; `Custom` lets the orchestrator add extra suites
/// (e.g. `cargo clippy -- -D warnings`) without modifying this file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TestSuite {
    /// `cargo test --lib` in the `src-tauri` directory.
    Cargo,
    /// `npx vitest run` in the project root.
    Vitest,
    /// Arbitrary command. `program` resolved through PATH.
    Custom {
        name: String,
        program: String,
        args: Vec<String>,
        /// Optional cwd override; defaults to `TestRunConfig::cwd`.
        cwd: Option<PathBuf>,
    },
}

impl TestSuite {
    /// Stable display name used in [`SuiteResult`] and logs.
    pub fn name(&self) -> String {
        match self {
            TestSuite::Cargo => "cargo test".to_string(),
            TestSuite::Vitest => "vitest".to_string(),
            TestSuite::Custom { name, .. } => name.clone(),
        }
    }
}

/// Final status of a single suite after the (optional) retry.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SuiteStatus {
    /// Passed on the first attempt.
    Pass,
    /// Failed on the first attempt, passed on retry. Counts as green for
    /// the gate but the orchestrator may surface a "this run was flaky"
    /// note to the user.
    Flaky,
    /// Failed on every attempt.
    Fail,
    /// Killed after exceeding the per-suite timeout.
    Timeout,
    /// Could not spawn (e.g. binary missing). Treated as failure.
    SpawnError,
}

impl SuiteStatus {
    /// True when this status counts as "green" for the commit gate.
    /// `Flaky` is green: the test eventually passed.
    pub fn is_green(self) -> bool {
        matches!(self, SuiteStatus::Pass | SuiteStatus::Flaky)
    }
}

/// Result of one suite — what the orchestrator will display in the
/// pre-commit gate UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuiteResult {
    pub name: String,
    pub status: SuiteStatus,
    /// Number of attempts actually run (1 or 2).
    pub attempts: u8,
    /// Total wall-clock duration of all attempts, in milliseconds.
    pub duration_ms: u128,
    /// Exit code of the final attempt. `None` for `Timeout` / `SpawnError`.
    pub exit_code: Option<i32>,
    /// Last `DEFAULT_OUTPUT_TAIL_BYTES` of stdout from the final attempt.
    pub stdout_tail: String,
    /// Last `DEFAULT_OUTPUT_TAIL_BYTES` of stderr from the final attempt.
    pub stderr_tail: String,
    /// When `status == SpawnError`, the underlying OS error message.
    pub spawn_error: Option<String>,
}

/// Top-level result of a test-runner invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunResult {
    pub suites: Vec<SuiteResult>,
    /// True when **every** suite is `is_green()`. This is the value the
    /// orchestrator gates the commit on.
    pub all_green: bool,
    /// Wall-clock duration of the whole run, in milliseconds.
    pub total_duration_ms: u128,
    /// Suite names that ended in `Flaky` — surfaced separately so the UI
    /// can highlight them without re-scanning `suites`.
    pub flaky_suites: Vec<String>,
}

/// Configuration for a single invocation of [`run_tests`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunConfig {
    pub suites: Vec<TestSuite>,
    /// Working directory for `Cargo` / `Vitest` / suites without an
    /// explicit `cwd`. Typically the workspace root.
    pub cwd: PathBuf,
    /// If true, a failing suite is run a second time before being
    /// reported as `Fail`. Passing on retry → `Flaky`. Default true.
    pub retry_flaky_once: bool,
    /// Per-suite hard timeout. Default
    /// [`DEFAULT_PER_SUITE_TIMEOUT`] (5 min).
    pub per_suite_timeout: Duration,
    /// Number of bytes of stdout/stderr to retain per suite (tail).
    /// Default [`DEFAULT_OUTPUT_TAIL_BYTES`] (4 KiB).
    pub output_tail_bytes: usize,
}

impl TestRunConfig {
    /// Default config that runs the project's standard CI gate.
    pub fn default_ci_gate(workspace_root: PathBuf) -> Self {
        Self {
            suites: vec![TestSuite::Vitest, TestSuite::Cargo],
            cwd: workspace_root,
            retry_flaky_once: true,
            per_suite_timeout: DEFAULT_PER_SUITE_TIMEOUT,
            output_tail_bytes: DEFAULT_OUTPUT_TAIL_BYTES,
        }
    }
}

/// Run every suite in `config.suites` sequentially and return a
/// structured result. Never panics: spawn errors and timeouts are
/// reported as suite statuses.
pub async fn run_tests(config: &TestRunConfig) -> TestRunResult {
    let started = Instant::now();
    let mut results = Vec::with_capacity(config.suites.len());
    let mut flaky = Vec::new();

    for suite in &config.suites {
        let result = run_one_suite(suite, config).await;
        if matches!(result.status, SuiteStatus::Flaky) {
            flaky.push(result.name.clone());
        }
        results.push(result);
    }

    let all_green = results.iter().all(|r| r.status.is_green());
    TestRunResult {
        suites: results,
        all_green,
        total_duration_ms: started.elapsed().as_millis(),
        flaky_suites: flaky,
    }
}

async fn run_one_suite(suite: &TestSuite, config: &TestRunConfig) -> SuiteResult {
    let started = Instant::now();
    let first = run_one_attempt(suite, config).await;

    if first.green || !config.retry_flaky_once {
        let attempts = 1;
        let status = first.status;
        return SuiteResult {
            name: suite.name(),
            status,
            attempts,
            duration_ms: started.elapsed().as_millis(),
            exit_code: first.exit_code,
            stdout_tail: first.stdout_tail,
            stderr_tail: first.stderr_tail,
            spawn_error: first.spawn_error,
        };
    }

    // Retry once.
    let second = run_one_attempt(suite, config).await;
    let status = if second.green {
        // Failed first time, passed on retry → flaky.
        SuiteStatus::Flaky
    } else {
        second.status
    };

    SuiteResult {
        name: suite.name(),
        status,
        attempts: 2,
        duration_ms: started.elapsed().as_millis(),
        exit_code: second.exit_code,
        stdout_tail: second.stdout_tail,
        stderr_tail: second.stderr_tail,
        spawn_error: second.spawn_error,
    }
}

struct Attempt {
    green: bool,
    status: SuiteStatus,
    exit_code: Option<i32>,
    stdout_tail: String,
    stderr_tail: String,
    spawn_error: Option<String>,
}

async fn run_one_attempt(suite: &TestSuite, config: &TestRunConfig) -> Attempt {
    let (program, args, cwd) = command_for_suite(suite, &config.cwd);

    let mut cmd = tokio::process::Command::new(&program);
    cmd.args(&args).current_dir(&cwd);
    // Strip env vars that could change test behaviour.
    cmd.env_remove("RUST_LOG");
    cmd.env_remove("CARGO_TARGET_DIR");
    cmd.env_remove("NODE_OPTIONS");
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    cmd.stdin(std::process::Stdio::null());
    // On Windows, kill_on_drop is a no-op for the process group, but for
    // child processes spawned by cargo/vitest this at least kills the
    // top-level. Good enough for the timeout path.
    cmd.kill_on_drop(true);

    let child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return Attempt {
                green: false,
                status: SuiteStatus::SpawnError,
                exit_code: None,
                stdout_tail: String::new(),
                stderr_tail: String::new(),
                spawn_error: Some(format!("{program}: {e}")),
            };
        }
    };

    let timeout = config.per_suite_timeout;
    let waited = tokio::time::timeout(timeout, child.wait_with_output()).await;

    let output = match waited {
        Ok(Ok(o)) => o,
        Ok(Err(e)) => {
            return Attempt {
                green: false,
                status: SuiteStatus::SpawnError,
                exit_code: None,
                stdout_tail: String::new(),
                stderr_tail: String::new(),
                spawn_error: Some(format!("wait failed: {e}")),
            };
        }
        Err(_) => {
            // wait_with_output already moved the child; we can't kill
            // here. kill_on_drop fired when `child` was consumed.
            return Attempt {
                green: false,
                status: SuiteStatus::Timeout,
                exit_code: None,
                stdout_tail: String::new(),
                stderr_tail: String::new(),
                spawn_error: None,
            };
        }
    };

    let exit_code = output.status.code();
    let success = output.status.success();
    let stdout_tail = tail_string(&output.stdout, config.output_tail_bytes);
    let stderr_tail = tail_string(&output.stderr, config.output_tail_bytes);

    Attempt {
        green: success,
        status: if success {
            SuiteStatus::Pass
        } else {
            SuiteStatus::Fail
        },
        exit_code,
        stdout_tail,
        stderr_tail,
        spawn_error: None,
    }
}

fn command_for_suite(suite: &TestSuite, default_cwd: &std::path::Path) -> (String, Vec<String>, PathBuf) {
    match suite {
        TestSuite::Cargo => (
            "cargo".to_string(),
            vec!["test".to_string(), "--lib".to_string()],
            default_cwd.join("src-tauri"),
        ),
        TestSuite::Vitest => {
            // `npx` on Windows is `npx.cmd`. tokio::process::Command on
            // Windows resolves `.cmd` automatically when given the bare
            // name only on recent Rust, but we play it safe by trying
            // `npx.cmd` first on Windows.
            let program = if cfg!(windows) { "npx.cmd" } else { "npx" };
            (
                program.to_string(),
                vec!["vitest".to_string(), "run".to_string()],
                default_cwd.to_path_buf(),
            )
        }
        TestSuite::Custom {
            program, args, cwd, ..
        } => (
            program.clone(),
            args.clone(),
            cwd.clone().unwrap_or_else(|| default_cwd.to_path_buf()),
        ),
    }
}

/// Return the last `max_bytes` bytes of `bytes` as a UTF-8 string,
/// replacing invalid sequences with `U+FFFD`. If the truncation cut
/// mid-character, [`String::from_utf8_lossy`] handles it.
fn tail_string(bytes: &[u8], max_bytes: usize) -> String {
    if bytes.len() <= max_bytes {
        return String::from_utf8_lossy(bytes).into_owned();
    }
    let start = bytes.len() - max_bytes;
    String::from_utf8_lossy(&bytes[start..]).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pick a no-op exit-zero command per platform.
    fn ok_suite(name: &str) -> TestSuite {
        if cfg!(windows) {
            TestSuite::Custom {
                name: name.to_string(),
                program: "cmd".to_string(),
                args: vec!["/C".to_string(), "exit 0".to_string()],
                cwd: None,
            }
        } else {
            TestSuite::Custom {
                name: name.to_string(),
                program: "sh".to_string(),
                args: vec!["-c".to_string(), "exit 0".to_string()],
                cwd: None,
            }
        }
    }

    fn fail_suite(name: &str) -> TestSuite {
        if cfg!(windows) {
            TestSuite::Custom {
                name: name.to_string(),
                program: "cmd".to_string(),
                args: vec!["/C".to_string(), "exit 1".to_string()],
                cwd: None,
            }
        } else {
            TestSuite::Custom {
                name: name.to_string(),
                program: "sh".to_string(),
                args: vec!["-c".to_string(), "exit 1".to_string()],
                cwd: None,
            }
        }
    }

    /// A suite that runs longer than the timeout we give it.
    fn slow_suite(name: &str) -> TestSuite {
        if cfg!(windows) {
            // ping -n 4 ~= 3 seconds.
            TestSuite::Custom {
                name: name.to_string(),
                program: "cmd".to_string(),
                args: vec![
                    "/C".to_string(),
                    "ping -n 4 127.0.0.1 > NUL".to_string(),
                ],
                cwd: None,
            }
        } else {
            TestSuite::Custom {
                name: name.to_string(),
                program: "sh".to_string(),
                args: vec!["-c".to_string(), "sleep 3".to_string()],
                cwd: None,
            }
        }
    }

    fn cwd() -> PathBuf {
        std::env::current_dir().unwrap()
    }

    #[tokio::test]
    async fn passing_suite_is_green() {
        let cfg = TestRunConfig {
            suites: vec![ok_suite("ok")],
            cwd: cwd(),
            retry_flaky_once: true,
            per_suite_timeout: Duration::from_secs(10),
            output_tail_bytes: 1024,
        };
        let result = run_tests(&cfg).await;
        assert!(result.all_green);
        assert_eq!(result.suites.len(), 1);
        assert_eq!(result.suites[0].status, SuiteStatus::Pass);
        assert_eq!(result.suites[0].attempts, 1);
        assert!(result.flaky_suites.is_empty());
    }

    #[tokio::test]
    async fn failing_suite_is_retried_then_reported_fail() {
        let cfg = TestRunConfig {
            suites: vec![fail_suite("always-fails")],
            cwd: cwd(),
            retry_flaky_once: true,
            per_suite_timeout: Duration::from_secs(10),
            output_tail_bytes: 1024,
        };
        let result = run_tests(&cfg).await;
        assert!(!result.all_green);
        assert_eq!(result.suites[0].status, SuiteStatus::Fail);
        assert_eq!(result.suites[0].attempts, 2);
        assert_eq!(result.suites[0].exit_code, Some(1));
    }

    #[tokio::test]
    async fn failing_suite_with_no_retry_runs_once() {
        let cfg = TestRunConfig {
            suites: vec![fail_suite("once")],
            cwd: cwd(),
            retry_flaky_once: false,
            per_suite_timeout: Duration::from_secs(10),
            output_tail_bytes: 1024,
        };
        let result = run_tests(&cfg).await;
        assert!(!result.all_green);
        assert_eq!(result.suites[0].status, SuiteStatus::Fail);
        assert_eq!(result.suites[0].attempts, 1);
    }

    #[tokio::test]
    async fn slow_suite_is_killed_with_timeout_status() {
        let cfg = TestRunConfig {
            suites: vec![slow_suite("slow")],
            cwd: cwd(),
            retry_flaky_once: false,
            per_suite_timeout: Duration::from_millis(500),
            output_tail_bytes: 1024,
        };
        let result = run_tests(&cfg).await;
        assert!(!result.all_green);
        assert_eq!(result.suites[0].status, SuiteStatus::Timeout);
        assert_eq!(result.suites[0].exit_code, None);
    }

    #[tokio::test]
    async fn missing_program_yields_spawn_error() {
        let cfg = TestRunConfig {
            suites: vec![TestSuite::Custom {
                name: "missing".to_string(),
                program: "this-binary-does-not-exist-xyz".to_string(),
                args: vec![],
                cwd: None,
            }],
            cwd: cwd(),
            retry_flaky_once: false,
            per_suite_timeout: Duration::from_secs(5),
            output_tail_bytes: 1024,
        };
        let result = run_tests(&cfg).await;
        assert!(!result.all_green);
        assert_eq!(result.suites[0].status, SuiteStatus::SpawnError);
        assert!(result.suites[0].spawn_error.is_some());
    }

    #[tokio::test]
    async fn mixed_suites_propagate_per_suite_status() {
        let cfg = TestRunConfig {
            suites: vec![ok_suite("good"), fail_suite("bad")],
            cwd: cwd(),
            retry_flaky_once: false,
            per_suite_timeout: Duration::from_secs(10),
            output_tail_bytes: 1024,
        };
        let result = run_tests(&cfg).await;
        assert!(!result.all_green);
        assert_eq!(result.suites.len(), 2);
        assert_eq!(result.suites[0].status, SuiteStatus::Pass);
        assert_eq!(result.suites[1].status, SuiteStatus::Fail);
    }

    #[test]
    fn suite_status_is_green_treats_flaky_as_green() {
        assert!(SuiteStatus::Pass.is_green());
        assert!(SuiteStatus::Flaky.is_green());
        assert!(!SuiteStatus::Fail.is_green());
        assert!(!SuiteStatus::Timeout.is_green());
        assert!(!SuiteStatus::SpawnError.is_green());
    }

    #[test]
    fn suite_name_is_stable() {
        assert_eq!(TestSuite::Cargo.name(), "cargo test");
        assert_eq!(TestSuite::Vitest.name(), "vitest");
        let custom = TestSuite::Custom {
            name: "clippy".to_string(),
            program: "cargo".to_string(),
            args: vec!["clippy".to_string()],
            cwd: None,
        };
        assert_eq!(custom.name(), "clippy");
    }

    #[test]
    fn tail_string_returns_full_text_when_under_limit() {
        let s = tail_string(b"hello world", 4096);
        assert_eq!(s, "hello world");
    }

    #[test]
    fn tail_string_keeps_only_last_max_bytes() {
        let payload: Vec<u8> = (0..10_000u32).map(|i| (i % 26) as u8 + b'a').collect();
        let s = tail_string(&payload, 100);
        assert_eq!(s.len(), 100);
        // The last byte of `payload` should be the last byte of `s`.
        assert_eq!(s.as_bytes().last(), payload.last());
    }

    #[test]
    fn default_ci_gate_includes_both_suites() {
        let cfg = TestRunConfig::default_ci_gate(PathBuf::from("/tmp"));
        assert_eq!(cfg.suites.len(), 2);
        assert!(matches!(cfg.suites[0], TestSuite::Vitest));
        assert!(matches!(cfg.suites[1], TestSuite::Cargo));
        assert!(cfg.retry_flaky_once);
    }

    #[test]
    fn command_for_suite_cargo_targets_src_tauri() {
        let (program, args, cwd) = command_for_suite(&TestSuite::Cargo, std::path::Path::new("/repo"));
        assert_eq!(program, "cargo");
        assert_eq!(args, vec!["test".to_string(), "--lib".to_string()]);
        assert!(cwd.ends_with("src-tauri"));
    }

    #[test]
    fn command_for_suite_vitest_runs_in_workspace_root() {
        let (program, args, cwd) =
            command_for_suite(&TestSuite::Vitest, std::path::Path::new("/repo"));
        assert!(program.starts_with("npx"));
        assert_eq!(args, vec!["vitest".to_string(), "run".to_string()]);
        assert_eq!(cwd, std::path::Path::new("/repo"));
    }

    #[test]
    fn command_for_suite_custom_uses_explicit_cwd_when_set() {
        let suite = TestSuite::Custom {
            name: "x".to_string(),
            program: "p".to_string(),
            args: vec!["a".to_string()],
            cwd: Some(PathBuf::from("/elsewhere")),
        };
        let (_, _, cwd) = command_for_suite(&suite, std::path::Path::new("/repo"));
        assert_eq!(cwd, PathBuf::from("/elsewhere"));
    }

    #[test]
    fn test_run_result_serializes_to_json() {
        let r = TestRunResult {
            suites: vec![SuiteResult {
                name: "x".to_string(),
                status: SuiteStatus::Pass,
                attempts: 1,
                duration_ms: 42,
                exit_code: Some(0),
                stdout_tail: "ok".to_string(),
                stderr_tail: String::new(),
                spawn_error: None,
            }],
            all_green: true,
            total_duration_ms: 42,
            flaky_suites: vec![],
        };
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("\"all_green\":true"));
        assert!(json.contains("\"status\":\"pass\""));
    }
}
