use serde::Serialize;
use sysinfo::{Disks, System};

/// Coarse RAM tier used for model recommendations.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum RamTier {
    /// < 4 GB
    VeryLow,
    /// 4–8 GB
    Low,
    /// 8–16 GB
    Medium,
    /// 16–32 GB
    High,
    /// ≥ 32 GB
    VeryHigh,
}

impl RamTier {
    pub fn from_mb(total_mb: u64) -> Self {
        match total_mb {
            0..=4095 => RamTier::VeryLow,
            4096..=8191 => RamTier::Low,
            8192..=16383 => RamTier::Medium,
            16384..=32767 => RamTier::High,
            _ => RamTier::VeryHigh,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            RamTier::VeryLow => "< 4 GB",
            RamTier::Low => "4–8 GB",
            RamTier::Medium => "8–16 GB",
            RamTier::High => "16–32 GB",
            RamTier::VeryHigh => "≥ 32 GB",
        }
    }
}

/// Hardware information collected from the host system.
#[derive(Debug, Clone, Serialize)]
pub struct SystemInfo {
    /// Total RAM in megabytes.
    pub total_ram_mb: u64,
    /// Human-readable RAM tier label (e.g. "8–16 GB").
    pub ram_tier_label: String,
    /// Number of logical CPU cores.
    pub cpu_cores: usize,
    /// CPU brand string (e.g. "Intel Core i7-12700H").
    pub cpu_name: String,
    /// OS name (e.g. "Windows 11", "macOS Sonoma", "Ubuntu 24.04").
    pub os_name: String,
    /// Architecture (e.g. "x86_64", "aarch64").
    pub arch: String,
    /// GPU information if available.
    pub gpu_name: Option<String>,
}

/// Collect hardware information from the host system.
pub fn collect() -> SystemInfo {
    let mut sys = System::new_all();
    sys.refresh_all();

    let total_ram_bytes = sys.total_memory();
    let total_ram_mb = total_ram_bytes / 1_048_576;

    let tier = RamTier::from_mb(total_ram_mb);
    let cpu_cores = sys.cpus().len();

    // Get more detailed CPU information
    let cpu_name = sys
        .cpus()
        .first()
        .map(|c| c.brand().trim().to_string())
        .unwrap_or_else(|| "Unknown CPU".to_string());

    // Enhanced OS detection for Windows
    let os_name = if cfg!(target_os = "windows") {
        // Try to get Windows version more accurately
        match System::name() {
            Some(name) => {
                let version = System::os_version().unwrap_or_default();
                if version.starts_with("10.0.") {
                    let build = version
                        .split('.')
                        .nth(2)
                        .unwrap_or("0")
                        .parse::<u32>()
                        .unwrap_or(0);
                    if build >= 22000 {
                        "Windows 11".to_string()
                    } else {
                        "Windows 10".to_string()
                    }
                } else {
                    format!("{} {}", name, version)
                }
            }
            None => "Windows".to_string(),
        }
    } else {
        format!(
            "{} {}",
            System::name().unwrap_or_else(|| "Unknown OS".to_string()),
            System::os_version().unwrap_or_default()
        )
        .trim()
        .to_string()
    };

    let arch = std::env::consts::ARCH.to_string();

    // Try to detect GPU - basic detection for now
    let gpu_name = detect_gpu();

    SystemInfo {
        total_ram_mb,
        ram_tier_label: tier.label().to_string(),
        cpu_cores,
        cpu_name,
        os_name,
        arch,
        gpu_name,
    }
}

// ── Disk info ────────────────────────────────────────────────────────

/// Disk space information for a single drive/mount.
#[derive(Debug, Clone, Serialize)]
pub struct DiskInfo {
    /// Mount point or drive letter (e.g. "C:\\" or "/").
    pub mount_point: String,
    /// Drive label (e.g. "Windows", "Data") or empty string.
    pub label: String,
    /// Available (free) space in bytes.
    pub available_bytes: u64,
    /// Total capacity in bytes.
    pub total_bytes: u64,
}

