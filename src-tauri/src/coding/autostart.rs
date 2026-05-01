//! Windows auto-start support via the per-user Run registry key.
//!
//! Writes / removes `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run`
//! entry named `TerranSoul`. Reversible (no admin needed). On non-Windows
//! platforms every function is a no-op returning `Ok(())` so callers don't
//! need conditional compilation.

#[cfg(target_os = "windows")]
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
#[cfg(target_os = "windows")]
const VALUE_NAME: &str = "TerranSoul";

/// Whether the per-user auto-start entry currently exists.
pub fn is_enabled() -> bool {
    #[cfg(target_os = "windows")]
    {
        // Use the `reg` CLI rather than pulling in `winreg` as a new dep.
        // Costs one process spawn per call but is otherwise stable.
        let out = std::process::Command::new("reg")
            .args(["query", &format!(r"HKCU\{RUN_KEY}"), "/v", VALUE_NAME])
            .output();
        match out {
            Ok(o) => o.status.success(),
            Err(_) => false,
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

/// Enable or disable the auto-start entry. `exe_path` is the absolute
/// path to TerranSoul's executable; on Windows it gets quoted in the
/// registry value so paths with spaces work correctly.
pub fn set_enabled(enabled: bool, _exe_path: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        if enabled {
            // `reg add ... /d "<path>" /f` overwrites idempotently.
            let value = format!("\"{}\"", _exe_path);
            let out = std::process::Command::new("reg")
                .args([
                    "add",
                    &format!(r"HKCU\{RUN_KEY}"),
                    "/v",
                    VALUE_NAME,
                    "/t",
                    "REG_SZ",
                    "/d",
                    &value,
                    "/f",
                ])
                .output()
                .map_err(|e| format!("reg add failed to start: {e}"))?;
            if !out.status.success() {
                return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
            }
            Ok(())
        } else {
            let out = std::process::Command::new("reg")
                .args([
                    "delete",
                    &format!(r"HKCU\{RUN_KEY}"),
                    "/v",
                    VALUE_NAME,
                    "/f",
                ])
                .output()
                .map_err(|e| format!("reg delete failed to start: {e}"))?;
            // `reg delete` returns nonzero when the value is already
            // absent — treat that as success because the desired state
            // (disabled) matches reality.
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                if !stderr.contains("unable to find") && !stderr.is_empty() {
                    return Err(stderr.trim().to_string());
                }
            }
            Ok(())
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = enabled;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_windows_set_enabled_is_noop() {
        // On Windows this would actually mutate the registry; we keep the
        // test cheap by only asserting that the function returns Ok in
        // the path we exercise. When the test runs on Linux/macOS the
        // cfg-guarded body is a no-op; on Windows we still avoid asserting
        // stateful behaviour — just that the call succeeds for the
        // disable path which is idempotent.
        let result = set_enabled(false, "C:/nonexistent/terransoul.exe");
        // On Windows `reg delete` may return Ok or "unable to find" (also
        // mapped to Ok). Either way the call should not fail.
        assert!(
            result.is_ok(),
            "set_enabled(false) should not error: {result:?}"
        );
    }

    #[test]
    fn is_enabled_returns_bool_without_panicking() {
        let _ = is_enabled();
    }
}