/// List all mounted drives/partitions with their space info.
pub fn list_drives() -> Vec<DiskInfo> {
    let disks = Disks::new_with_refreshed_list();
    disks
        .list()
        .iter()
        .map(|d| DiskInfo {
            mount_point: d.mount_point().to_string_lossy().to_string(),
            label: d.name().to_string_lossy().to_string(),
            available_bytes: d.available_space(),
            total_bytes: d.total_space(),
        })
        .collect()
}

/// Get disk space info for the drive containing the given path.
pub fn disk_info_for_path(path: &str) -> Option<DiskInfo> {
    let target = std::path::Path::new(path);
    let disks = Disks::new_with_refreshed_list();

    // Find the disk whose mount point is the longest prefix of the target path.
    disks
        .list()
        .iter()
        .filter(|d| target.starts_with(d.mount_point()))
        .max_by_key(|d| d.mount_point().as_os_str().len())
        .map(|d| DiskInfo {
            mount_point: d.mount_point().to_string_lossy().to_string(),
            label: d.name().to_string_lossy().to_string(),
            available_bytes: d.available_space(),
            total_bytes: d.total_space(),
        })
}

/// Return the directory where Ollama stores downloaded models.
///
/// Checks the `OLLAMA_MODELS` environment variable first.
/// Falls back to the platform default:
/// - Windows: `%USERPROFILE%\.ollama\models`
/// - macOS/Linux: `~/.ollama/models`
pub fn ollama_models_dir() -> String {
    if let Ok(dir) = std::env::var("OLLAMA_MODELS") {
        if !dir.is_empty() {
            return dir;
        }
    }
    let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    home.join(".ollama")
        .join("models")
        .to_string_lossy()
        .to_string()
}

/// Attempt to detect GPU information (basic implementation)
fn detect_gpu() -> Option<String> {
    // On Windows, try to get GPU info via WMI or registry
    #[cfg(target_os = "windows")]
    {
        // This is a simplified detection - in a full implementation
        // you'd use Windows APIs like DXGI or WMI
        // For now, return None and let the frontend handle it
        None
    }

    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ram_tier_very_low() {
        assert_eq!(RamTier::from_mb(2048), RamTier::VeryLow);
        assert_eq!(RamTier::from_mb(0), RamTier::VeryLow);
        assert_eq!(RamTier::from_mb(4095), RamTier::VeryLow);
    }

    #[test]
    fn ram_tier_low() {
        assert_eq!(RamTier::from_mb(4096), RamTier::Low);
        assert_eq!(RamTier::from_mb(6000), RamTier::Low);
        assert_eq!(RamTier::from_mb(8191), RamTier::Low);
    }

    #[test]
    fn ram_tier_medium() {
        assert_eq!(RamTier::from_mb(8192), RamTier::Medium);
        assert_eq!(RamTier::from_mb(12000), RamTier::Medium);
        assert_eq!(RamTier::from_mb(16383), RamTier::Medium);
    }

    #[test]
    fn ram_tier_high() {
        assert_eq!(RamTier::from_mb(16384), RamTier::High);
        assert_eq!(RamTier::from_mb(24000), RamTier::High);
        assert_eq!(RamTier::from_mb(32767), RamTier::High);
    }

    #[test]
    fn ram_tier_very_high() {
        assert_eq!(RamTier::from_mb(32768), RamTier::VeryHigh);
        assert_eq!(RamTier::from_mb(65536), RamTier::VeryHigh);
    }

    #[test]
    fn tier_labels_are_non_empty() {
        for tier in [
            RamTier::VeryLow,
            RamTier::Low,
            RamTier::Medium,
            RamTier::High,
            RamTier::VeryHigh,
        ] {
            assert!(!tier.label().is_empty());
        }
    }

    #[test]
    fn collect_returns_sensible_values() {
        let info = collect();
        assert!(info.total_ram_mb > 0, "RAM must be positive");
        assert!(info.cpu_cores > 0, "CPU cores must be positive");
        assert!(!info.cpu_name.is_empty());
        assert!(!info.arch.is_empty());
    }
}
